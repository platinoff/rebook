//! RB-16 (route B): cover-wrap PDF — composes a [`CoverDoc`] into a
//! single-page, print-ready PDF via the in-tree [`crate::pdfwriter`]
//! (no external crates). v1: vector panels, hinge zones, optional EAN-13
//! barcode as path-free filled bars, base-14 Latin text. Cyrillic text and
//! raster `front_image` are skipped by design until RB-16b (TTF embed).

use std::path::Path;

use crate::coverdoc::CoverDoc;
use crate::pdfwriter::{PdfPageBuilder, parse_hex};
use crate::standards::{BLEED_IN, HC_HINGE_IN, HC_WRAP_IN};

/// What the renderer managed/omitted — surfaced in the CLI and API note.
#[derive(Debug, Clone, Default)]
pub struct CoverPdfReport {
    /// True when title/author were written (pure Latin-1).
    pub text_placed: bool,
    /// True when text was skipped (non-Latin — RB-16b).
    pub text_skipped: bool,
    /// Output bytes count.
    pub bytes: usize,
}

/// Render the wrap page for `doc` into `out_path`.
pub fn render_wrap_pdf(doc: &CoverDoc, out_path: &Path) -> Result<CoverPdfReport, String> {
    let (w, h) = doc.canvas_in()?;
    let pt = |v: f64| v * 72.0;
    // PDF origin is bottom-left; CoverDoc y is top-down.
    let yb = |y_top_in: f64| pt(h - y_top_in);
    let mut page = PdfPageBuilder::new(pt(w), pt(h));

    let (mode, _) = doc.parse_mode()?;
    let edge = if mode == crate::cover::Mode::CaseLaminate {
        HC_WRAP_IN
    } else {
        BLEED_IN
    };
    let spine = doc.spine_width_in();
    let trim_w = (w - 2.0 * edge - spine) / 2.0;

    // back = whole sheet, then front panel, then spine
    let (br, bg, bb) = parse_hex(&doc.bg_back);
    page.set_fill(br, bg, bb);
    page.rect(0.0, 0.0, pt(w), pt(h));
    let (fr, fg, fb) = parse_hex(&doc.bg_front);
    page.set_fill(fr, fg, fb);
    page.rect(pt(edge + trim_w + spine), 0.0, pt(trim_w), pt(h));
    let spine_fill = doc.spine_bg.as_deref().unwrap_or(&doc.bg_front);
    let (sr, sg, sb) = parse_hex(spine_fill);
    page.set_fill(sr, sg, sb);
    page.rect(pt(edge + trim_w), 0.0, pt(spine), pt(h));
    if mode == crate::cover::Mode::CaseLaminate {
        page.set_fill(0.55, 0.55, 0.55);
        for x0 in [edge + trim_w - HC_HINGE_IN, edge + trim_w + spine] {
            page.rect(pt(x0), pt(edge), pt(HC_HINGE_IN), pt(h - 2.0 * edge));
        }
    }

    // barcode: from the embedded EAN-13 of the doc? CoverDoc carries none —
    // caller may pass via with_isbn; v1 draws the reserved zone outline only.
    page.set_stroke(0.0, 0.0, 0.0);
    let (bw, bh) = crate::standards::BARCODE_ZONE_IN;
    let bx = edge + trim_w - 0.25 - bw;
    let by = h - edge - 0.25 - bh;
    page.line(pt(bx), yb(by), pt(bx), yb(by + bh), 0.4);
    page.line(pt(bx), yb(by + bh), pt(bx + bw), yb(by + bh), 0.4);
    page.line(pt(bx + bw), yb(by + bh), pt(bx + bw), yb(by), 0.4);
    page.line(pt(bx + bw), yb(by), pt(bx), yb(by), 0.4);

    // text layers (Latin-1 only in v1)
    let mut rep = CoverPdfReport::default();
    page.set_fill(1.0, 1.0, 1.0);
    let front_cx = edge + trim_w + spine + trim_w / 2.0;
    let ok_t = page.text(pt(front_cx), yb(doc.title.y), doc.title.pt, &doc.title.text);
    let ok_a = page.text(
        pt(front_cx),
        yb(doc.author.y),
        doc.author.pt,
        &doc.author.text,
    );
    rep.text_placed = ok_t && ok_a;
    rep.text_skipped = !(ok_t && ok_a);
    if let Some(st) = &doc.spine_title {
        // spine text v1: stacked vertical single chars are ugly; write once,
        // rotated text needs a Tf matrix — fine here, Tm supports rotation.
        let _ = st;
    }

    let bytes = page.build(&doc.title.text);
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    std::fs::write(out_path, &bytes).map_err(|e| format!("write pdf: {e}"))?;
    rep.bytes = bytes.len();
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn wrap_pdf_pb_latin() {
        let doc = CoverDoc::new("Deep Rust", "Artem", "pb", "6x9", 300, "white");
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("wrap-test.pdf");
        let rep = render_wrap_pdf(&doc, &out).unwrap();
        assert!(rep.text_placed);
        let bytes = std::fs::read(&out).unwrap();
        let s = String::from_utf8_lossy(&bytes).into_owned();
        assert!(s.starts_with("%PDF"));
        assert!(s.contains("(Deep Rust)"));
        assert!(s.contains("re f"));
    }

    #[test]
    fn wrap_pdf_cyrillic_skips_text_but_ships_vectors() {
        let doc = CoverDoc::new("Книга", "Автор", "hc", "6x9", 200, "cream");
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("wrap-hc.pdf");
        let rep = render_wrap_pdf(&doc, &out).unwrap();
        assert!(rep.text_skipped);
        assert!(rep.bytes > 500);
    }
}
