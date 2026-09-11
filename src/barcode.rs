//! EAN-13 barcode rendering (ISBN → black/white SVG bars) for print covers.
//!
//! Encoding: start `101`, six left-hand digits (L/G by the first digit's
//! parity table), center `01010`, six right-hand digits (R), end `101` =
//! 95 modules. L codes below; `R = !L`, `G = reverse(R)` — derived, so only
//! one table is hand-written.

use crate::standards::isbn_to_ean13;

/// Left (odd) encodings for digits 0..=9.
const L_CODES: [&str; 10] = [
    "0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011",
    "0110111", "0001011",
];

/// Parity pattern for the left half, selected by the first digit.
const PARITY: [&str; 10] = [
    "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL",
    "LGGLGL",
];

#[cfg(test)]
fn complement(bits: &str) -> String {
    bits.chars()
        .map(|c| if c == '0' { '1' } else { '0' })
        .collect()
}

fn encode_digit(d: u8, kind: char) -> Result<&'static str, String> {
    let table: &[&str; 10] = match kind {
        'L' => &L_CODES,
        'R' => &R_CODES,
        'G' => &G_CODES,
        k => return Err(format!("unknown EAN symbol set {k}")),
    };
    table
        .get(usize::from(d))
        .copied()
        .ok_or_else(|| format!("not a digit: {d}"))
}

/// Static complements of [`L_CODES`] (R set).
const R_CODES: [&str; 10] = [
    "1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100",
    "1001000", "1110100",
];

/// Static reverses of [`R_CODES`] (G set).
const G_CODES: [&str; 10] = [
    "0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001",
    "0001001", "0010111",
];

/// 95-module bit string for a 13-digit EAN code (check digit recomputed).
pub fn ean13_bits(raw_isbn: &str) -> Result<String, String> {
    let code = isbn_to_ean13(raw_isbn)?;
    let d: Vec<u8> = code.bytes().map(|b| b - b'0').collect();
    let mut bits = String::with_capacity(95);
    bits.push_str("101");
    let parity = PARITY
        .get(d[0] as usize)
        .ok_or_else(|| "bad first digit".to_string())?;
    for (i, kind) in parity.chars().enumerate() {
        bits.push_str(encode_digit(d[1 + i], kind)?);
    }
    bits.push_str("01010");
    for i in 0..6 {
        bits.push_str(encode_digit(d[7 + i], 'R')?);
    }
    bits.push_str("101");
    Ok(bits)
}

/// Bars as (start_module, len) runs of black from a 95-bit string.
fn black_runs(bits: &str) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let b = bits.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'1' {
            let start = i;
            while i < b.len() && b[i] == b'1' {
                i += 1;
            }
            runs.push((start, i - start));
        } else {
            i += 1;
        }
    }
    runs
}

/// Standalone SVG of the EAN-13 for `isbn`, sized `w_in` × `h_in` inches.
/// White background, bars fill ~78 % of the height, human-readable digits
/// under the bars (KDP: 100 %K on solid white).
pub fn barcode_svg(raw_isbn: &str, w_in: f64, h_in: f64) -> Result<String, String> {
    let bits = ean13_bits(raw_isbn)?;
    let digits = isbn_to_ean13(raw_isbn)?;
    let w_px = (w_in * 300.0).round();
    let h_px = (h_in * 300.0).round();
    let quiet_px = (w_px * 0.06).round();
    let bar_w = (w_px - 2.0 * quiet_px) / 95.0;
    let bar_h = h_px * 0.78;
    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w_px:.0}\" height=\"{h_px:.0}\" viewBox=\"0 0 {w_px:.0} {h_px:.0}\">\n"
    ));
    s.push_str(&format!(
        "  <rect width=\"{w_px:.0}\" height=\"{h_px:.0}\" fill=\"#ffffff\"/>\n"
    ));
    for (m, len) in black_runs(&bits) {
        let x = quiet_px + m as f64 * bar_w;
        s.push_str(&format!(
            "  <rect x=\"{x:.2}\" y=\"0\" width=\"{:.2}\" height=\"{bar_h:.0}\" fill=\"#000000\"/>\n",
            len as f64 * bar_w
        ));
    }
    let font = (h_px * 0.18).round();
    s.push_str(&format!(
        "  <text x=\"{:.0}\" y=\"{:.0}\" font-family=\"monospace\" font-size=\"{font:.0}\" fill=\"#000000\">{digits}</text>\n",
        quiet_px,
        h_px - 2.0,
    ));
    s.push_str("</svg>\n");
    Ok(s)
}

/// A `<g>` fragment of the barcode placed inside an existing SVG at
/// (x_px, y_px, w_px, h_px), sized for 300 DPI cover coordinates.
pub fn barcode_fragment(
    raw_isbn: &str,
    x_px: f64,
    y_px: f64,
    w_px: f64,
    h_px: f64,
) -> Result<String, String> {
    let bits = ean13_bits(raw_isbn)?;
    let digits = isbn_to_ean13(raw_isbn)?;
    let quiet = w_px * 0.06;
    let bar_w = (w_px - 2.0 * quiet) / 95.0;
    let bar_h = h_px * 0.78;
    let mut s = format!(
        "  <g id=\"barcode\">\n    <rect x=\"{x_px:.0}\" y=\"{y_px:.0}\" width=\"{w_px:.0}\" height=\"{h_px:.0}\" fill=\"#ffffff\"/>\n"
    );
    for (m, len) in black_runs(&bits) {
        s.push_str(&format!(
            "    <rect x=\"{:.2}\" y=\"{y_px:.0}\" width=\"{:.2}\" height=\"{bar_h:.0}\" fill=\"#000000\"/>\n",
            x_px + quiet + m as f64 * bar_w,
            len as f64 * bar_w
        ));
    }
    let font = (h_px * 0.18).round();
    s.push_str(&format!(
        "    <text x=\"{:.2}\" y=\"{:.0}\" font-family=\"monospace\" font-size=\"{font:.0}\" fill=\"#000000\">{}</text>\n",
        x_px + quiet,
        y_px + h_px - 2.0,
        digits
    ));
    s.push_str("  </g>\n");
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_frame_is_correct() {
        let b = ean13_bits("978-3-16-148410-0").unwrap();
        assert_eq!(b.len(), 95);
        assert!(b.starts_with("101"));
        assert!(b.ends_with("101"));
        assert_eq!(&b[45..50], "01010");
    }

    #[test]
    fn encodings_consistent() {
        for i in 0..10 {
            assert_eq!(complement(L_CODES[i]), R_CODES[i], "R[{i}]");
            let rev: String = R_CODES[i].chars().rev().collect();
            assert_eq!(G_CODES[i], rev.as_str(), "G[{i}]");
        }
    }

    #[test]
    fn black_runs_tile_the_bars() {
        let b = ean13_bits("9783161484100").unwrap();
        let runs = black_runs(&b);
        assert!(runs.len() >= 27); // 2 guards + 12 digits × ≥2 runs, no merges
        assert_eq!(runs[0], (0, 1)); // start guard 101
        assert_eq!(runs[runs.len() - 1], (94, 1)); // end guard last module
    }

    #[test]
    fn svg_output_well_formed() {
        let svg = barcode_svg("0-306-40615-2", 2.0, 1.2).unwrap();
        assert!(svg.starts_with("<svg xmlns="));
        assert!(svg.ends_with("</svg>\n"));
        assert!(svg.contains("9780306406157")); // converted ISBN shown
        let g = barcode_fragment("9783161484100", 10.0, 20.0, 600.0, 360.0).unwrap();
        assert!(g.contains("<g id=\"barcode\">"));
        assert!(g.contains("</g>"));
    }

    #[test]
    fn invalid_isbn_rejected() {
        assert!(ean13_bits("123").is_err());
        assert!(barcode_svg("1234567890123", 2.0, 1.2).is_err()); // not 978/979
    }
}
