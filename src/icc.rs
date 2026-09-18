//! RB-46: CMYK ICC drop-in for PDF/X-1a wrap (`/DestOutputProfile`).
//!
//! The real **Coated FOGRA39** (ECI `ISOcoated_v2_eci.icc`, ~0.5 MiB) is **not**
//! shipped — drop the file in `icc/` or set `REBOOK_ICC_CMYK` to its path.
//! Bytes are accepted only when the ICC header says `acsp` + color space `CMYK`.

use std::path::{Path, PathBuf};

/// Relative drop-in names searched under cwd and the crate root.
pub const DROP_IN: &[&str] = &[
    "icc/FOGRA39.icc",
    "icc/ISOcoated_v2_eci.icc",
    "icc/ISOcoated_v2_300_eci.icc",
    "icc/CoatedFOGRA39.icc",
];

/// True when `bytes` look like a 4-component (CMYK) ICC profile.
pub fn looks_cmyk(bytes: &[u8]) -> bool {
    bytes.len() >= 128 && &bytes[36..40] == b"acsp" && &bytes[16..20] == b"CMYK"
}

/// Read one file; `None` if missing or not a CMYK ICC.
pub fn read_cmyk_icc_file(path: &Path) -> Option<Vec<u8>> {
    let b = std::fs::read(path).ok()?;
    looks_cmyk(&b).then_some(b)
}

/// Search `REBOOK_ICC_CMYK` then [`DROP_IN`] under cwd and `CARGO_MANIFEST_DIR`.
pub fn discover_cmyk_icc() -> Option<Vec<u8>> {
    discover_cmyk_icc_in(&candidate_paths())
}

/// Same as [`discover_cmyk_icc`] but with an explicit path list (tests).
pub fn discover_cmyk_icc_in(paths: &[PathBuf]) -> Option<Vec<u8>> {
    paths.iter().find_map(|p| read_cmyk_icc_file(p))
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("REBOOK_ICC_CMYK") {
        let p = p.trim();
        if !p.is_empty() {
            out.push(PathBuf::from(p));
        }
    }
    let mut roots = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    for root in roots {
        for rel in DROP_IN {
            out.push(root.join(rel));
        }
    }
    out
}

/// Minimal structurally-valid CMYK ICC header (128 bytes, no tags).
/// For tests / DestOutputProfile wiring — not a substitute for FOGRA39 on press.
pub fn minimal_cmyk_icc() -> Vec<u8> {
    let mut v = vec![0u8; 128];
    v[0..4].copy_from_slice(&128u32.to_be_bytes());
    v[12..16].copy_from_slice(b"prtr");
    v[16..20].copy_from_slice(b"CMYK");
    v[20..24].copy_from_slice(b"XYZ ");
    v[36..40].copy_from_slice(b"acsp");
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_detects_cmyk_and_rejects_junk() {
        let icc = minimal_cmyk_icc();
        assert!(looks_cmyk(&icc));
        assert!(!looks_cmyk(&icc[..40]));
        let mut rgb = icc.clone();
        rgb[16..20].copy_from_slice(b"RGB ");
        assert!(!looks_cmyk(&rgb));
    }

    #[test]
    fn read_file_accepts_drop_in() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("icc-rb46");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("FOGRA39.icc");
        std::fs::write(&p, minimal_cmyk_icc()).unwrap();
        assert!(discover_cmyk_icc_in(&[p]).is_some());
        let junk = dir.join("srgb.icc");
        std::fs::write(&junk, b"not an icc").unwrap();
        assert!(discover_cmyk_icc_in(&[junk]).is_none());
    }
}
