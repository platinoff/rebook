//! RB-34: own justified interior engine — replaces `genpdf` for the book body.
//!
//! Metrics mirror the genpdf path exactly (so page numbers of an existing
//! draft stay ± the old build): uniform margins `(max(gutter,0.375)+0.25)in`,
//! body 12pt / line-height 1.35, chapter openers 30/22pt centered, H2 18pt,
//! H3 15pt, code 9pt, half-title on page 1, footer numbers from page 2.
//! On top of that: real justification (word-space stretch through the TJ
//! operator) and widow/orphan packing (a split paragraph keeps >= 2 lines on
//! both sides). The embedded TrueType is subset per book ([`crate::ttf`]),
//! which is where the size win comes from.

use std::collections::BTreeSet;
use std::path::Path;

use crate::interior::{BlockKind, discover_fonts, md_to_plain};
use crate::pdfwriter::{EmbeddedFont, PdfPageBuilder};
use crate::standards::Trim;
use crate::ttf::TtfFont;
use crate::{Book, Chapter};

const BODY: f64 = 12.0;
const LEADING: f64 = 1.35;
const CH_NO: f64 = 30.0;
const CH_TTL: f64 = 22.0;
const H2: f64 = 18.0;
const H3: f64 = 15.0;
const CODE: f64 = 9.0;
const FOOTER: f64 = 9.0;

/// Horizontal alignment / justification intent of one laid-out line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
}

/// One line in the vertical flow (empty `words` = blank line).
#[derive(Debug, Clone)]
pub struct Flow {
    pub words: Vec<String>,
    pub size: f64,
    pub align: Align,
    /// Stretch to the full measure when this line ends mid-paragraph.
    pub justify: bool,
    /// Paragraph id (`-1` for stand-alone lines).
    pub para: i64,
    /// Glue the next line to this one (headings keep-with-next).
    pub keep: bool,
    /// Force a page break before this line.
    pub page_break: bool,
}

impl Flow {
    fn blank() -> Flow {
        Flow {
            words: vec![],
            size: BODY,
            align: Align::Left,
            justify: false,
            para: -1,
            keep: false,
            page_break: false,
        }
    }
    fn line(text: &str, size: f64, align: Align) -> Flow {
        Flow {
            words: text
                .split(' ')
                .filter(|w| !w.is_empty())
                .map(String::from)
                .collect(),
            size,
            align,
            justify: false,
            para: -1,
            keep: false,
            page_break: false,
        }
    }
    fn height(&self) -> f64 {
        self.size * LEADING
    }
}

fn run_w(font: &TtfFont, s: &str, size: f64) -> f64 {
    font.run_width_pt(s, size)
}

/// Greedy word wrap with Knuth–Liang hyphenation (RB-45).
/// A token wider than the measure is split on dictionary points, then
/// characters, always with a visible `-`.
pub fn wrap(
    font: &TtfFont,
    text: &str,
    size: f64,
    width: f64,
    para: i64,
    lang_en: bool,
) -> Vec<Flow> {
    let sp = run_w(font, " ", size).max(size * 0.2);
    let mut out = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    let mut cur_w = 0.0f64;
    let push = |cur: &mut Vec<String>, cur_w: &mut f64, out: &mut Vec<Flow>| {
        if !cur.is_empty() {
            out.push(Flow {
                words: cur.clone(),
                size,
                align: Align::Left,
                justify: cur.len() > 1 && *cur_w > 0.0,
                para,
                keep: false,
                page_break: false,
            });
            cur.clear();
        }
        *cur_w = 0.0;
    };
    let measure = |s: &str| run_w(font, s, size);
    let mut pending: Option<String> = None;
    let mut words = text.split_whitespace();
    loop {
        let word = match pending.take() {
            Some(p) => p,
            None => match words.next() {
                Some(w) => w.to_string(),
                None => break,
            },
        };
        let w = measure(&word);
        let space = if cur.is_empty() { 0.0 } else { sp };
        let remain = width - cur_w - space;
        if w <= remain + 1e-6 {
            cur.push(word);
            cur_w += space + w;
            continue;
        }
        if let Some((left, right)) =
            crate::hyphen::split_for_width(&word, remain.max(0.0), measure, lang_en)
        {
            let lw = measure(&left);
            cur.push(left);
            cur_w += space + lw;
            push(&mut cur, &mut cur_w, &mut out);
            pending = Some(right);
            continue;
        }
        if !cur.is_empty() {
            push(&mut cur, &mut cur_w, &mut out);
            pending = Some(word);
            continue;
        }
        // empty line: word still wider than the measure
        if let Some((left, right)) = crate::hyphen::split_for_width(&word, width, measure, lang_en)
            .or_else(|| crate::hyphen::hard_split(&word, width, measure))
        {
            cur.push(left);
            push(&mut cur, &mut cur_w, &mut out);
            pending = Some(right);
            continue;
        }
        cur.push(word);
        cur_w = w;
    }
    push(&mut cur, &mut cur_w, &mut out);
    if let Some(last) = out.last_mut()
        && last.para == para
    {
        last.justify = false;
    }
    out
}

/// Vertical packing with widow/orphan control; returns pages + fix counters.
#[derive(Debug, Default, Clone, Copy)]
pub struct Stats {
    pub pages: usize,
    pub widows: usize,
    pub orphans: usize,
}

pub fn paginate(flows: &[Flow], height: f64) -> (Vec<Vec<Flow>>, Stats) {
    let mut st = Stats::default();
    let mut pages: Vec<Vec<Flow>> = Vec::new();
    let mut cur: Vec<Flow> = Vec::new();
    let mut y = height;
    // paragraph id -> (start, end) index range (same-size lines per paragraph)
    let mut paras: std::collections::BTreeMap<i64, (usize, usize)> = Default::default();
    for (i, f) in flows.iter().enumerate() {
        if f.para >= 0 {
            let e = paras.entry(f.para).or_insert((i, i + 1));
            e.1 = i + 1;
        }
    }
    let mut i = 0usize;
    while i < flows.len() {
        let f = &flows[i];
        if f.page_break && !cur.is_empty() {
            pages.push(std::mem::take(&mut cur));
            y = height;
        }
        if f.para >= 0 {
            let pe = paras[&f.para].1;
            let np = pe - i;
            let lh = f.height();
            if (np as f64) * lh <= y + 1e-6 {
                cur.extend(flows[i..pe].iter().cloned());
                y -= np as f64 * lh;
                i = pe;
                continue;
            }
            let mut k = (y / lh).floor() as usize;
            if k > np {
                k = np;
            }
            if k == np - 1 {
                k = np.saturating_sub(2);
                st.widows += 1;
            }
            if k == 1 && np >= 2 {
                k = 0;
                st.orphans += 1;
            }
            if k == 0 {
                if cur.is_empty() {
                    k = np.min(2); // pathological: para taller than a page
                } else {
                    pages.push(std::mem::take(&mut cur));
                    y = height;
                    continue;
                }
            }
            cur.extend(flows[i..i + k].iter().cloned());
            y -= k as f64 * lh;
            i += k;
            continue;
        }
        let lh = f.height();
        let need = lh
            + if f.keep && i + 1 < flows.len() {
                flows[i + 1].height()
            } else {
                0.0
            };
        if need > y + 1e-6 && !cur.is_empty() {
            if f.keep {
                st.orphans += 1;
            }
            pages.push(std::mem::take(&mut cur));
            y = height;
            continue;
        }
        cur.push(f.clone());
        y -= lh;
        i += 1;
    }
    if !cur.is_empty() {
        pages.push(cur);
    }
    if pages.is_empty() {
        pages.push(vec![]);
    }
    st.pages = pages.len();
    (pages, st)
}

/// Page 1 is the half-title; chapter content (and footer numbers) start at 2.
/// Studio page-view (RB-44) uses the same pad so `.pgn` / `pvNo` stay within ±1
/// of [`render`].
pub const HALF_TITLE_PAGES: u32 = 1;

/// Book page for a 1-based local page inside a chapter.
/// `pages_before` is the sum of measured pages of earlier chapters.
pub fn book_page(pages_before: u32, local: u32) -> u32 {
    HALF_TITLE_PAGES + pages_before + local
}

/// Total book pages: half-title plus each chapter (empty chapter counts as 1).
pub fn book_page_total(chap_pages: &[u32]) -> u32 {
    HALF_TITLE_PAGES + chap_pages.iter().map(|n| (*n).max(1)).sum::<u32>()
}

/// Footer is drawn only when the 0-based page index is `> 0` (`render`: `if pi > 0`).
pub fn footer_visible(book_page: u32) -> bool {
    book_page > HALF_TITLE_PAGES
}

/// Map a 1-based book page onto `(chapter_index, local_page)`.
/// `None` = half-title (page 1) or past the last chapter.
pub fn locate_book_page(book_page: u32, chap_pages: &[u32]) -> Option<(usize, u32)> {
    if book_page <= HALF_TITLE_PAGES {
        return None;
    }
    let mut acc = HALF_TITLE_PAGES;
    for (i, n) in chap_pages.iter().enumerate() {
        let n = (*n).max(1);
        if book_page <= acc + n {
            return Some((i, book_page - acc));
        }
        acc += n;
    }
    None
}

/// Book flow: half-title page, then each chapter on its own page.
pub fn flows_for(
    book: &Book,
    chapters: &[Chapter],
    font: &TtfFont,
    width: f64,
    lang_en: bool,
) -> Vec<Flow> {
    let mut fl = Vec::new();
    for _ in 0..8 {
        fl.push(Flow::blank());
    }
    let mut t = Flow::line(&book.title, 26.0, Align::Center);
    t.page_break = false;
    fl.push(t);
    fl.push(Flow::line(&book.author, 14.0, Align::Center));
    let mut para = 0i64;
    for ch in chapters {
        let mut no = Flow::line(
            &format!(
                "{} {}",
                if lang_en { "Chapter" } else { "Розділ" },
                ch.number
            ),
            CH_NO,
            Align::Center,
        );
        no.page_break = true;
        let mut ttl = Flow::line(&ch.title, CH_TTL, Align::Center);
        ttl.keep = true;
        fl.push(no);
        fl.push(ttl);
        fl.push(Flow::blank());
        fl.push(Flow::blank());
        for (kind, text) in md_to_plain(&ch.content) {
            match kind {
                BlockKind::Rule => fl.push(Flow::blank()),
                BlockKind::Code => {
                    for line in text.lines() {
                        let mut c = Flow::blank();
                        c.words = vec![line.to_string()];
                        c.size = CODE;
                        fl.push(c);
                    }
                }
                BlockKind::Heading2 | BlockKind::Heading3 => {
                    let mut h = Flow::line(
                        &text,
                        if kind == BlockKind::Heading2 { H2 } else { H3 },
                        Align::Left,
                    );
                    h.keep = true;
                    fl.push(h);
                    fl.push(Flow::blank());
                }
                _ => {
                    let lines = wrap(font, &text, BODY, width, para, lang_en);
                    para += 1;
                    fl.extend(lines);
                }
            }
        }
    }
    fl
}

fn cids(font: &TtfFont, s: &str) -> Vec<u16> {
    s.chars().map(|c| font.glyph(c)).collect()
}

/// Layout + render the interior PDF; returns (bytes written, pages, stats).
pub fn render(
    book: &Book,
    chapters: &[Chapter],
    trim: &Trim,
    out_path: &Path,
) -> Result<(usize, Stats), String> {
    let paths = discover_fonts()?;
    let font = TtfFont::load(&paths.regular)?;
    let est = crate::shelf::estimate_pages(chapters);
    let gutter = crate::standards::gutter_in(est).unwrap_or(0.375);
    let margin_pt = (gutter.max(0.375) + 0.25) * 72.0;
    let w_pt = trim.w * 72.0;
    let h_pt = trim.h * 72.0;
    let width = w_pt - 2.0 * margin_pt;
    let height = h_pt - 2.0 * margin_pt;
    let lang_en = book.language == "en";
    let flows = flows_for(book, chapters, &font, width, lang_en);
    let (pages, mut st) = paginate(&flows, height);

    // glyphs needed: every flow char, digits for the footers
    let mut used: BTreeSet<u16> = BTreeSet::new();
    let eat = |s: &str, used: &mut BTreeSet<u16>| {
        for c in s.chars() {
            used.insert(font.glyph(c));
        }
    };
    for f in flows.iter() {
        for w in &f.words {
            eat(w, &mut used);
        }
    }
    for d in 0..10 {
        eat(&d.to_string(), &mut used);
    }
    eat(" ", &mut used);
    eat(crate::hyphen::HYPHEN, &mut used);

    let subset = font.subset(&used)?;
    let maxg = *used.iter().max().unwrap_or(&0) as usize;
    let maxg = maxg.max(1);
    let scale = 1000.0 / font.units_per_em as f64;
    let base = format!("{}-RebookSub", font.base_font.replace(' ', ""));
    let emb = EmbeddedFont {
        base,
        data: subset.clone(),
        widths: (0..=maxg as u16).map(|g| font.width_1000(g)).collect(),
        ascent_1000: (font.ascender as f64 * scale) as i32,
        descent_1000: (font.descender as f64 * scale) as i32,
        bbox_1000: (
            (font.bbox.0 as f64 * scale) as i32,
            (font.bbox.1 as f64 * scale) as i32,
            (font.bbox.2 as f64 * scale) as i32,
            (font.bbox.3 as f64 * scale) as i32,
        ),
    };

    let sp_w = run_w(&font, " ", BODY);
    let mut pb = PdfPageBuilder::new(w_pt, h_pt);
    pb.set_embedded_font(emb);
    pb.set_fill(0.0, 0.0, 0.0);
    for (pi, page) in pages.iter().enumerate() {
        if pi > 0 {
            pb.new_page(w_pt, h_pt);
        }
        let mut y = h_pt - margin_pt;
        for f in page {
            let lh = f.height();
            y -= lh;
            if f.words.is_empty() {
                continue;
            }
            let size = f.size;
            let line_w = f.words.iter().map(|w| run_w(&font, w, size)).sum::<f64>()
                + sp_w * size / BODY * (f.words.len().saturating_sub(1)) as f64;
            let baseline = y + (f.size * 0.22).min(lh * 0.25);
            match f.align {
                Align::Center => {
                    let x = margin_pt + (width - line_w) / 2.0;
                    let text = f.words.join(" ");
                    let c = cids(&font, &text);
                    pb.text_cid_at(0, x, baseline, size, &c, false);
                }
                Align::Left => {
                    let justify = f.justify && f.words.len() > 1 && line_w < width - 0.5;
                    if justify {
                        let gap = (width
                            - f.words.iter().map(|w| run_w(&font, w, size)).sum::<f64>())
                            / (f.words.len() - 1) as f64;
                        let runs: Vec<(Vec<u16>, f64)> =
                            f.words.iter().map(|w| (cids(&font, w), gap)).collect();
                        let refs: Vec<(&[u16], f64)> =
                            runs.iter().map(|(r, g)| (r.as_slice(), *g)).collect();
                        pb.text_cid_tj(0, margin_pt, baseline, size, &refs);
                    } else {
                        let text = f.words.join(" ");
                        let c = cids(&font, &text);
                        pb.text_cid_at(0, margin_pt, baseline, size, &c, false);
                    }
                }
            }
        }
        if pi > 0 {
            let n = format!("{}", pi + 1);
            let c = cids(&font, &n);
            let cw = pb.cid_width_pt_at(0, &c, FOOTER);
            pb.text_cid_at(
                0,
                margin_pt + (width - cw) / 2.0,
                margin_pt / 2.0,
                FOOTER,
                &c,
                false,
            );
        }
    }
    let bytes = pb.build(&format!("{} — {}", book.title, book.author));
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    std::fs::write(out_path, &bytes).map_err(|e| format!("write pdf: {e}"))?;
    st.pages = pages.len();
    Ok((bytes.len(), st))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChapterMeta;

    fn font() -> Option<TtfFont> {
        discover_fonts()
            .ok()
            .and_then(|f| TtfFont::load(&f.regular).ok())
    }

    fn book_ch() -> (Book, Vec<Chapter>) {
        let content = "Це речення досить довге, щоб перекрити кілька рядків на сторінці формату шість на дев'ять із полями KDP. "
            .repeat(30);
        (
            Book {
                title: "Justify Test".into(),
                author: "A. B".into(),
                edition: 1,
                year: 2026,
                format: "EPUB 3.2".into(),
                language: "uk".into(),
                isbn: None,
                chapters: vec![ChapterMeta {
                    number: 1,
                    title: "Глава один".into(),
                    file: "c1.md".into(),
                }],
            },
            vec![Chapter {
                number: 1,
                title: "Глава один".into(),
                content,
            }],
        )
    }

    #[test]
    fn wrap_justifies_full_lines_to_the_measure() {
        let Some(tf) = font() else { return };
        let width = 300.0;
        let text = "слово ".repeat(80);
        let lines = wrap(&tf, &text, BODY, width, 0, false);
        assert!(lines.len() > 3);
        for l in &lines[..lines.len() - 1] {
            assert!(
                l.justify || l.words.iter().any(|w| w.ends_with('-')),
                "interior lines must stretch or end on a hyphen"
            );
            assert!(
                l.words.len() > 1 || l.words.iter().any(|w| w.ends_with('-')),
                "a full line is several words or a hyphenated fragment"
            );
            let w: f64 = l
                .words
                .iter()
                .map(|x| tf.run_width_pt(x, BODY))
                .sum::<f64>()
                + tf.run_width_pt(" ", BODY) * (l.words.len() - 1) as f64;
            assert!(w <= width + 0.5, "line overflow {w} > {width}");
        }
        assert!(!lines.last().unwrap().justify, "last line stays left");
    }

    #[test]
    fn wrap_hyphenates_long_english_word() {
        let Some(tf) = font() else { return };
        let w = tf.run_width_pt("hyphenation", BODY);
        let width = w * 0.62;
        let lines = wrap(&tf, "hyphenation", BODY, width, 0, true);
        let joined: String = lines.iter().flat_map(|l| l.words.iter()).cloned().collect();
        assert!(
            lines
                .iter()
                .any(|l| l.words.iter().any(|x| x.ends_with('-'))),
            "expected a hyphen break, got {:?}",
            lines.iter().map(|l| &l.words).collect::<Vec<_>>()
        );
        assert_eq!(joined.replace('-', ""), "hyphenation");
        for l in &lines {
            let lw: f64 = l
                .words
                .iter()
                .map(|x| tf.run_width_pt(x, BODY))
                .sum::<f64>()
                + tf.run_width_pt(" ", BODY) * l.words.len().saturating_sub(1) as f64;
            assert!(lw <= width + 0.5, "overflow {lw} > {width}");
        }
    }

    #[test]
    fn widow_orphan_keeps_two_each_side() {
        let tf = font();
        let width = 300.0;
        let lines = match &tf {
            Some(f) => wrap(f, &"w ".repeat(400), BODY, width, 0, true),
            None => (0..40)
                .map(|_| Flow {
                    words: vec!["word".into()],
                    size: BODY,
                    align: Align::Left,
                    justify: true,
                    para: 0,
                    keep: false,
                    page_break: false,
                })
                .collect(),
        };
        let np = lines.len();
        let page_h = (np as f64 - 1.0) * BODY * LEADING; // force a 1-line widow
        let (pages, st) = paginate(&lines, page_h);
        assert_eq!(pages.len(), 2);
        let first = pages[0].len();
        assert!(
            first >= 2 && np - first >= 2,
            "widow/orphan survived: {first}/{np}"
        );
        assert!(st.widows + st.orphans >= 1, "counters never fired");
    }

    #[test]
    fn subset_reparses_and_shrinks() {
        let Some(tf) = font() else { return };
        let used: BTreeSet<u16> = "Hello world 123 — абвґґд"
            .chars()
            .map(|c| tf.glyph(c))
            .collect();
        let sub = tf.subset(&used).unwrap();
        let re = TtfFont::parse(sub.clone()).unwrap();
        assert_ne!(re.glyph('A'), 0);
        assert_ne!(re.glyph('а'), 0, "cyrillic gid must survive");
        assert_eq!(
            re.advance_units(re.glyph('A')),
            tf.advance_units(tf.glyph('A'))
        );
        assert!(
            sub.len() < tf.data.len(),
            "subset {} >= full {}",
            sub.len(),
            tf.data.len()
        );
    }

    #[test]
    fn own_engine_renders_justified_smaller_pdf() {
        let Some(tf) = font() else { return };
        let _ = &tf;
        if discover_fonts().is_err() {
            return;
        }
        let out = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("interior-own.pdf");
        let _ = std::fs::remove_file(&out);
        let (b, c) = book_ch();
        let trim = crate::standards::find_trim(crate::standards::PAPERBACK_TRIMS, "6x9").unwrap();
        let (size, st) = render(&b, &c, trim, &out).unwrap();
        assert!(
            st.pages >= 2,
            "expected a multi-page book, got {}",
            st.pages
        );
        let bytes = std::fs::read(&out).unwrap();
        let s = String::from_utf8_lossy(&bytes).into_owned();
        assert!(s.starts_with("%PDF"));
        assert!(
            s.contains(&format!("/Count {}", st.pages)),
            "page count object"
        );
        assert!(s.contains(" TJ ET"), "justified TJ runs missing");
        let gpath = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("interior-genpdf.pdf");
        let gs = crate::interior::render_interior_pdf_genpdf(&b, &c, trim, &gpath).unwrap();
        assert!(size * 2 < gs, "own {size} !< half of genpdf {gs}");
        // a real parser must accept the own-engine PDF (not just string shape)
        let doc = lopdf::Document::load(&out).expect("lopdf must parse own PDF");
        assert_eq!(doc.get_pages().len(), st.pages, "lopdf page count");
        assert!(doc.trailer.get(b"Root").is_ok(), "catalog reachable");
    }

    #[test]
    fn rb44_book_page_matches_half_title_and_footer() {
        let ch = [3u32, 2];
        assert_eq!(book_page(0, 1), 2);
        assert_eq!(book_page(3, 1), 5);
        assert_eq!(book_page_total(&ch), 6);
        assert_eq!(locate_book_page(1, &ch), None);
        assert_eq!(locate_book_page(2, &ch), Some((0, 1)));
        assert_eq!(locate_book_page(4, &ch), Some((0, 3)));
        assert_eq!(locate_book_page(5, &ch), Some((1, 1)));
        assert_eq!(locate_book_page(6, &ch), Some((1, 2)));
        assert_eq!(locate_book_page(7, &ch), None);
        assert!(!footer_visible(1));
        assert!(footer_visible(2));
        assert_eq!(book_page_total(&[0, 4]), 1 + 1 + 4);
    }
}
