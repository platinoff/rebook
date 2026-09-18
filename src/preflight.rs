//! RB-29: pre-KDP preflight — take an existing (published) EPUB, pull its
//! real cover art + metadata, compute KDP print geometry for a target format
//! and emit a warning list so nothing "попливе" after upload: page-range,
//! spine-vs-text rules, cover DPI/aspect, ISBN/EAN-13 validity.

use serde::Serialize;

use crate::Book;
use crate::standards::{
    HARDCOVER_TRIMS, PAPERBACK_TRIMS, Paper, Trim, ebook_cover_ok, even_pages, find_trim,
    hardcover_spine_approx, isbn_to_ean13, paperback_pages_ok, spine_text_allowed, spine_width,
};
use crate::viewer::Epub;

/// One checkbox of the preflight report.
#[derive(Debug, Clone, Serialize)]
pub struct Warn {
    /// Stable id (print:pages-range, …).
    pub id: &'static str,
    /// True when OK.
    pub ok: bool,
    /// Human-readable detail (uk).
    pub detail: String,
}

/// Cover art extracted from the EPUB, as a data-URI for the browser.
#[derive(Debug, Clone, Serialize)]
pub struct CoverArt {
    /// `data:image/…;base64,…`
    pub data_uri: String,
    /// `png` | `jpeg` | …
    pub mime: String,
    /// Entry path the cover was read from (RB-36), e.g. `OEBPS/cover.jpeg`.
    pub file: String,
    /// Pixel width (best effort from IHDR/JPEG SOF).
    pub w_px: u32,
    pub h_px: u32,
}

/// Full preflight report consumed by `/view3d`.
#[derive(Debug, Clone, Serialize)]
pub struct Preflight {
    pub title: String,
    pub author: String,
    pub language: String,
    pub isbn: Option<String>,
    pub cover: Option<CoverArt>,
    /// `hardcover` | `paperback` | `ebook`.
    pub mode: String,
    pub trim: String,
    pub trim_w_in: f64,
    pub trim_h_in: f64,
    pub pages: u32,
    pub spine_in: f64,
    /// Wrap file size (back+spine+front+bleed) inches, print modes.
    pub wrap_w_in: f64,
    pub wrap_h_in: f64,
    /// Back panel auto-fill color (hex) — the flat companion of the cover.
    pub back_color: String,
    pub spine_color: String,
    pub warnings: Vec<Warn>,
}

fn push(v: &mut Vec<Warn>, id: &'static str, ok: bool, detail: impl Into<String>) {
    v.push(Warn {
        id,
        ok,
        detail: detail.into(),
    });
}

/// JPEG SOF color space (RB-46).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegSpace {
    Gray,
    Rgb,
    Cmyk,
}

/// Size + color space from JPEG markers (no decode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegInfo {
    pub w: u32,
    pub h: u32,
    pub space: JpegSpace,
    /// Photoshop APP14 CMYK often needs PDF `/Decode [1 0 1 0 1 0 1 0]`.
    pub invert: bool,
}

/// Walk JPEG markers for SOF0–SOF3 size/components and Adobe APP14.
pub(crate) fn jpeg_info(b: &[u8]) -> Option<JpegInfo> {
    if b.len() < 4 || b[0..2] != [0xFF, 0xD8] {
        return None;
    }
    let mut i = 2usize;
    let mut adobe_tf: Option<u8> = None;
    let mut dim: Option<(u32, u32, u8)> = None;
    while i + 9 < b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = b[i + 1];
        if matches!(m, 0xC0..=0xC3) {
            let h = u16::from_be_bytes(b[i + 5..i + 7].try_into().ok()?) as u32;
            let w = u16::from_be_bytes(b[i + 7..i + 9].try_into().ok()?) as u32;
            let nf = *b.get(i + 9)?;
            dim = Some((w, h, nf));
            break;
        }
        if matches!(m, 0xD8 | 0x01 | 0x00) || (0xD0..=0xD7).contains(&m) {
            i += 2;
            continue;
        }
        if m == 0xD9 {
            break;
        }
        let len = u16::from_be_bytes(b[i + 2..i + 4].try_into().ok()?) as usize;
        if m == 0xEE && len >= 14 && i + 2 + len <= b.len() {
            let payload = &b[i + 4..i + 2 + len];
            if payload.starts_with(b"Adobe") && payload.len() >= 12 {
                adobe_tf = Some(payload[11]);
            }
        }
        i += 2 + len;
    }
    let (w, h, nf) = dim?;
    let space = match nf {
        1 => JpegSpace::Gray,
        4 => JpegSpace::Cmyk,
        _ => JpegSpace::Rgb,
    };
    let invert = space == JpegSpace::Cmyk && adobe_tf.is_some();
    Some(JpegInfo {
        w,
        h,
        space,
        invert,
    })
}

/// PNG IHDR / JPEG SOF0/SOF2 size sniffing (no image crate).
pub(crate) fn img_size(b: &[u8]) -> Option<(u32, u32)> {
    if b.len() > 24 && b[0..8] == *b"\x89PNG\r\n\x1a\n" && &b[12..16] == b"IHDR" {
        let w = u32::from_be_bytes(b[16..20].try_into().ok()?);
        let h = u32::from_be_bytes(b[20..24].try_into().ok()?);
        return Some((w, h));
    }
    jpeg_info(b).map(|j| (j.w, j.h))
}

/// SOF-only CMYK JPEG for marker sniffing and PDF/X shape tests (not a real scan).
#[cfg(test)]
pub(crate) fn stub_cmyk_jpeg(w: u16, h: u16) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&[0xFF, 0xEE, 0x00, 0x0E]);
    v.extend_from_slice(b"Adobe");
    v.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0]);
    let sof_len: u16 = 8 + 3 * 4;
    v.extend_from_slice(&[0xFF, 0xC0]);
    v.extend_from_slice(&sof_len.to_be_bytes());
    v.push(8);
    v.extend_from_slice(&h.to_be_bytes());
    v.extend_from_slice(&w.to_be_bytes());
    v.push(4);
    for id in 1u8..=4 {
        v.push(id);
        v.push(0x11);
        v.push(0);
    }
    v.extend_from_slice(&[0xFF, 0xD9]);
    v
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Minimal base64 (for the cover data-URI).
pub fn b64_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for ch in data.chunks(3) {
        let b0 = ch[0] as u32;
        let b1 = *ch.get(1).unwrap_or(&0) as u32;
        let b2 = *ch.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        s.push(B64[(n >> 18) as usize & 63] as char);
        s.push(B64[(n >> 12) as usize & 63] as char);
        s.push(if ch.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        s.push(if ch.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    s
}

/// Locate + read the cover image from an EPUB (OPF `properties="cover-image"`,
/// `meta name="cover"`, or an entry named cover.*).
pub fn cover_from_epub(epub: &Epub) -> Option<CoverArt> {
    let opf = epub.text("OEBPS/content.opf")?;
    let mut href: Option<String> = None;
    let mut mime = String::new();
    // properties="cover-image"
    for seg in opf.split("<item") {
        let tag = seg.split('>').next().unwrap_or("");
        if tag.contains("cover-image") {
            if let Some(h) = attr(tag, "href") {
                href = Some(h.to_string());
            }
            if let Some(mt) = attr(tag, "media-type") {
                mime = mt.to_string();
            }
            break;
        }
    }
    // <meta name="cover" content="ID">
    if href.is_none()
        && let Some(id) = opf
            .split("<meta")
            .find(|s| s.contains("name=\"cover\""))
            .and_then(|s| attr(s.split('>').next().unwrap_or(s), "content"))
    {
        for seg in opf.split("<item") {
            let tag = seg.split('>').next().unwrap_or("");
            if attr(tag, "id") == Some(id) {
                href = attr(tag, "href").map(str::to_string);
                mime = attr(tag, "media-type").unwrap_or("").to_string();
                break;
            }
        }
    }
    let mut bytes = None;
    let mut name = String::new();
    if let Some(h) = href {
        for cand in [
            format!("OEBPS/{h}"),
            h.clone(),
            h.strip_prefix("./").unwrap_or(&h).to_string(),
        ] {
            if let Some(b) = epub.get(&cand) {
                bytes = Some(b);
                name = cand;
                break;
            }
        }
    }
    if bytes.is_none() {
        for (n, b) in epub.entries_iter() {
            let l = n.to_ascii_lowercase();
            if l.contains("cover")
                && [".png", ".jpg", ".jpeg", ".webp", ".gif"]
                    .iter()
                    .any(|e| l.ends_with(e))
            {
                bytes = Some(b.clone());
                name = n.clone();
                break;
            }
        }
    }
    let bytes = bytes?;
    if mime.is_empty() {
        mime = match name.to_ascii_lowercase() {
            n if n.ends_with(".png") => "image/png",
            n if n.ends_with(".webp") => "image/webp",
            n if n.ends_with(".gif") => "image/gif",
            _ => "image/jpeg",
        }
        .to_string();
    }
    let (w, h) = img_size(&bytes).unwrap_or((0, 0));
    Some(CoverArt {
        data_uri: format!("data:{mime};base64,{}", b64_encode(&bytes)),
        mime: mime.rsplit('/').next().unwrap_or("png").to_string(),
        file: name,
        w_px: w,
        h_px: h,
    })
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let i = tag.find(&key)? + key.len();
    let rest = &tag[i..];
    let end = rest.find('"')?;
    Some(rest[..end].trim())
}

/// Run the preflight for one book + target.
pub fn run(
    epub: &Epub,
    book: &Book,
    mode: &str,
    trim_label: &str,
    paper: Paper,
    pages: u32,
) -> Preflight {
    let pages = even_pages(pages);
    let mut warnings = Vec::new();
    let is_hc = mode == "hardcover";
    let is_ebook = mode == "ebook";
    let table = if is_hc {
        HARDCOVER_TRIMS
    } else {
        PAPERBACK_TRIMS
    };
    let trim: &Trim = find_trim(table, trim_label)
        .or_else(|| find_trim(PAPERBACK_TRIMS, "6x9"))
        .expect("6x9 exists");

    let (spine, wrap) = if is_ebook {
        (0.0, (trim.w, trim.h))
    } else if is_hc {
        let s = hardcover_spine_approx(pages, paper);
        (
            s,
            (
                2.0 * trim.w + s + 2.0 * crate::standards::HC_WRAP_IN,
                trim.h + 2.0 * crate::standards::HC_WRAP_IN,
            ),
        )
    } else {
        let s = spine_width(pages, paper);
        (
            s,
            (
                2.0 * trim.w + s + 2.0 * crate::standards::BLEED_IN,
                trim.h + 2.0 * crate::standards::BLEED_IN,
            ),
        )
    };

    // — checks —
    if is_hc {
        push(
            &mut warnings,
            "hc:pages",
            (76..=550).contains(&pages),
            format!(
                "{} стор.{} (KDP hardcover: 75–550, парне)",
                pages,
                if (76..=550).contains(&pages) {
                    " у межах"
                } else {
                    " — ПОЗА МЕЖАМІ"
                }
            ),
        );
    } else if !is_ebook {
        push(
            &mut warnings,
            "pb:pages-range",
            paperback_pages_ok(trim, pages, paper),
            format!(
                "{pages} стор. для trim {} / {:?} (межі 24–{})",
                trim.label, paper, trim.max.0
            ),
        );
    }
    if !is_ebook {
        let usable = spine - 2.0 * 0.0625;
        let text_ok = !spine_text_allowed(pages) || usable >= 0.12;
        push(
            &mut warnings,
            "print:spine-text",
            text_ok,
            if spine_text_allowed(pages) {
                format!(
                    "корінець {spine:.4}″ → робоча зона {usable:.4}″ {}",
                    if usable >= 0.12 {
                        "достатньо для тексту"
                    } else {
                        "— ЗАНАДТО ВУЗЬКИЙ для назви (потрібно ≥0.12″) "
                    }
                )
            } else {
                "менше 80 стор. — назва на корінці заборонена".to_string()
            },
        );
    }
    let cover = cover_from_epub(epub);
    match &cover {
        Some(c) => {
            push(
                &mut warnings,
                "cover:found",
                true,
                format!("обкладинку знайдено: {} ({})", c.file, c.mime),
            );
            if c.w_px > 0 && !is_ebook {
                let dpi = c.w_px as f64 / trim.w;
                push(
                    &mut warnings,
                    "cover:dpi",
                    dpi >= 300.0,
                    format!(
                        "обкладинка {} ({}px) на панель {:.1}″ → {:.0} DPI (KDP ≥300)",
                        c.file, c.w_px, trim.w, dpi
                    ),
                );
            }
            if c.w_px > 0 && c.h_px > 0 {
                let ar = c.h_px as f64 / c.w_px as f64;
                if is_ebook {
                    push(
                        &mut warnings,
                        "cover:aspect",
                        ebook_cover_ok(c.w_px, c.h_px),
                        format!("eBook cover {:.2}:1 (H:W, KDP ≥1.6:1)", ar),
                    );
                } else {
                    push(
                        &mut warnings,
                        "cover:fit",
                        true,
                        format!(
                            "фото {:.1}×{:.1}px ляже на панель {:.1}×{:.1}″ — вирівнювання center/crop",
                            c.w_px as f64, c.h_px as f64, trim.w, trim.h
                        ),
                    );
                }
            }
        }
        None => push(
            &mut warnings,
            "cover:found",
            false,
            "у EPUB не знайдено cover-image — передня панель буде автозаливка",
        ),
    }
    let isbn = epub_isbn(epub);
    if let Some(i) = &isbn {
        match isbn_to_ean13(i) {
            Ok(e) => push(
                &mut warnings,
                "isbn:ean13",
                true,
                format!("{i} → EAN-13 {e} ✓"),
            ),
            Err(e) => push(&mut warnings, "isbn:ean13", false, e),
        }
    } else {
        push(
            &mut warnings,
            "isbn:present",
            false,
            "ISBN у EPUB немає — KDP видасть свій, barcode-зона резервується",
        );
    }
    push(&mut warnings, "back:autofill", true, "задня панель + корінець — автозаливка напівтоном обкладинки; перевір розташування тексту у safe-зоні 0.25″".to_string());

    Preflight {
        title: book.title.clone(),
        author: book.author.clone(),
        language: book.language.clone(),
        isbn,
        cover,
        mode: mode.to_string(),
        trim: trim.label.to_string(),
        trim_w_in: trim.w,
        trim_h_in: trim.h,
        pages,
        spine_in: spine,
        wrap_w_in: wrap.0,
        wrap_h_in: wrap.1,
        back_color: "#16202c".to_string(),
        spine_color: "#16202c".to_string(),
        warnings,
    }
}

/// ISBN from the OPF identifiers (`urn:isbn:` / identifier-type isbn).
fn epub_isbn(epub: &Epub) -> Option<String> {
    let opf = epub.text("OEBPS/content.opf")?;
    let i = opf.find("urn:isbn:")?;
    let rest = &opf[i + 9..];
    let end = rest.find(['<', '"', ' ']).unwrap_or(rest.len());
    let raw: String = rest[..end]
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'X')
        .collect();
    (!raw.is_empty()).then_some(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChapterMeta;

    fn png1x1() -> Vec<u8> {
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==";
        let rev = |c: char| B64.iter().position(|&x| x == c as u8).unwrap() as u32;
        let mut out = Vec::new();
        let mut acc = 0u32;
        let mut bits = 0;
        for c in b64.chars() {
            if c == '=' {
                break;
            }
            acc = (acc << 6) | rev(c);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
            }
        }
        out
    }

    fn synth_epub(with_cover: bool, isbn: bool) -> Epub {
        let mut opf = String::from(
            r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>T</dc:title><dc:creator>A</dc:creator><dc:language>uk</dc:language>"#,
        );
        if isbn {
            opf.push_str(
                r#"<dc:identifier id="pub-id">urn:isbn:978-3-16-148410-0</dc:identifier>"#,
            );
        }
        opf.push_str(r#"</metadata><manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>"#);
        if with_cover {
            opf.push_str(r#"<item id="cover-image" href="cover.png" media-type="image/png" properties="cover-image"/>"#);
        }
        opf.push_str("</manifest></package>");
        let mut entries = vec![("OEBPS/content.opf".to_string(), opf.into_bytes())];
        if with_cover {
            entries.push(("OEBPS/cover.png".to_string(), png1x1()));
        }
        Epub::from_entries(entries)
    }

    fn book() -> Book {
        Book {
            title: "T".into(),
            author: "A".into(),
            edition: 1,
            year: 2026,
            format: String::new(),
            language: "uk".into(),
            isbn: None,
            chapters: vec![ChapterMeta {
                number: 1,
                title: "x".into(),
                file: "chapters/01.md".into(),
            }],
        }
    }

    #[test]
    fn b64_vectors() {
        assert_eq!(b64_encode(b"foo"), "Zm9v");
        assert_eq!(b64_encode(b"m"), "bQ==");
        assert_eq!(b64_encode(&png1x1()).len() % 4, 0);
    }

    #[test]
    fn png_size_sniff() {
        assert_eq!(img_size(&png1x1()), Some((1, 1)));
        assert_eq!(img_size(b"nope"), None);
    }

    #[test]
    fn cover_extraction_and_preflight() {
        let cov = cover_from_epub(&synth_epub(true, true)).expect("cover found");
        assert_eq!((cov.w_px, cov.h_px), (1, 1));
        assert_eq!(cov.file, "OEBPS/cover.png", "RB-36: which file was read");
        assert!(cov.data_uri.starts_with("data:image/png;base64,"));
        assert!(cover_from_epub(&synth_epub(false, false)).is_none());

        let pf = run(
            &synth_epub(true, true),
            &book(),
            "paperback",
            "6x9",
            Paper::White,
            300,
        );
        assert!(pf.warnings.iter().any(|w| w.id == "cover:found" && w.ok));
        assert!(
            pf.warnings.iter().any(|w| w.id == "isbn:ean13" && w.ok),
            "ISBN from OPF: {:?}",
            pf.isbn
        );
        assert_eq!(pf.isbn.as_deref(), Some("9783161484100"));
        let dpi = pf.warnings.iter().find(|w| w.id == "cover:dpi").unwrap();
        assert!(!dpi.ok); // 1px cover can't be 300 DPI
        let pf2 = run(
            &synth_epub(false, false),
            &book(),
            "hardcover",
            "6x9",
            Paper::White,
            20,
        );
        assert!(!pf2.warnings.iter().any(|w| w.id == "hc:pages" && w.ok));
        assert!(pf2.spine_in > 0.10 && pf2.spine_in < 0.12); // 20p ≈ 0.045+0.06
    }

    #[test]
    fn jpeg_info_reads_cmyk_sof_and_adobe_app14() {
        let j = stub_cmyk_jpeg(8, 8);
        let info = jpeg_info(&j).expect("cmyk jpeg");
        assert_eq!(info.w, 8);
        assert_eq!(info.h, 8);
        assert_eq!(info.space, JpegSpace::Cmyk);
        assert!(info.invert, "APP14 Adobe ⇒ invert Decode");
        assert_eq!(img_size(&j), Some((8, 8)));
    }
}
