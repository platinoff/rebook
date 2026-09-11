//! KDP / print standardization: trims, spine math, covers, gutters, barcodes.
//!
//! Numbers verified 2026-09-11 against the KDP help pages and the wave-1
//! research matrix in [`docs/REBOOK_DEV_PLAN.md`]. Everything here is pure
//! math over inches — no I/O, no clock, so it is trivially unit-testable and
//! usable by both the CLI and the web service.

/// Print rasterization baseline for cover templates (KDP: ≥300 DPI).
pub const DPI: f64 = 300.0;

/// Paperback outer bleed per edge, inches (KDP).
pub const BLEED_IN: f64 = 0.125;
/// Hardcover wrap past each cover edge, inches (KDP case-laminate).
pub const HC_WRAP_IN: f64 = 0.51;
/// Hardcover hinge dead zone each side of the spine, inches (KDP).
pub const HC_HINGE_IN: f64 = 0.4;
/// Hardcover content-safe distance from the book edge, inches (KDP).
pub const HC_SAFE_IN: f64 = 0.635;
/// Spine text allowed only above this page count (KDP).
pub const SPINE_TEXT_MIN_PAGES: u32 = 79;
/// Recommended clearance of spine text from the spine edges, inches.
pub const SPINE_TEXT_CLEARANCE_IN: f64 = 0.0625;

/// Convert inches to pixels at [`DPI`] (rounds to nearest pixel).
pub fn inches_to_px(inches: f64) -> u32 {
    (inches * DPI).round() as u32
}

/// Body paper (KDP paperback ink/paper combinations).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paper {
    /// B&W interior on white paper.
    White,
    /// B&W interior on cream paper.
    Cream,
    /// B&W interior on groundwood paper.
    Groundwood,
    /// Premium color interior on white paper.
    PremiumColor,
}

impl Paper {
    /// Inches of spine per (even) page.
    pub const fn spine_per_page_in(self) -> f64 {
        match self {
            Paper::White => 0.002252,
            Paper::Cream => 0.0025,
            Paper::Groundwood => 0.00235,
            Paper::PremiumColor => 0.002347,
        }
    }

    /// Valid interior page range `(min, max)` for standard trims.
    pub const fn page_range(self) -> (u32, u32) {
        match self {
            Paper::White => (24, 828),
            Paper::Cream => (24, 776),
            Paper::Groundwood => (24, 812),
            Paper::PremiumColor => (24, 828),
        }
    }
}

/// Round a page count up to an even number (printers sheet the book).
pub const fn even_pages(pages: u32) -> u32 {
    if pages.is_multiple_of(2) {
        pages
    } else {
        pages + 1
    }
}

/// Paperback spine width, inches (`pages × per-page constant`, rounded up to
/// even pages first).
pub fn spine_width(pages: u32, paper: Paper) -> f64 {
    even_pages(pages) as f64 * paper.spine_per_page_in()
}

/// Inside (gutter) margin by page band, inches (KDP print interiors).
/// `None` when the page count is outside paperback limits.
pub fn gutter_in(pages: u32) -> Option<f64> {
    let pages = even_pages(pages);
    match pages {
        24..=150 => Some(0.375),
        151..=300 => Some(0.5),
        301..=500 => Some(0.625),
        501..=700 => Some(0.75),
        701..=828 => Some(0.875),
        _ => None,
    }
}

/// One KDP trim size (inches).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trim {
    /// Canonical label, e.g. `"6x9"`.
    pub label: &'static str,
    /// Trim width, inches.
    pub w: f64,
    /// Trim height, inches.
    pub h: f64,
    /// Max pages per paper: `(white, cream, groundwood, premium_color)`.
    pub max: (u32, u32, u32, u32),
}

/// Standard-trim max tuple (828 / 776 / 812 / 828).
const STD_MAX: (u32, u32, u32, u32) = (828, 776, 812, 828);

/// KDP paperback standard trims (inches), page caps per paper.
pub const PAPERBACK_TRIMS: &[Trim] = &[
    Trim {
        label: "5x8",
        w: 5.0,
        h: 8.0,
        max: STD_MAX,
    },
    Trim {
        label: "5.06x7.81",
        w: 5.06,
        h: 7.81,
        max: STD_MAX,
    },
    Trim {
        label: "5.25x8",
        w: 5.25,
        h: 8.0,
        max: STD_MAX,
    },
    Trim {
        label: "5.5x8.5",
        w: 5.5,
        h: 8.5,
        max: STD_MAX,
    },
    Trim {
        label: "6x9",
        w: 6.0,
        h: 9.0,
        max: STD_MAX,
    },
    Trim {
        label: "6.14x9.21",
        w: 6.14,
        h: 9.21,
        max: STD_MAX,
    },
    Trim {
        label: "6.69x9.61",
        w: 6.69,
        h: 9.61,
        max: STD_MAX,
    },
    Trim {
        label: "7x10",
        w: 7.0,
        h: 10.0,
        max: STD_MAX,
    },
    Trim {
        label: "7.44x9.69",
        w: 7.44,
        h: 9.69,
        max: STD_MAX,
    },
    Trim {
        label: "7.5x9.25",
        w: 7.5,
        h: 9.25,
        max: STD_MAX,
    },
    Trim {
        label: "8x10",
        w: 8.0,
        h: 10.0,
        max: STD_MAX,
    },
    Trim {
        label: "8.25x6",
        w: 8.25,
        h: 6.0,
        max: (800, 750, 784, 800),
    },
    Trim {
        label: "8.25x8.25",
        w: 8.25,
        h: 8.25,
        max: (800, 750, 784, 800),
    },
    Trim {
        label: "8.27x11.69",
        w: 8.27,
        h: 11.69,
        max: (780, 730, 764, 590),
    },
    Trim {
        label: "8.5x8.5",
        w: 8.5,
        h: 8.5,
        max: (590, 550, 578, 590),
    },
    Trim {
        label: "8.5x11",
        w: 8.5,
        h: 11.0,
        max: (590, 550, 578, 590),
    },
];

/// KDP hardcover (case laminate) trims — 75–550 pages, all options.
pub const HARDCOVER_TRIMS: &[Trim] = &[
    Trim {
        label: "5.5x8.5",
        w: 5.5,
        h: 8.5,
        max: (550, 550, 550, 550),
    },
    Trim {
        label: "6x9",
        w: 6.0,
        h: 9.0,
        max: (550, 550, 550, 550),
    },
    Trim {
        label: "6.14x9.21",
        w: 6.14,
        h: 9.21,
        max: (550, 550, 550, 550),
    },
    Trim {
        label: "7x10",
        w: 7.0,
        h: 10.0,
        max: (550, 550, 550, 550),
    },
    Trim {
        label: "8.25x11",
        w: 8.25,
        h: 11.0,
        max: (550, 550, 550, 550),
    },
];

/// Look up a trim by label in a `'static` table.
pub fn find_trim(table: &'static [Trim], label: &str) -> Option<&'static Trim> {
    table.iter().find(|t| t.label == label)
}

/// Page-count max for a trim + paper.
pub const fn max_pages_for(trim: &Trim, paper: Paper) -> u32 {
    match paper {
        Paper::White => trim.max.0,
        Paper::Cream => trim.max.1,
        Paper::Groundwood => trim.max.2,
        Paper::PremiumColor => trim.max.3,
    }
}

/// True when `pages` fits the trim + paper combination (after even rounding).
pub fn paperback_pages_ok(trim: &Trim, pages: u32, paper: Paper) -> bool {
    let pages = even_pages(pages);
    let (min, _) = paper.page_range();
    pages >= min && pages <= max_pages_for(trim, paper)
}

/// Full-wrap paperback cover file size, inches:
/// `W = bleed + back + spine + front + bleed`, `H = trimH + 2×bleed`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverInches {
    /// Full-wrap width (back+spine+front+bleeds), inches.
    pub w: f64,
    /// Full-wrap height, inches.
    pub h: f64,
}

/// Paperback cover geometry for a trim/paper/page count.
/// `Err` when the page count is out of range for the combination.
pub fn paperback_cover(trim: &Trim, pages: u32, paper: Paper) -> Result<CoverInches, String> {
    if !paperback_pages_ok(trim, pages, paper) {
        return Err(format!(
            "{} pages invalid for trim {} / {:?} (KDP range {}–{} even)",
            pages,
            trim.label,
            paper,
            even_pages(paper.page_range().0),
            max_pages_for(trim, paper)
        ));
    }
    let spine = spine_width(pages, paper);
    Ok(CoverInches {
        w: 2.0 * BLEED_IN + 2.0 * trim.w + spine,
        h: trim.h + 2.0 * BLEED_IN,
    })
}

/// KDP hardcover spine approximation: paperback-style constant plus a
/// ~0.06″ board allowance. KDP does not publish the constant — always treat
/// the KDP-generated template as truth; this is a working estimate.
pub fn hardcover_spine_approx(pages: u32, paper: Paper) -> f64 {
    spine_width(pages, paper) + 0.06
}

/// Hardcover (case laminate) cover sheet: back+spine+front plus a
/// [`HC_WRAP_IN`] wrap margin on every outer side.
pub fn hardcover_cover(trim: &Trim, pages: u32, paper: Paper) -> Result<CoverInches, String> {
    let pages = even_pages(pages);
    if !(75..=550).contains(&pages) {
        return Err(format!(
            "{} pages outside KDP hardcover range 75–550 (even)",
            pages
        ));
    }
    let spine = hardcover_spine_approx(pages, paper);
    Ok(CoverInches {
        w: 2.0 * trim.w + spine + 2.0 * HC_WRAP_IN,
        h: trim.h + 2.0 * HC_WRAP_IN,
    })
}

/// KDP back-cover barcode zone (recommended / minimum), inches.
pub const BARCODE_ZONE_IN: (f64, f64) = (2.0, 1.2);
/// Minimum acceptable barcode zone, inches.
pub const BARCODE_MIN_IN: (f64, f64) = (1.4, 0.8);

/// KDP eBook cover ideal size (px) and limits.
pub const EBOOK_COVER_PX: (u32, u32) = (1600, 2560);
/// Minimum eBook cover height, px.
pub const EBOOK_MIN_H_PX: u32 = 1000;
/// Minimum eBook cover width, px.
pub const EBOOK_MIN_W_PX: u32 = 625;
/// Maximum eBook cover edge, px.
pub const EBOOK_MAX_PX: u32 = 10_000;

/// True when a cover image satisfies KDP eBook cover limits
/// (≥1.6:1 height:width, 625×1000 … 10 000² px).
pub fn ebook_cover_ok(w_px: u32, h_px: u32) -> bool {
    w_px >= EBOOK_MIN_W_PX
        && h_px >= EBOOK_MIN_H_PX
        && w_px <= EBOOK_MAX_PX
        && h_px <= EBOOK_MAX_PX
        && h_px as f64 / w_px as f64 >= 1.6
}

/// EAN-13 modulo-10 check digit (weights 1,3 from the left) over 12 digits.
pub fn ean13_check_digit(first12: &[u8; 12]) -> u8 {
    let sum: u32 = first12
        .iter()
        .enumerate()
        .map(|(i, &d)| d as u32 * if i % 2 == 0 { 1 } else { 3 })
        .sum();
    (10 - sum % 10) as u8 % 10
}

/// Keep digits (and a trailing X) from an ISBN-ish string.
fn isbn_digits(raw: &str) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::with_capacity(13);
    for c in raw.chars() {
        if c.is_ascii_digit() {
            out.push(c as u8 - b'0');
        } else if c == '-' || c == ' ' {
            // separators are ignored
        } else if (c == 'x' || c == 'X') && out.len() == 10 {
            out.push(10); // ISBN-10 check value X = 10, dropped in the 978 conversion
        } else {
            return Err(format!("bad character {} in ISBN", c));
        }
    }
    Ok(out)
}

/// Normalize an ISBN-10 or ISBN-13 (dashes optional) to a 13-digit EAN
/// string, computing the check digit. Bookland: ISBN-10 gains a `978`
/// prefix and a recomputed final digit; ISBN-13 keeps 978/979 prefixes.
pub fn isbn_to_ean13(raw: &str) -> Result<String, String> {
    let digits = isbn_digits(raw.trim())?;
    let first12: [u8; 12] = match digits.len() {
        10 => {
            if digits[..9].iter().any(|&d| d > 9) {
                return Err("ISBN-10 has a non-digit".to_string());
            }
            let mut f = [0u8; 12];
            f[..3].copy_from_slice(&[9, 7, 8]);
            f[3..12].copy_from_slice(&digits[..9]);
            f
        }
        13 => {
            let (p1, p2, p3) = (digits[0], digits[1], digits[2]);
            if !(p1 == 9 && p2 == 7 && (p3 == 8 || p3 == 9)) {
                return Err("ISBN-13 must start with Bookland 978/979".to_string());
            }
            let mut f = [0u8; 12];
            f.copy_from_slice(&digits[..12]);
            f
        }
        n => return Err(format!("ISBN has {} digits (want 10 or 13)", n)),
    };
    let cd = ean13_check_digit(&first12);
    let mut out = String::with_capacity(13);
    for d in first12 {
        out.push((b'0' + d) as char);
    }
    out.push((b'0' + cd) as char);
    Ok(out)
}

/// True when a 13-digit string is a valid EAN-13 (incl. check digit).
pub fn ean13_is_valid(code: &str) -> bool {
    let d: Vec<u8> = code
        .bytes()
        .filter(|b| b.is_ascii_digit())
        .map(|b| b - b'0')
        .collect();
    d.len() == 13 && ean13_check_digit(&d[..12].try_into().unwrap()) == d[12]
}

/// Spine text is only permitted above [`SPINE_TEXT_MIN_PAGES`] pages.
pub const fn spine_text_allowed(pages: u32) -> bool {
    pages > SPINE_TEXT_MIN_PAGES
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trim69() -> &'static Trim {
        find_trim(PAPERBACK_TRIMS, "6x9").unwrap()
    }

    #[test]
    fn spine_formulas_match_kdp_constants() {
        assert!((spine_width(300, Paper::White) - 0.6756).abs() < 1e-9);
        assert!((spine_width(200, Paper::Cream) - 0.5).abs() < 1e-9);
        assert!((spine_width(300, Paper::Groundwood) - 0.705).abs() < 1e-9);
        assert!((spine_width(300, Paper::PremiumColor) - 0.7041).abs() < 1e-9);
    }

    #[test]
    fn odd_pages_round_up_to_even() {
        assert_eq!(even_pages(299), 300);
        assert_eq!(even_pages(300), 300);
        assert!((spine_width(299, Paper::White) - spine_width(300, Paper::White)).abs() < 1e-12);
    }

    #[test]
    fn paperback_cover_worked_example() {
        // 6x9, 300 pages white → 12.9256 × 9.25 in → 3878 × 2775 px.
        let c = paperback_cover(trim69(), 300, Paper::White).unwrap();
        assert!((c.w - 12.9256).abs() < 1e-6);
        assert!((c.h - 9.25).abs() < 1e-6);
        assert_eq!(inches_to_px(c.w), 3878);
        assert_eq!(inches_to_px(c.h), 2775);
    }

    #[test]
    fn gutter_bands() {
        assert!((gutter_in(100).unwrap() - 0.375).abs() < 1e-9);
        assert!((gutter_in(151).unwrap() - 0.5).abs() < 1e-9);
        assert!((gutter_in(400).unwrap() - 0.625).abs() < 1e-9);
        assert!((gutter_in(701).unwrap() - 0.875).abs() < 1e-9);
        assert_eq!(gutter_in(12), None);
        assert_eq!(gutter_in(900), None);
    }

    #[test]
    fn page_limits_per_paper_and_trim() {
        assert!(paperback_pages_ok(trim69(), 828, Paper::White));
        assert!(!paperback_pages_ok(trim69(), 830, Paper::White));
        assert!(!paperback_pages_ok(trim69(), 820, Paper::Cream));
        assert!(paperback_pages_ok(trim69(), 776, Paper::Cream));
        let big = find_trim(PAPERBACK_TRIMS, "8.5x11").unwrap();
        assert!(paperback_pages_ok(big, 590, Paper::White));
        assert!(!paperback_pages_ok(big, 700, Paper::White));
    }

    #[test]
    fn spine_text_needs_over_79_pages() {
        assert!(!spine_text_allowed(79));
        assert!(spine_text_allowed(80));
    }

    #[test]
    fn hardcover_geometry_and_range() {
        let hc = find_trim(HARDCOVER_TRIMS, "6x9").unwrap();
        let c = hardcover_cover(hc, 200, Paper::White).unwrap();
        let spine = 200.0 * 0.002252 + 0.06;
        assert!((c.w - (12.0 + spine + 1.02)).abs() < 1e-9);
        assert!((c.h - (9.0 + 1.02)).abs() < 1e-9);
        assert!(hardcover_cover(hc, 74, Paper::White).is_err());
        assert!(hardcover_cover(hc, 552, Paper::White).is_err());
        assert!(hardcover_cover(hc, 75, Paper::White).is_ok()); // → even 76
    }

    #[test]
    fn ebook_cover_limits() {
        assert!(ebook_cover_ok(1600, 2560));
        assert!(ebook_cover_ok(625, 1000));
        assert!(!ebook_cover_ok(600, 1000)); // too narrow
        assert!(!ebook_cover_ok(1000, 1000)); // ratio < 1.6
        assert!(!ebook_cover_ok(1600, 10_001)); // too tall
    }

    #[test]
    fn ean13_check_digit_vectors() {
        // 978-3-16-148410-0 (canonical valid ISBN-13).
        let first12: [u8; 12] = [9, 7, 8, 3, 1, 6, 1, 4, 8, 4, 1, 0];
        assert_eq!(ean13_check_digit(&first12), 0);
        assert!(ean13_is_valid("9783161484100"));
        assert!(!ean13_is_valid("9783161484101"));
    }

    #[test]
    fn isbn10_converts_to_bookland_ean13() {
        // 0-306-40615-2 → 9780306406157 (known vector).
        assert_eq!(isbn_to_ean13("0-306-40615-2").unwrap(), "9780306406157");
        assert_eq!(isbn_to_ean13("9783161484100").unwrap(), "9783161484100");
        assert!(
            isbn_to_ean13("9773161484100")
                .unwrap_err()
                .contains("978/979")
        );
        assert!(isbn_to_ean13("123").is_err());
    }
}
