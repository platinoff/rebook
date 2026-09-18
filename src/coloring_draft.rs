//! Studio seed for Classic American Iron (RB-52).
//!
//! Writes a Ukrainian paperback draft (`classic-american-iron`) with front
//! matter, one chapter per car, SVG includes under `assets/`, then forks the
//! English KDP listing edition (`classic-american-iron-en`). Studio page-view
//! treats each `![…](assets/…svg)` as one plate page.

use std::path::Path;

use crate::coloring::{Roster, interior_pages, kdp_ok, load_roster, plate_count};
use crate::coloring_svg::{CaptionLang, Side, plate_file, views_for, write_plates_lang};
use crate::drafts::{
    DraftMeta, MetaPatch, create, draft_dir, fork_translation, persist_meta, save_chapter,
    save_meta,
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
    Ok((
        crate::drafts::load(root, &meta.id)?,
        crate::drafts::load(root, &en.id)?,
    ))
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
    for (i, view) in views_for(car).into_iter().enumerate() {
        let v = plate_file(car, i, Side::Verso);
        let r = plate_file(car, i, Side::Recto);
        let alt = format!("{} {} {} {:?}", car.year, car.make, car.model, view);
        md.push_str(&format!("![{alt} verso](assets/{v})\n\n"));
        md.push_str(&format!("![{alt} recto](assets/{r})\n\n"));
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
                "## ©\n\nТекст і креслення — авторські силуети, не ліцензовані креслення заводів.\n\n\
Штучний інтелект допоміг скласти чернетку; автор вичитав і відповідає за книгу.\n\n\
Друк: Amazon KDP, paperback 8.5×11, чорно-білий інтер'єр на білому папері.\n"
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
Олівець і крейда — на лицьовій (recto). Маркери просочують папір: підклади аркуш під зворот (verso).\n\n\
Verso — не порожня сторінка: марка · модель · рік і герб гаража. Так KDP не бачить «зайвих бланків».\n\n\
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
                    "## Тачки\n\nДвадцять чотири класи. Герої — три види, signature — два, прості — один. {} плейтів.\n",
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
                "## ©\n\nText and drawings are original silhouettes, not licensed factory blueprints.\n\n\
AI helped draft this book; the author reviewed it and is responsible for the work.\n\n\
Print: Amazon KDP, paperback 8.5×11, black interior on white paper.\n"
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
Pencil and crayon on the recto. Markers bleed: slip a sheet under the verso.\n\n\
The verso is not blank: make · model · year and the garage crest. KDP rejects runs of empty backs.\n\n\
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
                    "## The cars\n\nTwenty-four classics. Heroes get three views, signature two, simple one. {} plates.\n",
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
        md.push_str(&format!(
            "{} · {} · {} — {} ×{}\n\n",
            c.year, c.make, c.model, c.tier, c.plates
        ));
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
        assert_eq!(uk.pages, Some(104));
        assert_eq!(uk.chapters.len(), 8 + 24 + 2);
        let c1 = chapter_content(&r, &uk.id, 1).unwrap();
        assert!(c1.contains("Класичні американські тачки"));
        let car = chapter_content(&r, &uk.id, 9).unwrap();
        assert!(car.contains("](assets/1959-cadillac-eldorado-biarritz-p0-verso.svg)"));
        assert!(car.contains("](assets/1959-cadillac-eldorado-biarritz-p0-recto.svg)"));
        let svg = String::from_utf8(
            load_asset(&r, &uk.id, "1959-cadillac-eldorado-biarritz-p0-verso.svg").unwrap(),
        )
        .unwrap();
        assert!(svg.contains("МАРКА"));
        assert_eq!(en.id, format!("{DRAFT_ID}-en"));
        assert_eq!(en.language, "en");
        assert_eq!(en.translation_of.as_deref(), Some(DRAFT_ID));
        assert!(en.isbn.is_none());
        assert_eq!(en.title, "Classic American Iron");
        let en1 = chapter_content(&r, &en.id, 1).unwrap();
        assert!(en1.contains("Classic American Iron"));
        assert!(!en1.contains("Класичні"));
        let en_svg = String::from_utf8(
            load_asset(&r, &en.id, "1959-cadillac-eldorado-biarritz-p0-verso.svg").unwrap(),
        )
        .unwrap();
        assert!(en_svg.contains("MAKE"));
        assert_eq!(en.chapters.len(), uk.chapters.len());
    }
}
