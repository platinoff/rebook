//! Shelf: finished products per target format under `products/<slug>/`.
//!
//! One `build_product` pass produces every targeted format from a single
//! config: strict `ebook/*.epub`, plus `paperback/` and `hardcover/`
//! packages — each a zip with the full-wrap cover SVG (from
//! [`crate::standards`] geometry), optional EAN-13 `barcode.svg`, and
//! `manifest.json` (trim/pages/paper/spine/gutter/barcode zone) + a KDP
//! upload `CHECKLIST.md`. Everything stays in the repo workspace tree and is
//! gitignored like live content.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::barcode::barcode_svg;
use crate::cover::{Mode, template, template_svg};
use crate::epub::{EpubConfig, generate_epub};
use crate::standards::{
    BARCODE_ZONE_IN, HARDCOVER_TRIMS, PAPERBACK_TRIMS, Paper, find_trim, gutter_in, spine_width,
};
use crate::viewer::slug;
use crate::{Book, Chapter};

/// ~300 words per standard page — rough print-page estimate when the author
/// does not set `pages` explicitly (the author must confirm from a proof).
pub const WORDS_PER_PAGE: usize = 300;

/// `product.json` — what to build and with which print specs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductConfig {
    /// Targets among ebook / paperback / hardcover.
    pub targets: Vec<String>,
    /// Paperback trim label (see [`crate::standards::PAPERBACK_TRIMS`]).
    #[serde(default = "default_trim")]
    pub trim: String,
    /// Explicit print page count (else estimated from word count).
    #[serde(default)]
    pub pages: Option<u32>,
    /// Interior paper.
    #[serde(default)]
    pub paper: PaperName,
    /// ISBN for the barcode (optional; KDP can stamp its own).
    #[serde(default)]
    pub isbn: Option<String>,
}

fn default_trim() -> String {
    "6x9".to_string()
}

/// Paper as it appears in `product.json`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PaperName {
    #[default]
    White,
    Cream,
    Ground,
    Premium,
}

impl PaperName {
    /// Map to the [`Paper`] constants.
    pub const fn paper(self) -> Paper {
        match self {
            PaperName::White => Paper::White,
            PaperName::Cream => Paper::Cream,
            PaperName::Ground => Paper::Groundwood,
            PaperName::Premium => Paper::PremiumColor,
        }
    }
}

impl Default for ProductConfig {
    fn default() -> Self {
        ProductConfig {
            targets: vec!["ebook".to_string()],
            trim: default_trim(),
            pages: None,
            paper: PaperName::White,
            isbn: None,
        }
    }
}

/// Rough page estimate from prose word count, rounded up to even.
pub fn estimate_pages(chapters: &[Chapter]) -> u32 {
    let words: usize = chapters.iter().map(|c| c.word_count()).sum();
    crate::standards::even_pages((words / WORDS_PER_PAGE).max(1) as u32)
}

/// Paths of everything a build produced.
#[derive(Debug, Clone, Serialize)]
pub struct ProductPaths {
    /// `products/<slug>` root.
    pub dir: PathBuf,
    /// ebook epub, when targeted.
    pub ebook: Option<PathBuf>,
    /// paperback zip package, when targeted.
    pub paperback: Option<PathBuf>,
    /// hardcover zip package, when targeted.
    pub hardcover: Option<PathBuf>,
    /// resolved page count (explicit or estimated).
    pub pages: u32,
    /// true when `pages` came from [`estimate_pages`], not the config.
    pub pages_estimated: bool,
}

/// Per-format package manifest (numbers from `standards`, verifiable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    /// ebook / paperback / hardcover.
    pub format: String,
    /// Trim label (print formats).
    pub trim: String,
    /// Even page count used.
    pub pages: u32,
    /// Interior paper.
    pub paper: String,
    /// Spine width, inches (0 for ebook).
    pub spine_in: f64,
    /// Gutter, inches (0 for ebook).
    pub gutter_in: f64,
    /// Full cover file size, inches.
    pub cover_in: (f64, f64),
    /// Reserved barcode zone, inches.
    pub barcode_zone_in: (f64, f64),
    /// EAN-13 carried by `barcode.svg`, when an ISBN was given.
    pub ean13: Option<String>,
    /// True when `pages` was estimated from words.
    pub pages_estimated: bool,
}

/// Build every targeted product for `book` into `<base>/products/<slug>/`.
pub fn build_product(
    base: &Path,
    book: &Book,
    chapters: &[Chapter],
    cfg: &ProductConfig,
) -> Result<ProductPaths, String> {
    let s = slug(&book.title);
    let dir = base.join("products").join(&s);
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir products: {e}"))?;
    let mut paths = ProductPaths {
        dir,
        ebook: None,
        paperback: None,
        hardcover: None,
        pages: cfg.pages.unwrap_or_else(|| estimate_pages(chapters)),
        pages_estimated: cfg.pages.is_none(),
    };

    for target in &cfg.targets {
        match target.as_str() {
            "ebook" => {
                let out = paths.dir.join("ebook").join(format!("{s}.epub"));
                std::fs::create_dir_all(out.parent().unwrap())
                    .map_err(|e| format!("mkdir ebook: {e}"))?;
                let config = EpubConfig {
                    title: book.title.clone(),
                    author: book.author.clone(),
                    output_path: out.to_string_lossy().into_owned(),
                    cover_image: None,
                    language: book.language.clone(),
                };
                generate_epub(&config, book, chapters)?;
                paths.ebook = Some(out);
                write_checklist(&paths.dir.join("ebook").join("CHECKLIST.md"), "ebook", "")?;
            }
            "paperback" => {
                let p = print_package(&mut paths, &s, cfg, Mode::Paperback, PAPERBACK_TRIMS)?;
                paths.paperback = Some(p);
            }
            "hardcover" => {
                let p = print_package(&mut paths, &s, cfg, Mode::CaseLaminate, HARDCOVER_TRIMS)?;
                paths.hardcover = Some(p);
            }
            other => {
                return Err(format!(
                    "unknown target {other} (ebook|paperback|hardcover)"
                ));
            }
        }
    }
    Ok(paths)
}

fn print_package(
    paths: &mut ProductPaths,
    s: &str,
    cfg: &ProductConfig,
    mode: Mode,
    table: &'static [crate::standards::Trim],
) -> Result<PathBuf, String> {
    let trim = find_trim(table, &cfg.trim)
        .ok_or_else(|| format!("trim {} not available for {}", cfg.trim, mode.tag()))?;
    let paper = cfg.paper.paper();
    let tpl = template(trim, paths.pages, paper, mode)?;
    let dir = paths.dir.join(mode.dir_name());
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {dir:?}: {e}"))?;

    let wrap = template_svg(&tpl);
    std::fs::write(dir.join("cover-wrap.svg"), &wrap)
        .map_err(|e| format!("write cover-wrap: {e}"))?;

    let ean13 = match &cfg.isbn {
        Some(isbn) => {
            let (bw, bh) = BARCODE_ZONE_IN;
            let svg = barcode_svg(isbn, bw, bh)?;
            std::fs::write(dir.join("barcode.svg"), svg).map_err(|e| e.to_string())?;
            Some(crate::standards::isbn_to_ean13(isbn)?)
        }
        None => None,
    };

    let spine = if mode == Mode::Paperback {
        spine_width(paths.pages, paper)
    } else {
        crate::standards::hardcover_spine_approx(paths.pages, paper)
    };
    let manifest = PackageManifest {
        format: mode.tag().to_string(),
        trim: trim.label.to_string(),
        pages: tpl.pages,
        paper: format!("{paper:?}"),
        spine_in: spine,
        gutter_in: gutter_in(paths.pages).unwrap_or(0.0),
        cover_in: (tpl.size.w, tpl.size.h),
        barcode_zone_in: BARCODE_ZONE_IN,
        ean13,
        pages_estimated: paths.pages_estimated,
    };
    let mj = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("manifest.json"), &mj).map_err(|e| e.to_string())?;
    let cl = kdp_checklist(&manifest);
    std::fs::write(dir.join("CHECKLIST.md"), cl).map_err(|e| e.to_string())?;

    let zip_path = paths.dir.join(format!("{s}-{}.zip", mode.tag()));
    zip_dir(&dir, &zip_path)?;
    Ok(zip_path)
}

/// Bundle a whole directory into a deterministic zip (stored order: sorted).
fn zip_dir(dir: &Path, zip_path: &Path) -> Result<(), String> {
    let file = std::fs::File::create(zip_path).map_err(|e| format!("create zip: {e}"))?;
    let mut writer = zip::ZipWriter::new(file);
    let mut entries: Vec<PathBuf> = Vec::new();
    collect(dir, dir, &mut entries)?;
    entries.sort();
    for full in entries {
        let rel = full
            .strip_prefix(dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file(rel, options)
            .map_err(|e| format!("zip start: {e}"))?;
        let bytes = std::fs::read(&full).map_err(|e| format!("zip read: {e}"))?;
        std::io::Write::write_all(&mut writer, &bytes).map_err(|e| format!("zip write: {e}"))?;
    }
    writer.finish().map_err(|e| format!("zip finish: {e}"))?;
    Ok(())
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for ent in std::fs::read_dir(dir).map_err(|e| format!("readdir {root:?}: {e}"))? {
        let p = ent.map_err(|e| e.to_string())?.path();
        if p.is_dir() {
            collect(root, &p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

/// The KDP-side to-do per package — the "upload checklist" half of RB-8.
pub fn kdp_checklist(m: &PackageManifest) -> String {
    let mut s = String::new();
    s.push_str(&format!("# KDP checklist — {} ({})\n\n", m.format, m.trim));
    s.push_str(&format!(
        "- pages: {}{}\n",
        m.pages,
        if m.pages_estimated {
            " (ESTIMATED — confirm from a proof before ordering!)"
        } else {
            ""
        }
    ));
    s.push_str(&format!(
        "- paper: {} · spine {:.4}in · gutter {:.3}in\n",
        m.paper, m.spine_in, m.gutter_in
    ));
    s.push_str(&format!("- cover-wrap: {:.4}×{:.4}in at 300 DPI (cover-wrap.svg → flatten to single PDF ≥300 DPI, no crop marks)\n", m.cover_in.0, m.cover_in.1));
    s.push_str(&format!(
        "- barcode zone 2.0×1.2in bottom-right: {}\n",
        match &m.ean13 {
            Some(e) => format!("supplied (barcode.svg, EAN-13 {e})"),
            None => "left clear — KDP will stamp".to_string(),
        }
    ));
    match m.format.as_str() {
        "paperback" => s.push_str("- interior: single-page PDF, no spreads; upload cover PDF + interior PDF + ebook file separately in KDP\n"),
        "hc" => s.push_str("- case laminate only (no jacket/cloth); 75–550 pages; art prints on the board — keep spine text out of the 0.4in hinge zones\n"),
        _ => {}
    }
    s.push_str("- verify against the KDP-generated cover template before final export (spine constants are KDP's source of truth)\n");
    s
}

fn write_checklist(path: &Path, format: &str, extra: &str) -> Result<(), String> {
    let body = format!(
        "# KDP checklist — {format}\n\n- upload the strict EPUB from this folder (EPUBCheck-clean: mimetype first/stored, nav.xhtml, dcterms:modified)\n{extra}- DRM: per-book toggle; DRM-free buyers may download EPUB since 2026-01-20\n- cover (front-only JPEG/TIFF 1600×2560) uploaded separately in KDP\n"
    );
    std::fs::write(path, body).map_err(|e| format!("write checklist: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChapterMeta;

    fn mini() -> (Book, Vec<Chapter>) {
        let book = Book {
            title: "Shelf Test".to_string(),
            author: "A".to_string(),
            edition: 1,
            year: 2026,
            format: "EPUB 3.2".to_string(),
            language: "uk".to_string(),
            chapters: vec![ChapterMeta {
                number: 1,
                title: "One".to_string(),
                file: "c1.md".to_string(),
            }],
        };
        let chapters = vec![Chapter {
            number: 1,
            title: "One".to_string(),
            content: "слово ".repeat(900),
        }];
        (book, chapters)
    }

    #[test]
    fn estimate_rounds_even() {
        let (_b, ch) = mini();
        assert_eq!(estimate_pages(&ch) % 2, 0);
        assert_eq!(estimate_pages(&ch), 4); // 900 words / 300 wpp → 3 → even 4
    }

    #[test]
    fn full_product_tree_builds() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("shelf-test");
        let _ = std::fs::remove_dir_all(&base);
        let (book, chapters) = mini();
        let cfg = ProductConfig {
            targets: vec![
                "ebook".to_string(),
                "paperback".to_string(),
                "hardcover".to_string(),
            ],
            trim: "6x9".to_string(),
            pages: Some(120),
            paper: PaperName::Cream,
            isbn: Some("978-3-16-148410-0".to_string()),
        };
        let p = build_product(&base, &book, &chapters, &cfg).unwrap();
        assert_eq!(p.pages, 120);
        assert!(!p.pages_estimated);
        let ep = p.ebook.clone().unwrap();
        assert!(ep.exists());
        assert!(p.paperback.clone().unwrap().exists());
        assert!(p.hardcover.clone().unwrap().exists());
        assert!(p.dir.join("paperback").join("manifest.json").exists());
        assert!(p.dir.join("paperback").join("cover-wrap.svg").exists());
        assert!(p.dir.join("paperback").join("barcode.svg").exists());
        assert!(p.dir.join("paperback").join("CHECKLIST.md").exists());

        let mf: PackageManifest = serde_json::from_str(
            &std::fs::read_to_string(p.dir.join("paperback").join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert!((mf.spine_in - 120.0 * 0.0025).abs() < 1e-9);
        assert!((mf.gutter_in - 0.375).abs() < 1e-9);
        assert_eq!(mf.ean13.as_deref(), Some("9783161484100"));

        // the zip opens with the zip reader and carries the expected entries
        let f = std::fs::File::open(p.paperback.unwrap()).unwrap();
        let mut ar = zip::ZipArchive::new(f).unwrap();
        let names: Vec<String> = (0..ar.len())
            .map(|i| ar.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains(&"cover-wrap.svg".to_string()));
        assert!(names.contains(&"manifest.json".to_string()));
        assert!(names.contains(&"barcode.svg".to_string()));
    }

    #[test]
    fn unknown_target_errors() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("shelf-bad");
        let _ = std::fs::remove_dir_all(&base);
        let (book, chapters) = mini();
        let cfg = ProductConfig {
            targets: vec!["audiobook".to_string()],
            ..Default::default()
        };
        assert!(build_product(&base, &book, &chapters, &cfg).is_err());
    }
}
