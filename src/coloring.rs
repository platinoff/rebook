//! Classic-American-cars coloring paperback: roster + KDP page math.
//!
//! Interior is **B&W on white**, trim **8.5×11**, **no bleed**. Each plate is a
//! recto line-art page; the verso is a caption (make / model / year + custom
//! mark) so KDP does not see a run of empty backs. Cars do not all get the
//! same plate count — hero shapes need more views.

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
/// One coloring plate = verso caption + recto art.
pub const PAGES_PER_PLATE: u32 = 2;

/// One car in the roster.
#[derive(Debug, Clone, Deserialize)]
pub struct Car {
    /// Manufacturer (Cadillac, Ford, …).
    pub make: String,
    /// Model name as printed on the verso.
    pub model: String,
    /// Model year.
    pub year: u16,
    /// `hero` / `signature` / `simple` — drives default plate count.
    pub tier: String,
    /// Distinct line-art views (1–3).
    pub plates: u32,
    /// Why this car gets that many plates.
    #[serde(default)]
    pub why: String,
}

/// KDP print knobs for this title.
#[derive(Debug, Clone, Deserialize)]
pub struct KdpSpec {
    /// Always `paperback` for this book.
    pub format: String,
    /// Trim label (`8.5x11`).
    pub trim: String,
    /// `white` | `cream` | …
    pub paper: String,
    /// Interior ink (`bw`).
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

/// Sum of plates across cars.
pub fn plate_count(r: &Roster) -> u32 {
    r.cars.iter().map(|c| c.plates).sum()
}

/// Interior PDF page count (even): front + 2×plates + back.
pub fn interior_pages(r: &Roster) -> u32 {
    even_pages(r.front_matter_pages + plate_count(r) * PAGES_PER_PLATE + r.back_matter_pages)
}

/// Paper from the roster (`white` default).
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
    let trim = find_trim(PAPERBACK_TRIMS, &r.kdp.trim)
        .ok_or_else(|| format!("unknown trim {}", r.kdp.trim))?;
    let pages = interior_pages(r);
    let paper = roster_paper(r);
    if paper != Paper::White {
        return Err("coloring interiors print B&W on white paper".into());
    }
    if !paperback_pages_ok(trim, pages, paper) {
        return Err(format!("{pages} pages invalid for {} / white", r.kdp.trim));
    }
    if gutter_in(pages).is_none() {
        return Err("no gutter band for page count".into());
    }
    let _ = paperback_cover(trim, pages, paper)?;
    for c in &r.cars {
        let expect = match c.tier.as_str() {
            "hero" => 3,
            "signature" => 2,
            "simple" => 1,
            other => return Err(format!("unknown tier {other}")),
        };
        if c.plates != expect {
            return Err(format!(
                "{} {} {}: plates {} ≠ tier {}",
                c.year, c.make, c.model, c.plates, c.tier
            ));
        }
        if c.make.trim().is_empty() || c.model.trim().is_empty() {
            return Err("make/model required on every verso".into());
        }
        if c.why.trim().is_empty() {
            return Err(format!(
                "{} {} {}: need a why for plate count",
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
        assert_eq!(plate_count(&r), 46);
        assert_eq!(interior_pages(&r), 104);
        assert_eq!(r.kdp.trim, "8.5x11");
        assert!(!r.kdp.bleed);
        assert_eq!(r.source_language, "uk");
        assert!((gutter_in(104).unwrap() - 0.375).abs() < 1e-9);
        assert!(
            spine_text_allowed(104),
            "104 pages clears the 79-page spine floor"
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
