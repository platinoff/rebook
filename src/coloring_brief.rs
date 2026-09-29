//! Art brief for a coloring book (RB-58): one entry per master PNG with the
//! body kind, view, target frame on the master canvas, and the custom build
//! every view must repeat. Written into the local book folder, never into git.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::coloring::{Roster, body_kind, kdp_ok};
use crate::coloring_svg::{
    Side, View, art_png_name, identity_png_name, safe_box, subject_box, views_for,
};

/// Master canvas, pixels (3:4 portrait, same shape as the book-1 masters).
pub const MASTER_W: u32 = 2400;
/// Master canvas height, pixels.
pub const MASTER_H: u32 = 3200;

/// Rule printed on every brief when the roster has none.
pub const DEFAULT_RULE: &str = "Same custom build in the color plate and all four contour views. \
No logos, badges, lettering or plates anywhere in the art.";

/// Target rectangle on the master canvas, pixels.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PxBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// One master PNG to draw.
#[derive(Debug, Clone, Serialize)]
pub struct BriefItem {
    pub file: String,
    pub vehicle: String,
    pub kind: String,
    pub view: String,
    /// Full-color identity plate (true) or black contour on white (false).
    pub color: bool,
    pub custom: String,
    pub aspect: f64,
    pub frame: PxBox,
}

/// Whole brief for the active roster.
#[derive(Debug, Clone, Serialize)]
pub struct Brief {
    pub title: String,
    pub rule: String,
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub items: Vec<BriefItem>,
}

/// Default output: `--book DIR/build/brief`, else `build/coloring-brief`.
pub fn default_dir() -> PathBuf {
    match crate::paths::book_dir() {
        Some(book) => book.join("build").join("brief"),
        None => PathBuf::from("build/coloring-brief"),
    }
}

/// Build the brief (identity plate framed like the ¾ view, then four contours).
pub fn build(roster: &Roster) -> Result<Brief, String> {
    kdp_ok(roster)?;
    let safe = safe_box(Side::Recto, crate::coloring::interior_pages(roster));
    let art_h = safe.h - 16.0;
    let px = |b: &crate::coloring_svg::SafeBox| PxBox {
        x: (((b.x - safe.x) / safe.w) * MASTER_W as f64).round() as u32,
        y: (((b.y - safe.y) / art_h) * MASTER_H as f64).round() as u32,
        w: ((b.w / safe.w) * MASTER_W as f64).round() as u32,
        h: ((b.h / art_h) * MASTER_H as f64).round() as u32,
    };
    let mut items = Vec::new();
    for car in &roster.cars {
        let kind = body_kind(car)?;
        let vehicle = format!("{} {} {}", car.year, car.make, car.model);
        let tq = subject_box(&safe, kind, View::ThreeQuarter);
        items.push(BriefItem {
            file: identity_png_name(car),
            vehicle: vehicle.clone(),
            kind: kind.as_str().into(),
            view: "color identity (three-quarter)".into(),
            color: true,
            custom: car.custom.clone(),
            aspect: crate::coloring_svg::frame_aspect(kind, View::ThreeQuarter),
            frame: px(&tq),
        });
        for (i, view) in views_for(car).into_iter().enumerate() {
            let b = subject_box(&safe, kind, view);
            items.push(BriefItem {
                file: art_png_name(car, i),
                vehicle: vehicle.clone(),
                kind: kind.as_str().into(),
                view: view_name(view).into(),
                color: false,
                custom: car.custom.clone(),
                aspect: crate::coloring_svg::frame_aspect(kind, view),
                frame: px(&b),
            });
        }
    }
    let rule = if roster.rule.trim().is_empty() {
        DEFAULT_RULE.to_string()
    } else {
        roster.rule.clone()
    };
    Ok(Brief {
        title: roster.title_en.clone(),
        rule,
        canvas_w: MASTER_W,
        canvas_h: MASTER_H,
        items,
    })
}

/// Write `art-brief.json` + `art-brief.md` into `dir`. Returns item count.
pub fn write(roster: &Roster, dir: &Path) -> Result<usize, String> {
    let brief = build(roster)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("mkdir brief: {e}"))?;
    let json = serde_json::to_string_pretty(&brief).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("art-brief.json"), json).map_err(|e| format!("write json: {e}"))?;
    std::fs::write(dir.join("art-brief.md"), to_markdown(&brief))
        .map_err(|e| format!("write md: {e}"))?;
    Ok(brief.items.len())
}

fn view_name(v: View) -> &'static str {
    match v {
        View::ThreeQuarter => "three-quarter",
        View::Profile => "profile",
        View::Rear => "rear",
        View::Front => "front",
        View::Badge => "detail",
    }
}

fn to_markdown(b: &Brief) -> String {
    let mut md = format!(
        "# Art brief — {}\n\n**Rule:** {}\n\nCanvas {}×{} px, white background. Contour plates: black line only, no fills, no gray. \
Keep the vehicle inside the frame box; nothing in the outer margin.\n",
        b.title, b.rule, b.canvas_w, b.canvas_h
    );
    let mut last = String::new();
    for it in &b.items {
        if it.vehicle != last {
            md.push_str(&format!(
                "\n## {} ({})\n\nCustom build: {}\n\n| File | View | Frame x,y,w,h px | Aspect |\n|---|---|---|---|\n",
                it.vehicle,
                it.kind,
                if it.custom.is_empty() { "—" } else { &it.custom }
            ));
            last = it.vehicle.clone();
        }
        md.push_str(&format!(
            "| `{}` | {}{} | {},{},{},{} | {:.2} |\n",
            it.file,
            it.view,
            if it.color { " · color" } else { "" },
            it.frame.x,
            it.frame.y,
            it.frame.w,
            it.frame.h,
            it.aspect
        ));
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coloring::load_roster;

    #[test]
    fn brief_lists_every_master_inside_canvas() {
        let r = load_roster().unwrap();
        let b = build(&r).unwrap();
        assert_eq!(b.items.len(), r.cars.len() * 5);
        assert_eq!(b.items.iter().filter(|i| i.color).count(), r.cars.len());
        for it in &b.items {
            assert!(it.frame.x + it.frame.w <= MASTER_W);
            assert!(it.frame.y + it.frame.h <= MASTER_H);
            assert!(it.frame.w > 0 && it.frame.h > 0);
        }
        assert_eq!(b.rule, DEFAULT_RULE);
        let md = to_markdown(&b);
        assert!(md.contains("| `") && md.contains("Custom build"));
    }

    #[test]
    fn truck_kinds_change_the_frame() {
        let mut r = load_roster().unwrap();
        r.cars.iter_mut().for_each(|c| c.kind = "cabover".into());
        let cab = build(&r).unwrap();
        r.cars.iter_mut().for_each(|c| c.kind = "car".into());
        let car = build(&r).unwrap();
        let front = |b: &Brief| {
            b.items
                .iter()
                .find(|i| i.view == "front")
                .unwrap()
                .frame
                .clone()
        };
        assert!(front(&cab).h > front(&car).h);
        assert_eq!(cab.items[0].kind, "cabover");
    }
}
