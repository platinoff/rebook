//! Book metadata, chapter loading, and organization for
//! "Rust перед сном" (14 chapters, ~8000 words each).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod ai;
pub mod barcode;
pub mod coloring;
pub mod cover;
pub mod coverdoc;
pub mod coverpdf;
pub mod drafts;
pub mod epub;
pub mod hyphen;
pub mod icc;
pub mod interior;
pub mod interior_pdf;
pub mod kdp;
pub mod pdfwriter;
pub mod preflight;
pub mod shelf;
pub mod standards;
pub mod ttf;
pub mod viewer;
pub mod web;

/// Chapter descriptor from `book.json` — points at the markdown file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMeta {
    pub number: u32,
    pub title: String,
    pub file: String,
}

/// Book metadata plus the ordered chapter list; mirrors `book.json`.
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
    #[serde(default = "default_language")]
    pub language: String,
    /// Optional ISBN-13 (ISBN-10 accepted and converted). Lands in the OPF
    /// as `urn:isbn:` so KDP preflight and Kindle title metadata agree.
    #[serde(default)]
    pub isbn: Option<String>,
    pub chapters: Vec<ChapterMeta>,
}

fn default_language() -> String {
    "uk".to_string()
}

/// Load a `Book` (metadata + chapter list) from a JSON file (e.g. `book.json`).
pub fn load_book<P: AsRef<Path>>(path: P) -> Result<Book, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read book file: {}", e))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Failed to parse book JSON: {}", e))
}

/// A loaded chapter: number, title, and the full markdown content.
#[derive(Debug, Clone)]
pub struct Chapter {
    pub number: u32,
    pub title: String,
    pub content: String,
}

impl Chapter {
    /// Number of whitespace-separated words in the chapter content.
    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }
}

/// Load every chapter markdown file listed in the book, in order.
///
/// `dir` is the base directory the chapter `file` paths are relative to
/// (e.g. `"."` when run from the repo root).
pub fn load_chapters<P: AsRef<Path>>(dir: P, book: &Book) -> Result<Vec<Chapter>, String> {
    let base: PathBuf = dir.as_ref().to_path_buf();
    let mut chapters = Vec::with_capacity(book.chapters.len());
    for meta in &book.chapters {
        let path = base.join(&meta.file);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", meta.file, e))?;
        chapters.push(Chapter {
            number: meta.number,
            title: meta.title.clone(),
            content,
        });
    }
    Ok(chapters)
}
