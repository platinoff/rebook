//! Wave-3 RB-15: interior print PDF (KDP RGB path) via `genpdf`.
//!
//! Single pages (no spreads), custom trim size from [`crate::standards`],
//! margins derived from the KDP gutter table, fonts embedded from system TTFs
//! (Cyrillic-capable serif). Owner's crate pick: `genpdf` (interior) +
//! `printpdf` (later wrap PDF, RB-16).

use std::path::{Path, PathBuf};

use genpdf::Element;

use crate::standards::Trim;
use crate::{Book, Chapter};

/// A font family path set (regular required; others fall back to it).
#[derive(Debug, Clone)]
pub struct FontPaths {
    /// Regular TTF.
    pub regular: PathBuf,
    /// Bold TTF (defaults to regular).
    pub bold: PathBuf,
    /// Italic TTF (defaults to regular).
    pub italic: PathBuf,
    /// Bold-italic TTF (defaults to regular).
    pub bold_italic: PathBuf,
}

/// Find a Cyrillic-capable TTF family: env overrides first, then common
/// Windows font files, then a Linux default. `Err` when nothing exists.
pub fn discover_fonts() -> Result<FontPaths, String> {
    if let Ok(reg) = std::env::var("REBOOK_FONT_TTF") {
        let reg = PathBuf::from(reg);
        if !reg.exists() {
            return Err(format!("REBOOK_FONT_TTF not found: {}", reg.display()));
        }
        let pick = |v: &str| {
            std::env::var(v)
                .ok()
                .map(PathBuf::from)
                .filter(|p| p.exists())
                .unwrap_or_else(|| reg.clone())
        };
        return Ok(FontPaths {
            regular: reg.clone(),
            bold: pick("REBOOK_FONT_TTF_BOLD"),
            italic: pick("REBOOK_FONT_TTF_ITALIC"),
            bold_italic: pick("REBOOK_FONT_TTF_BOLD_ITALIC"),
        });
    }
    let root = match std::env::var("WINDIR") {
        Ok(w) => PathBuf::from(w).join("Fonts"),
        Err(_) => PathBuf::from("C:\\Windows\\Fonts"),
    };
    let sets: [(&str, &str, &str, &str); 3] = [
        ("times.ttf", "timesbd.ttf", "timesi.ttf", "timesbi.ttf"),
        (
            "georgia.ttf",
            "georgiab.ttf",
            "georgiai.ttf",
            "georgiaz.ttf",
        ),
        ("arial.ttf", "arialbd.ttf", "ariali.ttf", "arialbi.ttf"),
    ];
    for (r, b, i, bi) in sets {
        let regular = root.join(r);
        if regular.exists() {
            let (bold, italic, bold_italic) = {
                let opt = |f: &str| {
                    let p = root.join(f);
                    if p.exists() { p } else { regular.clone() }
                };
                (opt(b), opt(i), opt(bi))
            };
            return Ok(FontPaths {
                regular,
                bold,
                italic,
                bold_italic,
            });
        }
    }
    // Linux fallback family names.
    for (r, b, i, bi) in [(
        "/usr/share/fonts/liberation-serif/LiberationSerif-Regular.ttf",
        "/usr/share/fonts/liberation-serif/LiberationSerif-Bold.ttf",
        "/usr/share/fonts/liberation-serif/LiberationSerif-Italic.ttf",
        "/usr/share/fonts/liberation-serif/LiberationSerif-BoldItalic.ttf",
    )] {
        if Path::new(r).exists() {
            return Ok(FontPaths {
                regular: PathBuf::from(r),
                bold: PathBuf::from(b),
                italic: PathBuf::from(i),
                bold_italic: PathBuf::from(bi),
            });
        }
    }
    Err("no Cyrillic-capable TTF found (set REBOOK_FONT_TTF)".to_string())
}

/// Strip the tiny markdown surface into plain text for PDF paragraphs.
pub fn md_to_plain(md: &str) -> Vec<(BlockKind, String)> {
    let mut blocks = Vec::new();
    let mut para: Vec<String> = Vec::new();
    let mut in_code = false;
    let mut code: Vec<String> = Vec::new();
    let flush = |para: &mut Vec<String>, blocks: &mut Vec<(BlockKind, String)>| {
        if !para.is_empty() {
            let text = para.join(" ");
            if !text.trim().is_empty() {
                blocks.push((BlockKind::Paragraph, clean_inline(text.as_str())));
            }
            para.clear();
        }
    };
    for raw in md.lines() {
        let line = raw.trim_end();
        if in_code {
            if line.trim_start().starts_with("```") {
                in_code = false;
                let body = code.join("\n");
                if !body.trim().is_empty() {
                    blocks.push((BlockKind::Code, body));
                }
                code.clear();
            } else {
                code.push(line.to_string());
            }
            continue;
        }
        let t = line.trim_start();
        if t.starts_with("```") {
            flush(&mut para, &mut blocks);
            in_code = true;
        } else if let Some(h) = t.strip_prefix("### ") {
            flush(&mut para, &mut blocks);
            blocks.push((BlockKind::Heading3, clean_inline(h.trim())));
        } else if let Some(h) = t.strip_prefix("## ") {
            flush(&mut para, &mut blocks);
            blocks.push((BlockKind::Heading2, clean_inline(h.trim())));
        } else if t.starts_with('>') {
            flush(&mut para, &mut blocks);
            blocks.push((
                BlockKind::Quote,
                clean_inline(t.trim_start_matches('>').trim()),
            ));
        } else if t.starts_with("---") {
            flush(&mut para, &mut blocks);
            blocks.push((BlockKind::Rule, String::new()));
        } else if line.trim().is_empty() {
            flush(&mut para, &mut blocks);
        } else {
            para.push(t.to_string());
        }
    }
    if in_code && !code.is_empty() {
        blocks.push((BlockKind::Code, code.join("\n")));
    }
    flush(&mut para, &mut blocks);
    blocks
}

fn clean_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_back = false;
    for ch in s.chars() {
        if ch == '`' {
            continue;
        }
        if ch == '*' {
            prev_back = false;
            continue;
        }
        if ch == '\\' {
            prev_back = true;
            continue;
        }
        out.push(ch);
        prev_back = false;
    }
    let _ = prev_back;
    out
}

/// Kind of a text block produced by [`md_to_plain`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// `##` heading — starts a new page.
    Heading2,
    /// `###` heading.
    Heading3,
    /// Body paragraph.
    Paragraph,
    /// Blockquote line.
    Quote,
    /// Fenced code block.
    Code,
    /// Horizontal rule (spacer).
    Rule,
}

/// Render the interior PDF: one page per chapter start, trim `page` size,
/// margins from the KDP gutter table, embedded fonts. Returns file bytes.
pub fn render_interior_pdf(
    book: &Book,
    chapters: &[Chapter],
    trim: &Trim,
    out_path: &Path,
) -> Result<usize, String> {
    let fonts = discover_fonts()?;
    let family = genpdf::fonts::FontFamily {
        regular: genpdf::fonts::FontData::load(&fonts.regular, None)
            .map_err(|e| format!("font {}: {e}", fonts.regular.display()))?,
        bold: genpdf::fonts::FontData::load(&fonts.bold, None)
            .map_err(|e| format!("font {}: {e}", fonts.bold.display()))?,
        italic: genpdf::fonts::FontData::load(&fonts.italic, None)
            .map_err(|e| format!("font {}: {e}", fonts.italic.display()))?,
        bold_italic: genpdf::fonts::FontData::load(&fonts.bold_italic, None)
            .map_err(|e| format!("font {}: {e}", fonts.bold_italic.display()))?,
    };
    let mut doc = genpdf::Document::new(family);
    doc.set_title(format!("{} — {}", book.title, book.author));
    doc.set_minimal_conformance();
    // inches → mm; KDP gutter table for the rough page band.
    let est_pages = crate::shelf::estimate_pages(chapters);
    let gutter = crate::standards::gutter_in(est_pages).unwrap_or(0.375);
    let margin_mm = (gutter.max(0.375) + 0.25) * 25.4;
    let w_mm = trim.w * 25.4;
    let h_mm = trim.h * 25.4;
    doc.set_paper_size(genpdf::Size::new(
        genpdf::Mm::from(w_mm),
        genpdf::Mm::from(h_mm),
    ));
    let mut deco = genpdf::SimplePageDecorator::new();
    deco.set_margins(genpdf::Mm::from(margin_mm));
    doc.set_page_decorator(deco);

    // half-title page
    doc.push(genpdf::elements::Break::new(8));
    let mut title_p = genpdf::elements::Paragraph::new(book.title.clone());
    title_p.set_alignment(genpdf::Alignment::Center);
    doc.push(title_p.styled(genpdf::style::Style::default().with_font_size(26)));
    let mut author_p = genpdf::elements::Paragraph::new(book.author.clone());
    author_p.set_alignment(genpdf::Alignment::Center);
    doc.push(author_p.styled(genpdf::style::Style::default().with_font_size(14)));
    for ch in chapters {
        doc.push(genpdf::elements::PageBreak::new());
        let mut ch_p = genpdf::elements::Paragraph::new(format!(
            "{} {}",
            if book.language == "en" {
                "Chapter"
            } else {
                "Розділ"
            },
            ch.number
        ));
        ch_p.set_alignment(genpdf::Alignment::Center);
        doc.push(ch_p.styled(genpdf::style::Style::default().with_font_size(30)));
        let mut t_p = genpdf::elements::Paragraph::new(ch.title.clone());
        t_p.set_alignment(genpdf::Alignment::Center);
        doc.push(t_p.styled(genpdf::style::Style::default().with_font_size(22)));
        doc.push(genpdf::elements::Break::new(2));
        for (kind, text) in md_to_plain(&ch.content) {
            match kind {
                BlockKind::Rule => doc.push(genpdf::elements::Break::new(1)),
                BlockKind::Code => {
                    for line in text.lines() {
                        doc.push(
                            genpdf::elements::Paragraph::new(line.to_string())
                                .styled(genpdf::style::Style::default().with_font_size(9)),
                        );
                    }
                }
                BlockKind::Heading2 | BlockKind::Heading3 => {
                    let size = if kind == BlockKind::Heading2 { 18 } else { 15 };
                    doc.push(
                        genpdf::elements::Paragraph::new(text)
                            .styled(genpdf::style::Style::default().with_font_size(size)),
                    );
                    doc.push(genpdf::elements::Break::new(1));
                }
                _ => doc.push(
                    genpdf::elements::Paragraph::new(text).styled(
                        genpdf::style::Style::default()
                            .with_font_size(12)
                            .with_line_spacing(1.35),
                    ),
                ),
            }
        }
    }
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    doc.render_to_file(out_path)
        .map_err(|e| format!("pdf render: {e}"))?;
    let bytes = std::fs::metadata(out_path)
        .map(|m| m.len() as usize)
        .unwrap_or(0);
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChapterMeta;

    fn mini() -> (Book, Vec<Chapter>) {
        let book = Book {
            title: "Interior Test".to_string(),
            author: "A. B".to_string(),
            edition: 1,
            year: 2026,
            format: "EPUB 3.2".to_string(),
            language: "uk".to_string(),
            chapters: vec![ChapterMeta {
                number: 1,
                title: "Глава".to_string(),
                file: "c1.md".to_string(),
            }],
        };
        let chapters = vec![Chapter {
            number: 1,
            title: "Глава".to_string(),
            content: "## Початок\n\nТекст розділу з **курсивом** і `кодом`.\n\n> цитата\n\n```\nlet x = 1;\n```".to_string(),
        }];
        (book, chapters)
    }

    #[test]
    fn md_to_plain_blocks() {
        let blocks = md_to_plain("## H2\n\npara **one**\n\n> quote\n\n---\n\n### H3");
        let kinds: Vec<_> = blocks.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                BlockKind::Heading2,
                BlockKind::Paragraph,
                BlockKind::Quote,
                BlockKind::Rule,
                BlockKind::Heading3
            ]
        );
        assert_eq!(blocks[1].1, "para one");
    }

    #[test]
    fn renders_pdf_when_fonts_present() {
        if discover_fonts().is_err() {
            eprintln!("no system TTF — skipping PDF render test");
            return;
        }
        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("interior-test.pdf");
        let _ = std::fs::remove_file(&out);
        let (book, chapters) = mini();
        let trim = crate::standards::find_trim(crate::standards::PAPERBACK_TRIMS, "6x9").unwrap();
        let size = render_interior_pdf(&book, &chapters, trim, &out).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"%PDF"), "not a PDF");
        assert!(size > 3000, "suspiciously small pdf: {size}");
    }
}
