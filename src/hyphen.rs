//! RB-45: Knuth–Liang hyphenation for interior wrap (Ukrainian + English).
//!
//! `rustybuzz` is already a transitive dep (OpenType shaping via genpdf/usvg).
//! It does **not** ship hyphenation dictionaries. TeX patterns come from
//! [`hypher`](https://docs.rs/hypher) (`english` + `ukrainian` only).

use hypher::{Lang, hyphenate};

/// Visible hyphen inserted at a line break (PDF / KDP subset has this glyph).
pub const HYPHEN: &str = "-";

fn lang(lang_en: bool) -> Lang {
    if lang_en {
        Lang::English
    } else {
        Lang::Ukrainian
    }
}

fn looks_hyphenatable(word: &str) -> bool {
    word.chars().filter(|c| c.is_alphabetic()).count() >= 5
        && !word.contains("://")
        && !word.contains('@')
}

/// Byte offsets where `word` may break (dictionary + soft hyphens). Empty if
/// the token is too short, a URL, or an email.
pub fn break_bytes(word: &str, lang_en: bool) -> Vec<usize> {
    if !looks_hyphenatable(word) {
        return shy_only(word);
    }
    let mut out = Vec::new();
    let mut acc = 0usize;
    let parts: Vec<&str> = hyphenate(word, lang(lang_en)).collect();
    for (i, p) in parts.iter().enumerate() {
        acc += p.len();
        if i + 1 < parts.len() && acc > 0 && acc < word.len() {
            out.push(acc);
        }
    }
    out.extend(shy_only(word));
    out.sort_unstable();
    out.dedup();
    out
}

fn shy_only(word: &str) -> Vec<usize> {
    word.char_indices()
        .filter(|(_, c)| *c == '\u{00AD}')
        .map(|(i, _)| i)
        .filter(|&i| i > 0 && i < word.len())
        .collect()
}

/// Longest prefix of `word` that fits in `avail` (including a trailing `-`).
/// `None` if no legal break fits.
pub fn split_for_width(
    word: &str,
    avail: f64,
    mut measure: impl FnMut(&str) -> f64,
    lang_en: bool,
) -> Option<(String, String)> {
    let hy_w = measure(HYPHEN);
    let mut best: Option<usize> = None;
    for off in break_bytes(word, lang_en) {
        let left = word[..off].trim_end_matches('\u{00AD}');
        let right = word[off..].trim_start_matches('\u{00AD}');
        if left.is_empty() || right.is_empty() {
            continue;
        }
        if measure(left) + hy_w <= avail + 1e-6 {
            best = Some(off);
        }
    }
    best.map(|off| {
        let left = word[..off].trim_end_matches('\u{00AD}');
        let right = word[off..].trim_start_matches('\u{00AD}');
        (format!("{left}{HYPHEN}"), right.to_string())
    })
}

/// Character-level fallback when a token is wider than the full measure and
/// the dictionary has no remaining break. Always inserts a visible hyphen.
pub fn hard_split(
    word: &str,
    avail: f64,
    mut measure: impl FnMut(&str) -> f64,
) -> Option<(String, String)> {
    let hy_w = measure(HYPHEN);
    let mut last_off = 0usize;
    for (off, _) in word.char_indices() {
        if off == 0 {
            continue;
        }
        let left = &word[..off];
        if measure(left) + hy_w <= avail + 1e-6 {
            last_off = off;
        } else {
            break;
        }
    }
    if last_off == 0 || last_off >= word.len() {
        return None;
    }
    Some((
        format!("{}{HYPHEN}", &word[..last_off]),
        word[last_off..].to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_extensive_matches_hypher_docs() {
        let joined = {
            use hypher::{Lang, hyphenate};
            hyphenate("extensive", Lang::English)
                .collect::<Vec<_>>()
                .join("-")
        };
        assert_eq!(joined, "ex-ten-sive");
        let br = break_bytes("extensive", true);
        assert!(br.contains(&2), "{br:?}"); // "ex"
        assert!(br.contains(&5), "{br:?}"); // "exten"
    }

    #[test]
    fn ukrainian_long_word_has_breaks() {
        let br = break_bytes("компілятором", false);
        assert!(
            !br.is_empty(),
            "expected Knuth-Liang points in компілятором"
        );
        assert!(br.iter().all(|&o| o > 0 && o < "компілятором".len()));
    }

    #[test]
    fn urls_and_short_words_do_not_break() {
        assert!(break_bytes("and", true).is_empty());
        assert!(break_bytes("https://example.com/path", true).is_empty());
        assert!(break_bytes("a@b.c", true).is_empty());
    }

    #[test]
    fn soft_hyphen_is_a_break() {
        let w = "abc\u{00AD}defgh";
        let br = break_bytes(w, true);
        assert!(br.contains(&3), "{br:?}");
    }

    #[test]
    fn split_picks_longest_prefix_that_fits() {
        let word = "extensive";
        let (left, right) = split_for_width(word, 4.5, |s| s.chars().count() as f64, true)
            .expect("should hyphenate");
        assert!(left.ends_with('-'));
        assert_eq!(format!("{left}{right}").replace('-', ""), word);
        // avail 4.5 chars → "ex-" (3) or "exten-" (6) ; 6 > 4.5 so "ex-"
        assert_eq!(left, "ex-");
        assert_eq!(right, "tensive");
    }

    #[test]
    fn hard_split_inserts_hyphen() {
        let (left, right) = hard_split("abcdefghij", 4.0, |s| s.chars().count() as f64).unwrap();
        assert_eq!(left, "abc-");
        assert_eq!(right, "defghij");
    }
}
