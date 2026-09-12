//! Cover Studio document model (`cover.json`) and compositor.
//!
//! A [`CoverDoc`] describes mode/trim/pages/paper plus a small, Canva-like
//! layer stack (backgrounds, one optional front image, title/author/spine
//! text). [`auto_layout`] derives positions from book metadata + the
//! [`crate::standards`] safe zones (spine text only when pages > 79, content
//! ≥ 0.25″ from trim). [`render_svg`] emits final artwork — no guide lines —
//! at 300 DPI; PNG export happens client-side (canvas) in `/cover`.

use serde::{Deserialize, Serialize};

use crate::cover::{Mode, template};
use crate::standards::{
    BLEED_IN, HARDCOVER_TRIMS, HC_WRAP_IN, PAPERBACK_TRIMS, Paper, Trim, ebook_cover_ok,
    even_pages, find_trim, spine_text_allowed,
};

/// One text layer; x/y are the text-center in file inches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextLayer {
    /// Text content.
    pub text: String,
    /// Center x, inches from the file's left edge.
    pub x: f64,
    /// Baseline-ish center y, inches from the top.
    pub y: f64,
    /// Font size in points (1pt = 1/72in).
    pub pt: f64,
    /// Fill (any CSS color).
    pub color: String,
    /// True for the vertical spine line.
    #[serde(default)]
    pub spine: bool,
}

/// The cover document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverDoc {
    /// `pb` | `hc` | `dj` | `ebook`.
    pub mode: String,
    /// Trim label (`6x9`); ebook ignores it (fixed 1600×2560).
    #[serde(default = "default_trim")]
    pub trim: String,
    /// Interior page count (drives spine width + spine-text rule).
    #[serde(default = "default_pages")]
    pub pages: u32,
    /// `white` | `cream` | `ground` | `premium` (spine math only).
    #[serde(default)]
    pub paper: String,
    /// Front panel fill.
    #[serde(default = "default_color")]
    pub bg_front: String,
    /// Back panel fill.
    #[serde(default = "default_color")]
    pub bg_back: String,
    /// Spine fill (defaults to `bg_front`).
    #[serde(default)]
    pub spine_bg: Option<String>,
    /// Optional front artwork as a `data:` URI (png/jpg/webp/svg) placed
    /// full-front under the text layers.
    #[serde(default)]
    pub front_image: Option<String>,
    /// ISBN for the cover barcode (RB-16b; None = zone left clear).
    #[serde(default)]
    pub isbn: Option<String>,
    /// Title layer (front).
    pub title: TextLayer,
    /// Author layer (front, bottom).
    pub author: TextLayer,
    /// Optional spine text (auto-gated by `pages > 79`).
    #[serde(default)]
    pub spine_title: Option<TextLayer>,
}

fn default_trim() -> String {
    "6x9".to_string()
}
fn default_pages() -> u32 {
    300
}
fn default_color() -> String {
    "#1d2733".to_string()
}

impl CoverDoc {
    /// Fresh doc from book metadata with auto-layout applied.
    pub fn new(
        title: &str,
        author: &str,
        mode: &str,
        trim: &str,
        pages: u32,
        paper: &str,
    ) -> CoverDoc {
        let mut doc = CoverDoc {
            mode: mode.to_string(),
            trim: trim.to_string(),
            pages,
            paper: paper.to_string(),
            bg_front: default_color(),
            bg_back: default_color(),
            spine_bg: None,
            front_image: None,
            isbn: None,
            title: TextLayer {
                text: title.into(),
                x: 0.0,
                y: 0.0,
                pt: 36.0,
                color: "#ffffff".into(),
                spine: false,
            },
            author: TextLayer {
                text: format!("by {author}"),
                x: 0.0,
                y: 0.0,
                pt: 18.0,
                color: "#e6e6e6".into(),
                spine: false,
            },
            spine_title: None,
        };
        doc.auto_layout();
        doc
    }

    /// File geometry (inches) for the doc's canvas.
    pub fn canvas_in(&self) -> Result<(f64, f64), String> {
        if self.mode == "ebook" {
            return Ok((5.3333, 8.5333)); // 1600×2560 @300DPI
        }
        let (mode, table) = self.parse_mode()?;
        let trim = find_trim(table, &self.trim)
            .ok_or_else(|| format!("trim {} not available for {}", self.trim, mode.tag()))?;
        let t = template(trim, self.pages, self.paper_enum(), mode)?;
        Ok((t.size.w, t.size.h))
    }

    pub(crate) fn parse_mode(&self) -> Result<(Mode, &'static [Trim]), String> {
        match self.mode.as_str() {
            "pb" => Ok((Mode::Paperback, PAPERBACK_TRIMS)),
            "hc" => Ok((Mode::CaseLaminate, HARDCOVER_TRIMS)),
            "dj" => Ok((Mode::DustJacket, PAPERBACK_TRIMS)),
            other => Err(format!("unknown cover mode {other} (pb|hc|dj|ebook)")),
        }
    }

    fn paper_enum(&self) -> Paper {
        match self.paper.as_str() {
            "cream" => Paper::Cream,
            "ground" => Paper::Groundwood,
            "premium" => Paper::PremiumColor,
            _ => Paper::White,
        }
    }

    /// Derive text positions from canvas + safe zones. Spine text only when
    /// [`spine_text_allowed`].
    pub fn auto_layout(&mut self) {
        let pages = even_pages(self.pages);
        if let Ok((w, h)) = self.canvas_in() {
            if self.mode == "ebook" {
                self.title.x = w / 2.0;
                self.title.y = h * 0.30;
                self.author.x = w / 2.0;
                self.author.y = h * 0.52;
                return;
            }
            let (mode, _) = self
                .parse_mode()
                .unwrap_or((Mode::Paperback, PAPERBACK_TRIMS));
            let edge = if mode == Mode::CaseLaminate {
                HC_WRAP_IN
            } else {
                BLEED_IN
            };
            let table = if mode == Mode::CaseLaminate {
                HARDCOVER_TRIMS
            } else {
                PAPERBACK_TRIMS
            };
            let trim = match find_trim(table, &self.trim) {
                Some(t) => *t,
                None => return,
            };
            let spine = self.spine_width_in();
            // front panel spans the right-hand side.
            let front_x = w - edge - trim.w / 2.0;
            self.title.x = front_x;
            self.title.y = edge + trim.h * 0.32;
            self.author.x = front_x;
            self.author.y = edge + trim.h * 0.86;
            if spine_text_allowed(pages) {
                self.spine_title = Some(TextLayer {
                    text: self.title.text.clone(),
                    x: edge + trim.w + spine / 2.0,
                    y: h / 2.0,
                    pt: (spine * 72.0 * 0.32).clamp(6.0, 14.0),
                    color: self.title.color.clone(),
                    spine: true,
                });
            } else {
                self.spine_title = None;
            }
        }
    }

    pub(crate) fn spine_width_in(&self) -> f64 {
        let paper = self.paper_enum();
        let pages = even_pages(self.pages);
        match self.mode.as_str() {
            "hc" => crate::standards::hardcover_spine_approx(pages, paper),
            _ => crate::standards::spine_width(pages, paper),
        }
    }

    /// Compose final artwork SVG (no guides).
    pub fn render_svg(&self) -> Result<String, String> {
        let (w, h) = self.canvas_in()?;
        let px = |v: f64| (v * crate::standards::DPI).round();
        let (w_px, h_px) = (px(w), px(h));
        let mut s = String::new();
        s.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w_px:.0}\" height=\"{h_px:.0}\" viewBox=\"0 0 {w_px:.0} {h_px:.0}\">\n"
        ));
        s.push_str(&format!(
            "  <desc>rebook cover {} {}p</desc>\n",
            self.trim, self.pages
        ));
        s.push_str(&format!(
            "  <rect width=\"{w_px:.0}\" height=\"{h_px:.0}\" fill=\"{}\"/>\n",
            esc(&self.bg_back)
        ));
        if self.mode != "ebook" {
            let spine = self.spine_width_in();
            let (mode, _) = self.parse_mode()?;
            let edge = if mode == Mode::CaseLaminate {
                HC_WRAP_IN
            } else {
                BLEED_IN
            };
            let table = if mode == Mode::CaseLaminate {
                HARDCOVER_TRIMS
            } else {
                PAPERBACK_TRIMS
            };
            let trim = find_trim(table, &self.trim)
                .ok_or_else(|| format!("trim {} not available", self.trim))?;
            let spine_fill = self
                .spine_bg
                .clone()
                .unwrap_or_else(|| self.bg_front.clone());
            // front panel + spine as rects over the back fill
            s.push_str(&format!(
                "  <rect x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" fill=\"{}\"/>\n",
                px(edge + trim.w + spine),
                px(edge),
                px(trim.w),
                px(h - 2.0 * edge),
                esc(&self.bg_front)
            ));
            s.push_str(&format!(
                "  <rect x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" fill=\"{}\"/>\n",
                px(edge + trim.w),
                px(edge),
                px(spine),
                px(h - 2.0 * edge),
                esc(&spine_fill)
            ));
            if let Some(img) = &self.front_image {
                s.push_str(&format!(
                    "  <image href=\"{img}\" x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" preserveAspectRatio=\"xMidYMid slice\"/>\n",
                    px(edge + trim.w + spine), px(edge), px(trim.w), px(h - 2.0 * edge)
                ));
            }
        } else if let Some(img) = &self.front_image {
            s.push_str(&format!(
                "  <image href=\"{img}\" x=\"0\" y=\"0\" width=\"{w_px:.0}\" height=\"{h_px:.0}\" preserveAspectRatio=\"xMidYMid slice\"/>\n"
            ));
        }
        for layer in [
            Some(&self.title),
            Some(&self.author),
            self.spine_title.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            s.push_str(&render_text(px, layer));
        }
        s.push_str("</svg>\n");
        Ok(s)
    }

    /// JSON body for `cover.json`.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

fn render_text(px: impl Fn(f64) -> f64, l: &TextLayer) -> String {
    let font_px = l.pt * crate::standards::DPI / 72.0;
    if l.spine {
        format!(
            "  <text x=\"{:.0}\" y=\"{:.0}\" transform=\"rotate(-90 {:.0} {:.0})\" text-anchor=\"middle\" font-family=\"Georgia, serif\" font-size=\"{font_px:.0}\" fill=\"{}\">{}</text>\n",
            px(l.x),
            px(l.y),
            px(l.x),
            px(l.y),
            esc(&l.color),
            esc(&l.text)
        )
    } else {
        format!(
            "  <text x=\"{:.0}\" y=\"{:.0}\" text-anchor=\"middle\" font-family=\"Georgia, serif\" font-size=\"{font_px:.0}\" fill=\"{}\">{}</text>\n",
            px(l.x),
            px(l.y),
            esc(&l.color),
            esc(&l.text)
        )
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Validate an ebook cover image size (delegates to standards limits).
pub fn ebook_size_ok(w_px: u32, h_px: u32) -> bool {
    ebook_cover_ok(w_px, h_px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_layout_pb_ebook_and_spine_gate() {
        let mut d = CoverDoc::new("Deep Rust", "Artem", "pb", "6x9", 300, "white");
        let (w, h) = d.canvas_in().unwrap();
        assert!((w - 12.9256).abs() < 1e-3);
        assert!((h - 9.25).abs() < 1e-3);
        assert!((d.title.x - (w - 0.125 - 3.0)).abs() < 0.01); // centered on the front panel
        assert!(d.spine_title.is_some()); // 300 > 79
        d.pages = 60;
        d.auto_layout();
        assert!(d.spine_title.is_none()); // no spine text under the rule
        let e = CoverDoc::new("T", "A", "ebook", "6x9", 300, "white");
        let (ew, eh) = e.canvas_in().unwrap();
        assert!((ew * 300.0 - 1600.0).abs() < 1.0 && (eh * 300.0 - 2560.0).abs() < 1.0);
    }

    #[test]
    fn render_contains_layers_and_geometry() {
        let d = CoverDoc::new("Книга & Тест", "Автор", "pb", "6x9", 300, "white");
        let svg = d.render_svg().unwrap();
        assert!(svg.contains("viewBox=\"0 0 3878 2775\""));
        assert!(svg.contains("Книга &amp; Тест"));
        assert!(svg.contains("Автор"));
        assert!(svg.contains("rotate(-90"));
        assert!(!svg.contains("stroke-dasharray")); // artwork, not guides
    }

    #[test]
    fn json_roundtrip() {
        let d = CoverDoc::new("R", "A", "hc", "6x9", 120, "cream");
        let j = d.to_json();
        let back: CoverDoc = serde_json::from_str(&j).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn bad_mode_errors() {
        let mut d = CoverDoc::new("R", "A", "pb", "6x9", 300, "white");
        d.mode = "vinyl".into();
        assert!(d.render_svg().is_err());
    }

    #[test]
    fn ebook_gate_delegates() {
        assert!(ebook_size_ok(1600, 2560));
        assert!(!ebook_size_ok(800, 800));
    }
}
