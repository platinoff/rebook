//! Classic-American-cars coloring paperback: roster + KDP page math.
//!
//! Trim **8.5×11**, **no bleed**, **premium color** interior (one color
//! identity plate per car, then four black contour views). Make / model /
//! year is printed **once** on the color plate — the four views do not
//! repeat the caption.

use serde::Deserialize;

use crate::standards::{
    PAPERBACK_TRIMS, Paper, even_pages, find_trim, gutter_in, paperback_cover, paperback_pages_ok,
};

/// Embedded roster (`samples/coloring-cars.json`).
pub const ROSTER_JSON: &str = include_str!("../samples/coloring-cars.json");

/// KDP paperback line-art minimum (help: charts/graphics ≥ 0.75 pt).
pub const LINE_MIN_PT: f64 = 0.75;
/// No-bleed outside margin, inches (KDP coloring practice).
pub const OUTSIDE_MARGIN_IN: f64 = 0.25;
/// Color identity plate (make · model · year) printed once per car.
pub const COLOR_PAGES_PER_CAR: u32 = 1;
/// Contour views after the identity plate: ¾, profile, rear, front.
pub const VIEWS_PER_CAR: u32 = 4;

/// One car in the roster.
#[derive(Debug, Clone, Deserialize)]
pub struct Car {
    /// Manufacturer (Cadillac, Ford, …).
    pub make: String,
    /// Model name as printed on the color identity plate.
    pub model: String,
    /// Model year.
    pub year: u16,
    /// `hero` / `signature` / `simple`.
    pub tier: String,
    /// Distinct line-art views (always 4 in this edition).
    pub plates: u32,
    /// Why this car is in the book (source language).
    #[serde(default)]
    pub why: String,
    /// English why (write-then-translate fork).
    #[serde(default)]
    pub why_en: String,
}

/// KDP print knobs for this title.
#[derive(Debug, Clone, Deserialize)]
pub struct KdpSpec {
    /// Always `paperback` for this book.
    pub format: String,
    /// Trim label (`8.5x11`).
    pub trim: String,
    /// `white` | `cream` | `premium` (spine + page-cap).
    pub paper: String,
    /// Interior ink (`color` — identity plates are full color).
    pub ink: String,
    /// Coloring interiors stay inside the trim (no bleed).
    pub bleed: bool,
    /// Stroke floor in points.
    pub line_min_pt: f64,
    /// Outside margin inches.
    pub outside_margin_in: f64,
}

/// Full coloring-book plan.
#[derive(Debug, Clone, Deserialize)]
pub struct Roster {
    /// Stable slug.
    pub id: String,
    /// Ukrainian working title.
    pub title_uk: String,
    /// English KDP title.
    pub title_en: String,
    /// Ukrainian subtitle.
    pub subtitle_uk: String,
    /// English subtitle.
    pub subtitle_en: String,
    /// Author line.
    pub author: String,
    /// Write-then-translate source.
    pub source_language: String,
    /// Print spec.
    pub kdp: KdpSpec,
    /// Title / copyright / belongs-to / how-to / TOC (even).
    pub front_matter_pages: u32,
    /// Index (even).
    pub back_matter_pages: u32,
    /// Cars, display order.
    pub cars: Vec<Car>,
}

/// Parse the embedded roster.
pub fn load_roster() -> Result<Roster, String> {
    serde_json::from_str(ROSTER_JSON).map_err(|e| e.to_string())
}

/// Sum of contour views across cars (not counting color identity plates).
pub fn plate_count(r: &Roster) -> u32 {
    r.cars.iter().map(|c| c.plates).sum()
}

/// Pages of car content: 1 color identity + N contour views, per car.
pub fn car_content_pages(r: &Roster) -> u32 {
    r.cars.len() as u32 * COLOR_PAGES_PER_CAR + plate_count(r)
}

/// Interior PDF page count (even): front + identity/views + back.
pub fn interior_pages(r: &Roster) -> u32 {
    even_pages(r.front_matter_pages + car_content_pages(r) + r.back_matter_pages)
}

/// Paper from the roster (`premium` color default for this title).
pub fn roster_paper(r: &Roster) -> Paper {
    match r.kdp.paper.as_str() {
        "cream" => Paper::Cream,
        "groundwood" | "ground" => Paper::Groundwood,
        "premium" | "premium_color" => Paper::PremiumColor,
        _ => Paper::White,
    }
}

/// KDP gate for this interior (trim, pages, no-bleed coloring defaults).
pub fn kdp_ok(r: &Roster) -> Result<(), String> {
    if r.kdp.format != "paperback" {
        return Err(format!("coloring book is paperback, not {}", r.kdp.format));
    }
    if r.kdp.bleed {
        return Err("coloring interiors use no bleed (art inside 0.25in)".into());
    }
    if r.kdp.line_min_pt < LINE_MIN_PT {
        return Err(format!(
            "line art must be ≥ {LINE_MIN_PT} pt (KDP graphic minimum)"
        ));
    }
    if (r.kdp.outside_margin_in - OUTSIDE_MARGIN_IN).abs() > 1e-9 {
        return Err("outside margin must be 0.25in (no-bleed coloring)".into());
    }
    if r.kdp.ink != "color" {
        return Err("identity plates are color — interior ink must be color".into());
    }
    let trim = find_trim(PAPERBACK_TRIMS, &r.kdp.trim)
        .ok_or_else(|| format!("unknown trim {}", r.kdp.trim))?;
    let pages = interior_pages(r);
    let paper = roster_paper(r);
    if paper != Paper::PremiumColor {
        return Err("color identity plates print on premium color paper".into());
    }
    if !paperback_pages_ok(trim, pages, paper) {
        return Err(format!(
            "{pages} pages invalid for {} / premium color (24–590)",
            r.kdp.trim
        ));
    }
    if gutter_in(pages).is_none() {
        return Err("no gutter band for page count".into());
    }
    let _ = paperback_cover(trim, pages, paper)?;
    if !r.front_matter_pages.is_multiple_of(2) || !r.back_matter_pages.is_multiple_of(2) {
        return Err("front/back matter must be even so the first car opens recto".into());
    }
    for c in &r.cars {
        if c.plates != VIEWS_PER_CAR {
            return Err(format!(
                "{} {} {}: four contour views per model, got {}",
                c.year, c.make, c.model, c.plates
            ));
        }
        if c.make.trim().is_empty() || c.model.trim().is_empty() {
            return Err("make/model required on every identity plate".into());
        }
        if c.why.trim().is_empty() || c.why_en.trim().is_empty() {
            return Err(format!(
                "{} {} {}: need why + why_en",
                c.year, c.make, c.model
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standards::spine_text_allowed;

    #[test]
    fn roster_is_kdp_coloring_paperback() {
        let r = load_roster().unwrap();
        kdp_ok(&r).unwrap();
        assert_eq!(r.cars.len(), 24);
        assert_eq!(plate_count(&r), 96);
        assert_eq!(car_content_pages(&r), 120);
        assert_eq!(interior_pages(&r), 130);
        assert_eq!(r.kdp.trim, "8.5x11");
        assert_eq!(r.kdp.ink, "color");
        assert!(!r.kdp.bleed);
        assert_eq!(r.source_language, "uk");
        assert_eq!(r.front_matter_pages, 8);
        assert!((gutter_in(130).unwrap() - 0.375).abs() < 1e-9);
        assert!(
            spine_text_allowed(130),
            "130 pages is above the 79-page spine-text floor"
        );
        let hero = r.cars.iter().filter(|c| c.tier == "hero").count();
        let sig = r.cars.iter().filter(|c| c.tier == "signature").count();
        let simple = r.cars.iter().filter(|c| c.tier == "simple").count();
        assert_eq!((hero, sig, simple), (6, 10, 8));
    }

    #[test]
    fn caption_identity_is_unique() {
        let r = load_roster().unwrap();
        let mut keys: Vec<String> = r
            .cars
            .iter()
            .map(|c| format!("{}|{}|{}", c.year, c.make, c.model))
            .collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), r.cars.len());
    }
}
