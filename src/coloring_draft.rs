//! Studio seed for Classic American Iron (RB-52).
//!
//! Writes a Ukrainian paperback draft (`classic-american-iron`) with front
//! matter, one chapter per car, SVG includes under `assets/`, then forks the
//! English KDP listing edition (`classic-american-iron-en`). Studio page-view
//! treats each `![…](assets/…svg)` as one plate page.

use std::path::Path;

use crate::coloring::{Roster, interior_pages, kdp_ok, load_roster, plate_count};
use crate::coloring_svg::{
    CaptionLang, art_dir, art_png_name, identity_png_name, plate_file, views_for, write_plates_lang,
};
use crate::coverdoc::CoverDoc;
use crate::drafts::{
    DraftMeta, MetaPatch, create, draft_dir, fork_translation, persist_meta, save_chapter,
    save_cover, save_meta,
};

/// Seed id (ASCII slug of the EN listing title).
pub const DRAFT_ID: &str = "classic-american-iron";

/// Materialize uk + en Studio drafts under `root`. Returns `(uk, en)`.
pub fn seed(root: &Path) -> Result<(DraftMeta, DraftMeta), String> {
    let roster = load_roster()?;
    kdp_ok(&roster)?;
    std::fs::create_dir_all(root).map_err(|e| format!("mkdir drafts: {e}"))?;
    let en_id = format!("{DRAFT_ID}-en");
    for id in [DRAFT_ID, en_id.as_str()] {
        let dir = draft_dir(root, id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("clear {id}: {e}"))?;
        }
    }
    let created = create(root, roster.title_en.as_str(), roster.author.as_str(), "uk")?;
    if created.id != DRAFT_ID {
        return Err(format!("expected draft id {DRAFT_ID}, got {}", created.id));
    }
    let paperback = ["paperback".to_string()];
    let mut meta = save_meta(
        root,
        &created.id,
        &MetaPatch {
            title: Some(roster.title_uk.as_str()),
            author: Some(roster.author.as_str()),
            language: Some("uk"),
            formats: Some(&paperback),
            trim: Some("8.5x11"),
            pages: Some(interior_pages(&roster)),
            isbn: None,
        },
    )?;
    meta.source_language = Some("uk".into());
    persist_meta(root, &meta)?;
    let assets = draft_dir(root, &meta.id).join("assets");
    write_plates_lang(&roster, &assets, CaptionLang::Uk)?;
    write_chapters(root, &meta.id, &roster, CaptionLang::Uk)?;
    let mut en = fork_translation(root, &meta.id, "en")?;
    en.title = roster.title_en.clone();
    persist_meta(root, &en)?;
    write_plates_lang(
        &roster,
        &draft_dir(root, &en.id).join("assets"),
        CaptionLang::En,
    )?;
    write_chapters(root, &en.id, &roster, CaptionLang::En)?;
    seed_wrap(root, &meta.id, &roster)?;
    seed_wrap(root, &en.id, &roster)?;
    Ok((
        crate::drafts::load(root, &meta.id)?,
        crate::drafts::load(root, &en.id)?,
    ))
}

/// English paperback wrap: Cobra front + matching back blurb. Type lives in the art.
fn seed_wrap(root: &Path, id: &str, roster: &Roster) -> Result<(), String> {
    let mut doc = CoverDoc::new(
        roster.title_en.as_str(),
        roster.author.as_str(),
        "pb",
        "8.5x11",
        interior_pages(roster),
        "premium",
    );
    doc.title.text.clear();
    doc.author.text.clear();
    doc.bg_front = "#1a120c".into();
    doc.bg_back = "#1a120c".into();
    doc.spine_bg = Some("#120c08".into());
    let dir = draft_dir(root, id);
    std::fs::create_dir_all(dir.join("assets")).map_err(|e| e.to_string())?;
    let front = art_dir().join("cover-front.png");
    if front.is_file() {
        let bytes = std::fs::read(&front).map_err(|e| format!("read cover-front: {e}"))?;
        std::fs::write(dir.join("cover.png"), &bytes).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("assets").join("cover.png"), &bytes).map_err(|e| e.to_string())?;
        doc.front_image = Some(png_data_uri(&bytes));
    }
    let back = art_dir().join("cover-back.png");
    if back.is_file() {
        let bytes = std::fs::read(&back).map_err(|e| format!("read cover-back: {e}"))?;
        std::fs::write(dir.join("cover-back.png"), &bytes).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("assets").join("cover-back.png"), &bytes)
            .map_err(|e| e.to_string())?;
        doc.back_image = Some(png_data_uri(&bytes));
    }
    save_cover(root, id, &doc.to_json())
}

fn png_data_uri(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        crate::preflight::b64_encode(bytes)
    )
}

fn write_chapters(root: &Path, id: &str, roster: &Roster, lang: CaptionLang) -> Result<(), String> {
    let mut n = 1u32;
    for (title, body) in front_matter(roster, lang) {
        save_chapter(root, id, n, &title, &body, "md")?;
        n += 1;
    }
    for car in &roster.cars {
        let title = format!("{} {} {}", car.year, car.make, car.model);
        save_chapter(root, id, n, &title, &car_md(car, lang), "md")?;
        n += 1;
    }
    for (title, body) in back_matter(roster, lang) {
        save_chapter(root, id, n, &title, &body, "md")?;
        n += 1;
    }
    let expect = 8 + roster.cars.len() as u32 + 2;
    if n - 1 != expect {
        return Err(format!("chapter count {} ≠ {expect}", n - 1));
    }
    Ok(())
}

fn car_md(car: &crate::coloring::Car, lang: CaptionLang) -> String {
    let why = match lang {
        CaptionLang::Uk => car.why.as_str(),
        CaptionLang::En => car.why_en.as_str(),
    };
    let mut md = format!("> {why}\n\n");
    let color = identity_png_name(car);
    let color_href = if art_dir().join(&color).is_file() {
        color
    } else {
        format!("{}-color.svg", crate::coloring_svg::car_slug(car))
    };
    md.push_str(&format!(
        "![{} {} {} color](assets/{color_href})\n\n",
        car.year, car.make, car.model
    ));
    for (i, view) in views_for(car).into_iter().enumerate() {
        let png = art_png_name(car, i);
        let r = if art_dir().join(&png).is_file() {
            png
        } else {
            plate_file(car, i, crate::coloring_svg::Side::Recto)
        };
        let alt = format!("{} {} {} {:?}", car.year, car.make, car.model, view);
        md.push_str(&format!("![{alt}](assets/{r})\n\n"));
    }
    md
}

fn front_matter(roster: &Roster, lang: CaptionLang) -> Vec<(String, String)> {
    let cars = &roster.cars;
    let mid = cars.len() / 2;
    match lang {
        CaptionLang::Uk => vec![
            (
                "Титул".into(),
                format!(
                    "# {}\n\n{}\n\n{}\n",
                    roster.title_uk, roster.subtitle_uk, roster.author
                ),
            ),
            (
                "Copyright".into(),
                "## ©\n\nТекст і контурні ілюстрації — оригінал цього видання, не заводські креслення.\n\n\
Штучний інтелект допоміг намалювати пластини; автор вичитав і відповідає за книгу.\n\n\
Друк: Amazon KDP, paperback 8.5×11, преміум-колір, без вильоту.\n"
                    .into(),
            ),
            (
                "Ця книжка належить".into(),
                "## Ця книжка належить\n\nІм'я: ________________\n\nГараж / місто: ________________\n\nРік: ________________\n"
                    .into(),
            ),
            (
                "Як розмальовувати".into(),
                "## Як розмальовувати\n\n\
На кожну модель — одна кольорова сторінка (марка · модель · рік, один раз). Далі чотири контури: ¾, профіль, зад, фас — без повтору підпису.\n\n\
Олівець і крейда на контурі. Маркери просочують папір: підклади аркуш.\n\n\
Лінія ≥ 0.75 pt; у цьому виданні 1.25 pt.\n"
                    .into(),
            ),
            ("Зміст I".into(), toc_md(cars[..mid].iter(), CaptionLang::Uk)),
            ("Зміст II".into(), toc_md(cars[mid..].iter(), CaptionLang::Uk)),
            (
                "Присвята".into(),
                "## Присвята\n\nТим, хто впізнає плавці '59 і split-window '63 з пів силуета.\n"
                    .into(),
            ),
            (
                "Тачки".into(),
                format!(
                    "## Тачки\n\nДвадцять чотири класи. На модель: 1 колір + 4 контури. {} контурних плейтів.\n",
                    plate_count(roster)
                ),
            ),
        ],
        CaptionLang::En => vec![
            (
                "Title".into(),
                format!(
                    "# {}\n\n{}\n\n{}\n",
                    roster.title_en, roster.subtitle_en, roster.author
                ),
            ),
            (
                "Copyright".into(),
                "## ©\n\nText and contour illustrations are original to this edition, not factory blueprints.\n\n\
AI helped draw the plates; the author reviewed them and is responsible for the work.\n\n\
Print: Amazon KDP, paperback 8.5×11, premium color interior, no bleed.\n"
                    .into(),
            ),
            (
                "This book belongs to".into(),
                "## This book belongs to\n\nName: ________________\n\nGarage / city: ________________\n\nYear: ________________\n"
                    .into(),
            ),
            (
                "How to color".into(),
                "## How to color\n\n\
Each car opens with one color plate — make · model · year, once. Then four black contour views: three-quarter, profile, rear, front. No repeated captions. No badge plates.\n\n\
Pencil and crayon on the contour. Markers bleed: slip a sheet underneath.\n\n\
Line weight ≥ 0.75 pt; this edition draws at 1.25 pt.\n"
                    .into(),
            ),
            (
                "Contents I".into(),
                toc_md(cars[..mid].iter(), CaptionLang::En),
            ),
            (
                "Contents II".into(),
                toc_md(cars[mid..].iter(), CaptionLang::En),
            ),
            (
                "Dedication".into(),
                "## Dedication\n\nFor anyone who can name '59 fins and a '63 split-window from half a silhouette.\n"
                    .into(),
            ),
            (
                "The cars".into(),
                format!(
                    "## The cars\n\nTwenty-four classics. Each car: one color plate + four contour views. {} contour plates.\n",
                    plate_count(roster)
                ),
            ),
        ],
    }
}

fn toc_md<'a, I>(cars: I, lang: CaptionLang) -> String
where
    I: Iterator<Item = &'a crate::coloring::Car>,
{
    let mut md = match lang {
        CaptionLang::Uk => String::from("## Зміст\n\n"),
        CaptionLang::En => String::from("## Contents\n\n"),
    };
    for c in cars {
        md.push_str(&format!("{} · {} · {}\n\n", c.year, c.make, c.model));
    }
    md
}

fn back_matter(roster: &Roster, lang: CaptionLang) -> Vec<(String, String)> {
    let mut by_year: Vec<&crate::coloring::Car> = roster.cars.iter().collect();
    by_year.sort_by_key(|c| (c.year, c.make.as_str(), c.model.as_str()));
    let mut by_make = roster.cars.clone();
    by_make.sort_by(|a, b| {
        a.make
            .cmp(&b.make)
            .then(a.year.cmp(&b.year))
            .then(a.model.cmp(&b.model))
    });
    let (y_title, m_title, y_h, m_h) = match lang {
        CaptionLang::Uk => (
            "Покажчик за роком",
            "Покажчик за маркою",
            "## За роком",
            "## За маркою",
        ),
        CaptionLang::En => ("Index by year", "Index by make", "## By year", "## By make"),
    };
    let mut y = format!("{y_h}\n\n");
    for c in by_year {
        y.push_str(&format!("{} · {} {}\n\n", c.year, c.make, c.model));
    }
    let mut m = format!("{m_h}\n\n");
    for c in &by_make {
        m.push_str(&format!("{} · {} {}\n\n", c.make, c.year, c.model));
    }
    vec![(y_title.into(), y), (m_title.into(), m)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drafts::{chapter_content, load_asset};

    fn root() -> std::path::PathBuf {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("coloring-draft-test");
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn seed_uk_and_en_with_svg_includes() {
        let r = root();
        let (uk, en) = seed(&r).unwrap();
        assert_eq!(uk.id, DRAFT_ID);
        assert_eq!(uk.language, "uk");
        assert_eq!(uk.source_language.as_deref(), Some("uk"));
        assert_eq!(uk.formats, vec!["paperback".to_string()]);
        assert_eq!(uk.trim.as_deref(), Some("8.5x11"));
        assert_eq!(uk.pages, Some(130));
        assert_eq!(uk.chapters.len(), 8 + 24 + 2);
        let c1 = chapter_content(&r, &uk.id, 1).unwrap();
        assert!(c1.contains("Класичні американські тачки"));
        let car = chapter_content(&r, &uk.id, 9).unwrap();
        assert!(
            car.contains("](assets/1959-cadillac-eldorado-biarritz-color.png)")
                || car.contains("](assets/1959-cadillac-eldorado-biarritz-color.svg)"),
            "color identity once"
        );
        assert!(
            car.contains("](assets/1959-cadillac-eldorado-biarritz-p0-recto.svg)")
                || car.contains("](assets/1959-cadillac-eldorado-biarritz-p0.png)"),
            "first contour view"
        );
        assert!(
            car.contains("p3.png") || car.contains("p3-recto.svg"),
            "four contour views"
        );
        assert!(!car.contains("verso.svg"), "name is not repeated on versos");
        let svg = String::from_utf8(
            load_asset(&r, &uk.id, "1959-cadillac-eldorado-biarritz-color.svg").unwrap(),
        )
        .unwrap();
        assert!(svg.contains("Cadillac") || svg.contains("identity"));
        assert_eq!(en.id, format!("{DRAFT_ID}-en"));
        assert_eq!(en.language, "en");
        assert_eq!(en.translation_of.as_deref(), Some(DRAFT_ID));
        assert!(en.isbn.is_none());
        assert_eq!(en.title, "Classic American Iron");
        let en1 = chapter_content(&r, &en.id, 1).unwrap();
        assert!(en1.contains("Classic American Iron"));
        assert!(!en1.contains("Класичні"));
        let en_svg = String::from_utf8(
            load_asset(&r, &en.id, "1959-cadillac-eldorado-biarritz-color.svg").unwrap(),
        )
        .unwrap();
        assert!(
            en_svg.contains("MAKE") || en_svg.contains("identity") || en_svg.contains("Cadillac")
        );
        assert_eq!(en.chapters.len(), uk.chapters.len());
        if art_dir().join("cover-front.png").is_file() {
            let cj = crate::drafts::load_cover(&r, &en.id).expect("en wrap");
            let doc: CoverDoc = serde_json::from_str(&cj).unwrap();
            assert_eq!(doc.pages, 130);
            assert_eq!(doc.trim, "8.5x11");
            assert_eq!(doc.mode, "pb");
            assert!(doc.front_image.is_some());
            assert!(doc.back_image.is_some());
            assert!(doc.title.text.is_empty());
            assert!(doc.spine_title.is_some(), "130 pages get spine text");
            assert!(r.join(&en.id).join("cover.png").is_file());
            assert!(r.join(&en.id).join("cover-back.png").is_file());
        }
    }
}
