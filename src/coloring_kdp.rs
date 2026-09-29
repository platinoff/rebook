//! KDP paperback package for a coloring book: interior PDF + wrap PDF.
//!
//! KDP wants two files (RGB, no-bleed 8.5×11, premium color, 130 pages):
//! `interior.pdf` (single pages) and `cover-wrap.pdf` (one-piece wrap, barcode
//! zone empty so KDP can stamp a free ISBN). Cover art must be JPEG — the wrap
//! writer skips PNG.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::coloring::{
    OUTSIDE_MARGIN_IN, Roster, interior_pages, kdp_ok, load_roster, plate_count,
};
use crate::coloring_svg::{
    PAGE_H, PAGE_W, art_dir, art_png_name, identity_png_name, safe_box, side_for_page, views_for,
};
use crate::coverdoc::CoverDoc;
use crate::coverpdf::{CoverPdfReport, render_wrap_pdf};
use crate::pdfwriter::PdfPageBuilder;
use crate::preflight::{b64_encode, jpeg_info};
use crate::standards::{PAPERBACK_TRIMS, Paper, find_trim, paperback_cover, spine_width};

/// Default output dir: `--book DIR/build/kdp`, else `build/coloring-kdp`.
pub fn default_dir() -> PathBuf {
    match crate::paths::book_dir() {
        Some(book) => book.join("build").join("kdp"),
        None => PathBuf::from("build/coloring-kdp"),
    }
}

/// KDP form copy for one book (`--book DIR/listing.json`). Book text lives in
/// the local book folder, never in `src/`; empty fields get generic defaults.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Listing {
    /// KDP subtitle field (defaults to roster `subtitle_en`).
    pub subtitle: String,
    /// Back-cover / product description.
    pub description: String,
    /// Up to seven KDP keywords.
    pub keywords: Vec<String>,
    /// Browse categories.
    pub categories: Vec<String>,
    /// Singular subject noun in interior copy (`car`, `truck`).
    pub noun: String,
    /// Plural subject noun (`cars`, `trucks`).
    pub noun_plural: String,
    /// Dedication page body.
    pub dedication: String,
    /// Ukrainian dedication (Studio source draft).
    pub dedication_uk: String,
    /// Ukrainian plural section title (`Тачки`, `Вантажівки`).
    pub noun_plural_uk: String,
    /// One-line hint of the cover art for the upload checklist.
    pub cover_note: String,
    /// Extra blocked terms (other authors, series) for the metadata lint.
    pub deny: Vec<String>,
    /// Roster words the lint may let through (generic in this book).
    pub allow: Vec<String>,
}

/// Listing of the active book, generic defaults filled from the roster.
pub fn load_listing(roster: &Roster) -> Result<Listing, String> {
    let mut l = match crate::paths::book_dir() {
        Some(dir) => {
            let path = dir.join(crate::paths::BOOK_LISTING);
            if path.is_file() {
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| format!("read {}: {e}", path.display()))?;
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?
            } else {
                Listing::default()
            }
        }
        None => Listing::default(),
    };
    l.fill_defaults(roster);
    Ok(l)
}

impl Listing {
    /// Fill empty fields with generic, brand-free copy from the roster.
    pub fn fill_defaults(&mut self, roster: &Roster) {
        if self.subtitle.trim().is_empty() {
            self.subtitle = roster.subtitle_en.clone();
        }
        if self.noun.trim().is_empty() {
            self.noun = "car".into();
        }
        if self.noun_plural.trim().is_empty() {
            self.noun_plural = format!("{}s", self.noun);
        }
        if self.description.trim().is_empty() {
            self.description = format!(
                "{title} is an adult coloring book of {n} classic {np}. Each model opens with one \
full-color identity plate, then four black contour views: three-quarter, profile, rear, and front. \
Pencil and crayon on the contour pages; slip a sheet under markers.",
                title = roster.title_en,
                n = number_word(roster.cars.len()),
                np = self.noun_plural,
            );
        }
        if self.categories.is_empty() {
            self.categories =
                vec!["Nonfiction > Crafts, Hobbies & Home > Coloring Books for Grown-Ups".into()];
        }
        if self.dedication.trim().is_empty() {
            self.dedication = format!(
                "For everyone who still turns to look at classic {}.",
                self.noun_plural
            );
        }
        if self.dedication_uk.trim().is_empty() {
            self.dedication_uk = "Усім, хто досі обертається вслід класиці.".into();
        }
        if self.noun_plural_uk.trim().is_empty() {
            self.noun_plural_uk = "Моделі".into();
        }
        if self.cover_note.trim().is_empty() {
            self.cover_note = "cover art, no fonts".into();
        }
    }
}

/// English number word up to thirty, digits above.
pub fn number_word(n: usize) -> String {
    const W: [&str; 31] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
        "twenty-one",
        "twenty-two",
        "twenty-three",
        "twenty-four",
        "twenty-five",
        "twenty-six",
        "twenty-seven",
        "twenty-eight",
        "twenty-nine",
        "thirty",
    ];
    W.get(n)
        .map(|s| s.to_string())
        .unwrap_or_else(|| n.to_string())
}

/// Uppercase the first letter.
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// One Print Previewer finding (cover size, margins, missing files).
#[derive(Debug, Clone, Serialize)]
pub struct PreviewIssue {
    /// True = blocks Approve (KDP error), false = warning.
    pub error: bool,
    pub title: String,
    pub detail: String,
}

/// Geometry + file checks for the local Print Previewer (`/kdp`).
#[derive(Debug, Clone, Serialize)]
pub struct PreviewMeta {
    pub title: String,
    pub author: String,
    pub pages: u32,
    pub trim: String,
    pub paper: String,
    pub expected_wrap_w: f64,
    pub expected_wrap_h: f64,
    pub spine_in: f64,
    pub outside_in: f64,
    pub gutter_in: f64,
    pub barcode_w: f64,
    pub barcode_h: f64,
    pub has_interior: bool,
    pub has_wrap: bool,
    pub interior_w: Option<f64>,
    pub interior_h: Option<f64>,
    pub interior_count: Option<u32>,
    pub wrap_w: Option<f64>,
    pub wrap_h: Option<f64>,
    pub wrap_count: Option<u32>,
    pub plates: Vec<PreviewPlate>,
    pub issues: Vec<PreviewIssue>,
}

/// One interior page in the previewer (JPEG when it is art).
#[derive(Debug, Clone, Serialize)]
pub struct PreviewPlate {
    pub n: u32,
    pub kind: String,
    pub jpeg: Option<String>,
}

/// Inspect `dir/interior.pdf` + `dir/cover-wrap.pdf` the way KDP sizes a wrap.
pub fn preview_meta(dir: &Path) -> PreviewMeta {
    let roster = load_roster().ok();
    let pages = roster.as_ref().map(interior_pages).unwrap_or(130);
    let trim = find_trim(PAPERBACK_TRIMS, "8.5x11").expect("8.5x11");
    let cover = paperback_cover(trim, pages, Paper::PremiumColor).expect("cover math");
    let spine = spine_width(pages, Paper::PremiumColor);
    let gutter = crate::standards::gutter_in(pages).unwrap_or(0.375);
    let mut issues = Vec::new();
    let interior_path = dir.join("interior.pdf");
    let wrap_path = dir.join("cover-wrap.pdf");
    let has_interior = interior_path.is_file();
    let has_wrap = wrap_path.is_file();
    let interior_bytes = read_prefix(&interior_path, 64 * 1024);
    let wrap_bytes = read_prefix(&wrap_path, 64 * 1024);
    let interior = interior_bytes.as_deref().and_then(pdf_media_and_count);
    let wrap = wrap_bytes.as_deref().and_then(pdf_media_and_count);
    if !has_interior {
        issues.push(PreviewIssue {
            error: true,
            title: "Manuscript missing".into(),
            detail: format!("No {} — run coloring-kdp", interior_path.display()),
        });
    }
    if !has_wrap {
        issues.push(PreviewIssue {
            error: true,
            title: "Cover wrap missing".into(),
            detail: format!("No {} — run coloring-kdp", wrap_path.display()),
        });
    }
    if let Some((w, h, n)) = interior {
        if (w - trim.w).abs() > 0.02 || (h - trim.h).abs() > 0.02 {
            issues.push(PreviewIssue {
                error: true,
                title: "Interior trim size".into(),
                detail: format!(
                    "Expected {}x{:.3} in, file is {w:.3}x{h:.3}",
                    trim.w, trim.h
                ),
            });
        }
        if n != pages {
            issues.push(PreviewIssue {
                error: true,
                title: "Page count".into(),
                detail: format!("Roster wants {pages} pages, PDF /Count is {n}"),
            });
        }
    }
    if let Some(r) = roster.as_ref()
        && let Ok(listing) = load_listing(r)
    {
        for h in crate::kdp_lint::lint_listing(r, &listing) {
            issues.push(PreviewIssue {
                error: true,
                title: format!("Metadata: {}", h.field),
                detail: crate::kdp_lint::describe(std::slice::from_ref(&h)),
            });
        }
    }
    let plates = roster.as_ref().map(preview_plates).unwrap_or_default();
    if let Some((w, h, n)) = wrap {
        if n != 1 {
            issues.push(PreviewIssue {
                error: true,
                title: "Cover page count".into(),
                detail: format!("Wrap must be one page, PDF /Count is {n}"),
            });
        }
        if (w - trim.w).abs() < 0.05 && (h - trim.h).abs() < 0.05 {
            issues.push(PreviewIssue {
                error: true,
                title: "Your expected cover size is wrong file".into(),
                detail: format!(
                    "Expected {ew:.3}x{eh:.3} but the submitted file size is {w:.3}x{h:.3}. Cover slot got a trim page (interior.pdf) instead of cover-wrap.pdf.",
                    ew = cover.w,
                    eh = cover.h
                ),
            });
        } else if (w - cover.w).abs() > 0.02 || (h - cover.h).abs() > 0.02 {
            issues.push(PreviewIssue {
                error: true,
                title: "Cover size".into(),
                detail: format!(
                    "Expected {ew:.3}x{eh:.3} but the submitted file size is {w:.3}x{h:.3}.",
                    ew = cover.w,
                    eh = cover.h
                ),
            });
        }
    }
    PreviewMeta {
        title: roster
            .as_ref()
            .map(|r| r.title_en.clone())
            .unwrap_or_else(|| "Coloring book".into()),
        author: roster
            .as_ref()
            .map(|r| r.author.clone())
            .unwrap_or_default(),
        pages,
        trim: "8.5x11".into(),
        paper: "Premium color / white".into(),
        expected_wrap_w: cover.w,
        expected_wrap_h: cover.h,
        spine_in: spine,
        outside_in: OUTSIDE_MARGIN_IN,
        gutter_in: gutter,
        barcode_w: crate::standards::BARCODE_ZONE_IN.0,
        barcode_h: crate::standards::BARCODE_ZONE_IN.1,
        has_interior,
        has_wrap,
        interior_w: interior.map(|t| t.0),
        interior_h: interior.map(|t| t.1),
        interior_count: interior.map(|t| t.2),
        wrap_w: wrap.map(|t| t.0),
        wrap_h: wrap.map(|t| t.1),
        wrap_count: wrap.map(|t| t.2),
        plates,
        issues,
    }
}

fn preview_plates(roster: &Roster) -> Vec<PreviewPlate> {
    let mut out = Vec::new();
    let mut n = 1u32;
    let mut listing = Listing::default();
    listing.fill_defaults(roster);
    for _ in front_pages(roster, &listing) {
        out.push(PreviewPlate {
            n,
            kind: "text".into(),
            jpeg: None,
        });
        n += 1;
    }
    for car in &roster.cars {
        out.push(PreviewPlate {
            n,
            kind: "art".into(),
            jpeg: Some(identity_png_name(car).replace(".png", ".jpg")),
        });
        n += 1;
        for (i, _) in views_for(car).into_iter().enumerate() {
            out.push(PreviewPlate {
                n,
                kind: "art".into(),
                jpeg: Some(art_png_name(car, i).replace(".png", ".jpg")),
            });
            n += 1;
        }
    }
    for _ in back_pages(roster) {
        out.push(PreviewPlate {
            n,
            kind: "text".into(),
            jpeg: None,
        });
        n += 1;
    }
    out
}

fn read_prefix(path: &Path, n: usize) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; n];
    let got = f.read(&mut buf).ok()?;
    buf.truncate(got);
    Some(buf)
}

/// First `/MediaBox` (inches) and `/Count`.
pub fn pdf_media_and_count(bytes: &[u8]) -> Option<(f64, f64, u32)> {
    let n = bytes.len().min(64 * 1024);
    let head = String::from_utf8_lossy(&bytes[..n]);
    let box_re = media_box_pts(&head)?;
    let count = regex_count(&head).or_else(|| {
        let tail_n = bytes.len().min(80_000);
        let tail = String::from_utf8_lossy(&bytes[bytes.len() - tail_n..]);
        regex_count(&tail)
    })?;
    Some((box_re.0 / 72.0, box_re.1 / 72.0, count))
}

fn media_box_pts(s: &str) -> Option<(f64, f64)> {
    let i = s.find("/MediaBox")?;
    let rest = s[i..].find('[').map(|j| &s[i + j + 1..])?;
    let end = rest.find(']')?;
    let nums: Vec<f64> = rest[..end]
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    if nums.len() == 4 {
        Some((nums[2] - nums[0], nums[3] - nums[1]))
    } else {
        None
    }
}

fn regex_count(s: &str) -> Option<u32> {
    let i = s.rfind("/Count ")?;
    s[i + 7..].split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
fn text_below_margin(bytes: &[u8], min_pt: f64) -> bool {
    each_content_stream(bytes, |stream| {
        let s = String::from_utf8_lossy(stream);
        let mut prev = "";
        for part in s.split(" Tm") {
            let nums: Vec<f64> = prev
                .split_whitespace()
                .rev()
                .take(2)
                .filter_map(|t| t.parse().ok())
                .collect();
            if nums.len() == 2 && nums[0] < min_pt - 0.05 {
                return true;
            }
            prev = part;
        }
        false
    })
}

#[cfg(test)]
fn full_page_fill(bytes: &[u8], w: f64, h: f64) -> bool {
    let needle = format!("0.0000 0.0000 {w:.4} {h:.4} re f");
    each_content_stream(bytes, |stream| {
        String::from_utf8_lossy(stream).contains(&needle)
    })
}

#[cfg(test)]
fn each_content_stream(bytes: &[u8], mut hit: impl FnMut(&[u8]) -> bool) -> bool {
    let mut i = 0;
    while i + 8 < bytes.len() {
        let Some(rel) = find_bytes(&bytes[i..], b"stream") else {
            break;
        };
        let keyword = i + rel;
        if keyword >= 3 && bytes[keyword - 3..keyword] == *b"end" {
            i = keyword + 6;
            continue;
        }
        let dict_from = keyword.saturating_sub(240);
        let dict = &bytes[dict_from..keyword];
        let jpeg = find_bytes(dict, b"/DCTDecode").is_some();
        let declared = length_from_dict(dict);
        let mut start = keyword + 6;
        if start < bytes.len() && bytes[start] == b'\r' {
            start += 1;
        }
        if start < bytes.len() && bytes[start] == b'\n' {
            start += 1;
        }
        let end = declared
            .map(|n| (start + n).min(bytes.len()))
            .or_else(|| find_bytes(&bytes[start..], b"endstream").map(|e| start + e))
            .unwrap_or(bytes.len());
        i = end + 9;
        if jpeg || bytes.get(start..start.saturating_add(2)) == Some(&[0xFF, 0xD8]) {
            continue;
        }
        if hit(&bytes[start..end]) {
            return true;
        }
    }
    false
}

#[cfg(test)]
fn length_from_dict(dict: &[u8]) -> Option<usize> {
    let s = std::str::from_utf8(dict).ok()?;
    let i = s.rfind("/Length ")?;
    s[i + 8..].split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Loud alias so the Cover slot is not interior.pdf (8.5×11).
pub const WRAP_UPLOAD_NAME: &str = "UPLOAD-COVER-ONLY-17.555x11.25in.pdf";

/// Files written by [`package`].
#[derive(Debug, Clone)]
pub struct KdpFiles {
    /// 8.5×11 interior, 130 pages.
    pub interior: PathBuf,
    /// Full wrap including bleed + spine.
    pub wrap: PathBuf,
    /// Listing copy for the KDP form.
    pub listing: PathBuf,
}

/// Build JPEG cache + both PDFs + listing copy under `out_dir`.
pub fn package(out_dir: &Path) -> Result<KdpFiles, String> {
    let roster = load_roster()?;
    kdp_ok(&roster)?;
    let listing = load_listing(&roster)?;
    let hits = crate::kdp_lint::lint_listing(&roster, &listing);
    if !hits.is_empty() {
        return Err(format!(
            "KDP metadata lint failed (fix listing.json):\n{}",
            crate::kdp_lint::describe(&hits)
        ));
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir kdp: {e}"))?;
    let jpeg_dir = out_dir.join("jpeg");
    let pages = interior_pages(&roster);
    let spine = spine_width(pages, Paper::PremiumColor);
    rasterize_masters(
        &art_dir(),
        &jpeg_dir,
        pages,
        spine,
        &roster.title_en.to_ascii_uppercase(),
    )?;
    let interior = out_dir.join("interior.pdf");
    let wrap = out_dir.join("cover-wrap.pdf");
    write_interior(&roster, &listing, &jpeg_dir, &interior)?;
    let wrap_rep = write_wrap(&roster, &jpeg_dir, &wrap)?;
    if !wrap_rep.image_placed {
        return Err("cover wrap skipped the JPEG (need 300 DPI RGB JPEG)".into());
    }
    let wrap_upload = out_dir.join(WRAP_UPLOAD_NAME);
    std::fs::copy(&wrap, &wrap_upload).map_err(|e| format!("copy wrap alias: {e}"))?;
    std::fs::copy(&wrap, out_dir.join("cover.pdf")).map_err(|e| format!("copy cover.pdf: {e}"))?;
    let listing_path = out_dir.join("KDP.txt");
    std::fs::write(&listing_path, listing_copy(&roster, &listing))
        .map_err(|e| format!("write listing: {e}"))?;
    Ok(KdpFiles {
        interior,
        wrap,
        listing: listing_path,
    })
}

fn rasterize_masters(
    art: &Path,
    jpeg_dir: &Path,
    pages: u32,
    spine: f64,
    spine_text: &str,
) -> Result<(), String> {
    if !art.is_dir() {
        return Err(format!("art masters missing: {}", art.display()));
    }
    std::fs::create_dir_all(jpeg_dir).map_err(|e| format!("mkdir jpeg: {e}"))?;
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/coloring-png-to-jpeg.ps1");
    if !script.is_file() {
        return Err(format!("missing {}", script.display()));
    }
    let st = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script.to_string_lossy(),
            "-ArtDir",
            &art.to_string_lossy(),
            "-OutDir",
            &jpeg_dir.to_string_lossy(),
            "-Pages",
            &pages.to_string(),
            "-SpineIn",
            &format!("{spine:.6}"),
            "-SpineText",
            spine_text,
        ])
        .status()
        .map_err(|e| format!("powershell: {e}"))?;
    if !st.success() {
        return Err(format!("png→jpeg failed (status {st})"));
    }
    for name in ["cover-front.jpg", "cover-back.jpg", "cover-wrap.jpg"] {
        if !jpeg_dir.join(name).is_file() {
            return Err(format!("missing {name} after rasterize"));
        }
    }
    Ok(())
}

fn write_interior(
    roster: &Roster,
    listing: &Listing,
    jpeg_dir: &Path,
    out: &Path,
) -> Result<(), String> {
    let pages = interior_pages(roster);
    let plan = interior_plan(roster, listing, jpeg_dir)?;
    if plan.len() != pages as usize {
        return Err(format!("plan {} pages, roster wants {pages}", plan.len()));
    }
    let mut pb = PdfPageBuilder::new(PAGE_W, PAGE_H);
    for (i, kind) in plan.iter().enumerate() {
        if i > 0 {
            pb.new_page(PAGE_W, PAGE_H);
        }
        let page_no = (i as u32) + 1;
        match kind {
            PageKind::Text { heading, body } => {
                paint_text(&mut pb, page_no, pages, heading, body);
            }
            PageKind::Art { jpeg } => {
                paint_art(&mut pb, page_no, pages, jpeg)?;
            }
        }
        paint_folio(&mut pb, page_no);
    }
    let bytes = pb.build(&roster.title_en);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir interior: {e}"))?;
    }
    std::fs::write(out, bytes).map_err(|e| format!("write interior: {e}"))?;
    Ok(())
}

fn write_wrap(roster: &Roster, jpeg_dir: &Path, out: &Path) -> Result<CoverPdfReport, String> {
    let flat = jpeg_dir.join("cover-wrap.jpg");
    if flat.is_file() {
        return write_flat_wrap(&flat, roster, out);
    }
    let pages = interior_pages(roster);
    let mut doc = CoverDoc::new(
        roster.title_en.as_str(),
        roster.author.as_str(),
        "pb",
        "8.5x11",
        pages,
        "premium",
    );
    doc.title.text.clear();
    doc.author.text.clear();
    doc.bg_front = "#1a120c".into();
    doc.bg_back = "#1a120c".into();
    doc.spine_bg = Some("#120c08".into());
    doc.isbn = None;
    if let Some(st) = doc.spine_title.as_mut() {
        st.text = roster.title_en.to_ascii_uppercase();
    }
    doc.front_image = Some(jpeg_uri(&jpeg_dir.join("cover-front.jpg"))?);
    doc.back_image = Some(jpeg_uri(&jpeg_dir.join("cover-back.jpg"))?);
    render_wrap_pdf(&doc, out)
}

/// One JPEG, exact wrap MediaBox, no Helvetica — KDP Cover slot.
fn write_flat_wrap(jpeg: &Path, roster: &Roster, out: &Path) -> Result<CoverPdfReport, String> {
    let bytes = std::fs::read(jpeg).map_err(|e| format!("read {}: {e}", jpeg.display()))?;
    let info = jpeg_info(&bytes).ok_or_else(|| format!("not jpeg: {}", jpeg.display()))?;
    if info.space != crate::preflight::JpegSpace::Rgb {
        return Err(format!("{} is not RGB JPEG", jpeg.display()));
    }
    let pages = interior_pages(roster);
    let trim = find_trim(PAPERBACK_TRIMS, "8.5x11").expect("8.5x11");
    let cover = paperback_cover(trim, pages, Paper::PremiumColor).expect("cover math");
    let mut pb = PdfPageBuilder::new(cover.w * 72.0, cover.h * 72.0);
    let idx = pb.add_image(info.w, info.h, bytes);
    pb.draw_image_contain(idx, 0.0, 0.0, cover.w * 72.0, cover.h * 72.0);
    let pdf = pb.build("");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir wrap: {e}"))?;
    }
    std::fs::write(out, &pdf).map_err(|e| format!("write wrap: {e}"))?;
    Ok(CoverPdfReport {
        image_placed: true,
        bytes: pdf.len(),
        ..CoverPdfReport::default()
    })
}

fn jpeg_uri(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let info = jpeg_info(&bytes).ok_or_else(|| format!("not jpeg: {}", path.display()))?;
    if info.space != crate::preflight::JpegSpace::Rgb {
        return Err(format!("{} is not RGB JPEG", path.display()));
    }
    Ok(format!("data:image/jpeg;base64,{}", b64_encode(&bytes)))
}

enum PageKind {
    Text { heading: String, body: String },
    Art { jpeg: PathBuf },
}

fn interior_plan(
    roster: &Roster,
    listing: &Listing,
    jpeg_dir: &Path,
) -> Result<Vec<PageKind>, String> {
    let mut out = Vec::with_capacity(interior_pages(roster) as usize);
    for (heading, body) in front_pages(roster, listing) {
        out.push(PageKind::Text { heading, body });
    }
    for car in &roster.cars {
        out.push(art_page(jpeg_dir, &identity_png_name(car))?);
        for (i, _) in views_for(car).into_iter().enumerate() {
            out.push(art_page(jpeg_dir, &art_png_name(car, i))?);
        }
    }
    for (heading, body) in back_pages(roster) {
        out.push(PageKind::Text { heading, body });
    }
    Ok(out)
}

fn art_page(jpeg_dir: &Path, png_name: &str) -> Result<PageKind, String> {
    let jpg = jpeg_dir.join(png_name.replace(".png", ".jpg"));
    if !jpg.is_file() {
        return Err(format!("missing plate JPEG {}", jpg.display()));
    }
    Ok(PageKind::Art { jpeg: jpg })
}

/// Baseline of the folio, points from the bottom. Must sit above KDP's 0.25"
/// no-bleed outside margin (18 pt) — 12 pt was getting flagged in Previewer.
fn folio_y() -> f64 {
    OUTSIDE_MARGIN_IN * 72.0 + 8.0
}

fn paint_folio(pb: &mut PdfPageBuilder, page: u32) {
    pb.set_fill(0.15, 0.15, 0.15);
    let label = page.to_string();
    let w = label.len() as f64 * 6.0;
    pb.text(PAGE_W / 2.0 - w / 2.0, folio_y(), 10.0, &label);
}

fn paint_text(pb: &mut PdfPageBuilder, page: u32, pages: u32, heading: &str, body: &str) {
    let safe = safe_box(side_for_page(page), pages);
    pb.set_fill(0.08, 0.08, 0.08);
    let mut y = PAGE_H - safe.y - 36.0;
    let heading = latin1(heading);
    pb.text(safe.x, y, 22.0, &heading);
    y -= 36.0;
    pb.set_fill(0.12, 0.12, 0.12);
    let max_chars = ((safe.w / 7.2) as usize).clamp(28, 92);
    for para in body.split("\n\n") {
        for line in wrap_line(&latin1(para), max_chars) {
            if y < safe.y + 28.0 {
                break;
            }
            pb.text(safe.x, y, 12.0, &line);
            y -= 18.0;
        }
        y -= 10.0;
    }
}

fn paint_art(pb: &mut PdfPageBuilder, page: u32, pages: u32, jpeg: &Path) -> Result<(), String> {
    let bytes = std::fs::read(jpeg).map_err(|e| format!("read {}: {e}", jpeg.display()))?;
    let info = jpeg_info(&bytes).ok_or_else(|| format!("not jpeg: {}", jpeg.display()))?;
    let idx = pb.add_image(info.w, info.h, bytes);
    let safe = safe_box(side_for_page(page), pages);
    let img_h = (PAGE_H - safe.y - folio_y() - 16.0).max(72.0);
    let y = PAGE_H - safe.y - img_h;
    pb.draw_image_contain(idx, safe.x, y, safe.w, img_h);
    Ok(())
}

fn latin1(s: &str) -> String {
    s.replace('—', "--")
        .replace('–', "-")
        .replace(['\u{2019}', '\u{2018}'], "'")
        .replace(['\u{201c}', '\u{201d}'], "\"")
        .replace('·', " | ")
        .replace('×', "x")
        .chars()
        .map(|c| if (c as u32) <= 0xFF { c } else { '?' })
        .collect()
}

fn wrap_line(s: &str, max: usize) -> Vec<String> {
    let s = s.replace('\n', " ");
    let mut lines = Vec::new();
    let mut cur = String::new();
    for w in s.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(w);
        } else if cur.len() + 1 + w.len() <= max {
            cur.push(' ');
            cur.push_str(w);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur.push_str(w);
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn one_plate(roster: &Roster) -> bool {
    !roster.cars.is_empty() && roster.cars.iter().all(|c| c.plates == 1)
}

fn front_pages(roster: &Roster, listing: &Listing) -> Vec<(String, String)> {
    let mid = roster.cars.len() / 2;
    let noun = &listing.noun;
    let one = one_plate(roster);
    vec![
        (
            roster.title_en.clone(),
            format!(
                "{}\n\n{}\n\nPaperback coloring book. 8.5 x 11 in. Premium color. No bleed.",
                roster.subtitle_en, roster.author
            ),
        ),
        (
            "Copyright".into(),
            if one {
                "Text and illustrations are original to this edition.\n\n\
AI helped draw the plates; the author reviewed them and is responsible for the work.\n\n\
Print: Amazon KDP, paperback 8.5x11, premium color interior, no bleed."
            } else {
                "Text and contour illustrations are original to this edition, not factory blueprints.\n\n\
AI helped draw the plates; the author reviewed them and is responsible for the work.\n\n\
Print: Amazon KDP, paperback 8.5x11, premium color interior, no bleed."
            }
            .into(),
        ),
        (
            "This book belongs to".into(),
            if one {
                "Name: ______________________________\n\nCity: ______________________\n\nYear: __________"
            } else {
                "Name: ______________________________\n\nGarage / city: ______________________\n\nYear: __________"
            }
            .into(),
        ),
        (
            "How to color".into(),
            format!(
                "{}\n\n\
Pencil and crayon on the contour. Markers bleed: slip a sheet underneath.\n\n\
Line weight is at least 0.75 pt; this edition draws at 1.25 pt.",
                if one {
                    format!("Each {noun} opens with one color plate, then one black coloring page of that same subject.")
                } else {
                    format!("Each {noun} opens with one color plate -- make, model, and year, once. Then four black contour views: three-quarter, profile, rear, front. No repeated captions.")
                }
            ),
        ),
        (
            "Contents".into(),
            toc(&roster.cars[..mid]),
        ),
        (
            "Contents (continued)".into(),
            toc(&roster.cars[mid..]),
        ),
        ("Dedication".into(), listing.dedication.clone()),
        (
            format!("The {}", listing.noun_plural),
            format!(
                "{} subjects. Each {noun}: one color plate + {}. {} coloring pages.",
                capitalize(&number_word(roster.cars.len())),
                if one {
                    "one coloring page"
                } else {
                    "four contour views"
                },
                plate_count(roster)
            ),
        ),
    ]
}

fn back_pages(roster: &Roster) -> Vec<(String, String)> {
    let mut by_name = roster.cars.clone();
    by_name.sort_by(|a, b| a.make.cmp(&b.make).then(a.model.cmp(&b.model)));
    let mut by_place = roster.cars.clone();
    by_place.sort_by(|a, b| a.model.cmp(&b.model).then(a.make.cmp(&b.make)));
    let mut names = String::new();
    for c in &by_name {
        names.push_str(&format!("{}\n\n", subject_line(c)));
    }
    let mut places = String::new();
    for c in &by_place {
        places.push_str(&format!("{}\n\n", subject_line(c)));
    }
    if one_plate(roster) {
        vec![
            ("Index by name".into(), names),
            ("Index by place".into(), places),
        ]
    } else {
        let mut by_year: Vec<&crate::coloring::Car> = roster.cars.iter().collect();
        by_year.sort_by_key(|c| (c.year, c.make.as_str(), c.model.as_str()));
        let mut y = String::new();
        for c in by_year {
            y.push_str(&format!("{}\n\n", subject_line(c)));
        }
        vec![("Index by year".into(), y), ("Index by make".into(), names)]
    }
}

fn subject_line(c: &crate::coloring::Car) -> String {
    if c.year == 0 {
        format!("{} {}", c.make, c.model)
    } else {
        format!("{}  {} {}", c.year, c.make, c.model)
    }
}

fn toc(cars: &[crate::coloring::Car]) -> String {
    let mut s = String::new();
    for c in cars {
        s.push_str(&format!("{}\n\n", subject_line(c)));
    }
    s
}

fn indent_block(s: &str, width: usize) -> String {
    let mut out = String::new();
    for para in s.split("\n\n") {
        for line in wrap_line(para, width) {
            out.push_str("  ");
            out.push_str(&line);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

fn listing_copy(roster: &Roster, listing: &Listing) -> String {
    let description = indent_block(&listing.description, 72);
    let categories = listing
        .categories
        .iter()
        .map(|c| format!("  {c}\n"))
        .collect::<String>();
    let keywords = if listing.keywords.is_empty() {
        "  (none yet -- add \"keywords\" to listing.json in the book folder)\n".to_string()
    } else {
        listing
            .keywords
            .iter()
            .map(|k| format!("  {k}\n"))
            .collect::<String>()
    };
    let pages = interior_pages(roster);
    let trim = find_trim(PAPERBACK_TRIMS, "8.5x11").expect("8.5x11");
    let cover = paperback_cover(trim, pages, Paper::PremiumColor).expect("cover math");
    let spine = spine_width(pages, Paper::PremiumColor);
    format!(
        "KDP paperback upload -- {title}\n\
================================\n\n\
Create a NEW paperback (not ebook, not hardcover).\n\n\
79 PAGES IS NOT A MAXIMUM\n\
  The cover screen note (\"at least 79 pages\" / Cover Creator spine text)\n\
  is the MINIMUM page count before Amazon will print text on the spine.\n\
  This interior is {pages} pages. Premium color 8.5x11 allows 24-590.\n\
  Do not cut pages. Do not use Cover Creator.\n\
  If Print Previewer is blocked: wait until KDP shows {pages} pages under\n\
  Print Options, then upload cover-wrap.pdf as \"Print-ready PDF cover\".\n\n\
LANGUAGE\n\
  English\n\n\
TITLE\n\
  {title}\n\n\
SUBTITLE\n\
  {subtitle}\n\n\
AUTHOR\n\
  {author}\n\n\
DESCRIPTION (paste)\n\
{description}\
PUBLISHING RIGHTS\n\
  I own the copyright and I hold the necessary publishing rights\n\n\
AUDIENCE\n\
  Not a children's book. Adult / teen coloring. Uncheck any \"for children\" box.\n\n\
CATEGORIES (pick two close matches)\n\
{categories}\n\
KEYWORDS (seven -- no brand names, no other authors, no free/bestseller)\n\
{keywords}\n\
ISBN\n\
  Get a free KDP ISBN. Do not upload your own barcode -- the wrap leaves\n\
  the lower-right of the back cover empty for Amazon's stamp.\n\n\
PRINT OPTIONS\n\
  Ink and paper: Premium color / white\n\
  Trim: 8.5 x 11 in (21.59 x 27.94 cm)\n\
  Bleed: No bleed\n\
  Cover finish: Matte (recommended) or glossy\n\
  Page count KDP will read from the PDF: {pages} (even)\n\
  Spine (our math): {spine:.4} in\n\
  Wrap file size (our math): {ww:.4} x {wh:.4} in\n\n\
UPLOAD (one file at a time — wait for Processing complete)\n\
  1. Manuscript slot: interior.pdf\n\
     (8.500 x 11.000 in, {pages} pages, white title page)\n\
     Wait until Print Options shows {pages} pages.\n\
  2. Cover slot: cover.pdf\n\
     ({ww:.3} x {wh:.3} in, ONE page, {cover_note})\n\
     Same bytes as {wrap_name}.\n\
  If Previewer says expected {ww:.3}x{wh:.3} but submitted 8.500x11.000,\n\
  the Cover slot still has interior.pdf. Delete Cover, upload cover.pdf.\n\
  Do not use Cover Creator. If the spinner never finishes: close, incognito,\n\
  manuscript first, wait, then cover.pdf only.\n\n\
AFTER UPLOAD\n\
  Open KDP Print Preview. Check: barcode sits in the empty back corner;\n\
  spine text is readable; no art in the gutter; color plates are color;\n\
  contour plates are black line on white. Order a proof copy before Publish.\n\
  Premium color 8.5x11 at {pages} pages has a high print cost -- set list\n\
  price from KDP's calculated minimum.\n",
        title = roster.title_en,
        subtitle = listing.subtitle,
        author = roster.author,
        cover_note = listing.cover_note,
        pages = pages,
        spine = spine,
        ww = cover.w,
        wh = cover.h,
        wrap_name = WRAP_UPLOAD_NAME,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interior_plan_is_130_english() {
        let r = load_roster().unwrap();
        kdp_ok(&r).unwrap();
        let l = load_listing(&r).unwrap();
        assert_eq!(front_pages(&r, &l).len(), 8);
        assert_eq!(back_pages(&r).len(), 2);
        let jpeg = PathBuf::from("no-jpegs");
        let n_art = r.cars.len() * 5;
        assert_eq!(n_art, 120);
        let missing = interior_plan(&r, &l, &jpeg);
        assert!(missing.is_err(), "plan must require plate JPEGs");
        assert_eq!(8 + 120 + 2, interior_pages(&r));
    }

    #[test]
    fn listing_defaults_are_generic_and_json_overrides() {
        let r = load_roster().unwrap();
        let mut l = Listing::default();
        l.fill_defaults(&r);
        assert_eq!(l.subtitle, r.subtitle_en);
        assert_eq!((l.noun.as_str(), l.noun_plural.as_str()), ("car", "cars"));
        assert!(l.description.contains("twenty-four classic cars"));
        let txt = listing_copy(&r, &l);
        assert!(txt.contains("none yet"), "no keywords baked into src");
        let mut t: Listing = serde_json::from_str(
            r#"{"noun":"truck","keywords":["kw one","kw two"],"description":"Desc."}"#,
        )
        .unwrap();
        t.fill_defaults(&r);
        assert_eq!(t.noun_plural, "trucks");
        let txt = listing_copy(&r, &t);
        assert!(txt.contains("  kw one\n  kw two\n"));
        assert!(txt.contains("  Desc.\n"));
        let front = front_pages(&r, &t);
        assert!(front.iter().any(|(h, _)| h == "The trucks"));
        assert!(
            front
                .iter()
                .any(|(_, b)| b.starts_with("Twenty-four classics. Each truck"))
        );
    }

    #[test]
    fn folio_sits_inside_no_bleed_outside_margin() {
        assert!(folio_y() >= OUTSIDE_MARGIN_IN * 72.0);
    }

    #[test]
    fn wrap_geometry_matches_kdp_premium_130() {
        let r = load_roster().unwrap();
        let pages = interior_pages(&r);
        let trim = find_trim(PAPERBACK_TRIMS, "8.5x11").unwrap();
        let c = paperback_cover(trim, pages, Paper::PremiumColor).unwrap();
        assert!((c.h - 11.25).abs() < 1e-9, "11 + 2*0.125 bleed");
        assert!(
            c.w > 17.5 && c.w < 17.6,
            "two 8.5 panels + spine + bleed, got {}",
            c.w
        );
    }

    #[test]
    fn latin1_strips_emdash() {
        assert!(
            latin1("plates — original")
                .chars()
                .all(|c| (c as u32) <= 0xFF)
        );
        assert!(latin1("plates — original").contains("--"));
    }

    #[test]
    fn pdf_media_and_count_reads_writer() {
        let mut p = crate::pdfwriter::PdfPageBuilder::new(612.0, 792.0);
        p.new_page(612.0, 792.0);
        let b = p.build("t");
        let (w, h, n) = pdf_media_and_count(&b).expect("pages dict");
        assert_eq!(n, 2);
        assert!((w - 8.5).abs() < 0.01);
        assert!((h - 11.0).abs() < 0.01);
    }

    #[test]
    fn preview_meta_flags_missing_dir() {
        let m = preview_meta(Path::new("no-such-kdp-dir-xyz"));
        assert!(m.issues.iter().any(|i| i.title.contains("missing")));
        assert!(!m.has_wrap && !m.has_interior);
        assert!((m.expected_wrap_h - 11.25).abs() < 1e-9);
    }

    #[test]
    fn margin_scan_flags_low_folio_only() {
        let mut low = crate::pdfwriter::PdfPageBuilder::new(612.0, 792.0);
        low.text(100.0, 12.0, 10.0, "1");
        assert!(text_below_margin(&low.build("t"), 18.0));
        let mut ok = crate::pdfwriter::PdfPageBuilder::new(612.0, 792.0);
        ok.text(100.0, 26.0, 10.0, "1");
        assert!(!text_below_margin(&ok.build("t"), 18.0));
        let mut fill = crate::pdfwriter::PdfPageBuilder::new(612.0, 792.0);
        fill.rect(0.0, 0.0, 612.0, 792.0);
        assert!(full_page_fill(&fill.build("t"), 612.0, 792.0));
    }
}
