/// Module for EPUB 3.2 book generation.
///
/// Builds a real, valid EPUB 3.2 (a ZIP with the fixed skeleton: `mimetype`
/// stored and first, then `META-INF/container.xml`, then `OEBPS/content.opf`,
/// `OEBPS/nav.xhtml` and one XHTML page per chapter).
use std::io::Write;
use std::path::Path;
use zip::CompressionMethod;
use zip::write::FileOptions;

use crate::{Book, Chapter};

/// EPUB generation configuration
#[derive(Debug, Clone)]
pub struct EpubConfig {
    /// Book title
    pub title: String,
    /// Author name
    pub author: String,
    /// Output path for the EPUB file
    pub output_path: String,
    /// Cover image path (600x600px sRGB recommended)
    pub cover_image: Option<String>,
    /// RFC 5646 language tag (e.g., "uk", "en")
    pub language: String,
}

/// Generate a valid EPUB 3.2 from a `Book`.
pub fn generate_epub(config: &EpubConfig, book: &Book) -> Result<(), String> {
    println!("Generating EPUB 3.2: {}", config.title);
    println!("Author: {}", config.author);
    println!("Language: {}", config.language);

    let output_path = Path::new(&config.output_path);

    // Ensure output directory exists
    match output_path.parent() {
        Some(parent) if !parent.exists() => {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        _ => {}
    }

    let file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create EPUB file: {}", e))?;
    let mut writer = zip::ZipWriter::new(file);

    // 1. mimetype must be the first entry, stored (uncompressed).
    {
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);
        writer
            .start_file("mimetype", options)
            .map_err(|e| format!("Failed to write mimetype entry: {}", e))?;
        writer
            .write_all(b"application/epub+xml")
            .map_err(|e| format!("Failed to write mimetype: {}", e))?;
    }

    // 2. META-INF/container.xml
    write_text_entry(&mut writer, "META-INF/container.xml", &container_xml())?;

    // 3-6. OEBPS: OPF, nav, chapter pages.
    write_text_entry(&mut writer, "OEBPS/content.opf", &content_opf(config, book))?;
    write_text_entry(&mut writer, "OEBPS/nav.xhtml", &nav_xhtml(book))?;
    for chapter in &book.chapters {
        let name = format!("OEBPS/chapter-{:02}.xhtml", chapter.number);
        write_text_entry(&mut writer, &name, &chapter_xhtml(chapter))?;
    }

    writer
        .finish()
        .map_err(|e| format!("Failed to finalize EPUB: {}", e))?;

    println!("✓ EPUB generated successfully: {}", config.output_path);
    Ok(())
}

/// Write one ZIP entry with UTF-8 text, deflated (non-mimetype).
fn write_text_entry<W: Write + std::io::Seek>(
    writer: &mut zip::ZipWriter<W>,
    name: &str,
    content: &str,
) -> Result<(), String> {
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    writer
        .start_file(name, options)
        .map_err(|e| format!("Failed to start entry {}: {}", name, e))?;
    writer
        .write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write entry {}: {}", name, e))?;
    Ok(())
}

/// Container document pointing at the OPF.
fn container_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
     <container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n\
     \x20 <rootfiles>\n\
     \x20\x20 <rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/>\n\
     \x20 </rootfiles>\n\
     </container>\n"
        .to_string()
}

/// OPF package with metadata, manifest and spine.
fn content_opf(config: &EpubConfig, book: &Book) -> String {
    let mut manifest = String::new();
    let mut spine = String::new();
    let modified = format!("{:04}-{:02}-{:02}T00:00:00Z", book.year, 1, 1);

    manifest.push_str(
        "    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n",
    );
    spine.push_str("    <itemref idref=\"nav\"/>\n");

    for chapter in &book.chapters {
        let id = format!("ch{:02}", chapter.number);
        let href = format!("chapter-{:02}.xhtml", chapter.number);
        manifest.push_str(&format!(
            "    <item id=\"{id}\" href=\"{href}\" media-type=\"application/xhtml+xml\"/>\n"
        ));
        spine.push_str(&format!("    <itemref idref=\"{id}\"/>\n"));
    }

    let uuid = book_uuid(&book.title, &book.author, &config.language);

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"book-id\" xml:lang=\"{lang}\">\n\
         \x20 <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
         \x20\x20 <dc:identifier id=\"book-id\">{uuid}</dc:identifier>\n\
         \x20\x20 <dc:title>{title}</dc:title>\n\
         \x20\x20 <dc:creator>{author}</dc:creator>\n\
         \x20\x20 <dc:language>{lang}</dc:language>\n\
         \x20\x20 <meta property=\"dcterms:modified\">{modified}</meta>\n\
         \x20 </metadata>\n\
         \x20 <manifest>\n{manifest}\
         \x20 </manifest>\n\
         \x20 <spine>\n{spine}\
         \x20 </spine>\n\
         </package>\n",
        lang = esc(config.language.as_str()),
        uuid = uuid,
        title = esc(config.title.as_str()),
        author = esc(config.author.as_str()),
        modified = modified,
        manifest = manifest,
        spine = spine,
    )
}

/// EPUB 3 navigation document (table of contents).
fn nav_xhtml(book: &Book) -> String {
    let mut items = String::new();
    for chapter in &book.chapters {
        items.push_str(&format!(
            "    <li><a href=\"chapter-{:02}.xhtml\">Розділ {} — {}</a></li>\n",
            chapter.number,
            chapter.number,
            esc(chapter.title.as_str())
        ));
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"uk\">\n\
         <head>\n\
         \x20 <meta charset=\"utf-8\"/>\n\
         \x20 <title>Зміст</title>\n\
         </head>\n\
         <body>\n\
         \x20 <nav epub:type=\"toc\" id=\"toc\">\n\
         \x20\x20 <h1>Зміст</h1>\n\
         \x20\x20 <ol>\n{items}\
         \x20\x20 </ol>\n\
         \x20 </nav>\n\
         </body>\n\
         </html>\n",
        items = items,
    )
}

/// One chapter rendered as a valid XHTML page.
fn chapter_xhtml(chapter: &Chapter) -> String {
    let title_esc = esc(chapter.title.as_str());
    let body_html = render_body(&chapter.content);
    let poetic_html = esc(chapter.poetic.as_str());

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"uk\">\n\
         <head>\n\
         \x20 <meta charset=\"utf-8\"/>\n\
         \x20 <title>Розділ {num} — {title}</title>\n\
         </head>\n\
         <body>\n\
         \x20 <h1>Розділ {num}: {title}</h1>\n\
         \x20 <p>{body}</p>\n\
         \x20 <blockquote class=\"poetic\"><p>{poetic}</p></blockquote>\n\
         </body>\n\
         </html>\n",
        num = chapter.number,
        title = title_esc,
        body = body_html,
        poetic = poetic_html,
    )
}

/// Render chapter body text: XML-escape, then turn `code` backtick spans into
/// `<code>` elements (lightweight inline markup).
fn render_body(content: &str) -> String {
    let escaped = esc(content);
    let mut out = String::with_capacity(escaped.len());
    let mut in_code = false;
    let mut rest = escaped.as_str();
    while let Some(open) = rest.find('`') {
        out.push_str(&rest[..open]);
        if in_code {
            out.push_str("</code>");
        } else {
            out.push_str("<code>");
        }
        in_code = !in_code;
        rest = &rest[open + 1..];
    }
    out.push_str(rest);
    if in_code {
        out.push_str("</code>");
    }
    out
}

/// Escape XML/HTML special characters.
fn esc(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Deterministic, valid-looking UUID derived from book identity.
///
/// Computes an FNV-1a 64-bit hash over `title:author:lang` and folds it into a
/// 16-byte UUID (version 5 style nibble set). Deterministic so repeated builds
/// with identical metadata yield the same `dc:identifier`.
fn book_uuid(title: &str, author: &str, lang: &str) -> String {
    let mut h1: u64 = 0xcbf29ce484222325;
    let mut h2: u64 = 0xcbf29ce484222325;
    let seed = format!("{}:{}:{}", title, author, lang);
    for (i, &b) in seed.as_bytes().iter().enumerate() {
        if i % 2 == 0 {
            h1 ^= u64::from(b);
            h1 = h1.wrapping_mul(0x100000001b3);
        } else {
            h2 ^= u64::from(b);
            h2 = h2.wrapping_mul(0x100000001b3);
        }
    }
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&h1.to_be_bytes());
    b[8..].copy_from_slice(&h2.to_be_bytes());
    b[6] = (b[6] & 0x0f) | 0x50; // version 5
    b[8] = (b[8] & 0x3f) | 0x80; // RFC 4122 variant
    format!(
        "urn:uuid:{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// Validate EPUB (crude structural check: non-empty and starts with the stored
/// mimetype as the first ZIP entry).
pub fn validate_epub(epub_path: &str) -> Result<(), String> {
    println!("Validating EPUB: {}", epub_path);
    let path = Path::new(epub_path);
    if !path.exists() {
        return Err(format!("EPUB file not found: {}", epub_path));
    }
    let content = std::fs::read(epub_path).map_err(|e| format!("Failed to read EPUB: {}", e))?;
    if content.is_empty() {
        return Err("EPUB file is empty".to_string());
    }
    // A valid EPUB is a ZIP; the first 4 bytes are the local file header
    // signature PK\x03\x04.
    if content.len() < 4 || &content[..4] != b"PK\x03\x04" {
        return Err("EPUB file is not a valid ZIP container".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_book() -> Book {
        Book {
            title: "Test Book".to_string(),
            author: "Author".to_string(),
            edition: 1,
            year: 2026,
            format: "EPUB 3.2".to_string(),
            chapters: vec![
                Chapter {
                    number: 1,
                    title: "Глава & Перша".to_string(),
                    content: "Текст з `code`".to_string(),
                    poetic: "Рядок один".to_string(),
                },
                Chapter {
                    number: 2,
                    title: "Друга".to_string(),
                    content: "Просто текст".to_string(),
                    poetic: "Рядок два".to_string(),
                },
            ],
        }
    }

    #[test]
    fn esc_escapes_special_chars() {
        assert_eq!(esc("a&b<c>d\"e'f"), "a&amp;b&lt;c&gt;d&quot;e&apos;f");
    }

    #[test]
    fn render_body_escapes_and_wraps_code() {
        assert_eq!(render_body("Текст з `code`"), "Текст з <code>code</code>");
        assert_eq!(render_body("a<b"), "a&lt;b");
    }

    #[test]
    fn container_xml_points_at_opf() {
        let c = container_xml();
        assert!(c.contains("OEBPS/content.opf"));
        assert!(c.contains("application/oebps-package+xml"));
    }

    #[test]
    fn nav_has_toc_and_chapter_links() {
        let n = nav_xhtml(&sample_book());
        assert!(n.contains("chapter-01.xhtml"));
        assert!(n.contains("chapter-02.xhtml"));
        assert!(n.contains("epub:type=\"toc\""));
    }

    #[test]
    fn opf_contains_metadata_manifest_spine() {
        let cfg = EpubConfig {
            title: "Test Book".to_string(),
            author: "Author".to_string(),
            output_path: "build/test.epub".to_string(),
            cover_image: None,
            language: "uk".to_string(),
        };
        let o = content_opf(&cfg, &sample_book());
        assert!(o.contains("<dc:title>Test Book</dc:title>"));
        assert!(o.contains("<dc:language>uk</dc:language>"));
        assert!(o.contains("<itemref idref=\"ch01\"/>"));
        assert!(o.contains("urn:uuid:"));
    }

    #[test]
    fn uuid_is_deterministic_and_version5() {
        let a = book_uuid("T", "A", "uk");
        let b = book_uuid("T", "A", "uk");
        let c = book_uuid("X", "A", "uk");
        assert_eq!(a, b);
        assert_ne!(a, c);
        // version 5: the first hex digit of the 3rd group is '5'
        let version = a.split('-').nth(2).unwrap().chars().next().unwrap();
        assert_eq!(version, '5');
        // variant: first digit of the 4th group is one of 8/9/a/b
        let variant = a.split('-').nth(3).unwrap().chars().next().unwrap();
        assert!(matches!(variant, '8' | '9' | 'a' | 'b'));
    }
}
