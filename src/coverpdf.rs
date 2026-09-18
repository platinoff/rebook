//! RB-16b: cover-wrap PDF via the in-tree PDF writer — Cyrillic (Type0),
//! rotated spine, vector EAN-13. RB-16c JPEG passthrough; RB-46 CMYK JPEG
//! + FOGRA39 ICC drop-in on PDF/X-1a.

use std::path::Path;

use crate::coverdoc::CoverDoc;
use crate::pdfwriter::{
    EmbeddedFont, PdfPageBuilder, PdfxProfile, ink_pct, parse_hex, rgb_to_cmyk,
};
use crate::standards::{BARCODE_ZONE_IN, BLEED_IN, HC_HINGE_IN, HC_WRAP_IN};
use crate::ttf::TtfFont;

/// Export options for the wrap PDF (RB-17: CMYK + PDF/X-1a for Ingram).
#[derive(Debug, Clone, Default)]
pub struct WrapOpts {
    /// Convert all fills to DeviceCMYK (`k` ops).
    pub cmyk: bool,
    /// Emit the PDF/X-1a wrapper (XMP, OutputIntent, boxes).
    pub pdfx: bool,
    /// CMYK ICC profile bytes for `/DestOutputProfile` (full X-1a).
    pub icc: Option<Vec<u8>>,
}

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
    /// CMYK export used.
    pub cmyk: bool,
    /// Max total ink coverage percent (gate: ≤ 240 for Ingram).
    pub ink_max_pct: f64,
    /// RB-16c: JPEG front art embedded as a DCTDecode XObject.
    pub image_placed: bool,
    /// RB-16c: front_image present but not embeddable (PNG/WebP or RGB-in-CMYK).
    pub image_skipped: bool,
    /// RB-46: CMYK ICC attached as `/DestOutputProfile`.
    pub icc_attached: bool,
}

/// Fill a color as RGB or CMYK (RB-17), tracking worst-case ink.
fn fill(page: &mut PdfPageBuilder, hex: &str, cmyk: bool, ink: &mut f64) {
    let (r, g, b) = parse_hex(hex);
    if cmyk {
        let (c, m, y, k) = rgb_to_cmyk(r, g, b);
        *ink = (*ink).max(ink_pct(c, m, y, k));
        page.set_fill_cmyk(c, m, y, k);
    } else {
        *ink = (*ink).max(ink_pct(r, g, b, 0.0));
        page.set_fill(r, g, b);
    }
}

fn is_latin1(s: &str) -> bool {
    s.chars().all(|c| (c as u32) <= 0xFF && c != '\n')
}

/// Render the wrap page for `doc` into `out_path`.
pub fn render_wrap_pdf(doc: &CoverDoc, out_path: &Path) -> Result<CoverPdfReport, String> {
    render_wrap_pdf_opts(doc, out_path, &WrapOpts::default())
}

/// RB-17: full-control export — RGB/CMYK fills, PDF/X-1a wrapper with an
/// optional CMYK ICC (`REBOOK_ICC_CMYK` env or `opts.icc`).
pub fn render_wrap_pdf_opts(
    doc: &CoverDoc,
    out_path: &Path,
    opts: &WrapOpts,
) -> Result<CoverPdfReport, String> {
    let (w, h) = doc.canvas_in()?;
    let pt = |v: f64| v * 72.0;
    // PDF origin is bottom-left; CoverDoc y is top-down.
    let yb = |y_top_in: f64| pt(h - y_top_in);
    let mut page = PdfPageBuilder::new(pt(w), pt(h));

    let ebook = doc.mode == "ebook";
    let mut ink_max = 0.0f64;
    let cmyk = opts.cmyk;
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
    fill(&mut page, &doc.bg_back, cmyk, &mut ink_max);
    page.rect(0.0, 0.0, pt(w), pt(h));
    if !ebook {
        fill(&mut page, &doc.bg_front, cmyk, &mut ink_max);
        page.rect(pt(edge + trim_w + spine), 0.0, pt(trim_w), pt(h));
        let spine_fill = doc.spine_bg.clone().unwrap_or_else(|| doc.bg_front.clone());
        fill(&mut page, &spine_fill, cmyk, &mut ink_max);
        page.rect(pt(edge + trim_w), 0.0, pt(spine), pt(h));
    } else {
        fill(&mut page, &doc.bg_front, cmyk, &mut ink_max);
        page.rect(0.0, 0.0, pt(w), pt(h));
    }
    if !ebook && mode == crate::cover::Mode::CaseLaminate {
        if cmyk {
            page.set_fill_cmyk(0.0, 0.0, 0.0, 0.45);
        } else {
            page.set_fill(0.55, 0.55, 0.55);
        }
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
        if cmyk {
            page.set_fill_cmyk(0.0, 0.0, 0.0, 1.0);
            ink_max = ink_max.max(100.0);
        } else {
            page.set_fill(0.0, 0.0, 0.0);
        }
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
        if cmyk {
            page.set_fill_cmyk(0.0, 0.0, 0.0, 1.0);
        } else {
            page.set_fill(0.0, 0.0, 0.0);
        }
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

    // RB-16c / RB-46: JPEG passthrough. RGB JPEGs stay DeviceRGB (KDP).
    // CMYK JPEGs (SOF nf=4) ride X-1a as DeviceCMYK. PNG/WebP still skip.
    if let Some(uri) = doc.front_image.as_deref() {
        let b64 = uri.split_once("base64,").map(|(_, r)| r).unwrap_or("");
        match crate::drafts::b64_decode(b64) {
            Ok(bytes) if bytes.starts_with(&[0xFF, 0xD8]) => {
                match crate::preflight::jpeg_info(&bytes) {
                    Some(info) => {
                        let is_cmyk = info.space == crate::preflight::JpegSpace::Cmyk;
                        if cmyk != is_cmyk {
                            rep.image_skipped = true;
                        } else {
                            let idx = page.add_image_space(
                                info.w,
                                info.h,
                                bytes,
                                is_cmyk,
                                is_cmyk && info.invert,
                            );
                            if ebook {
                                page.draw_image_cover(idx, 0.0, 0.0, pt(w), pt(h));
                            } else {
                                page.draw_image_cover(
                                    idx,
                                    pt(edge + trim_w + spine),
                                    0.0,
                                    pt(trim_w),
                                    pt(h),
                                );
                            }
                            rep.image_placed = true;
                        }
                    }
                    None => rep.image_skipped = true,
                }
            }
            _ => rep.image_skipped = true,
        }
    }

    // text layers — centre front panel, white; spine rotated 90°
    let front_cx = if ebook {
        w / 2.0
    } else {
        edge + trim_w + spine + trim_w / 2.0
    };
    if cmyk {
        page.set_fill_cmyk(0.0, 0.0, 0.0, 0.0);
    } else {
        page.set_fill(1.0, 1.0, 1.0);
    }
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
    rep.cmyk = cmyk;
    rep.ink_max_pct = ink_max;
    if opts.pdfx {
        let icc = opts.icc.clone().or_else(crate::icc::discover_cmyk_icc);
        rep.icc_attached = icc.is_some();
        page.set_pdfx(PdfxProfile {
            condition: "Coated FOGRA39 (ISO 12647-2:2004)".to_string(),
            icc,
            trim: None,
        });
    }

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
    fn wrap_pdf_embeds_jpeg_raster() {
        // real repo JPEG (en/cover_kdp.jpg) → DCTDecode XObject on the front panel
        let jpg = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("en/cover_kdp.jpg"))
            .expect("repo cover jpg");
        let uri = format!(
            "data:image/jpeg;base64,{}",
            crate::preflight::b64_encode(&jpg)
        );
        let mut d = doc_with(
            CoverDoc::new("Raster Test", "A", "pb", "6x9", 300, "white"),
            None,
        );
        d.auto_layout();
        d.front_image = Some(uri.clone());
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("wrap-raster.pdf");
        let rep = render_wrap_pdf(&d, &out).unwrap();
        assert!(rep.image_placed, "JPEG must ride the wrap PDF");
        let bytes = std::fs::read(&out).unwrap();
        let s = String::from_utf8_lossy(&bytes).into_owned();
        assert!(
            s.contains("/DCTDecode") && s.contains("/Im0"),
            "xobject missing"
        );
        assert!(s.contains("/XObject"), "resources must list the image");
        let doc = lopdf::Document::load(&out).expect("lopdf parses raster wrap");
        assert_eq!(doc.get_pages().len(), 1);
        // PNG (the studio upload default) must be reported, not faked:
        let png = crate::preflight::b64_encode(&[0x89, b'P', b'N', b'G', 0, 1, 2, 3, 4]);
        d.front_image = Some(format!("data:image/png;base64,{png}"));
        let rep = render_wrap_pdf(&d, &out).unwrap();
        assert!(
            !rep.image_placed && rep.image_skipped,
            "PNG → convert-to-JPEG notice"
        );
        // CMYK/X-1a stays vector-only (RGB raster would break the output intent):
        d.front_image = Some(uri);
        let rep = render_wrap_pdf_opts(
            &d,
            &out,
            &WrapOpts {
                cmyk: true,
                pdfx: false,
                icc: None,
            },
        )
        .unwrap();
        assert!(
            !rep.image_placed && rep.image_skipped,
            "cmyk must skip RGB raster"
        );
    }

    #[test]
    fn pdfx_attaches_cmyk_icc_and_cmyk_jpeg() {
        let jpg = crate::preflight::stub_cmyk_jpeg(8, 8);
        let uri = format!(
            "data:image/jpeg;base64,{}",
            crate::preflight::b64_encode(&jpg)
        );
        let mut d = doc_with(
            CoverDoc::new("FOGRA drop-in", "A", "pb", "6x9", 300, "white"),
            None,
        );
        d.auto_layout();
        d.front_image = Some(uri);
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("wrap-cmyk-jpeg.pdf");
        let icc = crate::icc::minimal_cmyk_icc();
        let rep = render_wrap_pdf_opts(
            &d,
            &out,
            &WrapOpts {
                cmyk: true,
                pdfx: true,
                icc: Some(icc.clone()),
            },
        )
        .unwrap();
        assert!(rep.image_placed, "CMYK JPEG must passthrough on X-1a");
        assert!(rep.icc_attached);
        let s = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
        assert!(s.contains("/DeviceCMYK"), "image + ICC alternate");
        assert!(s.contains("/DCTDecode"));
        assert!(s.contains("/Decode [1 0 1 0 1 0 1 0]"), "Adobe invert");
        assert!(s.contains("/DestOutputProfile"), "ICC linked from OI");
        assert!(s.contains("/N 4"));
        assert!(s.contains("GTS_PDFXVersion"));
        // drop-in file path is accepted by discover:
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("icc-rb46");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("FOGRA39.icc");
        std::fs::write(&p, &icc).unwrap();
        assert!(crate::icc::discover_cmyk_icc_in(&[p]).is_some());
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
    fn pdfx_cmyk_wrapper_is_pdf_x_1a() {
        let d = doc_with(
            CoverDoc::new("Ingram Test", "A", "hc", "6x9", 200, "cream"),
            Some("978-3-16-148410-0"),
        );
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("wrap-x.pdf");
        let rep = render_wrap_pdf_opts(
            &d,
            &out,
            &WrapOpts {
                cmyk: true,
                pdfx: true,
                icc: None,
            },
        )
        .unwrap();
        assert!(rep.cmyk);
        assert!(rep.ink_max_pct <= 240.0, "ink {}", rep.ink_max_pct);
        let s = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
        assert!(s.contains("/OutputIntents"), "catalog OI");
        assert!(s.contains("GTS_PDFXVersion"), "XMP conformance");
        assert!(s.contains("PDF/X-1a:2001"));
        assert!(s.contains("/TrimBox"), "page boxes");
        assert!(s.contains(" k\n"), "CMYK fill ops");
        assert!(!s.contains(" rg\n"), "no RGB fills in cmyk mode");
    }

    #[test]
    fn ink_and_color_helpers() {
        // naive conversion stays under the Ingram TAC gate
        let (c, m, y, k) = crate::pdfwriter::rgb_to_cmyk(0.1, 0.2, 0.3);
        assert!(crate::pdfwriter::ink_pct(c, m, y, k) <= 240.0);
        assert_eq!(
            crate::pdfwriter::rgb_to_cmyk(0.0, 0.0, 0.0),
            (0.0, 0.0, 0.0, 1.0)
        );
        assert_eq!(
            crate::pdfwriter::rgb_to_cmyk(1.0, 1.0, 1.0),
            (0.0, 0.0, 0.0, 0.0)
        );
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
