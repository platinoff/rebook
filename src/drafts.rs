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
    /// Print trim override (RB-26 meta).
    #[serde(default)]
    pub trim: Option<String>,
    /// Explicit print page count (RB-26 meta).
    #[serde(default)]
    pub pages: Option<u32>,
    /// ISBN for the print barcode (RB-26 meta).
    #[serde(default)]
    pub isbn: Option<String>,
    /// Language the author writes first (uk by default). `language` is this edition.
    #[serde(default)]
    pub source_language: Option<String>,
    /// Draft id this edition was forked from (write-then-translate).
    #[serde(default)]
    pub translation_of: Option<String>,
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
        trim: None,
        pages: None,
        isbn: None,
        source_language: None,
        translation_of: None,
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
        None => meta.chapters.push(entry), // new chapters go to the end (order is authorial)
    }
    meta.updated = now();
    std::fs::write(meta_path(root, id), meta.to_json())
        .map_err(|e| format!("write draft.json: {e}"))?;
    Ok(meta)
}

/// Delete a chapter and renumber the rest contiguously (files included).
pub fn delete_chapter(root: &Path, id: &str, number: u32) -> Result<DraftMeta, String> {
    let mut meta = load(root, id)?;
    let idx = meta
        .chapters
        .iter()
        .position(|c| c.number == number)
        .ok_or_else(|| format!("no chapter {number} in {id}"))?;
    let removed = meta.chapters.remove(idx);
    let _ = std::fs::remove_file(draft_dir(root, id).join(&removed.file));
    renumber_chapters(root, id, &mut meta)?;
    meta.updated = now();
    std::fs::write(meta_path(root, id), meta.to_json()).map_err(|e| e.to_string())?;
    Ok(meta)
}

/// Apply a new chapter order (given as the old numbering sequence, remaining
/// chapters keep relative order); renumbers `chNN` files contiguously.
pub fn reorder_chapters(root: &Path, id: &str, order: &[u32]) -> Result<DraftMeta, String> {
    let mut meta = load(root, id)?;
    let mut picked: Vec<ChapterMeta> = Vec::new();
    for n in order {
        let idx = meta
            .chapters
            .iter()
            .position(|c| c.number == *n)
            .ok_or_else(|| format!("no chapter {n} in {id}"))?;
        picked.push(meta.chapters.remove(idx));
    }
    picked.append(&mut meta.chapters);
    meta.chapters = picked;
    renumber_chapters(root, id, &mut meta)?;
    meta.updated = now();
    std::fs::write(meta_path(root, id), meta.to_json()).map_err(|e| e.to_string())?;
    Ok(meta)
}

/// Rename chapter files to contiguous ch01..chNN (extension preserved) and
/// rewrite numbering. Two-pass rename avoids same-extension collisions.
fn renumber_chapters(root: &Path, id: &str, meta: &mut DraftMeta) -> Result<(), String> {
    let dir = draft_dir(root, id);
    let ext_of = |f: &str| {
        f.rsplit_once('.')
            .map(|(_, e)| e.to_string())
            .unwrap_or_else(|| "md".to_string())
    };
    // pass 1: move to tmp names
    for (i, c) in meta.chapters.iter().enumerate() {
        std::fs::rename(dir.join(&c.file), dir.join(format!("chapters/tmp{i}.md")))
            .map_err(|e| format!("tmp rename {}: {e}", c.file))?;
    }
    // pass 2: tmp → final
    for (i, c) in meta.chapters.iter_mut().enumerate() {
        let ext = ext_of(&c.file);
        let old = c.file.clone();
        let file = format!("chapters/ch{:02}.{ext}", i + 1);
        std::fs::rename(dir.join(format!("chapters/tmp{i}.md")), dir.join(&file))
            .map_err(|e| format!("final rename {old}: {e}"))?;
        c.file = file;
        c.number = (i + 1) as u32;
    }
    Ok(())
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

/// Read an asset (path-safe name only).
pub fn load_asset(root: &Path, id: &str, name: &str) -> Result<Vec<u8>, String> {
    safe_id(id)?;
    safe_asset_name(name)?;
    let _ = load(root, id)?;
    std::fs::read(draft_dir(root, id).join("assets").join(name))
        .map_err(|e| format!("no asset {name}: {e}"))
}

/// MIME for a known asset extension (`html` is served as text, not executed).
pub fn asset_mime(name: &str) -> &'static str {
    match Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("svg") => "image/svg+xml; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("html") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Persist `draft.json` after in-memory edits (seed / fork patch).
pub fn persist_meta(root: &Path, meta: &DraftMeta) -> Result<(), String> {
    safe_id(&meta.id)?;
    std::fs::write(meta_path(root, &meta.id), meta.to_json())
        .map_err(|e| format!("write draft.json: {e}"))
}

/// Delete a draft folder entirely.
pub fn delete(root: &Path, id: &str) -> Result<(), String> {
    safe_id(id)?;
    let _ = load(root, id)?;
    std::fs::remove_dir_all(draft_dir(root, id)).map_err(|e| format!("delete {id}: {e}"))
}

/// RB-26: patch front matter (only provided fields), then persist.
#[derive(Debug, Default)]
pub struct MetaPatch<'a> {
    /// New title (ignored when empty).
    pub title: Option<&'a str>,
    /// New author.
    pub author: Option<&'a str>,
    /// `uk` | `en`.
    pub language: Option<&'a str>,
    /// ebook / paperback / hardcover.
    pub formats: Option<&'a [String]>,
    /// Print trim label.
    pub trim: Option<&'a str>,
    /// Explicit even page count.
    pub pages: Option<u32>,
    /// ISBN (validated via EAN-13).
    pub isbn: Option<&'a str>,
}

pub fn save_meta(root: &Path, id: &str, p: &MetaPatch<'_>) -> Result<DraftMeta, String> {
    let mut meta = load(root, id)?;
    if let Some(t) = p.title.map(str::trim)
        && !t.is_empty()
    {
        meta.title = t.to_string();
    }
    if let Some(a) = p.author {
        meta.author = a.trim().to_string();
    }
    if let Some(l) = p.language.map(str::trim)
        && matches!(l, "uk" | "en")
    {
        meta.language = l.to_string();
    }
    if let Some(f) = p.formats {
        let allowed = ["ebook", "paperback", "hardcover"];
        for t in f {
            if !allowed.contains(&t.as_str()) {
                return Err(format!("unknown format {t:?}"));
            }
        }
        meta.formats = f.to_vec();
    }
    if let Some(t) = p.trim.map(str::trim).filter(|t| !t.is_empty()) {
        meta.trim = Some(t.to_string());
    }
    if let Some(pg) = p.pages {
        meta.pages = Some(crate::standards::even_pages(pg));
    }
    if let Some(i) = p.isbn.map(str::trim).filter(|i| !i.is_empty()) {
        crate::standards::isbn_to_ean13(i)?;
        meta.isbn = Some(i.to_string());
    }
    meta.updated = now();
    std::fs::write(meta_path(root, &meta.id), meta.to_json()).map_err(|e| e.to_string())?;
    Ok(meta)
}

/// Decode a `data:image/…;base64,…` URI into PNG/JPG bytes.
pub(crate) fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    let mut acc: u32 = 0;
    let mut nbits = 0u32;
    let mut out = Vec::new();
    for &b in bytes {
        if b == b'=' {
            break;
        }
        let v = T.iter().position(|&c| c == b).ok_or("bad base64 byte")? as u32;
        acc = (acc << 6) | v;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            out.push((acc >> nbits) as u8);
        }
    }
    Ok(out)
}

/// RB-26: store an uploaded cover image (data-URI) as the draft's cover,
/// at both dir/cover.png (shelf auto-detect) and assets/cover.png (promote).
pub fn save_cover_img(root: &Path, id: &str, data_uri: &str) -> Result<(), String> {
    let _ = load(root, id)?;
    let b64 = data_uri
        .split_once("base64,")
        .map(|(_, r)| r)
        .ok_or("expected data:image/…;base64, URI")?;
    let bytes = b64_decode(b64)?;
    if bytes.len() < 8 || bytes.len() > 20_000_000 {
        return Err("cover image size out of range".to_string());
    }
    let is_png = bytes.starts_with(b"\x89PNG");
    let is_jpg = bytes.starts_with(&[0xFF, 0xD8]);
    if !is_png && !is_jpg {
        return Err("cover must be PNG or JPEG".to_string());
    }
    let dir = draft_dir(root, id);
    std::fs::create_dir_all(dir.join("assets")).map_err(|e| e.to_string())?;
    let name = if is_png { "cover.png" } else { "cover.jpg" };
    std::fs::write(dir.join(name), &bytes).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("assets").join(name), &bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// RB-26: build the draft's targeted print/ebook products in one pass
/// (shelf engine, trim 6×9, white paper, pages estimated). Returns file list.
pub fn build_products(root: &Path, id: &str) -> Result<Vec<String>, String> {
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
        isbn: meta.isbn.clone(),
        chapters: meta.chapters.clone(),
    };
    let chapters = crate::load_chapters(&dir, &book)?;
    let targets: Vec<String> = if meta.formats.is_empty() {
        vec!["ebook".to_string()]
    } else {
        meta.formats.clone()
    };
    let mut cfg = crate::shelf::ProductConfig {
        targets,
        trim: meta.trim.clone().unwrap_or_else(|| "6x9".to_string()),
        pages: meta.pages,
        paper: crate::shelf::PaperName::White,
        isbn: meta.isbn.clone(),
    };
    if !cfg.targets.iter().any(|t| t == "ebook") {
        cfg.targets.insert(0, "ebook".to_string());
    }
    let paths = crate::shelf::build_product(&dir, &book, &chapters, &cfg)?;
    let mut files = Vec::new();
    for p in [&paths.ebook, &paths.paperback, &paths.hardcover]
        .into_iter()
        .flatten()
    {
        files.push(
            p.strip_prefix(&dir)
                .unwrap_or(p.as_path())
                .to_string_lossy()
                .replace('\\', "/"),
        );
    }
    Ok(files)
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
        isbn: meta.isbn.clone(),
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
        cover_image: crate::epub::find_cover_file(&dir),
        language: book.language.clone(),
        isbn: book.isbn.clone(),
    };
    generate_epub(&config, &book, &chapters)?;
    Ok(out)
}

/// Fork a draft into another language edition (write-then-translate).
/// Copies chapter markdown as-is; the author translates in Studio.
/// Print ISBN is per-edition, so it is cleared.
pub fn fork_translation(root: &Path, id: &str, target_lang: &str) -> Result<DraftMeta, String> {
    if !matches!(target_lang, "uk" | "en") {
        return Err("translation target must be uk or en".to_string());
    }
    let src = load(root, id)?;
    if src.language == target_lang {
        return Err(format!("draft {id} is already {target_lang}"));
    }
    let new_id = format!("{id}-{target_lang}");
    safe_id(&new_id)?;
    let dest = draft_dir(root, &new_id);
    if dest.exists() {
        return Err(format!("draft {new_id} already exists"));
    }
    let src_dir = draft_dir(root, &src.id);
    copy_dir(&src_dir, &dest)?;
    let mut meta = src;
    meta.id = new_id.clone();
    meta.source_language = Some(meta.language.clone());
    meta.translation_of = Some(id.to_string());
    meta.language = target_lang.to_string();
    meta.isbn = None;
    meta.title = format!("{} [{}]", meta.title, target_lang);
    meta.updated = now();
    std::fs::write(meta_path(root, &meta.id), meta.to_json())
        .map_err(|e| format!("write translated draft.json: {e}"))?;
    Ok(meta)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("mkdir {to:?}: {e}"))?;
    let entries = std::fs::read_dir(from).map_err(|e| format!("read {from:?}: {e}"))?;
    for ent in entries.flatten() {
        let src = ent.path();
        let name = ent.file_name();
        let dst = to.join(&name);
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| format!("copy {name:?}: {e}"))?;
        }
    }
    Ok(())
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
    fn reorder_and_delete_renumber() {
        let r = test_root("reorder");
        let m = create(&r, "Order Book", "a", "uk").unwrap();
        save_chapter(&r, &m.id, 1, "One", "one", "md").unwrap();
        save_chapter(&r, &m.id, 2, "Two", "two", "mdc").unwrap();
        let m = save_chapter(&r, &m.id, 3, "Three", "three", "md").unwrap();
        let m = reorder_chapters(&r, &m.id, &[3, 1]).unwrap();
        assert_eq!(m.chapters[0].title, "Three");
        assert_eq!(m.chapters[0].number, 1);
        assert_eq!(m.chapters[1].title, "One");
        assert_eq!(m.chapters[2].title, "Two"); // appended, contiguous
        assert_eq!(m.chapters[2].number, 3);
        assert_eq!(m.chapters[2].file, "chapters/ch03.mdc"); // ext preserved
        assert_eq!(chapter_content(&r, &m.id, 1).unwrap(), "three".to_string());
        let m = delete_chapter(&r, &m.id, 2).unwrap();
        assert_eq!(m.chapters.len(), 2);
        assert_eq!(m.chapters[0].title, "Three");
        assert_eq!(m.chapters[1].title, "Two");
        assert_eq!(m.chapters[1].number, 2);
    }

    #[test]
    fn promote_empty_chapters_errors() {
        let r = test_root("empty");
        let m = create(&r, "No Chapters", "a", "uk").unwrap();
        assert!(promote(&r, &m.id).is_err());
    }

    #[test]
    fn fork_translation_copies_chapters_and_clears_isbn() {
        let r = test_root("translate");
        let m = create(&r, "Night Book", "A", "uk").unwrap();
        save_chapter(&r, &m.id, 1, "Один", "текст рідною", "md").unwrap();
        save_meta(
            &r,
            &m.id,
            &MetaPatch {
                isbn: Some("978-3-16-148410-0"),
                ..MetaPatch::default()
            },
        )
        .unwrap();
        let t = fork_translation(&r, &m.id, "en").unwrap();
        assert_eq!(t.id, "night-book-en");
        assert_eq!(t.language, "en");
        assert_eq!(t.source_language.as_deref(), Some("uk"));
        assert_eq!(t.translation_of.as_deref(), Some("night-book"));
        assert!(t.isbn.is_none(), "print ISBN is per-edition");
        assert_eq!(chapter_content(&r, &t.id, 1).unwrap(), "текст рідною");
        assert!(fork_translation(&r, &m.id, "uk").is_err());
        assert!(fork_translation(&r, &m.id, "en").is_err());
    }
}
