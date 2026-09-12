//! RB-16b: cover-wrap PDF via the in-tree PDF writer — now with real
//! Cyrillic text (embedded Type0 TrueType), rotated spine text, and vector
//! EAN-13 barcode. Vector-only still: `front_image` rasters are omitted
//! (inline images are RB-16c).

use std::path::Path;

use crate::coverdoc::CoverDoc;
use crate::pdfwriter::{EmbeddedFont, PdfPageBuilder, parse_hex};
use crate::standards::{BARCODE_ZONE_IN, BLEED_IN, HC_HINGE_IN, HC_WRAP_IN};
use crate::ttf::TtfFont;

/// What the renderer managed/omitted.
#[derive(Debug, Clone, Default)]
pub struct CoverPdfReport {
    /// Text layers drawn (Helvetica Latin-1 or embedded TTF).
    pub text_placed: bool,
    /// True when an embedded font carried non-Latin text.
    pub text_embedded: bool,
    /// True when text was skipped entirely (no usable font).
    pub text_skipped: bool,
    /// Barcode vector modules drawn (0 without ISBN).
    pub barcode_bars: usize,
    /// Output bytes.
    pub bytes: usize,
}

fn is_latin1(s: &str) -> bool {
    s.chars().all(|c| (c as u32) <= 0xFF && c != '\n')
}

/// Render the wrap page for `doc` into `out_path`.
pub fn render_wrap_pdf(doc: &CoverDoc, out_path: &Path) -> Result<CoverPdfReport, String> {
    let (w, h) = doc.canvas_in()?;
    let pt = |v: f64| v * 72.0;
    // PDF origin is bottom-left; CoverDoc y is top-down.
    let yb = |y_top_in: f64| pt(h - y_top_in);
    let mut page = PdfPageBuilder::new(pt(w), pt(h));

    let ebook = doc.mode == "ebook";
    let (edge, spine, trim_w) = if ebook {
        (0.0, 0.0, w)
    } else {
        let (mode, _) = doc.parse_mode()?;
        let edge = if mode == crate::cover::Mode::CaseLaminate {
            HC_WRAP_IN
        } else {
            BLEED_IN
        };
        let spine = doc.spine_width_in();
        (edge, spine, (w - 2.0 * edge - spine) / 2.0)
    };
    let mode = doc
        .parse_mode()
        .map(|(m, _)| m)
        .unwrap_or(crate::cover::Mode::Paperback);

    // back = whole sheet, then front panel, then spine
    let (br, bg, bb) = parse_hex(&doc.bg_back);
    page.set_fill(br, bg, bb);
    page.rect(0.0, 0.0, pt(w), pt(h));
    if !ebook {
        let (fr, fg, fb) = parse_hex(&doc.bg_front);
        page.set_fill(fr, fg, fb);
        page.rect(pt(edge + trim_w + spine), 0.0, pt(trim_w), pt(h));
        let spine_fill = doc.spine_bg.clone().unwrap_or_else(|| doc.bg_front.clone());
        let (sr, sg, sb) = parse_hex(spine_fill.as_str());
        page.set_fill(sr, sg, sb);
        page.rect(pt(edge + trim_w), 0.0, pt(spine), pt(h));
    } else {
        let (fr, fg, fb) = parse_hex(&doc.bg_front);
        page.set_fill(fr, fg, fb);
        page.rect(0.0, 0.0, pt(w), pt(h));
    }
    if !ebook && mode == crate::cover::Mode::CaseLaminate {
        page.set_fill(0.55, 0.55, 0.55);
        for x0 in [edge + trim_w - HC_HINGE_IN, edge + trim_w + spine] {
            page.rect(pt(x0), pt(edge), pt(HC_HINGE_IN), pt(h - 2.0 * edge));
        }
    }

    // vector EAN-13 barcode (100%K on white) when the doc carries an ISBN
    let mut rep = CoverPdfReport::default();
    let (bw, bh) = BARCODE_ZONE_IN;
    let bx = edge + trim_w - 0.25 - bw;
    let by_bottom = h - edge - 0.25 - bh;
    if let Some(isbn) = &doc.isbn {
        let bits = crate::barcode::ean13_bits(isbn)?;
        page.set_fill(0.0, 0.0, 0.0);
        let module = pt(bw - 0.5) / 95.0; // quiet zone ~0.25in total
        let x0 = pt(bx + 0.25);
        for (m, b) in bits.chars().enumerate() {
            if b == '1' {
                page.rect(
                    x0 + m as f64 * module,
                    pt(by_bottom + 0.28),
                    module,
                    pt(bh - 0.28),
                );
                rep.barcode_bars += 1;
            }
        }
        // human-readable digits (Latin-1 safe)
        let digits = crate::standards::isbn_to_ean13(isbn)?;
        page.set_fill(0.0, 0.0, 0.0);
        page.text(
            pt(bx + bw / 2.0) - pt(0.55),
            pt(by_bottom) + pt(0.08),
            7.0,
            &digits,
        );
    }

    // embedded TTF for non-Latin text (system Times/Georgia/Arial)
    let font: Option<TtfFont> = crate::interior::discover_fonts()
        .ok()
        .and_then(|f| TtfFont::load(&f.regular).ok());
    let mut embedded_ok = false;
    if let Some(tf) = &font {
        let widths: Vec<i32> = (0..tf.num_glyphs).map(|g| tf.width_1000(g)).collect();
        let scale = 1000.0 / tf.units_per_em as f64;
        page.set_embedded_font(EmbeddedFont {
            base: format!("{}-Rebook", tf.base_font.replace(' ', "")),
            data: tf.data.clone(),
            widths,
            ascent_1000: (tf.ascender as f64 * scale) as i32,
            descent_1000: (tf.descender as f64 * scale) as i32,
            bbox_1000: (
                (tf.bbox.0 as f64 * scale) as i32,
                (tf.bbox.1 as f64 * scale) as i32,
                (tf.bbox.2 as f64 * scale) as i32,
                (tf.bbox.3 as f64 * scale) as i32,
            ),
        });
        embedded_ok = true;
    }

    // text layers — centre front panel, white; spine rotated 90°
    let front_cx = if ebook {
        w / 2.0
    } else {
        edge + trim_w + spine + trim_w / 2.0
    };
    page.set_fill(1.0, 1.0, 1.0);
    let mut placed = true;
    let mut any_embedded = false;
    for (x_in, y_top_in, size, text, _rot90) in [
        (
            front_cx,
            doc.title.y,
            doc.title.pt,
            doc.title.text.as_str(),
            false,
        ),
        (
            front_cx,
            doc.author.y,
            doc.author.pt,
            doc.author.text.as_str(),
            false,
        ),
    ] {
        if text.trim().is_empty() {
            continue;
        }
        if is_latin1(text) {
            placed &= page.text(pt(x_in), yb(y_top_in), size, text);
        } else if embedded_ok {
            let cids: Vec<u16> = text
                .chars()
                .map(|c| font.as_ref().unwrap().glyph(c))
                .collect();
            let run = page.cid_width_pt(&cids, size);
            let x = pt(x_in) - run / 2.0;
            page.text_cid(x, yb(y_top_in), size, &cids, false);
            any_embedded = true;
        } else {
            placed = false;
        }
    }
    if let Some(st) = &doc.spine_title
        && embedded_ok
    {
        let cids: Vec<u16> = st
            .text
            .chars()
            .map(|x| font.as_ref().unwrap().glyph(x))
            .collect();
        let run = page.cid_width_pt(&cids, st.pt);
        let x = pt(edge + trim_w + spine / 2.0) + st.pt / 3.0;
        let y = pt(h / 2.0) - run / 2.0;
        page.text_cid(x, y, st.pt, &cids, true);
        any_embedded = true;
    }
    rep.text_placed = placed;
    rep.text_embedded = any_embedded;
    rep.text_skipped = !placed;

    let bytes = page.build(if doc.title.text.trim().is_empty() {
        "rebook cover"
    } else {
        &doc.title.text
    });
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

    fn doc_with(mut d: CoverDoc, isbn: Option<&str>) -> CoverDoc {
        d.isbn = isbn.map(str::to_string);
        d
    }

    #[test]
    fn wrap_pdf_cyrillic_embeds_ttf() {
        let d = doc_with(
            CoverDoc::new("Книга & Тест", "Автор", "pb", "6x9", 300, "white"),
            None,
        );
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("wrap-cyr.pdf");
        let rep = render_wrap_pdf(&d, &out).unwrap();
        assert!(rep.text_placed, "text must be placed via embedded font");
        assert!(rep.text_embedded);
        let bytes = std::fs::read(&out).unwrap();
        let s = String::from_utf8_lossy(&bytes).into_owned();
        assert!(s.starts_with("%PDF"));
        assert!(s.contains("/Type0"));
        assert!(s.contains("/CIDFontType2"));
        assert!(s.contains("/FontFile2"));
        assert!(s.contains("/Identity-H"));
    }

    #[test]
    fn wrap_pdf_spine_rotation_and_barcode() {
        let mut d = doc_with(
            CoverDoc::new("Deep Rust", "Artem", "hc", "6x9", 200, "cream"),
            Some("978-3-16-148410-0"),
        );
        d.auto_layout();
        assert!(d.spine_title.is_some(), "200 pages ⇒ spine text");
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("wrap-hc2.pdf");
        let rep = render_wrap_pdf(&d, &out).unwrap();
        assert!(
            rep.barcode_bars > 40,
            "barcode modules drawn: {}",
            rep.barcode_bars
        );
        let s = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
        assert!(s.contains("0 1 -1 0"), "rotated Tm for spine text");
    }

    #[test]
    fn latin_still_uses_base14() {
        let d = CoverDoc::new("Plain Title", "A", "ebook", "6x9", 300, "white");
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("wrap-lat.pdf");
        let rep = render_wrap_pdf(&d, &out).unwrap();
        assert!(rep.text_placed);
        // ebook has no spine text → pure base-14
        assert!(!rep.text_embedded);
    }
}
