//! KDP metadata lint for coloring books: title, subtitle, description and
//! keywords must not carry trademarks (makes / models from the roster), other
//! authors (`listing.json` `deny`), or price / rank claims (free, bestseller).
//! Kindle Content Review rejects those; interior art may still show the cars.

use serde::Serialize;

use crate::coloring::Roster;
use crate::coloring_kdp::Listing;

/// KDP keyword boxes on the paperback form.
pub const MAX_KEYWORDS: usize = 7;
/// Characters per keyword box.
pub const MAX_KEYWORD_CHARS: usize = 50;

/// Always-banned claims in KDP metadata.
const BANNED: &[&str] = &[
    "free",
    "bestseller",
    "best seller",
    "best-seller",
    "bestselling",
    "best-selling",
    "#1",
    "number one",
    "kindle unlimited",
    "amazon",
    "kdp",
];

/// Model words that name a body style, not a trademark.
const GENERIC: &[&str] = &[
    "custom",
    "coupe",
    "window",
    "pickup",
    "express",
    "series",
    "model",
    "carrier",
    "wagon",
    "power",
    "sedan",
    "roadster",
    "convertible",
    "truck",
    "big",
    "red",
];

/// One blocked term in one metadata field.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LintHit {
    /// `title`, `subtitle`, `description`, `keyword 3`, …
    pub field: String,
    /// Deny-list term that matched.
    pub term: String,
    /// `trademark`, `claim`, `deny`, `length`, `count`.
    pub reason: String,
}

/// Every blocking problem in the listing; empty = clean.
pub fn lint_listing(roster: &Roster, listing: &Listing) -> Vec<LintHit> {
    let terms = deny_terms(roster, listing);
    let mut hits = Vec::new();
    let mut scan = |field: String, text: &str| {
        for (term, reason) in &terms {
            if contains_term(text, term) {
                hits.push(LintHit {
                    field: field.clone(),
                    term: term.clone(),
                    reason: (*reason).into(),
                });
            }
        }
    };
    scan("title".into(), &roster.title_en);
    scan("subtitle".into(), &listing.subtitle);
    scan("description".into(), &listing.description);
    for (i, k) in listing.keywords.iter().enumerate() {
        scan(format!("keyword {}", i + 1), k);
    }
    if listing.keywords.len() > MAX_KEYWORDS {
        hits.push(LintHit {
            field: "keywords".into(),
            term: listing.keywords.len().to_string(),
            reason: "count".into(),
        });
    }
    for (i, k) in listing.keywords.iter().enumerate() {
        if k.chars().count() > MAX_KEYWORD_CHARS {
            hits.push(LintHit {
                field: format!("keyword {}", i + 1),
                term: k.clone(),
                reason: "length".into(),
            });
        }
    }
    hits
}

/// One line per hit, for CLI errors and `KDP.txt`.
pub fn describe(hits: &[LintHit]) -> String {
    hits.iter()
        .map(|h| match h.reason.as_str() {
            "count" => format!("{}: {} keywords, KDP takes {MAX_KEYWORDS}", h.field, h.term),
            "length" => format!("{}: over {MAX_KEYWORD_CHARS} chars", h.field),
            r => format!("{}: \"{}\" ({r})", h.field, h.term),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn deny_terms(roster: &Roster, listing: &Listing) -> Vec<(String, &'static str)> {
    let allow: Vec<String> = listing.allow.iter().map(|a| a.to_lowercase()).collect();
    let mut out: Vec<(String, &'static str)> = Vec::new();
    let mut push = |t: String, reason: &'static str| {
        let t = t.trim().to_lowercase();
        if !t.is_empty() && !allow.contains(&t) && !out.iter().any(|(x, _)| *x == t) {
            out.push((t, reason));
        }
    };
    for b in BANNED {
        push((*b).into(), "claim");
    }
    for d in &listing.deny {
        push(d.clone(), "deny");
    }
    for c in &roster.cars {
        push(c.make.clone(), "trademark");
        if !GENERIC.contains(&c.model.trim().to_lowercase().as_str()) {
            push(c.model.clone(), "trademark");
        }
        for tok in model_tokens(&c.model) {
            push(tok, "trademark");
        }
    }
    out
}

/// Distinctive words of a model name: `Mustang Boss 429` → mustang, boss.
fn model_tokens(model: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in model.split_whitespace() {
        let tok = raw.trim_matches(|c: char| !c.is_alphanumeric());
        let alnum = tok.chars().filter(|c| c.is_alphanumeric()).count();
        let letters = tok.chars().filter(|c| c.is_alphabetic()).count();
        if letters == 0 {
            continue;
        }
        let lower = tok.to_lowercase();
        if GENERIC.contains(&lower.as_str()) {
            continue;
        }
        let has_digit = tok.chars().any(|c| c.is_ascii_digit());
        let all_caps = tok
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(|c| c.is_uppercase());
        if alnum >= 4 || (all_caps && letters >= 2) || (has_digit && alnum >= 3) {
            out.push(lower);
        }
    }
    out
}

/// Whole-word, case-insensitive match; also catches plural `s` / `es` / `'s`.
fn contains_term(text: &str, term: &str) -> bool {
    let hay = text.to_lowercase();
    let mut from = 0;
    while let Some(pos) = hay[from..].find(term) {
        let start = from + pos;
        let end = start + term.len();
        let before_ok = hay[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let rest = &hay[end..];
        let tail = ["'s", "es", "s", ""]
            .iter()
            .find(|suf| rest.starts_with(**suf))
            .map(|suf| suf.len())
            .unwrap_or(0);
        let after_ok = rest[tail..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        let bare_after_ok = rest.chars().next().is_none_or(|c| !c.is_alphanumeric());
        if before_ok && (after_ok || bare_after_ok) {
            return true;
        }
        from = start + term.len().max(1);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coloring::load_roster;

    fn listing(desc: &str, kws: &[&str]) -> Listing {
        Listing {
            subtitle: "A Coloring Book".into(),
            description: desc.into(),
            keywords: kws.iter().map(|s| s.to_string()).collect(),
            ..Listing::default()
        }
    }

    #[test]
    fn rejected_description_is_caught() {
        let r = load_roster().unwrap();
        let l = listing(
            "Cadillac fins, a split-window Sting Ray, a Cobra 427, Chargers, 'Cudas, Mustangs, Chevelles, and more.",
            &["adult coloring book"],
        );
        let terms: Vec<String> = lint_listing(&r, &l).into_iter().map(|h| h.term).collect();
        for want in [
            "cadillac", "cobra", "charger", "cuda", "mustang", "chevelle",
        ] {
            assert!(
                terms.iter().any(|t| t == want),
                "missing {want} in {terms:?}"
            );
        }
    }

    #[test]
    fn generic_listing_is_clean() {
        let r = load_roster().unwrap();
        let l = listing(
            "An adult coloring book of twenty-four classic U.S. cars. Each custom coupe opens with a color plate.",
            &[
                "adult coloring book",
                "classic cars",
                "muscle cars",
                "vintage cars",
                "car coloring",
                "american automobiles",
                "hot rod coloring",
            ],
        );
        assert_eq!(lint_listing(&r, &l), vec![]);
    }

    #[test]
    fn claims_counts_and_deny_list() {
        let r = load_roster().unwrap();
        let mut l = listing("The #1 bestseller, free inside.", &["a"; 8]);
        l.keywords[0] = "x".repeat(51);
        l.deny = vec!["Jane Painter".into()];
        l.description.push_str(" Like Jane Painter books.");
        let hits = lint_listing(&r, &l);
        let reasons: Vec<&str> = hits.iter().map(|h| h.reason.as_str()).collect();
        assert!(reasons.contains(&"claim"));
        assert!(reasons.contains(&"deny"));
        assert!(reasons.contains(&"count"));
        assert!(reasons.contains(&"length"));
        assert!(hits.iter().any(|h| h.term == "#1"));
    }

    #[test]
    fn word_boundaries_and_allow() {
        assert!(contains_term("carefree days", "free").eq(&false));
        assert!(contains_term("Two GTOs", "gto"));
        assert!(!contains_term("transportation", "trans"));
        let r = load_roster().unwrap();
        let mut l = listing("Impala-free zone", &[]);
        assert!(!lint_listing(&r, &l).is_empty());
        l.allow = vec!["impala".into(), "free".into()];
        assert!(lint_listing(&r, &l).is_empty());
    }
}
