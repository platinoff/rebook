//! Module for managing book chapters
//!
//! Handles chapter loading, validation, and organization
//! following the "Rustеред снов" structure (14 chapters, 8000 words max each).

use serde::Deserialize;
use std::path::Path;

pub mod epub;
pub mod kdp;

#[derive(Debug, Clone, Deserialize)]
pub struct Chapter {
    pub number: u32,
    pub title: String,
    pub content: String,
    pub poetic: String,
}

impl Chapter {
    /// Create a new chapter with validation
    pub fn new(number: u32, title: &str, content: &str, poetic: &str) -> Self {
        if number < 1 {
            panic!("Chapter number must be >= 1");
        }
        if title.is_empty() {
            panic!("Chapter title cannot be empty");
        }
        if content.is_empty() {
            panic!("Chapter content cannot be empty");
        }
        if poetic.is_empty() {
            panic!("Chapter must have a poetic element");
        }

        // Validate word count does not exceed maximum (8000 words per chapter)
        let word_count = content.split_whitespace().count();
        if word_count > 8000 {
            panic!(
                "Chapter content exceeds maximum 8000 words (current: {} words)",
                word_count
            );
        }

        Chapter {
            number,
            title: title.to_string(),
            content: content.to_string(),
            poetic: poetic.to_string(),
        }
    }

    /// Get the chapter as formatted markdown
    pub fn to_markdown(&self) -> String {
        format!(
            "# Chapter {}: {}\n\n{}\n\n> {}\n",
            self.number, self.title, self.content, self.poetic
        )
    }

    /// Validate chapter follows the rules
    /// - Max 8000 words
    /// - 1-2 poetic lines
    /// - Progressive content
    pub fn validate(&self) -> bool {
        let word_count = self.content.split_whitespace().count();
        let poetic_lines: usize = self.poetic.lines().count();

        word_count <= 8000 && (1..=2).contains(&poetic_lines)
    }
}

/// A full book: metadata plus an ordered list of chapters.
///
/// Mirrors the layout of `book.json` so it can be deserialized directly.
#[derive(Debug, Clone, Deserialize)]
pub struct Book {
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub edition: u32,
    #[serde(default)]
    pub year: u32,
    #[serde(default)]
    pub format: String,
    pub chapters: Vec<Chapter>,
}

/// Load a `Book` from a JSON file (e.g. `book.json`).
pub fn load_book<P: AsRef<Path>>(path: P) -> Result<Book, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read book file: {}", e))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Failed to parse book JSON: {}", e))
}
