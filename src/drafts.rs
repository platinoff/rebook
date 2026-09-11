//! Studio drafts: a local-first workspace under `workspace/drafts/<id>/`.
//!
//! One folder per draft book — `draft.json` (front matter), `chapters/chNN.md`
//! (or `.mdc`), `assets/*` (html/svg/images). Autosave lands on the chapter
//! files; `promote` renders a strict EPUB through the existing pipeline into
//! `build/`. Everything stays inside the workspace tree (kit rule: no roaming
//! temp dirs); the whole `workspace/` is gitignored like live book content.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::epub::{EpubConfig, generate_epub};
use crate::viewer::slug;
use crate::{Book, ChapterMeta};

/// Allowed asset extensions (html/svg/images) — everything else is rejected.
pub const ASSET_EXTS: &[&str] = &["html", "svg", "png", "jpg", "jpeg", "webp", "gif"];

/// Draft front matter, persisted as `draft.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftMeta {
    /// Stable folder slug (ASCII `[a-z0-9-]`).
    pub id: String,
    /// Book title.
    pub title: String,
    /// Author name.
    pub author: String,
    /// RFC 5646 language tag.
    pub language: String,
    /// Target formats: ebook / paperback / hardcover.
    #[serde(default)]
    pub formats: Vec<String>,
    /// Chapter list (number/title/file, file relative to the draft dir).
    #[serde(default)]
    pub chapters: Vec<ChapterMeta>,
    /// Unix seconds of the last save.
    #[serde(default)]
    pub updated: u64,
}

impl DraftMeta {
    /// JSON body for the API.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Folder of one draft (id must already be a safe slug).
pub fn draft_dir(root: &Path, id: &str) -> PathBuf {
    root.join(id)
}

fn meta_path(root: &Path, id: &str) -> PathBuf {
    draft_dir(root, id).join("draft.json")
}

/// Reject ids that are not pure slugs (blocks `..`, separators, empties).
fn safe_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 || slug(id) != id {
        return Err(format!("unsafe draft id: {id:?}"));
    }
    Ok(())
}

/// Validate an asset file name: no separators, known extension.
fn safe_asset_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name.starts_with('.')
    {
        return Err(format!("unsafe asset name: {name:?}"));
    }
    match Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        Some(ext) if ASSET_EXTS.contains(&ext.as_str()) => Ok(()),
        _ => Err(format!(
            "asset extension not allowed: {name:?} (use {})",
            ASSET_EXTS.join("/")
        )),
    }
}

/// Create a new draft folder + `draft.json`; returns its metadata.
pub fn create(root: &Path, title: &str, author: &str, language: &str) -> Result<DraftMeta, String> {
    let id = slug(title);
    safe_id(&id)?;
    let dir = draft_dir(root, &id);
    if dir.exists() {
        return Err(format!("draft {id} already exists"));
    }
    std::fs::create_dir_all(dir.join("chapters")).map_err(|e| format!("create {id}: {e}"))?;
    let meta = DraftMeta {
        id,
        title: title.to_string(),
        author: author.to_string(),
        language: if language.trim().is_empty() {
            "uk".to_string()
        } else {
            language.to_string()
        },
        formats: vec!["ebook".to_string()],
        chapters: Vec::new(),
        updated: now(),
    };
    std::fs::write(meta_path(root, &meta.id), meta.to_json())
        .map_err(|e| format!("write draft.json: {e}"))?;
    Ok(meta)
}

/// Load one draft's metadata.
pub fn load(root: &Path, id: &str) -> Result<DraftMeta, String> {
    safe_id(id)?;
    let bytes = std::fs::read(meta_path(root, id)).map_err(|e| format!("no draft {id}: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("bad draft.json {id}: {e}"))
}

/// All drafts, newest first.
pub fn list(root: &Path) -> Result<Vec<DraftMeta>, String> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return Ok(out),
    };
    for ent in entries.flatten() {
        if !ent.path().is_dir() {
            continue;
        }
        let name = ent.file_name().to_string_lossy().into_owned();
        if let Ok(m) = load(root, &name) {
            out.push(m);
        }
    }
    out.sort_by_key(|m| std::cmp::Reverse(m.updated));
    Ok(out)
}

/// Save (or upsert) one chapter as `chapters/chNN.ext` and refresh metadata.
pub fn save_chapter(
    root: &Path,
    id: &str,
    number: u32,
    title: &str,
    content: &str,
    ext: &str,
) -> Result<DraftMeta, String> {
    if ext != "md" && ext != "mdc" {
        return Err(format!("chapter extension not allowed: {ext} (md/mdc)"));
    }
    let mut meta = load(root, id)?;
    let file = format!("chapters/ch{number:02}.{ext}");
    let path = draft_dir(root, id).join(&file);
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("mkdir chapters: {e}"))?;
    std::fs::write(&path, content).map_err(|e| format!("write {file}: {e}"))?;
    let entry = ChapterMeta {
        number,
        title: title.to_string(),
        file: file.clone(),
    };
    match meta.chapters.iter_mut().find(|c| c.number == number) {
        Some(existing) => *existing = entry,
        None => meta.chapters.push(entry),
    }
    meta.chapters.sort_by_key(|c| c.number);
    meta.updated = now();
    std::fs::write(meta_path(root, id), meta.to_json())
        .map_err(|e| format!("write draft.json: {e}"))?;
    Ok(meta)
}

/// Read a saved chapter's text (None when absent).
pub fn chapter_content(root: &Path, id: &str, number: u32) -> Option<String> {
    let meta = load(root, id).ok()?;
    let file = meta
        .chapters
        .iter()
        .find(|c| c.number == number)?
        .file
        .clone();
    std::fs::read_to_string(draft_dir(root, id).join(file)).ok()
}

/// Save the draft's `cover.json` (Cover Studio document).
pub fn save_cover(root: &Path, id: &str, json: &str) -> Result<(), String> {
    let _ = load(root, id)?;
    serde_json::from_str::<crate::coverdoc::CoverDoc>(json)
        .map_err(|e| format!("bad cover.json: {e}"))?;
    std::fs::write(draft_dir(root, id).join("cover.json"), json)
        .map_err(|e| format!("write cover.json: {e}"))
}

/// Read the draft's `cover.json` if present.
pub fn load_cover(root: &Path, id: &str) -> Option<String> {
    std::fs::read_to_string(draft_dir(root, id).join("cover.json")).ok()
}

/// Write an asset file (html/svg/images) under `assets/`.
pub fn save_asset(root: &Path, id: &str, name: &str, bytes: &[u8]) -> Result<(), String> {
    safe_asset_name(name)?;
    let meta = load(root, id)?;
    let dir = draft_dir(root, &meta.id).join("assets");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir assets: {e}"))?;
    std::fs::write(dir.join(name), bytes).map_err(|e| format!("write asset {name}: {e}"))
}

/// Delete a draft folder entirely.
pub fn delete(root: &Path, id: &str) -> Result<(), String> {
    safe_id(id)?;
    let _ = load(root, id)?;
    std::fs::remove_dir_all(draft_dir(root, id)).map_err(|e| format!("delete {id}: {e}"))
}

/// Promote a draft: render its markdown through the strict EPUB pipeline.
/// Returns the built EPUB path. Requires at least one chapter.
pub fn promote(root: &Path, id: &str) -> Result<PathBuf, String> {
    let meta = load(root, id)?;
    if meta.chapters.is_empty() {
        return Err(format!("draft {} has no chapters", meta.id));
    }
    let dir = draft_dir(root, &meta.id);
    let book = Book {
        title: meta.title.clone(),
        author: meta.author.clone(),
        edition: 1,
        year: (now() / 31_557_600) as u32 + 1,
        format: "EPUB 3.2".to_string(),
        language: meta.language.clone(),
        chapters: meta.chapters.clone(),
    };
    let chapters = crate::load_chapters(&dir, &book)?;
    let build = dir.join("build");
    std::fs::create_dir_all(&build).map_err(|e| format!("mkdir build: {e}"))?;
    let out = build.join(format!("{}.epub", meta.id));
    let config = EpubConfig {
        title: book.title.clone(),
        author: book.author.clone(),
        output_path: out.to_string_lossy().into_owned(),
        cover_image: crate::epub::cover_storage_name(
            &dir.join("assets/cover.png").to_string_lossy(),
        )
        .map(|_| dir.join("assets/cover.png").to_string_lossy().into_owned())
        .filter(|p| Path::new(p).exists()),
        language: book.language.clone(),
    };
    generate_epub(&config, &book, &chapters)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_root(tag: &str) -> PathBuf {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("drafts-test-{tag}"));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn create_list_save_promote_roundtrip() {
        let r = test_root("roundtrip");
        let m = create(&r, "My Test Book", "A. B", "en").unwrap();
        assert_eq!(m.id, "my-test-book");
        assert_eq!(list(&r).unwrap().len(), 1);
        let m = save_chapter(&r, &m.id, 1, "One", "# One\n\nhi", "md").unwrap();
        assert_eq!(m.chapters.len(), 1);
        assert_eq!(chapter_content(&r, &m.id, 1).unwrap(), "# One\n\nhi");
        let ep = promote(&r, &m.id).unwrap();
        assert!(ep.exists());
        let listing = crate::epub::check_epub(&ep.to_string_lossy(), &m.chapters, None).unwrap();
        assert!(listing.contains("all checks passed"));
        delete(&r, &m.id).unwrap();
        assert!(list(&r).unwrap().is_empty());
    }

    #[test]
    fn path_traversal_rejected() {
        let r = test_root("traversal");
        create(&r, "Ok", "a", "uk").unwrap();
        assert!(load(&r, "..").is_err());
        assert!(load(&r, "ok/../x").is_err());
        assert!(save_asset(&r, "ok", "../evil.png", b"x").is_err());
        assert!(save_asset(&r, "ok", "notes.exe", b"x").is_err());
        assert!(save_asset(&r, "ok", "diagram.SVG", b"<svg/>").is_ok());
        assert!(save_chapter(&r, "ok", 1, "t", "x", "exe").is_err());
    }

    #[test]
    fn duplicate_and_missing() {
        let r = test_root("dup");
        create(&r, "Same", "a", "uk").unwrap();
        assert!(create(&r, "Same", "b", "uk").is_err());
        assert!(load(&r, "absent").is_err());
        assert!(promote(&r, "absent").is_err());
    }

    #[test]
    fn promote_empty_chapters_errors() {
        let r = test_root("empty");
        let m = create(&r, "No Chapters", "a", "uk").unwrap();
        assert!(promote(&r, &m.id).is_err());
    }
}
