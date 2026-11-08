//! PNG/JPEG masters → 300 DPI RGB JPEG for the KDP interior and wrap.
//!
//! Cover panels are center-cropped to the 8.5×11 sheet. Plates are letterboxed
//! on white. The wrap is one bitmap (back + spine + front) so the PDF has no fonts.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use jpeg_encoder::{ColorType, Encoder};

use crate::preflight::{JpegSpace, jpeg_info};

const COVER_W: u32 = 2550;
const COVER_H: u32 = 3375;
const PLATE_W: u32 = 1800;
const PLATE_H: u32 = 2400;
const PLATE_Q: u8 = 72;
const WRAP_Q: u8 = 88;
const DPI: f64 = 300.0;

struct Rgb {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

/// Write `cover-front.jpg`, `cover-back.jpg`, `cover-wrap.jpg`, and one JPEG
/// per `*-pN.png` / `*-color.png` master.
pub fn rasterize(
    art: &Path,
    out_dir: &Path,
    spine_in: f64,
    spine_text: &str,
) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("mkdir jpeg: {e}"))?;
    let front = art.join("cover-front.png");
    let back = art.join("cover-back.png");
    let cover = Target {
        w: COVER_W,
        h: COVER_H,
        fill: true,
        pad: [0x1A, 0x12, 0x0C],
        quality: PLATE_Q,
        binarize: false,
    };
    fit_jpeg(&front, &out_dir.join("cover-front.jpg"), cover)?;
    fit_jpeg(&back, &out_dir.join("cover-back.jpg"), cover)?;
    write_wrap(out_dir, spine_in, spine_text)?;
    let mut names: Vec<_> = fs::read_dir(art)
        .map_err(|e| format!("read {}: {e}", art.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(is_plate_name)
        })
        .collect();
    names.sort();
    for src in names {
        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("bad name {}", src.display()))?;
        let dst = out_dir.join(format!("{stem}.jpg"));
        fit_jpeg(
            &src,
            &dst,
            Target {
                w: PLATE_W,
                h: PLATE_H,
                fill: false,
                pad: [0xFF, 0xFF, 0xFF],
                quality: PLATE_Q,
                binarize: true,
            },
        )?;
    }
    Ok(())
}

fn is_plate_name(name: &str) -> bool {
    let Some(base) = name.strip_suffix(".png") else {
        return false;
    };
    base.ends_with("-color")
        || base.ends_with("-p0")
        || base.ends_with("-p1")
        || base.ends_with("-p2")
        || base.ends_with("-p3")
}

#[derive(Clone, Copy)]
struct Target {
    w: u32,
    h: u32,
    fill: bool,
    pad: [u8; 3],
    quality: u8,
    binarize: bool,
}

fn fit_jpeg(src: &Path, dst: &Path, spec: Target) -> Result<(), String> {
    if skip_same(src, dst, spec.w, spec.h) {
        println!(
            "skip {}",
            dst.file_name().and_then(|s| s.to_str()).unwrap_or("jpeg")
        );
        return Ok(());
    }
    let mut image = load_rgb(src)?;
    if spec.binarize {
        binarize_ink(&mut image, 165);
    }
    let fitted = resample(&image, spec);
    let bytes = encode_jpeg(&fitted.px, fitted.w, fitted.h, spec.quality)?;
    fs::write(dst, bytes).map_err(|e| format!("write {}: {e}", dst.display()))?;
    println!(
        "ok {}",
        dst.file_name().and_then(|s| s.to_str()).unwrap_or("jpeg")
    );
    Ok(())
}

fn skip_same(src: &Path, dst: &Path, w: u32, h: u32) -> bool {
    let Ok(src_m) = fs::metadata(src).and_then(|m| m.modified()) else {
        return false;
    };
    let Ok(dst_m) = fs::metadata(dst) else {
        return false;
    };
    let Ok(dst_time) = dst_m.modified() else {
        return false;
    };
    if dst_time < src_m {
        return false;
    }
    let Ok(bytes) = fs::read(dst) else {
        return false;
    };
    jpeg_info(&bytes).is_some_and(|info| info.w == w && info.h == h)
}

fn write_wrap(out_dir: &Path, spine_in: f64, spine_text: &str) -> Result<(), String> {
    let bleed = (0.125 * DPI).round() as u32;
    let trim_w = (8.5 * DPI).round() as u32;
    let wrap_h = (11.25 * DPI).round() as u32;
    let spine_px = (spine_in * DPI).round().max(1.0) as u32;
    let wrap_w = bleed * 2 + trim_w * 2 + spine_px;
    let dst = out_dir.join("cover-wrap.jpg");
    let front_path = out_dir.join("cover-front.jpg");
    let back_path = out_dir.join("cover-back.jpg");
    if skip_wrap(&dst, &front_path, &back_path, wrap_w, wrap_h) {
        println!("skip cover-wrap.jpg");
        return Ok(());
    }
    let back = load_rgb(&back_path)?;
    let front = load_rgb(&front_path)?;
    let mut canvas = Rgb {
        w: wrap_w,
        h: wrap_h,
        px: vec![0x1A; (wrap_w as usize) * (wrap_h as usize) * 3],
    };
    // The pad byte above is only the R channel if we filled with 0x1A flat.
    // Fill the real brown explicitly.
    fill(&mut canvas, 0x1A, 0x12, 0x0C);
    blit_stretch(&mut canvas, 0, 0, bleed + trim_w, wrap_h, &back);
    let spine_x = bleed + trim_w;
    fill_rect(
        &mut canvas,
        spine_x,
        0,
        spine_px,
        wrap_h,
        [0x12, 0x0C, 0x08],
    );
    blit_stretch(
        &mut canvas,
        spine_x + spine_px,
        0,
        trim_w + bleed,
        wrap_h,
        &front,
    );
    paint_spine_text(&mut canvas, spine_x, spine_px, spine_text);
    let bytes = encode_jpeg(&canvas.px, canvas.w, canvas.h, WRAP_Q)?;
    fs::write(&dst, bytes).map_err(|e| format!("write {}: {e}", dst.display()))?;
    println!("ok cover-wrap.jpg {wrap_w}x{wrap_h}");
    Ok(())
}

fn skip_wrap(dst: &Path, front: &Path, back: &Path, w: u32, h: u32) -> bool {
    let Ok(dst_time) = fs::metadata(dst).and_then(|m| m.modified()) else {
        return false;
    };
    let newer = |p: &Path| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .is_ok_and(|t| dst_time >= t)
    };
    if !newer(front) || !newer(back) {
        return false;
    }
    fs::read(dst)
        .ok()
        .and_then(|b| jpeg_info(&b))
        .is_some_and(|info| info.w == w && info.h == h)
}

fn load_rgb(path: &Path) -> Result<Rgb, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if bytes.starts_with(&[0xFF, 0xD8]) {
        decode_jpeg(&bytes, path)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        decode_png(&bytes, path)
    } else {
        Err(format!("{} is not a PNG or JPEG", path.display()))
    }
}

fn decode_jpeg(bytes: &[u8], path: &Path) -> Result<Rgb, String> {
    let mut dec = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    let px = dec
        .decode()
        .map_err(|e| format!("jpeg {}: {e}", path.display()))?;
    let info = dec
        .info()
        .ok_or_else(|| format!("jpeg {}: no info", path.display()))?;
    let px = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => px,
        jpeg_decoder::PixelFormat::L8 => {
            let mut rgb = Vec::with_capacity(px.len() * 3);
            for g in px {
                rgb.extend([g, g, g]);
            }
            rgb
        }
        other => {
            return Err(format!("jpeg {}: unsupported {other:?}", path.display()));
        }
    };
    Ok(Rgb {
        w: u32::from(info.width),
        h: u32::from(info.height),
        px,
    })
}

fn decode_png(bytes: &[u8], path: &Path) -> Result<Rgb, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("png {}: {e}", path.display()))?;
    let buf_len = reader
        .output_buffer_size()
        .ok_or_else(|| format!("png {}: no buffer size", path.display()))?;
    let mut buf = vec![0; buf_len];
    let frame = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png {}: {e}", path.display()))?;
    let data = &buf[..frame.buffer_size()];
    let w = frame.width;
    let h = frame.height;
    let px = match frame.color_type {
        png::ColorType::Rgb => data.to_vec(),
        png::ColorType::Rgba => flatten_alpha(data, 4, 0xFF, 0xFF, 0xFF),
        png::ColorType::Grayscale => {
            let mut rgb = Vec::with_capacity(data.len() * 3);
            for g in data {
                rgb.extend([*g, *g, *g]);
            }
            rgb
        }
        png::ColorType::GrayscaleAlpha => flatten_alpha(data, 2, 0xFF, 0xFF, 0xFF),
        other => return Err(format!("png {}: unsupported {other:?}", path.display())),
    };
    Ok(Rgb { w, h, px })
}

fn flatten_alpha(data: &[u8], step: usize, br: u8, bg: u8, bb: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / step * 3);
    for px in data.chunks_exact(step) {
        let a = px[step - 1] as u16;
        let blend = |c: u8, b: u8| ((c as u16 * a + b as u16 * (255 - a)) / 255) as u8;
        let (r, g, b) = if step == 4 {
            (px[0], px[1], px[2])
        } else {
            (px[0], px[0], px[0])
        };
        out.extend([blend(r, br), blend(g, bg), blend(b, bb)]);
    }
    out
}

fn binarize_ink(img: &mut Rgb, dark_max: u8) {
    for px in img.px.as_chunks_mut::<3>().0 {
        let luma =
            (u32::from(px[0]) * 299 + u32::from(px[1]) * 587 + u32::from(px[2]) * 114) / 1000;
        let v = if luma <= u32::from(dark_max) { 0 } else { 255 };
        px[0] = v;
        px[1] = v;
        px[2] = v;
    }
}

fn resample(src: &Rgb, spec: Target) -> Rgb {
    let (w, h, fill) = (spec.w, spec.h, spec.fill);
    let [pr, pg, pb] = spec.pad;
    let sw = src.w.max(1) as f64;
    let sh = src.h.max(1) as f64;
    let scale = if fill {
        (w as f64 / sw).max(h as f64 / sh)
    } else {
        (w as f64 / sw).min(h as f64 / sh)
    };
    let dw = (sw * scale).round();
    let dh = (sh * scale).round();
    let dx = ((w as f64 - dw) / 2.0).round();
    let dy = ((h as f64 - dh) / 2.0).round();
    let mut px = vec![0u8; (w as usize) * (h as usize) * 3];
    for y in 0..h {
        for x in 0..w {
            let i = ((y as usize) * (w as usize) + x as usize) * 3;
            let inside = (x as f64) + 0.5 >= dx
                && (y as f64) + 0.5 >= dy
                && (x as f64) + 0.5 < dx + dw
                && (y as f64) + 0.5 < dy + dh;
            if inside {
                let sx = (x as f64 - dx) / scale;
                let sy = (y as f64 - dy) / scale;
                let [r, g, b] = bilinear(src, sx, sy);
                px[i] = r;
                px[i + 1] = g;
                px[i + 2] = b;
            } else {
                px[i] = pr;
                px[i + 1] = pg;
                px[i + 2] = pb;
            }
        }
    }
    Rgb { w, h, px }
}

fn bilinear(src: &Rgb, x: f64, y: f64) -> [u8; 3] {
    let x = x.clamp(0.0, src.w.saturating_sub(1) as f64);
    let y = y.clamp(0.0, src.h.saturating_sub(1) as f64);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(src.w.saturating_sub(1));
    let y1 = (y0 + 1).min(src.h.saturating_sub(1));
    let fx = (x - x0 as f64) as f32;
    let fy = (y - y0 as f64) as f32;
    let p = |xx, yy| {
        let i = ((yy as usize) * (src.w as usize) + xx as usize) * 3;
        [src.px[i] as f32, src.px[i + 1] as f32, src.px[i + 2] as f32]
    };
    let a = p(x0, y0);
    let b = p(x1, y0);
    let c = p(x0, y1);
    let d = p(x1, y1);
    let mix = |i: usize| {
        let top = a[i] + (b[i] - a[i]) * fx;
        let bot = c[i] + (d[i] - c[i]) * fx;
        (top + (bot - top) * fy).round().clamp(0.0, 255.0) as u8
    };
    [mix(0), mix(1), mix(2)]
}

fn blit_stretch(dst: &mut Rgb, dx: u32, dy: u32, dw: u32, dh: u32, src: &Rgb) {
    if dw == 0 || dh == 0 || src.w == 0 || src.h == 0 {
        return;
    }
    for y in 0..dh {
        for x in 0..dw {
            let sx = (x as f64 + 0.5) * src.w as f64 / dw as f64 - 0.5;
            let sy = (y as f64 + 0.5) * src.h as f64 / dh as f64 - 0.5;
            let [r, g, b] = bilinear(src, sx, sy);
            put(dst, dx + x, dy + y, r, g, b);
        }
    }
}

fn fill(img: &mut Rgb, r: u8, g: u8, b: u8) {
    for px in img.px.as_chunks_mut::<3>().0 {
        px[0] = r;
        px[1] = g;
        px[2] = b;
    }
}

fn fill_rect(img: &mut Rgb, x: u32, y: u32, w: u32, h: u32, color: [u8; 3]) {
    let [r, g, b] = color;
    for yy in y..y.saturating_add(h).min(img.h) {
        for xx in x..x.saturating_add(w).min(img.w) {
            put(img, xx, yy, r, g, b);
        }
    }
}

fn put(img: &mut Rgb, x: u32, y: u32, r: u8, g: u8, b: u8) {
    if x >= img.w || y >= img.h {
        return;
    }
    let i = ((y as usize) * (img.w as usize) + x as usize) * 3;
    img.px[i] = r;
    img.px[i + 1] = g;
    img.px[i + 2] = b;
}

fn paint_spine_text(img: &mut Rgb, spine_x: u32, spine_w: u32, text: &str) {
    let text = text.trim();
    if text.is_empty() || spine_w == 0 {
        return;
    }
    let s = ((spine_w as f64 * 0.42).max(18.0) / 7.0).round().max(1.0) as i32;
    let advance = 6 * s;
    let gh = 7 * s;
    let n = text.chars().count() as i32;
    let mut local_x = -(n * advance) / 2;
    let cx = spine_x as i32 + spine_w as i32 / 2;
    let cy = img.h as i32 / 2;
    for ch in text.chars() {
        let rows = glyph(ch);
        for gy in 0..7i32 {
            for gx in 0..5i32 {
                if rows[gy as usize] & (1 << (4 - gx)) == 0 {
                    continue;
                }
                for py in 0..s {
                    for px in 0..s {
                        let lx = local_x + gx * s + px;
                        let ly = -gh / 2 + gy * s + py;
                        // -90°: the title reads upward along the spine.
                        let sx = cx + ly;
                        let sy = cy - lx;
                        if sx >= 0 && sy >= 0 {
                            put(img, sx as u32, sy as u32, 0xE8, 0xDC, 0xC8);
                        }
                    }
                }
            }
        }
        local_x += advance;
    }
}

fn glyph(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        _ => [0; 7],
    }
}

fn encode_jpeg(px: &[u8], w: u32, h: u32, quality: u8) -> Result<Vec<u8>, String> {
    let w = u16::try_from(w).map_err(|_| format!("jpeg width {w} exceeds 65535"))?;
    let h = u16::try_from(h).map_err(|_| format!("jpeg height {h} exceeds 65535"))?;
    let mut out = Vec::new();
    Encoder::new(&mut out, quality)
        .encode(px, w, h, ColorType::Rgb)
        .map_err(|e| format!("jpeg encode: {e}"))?;
    let info = jpeg_info(&out).ok_or("encoded jpeg has no SOF")?;
    if info.space != JpegSpace::Rgb {
        return Err("encoded jpeg is not RGB".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_plate_is_rgb_jpeg() {
        let src = Rgb {
            w: 2,
            h: 2,
            px: vec![0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 255],
        };
        let fitted = resample(
            &src,
            Target {
                w: 4,
                h: 4,
                fill: false,
                pad: [255, 255, 255],
                quality: 80,
                binarize: false,
            },
        );
        assert_eq!(fitted.px.len(), 4 * 4 * 3);
        let bytes = encode_jpeg(&fitted.px, fitted.w, fitted.h, 80).unwrap();
        let info = jpeg_info(&bytes).unwrap();
        assert_eq!(info.space, JpegSpace::Rgb);
        assert_eq!((info.w, info.h), (4, 4));
    }

    #[test]
    fn contain_leaves_a_white_margin() {
        let src = Rgb {
            w: 2,
            h: 2,
            px: vec![0; 12],
        };
        let fitted = resample(
            &src,
            Target {
                w: 6,
                h: 4,
                fill: false,
                pad: [255, 255, 255],
                quality: 80,
                binarize: false,
            },
        );
        assert_eq!(&fitted.px[0..3], &[255, 255, 255]);
        let mid = (2 * 6 + 3) * 3;
        assert_eq!(&fitted.px[mid..mid + 3], &[0, 0, 0]);
    }
}
