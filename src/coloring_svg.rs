//! SVG plate kit for the Classic American Iron coloring paperback (RB-51).
//!
//! Pages are **8.5×11 in at 72 pt/in** (`viewBox="0 0 612 792"`). Stroke is
//! **1.25 pt** (above the KDP 0.75 pt graphic floor). No bleed: art lives in
//! the safe box (0.25″ outside, gutter from [`crate::standards::gutter_in`]).
//! Each car: one **color identity** plate (make / model / year once), then
//! four contour views (¾, profile, rear, front) with no repeated caption.

use std::path::{Path, PathBuf};

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

/// Caption language on the verso (write-then-translate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionLang {
    /// Source edition.
    Uk,
    /// KDP listing fork.
    En,
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
        3 => vec![View::ThreeQuarter, View::Profile, View::Rear],
        _ => vec![View::ThreeQuarter, View::Profile, View::Rear, View::Front],
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

/// Binding side from 1-based page number (odd = recto).
pub fn side_for_page(page: u32) -> Side {
    if page.is_multiple_of(2) {
        Side::Verso
    } else {
        Side::Recto
    }
}

/// Filename for one plate page (`{slug}-pN-verso.svg`).
pub fn plate_file(car: &Car, plate_i: usize, side: Side) -> String {
    let slug = car_slug(car);
    match side {
        Side::Verso => format!("{slug}-p{plate_i}-verso.svg"),
        Side::Recto => format!("{slug}-p{plate_i}-recto.svg"),
    }
}

/// Color identity page (`{slug}-color.svg`).
pub fn identity_file(car: &Car) -> String {
    format!("{}-color.svg", car_slug(car))
}

/// Master color plate (`{slug}-color.png`).
pub fn identity_png_name(car: &Car) -> String {
    format!("{}-color.png", car_slug(car))
}

/// One SVG page (verso caption or recto art). English captions (CLI default).
pub fn page_svg(car: &Car, view: View, side: Side, page: u32, pages: u32) -> String {
    page_svg_lang(car, view, side, page, pages, CaptionLang::En)
}

/// One SVG page with verso labels in `lang`.
pub fn page_svg_lang(
    car: &Car,
    view: View,
    side: Side,
    page: u32,
    pages: u32,
    lang: CaptionLang,
) -> String {
    let safe = safe_box(side, pages);
    let body = match side {
        Side::Verso => verso_inner(car, view, &safe, lang),
        Side::Recto => recto_inner(car, view, &safe, lang),
    };
    wrap_svg(
        &format!("{} {} {} {:?}", car.year, car.make, car.model, view),
        &safe,
        &body,
        page,
    )
}

/// Color identity plate (make · model · year once).
pub fn identity_svg(car: &Car, page: u32, pages: u32, lang: CaptionLang) -> String {
    let side = side_for_page(page);
    let safe = safe_box(side, pages);
    wrap_svg(
        &format!("{} {} {}", car.year, car.make, car.model),
        &safe,
        &identity_inner(car, &safe, lang),
        page,
    )
}

/// Contour coloring plate (no make/model/year — identity already printed).
pub fn contour_svg(car: &Car, view: View, page: u32, pages: u32, lang: CaptionLang) -> String {
    let side = side_for_page(page);
    let safe = safe_box(side, pages);
    wrap_svg(
        &format!("{} {} {} {:?}", car.year, car.make, car.model, view),
        &safe,
        &recto_inner(car, view, &safe, lang),
        page,
    )
}

fn wrap_svg(title: &str, safe: &SafeBox, body: &str, page: u32) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 {w} {h}" width="8.5in" height="11in" fill="none" stroke="#000" stroke-width="{sw}" stroke-linejoin="round" stroke-linecap="round">
<title>{title}</title>
<rect id="trim" x="0" y="0" width="{w}" height="{h}" fill="#fff" stroke="none"/>
<rect id="safe" x="{sx:.2}" y="{sy:.2}" width="{swd:.2}" height="{sh:.2}" stroke="#000" stroke-width="0" fill="none"/>
{body}
<text id="pgn" x="{px:.2}" y="{py:.2}" font-family="Georgia,serif" font-size="11" fill="#000" stroke="none" text-anchor="middle">{page}</text>
</svg>
"##,
        w = PAGE_W,
        h = PAGE_H,
        sw = STROKE_PT,
        title = esc(title),
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

/// Write identity + contour pages for every car into `dir`. Returns file count.
pub fn write_plates(roster: &Roster, dir: &Path) -> Result<usize, String> {
    write_plates_lang(roster, dir, CaptionLang::En)
}

/// Write pages with identity captions in `lang`.
pub fn write_plates_lang(roster: &Roster, dir: &Path, lang: CaptionLang) -> Result<usize, String> {
    if STROKE_PT < crate::coloring::LINE_MIN_PT {
        return Err(format!(
            "stroke {STROKE_PT} pt below KDP floor {}",
            crate::coloring::LINE_MIN_PT
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let pages = interior_pages(roster);
    let mut page = roster.front_matter_pages;
    let mut n = 0usize;
    for car in &roster.cars {
        page += 1;
        let ipath = dir.join(identity_file(car));
        std::fs::write(&ipath, identity_svg(car, page, pages, lang)).map_err(|e| e.to_string())?;
        if let Some(src) = identity_png_src(car) {
            let dest = dir.join(identity_png_name(car));
            std::fs::copy(&src, &dest).map_err(|e| format!("copy color: {e}"))?;
        }
        n += 1;
        for (i, view) in views_for(car).into_iter().enumerate() {
            page += 1;
            let rpath = dir.join(plate_file(car, i, Side::Recto));
            std::fs::write(&rpath, contour_svg(car, view, page, pages, lang))
                .map_err(|e| e.to_string())?;
            if let Some(src) = art_png_src(car, i) {
                let dest = dir.join(art_png_name(car, i));
                std::fs::copy(&src, &dest).map_err(|e| format!("copy art: {e}"))?;
            }
            n += 1;
        }
    }
    Ok(n)
}

fn identity_inner(car: &Car, safe: &SafeBox, lang: CaptionLang) -> String {
    if identity_png_src(car).is_some() {
        let name = identity_png_name(car);
        return format!(
            r#"<image id="identity" href="{name}" xlink:href="{name}" x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" preserveAspectRatio="xMidYMid slice"/>"#,
            x = safe.x,
            y = safe.y,
            w = safe.w,
            h = safe.h,
        );
    }
    let cx = safe.x + safe.w / 2.0;
    let (make_l, model_l, year_l) = match lang {
        CaptionLang::Uk => ("МАРКА", "МОДЕЛЬ", "РІК"),
        CaptionLang::En => ("MAKE", "MODEL", "YEAR"),
    };
    let mut t = String::new();
    t.push_str(&format!(
        r#"<text x="{cx:.2}" y="{y:.2}" text-anchor="middle" font-family="Georgia,serif" font-size="14" font-weight="400" fill="black" stroke="none">{year_l}</text>"#,
        y = safe.y + 120.0,
    ));
    t.push_str(&format!(
        r#"<text x="{cx:.2}" y="{y:.2}" text-anchor="middle" font-family="Georgia,serif" font-size="36" font-weight="700" fill="black" stroke="none">{n}</text>"#,
        y = safe.y + 168.0,
        n = esc(&car.year.to_string()),
    ));
    t.push_str(&format!(
        r#"<text x="{cx:.2}" y="{y:.2}" text-anchor="middle" font-family="Georgia,serif" font-size="14" font-weight="400" fill="black" stroke="none">{make_l}</text>"#,
        y = safe.y + 230.0,
    ));
    t.push_str(&format!(
        r#"<text x="{cx:.2}" y="{y:.2}" text-anchor="middle" font-family="Georgia,serif" font-size="32" font-weight="700" fill="black" stroke="none">{n}</text>"#,
        y = safe.y + 276.0,
        n = esc(&car.make.to_uppercase()),
    ));
    t.push_str(&format!(
        r#"<text x="{cx:.2}" y="{y:.2}" text-anchor="middle" font-family="Georgia,serif" font-size="14" font-weight="400" fill="black" stroke="none">{model_l}</text>"#,
        y = safe.y + 338.0,
    ));
    t.push_str(&format!(
        r#"<text x="{cx:.2}" y="{y:.2}" text-anchor="middle" font-family="Georgia,serif" font-size="28" font-weight="700" fill="black" stroke="none">{n}</text>"#,
        y = safe.y + 384.0,
        n = esc(&car.model),
    ));
    t
}

fn identity_png_src(car: &Car) -> Option<PathBuf> {
    let p = art_dir().join(identity_png_name(car));
    p.is_file().then_some(p)
}

fn verso_inner(car: &Car, view: View, safe: &SafeBox, lang: CaptionLang) -> String {
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
    let (make_l, model_l, year_l, why, footer) = match lang {
        CaptionLang::Uk => (
            "МАРКА",
            "МОДЕЛЬ",
            "РІК",
            car.why.as_str(),
            "олівці / крейда · маркери: підклади аркуш під цей verso",
        ),
        CaptionLang::En => (
            "MAKE",
            "MODEL",
            "YEAR",
            car.why_en.as_str(),
            "pencils / crayons · markers: slip a sheet under this verso",
        ),
    };
    t.push_str(&text(tx, y, 13.0, "400", make_l));
    y += 36.0;
    t.push_str(&text(tx, y, 28.0, "700", &car.make.to_uppercase()));
    y += 44.0;
    t.push_str(&text(tx, y, 13.0, "400", model_l));
    y += 36.0;
    t.push_str(&text(tx, y, 26.0, "700", &car.model));
    y += 44.0;
    t.push_str(&text(tx, y, 13.0, "400", year_l));
    y += 36.0;
    t.push_str(&text(tx, y, 28.0, "700", &car.year.to_string()));
    y += 52.0;
    t.push_str(&text(
        tx,
        y,
        14.0,
        "400",
        &format!("view · {} · {}", view_label(view, lang), why),
    ));
    t.push_str(&text(tx, safe.y + safe.h - 28.0, 12.0, "400", footer));
    t
}

/// Master line-art PNGs for print plates (`samples/coloring/art`).
pub fn art_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples/coloring/art")
}

/// `{slug}-pN.png` next to the recto SVG.
pub fn art_png_name(car: &Car, plate_i: usize) -> String {
    format!("{}-p{plate_i}.png", car_slug(car))
}

fn art_png_src(car: &Car, plate_i: usize) -> Option<PathBuf> {
    let p = art_dir().join(art_png_name(car, plate_i));
    p.is_file().then_some(p)
}

fn plate_index(car: &Car, view: View) -> usize {
    views_for(car).iter().position(|v| *v == view).unwrap_or(0)
}

/// Local drawing canvas (car group), points before page scale.
const CAR_W: f64 = 520.0;
const CAR_H: f64 = 186.0;

fn recto_inner(car: &Car, view: View, safe: &SafeBox, lang: CaptionLang) -> String {
    let i = plate_index(car, view);
    let label_y = safe.y + safe.h - 8.0;
    let label = format!(
        r#"<text x="{x:.2}" y="{label_y:.2}" text-anchor="middle" font-family="Georgia,serif" font-size="11" fill="black" stroke="none">{v}</text>"#,
        x = safe.x + safe.w / 2.0,
        v = esc(view_label(view, lang)),
    );
    if art_png_src(car, i).is_some() {
        let name = art_png_name(car, i);
        return format!(
            r#"<image id="car" href="{name}" xlink:href="{name}" x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" preserveAspectRatio="xMidYMid meet"/>{label}"#,
            x = safe.x,
            y = safe.y,
            w = safe.w,
            h = safe.h - 16.0,
        );
    }
    let sx = safe.w * 0.92 / CAR_W;
    let sy = (safe.h - 16.0) * 0.72 / CAR_H;
    let s = sx.min(sy);
    let ox = safe.x + (safe.w - CAR_W * s) * 0.5;
    let oy = safe.y + (safe.h - 16.0 - CAR_H * s) * 0.48;
    format!(
        r#"<g id="car" transform="translate({ox:.2} {oy:.2}) scale({s:.4} {s:.4})" stroke-width="{sw:.3}">{paths}</g>{label}"#,
        sw = STROKE_PT / s,
        paths = crate::coloring_art::draw(car, view)
    )
}

fn view_label(v: View, lang: CaptionLang) -> &'static str {
    match (v, lang) {
        (View::ThreeQuarter, CaptionLang::Uk) => "¾",
        (View::Profile, CaptionLang::Uk) => "профіль",
        (View::Rear, CaptionLang::Uk) => "зад",
        (View::Front, CaptionLang::Uk) => "фас",
        (View::Badge, CaptionLang::Uk) => "деталь",
        (View::ThreeQuarter, CaptionLang::En) => "¾",
        (View::Profile, CaptionLang::En) => "profile",
        (View::Rear, CaptionLang::En) => "rear",
        (View::Front, CaptionLang::En) => "front",
        (View::Badge, CaptionLang::En) => "detail",
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
        assert!(v.contains("YEAR"));
        assert!(v.contains("id=\"mark\""));
        let uk = page_svg_lang(
            car,
            View::ThreeQuarter,
            Side::Verso,
            10,
            pages,
            CaptionLang::Uk,
        );
        assert!(uk.contains("МАРКА"));
        assert!(uk.contains("РІК"));
        assert!(v.contains("stroke-width=\"1.25\""));
        assert!(!v.to_ascii_lowercase().contains("<script"));
        let art = page_svg(car, View::Rear, Side::Recto, 11, pages);
        assert!(art.contains("id=\"car\""));
        assert!(
            art.contains("<image") || art.contains("<circle") || art.contains("<ellipse"),
            "recto is print line art (raster plate or vector fallback)"
        );
        let verso_safe = safe_box(Side::Verso, pages);
        let recto_safe = safe_box(Side::Recto, pages);
        assert!(
            verso_safe.x < recto_safe.x,
            "recto gutter is the left inset"
        );
        assert_eq!(views_for(car).len(), 4);
        assert_eq!(family_of(car), Family::Fins);
        let idn = identity_svg(car, 9, pages, CaptionLang::En);
        assert!(idn.contains("Cadillac") || idn.contains("identity"));
        let contour = contour_svg(car, View::Profile, 10, pages, CaptionLang::En);
        assert!(!contour.contains("MAKE"));
        assert!(contour.contains("profile"));
    }

    #[test]
    fn write_plates_matches_roster() {
        let r = load_roster().unwrap();
        let dir = std::env::temp_dir().join("rebook-coloring-plates");
        let _ = std::fs::remove_dir_all(&dir);
        let n = write_plates(&r, &dir).unwrap();
        assert_eq!(n as u32, r.cars.len() as u32 + plate_count(&r));
        let sample = dir.join("1959-cadillac-eldorado-biarritz-color.svg");
        assert!(sample.is_file());
        let cobra = dir.join("1967-shelby-cobra-427-p0-recto.svg");
        assert!(cobra.is_file(), "four contour views per model");
        let last = dir.join("1967-shelby-cobra-427-p3-recto.svg");
        assert!(last.is_file());
    }
}
