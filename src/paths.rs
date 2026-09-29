//! Portable data home: the folder a copied `rust_book` binary treats as its
//! workspace. Loopback previewer and CLI both resolve books from here so a
//! user does not need the git checkout or `cargo`.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static HOME_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();
static BOOK_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

/// Coloring roster file inside a book folder.
pub const BOOK_ROSTER: &str = "roster.json";
/// Optional KDP listing copy inside a book folder.
pub const BOOK_LISTING: &str = "listing.json";

/// `rebook --book DIR` — the local (uncommitted) coloring book folder.
pub fn set_book_override(dir: PathBuf) -> Result<(), String> {
    let dir = check_book_dir(&dir)?;
    let _ = BOOK_OVERRIDE.set(dir);
    Ok(())
}

/// Local coloring book folder from `--book` or `REBOOK_BOOK`; `None` = bundled sample.
///
/// Layout: `roster.json`, optional `listing.json`, `art/*.png`, output in `build/kdp`.
pub fn book_dir() -> Option<PathBuf> {
    if let Some(p) = BOOK_OVERRIDE.get() {
        return Some(p.clone());
    }
    let raw = std::env::var("REBOOK_BOOK").ok()?;
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    check_book_dir(Path::new(raw)).ok()
}

/// Canonical book folder that holds a roster and is not a system/drive root.
pub fn check_book_dir(dir: &Path) -> Result<PathBuf, String> {
    let canon = dir
        .canonicalize()
        .map_err(|e| format!("book folder {}: {e}", dir.display()))?;
    let canon = strip_verbatim(canon);
    if is_unsafe_home(&canon) {
        return Err(format!("refusing book folder {}", canon.display()));
    }
    if !canon.join(BOOK_ROSTER).is_file() {
        return Err(format!("no {BOOK_ROSTER} in {}", canon.display()));
    }
    Ok(canon)
}

/// `\\?\S:\x` → `S:\x` so paths stay readable and PowerShell-friendly.
fn strip_verbatim(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => p,
    }
}

/// `rebook --dir PATH` / `-C PATH` — must be called before the first [`home`].
pub fn set_home_override(dir: PathBuf) {
    let _ = HOME_OVERRIDE.set(dir);
}

/// Directory that owns the user's books, drafts, and build output.
///
/// Order: `--dir` / `-C`, then `REBOOK_HOME`, then the cwd if it already looks
/// like a rebook folder, otherwise the folder that contains this executable.
pub fn home() -> PathBuf {
    if let Some(p) = HOME_OVERRIDE.get() {
        return p.clone();
    }
    if let Ok(h) = std::env::var("REBOOK_HOME") {
        let p = PathBuf::from(h.trim());
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    if let Ok(cwd) = std::env::current_dir()
        && looks_like_rebook_dir(&cwd)
    {
        return cwd;
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
        && !is_unsafe_home(dir)
    {
        return dir.to_path_buf();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Roots the previewer may walk for `*.epub`. Never the raw cwd unless it
/// already looks like a rebook folder (blocks `System32` / drive-root scans).
pub fn scan_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    push_unique(&mut roots, home());
    if let Ok(cwd) = std::env::current_dir()
        && looks_like_rebook_dir(&cwd)
    {
        push_unique(&mut roots, cwd);
    }
    roots
}

/// True when `dir` already holds a book project, the MIT sample, or this repo.
pub fn looks_like_rebook_dir(dir: &Path) -> bool {
    dir.join("book.json").is_file()
        || dir.join("samples").join("book.json").is_file()
        || dir.join("books").is_dir()
        || dir.join("workspace").is_dir()
        || dir.join("Cargo.toml").is_file()
}

/// Drive roots, Windows system folders — refuse to write or recurse there.
pub fn is_unsafe_home(dir: &Path) -> bool {
    if dir.components().count() <= 2 {
        return true;
    }
    let name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "windows"
            | "system32"
            | "syswow64"
            | "program files"
            | "program files (x86)"
            | "programdata"
            | "users"
            | "windows.old"
    )
}

/// Join `file` under `base` and reject `..` / absolute paths (chapter files).
pub fn safe_under(base: &Path, file: &str) -> Result<PathBuf, String> {
    let rel = Path::new(file);
    if rel.is_absolute() {
        return Err(format!("path must be relative: {file}"));
    }
    if rel
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!("path escapes project: {file}"));
    }
    Ok(base.join(rel))
}

fn push_unique(roots: &mut Vec<PathBuf>, p: PathBuf) {
    if !roots.iter().any(|r| r == &p) {
        roots.push(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_dir_rejected() {
        assert!(safe_under(Path::new("/proj"), "../etc/passwd").is_err());
        assert!(safe_under(Path::new("/proj"), "ch/../../x").is_err());
        assert!(safe_under(Path::new("/proj"), "chapters/01.md").is_ok());
    }

    #[test]
    fn absolute_rejected() {
        #[cfg(windows)]
        assert!(safe_under(Path::new(r"D:\proj"), r"C:\Windows\x").is_err());
        #[cfg(not(windows))]
        assert!(safe_under(Path::new("/proj"), "/tmp/x").is_err());
    }

    #[test]
    fn shallow_and_system_homes_are_unsafe() {
        assert!(is_unsafe_home(Path::new("C:\\")));
        assert!(is_unsafe_home(Path::new("C:\\Windows")));
        assert!(is_unsafe_home(Path::new("C:\\Windows\\System32")));
        assert!(!is_unsafe_home(Path::new("D:\\rebook")));
    }

    #[test]
    fn book_dir_needs_roster() {
        let tmp = std::env::temp_dir().join(format!("rebook-book-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(check_book_dir(&tmp).is_err());
        std::fs::write(tmp.join(BOOK_ROSTER), "{}").unwrap();
        let ok = check_book_dir(&tmp).unwrap();
        assert!(ok.join(BOOK_ROSTER).is_file());
        assert!(!ok.to_string_lossy().starts_with(r"\\?\"));
        assert!(check_book_dir(&tmp.join("..").join("missing-book")).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
