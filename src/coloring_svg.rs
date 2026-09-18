//! SVG plate kit for the Classic American Iron coloring paperback (RB-51).
//!
//! Pages are **8.5×11 in at 72 pt/in** (`viewBox="0 0 612 792"`). Stroke is
//! **1.25 pt** (above the KDP 0.75 pt graphic floor). No bleed: art lives in
//! the safe box (0.25″ outside, gutter from [`crate::standards::gutter_in`]).
//! Verso = caption + garage mark; recto = one line-art view.

use std::path::Path;

use crate::coloring::{Car, OUTSIDE_MARGIN_IN, Roster, interior_pages};
use crate::standards::gutter_in;

/// Points per inch (PostScript / PDF).
pub const PT_PER_IN: f64 = 72.0;
/// Trim width in points.
pub const PAGE_W: f64 = 8.5 * PT_PER_IN;
/// Trim height in points.
pub const PAGE_H: f64 = 11.0 * PT_PER_IN;
/// Drawn stroke (pt). Must stay ≥ [`crate::coloring::LINE_MIN_PT`] (0.75).
pub const STROKE_PT: f64 = 1.25;
/// Garage-crest size on the verso, points.
pub const MARK_PT: f64 = 96.0;

/// Left vs right page of a coloring spread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Even page: caption + mark (binding on the right).
    Verso,
    /// Odd page: car line art (binding on the left).
    Recto,
}

/// Which drawing of the car this plate shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// ¾ front — default “hero” shot.
    ThreeQuarter,
    /// Side profile.
    Profile,
    /// Rear / fins / split-window.
    Rear,
    /// Front fascia.
    Front,
    /// Badge / cockpit / engine cue.
    Badge,
}

/// Silhouette family (shape, not a licensed blueprint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Tail fins (Eldorado, Impala, Bel Air).
    Fins,
    /// Split-window fastback.
    Split,
    /// Open roadster + side pipes.
    Roadster,
    /// Flying-buttress C-pillar (Charger).
    Buttress,
    /// E-body + shaker (’Cuda / Challenger).
    Ebody,
    /// High-boss pony.
    Pony,
    /// Deuce / 3-window hot rod.
    Hotrod,
    /// Personal-luxury porthole.
    Tbird,
    /// Trans Am spoiler.
    TransAm,
    /// Lead sled.
    Sled,
    /// GT40 low racer.
    Racer,
    /// Generic muscle box.
    Muscle,
}

/// Safe-area rectangle in page points.
#[derive(Debug, Clone, Copy)]
pub struct SafeBox {
    /// Left.
    pub x: f64,
    /// Top.
    pub y: f64,
    /// Width.
    pub w: f64,
    /// Height.
    pub h: f64,
}

/// Views for this car, in plate order (length = `car.plates`).
pub fn views_for(car: &Car) -> Vec<View> {
    match car.plates {
        1 => vec![View::ThreeQuarter],
        2 => vec![View::ThreeQuarter, View::Front],
        _ => vec![View::ThreeQuarter, View::Rear, View::Badge],
    }
}

/// Map make/model to a silhouette family.
pub fn family_of(car: &Car) -> Family {
    let s = format!("{} {}", car.make, car.model).to_ascii_lowercase();
    if s.contains("cobra") {
        Family::Roadster
    } else if s.contains("gt40") {
        Family::Racer
    } else if s.contains("sting") || s.contains("corvette") {
        Family::Split
    } else if s.contains("eldorado") || s.contains("impala") || s.contains("bel air") {
        Family::Fins
    } else if s.contains("charger") {
        Family::Buttress
    } else if s.contains("cuda") || s.contains("challenger") {
        Family::Ebody
    } else if s.contains("deuce") || s.contains("3-window") {
        Family::Hotrod
    } else if s.contains("thunderbird") {
        Family::Tbird
    } else if s.contains("trans am") {
        Family::TransAm
    } else if s.contains("mercury") {
        Family::Sled
    } else if s.contains("mustang") {
        Family::Pony
    } else {
        Family::Muscle
    }
}

/// Safe box for a side given the book’s page count (gutter band).
pub fn safe_box(side: Side, pages: u32) -> SafeBox {
    let out = OUTSIDE_MARGIN_IN * PT_PER_IN;
    let gutter = gutter_in(pages).unwrap_or(0.375) * PT_PER_IN;
    let (left, right) = match side {
        Side::Recto => (gutter, out),
        Side::Verso => (out, gutter),
    };
    SafeBox {
        x: left,
        y: out,
        w: PAGE_W - left - right,
        h: PAGE_H - 2.0 * out,
    }
}

/// `make-model-year` slug for filenames.
pub fn car_slug(car: &Car) -> String {
    let raw = format!("{}-{}-{}", car.year, car.make, car.model);
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// One SVG page (verso caption or recto art).
pub fn page_svg(car: &Car, view: View, side: Side, page: u32, pages: u32) -> String {
    let safe = safe_box(side, pages);
    let body = match side {
        Side::Verso => verso_inner(car, view, &safe),
        Side::Recto => recto_inner(car, view, &safe),
    };
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="8.5in" height="11in" fill="none" stroke="#000" stroke-width="{sw}" stroke-linejoin="round" stroke-linecap="round">
<title>{title}</title>
<rect id="trim" x="0" y="0" width="{w}" height="{h}" stroke="none"/>
<rect id="safe" x="{sx:.2}" y="{sy:.2}" width="{swd:.2}" height="{sh:.2}" stroke="#000" stroke-width="0" fill="none"/>
{body}
<text id="pgn" x="{px:.2}" y="{py:.2}" font-family="Georgia,serif" font-size="11" fill="#000" stroke="none" text-anchor="middle">{page}</text>
</svg>
"##,
        w = PAGE_W,
        h = PAGE_H,
        sw = STROKE_PT,
        title = esc(&format!(
            "{} {} {} {:?}",
            car.year, car.make, car.model, view
        )),
        sx = safe.x,
        sy = safe.y,
        swd = safe.w,
        sh = safe.h,
        body = body,
        px = PAGE_W / 2.0,
        py = PAGE_H - 10.0,
        page = page,
    )
}

/// Write verso+recto SVG pairs for every plate into `dir`. Returns file count.
pub fn write_plates(roster: &Roster, dir: &Path) -> Result<usize, String> {
    if STROKE_PT < crate::coloring::LINE_MIN_PT {
        return Err(format!(
            "stroke {STROKE_PT} pt below KDP floor {}",
            crate::coloring::LINE_MIN_PT
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let pages = interior_pages(roster);
    let start = roster.front_matter_pages; // last front page; first verso = start+1
    let mut n = 0usize;
    let mut ordinal = 0u32;
    for car in &roster.cars {
        for (i, view) in views_for(car).into_iter().enumerate() {
            let verso_page = start + 1 + ordinal * 2;
            let recto_page = verso_page + 1;
            let slug = car_slug(car);
            let vpath = dir.join(format!("{slug}-p{i}-verso.svg"));
            let rpath = dir.join(format!("{slug}-p{i}-recto.svg"));
            std::fs::write(&vpath, page_svg(car, view, Side::Verso, verso_page, pages))
                .map_err(|e| e.to_string())?;
            std::fs::write(&rpath, page_svg(car, view, Side::Recto, recto_page, pages))
                .map_err(|e| e.to_string())?;
            n += 2;
            ordinal += 1;
        }
    }
    Ok(n)
}

fn verso_inner(car: &Car, view: View, safe: &SafeBox) -> String {
    let mark_s = MARK_PT / 240.0;
    let mx = safe.x + safe.w - MARK_PT - 8.0;
    let my = safe.y + 8.0;
    let tx = safe.x + 16.0;
    let mut y = safe.y + 64.0;
    let mut t = String::new();
    t.push_str(&format!(
        r#"<g id="mark" transform="translate({mx:.2} {my:.2}) scale({mark_s:.4})" stroke-width="3">{crest}</g>"#,
        crest = CREST
    ));
    t.push_str(&text(tx, y, 13.0, "400", "MAKE"));
    y += 36.0;
    t.push_str(&text(tx, y, 28.0, "700", &car.make.to_uppercase()));
    y += 44.0;
    t.push_str(&text(tx, y, 13.0, "400", "MODEL"));
    y += 36.0;
    t.push_str(&text(tx, y, 26.0, "700", &car.model));
    y += 44.0;
    t.push_str(&text(tx, y, 13.0, "400", "YEAR"));
    y += 36.0;
    t.push_str(&text(tx, y, 28.0, "700", &car.year.to_string()));
    y += 52.0;
    t.push_str(&text(
        tx,
        y,
        14.0,
        "400",
        &format!("view · {} · {}", view_label(view), car.why),
    ));
    t.push_str(&text(
        tx,
        safe.y + safe.h - 28.0,
        12.0,
        "400",
        "pencils / crayons · markers: slip a sheet under this verso",
    ));
    t
}

fn recto_inner(car: &Car, view: View, safe: &SafeBox) -> String {
    let fam = family_of(car);
    let ox = safe.x + safe.w * 0.06;
    let oy = safe.y + safe.h * 0.28;
    let sx = safe.w * 0.88 / 100.0;
    let sy = safe.h * 0.42 / 40.0;
    format!(
        r#"<g id="car" transform="translate({ox:.2} {oy:.2}) scale({sx:.4} {sy:.4})" stroke-width="{sw:.3}">{paths}</g>"#,
        sw = STROKE_PT / sx.min(sy),
        paths = car_paths(fam, view)
    )
}

fn view_label(v: View) -> &'static str {
    match v {
        View::ThreeQuarter => "¾",
        View::Profile => "profile",
        View::Rear => "rear",
        View::Front => "front",
        View::Badge => "detail",
    }
}

fn text(x: f64, y: f64, size: f64, weight: &str, s: &str) -> String {
    format!(
        r#"<text x="{x:.2}" y="{y:.2}" font-family="Georgia,serif" font-size="{size}" font-weight="{weight}" fill="black" stroke="none">{t}</text>"#,
        t = esc(s)
    )
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Garage crest paths (from `samples/coloring-mark.svg`), 240×240.
const CREST: &str = r#"<polygon points="120,18 210,58 210,150 120,222 30,150 30,58"/>
<polygon points="120,48 178,78 178,142 120,186 62,142 62,78"/>
<rect x="96" y="100" width="48" height="36"/>
<path d="M84 118 H156 M120 100 V136"/>
<circle cx="120" cy="78" r="10"/>
<path d="M70 168 Q120 198 170 168"/>"#;

fn wheels(x1: f64, x2: f64, y: f64, r: f64) -> String {
    format!(
        r#"<circle cx="{x1}" cy="{y}" r="{r}"/><circle cx="{x1}" cy="{y}" r="{ir}"/><circle cx="{x2}" cy="{y}" r="{r}"/><circle cx="{x2}" cy="{y}" r="{ir}"/>"#,
        ir = r * 0.42
    )
}

fn car_paths(family: Family, view: View) -> String {
    match view {
        View::Front => front_paths(family),
        View::Rear => rear_paths(family),
        View::Badge => badge_paths(family),
        View::Profile | View::ThreeQuarter => profile_paths(family, view == View::ThreeQuarter),
    }
}

fn profile_paths(family: Family, three_q: bool) -> String {
    let mut p = String::new();
    // ground line
    p.push_str(r#"<path d="M 4 34 H 96"/>"#);
    let body = match family {
        Family::Fins => {
            r#"<path d="M 10 30 Q 14 18 28 16 L 52 15 Q 68 10 82 14 L 94 20 L 92 28 L 78 26 Q 70 22 58 24 L 28 26 Q 16 28 12 32 Z"/>
<path d="M 78 18 L 96 8 L 94 20"/>"#
        }
        Family::Split => {
            r#"<path d="M 12 31 Q 18 16 36 14 L 62 13 Q 78 14 88 22 L 90 30 L 78 28 L 36 28 Q 20 29 14 32 Z"/>
<path d="M 58 14 L 64 8 L 72 14"/>"#
        }
        Family::Roadster => {
            r#"<path d="M 8 31 L 18 28 L 34 20 L 58 18 L 78 22 L 90 28 L 88 32 L 12 33 Z"/>
<path d="M 36 20 L 40 12 L 54 12 L 58 18"/>
<path d="M 22 31 L 70 31"/>"#
        }
        Family::Buttress => {
            r#"<path d="M 10 31 Q 16 18 32 16 L 58 15 Q 74 16 86 20 L 90 30 L 78 28 Q 70 18 58 20 L 32 24 Q 18 26 12 32 Z"/>
<path d="M 62 16 L 74 10 L 80 20"/>"#
        }
        Family::Ebody => {
            r#"<path d="M 11 31 Q 18 17 34 15 L 60 14 Q 76 15 88 22 L 90 30 L 34 28 Q 18 29 13 32 Z"/>
<path d="M 40 15 L 48 10 L 56 15"/>"#
        }
        Family::Pony => {
            r#"<path d="M 12 31 Q 20 18 36 16 L 58 15 Q 74 16 86 22 L 88 30 L 36 28 Q 20 29 14 32 Z"/>
<path d="M 44 16 Q 50 11 56 16"/>"#
        }
        Family::Hotrod => {
            r#"<path d="M 14 30 L 26 22 L 40 20 L 62 20 L 78 24 L 86 30 L 20 32 Z"/>
<path d="M 28 22 L 32 12 L 44 12 L 46 20"/>"#
        }
        Family::Tbird => {
            r#"<path d="M 12 31 Q 20 18 38 16 L 64 16 Q 80 18 88 26 L 86 31 L 38 28 Q 22 29 14 32 Z"/>
<path d="M 70 18 A 4 4 0 0 1 70 26"/>"#
        }
        Family::TransAm => {
            r#"<path d="M 11 31 Q 18 17 36 15 L 62 14 Q 78 16 88 24 L 88 31 L 36 28 Q 18 29 13 32 Z"/>
<path d="M 70 16 L 92 12 L 88 24"/>"#
        }
        Family::Sled => {
            r#"<path d="M 10 32 Q 16 20 30 16 L 70 15 Q 86 16 92 24 L 90 31 L 16 32 Z"/>"#
        }
        Family::Racer => {
            r#"<path d="M 8 28 L 24 22 L 48 18 L 78 20 L 94 26 L 90 30 L 12 30 Z"/>
<path d="M 70 20 L 88 16 L 90 26"/>"#
        }
        Family::Muscle => {
            r#"<path d="M 11 31 Q 18 18 34 16 L 62 15 Q 78 16 88 23 L 90 31 L 34 28 Q 18 29 13 32 Z"/>"#
        }
    };
    p.push_str(body);
    p.push_str(&wheels(24.0, 76.0, 32.0, 6.5));
    if three_q {
        p.push_str(r#"<path d="M 36 16 Q 44 20 52 16"/><path d="M 88 24 L 96 28"/>"#);
    }
    if matches!(family, Family::Roadster) {
        p.push_str(r#"<path d="M 28 31 L 30 36 M 34 31 L 36 36 M 40 31 L 42 36"/>"#);
    }
    if matches!(family, Family::Ebody | Family::Pony) {
        p.push_str(r#"<path d="M 42 16 L 46 13 L 50 16"/>"#);
    }
    p
}

fn front_paths(family: Family) -> String {
    let mut p = String::from(r#"<path d="M 20 34 H 80"/>"#);
    p.push_str(r#"<path d="M 28 30 Q 50 12 72 30 L 70 34 L 30 34 Z"/>"#);
    p.push_str(r#"<path d="M 34 22 L 66 22 L 64 28 L 36 28 Z"/>"#);
    p.push_str(&wheels(34.0, 66.0, 33.0, 5.5));
    match family {
        Family::Fins => p.push_str(r#"<path d="M 30 18 L 34 12 M 70 18 L 66 12"/>"#),
        Family::Ebody | Family::Pony => {
            p.push_str(r#"<path d="M 46 16 L 50 12 L 54 16"/><path d="M 38 26 H 62"/>"#)
        }
        Family::Roadster => p.push_str(r#"<path d="M 40 18 L 50 8 L 60 18"/>"#),
        Family::Racer => p.push_str(r#"<path d="M 32 26 H 68 M 50 12 V 26"/>"#),
        _ => p.push_str(r#"<path d="M 42 26 H 58"/>"#),
    }
    p
}

fn rear_paths(family: Family) -> String {
    let mut p = String::from(r#"<path d="M 22 34 H 78"/>"#);
    p.push_str(r#"<path d="M 30 32 Q 50 14 70 32 L 66 34 L 34 34 Z"/>"#);
    p.push_str(&wheels(36.0, 64.0, 33.0, 5.5));
    match family {
        Family::Fins => p.push_str(
            r#"<path d="M 32 18 L 28 6 L 36 20 M 68 18 L 72 6 L 64 20"/><path d="M 40 24 H 60"/>"#,
        ),
        Family::Split => {
            p.push_str(r#"<path d="M 38 18 L 48 16 L 48 26 L 38 26 Z"/><path d="M 52 16 L 62 18 L 62 26 L 52 26 Z"/>"#)
        }
        Family::Buttress => p.push_str(r#"<path d="M 34 16 Q 50 10 66 16 L 62 28 L 38 28 Z"/>"#),
        Family::TransAm => p.push_str(r#"<path d="M 28 14 H 72 L 68 18 H 32 Z"/>"#),
        _ => p.push_str(r#"<path d="M 40 22 H 60 M 44 26 H 56"/>"#),
    }
    p
}

fn badge_paths(family: Family) -> String {
    let mut p = String::from(r#"<rect x="30" y="8" width="40" height="26" rx="3"/>"#);
    p.push_str(r#"<circle cx="50" cy="21" r="8"/>"#);
    match family {
        Family::Roadster => p.push_str(r#"<path d="M 20 36 Q 50 30 80 36"/>"#),
        Family::Ebody => p.push_str(r#"<path d="M 42 12 L 50 6 L 58 12"/>"#),
        Family::Split => {
            p.push_str(r#"<path d="M 42 16 H 48 V 26 H 42 Z M 52 16 H 58 V 26 H 52 Z"/>"#)
        }
        Family::Fins => p.push_str(r#"<path d="M 34 10 L 38 4 M 66 10 L 62 4"/>"#),
        _ => p.push_str(r#"<path d="M 44 21 H 56"/>"#),
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coloring::{kdp_ok, load_roster, plate_count};

    #[test]
    fn kit_emits_caption_and_line_art() {
        let r = load_roster().unwrap();
        kdp_ok(&r).unwrap();
        let car = &r.cars[0];
        let pages = interior_pages(&r);
        let v = page_svg(car, View::ThreeQuarter, Side::Verso, 10, pages);
        assert!(v.contains("viewBox=\"0 0 612 792\""));
        assert!(v.contains("Cadillac"));
        assert!(v.contains("Eldorado"));
        assert!(v.contains("1959"));
        assert!(v.contains("id=\"mark\""));
        assert!(v.contains("stroke-width=\"1.25\""));
        assert!(!v.to_ascii_lowercase().contains("<script"));
        let art = page_svg(car, View::Rear, Side::Recto, 11, pages);
        assert!(art.contains("id=\"car\""));
        assert!(art.contains("<circle"));
        let verso_safe = safe_box(Side::Verso, pages);
        let recto_safe = safe_box(Side::Recto, pages);
        assert!(
            verso_safe.x < recto_safe.x,
            "recto gutter is the left inset"
        );
        assert_eq!(views_for(car).len(), 3);
        assert_eq!(family_of(car), Family::Fins);
    }

    #[test]
    fn write_plates_matches_roster() {
        let r = load_roster().unwrap();
        let dir = std::env::temp_dir().join("rebook-coloring-plates");
        let _ = std::fs::remove_dir_all(&dir);
        let n = write_plates(&r, &dir).unwrap();
        assert_eq!(n as u32, plate_count(&r) * 2);
        let sample = dir.join("1959-cadillac-eldorado-biarritz-p0-verso.svg");
        assert!(sample.is_file());
        let cobra = dir.join("1967-shelby-cobra-427-p2-recto.svg");
        assert!(cobra.is_file(), "hero third plate exists");
    }
}
