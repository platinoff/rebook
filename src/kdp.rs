//! Kindle Direct Publishing helpers.
//!
//! Amazon KDP accepts a **valid EPUB 3.2** and converts it server-side to
//! Kindle formats. There is no local AZW3 / KFX / MOBI step in rebook.

/// KDP upload configuration (EPUB path + identity). The `azw3_path` field
/// is retained so older callers still compile; it is ignored.
#[derive(Debug, Clone)]
pub struct KdpConfig {
    /// Ignored. Kept for API compatibility with the old AZW3 path.
    pub azw3_path: String,
    /// Book title
    pub title: String,
    /// Author name
    pub author: String,
    /// ISBN (optional; printed editions need one per format)
    pub isbn: Option<String>,
    /// whether to embed source for Kindle Previewer
    pub embed_source: bool,
    /// Use legacy MOBI format (deprecated since Aug 2022)
    pub legacy_mobi: bool,
}

/// Direct-upload reminder. Never writes an AZW3.
pub fn build_kdp_standard(_config: &KdpConfig) -> Result<(), String> {
    Err(KDP_UPLOAD_HINT.to_string())
}

/// Direct-upload reminder. Never shells out to a converter.
pub fn convert_epub_to_azw3(epub_path: &str, _azw3_path: &str) -> Result<(), String> {
    if !std::path::Path::new(epub_path).exists() {
        return Err(format!("EPUB file not found: {epub_path}"));
    }
    Err(KDP_UPLOAD_HINT.to_string())
}

const KDP_UPLOAD_HINT: &str = "KDP accepts a valid EPUB 3.2 directly and converts it \
     server-side. Upload the EPUB (build/*.epub) — there is no local AZW3/KFX step.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kdp_helpers_refuse_local_azw3() {
        let cfg = KdpConfig {
            azw3_path: "nope.azw3".into(),
            title: "T".into(),
            author: "A".into(),
            isbn: None,
            embed_source: false,
            legacy_mobi: false,
        };
        let e = build_kdp_standard(&cfg).unwrap_err();
        assert!(e.contains("EPUB 3.2"));
        assert!(!e.to_lowercase().contains("boko"));
    }
}
