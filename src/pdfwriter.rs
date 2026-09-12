//! RB-16 (route B): a minimal, dependency-free PDF writer — just enough of
//! ISO 32000 for print artwork: custom-size pages, filled rectangles (`re f`),
//! thick vector lines, and base-14 Helvetica text. No compression, single
//! font resource, deterministic output (byte-stable xref).
//!
//! Cyrillic text needs embedded TrueType (CID/Type0) — deferred to RB-16b;
//! until then [`text`] writes Latin-1 only and reports `skipped` for the rest.

use std::io::Write as _;

/// An embedded TrueType font (CID/Type0, Identity-H).
#[derive(Debug, Clone)]
pub struct EmbeddedFont {
    /// `/BaseFont` name (ASCII, no spaces).
    pub base: String,
    /// Raw TTF bytes for `/FontFile2`.
    pub data: Vec<u8>,
    /// Advance widths per GID, 1/1000 em.
    pub widths: Vec<i32>,
    /// Font descriptor metrics (1/1000 em): ascent, descent, bbox.
    pub ascent_1000: i32,
    pub descent_1000: i32,
    pub bbox_1000: (i32, i32, i32, i32),
}

/// PDF/X-1a wrapper settings for [`PdfPageBuilder::build`].
#[derive(Debug, Clone, Default)]
pub struct PdfxProfile {
    /// `OutputConditionIdentifier`, e.g. Coated FOGRA39 (Ingram).
    pub condition: String,
    /// Optional ICC stream bytes for `/DestOutputProfile` (full X-1a).
    pub icc: Option<Vec<u8>>,
    /// Trim box in points; defaults to the full media box.
    pub trim: Option<(f64, f64, f64, f64)>,
}

/// A page being built: ops accumulate as a content-stream `Vec<u8>`.
#[derive(Debug, Clone)]
pub struct PdfPageBuilder {
    /// Page width, points (72/inch).
    pub w_pt: f64,
    /// Page height, points.
    pub h_pt: f64,
    ops: Vec<u8>,
    embedded: Option<EmbeddedFont>,
    used_cids: std::collections::BTreeSet<u16>,
    pdfx: Option<PdfxProfile>,
}

impl PdfPageBuilder {
    /// New page with size in points.
    pub fn new(w_pt: f64, h_pt: f64) -> PdfPageBuilder {
        PdfPageBuilder {
            w_pt,
            h_pt,
            ops: Vec::new(),
            embedded: None,
            used_cids: Default::default(),
            pdfx: None,
        }
    }

    /// Enable PDF/X-1a document wrapping (catalog, boxes, XMP, OutputIntent).
    pub fn set_pdfx(&mut self, profile: PdfxProfile) {
        self.pdfx = Some(profile);
    }

    /// Set non-stroking (fill) color, sRGB components 0..=1.
    pub fn set_fill(&mut self, r: f64, g: f64, b: f64) {
        writeln!(self.ops, "{:.4} {:.4} {:.4} rg", cl(r), cl(g), cl(b)).ok();
    }

    /// PDF/X-1a fill: DeviceCMYK components 0..=1 (`k` operator).
    pub fn set_fill_cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) {
        writeln!(
            self.ops,
            "{:.4} {:.4} {:.4} {:.4} k",
            cl(c),
            cl(m),
            cl(y),
            cl(k)
        )
        .ok();
    }

    /// PDF/X-1a stroke color.
    pub fn set_stroke_cmyk(&mut self, c: f64, m: f64, y: f64, k: f64) {
        writeln!(
            self.ops,
            "{:.4} {:.4} {:.4} {:.4} K",
            cl(c),
            cl(m),
            cl(y),
            cl(k)
        )
        .ok();
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

    /// Register an embedded TrueType font used as `/F2` (Identity-H).
    pub fn set_embedded_font(&mut self, font: EmbeddedFont) {
        self.embedded = Some(font);
    }

    /// Write glyph-CID hex text (`/F2`), 90°-rotated when `rot90`.
    pub fn text_cid(&mut self, x: f64, y: f64, size: f64, cids: &[u16], rot90: bool) {
        if cids.is_empty() || self.embedded.is_none() {
            return;
        }
        self.used_cids.extend(cids.iter().copied());
        let hex: String = cids.iter().map(|c| format!("{c:04X}")).collect();
        let tm = if rot90 {
            format!("0 1 -1 0 {:.4} {:.4}", x, y)
        } else {
            format!("1 0 0 1 {:.4} {:.4}", x, y)
        };
        writeln!(self.ops, "BT /F2 {:.4} Tf {tm} Tm <{hex}> Tj ET", size).ok();
    }

    /// Width in points of a CID run at `size` (for centering); 0 without font.
    pub fn cid_width_pt(&self, cids: &[u16], size: f64) -> f64 {
        let Some(f) = &self.embedded else { return 0.0 };
        cids.iter()
            .map(|c| f.widths.get(*c as usize).copied().unwrap_or(0) as f64)
            .sum::<f64>()
            * size
            / 1000.0
    }

    /// Finish: bytes of the whole PDF document.
    pub fn build(self, title: &str) -> Vec<u8> {
        let content = self.ops;
        let used = self.used_cids;
        let emb = self.embedded;
        // object layout: 1 cat, 2 pages, 3 page, 4 contents, 5 F1,
        // [6 Type0, 7 CIDFont, 8 FontDescriptor, 9 FontFile2], Info last.
        let info_n: u32 = if emb.is_some() { 10 } else { 6 };
        let px = self.pdfx.clone();
        let (xmp_n, oi_n, icc_n) = if px.is_some() {
            (info_n + 1, info_n + 2, info_n + 3)
        } else {
            (0, 0, 0)
        };
        let last_obj = match &px {
            Some(p) if p.icc.is_some() => icc_n,
            Some(_) => oi_n,
            None => info_n,
        };
        let n_objs = last_obj;
        let mut out = Vec::new();
        out.extend_from_slice(b"%PDF-1.4\n");
        let mut offsets = Vec::with_capacity(n_objs as usize);
        let obj = |buf: &mut Vec<u8>, offs: &mut Vec<usize>, n: u32, body: &[u8]| {
            offs.push(buf.len());
            writeln!(buf, "{n} 0 obj").ok();
            buf.extend_from_slice(body);
            buf.extend_from_slice(b"\nendobj\n");
        };
        let catalog = match &px {
            Some(_) => format!(
                "<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << /DisplayDocTitle true >> /OutputIntents [{oi_n} 0 R] /Metadata {xmp_n} 0 R >>"
            ),
            None => "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        };
        obj(&mut out, &mut offsets, 1, catalog.as_bytes());
        obj(
            &mut out,
            &mut offsets,
            2,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        let fonts = if emb.is_some() {
            "<< /F1 5 0 R /F2 6 0 R >>"
        } else {
            "<< /F1 5 0 R >>"
        };
        let mut page = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.4} {:.4}]",
            self.w_pt, self.h_pt
        );
        if let Some(p) = &px {
            let trim = p.trim.unwrap_or((0.0, 0.0, self.w_pt, self.h_pt));
            page.push_str(&format!(
                " /BleedBox [0 0 {:.4} {:.4}] /TrimBox [{:.4} {:.4} {:.4} {:.4}] /CropBox [0 0 {:.4} {:.4}]",
                self.w_pt, self.h_pt, trim.0, trim.1, trim.2, trim.3, self.w_pt, self.h_pt
            ));
        }
        page.push_str(&format!(
            " /Resources << /Font {fonts} >> /Contents 4 0 R >>"
        ));
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
        if let Some(f) = emb {
            // /W: consecutive cid runs
            let mut w = String::from("[");
            let mut it = used.iter().peekable();
            while let Some(&start) = it.next() {
                let mut end = start;
                while let Some(&&nx) = it.peek() {
                    if nx == end + 1 {
                        end = nx;
                        it.next();
                    } else {
                        break;
                    }
                }
                if start == end {
                    w.push_str(&format!(
                        " {start} [{}]",
                        f.widths.get(start as usize).copied().unwrap_or(0)
                    ));
                } else {
                    let ws: Vec<String> = (start..=end)
                        .map(|c| f.widths.get(c as usize).copied().unwrap_or(0).to_string())
                        .collect();
                    w.push_str(&format!(" {start} [{}]", ws.join(" ")));
                }
            }
            w.push_str(" ]");
            let base = f.base.clone();
            obj(
                &mut out,
                &mut offsets,
                6,
                format!(
                    "<< /Type /Font /Subtype /Type0 /BaseFont /{base} /Encoding /Identity-H /DescendantFonts [7 0 R] >>"
                )
                .as_bytes(),
            );
            obj(
                &mut out,
                &mut offsets,
                7,
                format!(
                    "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{base} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 8 0 R /DW 1000 /W {w} /CIDToGIDMap /Identity >>"
                )
                .as_bytes(),
            );
            let (x0, y0, x1, y1) = f.bbox_1000;
            obj(
                &mut out,
                &mut offsets,
                8,
                format!(
                    "<< /Type /FontDescriptor /FontName /{base} /Flags 2 /FontBBox [{x0} {y0} {x1} {y1}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 9 0 R >>",
                    f.ascent_1000, f.descent_1000, f.ascent_1000
                )
                .as_bytes(),
            );
            offsets.push(out.len());
            write!(
                out,
                "9 0 obj\n<< /Length {} /Length1 {} >>\nstream\n",
                f.data.len(),
                f.data.len()
            )
            .ok();
            out.extend_from_slice(&f.data);
            out.extend_from_slice(b"\nendstream\nendobj\n");
        }
        let info = {
            let t = title
                .chars()
                .filter(|c| *c != '(' && *c != ')' && *c != '\\')
                .collect::<String>();
            format!("<< /Title ({t}) /Producer (rebook pdfwriter) >>")
        };
        offsets.push(out.len());
        write!(out, "{info_n} 0 obj\n{info}\nendobj\n").ok();
        if let Some(p) = &px {
            let xmp = xmp_packet(title, &p.condition);
            offsets.push(out.len());
            write!(
                out,
                "{xmp_n} 0 obj\n<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n",
                xmp.len()
            )
            .ok();
            out.extend_from_slice(xmp.as_bytes());
            out.extend_from_slice(b"\nendstream\nendobj\n");
            let cond = sanitize(&p.condition);
            let dest = if p.icc.is_some() {
                format!(" /DestOutputProfile {icc_n} 0 R")
            } else {
                String::new()
            };
            offsets.push(out.len());
            write!(
                out,
                "{oi_n} 0 obj\n<< /Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier ({cond}) /Registry (http://www.color.org) /Info ({cond}){dest} >>\nendobj\n"
            )
            .ok();
            if let Some(icc) = &p.icc {
                offsets.push(out.len());
                write!(
                    out,
                    "{icc_n} 0 obj\n<< /N 4 /Alternate /DeviceCMYK /Length {} >>\nstream\n",
                    icc.len()
                )
                .ok();
                out.extend_from_slice(icc);
                out.extend_from_slice(b"\nendstream\nendobj\n");
            }
        }
        let xref_pos = out.len();
        write!(out, "xref\n0 {}\n0000000000 65535 f \n", n_objs + 1).ok();
        for off in offsets {
            writeln!(out, "{off:010} 00000 n ").ok();
        }
        write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R /Info {info_n} 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
            n_objs + 1
        )
        .ok();
        out
    }
}

fn cl(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

/// Strip PDF string delimiters for safe `(literal)` embedding.
fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '(' | ')' | '\\'))
        .collect::<String>()
}

/// Minimal PDF/X-1a XMP packet (dc:title + GTS_PDFXVersion conformance).
fn xmp_packet(title: &str, condition: &str) -> String {
    let t = sanitize(title);
    let c = sanitize(condition);
    format!(
        "\u{feff}<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?><x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:pdfx=\"http://www.npes.org/pdfx/ns/id/\" pdfx:GTS_PDFXVersion=\"PDF/X-1a:2001\" pdfx:GTS_PDFXConformance=\"PDF/X-1a:2001\"/><rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{t}</rdf:li></rdf:Alt></dc:title></rdf:Description><rdf:Description rdf:about=\"\" xmlns:pdfx=\"http://www.npes.org/pdfx/ns/id/\"><pdfx:GTS_PDFXOutputCondition>{c}</pdfx:GTS_PDFXOutputCondition></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>"
    )
}

/// Naive sRGB(0..1) → CMYK(0..1) for vector fills (Ingram's TAC gate cares
/// about raster images, which naive math never pushes past 200 % ink).
pub fn rgb_to_cmyk(r: f64, g: f64, b: f64) -> (f64, f64, f64, f64) {
    let k = 1.0 - cl(r).max(cl(g)).max(cl(b));
    if k >= 1.0 - 1e-9 {
        return (0.0, 0.0, 0.0, 1.0);
    }
    let c = (1.0 - cl(r) - k) / (1.0 - k);
    let m = (1.0 - cl(g) - k) / (1.0 - k);
    let y = (1.0 - cl(b) - k) / (1.0 - k);
    (cl(c), cl(m), cl(y), cl(k))
}

/// Total area coverage (percent) of a CMYK color.
pub fn ink_pct(c: f64, m: f64, y: f64, k: f64) -> f64 {
    (c + m + y + k) * 100.0
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
