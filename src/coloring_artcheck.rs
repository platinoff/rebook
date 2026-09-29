//! Master-art gate for a coloring book (RB-59): every PNG the brief lists
//! exists, is 3:4 portrait, contour plates are black line on white (no gray
//! wash, no color, no big fills), identity plates are actually in color.

use std::path::Path;

use serde::Serialize;

use crate::coloring::Roster;
use crate::coloring_svg::{art_png_name, identity_png_name, views_for};

/// Master shape (width ÷ height).
pub const ASPECT: f64 = 0.75;
/// Allowed aspect drift.
pub const ASPECT_TOL: f64 = 0.02;
/// Contour: max share of saturated (colored) pixels.
pub const CONTOUR_MAX_COLOR: f64 = 0.01;
/// Contour: max share of mid-gray pixels (anti-aliased edges only).
pub const CONTOUR_MAX_GRAY: f64 = 0.12;
/// Contour: max share of near-black pixels (lines, not filled panels).
pub const CONTOUR_MAX_DARK: f64 = 0.25;
/// Identity plate: saturated share that proves color on its own.
pub const COLOR_MIN_COLOR: f64 = 0.10;
/// Identity plate: above this much white (and little color) it is line art,
/// not a painted scene. Black-paint garage scenes stay near 0% white.
pub const IDENTITY_MAX_WHITE: f64 = 0.60;

/// Pixel census of one master.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct PixStats {
    pub w: u32,
    pub h: u32,
    pub white: f64,
    pub dark: f64,
    pub gray: f64,
    pub color: f64,
}

/// One file that fails the gate.
#[derive(Debug, Clone, Serialize)]
pub struct ArtIssue {
    pub file: String,
    pub reason: String,
}

/// Gate result for a whole book.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ArtReport {
    pub expected: usize,
    pub present: usize,
    pub missing: Vec<String>,
    pub issues: Vec<ArtIssue>,
}

impl ArtReport {
    /// All masters present and clean.
    pub fn ok(&self) -> bool {
        self.missing.is_empty() && self.issues.is_empty()
    }
}

/// Count white / dark / gray / colored pixels of a PNG or JPEG (sniffed by
/// magic bytes — generated masters are often JPEG named `.png`).
pub fn image_stats(bytes: &[u8]) -> Result<PixStats, String> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        jpeg_stats(bytes)
    } else {
        png_stats(bytes)
    }
}

fn jpeg_stats(bytes: &[u8]) -> Result<PixStats, String> {
    let mut dec = jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));
    let px = dec.decode().map_err(|e| format!("jpeg: {e}"))?;
    let info = dec.info().ok_or_else(|| "jpeg: no header".to_string())?;
    let mut t = Tally::default();
    match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => px.iter().for_each(|&l| t.add(l, l, l, 255)),
        jpeg_decoder::PixelFormat::L16 => px
            .as_chunks::<2>()
            .0
            .iter()
            .for_each(|&[hi, _]| t.add(hi, hi, hi, 255)),
        jpeg_decoder::PixelFormat::RGB24 => px
            .as_chunks::<3>()
            .0
            .iter()
            .for_each(|&[r, g, b]| t.add(r, g, b, 255)),
        jpeg_decoder::PixelFormat::CMYK32 => {
            return Err("jpeg: CMYK master, want RGB".into());
        }
    }
    Ok(t.finish(u32::from(info.width), u32::from(info.height)))
}

#[derive(Default)]
struct Tally {
    white: u64,
    dark: u64,
    gray: u64,
    color: u64,
}

impl Tally {
    fn add(&mut self, r: u8, g: u8, b: u8, a: u8) {
        if a < 16 {
            self.white += 1;
            return;
        }
        let lum = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000;
        let sat = r.max(g).max(b) - r.min(g).min(b);
        if sat >= 40 {
            self.color += 1;
        } else if lum >= 225 {
            self.white += 1;
        } else if lum <= 60 {
            self.dark += 1;
        } else {
            self.gray += 1;
        }
    }

    fn finish(&self, w: u32, h: u32) -> PixStats {
        let n = (u64::from(w) * u64::from(h)).max(1) as f64;
        PixStats {
            w,
            h,
            white: self.white as f64 / n,
            dark: self.dark as f64 / n,
            gray: self.gray as f64 / n,
            color: self.color as f64 / n,
        }
    }
}

fn png_stats(bytes: &[u8]) -> Result<PixStats, String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().map_err(|e| format!("png: {e}"))?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "png: image too large".to_string())?;
    let mut buf = vec![0u8; size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png: {e}"))?;
    let ch = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err("png: palette not expanded".into()),
    };
    let mut t = Tally::default();
    for row in buf[..info.buffer_size()].chunks_exact(info.line_size) {
        for px in row[..info.width as usize * ch].chunks_exact(ch) {
            match ch {
                1 => t.add(px[0], px[0], px[0], 255),
                2 => t.add(px[0], px[0], px[0], px[1]),
                3 => t.add(px[0], px[1], px[2], 255),
                _ => t.add(px[0], px[1], px[2], px[3]),
            }
        }
    }
    Ok(t.finish(info.width, info.height))
}

/// Why a master fails, if it does.
pub fn judge(s: &PixStats, is_color: bool) -> Option<String> {
    let aspect = f64::from(s.w) / f64::from(s.h.max(1));
    if (aspect - ASPECT).abs() > ASPECT_TOL {
        return Some(format!("aspect {aspect:.3}, want 3:4 portrait"));
    }
    if is_color {
        if s.color < COLOR_MIN_COLOR && s.white > IDENTITY_MAX_WHITE {
            return Some(format!(
                "identity plate looks like line art ({:.0}% white, {:.1}% colored)",
                s.white * 100.0,
                s.color * 100.0
            ));
        }
        return None;
    }
    if s.color > CONTOUR_MAX_COLOR {
        return Some(format!("contour has color ({:.1}%)", s.color * 100.0));
    }
    if s.gray > CONTOUR_MAX_GRAY {
        return Some(format!("contour has gray shading ({:.1}%)", s.gray * 100.0));
    }
    if s.dark > CONTOUR_MAX_DARK {
        return Some(format!(
            "contour has black fills ({:.1}% dark)",
            s.dark * 100.0
        ));
    }
    None
}

/// Check every identity + contour master of `roster` under `art`.
pub fn check(roster: &Roster, art: &Path) -> ArtReport {
    let mut rep = ArtReport::default();
    for car in &roster.cars {
        let mut files = vec![(identity_png_name(car), true)];
        for (i, _) in views_for(car).into_iter().enumerate() {
            files.push((art_png_name(car, i), false));
        }
        for (file, is_color) in files {
            rep.expected += 1;
            let path = art.join(&file);
            let Ok(bytes) = std::fs::read(&path) else {
                rep.missing.push(file);
                continue;
            };
            rep.present += 1;
            let reason = match image_stats(&bytes) {
                Ok(s) => judge(&s, is_color),
                Err(e) => Some(e),
            };
            if let Some(reason) = reason {
                rep.issues.push(ArtIssue { file, reason });
            }
        }
    }
    rep
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_bytes(w: u32, h: u32, px: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w, h);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            let mut wr = enc.write_header().unwrap();
            let mut data = Vec::with_capacity((w * h * 3) as usize);
            for y in 0..h {
                for x in 0..w {
                    data.extend_from_slice(&px(x, y));
                }
            }
            wr.write_image_data(&data).unwrap();
        }
        out
    }

    #[test]
    fn line_art_passes_fills_and_color_fail() {
        let line = png_bytes(60, 80, |x, y| {
            if x == 10 || y == 20 {
                [0, 0, 0]
            } else {
                [255, 255, 255]
            }
        });
        let s = image_stats(&line).unwrap();
        assert_eq!((s.w, s.h), (60, 80));
        assert_eq!(judge(&s, false), None);
        assert!(judge(&s, true).unwrap().contains("line art"));
        let dark_scene = image_stats(&png_bytes(60, 80, |_, y| [(y as u8) / 2; 3])).unwrap();
        assert_eq!(
            judge(&dark_scene, true),
            None,
            "black car in a dark garage is fine"
        );

        let filled = png_bytes(60, 80, |x, _| if x < 30 { [0, 0, 0] } else { [255; 3] });
        assert!(
            judge(&image_stats(&filled).unwrap(), false)
                .unwrap()
                .contains("fills")
        );

        let washed = png_bytes(60, 80, |x, _| if x < 30 { [140; 3] } else { [255; 3] });
        assert!(
            judge(&image_stats(&washed).unwrap(), false)
                .unwrap()
                .contains("gray")
        );

        let red = png_bytes(60, 80, |_, y| if y < 40 { [200, 30, 30] } else { [255; 3] });
        let rs = image_stats(&red).unwrap();
        assert!(judge(&rs, false).unwrap().contains("color"));
        assert_eq!(judge(&rs, true), None);

        let wide = png_bytes(80, 60, |_, _| [255; 3]);
        assert!(
            judge(&image_stats(&wide).unwrap(), false)
                .unwrap()
                .contains("aspect")
        );
    }

    #[test]
    fn check_reports_missing_masters() {
        let r = crate::coloring::load_roster().unwrap();
        let rep = check(&r, Path::new("no-such-art-dir"));
        assert_eq!(rep.expected, r.cars.len() * 5);
        assert_eq!(rep.missing.len(), rep.expected);
        assert!(!rep.ok());
    }
}
