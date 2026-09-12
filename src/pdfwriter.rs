//! RB-16 (route B): a minimal, dependency-free PDF writer — just enough of
//! ISO 32000 for print artwork: custom-size pages, filled rectangles (`re f`),
//! thick vector lines, and base-14 Helvetica text. No compression, single
//! font resource, deterministic output (byte-stable xref).
//!
//! Cyrillic text needs embedded TrueType (CID/Type0) — deferred to RB-16b;
//! until then [`text`] writes Latin-1 only and reports `skipped` for the rest.

use std::io::Write as _;

/// A page being built: ops accumulate as a content-stream `Vec<u8>`.
#[derive(Debug, Clone)]
pub struct PdfPageBuilder {
    /// Page width, points (72/inch).
    pub w_pt: f64,
    /// Page height, points.
    pub h_pt: f64,
    ops: Vec<u8>,
}

impl PdfPageBuilder {
    /// New page with size in points.
    pub fn new(w_pt: f64, h_pt: f64) -> PdfPageBuilder {
        PdfPageBuilder {
            w_pt,
            h_pt,
            ops: Vec::new(),
        }
    }

    /// Set non-stroking (fill) color, sRGB components 0..=1.
    pub fn set_fill(&mut self, r: f64, g: f64, b: f64) {
        writeln!(self.ops, "{:.4} {:.4} {:.4} rg", cl(r), cl(g), cl(b)).ok();
    }

    /// Set stroking color.
    pub fn set_stroke(&mut self, r: f64, g: f64, b: f64) {
        writeln!(self.ops, "{:.4} {:.4} {:.4} RG", cl(r), cl(g), cl(b)).ok();
    }

    /// Filled rectangle, coordinates in points from the page's bottom-left.
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        writeln!(self.ops, "{:.4} {:.4} {:.4} {:.4} re f", x, y, w, h).ok();
    }

    /// Stroked line with width (pt).
    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, width_pt: f64) {
        writeln!(self.ops, "{:.4} w", width_pt).ok();
        writeln!(self.ops, "{:.4} {:.4} m {:.4} {:.4} l S", x1, y1, x2, y2).ok();
    }

    /// Helvetica text at (x, y) pt, size pt, current fill color. Returns
    /// `false` (and writes nothing) if the string has non-Latin-1 chars.
    pub fn text(&mut self, x: f64, y: f64, size: f64, s: &str) -> bool {
        if !s.chars().all(|c| (c as u32) <= 0xFF) {
            return false;
        }
        let esc: String = s
            .chars()
            .flat_map(|c| match c {
                '(' | ')' | '\\' => vec!['\\', c],
                c if (c as u32) > 126 && (c as u32) <= 0xFF => {
                    vec![
                        char::from(b'0' + (c as u32 / 64) as u8),
                        char::from(b'0' + ((c as u32 / 8) % 8) as u8),
                        char::from(b'0' + (c as u32 % 8) as u8),
                    ]
                }
                c => vec![c],
            })
            .collect();
        writeln!(
            self.ops,
            "BT /F1 {:.4} Tf 1 0 0 1 {:.4} {:.4} Tm ({}) Tj ET",
            size, x, y, esc
        )
        .ok();
        true
    }

    /// Finish: bytes of the whole PDF document.
    pub fn build(self, title: &str) -> Vec<u8> {
        let content = self.ops;
        // object numbers: 1 catalog, 2 pages, 3 page, 4 contents, 5 font
        let mut out = Vec::new();
        out.extend_from_slice(b"%PDF-1.4\n");
        let mut offsets = Vec::with_capacity(5);
        let obj = |buf: &mut Vec<u8>, offs: &mut Vec<usize>, n: u32, body: &[u8]| {
            offs.push(buf.len());
            writeln!(buf, "{n} 0 obj").ok();
            buf.extend_from_slice(body);
            buf.extend_from_slice(b"\nendobj\n");
        };
        obj(
            &mut out,
            &mut offsets,
            1,
            b"<< /Type /Catalog /Pages 2 0 R >>",
        );
        obj(
            &mut out,
            &mut offsets,
            2,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        let page = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.4} {:.4}] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
            self.w_pt, self.h_pt
        );
        obj(&mut out, &mut offsets, 3, page.as_bytes());
        let stream_hdr = format!("<< /Length {} >>\nstream\n", content.len());
        offsets.push(out.len());
        writeln!(out, "4 0 obj").ok();
        out.extend_from_slice(stream_hdr.as_bytes());
        out.extend_from_slice(&content);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        obj(
            &mut out,
            &mut offsets,
            5,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        );
        let info = {
            let t = title
                .chars()
                .filter(|c| *c != '(' && *c != ')' && *c != '\\')
                .collect::<String>();
            format!("<< /Title ({t}) /Producer (rebook pdfwriter) >>")
        };
        offsets.push(out.len());
        write!(out, "6 0 obj\n{info}\nendobj\n").ok();
        let xref_pos = out.len();
        write!(out, "xref\n0 7\n0000000000 65535 f \n").ok();
        for off in offsets {
            writeln!(out, "{off:010} 00000 n ").ok();
        }
        write!(
            out,
            "trailer\n<< /Size 7 /Root 1 0 R /Info 6 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n"
        )
        .ok();
        out
    }
}

fn cl(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

/// Parse `#rgb`/`#rrggbb` hex; falls back to `(fallback)`.
pub fn parse_hex(hex: &str) -> (f64, f64, f64) {
    let h = hex.trim().trim_start_matches('#');
    let full = match h.len() {
        3 => format!(
            "{}{}{}{}{}{}",
            &h[0..1],
            &h[0..1],
            &h[1..2],
            &h[1..2],
            &h[2..3],
            &h[2..3]
        ),
        6 => h.to_string(),
        _ => return (0.11, 0.15, 0.20),
    };
    let byte = |i: usize| {
        u8::from_str_radix(&full[i * 2..i * 2 + 2], 16)
            .map(|b| b as f64 / 255.0)
            .unwrap_or(0.0)
    };
    (byte(0), byte(1), byte(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_are_a_valid_lopdf_shape() {
        let mut p = PdfPageBuilder::new(612.0, 792.0);
        p.set_fill(0.1, 0.2, 0.3);
        p.rect(0.0, 0.0, 612.0, 792.0);
        assert!(p.text(100.0, 400.0, 18.0, "Hello (World)"));
        assert!(!p.text(100.0, 380.0, 12.0, "Кирилиця"));
        p.set_fill(0.0, 0.0, 0.0);
        p.line(10.0, 10.0, 10.0, 30.0, 2.0);
        let bytes = p.build("Test");
        let s = String::from_utf8_lossy(&bytes).into_owned();
        assert!(s.starts_with("%PDF-1.4"));
        assert!(s.contains("/MediaBox [0 0 612.0000 792.0000]"));
        assert!(s.contains("(Hello \\(World\\))"));
        assert!(!s.contains("Кирилиця"));
        assert!(s.ends_with("%%EOF\n"));
        // xref offset sanity: startxref points inside the file.
        let pos = s.rfind("startxref").unwrap();
        let off: usize = s[pos + 9..]
            .trim_start()
            .lines()
            .next()
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(off < bytes.len());
    }

    #[test]
    fn hex_parse() {
        assert_eq!(parse_hex("#ffffff"), (1.0, 1.0, 1.0));
        assert_eq!(parse_hex("#000"), (0.0, 0.0, 0.0));
        let (r, _, _) = parse_hex("#1d2733");
        assert!((r - 29.0 / 255.0).abs() < 1e-6);
        assert_eq!(parse_hex("garbage"), (0.11, 0.15, 0.20));
    }
}
