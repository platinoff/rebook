/// Module for Kindle Direct Publishing (KDP) publishing
///
/// Provides KDP publishing functionality for book distribution.
use std::path::Path;

/// KDP publishing configuration
#[derive(Debug, Clone)]
pub struct KdpConfig {
    /// Path to the AZW3/KFX file
    pub azw3_path: String,
    /// Book title
    pub title: String,
    /// Author name
    pub author: String,
    /// ISBN (optional)
    pub isbn: Option<String>,
    /// whether to embed source for Kindle Previewer
    pub embed_source: bool,
    /// Use legacy MOBI format (deprecated since Aug 2022)
    pub legacy_mobi: bool,
}

/// Build for KDP standard (AZW3/KFX)
pub fn build_kdp_standard(config: &KdpConfig) -> Result<(), String> {
    println!("Building KDP standard AZW3/KFX...");
    println!("Title: {}", config.title);
    println!("Author: {}", config.author);
    println!("File: {}", config.azw3_path);

    let azw3_path = Path::new(&config.azw3_path);
    if !azw3_path.exists() {
        return Err(format!("AZW3 file not found: {}", config.azw3_path));
    }

    println!("✓ KDP build validated: {}", config.azw3_path);
    Ok(())
}

/// Convert EPUB to AZW3/KFX using boko CLI
pub fn convert_epub_to_azw3(epub_path: &str, azw3_path: &str) -> Result<(), String> {
    println!("Converting EPUB to AZW3/KFX...");
    println!("EPUB: {}", epub_path);
    println!("AZW3 output: {}", azw3_path);

    // Use boko CLI to convert
    let result = std::process::Command::new("boko")
        .args(["convert", epub_path, azw3_path])
        .output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                println!("✓ Conversion successful");
                println!("{}", stdout);
                Ok(())
            } else {
                Err(format!("Conversion failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to run boko: {}", e)),
    }
}
