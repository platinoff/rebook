//! RB-16b: the slice of TrueType a PDF writer needs — no external crates.
//!
//! Parses `head`, `maxp`, `hhea`, `hmtx` and `cmap` (formats 4 and 12) from a
//! glyf/locf-free read-only view: enough to map Unicode → GID, report advance
//! widths for the `/W` array, and embed the file bytes as `/FontFile2`.
//! No layout, no shaping — our covers are one run per text layer.

use std::collections::BTreeMap;
use std::path::Path;

/// A read-only TrueType font (sfnt `0x00010000` or `true`).
#[derive(Debug, Clone)]
pub struct TtfFont {
    /// Raw file bytes (for `/FontFile2`).
    pub data: Vec<u8>,
    /// Units per em (`head`).
    pub units_per_em: u16,
    /// Glyph count (`maxp`).
    pub num_glyphs: u16,
    /// Ascender/descender in font units (`hhea`).
    pub ascender: i16,
    pub descender: i16,
    /// Font bounding box from `head`, font units (xMin, yMin, xMax, yMax).
    pub bbox: (i16, i16, i16, i16),
    /// PostScript-ish family name for `/BaseFont` (sanitized, ASCII only).
    pub base_font: String,
    cmap: Cmap,
    hmtx: Vec<u16>,
    hmtx_lsb: Vec<i16>,
    n_hmetrics: u16,
}

#[derive(Debug, Clone)]
enum Cmap {
    None,
    Fmt4 {
        so: usize,
        segx2: usize,
        ends: Vec<u16>,
        starts: Vec<u16>,
        deltas: Vec<u16>,
        ranges: Vec<u16>,
    },
    Fmt12(Vec<(u32, u32, u32)>),
}

fn u16be(b: &[u8], i: usize) -> u16 {
    u16::from_be_bytes([b[i], b[i + 1]])
}

fn i16be(b: &[u8], i: usize) -> i16 {
    i16::from_be_bytes([b[i], b[i + 1]])
}

fn u32be(b: &[u8], i: usize) -> u32 {
    u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// sfnt table checksum: sum of big-endian 32-bit words (mod 2^32).
fn table_checksum(b: &[u8]) -> u32 {
    let mut s: u32 = 0;
    for w in b.chunks(4) {
        let q = [
            w[0],
            *w.get(1).unwrap_or(&0),
            *w.get(2).unwrap_or(&0),
            *w.get(3).unwrap_or(&0),
        ];
        s = s.wrapping_add(u32::from_be_bytes(q));
    }
    s
}

impl TtfFont {
    /// Load + parse a TTF from disk.
    pub fn load(path: &Path) -> Result<TtfFont, String> {
        let data = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        Self::parse(data)
    }

    /// Parse from raw sfnt bytes.
    pub fn parse(data: Vec<u8>) -> Result<TtfFont, String> {
        if data.len() < 12 {
            return Err("file too short for sfnt".to_string());
        }
        let ver = u32be(&data, 0);
        if ver != 0x0001_0000 && ver != 0x7472_7565
        /* 'true' */
        {
            return Err(format!("not a TTF sfnt (0x{ver:08x})"));
        }
        let num = u16be(&data, 4) as usize;
        let mut tables: BTreeMap<[u8; 4], (usize, usize)> = BTreeMap::new();
        for i in 0..num {
            let e = 12 + i * 16;
            if e + 16 > data.len() {
                break;
            }
            let tag: [u8; 4] = data[e..e + 4].try_into().unwrap();
            let off = u32be(&data, e + 8) as usize;
            let len = u32be(&data, e + 12) as usize;
            tables.insert(tag, (off, len));
        }
        let head = tables.get(b"head").ok_or("no head table")?;
        let units_per_em = u16be(&data, head.0 + 18);
        if units_per_em == 0 {
            return Err("bad unitsPerEm".to_string());
        }
        let bbox = (
            i16be(&data, head.0 + 36),
            i16be(&data, head.0 + 38),
            i16be(&data, head.0 + 40),
            i16be(&data, head.0 + 42),
        );
        let maxp = tables.get(b"maxp").ok_or("no maxp table")?;
        let num_glyphs = u16be(&data, maxp.0 + 4);
        let hhea = tables.get(b"hhea").ok_or("no hhea table")?;
        let ascender = i16be(&data, hhea.0 + 4);
        let descender = i16be(&data, hhea.0 + 6);
        let n_hmetrics = u16be(&data, hhea.0 + 34) as usize;
        let hmtx = tables.get(b"hmtx").ok_or("no hmtx table")?;
        let mut advances = Vec::with_capacity(n_hmetrics);
        for i in 0..n_hmetrics {
            advances.push(u16be(&data, hmtx.0 + i * 4));
        }
        let mut lsbs = Vec::with_capacity(num_glyphs as usize);
        for i in 0..num_glyphs as usize {
            let v = if i < n_hmetrics {
                i16be(&data, hmtx.0 + i * 4 + 2)
            } else if n_hmetrics > 0 {
                i16be(&data, hmtx.0 + n_hmetrics * 4 + (i - n_hmetrics) * 2)
            } else {
                0
            };
            lsbs.push(v);
        }
        let cmap = Self::parse_cmap(&data, tables.get(b"cmap"))?;
        let name = tables
            .get(b"name")
            .and_then(|&(off, _)| {
                let count = u16be(&data, off + 2) as usize;
                for i in 0..count {
                    let r = off + 6 + i * 12;
                    if r + 12 > data.len() {
                        break;
                    }
                    let nid = u16be(&data, r + 6);
                    if nid == 4 {
                        let len = u16be(&data, r + 8) as usize;
                        let str_off = off + u16be(&data, r + 10) as usize;
                        if str_off + len <= data.len() {
                            let raw = &data[str_off..str_off + len];
                            let s = match u16be(&data, r) {
                                3 | 0 => String::from_utf16_lossy(
                                    &raw.chunks(2)
                                        .filter_map(|c| {
                                            if c.len() == 2 {
                                                Some(u16::from_be_bytes([c[0], c[1]]))
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<_>>(),
                                ),
                                _ => String::from_utf8_lossy(raw).into_owned(),
                            };
                            let base = s
                                .split(['-', ' '])
                                .filter(|p| !p.is_empty())
                                .collect::<Vec<_>>()
                                .join("");
                            let ascii = base
                                .chars()
                                .filter(|c| c.is_ascii_alphanumeric())
                                .collect::<String>();
                            if !ascii.is_empty() {
                                return Some(ascii);
                            }
                        }
                    }
                }
                None
            })
            .unwrap_or_else(|| "RebookFont".to_string());
        Ok(TtfFont {
            data,
            units_per_em,
            num_glyphs,
            ascender,
            descender,
            bbox,
            base_font: name,
            cmap,
            hmtx: advances,
            hmtx_lsb: lsbs,
            n_hmetrics: n_hmetrics as u16,
        })
    }

    fn parse_cmap(data: &[u8], cmap: Option<&(usize, usize)>) -> Result<Cmap, String> {
        let Some(&(off, len)) = cmap else {
            return Ok(Cmap::None);
        };
        let _ = len;
        let n = u16be(data, off + 2) as usize;
        let mut best: Option<(u32, u32)> = None; // (score, subtable offset)
        for i in 0..n {
            let e = off + 4 + i * 8;
            let pid = u16be(data, e);
            let eid = u16be(data, e + 2);
            let so = off + u32be(data, e + 4) as usize;
            let score = match (pid, eid) {
                (3, 10) | (0, 4) => 3,
                (3, 1) | (0, 3) => 2,
                (0, _) => 1,
                _ => 0,
            };
            if score > 0 && best.is_none_or(|(s, _)| score > s) {
                best = Some((score, so as u32));
            }
        }
        let Some((_, so)) = best else {
            return Ok(Cmap::None);
        };
        let so = so as usize;
        match u16be(data, so) {
            4 => {
                let seg_x2 = u16be(data, so + 6) as usize;
                let seg = seg_x2 / 2;
                let ends: Vec<u16> = (0..seg).map(|i| u16be(data, so + 14 + i * 2)).collect();
                let starts: Vec<u16> = (0..seg)
                    .map(|i| u16be(data, so + 14 + seg_x2 + i * 2))
                    .collect();
                let deltas: Vec<u16> = (0..seg)
                    .map(|i| u16be(data, so + 14 + seg_x2 * 2 + i * 2))
                    .collect();
                let ranges: Vec<u16> = (0..seg)
                    .map(|i| u16be(data, so + 14 + seg_x2 * 3 + i * 2))
                    .collect();
                Ok(Cmap::Fmt4 {
                    so,
                    segx2: seg_x2,
                    ends,
                    starts,
                    deltas,
                    ranges,
                })
            }
            12 => {
                let n_groups = u32be(data, so + 12) as usize;
                let mut g = Vec::with_capacity(n_groups);
                for i in 0..n_groups {
                    let e = so + 16 + i * 12;
                    g.push((u32be(data, e), u32be(data, e + 4), u32be(data, e + 8)));
                }
                Ok(Cmap::Fmt12(g))
            }
            f => {
                let _ = f;
                Ok(Cmap::None)
            }
        }
    }

    /// Glyph id for a char (0 = .notdef when unknown).
    pub fn glyph(&self, ch: char) -> u16 {
        let c = ch as u32;
        match &self.cmap {
            Cmap::None => 0,
            Cmap::Fmt12(groups) => groups
                .iter()
                .find(|(s, e, _)| *s <= c && c <= *e)
                .map(|(s, _, sg)| sg + (c - s))
                .and_then(|g| u16::try_from(g).ok())
                .unwrap_or(0),
            Cmap::Fmt4 {
                so,
                segx2,
                ends,
                starts,
                deltas,
                ranges,
            } => {
                for i in 0..ends.len() {
                    if ends[i] as u32 >= c {
                        let code = c as u16;
                        if starts[i] > code {
                            return 0;
                        }
                        let id_delta = deltas[i];
                        let id_range = ranges[i];
                        if id_range == 0 {
                            return id_delta.wrapping_add(code);
                        }
                        // addr = &ranges[i] + idRangeOffset + 2*(code-start[i])
                        let ranges_addr = *so + 14 + segx2 * 3 + i * 2;
                        let addr =
                            ranges_addr + id_range as usize + (code - starts[i]) as usize * 2;
                        if addr + 1 >= self.data.len() {
                            return 0;
                        }
                        let g = u16be(&self.data, addr);
                        if g == 0 {
                            return 0;
                        }
                        return g.wrapping_add(id_delta);
                    }
                }
                0
            }
        }
    }

    /// Advance width of a glyph in font units.
    pub fn advance_units(&self, gid: u16) -> u16 {
        let idx = (gid as usize).min(self.n_hmetrics.saturating_sub(1) as usize);
        self.hmtx.get(idx).copied().unwrap_or(0)
    }

    fn table(&self, tag: &[u8; 4]) -> Option<(usize, usize)> {
        let num = u16be(&self.data, 4) as usize;
        for i in 0..num {
            let e = 12 + i * 16;
            if e + 16 > self.data.len() {
                break;
            }
            if &self.data[e..e + 4] == tag {
                return Some((
                    u32be(&self.data, e + 8) as usize,
                    u32be(&self.data, e + 12) as usize,
                ));
            }
        }
        None
    }

    /// Byte ranges of `loca` entries (offset of `gid` may equal the next → empty glyph).
    pub fn loca(&self) -> Vec<u32> {
        let head_off = match self.table(b"head") {
            Some(o) => o.0,
            None => return Vec::new(),
        };
        let fmt = i16be(&self.data, head_off + 50);
        let n = self.num_glyphs as usize + 1;
        if fmt < 0 {
            (0..n)
                .map(|i| u32be(&self.data, self.loca_off() + i * 4))
                .collect()
        } else {
            (0..n)
                .map(|i| u16be(&self.data, self.loca_off() + i * 2) as u32 * 2)
                .collect()
        }
    }

    fn loca_off(&self) -> usize {
        self.table(b"loca").map(|(o, _)| o).unwrap_or(0)
    }

    /// `glyf` table byte range in the file.
    pub fn glyf_off_len(&self) -> Option<(usize, usize)> {
        self.table(b"glyf")
    }

    /// Glyph data bytes (empty for missing/simple-empty glyphs).
    pub fn glyph_bytes(&self, gid: u16) -> Vec<u8> {
        let loca = self.loca();
        let i = gid as usize;
        if i + 1 >= loca.len() || loca[i] == loca[i + 1] {
            return Vec::new();
        }
        let go = match self.glyf_off_len() {
            Some((o, _)) => o,
            None => return Vec::new(),
        };
        let a = go + loca[i] as usize;
        let b = (go + loca[i + 1] as usize).min(self.data.len());
        if a >= b {
            return Vec::new();
        }
        self.data[a..b].to_vec()
    }

    /// Composite glyph member GIDs (empty for simple glyphs).
    pub fn composite_parts(&self, gid: u16) -> Vec<u16> {
        let g = self.glyph_bytes(gid);
        if g.len() < 12 || i16be(&g, 2) >= 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut i = 10usize; // after simple header (8) + composite first entry starts at 10
        loop {
            if i + 4 > g.len() {
                break;
            }
            let flags = u16be(&g, i);
            let idx = u16be(&g, i + 2);
            out.push(idx);
            i += 4;
            let args1 = if flags & 0x0001 != 0 { 4 } else { 2 };
            let args2 = if flags & 0x0008 != 0 { 4 } else { 2 };
            i += args1 + args2;
            if flags & 0x0020 == 0 {
                break; // MORE_COMPONENTS clear
            }
        }
        out
    }

    /// GID closure: the set plus every composite dependency, transitively.
    pub fn closure(
        &self,
        seeds: &std::collections::BTreeSet<u16>,
    ) -> std::collections::BTreeSet<u16> {
        let mut out = seeds.clone();
        let mut stack: Vec<u16> = seeds.iter().copied().collect();
        while let Some(g) = stack.pop() {
            for p in self.composite_parts(g) {
                if out.insert(p) {
                    stack.push(p);
                }
            }
        }
        out.insert(0); // .notdef always
        out
    }

    /// Rebuild a minimal sfnt (cmap/glyf/head/hhea/hmtx/loca) containing only
    /// the closure of `used`. GID numbering is preserved (unused GIDs become
    /// empty glyphs), so Identity-H CIDs and the original cmap stay valid.
    /// Times-full embeds are ~500 KB; a book needs ~60–120 KB.
    pub fn subset(&self, used: &std::collections::BTreeSet<u16>) -> Result<Vec<u8>, String> {
        let gids = self.closure(used);
        let maxg = *gids.iter().next_back().ok_or("empty subset")? as usize;
        if maxg >= self.num_glyphs as usize {
            return Err(format!("gid {maxg} out of range"));
        }
        let n = maxg + 1;
        let loca = self.loca();
        if loca.len() < self.num_glyphs as usize + 1 {
            return Err("short loca".to_string());
        }
        let (go, _) = self.glyf_off_len().ok_or("no glyf")?;
        let mut glyf = Vec::new();
        let mut offs: Vec<u32> = Vec::with_capacity(n + 1);
        for g in 0..n {
            offs.push(glyf.len() as u32);
            if !gids.contains(&(g as u16)) {
                continue;
            }
            let a = go + loca[g] as usize;
            let b = (go + loca[g + 1] as usize).min(self.data.len());
            if a < b {
                glyf.extend_from_slice(&self.data[a..b]);
                while glyf.len() % 4 != 0 {
                    glyf.push(0);
                }
            }
        }
        offs.push(glyf.len() as u32);
        let short = glyf.len() <= 131_070;
        let mut loca_b = Vec::with_capacity((n + 1) * 4);
        for o in &offs {
            if short {
                loca_b.extend_from_slice(&((o / 2) as u16).to_be_bytes());
            } else {
                loca_b.extend_from_slice(&o.to_be_bytes());
            }
        }
        let mut hmtx_b = Vec::with_capacity(n * 4);
        for g in 0..n {
            let adv = self.advance_units(g as u16);
            let lsb = if gids.contains(&(g as u16)) {
                self.hmtx_lsb.get(g).copied().unwrap_or(0)
            } else {
                0
            };
            hmtx_b.extend_from_slice(&adv.to_be_bytes());
            hmtx_b.extend_from_slice(&lsb.to_be_bytes());
        }
        let mut head = {
            let (o, l) = self.table(b"head").ok_or("no head")?;
            self.data[o..o + l].to_vec()
        };
        head[8..12].copy_from_slice(&0u32.to_be_bytes());
        head[50..52].copy_from_slice(&(if short { 0i16 } else { 1i16 }).to_be_bytes());
        let mut hhea = {
            let (o, l) = self.table(b"hhea").ok_or("no hhea")?;
            self.data[o..o + l].to_vec()
        };
        hhea[34..36].copy_from_slice(&(n as u16).to_be_bytes());
        let maxp = {
            let (o, l) = self.table(b"maxp").ok_or("no maxp")?;
            let mut t = self.data[o..o + l].to_vec();
            t[4..6].copy_from_slice(&(n as u16).to_be_bytes());
            t
        };
        let cmap = {
            let (o, l) = self.table(b"cmap").ok_or("no cmap")?;
            self.data[o..o + l].to_vec()
        };
        let tables: [(&[u8; 4], Vec<u8>); 7] = [
            (b"cmap", cmap),
            (b"glyf", glyf),
            (b"head", head),
            (b"hhea", hhea),
            (b"hmtx", hmtx_b),
            (b"loca", loca_b),
            (b"maxp", maxp),
        ];
        let tn = tables.len() as u16;
        let pow2 = (tn as f64).log2().floor() as u32;
        let search_range = 16 * (1 << pow2);
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        out.extend_from_slice(&tn.to_be_bytes());
        out.extend_from_slice(&(search_range as u16).to_be_bytes());
        out.extend_from_slice(&(pow2 as u16).to_be_bytes());
        out.extend_from_slice(&((tn as u32 * 16) - search_range).to_be_bytes()[..2]);
        let mut body = Vec::new();
        let mut dir = Vec::new();
        let mut off = 12 + tn as usize * 16;
        for (tag, bytes) in &tables {
            let mut padded = bytes.clone();
            while padded.len() % 4 != 0 {
                padded.push(0);
            }
            let cs = table_checksum(&padded);
            dir.extend_from_slice(*tag);
            dir.extend_from_slice(&cs.to_be_bytes());
            dir.extend_from_slice(&(off as u32).to_be_bytes());
            dir.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            off += padded.len();
            body.extend_from_slice(&padded);
        }
        out.extend_from_slice(&dir);
        out.extend_from_slice(&body);
        Ok(out)
    }

    /// Advance width scaled to 1/1000 em (PDF /W units).
    pub fn width_1000(&self, gid: u16) -> i32 {
        (self.advance_units(gid) as i32 * 1000 / self.units_per_em as i32).min(2000)
    }

    /// Width of a text run at `size_pt`, points.
    pub fn run_width_pt(&self, s: &str, size_pt: f64) -> f64 {
        s.chars()
            .map(|c| self.width_1000(self.glyph(c)) as f64)
            .sum::<f64>()
            * size_pt
            / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn times() -> Option<TtfFont> {
        // pick any usable system family (stubbed faces are skipped)
        crate::interior::discover_fonts()
            .ok()
            .and_then(|f| TtfFont::load(&f.regular).ok())
    }

    #[test]
    fn parses_system_times_when_present() {
        let Some(f) = times() else {
            eprintln!("times.ttf missing — skip");
            return;
        };
        assert!(f.units_per_em >= 1000, "upem {}", f.units_per_em);
        assert!(f.num_glyphs > 100);
        let _ = f.n_hmetrics;
        assert_ne!(f.glyph('A'), 0);
        assert_ne!(f.glyph('К'), 0, "Cyrillic К must map");
        assert_ne!(f.glyph('з'), 0);
        assert!(f.width_1000(f.glyph('A')) > 300);
        assert!(f.run_width_pt("Hello", 12.0) > 20.0);
        assert!(!f.base_font.is_empty());
    }

    #[test]
    fn notdef_for_unknown() {
        let Some(f) = times() else { return };
        // Private-use unassigned should be 0 or a placeholder — must not panic.
        let _ = f.glyph('\u{10FFFF}');
    }
}
