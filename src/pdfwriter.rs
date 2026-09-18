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

/// An embedded raster for cover art (RB-16c/RB-46): JPEG passthrough (DCTDecode).
#[derive(Debug, Clone)]
pub struct RasterImage {
    pub w_px: u32,
    pub h_px: u32,
    pub data: Vec<u8>,
    /// `/DeviceCMYK` when true, else `/DeviceRGB`.
    pub cmyk: bool,
    /// Photoshop CMYK JPEG invert (`/Decode [1 0 1 0 1 0 1 0]`).
    pub invert: bool,
}

/// A page being built: ops accumulate as a content-stream `Vec<u8>`.
/// `new_page` rolls the current ops into a finished page; `build` emits all
/// pages (single-page callers never call `new_page` and get the old shape).
#[derive(Debug, Clone)]
pub struct PdfPageBuilder {
    /// Page width, points (72/inch).
    pub w_pt: f64,
    /// Page height, points.
    pub h_pt: f64,
    ops: Vec<u8>,
    pages: Vec<(f64, f64, Vec<u8>, std::collections::BTreeSet<usize>)>,
    page_images: std::collections::BTreeSet<usize>,
    embedded: Vec<EmbeddedFont>,
    used_cids: Vec<std::collections::BTreeSet<u16>>,
    images: Vec<RasterImage>,
    pdfx: Option<PdfxProfile>,
}

impl PdfPageBuilder {
    /// New page with size in points.
    pub fn new(w_pt: f64, h_pt: f64) -> PdfPageBuilder {
        PdfPageBuilder {
            w_pt,
            h_pt,
            ops: Vec::new(),
            pages: Vec::new(),
            page_images: Default::default(),
            embedded: Vec::new(),
            used_cids: Vec::new(),
            images: Vec::new(),
            pdfx: None,
        }
    }

    /// Register a JPEG raster (bytes as-is, DCTDecode). Returns the image index.
    pub fn add_image(&mut self, w_px: u32, h_px: u32, data: Vec<u8>) -> usize {
        self.add_image_space(w_px, h_px, data, false, false)
    }

    /// Register a JPEG with an explicit PDF color space (RB-46 CMYK passthrough).
    pub fn add_image_space(
        &mut self,
        w_px: u32,
        h_px: u32,
        data: Vec<u8>,
        cmyk: bool,
        invert: bool,
    ) -> usize {
        self.images.push(RasterImage {
            w_px,
            h_px,
            data,
            cmyk,
            invert,
        });
        self.images.len() - 1
    }

    /// Draw image `idx` into the box (cover-fit / slice, centered overflow) with
    /// a clip, bottom-left origin, points.
    pub fn draw_image_cover(&mut self, idx: usize, x: f64, y: f64, w: f64, h: f64) {
        self.blit_image(idx, x, y, w, h, true);
    }

    /// Draw image `idx` contained in the box (letterbox, no crop).
    pub fn draw_image_contain(&mut self, idx: usize, x: f64, y: f64, w: f64, h: f64) {
        self.blit_image(idx, x, y, w, h, false);
    }

    fn blit_image(&mut self, idx: usize, x: f64, y: f64, w: f64, h: f64, fill: bool) {
        let Some(im) = self.images.get(idx) else {
            return;
        };
        if im.w_px == 0 || im.h_px == 0 || w <= 0.0 || h <= 0.0 {
            return;
        }
        self.page_images.insert(idx);
        let ia = im.w_px as f64 / im.h_px as f64;
        let ba = w / h;
        let (sw, sh) = if fill {
            if ia > ba { (h * ia, h) } else { (w, w / ia) }
        } else if ia > ba {
            (w, w / ia)
        } else {
            (h * ia, h)
        };
        let ox = x + (w - sw) / 2.0;
        let oy = y + (h - sh) / 2.0;
        if fill {
            writeln!(self.ops, "q {:.4} {:.4} {:.4} {:.4} re W n", x, y, w, h).ok();
            writeln!(
                self.ops,
                "q {:.5} 0 0 {:.5} {:.4} {:.4} cm /Im{idx} Do Q Q",
                sw, sh, ox, oy
            )
            .ok();
        } else {
            writeln!(
                self.ops,
                "q {:.5} 0 0 {:.5} {:.4} {:.4} cm /Im{idx} Do Q",
                sw, sh, ox, oy
            )
            .ok();
        }
    }

    /// Finish the current page and start another one of the given size.
    pub fn new_page(&mut self, w_pt: f64, h_pt: f64) {
        let ops = std::mem::take(&mut self.ops);
        let used = std::mem::take(&mut self.page_images);
        self.pages.push((self.w_pt, self.h_pt, ops, used));
        self.w_pt = w_pt;
        self.h_pt = h_pt;
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
        self.add_embedded_font(font);
    }

    /// Register another embedded TrueType font (`/F3`, `/F4`, …). Returns its
    /// index for [`text_cid_at`].
    pub fn add_embedded_font(&mut self, font: EmbeddedFont) -> usize {
        self.embedded.push(font);
        self.used_cids.push(Default::default());
        self.embedded.len() - 1
    }

    /// Write glyph-CID hex text (`/F2`), 90°-rotated when `rot90`.
    pub fn text_cid(&mut self, x: f64, y: f64, size: f64, cids: &[u16], rot90: bool) {
        self.text_cid_at(0, x, y, size, cids, rot90);
    }

    /// Write glyph-CID hex text with the N-th embedded font.
    pub fn text_cid_at(
        &mut self,
        idx: usize,
        x: f64,
        y: f64,
        size: f64,
        cids: &[u16],
        rot90: bool,
    ) {
        if cids.is_empty() || self.embedded.get(idx).is_none() {
            return;
        }
        if let Some(u) = self.used_cids.get_mut(idx) {
            u.extend(cids.iter().copied());
        }
        let hex: String = cids.iter().map(|c| format!("{c:04X}")).collect();
        let tm = if rot90 {
            format!("0 1 -1 0 {:.4} {:.4}", x, y)
        } else {
            format!("1 0 0 1 {:.4} {:.4}", x, y)
        };
        writeln!(
            self.ops,
            "BT /F{} {:.4} Tf {tm} Tm <{hex}> Tj ET",
            2 + idx,
            size
        )
        .ok();
    }

    /// Justified glyph run: word runs with explicit inter-word gaps (pt).
    /// Gap after the last run is ignored. Uses font `idx` (`/F{2+idx}`).
    pub fn text_cid_tj(&mut self, idx: usize, x: f64, y: f64, size: f64, runs: &[(&[u16], f64)]) {
        let runs: Vec<&(&[u16], f64)> = runs.iter().filter(|(r, _)| !r.is_empty()).collect();
        if runs.is_empty() || self.embedded.get(idx).is_none() {
            return;
        }
        let scale = if size.abs() < 1e-6 {
            0.0
        } else {
            -1000.0 / size
        };
        let mut arr = String::new();
        for (i, (run, gap)) in runs.iter().enumerate() {
            if let Some(u) = self.used_cids.get_mut(idx) {
                u.extend(run.iter().copied());
            }
            let hex: String = run.iter().map(|c| format!("{c:04X}")).collect();
            arr.push_str(&format!("<{hex}>"));
            if i + 1 < runs.len() {
                arr.push_str(&format!(" {:.3}", *gap * scale));
            }
        }
        writeln!(
            self.ops,
            "BT /F{} {:.4} Tf 1 0 0 1 {:.4} {:.4} Tm [{arr}] TJ ET",
            2 + idx,
            size,
            x,
            y
        )
        .ok();
    }

    /// Width in points of a CID run at `size` (for centering); 0 without font.
    pub fn cid_width_pt(&self, cids: &[u16], size: f64) -> f64 {
        self.cid_width_pt_at(0, cids, size)
    }

    /// Width in points of a CID run against the N-th embedded font.
    pub fn cid_width_pt_at(&self, idx: usize, cids: &[u16], size: f64) -> f64 {
        let Some(f) = self.embedded.get(idx) else {
            return 0.0;
        };
        cids.iter()
            .map(|c| f.widths.get(*c as usize).copied().unwrap_or(0) as f64)
            .sum::<f64>()
            * size
            / 1000.0
    }

    /// Finish: bytes of the whole PDF document (all rolled + current page).
    pub fn build(self, title: &str) -> Vec<u8> {
        let mut all_pages = self.pages;
        all_pages.push((self.w_pt, self.h_pt, self.ops, self.page_images));
        let k = all_pages.len();
        let emb = &self.embedded;
        let used = &self.used_cids;
        let e = emb.len();
        // object layout: 1 cat, 2 pages, per page p: 3+2p page, 4+2p content,
        // base = 3+2k: F1 = base+1, then per font i: Type0 base+2+4i,
        // CIDFont +1, Descriptor +2, FontFile2 +3; Info/XMP/OI/ICC last.
        // base is itself F1 (no gap after the per-page 3+2p / 4+2p pairs).
        let base: u32 = 3 + 2 * k as u32;
        let f1 = base;
        let img0 = base + 1 + 4 * e as u32;
        let ni = self.images.len() as u32;
        let info_n = img0 + ni;
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
        out.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
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
        let mut kids = String::from("[");
        for p in 0..k {
            if p.is_multiple_of(8) {
                kids.push('\n');
            }
            kids.push_str(&format!("{} 0 R ", 3 + 2 * p));
        }
        kids.push(']');
        obj(
            &mut out,
            &mut offsets,
            2,
            format!("<< /Type /Pages /Kids {kids} /Count {k} >>").as_bytes(),
        );
        let mut fonts = format!("/F1 {f1} 0 R");
        for i in 0..e {
            fonts.push_str(&format!(" /F{} {} 0 R", 2 + i, base + 1 + 4 * i as u32));
        }
        for (p, (w, h, content, imgs)) in all_pages.iter().enumerate() {
            let page_no = 3 + 2 * p as u32;
            let cont_no = 4 + 2 * p as u32;
            let mut xobjs = String::new();
            if !imgs.is_empty() {
                xobjs.push_str(" /XObject <<");
                for i in imgs {
                    xobjs.push_str(&format!(" /Im{i} {} 0 R", img0 + *i as u32));
                }
                xobjs.push_str(" >>");
            }
            let resources =
                format!("<< /ProcSet [/PDF /Text /ImageC] /Font << {fonts} >>{xobjs} >>");
            let mut page = format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.4} {:.4}] /CropBox [0 0 {:.4} {:.4}]",
                w, h, w, h
            );
            if let Some(x) = &px {
                let trim = x.trim.unwrap_or((0.0, 0.0, *w, *h));
                page.push_str(&format!(
                    " /BleedBox [0 0 {:.4} {:.4}] /TrimBox [{:.4} {:.4} {:.4} {:.4}]",
                    w, h, trim.0, trim.1, trim.2, trim.3
                ));
            }
            page.push_str(&format!(
                " /Resources {resources} /Contents {cont_no} 0 R >>"
            ));
            obj(&mut out, &mut offsets, page_no, page.as_bytes());
            let stream_hdr = format!("<< /Length {} >>\nstream\n", content.len());
            offsets.push(out.len());
            writeln!(out, "{cont_no} 0 obj").ok();
            out.extend_from_slice(stream_hdr.as_bytes());
            out.extend_from_slice(content);
            out.extend_from_slice(b"\nendstream\nendobj\n");
        }
        obj(
            &mut out,
            &mut offsets,
            f1,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        );
        for (i, f) in emb.iter().enumerate() {
            let u = used.get(i).cloned().unwrap_or_default();
            let type0 = base + 1 + 4 * i as u32;
            let cid_n = type0 + 1;
            let fd_n = type0 + 2;
            let ff_n = type0 + 3;
            // /W: consecutive cid runs
            let mut w = String::from("[");
            let mut it = u.iter().peekable();
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
            let b = f.base.clone();
            obj(
                &mut out,
                &mut offsets,
                type0,
                format!(
                    "<< /Type /Font /Subtype /Type0 /BaseFont /{b} /Encoding /Identity-H /DescendantFonts [{cid_n} 0 R] >>"
                )
                .as_bytes(),
            );
            obj(
                &mut out,
                &mut offsets,
                cid_n,
                format!(
                    "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{b} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {fd_n} 0 R /DW 1000 /W {w} /CIDToGIDMap /Identity >>"
                )
                .as_bytes(),
            );
            let (x0, y0, x1, y1) = f.bbox_1000;
            obj(
                &mut out,
                &mut offsets,
                fd_n,
                format!(
                    "<< /Type /FontDescriptor /FontName /{b} /Flags 2 /FontBBox [{x0} {y0} {x1} {y1}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {ff_n} 0 R >>",
                    f.ascent_1000, f.descent_1000, f.ascent_1000
                )
                .as_bytes(),
            );
            offsets.push(out.len());
            write!(
                out,
                "{ff_n} 0 obj\n<< /Length {} /Length1 {} >>\nstream\n",
                f.data.len(),
                f.data.len()
            )
            .ok();
            out.extend_from_slice(&f.data);
            out.extend_from_slice(b"\nendstream\nendobj\n");
        }
        for (i, im) in self.images.iter().enumerate() {
            let n = img0 + i as u32;
            offsets.push(out.len());
            let space = if im.cmyk { "/DeviceCMYK" } else { "/DeviceRGB" };
            let decode = if im.cmyk && im.invert {
                " /Decode [1 0 1 0 1 0 1 0]"
            } else {
                ""
            };
            write!(
                out,
                "{n} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {space} /BitsPerComponent 8 /Filter /DCTDecode{decode} /Length {} >>\nstream\n",
                im.w_px,
                im.h_px,
                im.data.len()
            )
            .ok();
            out.extend_from_slice(&im.data);
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

    fn obj_slice(s: &str, n: u32) -> &str {
        let start = s.find(&format!("{n} 0 obj")).expect("obj");
        let rest = &s[start..];
        let end = rest.find("endobj").expect("endobj");
        &rest[..end]
    }

    #[test]
    fn each_page_lists_only_its_images() {
        let mut p = PdfPageBuilder::new(612.0, 792.0);
        let i0 = p.add_image(8, 8, crate::preflight::stub_rgb_jpeg(8, 8));
        p.draw_image_cover(i0, 0.0, 0.0, 200.0, 200.0);
        p.new_page(612.0, 792.0);
        let i1 = p.add_image(16, 16, crate::preflight::stub_rgb_jpeg(16, 16));
        p.draw_image_cover(i1, 0.0, 0.0, 200.0, 200.0);
        let bytes = p.build("pages");
        let s = String::from_utf8_lossy(&bytes).into_owned();
        let p0 = obj_slice(&s, 3);
        let p1 = obj_slice(&s, 5);
        assert!(p0.contains("/Im0"), "page 0 must reference Im0");
        assert!(
            !p0.contains("/Im1"),
            "page 0 must not pull Im1 (KDP previewer OOM)"
        );
        assert!(p1.contains("/Im1"), "page 1 must reference Im1");
        assert!(!p1.contains("/Im0"), "page 1 must not pull Im0");
        let doc = lopdf::Document::load_mem(&bytes).expect("lopdf");
        assert_eq!(doc.get_pages().len(), 2);
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
