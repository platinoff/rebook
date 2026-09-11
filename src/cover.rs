//! Full-wrap cover templates from [`crate::standards`]: guides-only SVG at
//! print resolution (300 DPI), three modes:
//!
//! - [`Mode::Paperback`] — KDP perfect bound (bleed 0.125″, back|spine|front).
//! - [`Mode::CaseLaminate`] — KDP hardcover (wrap 0.51″, hinge 0.4″ dead zones).
//! - [`Mode::DustJacket`] — Ingram-style jacket (flaps 3.25″, folds 0.25″,
//!   head/tail fold allowance 0.75″ — verify against the generated Ingram
//!   template before final art).
//!
//! Coordinates: viewBox in pixels at [`crate::standards::DPI`], 1 inch =
//! `DPI` px. Fold/trim/bleed guides are dashed; panels are labeled.

use crate::standards::{
    BLEED_IN, CoverInches, HC_HINGE_IN, HC_SAFE_IN, HC_WRAP_IN, Paper, Trim, even_pages,
    hardcover_cover, hardcover_spine_approx, paperback_cover, spine_width,
};

/// What kind of print cover the template is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// KDP paperback (perfect bound), full wrap.
    Paperback,
    /// KDP case-laminate hardcover, full wrap incl. board wrap margins.
    CaseLaminate,
    /// Dust jacket with flaps (IngramSpark-style geometry).
    DustJacket,
}

impl Mode {
    /// Parse `pb` / `hc` / `dj` (long names accepted too).
    pub fn parse(s: &str) -> Option<Mode> {
        match s.to_ascii_lowercase().as_str() {
            "pb" | "paperback" => Some(Mode::Paperback),
            "hc" | "case" | "case-laminate" => Some(Mode::CaseLaminate),
            "dj" | "jacket" => Some(Mode::DustJacket),
            _ => None,
        }
    }

    /// Stable mode tag for file names.
    pub const fn tag(self) -> &'static str {
        match self {
            Mode::Paperback => "pb",
            Mode::CaseLaminate => "hc",
            Mode::DustJacket => "dj",
        }
    }

    /// Product-folder name for the Shelf area.
    pub const fn dir_name(self) -> &'static str {
        match self {
            Mode::Paperback => "paperback",
            Mode::CaseLaminate => "hardcover",
            Mode::DustJacket => "jacket",
        }
    }
}

/// Dust-jacket flap depth, inches (Ingram FCG).
pub const DJ_FLAP_IN: f64 = 3.25;
/// Dust-jacket fold allowance, inches.
pub const DJ_FOLD_IN: f64 = 0.25;
/// Jacket head/tail fold-over onto the boards, inches (approximate).
pub const DJ_HEADTAIL_IN: f64 = 0.75;

/// A computed template: file size plus the x positions (inches from the
/// file's left edge) of the spine fold lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Template {
    /// Cover mode.
    pub mode: Mode,
    /// Trim used.
    pub trim: Trim,
    /// Pages rounded up to even.
    pub pages: u32,
    /// Paper used for the spine math.
    pub paper: Paper,
    /// Full file size in inches.
    pub size: CoverInches,
    /// Spine width in inches.
    pub spine: f64,
    /// x of the back|spine fold (inches).
    pub fold_back_spine: f64,
    /// x of the spine|front fold (inches).
    pub fold_spine_front: f64,
}

/// Compute a template; `Err` when pages are out of range for the mode.
pub fn template(trim: &Trim, pages: u32, paper: Paper, mode: Mode) -> Result<Template, String> {
    let (size, spine) = match mode {
        Mode::Paperback => {
            let s = paperback_cover(trim, pages, paper)?;
            (s, spine_width(pages, paper))
        }
        Mode::CaseLaminate => {
            let s = hardcover_cover(trim, pages, paper)?;
            (s, hardcover_spine_approx(pages, paper))
        }
        Mode::DustJacket => {
            if trim.h < 7.0 {
                return Err(
                    "dust jacket needs a trim of at least 7in height (Ingram limits)".to_string(),
                );
            }
            let spine = spine_width(pages, paper);
            let w = 2.0 * BLEED_IN + 2.0 * (DJ_FLAP_IN + DJ_FOLD_IN) + 2.0 * trim.w + spine;
            let h = trim.h + 2.0 * DJ_HEADTAIL_IN + 2.0 * BLEED_IN;
            (CoverInches { w, h }, spine)
        }
    };
    let fold_back_spine = match mode {
        Mode::Paperback => BLEED_IN + trim.w,
        Mode::CaseLaminate => HC_WRAP_IN + trim.w,
        Mode::DustJacket => BLEED_IN + DJ_FLAP_IN + DJ_FOLD_IN + trim.w,
    };
    Ok(Template {
        mode,
        trim: *trim,
        pages: even_pages(pages),
        paper,
        size,
        spine,
        fold_back_spine,
        fold_spine_front: fold_back_spine + spine,
    })
}

const BLEED_COLOR: &str = "#e04040";
const TRIM_COLOR: &str = "#2f6fd0";
const FOLD_COLOR: &str = "#7a7a7a";
const SAFE_COLOR: &str = "#2e9e44";
const BARCODE_COLOR: &str = "#c02020";

/// Render a guides-only SVG for a template.
pub fn template_svg(t: &Template) -> String {
    template_svg_with_isbn(t, None)
}

/// As [`template_svg`], but when `isbn` is given the EAN-13 barcode is
/// embedded into the reserved zone (100 %K on white) instead of the hint.
pub fn template_svg_with_isbn(t: &Template, isbn: Option<&str>) -> String {
    let px = |v: f64| (v * crate::standards::DPI).round();
    let w_px = px(t.size.w);
    let h_px = px(t.size.h);
    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w_px}\" height=\"{h_px}\" viewBox=\"0 0 {w_px} {h_px}\">\n"
    ));
    s.push_str(&format!(
        "  <desc>rebook cover template {} {}p {:?} {:.4}x{:.4}in @300DPI (spine {:.4}in)</desc>\n",
        t.trim.label, t.pages, t.paper, t.size.w, t.size.h, t.spine
    ));
    s.push_str("  <style>text{font-family:sans-serif;fill:#666} .lbl{font-size:24px} .dim{font-size:20px;fill:#999}</style>\n");
    s.push_str(&format!(
        "  <rect width=\"{w_px}\" height=\"{h_px}\" fill=\"#ffffff\"/>\n"
    ));

    // outer bleed / wrap guide + inner trim outline
    let (edge_in, inner_w, inner_h) = match t.mode {
        Mode::Paperback => (BLEED_IN, 2.0 * t.trim.w + t.spine, t.trim.h),
        Mode::CaseLaminate => (HC_WRAP_IN, 2.0 * t.trim.w + t.spine, t.trim.h),
        Mode::DustJacket => (
            BLEED_IN,
            t.size.w - 2.0 * BLEED_IN,
            t.size.h - 2.0 * BLEED_IN,
        ),
    };
    guide_rect(
        &mut s,
        (0.0, 0.0, t.size.w, t.size.h),
        1.0,
        4.0,
        BLEED_COLOR,
    );
    guide_rect(
        &mut s,
        (edge_in, edge_in, inner_w, inner_h),
        2.0,
        0.0,
        TRIM_COLOR,
    );

    // hardcover: hinge dead zones beside the spine + content-safe frame
    if t.mode == Mode::CaseLaminate {
        for x0 in [t.fold_back_spine - HC_HINGE_IN, t.fold_spine_front] {
            s.push_str(&format!(
                "  <rect x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" fill=\"#b8b8b8\" opacity=\"0.5\"/>\n",
                px(x0), px(HC_WRAP_IN), px(HC_HINGE_IN), px(t.trim.h)
            ));
        }
        guide_rect(
            &mut s,
            (
                HC_SAFE_IN,
                HC_SAFE_IN,
                t.size.w - 2.0 * HC_SAFE_IN,
                t.size.h - 2.0 * HC_SAFE_IN,
            ),
            2.0,
            6.0,
            SAFE_COLOR,
        );
    }

    let top = edge_in;
    let bot = t.size.h - edge_in;
    fold(&mut s, t.fold_back_spine, top, bot);
    fold(&mut s, t.fold_spine_front, top, bot);
    if t.mode == Mode::DustJacket {
        fold(&mut s, BLEED_IN + DJ_FLAP_IN, top, bot);
        fold(&mut s, t.size.w - BLEED_IN - DJ_FLAP_IN, top, bot);
    }

    // spine center line + panel labels
    let cx = t.fold_back_spine + t.spine / 2.0;
    s.push_str(&format!(
        "  <line x1=\"{:.0}\" y1=\"{:.0}\" x2=\"{:.0}\" y2=\"{:.0}\" stroke=\"{FOLD_COLOR}\" stroke-width=\"1\" stroke-dasharray=\"10,6\" opacity=\"0.8\"/>\n",
        px(cx), px(top), px(cx), px(bot)
    ));
    label(&mut s, t.fold_back_spine / 2.0, "BACK");
    label(&mut s, cx, "SPINE");
    label(&mut s, (t.fold_spine_front + t.size.w) / 2.0, "FRONT");
    if t.mode == Mode::DustJacket {
        label(&mut s, (BLEED_IN + DJ_FLAP_IN) / 2.0, "FLAP");
        label(&mut s, t.size.w - (BLEED_IN + DJ_FLAP_IN) / 2.0, "FLAP");
    }

    // barcode zone: 2.0x1.2in, back panel, 0.25in clear of spine and trim
    let (bw, bh) = crate::standards::BARCODE_ZONE_IN;
    let bx = t.fold_back_spine - 0.25 - bw;
    let by = bot - 0.25 - bh;
    s.push_str(&format!(
        "  <rect x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" fill=\"none\" stroke=\"{BARCODE_COLOR}\" stroke-width=\"2\" stroke-dasharray=\"8,6\"/>\n",
        px(bx), px(by), px(bw), px(bh)
    ));
    match isbn {
        Some(raw) => {
            match crate::barcode::barcode_fragment(raw, px(bx), px(by), px(bw), px(bh)) {
                Ok(g) => s.push_str(&g),
                Err(e) => s.push_str(&format!(
                    "  <text x=\"{:.0}\" y=\"{:.0}\" class=\"dim\" dy=\"-6\">barcode error: {e}</text>\n",
                    px(bx), px(by)
                )),
            }
        }
        None => s.push_str(&format!(
            "  <text x=\"{:.0}\" y=\"{:.0}\" class=\"dim\" dy=\"-6\">barcode 2.0x1.2in (100%K on white)</text>\n",
            px(bx), px(by)
        )),
    }

    s.push_str(&format!(
        "  <text x=\"16\" y=\"28\" class=\"dim\">{:.4} x {:.4} in = {w_px} x {h_px} px @300DPI · spine {:.4} in · pages {}</text>\n",
        t.size.w, t.size.h, t.spine, t.pages
    ));
    s.push_str("</svg>\n");
    s
}

fn fold(s: &mut String, x0: f64, y1: f64, y2: f64) {
    let px = |v: f64| (v * crate::standards::DPI).round();
    s.push_str(&format!(
        "  <line x1=\"{:.0}\" y1=\"{:.0}\" x2=\"{:.0}\" y2=\"{:.0}\" stroke=\"{FOLD_COLOR}\" stroke-width=\"2\" stroke-dasharray=\"14,8\"/>\n",
        px(x0), px(y1), px(x0), px(y2)
    ));
}

fn label(s: &mut String, x0: f64, text: &str) {
    s.push_str(&format!(
        "  <text x=\"{:.0}\" y=\"48\" text-anchor=\"middle\" class=\"lbl\">{text}</text>\n",
        (x0 * crate::standards::DPI).round()
    ));
}

fn guide_rect(s: &mut String, xywh: (f64, f64, f64, f64), stroke_w: f64, dash_px: f64, col: &str) {
    let px = |v: f64| (v * crate::standards::DPI).round();
    let (x0, y0, w, h) = xywh;
    let dash_attr = if dash_px > 0.0 {
        format!(" stroke-dasharray=\"{dash_px:.0},{dash_px:.0}\"")
    } else {
        String::new()
    };
    s.push_str(&format!(
        "  <rect x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" fill=\"none\" stroke=\"{col}\" stroke-width=\"{stroke_w}\"{dash_attr}/>\n",
        px(x0), px(y0), px(w), px(h)
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standards::{HARDCOVER_TRIMS, PAPERBACK_TRIMS, find_trim};

    fn t69() -> &'static Trim {
        find_trim(PAPERBACK_TRIMS, "6x9").unwrap()
    }

    #[test]
    fn paperback_template_matches_std_cover_math() {
        let tpl = template(t69(), 300, Paper::White, Mode::Paperback).unwrap();
        assert!((tpl.size.w - 12.9256).abs() < 1e-4);
        assert!((tpl.size.h - 9.25).abs() < 1e-4);
        assert!((tpl.fold_back_spine - 6.125).abs() < 1e-9);
        assert!((tpl.fold_spine_front - (6.125 + 0.6756)).abs() < 1e-4);
    }

    #[test]
    fn svg_dimensions_are_300dpi() {
        let tpl = template(t69(), 300, Paper::White, Mode::Paperback).unwrap();
        let svg = template_svg(&tpl);
        assert!(svg.contains("width=\"3878\""));
        assert!(svg.contains("height=\"2775\""));
        assert!(svg.contains("viewBox=\"0 0 3878 2775\""));
        assert!(svg.contains(">BACK</text>"));
        assert!(svg.contains(">SPINE</text>"));
        assert!(svg.contains(">FRONT</text>"));
        assert!(svg.contains("barcode 2.0x1.2in"));
    }

    #[test]
    fn case_laminate_and_dust_jacket_build() {
        let hc = find_trim(HARDCOVER_TRIMS, "6x9").unwrap();
        let tpl = template(hc, 200, Paper::White, Mode::CaseLaminate).unwrap();
        assert!((tpl.size.w - (12.0 + 0.5104 + 1.02)).abs() < 1e-3);
        let svg = template_svg(&tpl);
        assert!(svg.contains("#b8b8b8"));
        let dj = template(t69(), 300, Paper::White, Mode::DustJacket).unwrap();
        // 0.25 bleed + 2*(3.25+0.25) flaps/folds + 12 panels + 0.6756 spine
        assert!((dj.size.w - 19.9256).abs() < 1e-3);
        assert!((dj.size.h - (9.0 + 1.5 + 0.25)).abs() < 1e-3);
        let svg = template_svg(&dj);
        assert_eq!(svg.matches(">FLAP</text>").count(), 2);
    }

    #[test]
    fn bad_pages_rejected() {
        assert!(template(t69(), 900, Paper::White, Mode::Paperback).is_err());
        let hc = find_trim(HARDCOVER_TRIMS, "6x9").unwrap();
        assert!(template(hc, 74, Paper::White, Mode::CaseLaminate).is_err());
    }

    #[test]
    fn mode_parse() {
        assert_eq!(Mode::parse("PB"), Some(Mode::Paperback));
        assert_eq!(Mode::parse("hc"), Some(Mode::CaseLaminate));
        assert_eq!(Mode::parse("dj"), Some(Mode::DustJacket));
        assert_eq!(Mode::parse("xx"), None);
        assert_eq!(Mode::CaseLaminate.tag(), "hc");
    }
}
