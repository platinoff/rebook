/// Module for EPUB 3.2 book generation.
///
/// Builds a valid EPUB 3.2 (a ZIP with the fixed skeleton: `mimetype` stored
/// and first, then `META-INF/container.xml`, then `OEBPS/content.opf`,
/// `OEBPS/nav.xhtml`, `OEBPS/styles.css` and one XHTML page per chapter, plus
/// an optional cover image + cover page).
/// Chapter markdown is rendered to XHTML with a small, safe renderer.
use std::io::{Read, Seek, Write};
use std::path::Path;
use zip::CompressionMethod;
use zip::write::FileOptions;

use crate::{Book, Chapter, ChapterMeta};

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

const STYLES_CSS: &str = r#"body {
    font-family: Georgia, "EB Garamond", "Times New Roman", serif;
    line-height: 1.7;
    margin: 5%;
    color: #1a1a1a;
}
h1, h2, h3, h4 {
    font-family: Georgia, "EB Garamond", serif;
    line-height: 1.3;
}
h1 { font-size: 1.6em; margin-top: 0.4em; }
h2 { font-size: 1.3em; margin-top: 1.6em; }
h3 { font-size: 1.1em; margin-top: 1.2em; }
p { margin: 0.8em 0; text-align: justify; }
code {
    font-family: "Cascadia Code", Consolas, monospace;
    background: #f3f3f3;
    padding: 0 0.2em;
    font-size: 0.92em;
}
pre {
    background: #f5f5f5;
    border: 1px solid #ddd;
    padding: 0.8em;
    overflow-wrap: break-word;
    font-size: 0.88em;
}
pre code { background: none; padding: 0; }
blockquote {
    margin: 1.2em 0;
    padding: 0.2em 0 0.2em 1.2em;
    border-left: 3px solid #b8a88a;
    font-style: italic;
    color: #4a4a4a;
}
hr { border: none; border-top: 1px solid #ddd; margin: 1.8em 0; }
"#;

/// Generate a valid EPUB 3.2 from a `Book` and its loaded chapters.
pub fn generate_epub(config: &EpubConfig, book: &Book, chapters: &[Chapter]) -> Result<(), String> {
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

    let cover = load_cover(config)?;
    let has_cover = cover.is_some();

    let file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create EPUB file: {}", e))?;
    let mut writer = zip::ZipWriter::new(file);

    // 1. mimetype must be the first entry, stored (uncompressed).
    {
        let options: FileOptions<'_, ()> =
            FileOptions::default().compression_method(CompressionMethod::Stored);
        writer
            .start_file("mimetype", options)
            .map_err(|e| format!("Failed to write mimetype entry: {}", e))?;
        writer
            .write_all(b"application/epub+xml")
            .map_err(|e| format!("Failed to write mimetype: {}", e))?;
    }

    // 2. META-INF/container.xml
    write_text_entry(&mut writer, "META-INF/container.xml", &container_xml())?;

    // 3-7. OEBPS: OPF, nav, styles, chapter pages.
    write_text_entry(
        &mut writer,
        "OEBPS/content.opf",
        &content_opf(config, book, chapters),
    )?;
    write_text_entry(&mut writer, "OEBPS/nav.xhtml", &nav_xhtml(book, has_cover))?;
    write_text_entry(&mut writer, "OEBPS/styles.css", STYLES_CSS)?;
    if let Some((name, bytes)) = &cover {
        write_binary_entry(&mut writer, &format!("OEBPS/{name}"), bytes)?;
        write_text_entry(&mut writer, "OEBPS/cover.xhtml", &cover_xhtml(config, name))?;
    }
    for chapter in chapters {
        let name = format!("OEBPS/chapter-{:02}.xhtml", chapter.number);
        write_text_entry(
            &mut writer,
            &name,
            &chapter_xhtml(chapter, &config.language),
        )?;
    }

    writer
        .finish()
        .map_err(|e| format!("Failed to finalize EPUB: {}", e))?;

    println!("✓ EPUB generated successfully: {}", config.output_path);
    Ok(())
}

/// Write one ZIP entry with UTF-8 text, deflated (non-mimetype).
fn write_text_entry<W: Write + Seek>(
    writer: &mut zip::ZipWriter<W>,
    name: &str,
    content: &str,
) -> Result<(), String> {
    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::Deflated);
    writer
        .start_file(name, options)
        .map_err(|e| format!("Failed to start entry {}: {}", name, e))?;
    writer
        .write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write entry {}: {}", name, e))?;
    Ok(())
}

/// Write one ZIP entry with raw bytes, deflated.
fn write_binary_entry<W: Write + Seek>(
    writer: &mut zip::ZipWriter<W>,
    name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::Deflated);
    writer
        .start_file(name, options)
        .map_err(|e| format!("Failed to start entry {}: {}", name, e))?;
    writer
        .write_all(bytes)
        .map_err(|e| format!("Failed to write entry {}: {}", name, e))?;
    Ok(())
}

/// Normalized zip entry name for a cover image, or `None` for an unsupported
/// extension. Supported: png, jpg/jpeg, webp, gif, svg.
pub fn cover_storage_name(img: &str) -> Option<String> {
    let path = Path::new(img);
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => Some(format!("cover.{ext}")),
        _ => None,
    }
}

fn cover_media_type(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/svg+xml",
    }
}

/// Read the configured cover image from disk; `Ok(None)` when no cover is set.
fn load_cover(config: &EpubConfig) -> Result<Option<(String, Vec<u8>)>, String> {
    let img = match config
        .cover_image
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        Some(img) => img,
        None => return Ok(None),
    };
    let name = cover_storage_name(img).ok_or_else(|| {
        format!(
            "Unsupported cover image type: {} (use png/jpg/webp/gif/svg)",
            img
        )
    })?;
    let bytes =
        std::fs::read(img).map_err(|e| format!("Cannot read cover image {}: {}", img, e))?;
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("png");
    println!(
        "Cover: {} -> OEBPS/{} ({})",
        img,
        name,
        cover_media_type(ext)
    );
    Ok(Some((name, bytes)))
}

/// Cover page (EPUB3: `epub:type="cover"` container + image).
fn cover_xhtml(config: &EpubConfig, img: &str) -> String {
    let title = esc(&config.title);
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"{lang}\">\n\
         <head>\n\
         \x20 <meta charset=\"utf-8\"/>\n\
         \x20 <title>{title}</title>\n\
         \x20 <link rel=\"stylesheet\" type=\"text/css\" href=\"styles.css\"/>\n\
         </head>\n\
         <body>\n\
         \x20 <div epub:type=\"cover\">\n\
         \x20\x20 <img src=\"{img}\" alt=\"{title}\"/>\n\
         \x20 </div>\n\
         </body>\n\
         </html>\n",
        lang = esc(&config.language),
        title = title,
        img = img,
    )
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
fn content_opf(config: &EpubConfig, book: &Book, chapters: &[Chapter]) -> String {
    let mut manifest = String::new();
    let mut spine = String::new();
    let mut cover_meta = String::new();
    let mut cover_manifest = String::new();
    let mut cover_spine = String::new();
    let modified = format!("{:04}-{:02}-{:02}T00:00:00Z", book.year, 1, 1);

    if let Some(img) = config
        .cover_image
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        && let Some(name) = cover_storage_name(img)
    {
        let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("png");
        cover_meta.push_str("    <meta name=\"cover\" content=\"cover-image\"/>\n");
        cover_manifest.push_str(
                "    <item id=\"cover-page\" href=\"cover.xhtml\" media-type=\"application/xhtml+xml\"/>\n",
            );
        cover_manifest.push_str(&format!(
                "    <item id=\"cover-image\" href=\"{name}\" media-type=\"{media}\" properties=\"cover-image\"/>\n",
                name = name,
                media = cover_media_type(ext),
            ));
        cover_spine.push_str("    <itemref idref=\"cover-page\"/>\n");
    }

    manifest.push_str(
        "    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n",
    );
    manifest.push_str("    <item id=\"styles\" href=\"styles.css\" media-type=\"text/css\"/>\n");
    spine.push_str("    <itemref idref=\"nav\"/>\n");

    for chapter in chapters {
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
         {cover_meta}\
         \x20\x20 <dc:identifier id=\"book-id\">{uuid}</dc:identifier>\n\
         \x20\x20 <dc:title>{title}</dc:title>\n\
         \x20\x20 <dc:creator>{author}</dc:creator>\n\
         \x20\x20 <dc:language>{lang}</dc:language>\n\
         \x20\x20 <dc:date>{year}-01-01</dc:date>\n\
         \x20\x20 <meta property=\"dcterms:modified\">{modified}</meta>\n\
         \x20 </metadata>\n\
         \x20 <manifest>\n{cover_manifest}{manifest}\
         \x20 </manifest>\n\
         \x20 <spine>\n{cover_spine}{spine}\
         \x20 </spine>\n\
         </package>\n",
        lang = esc(&config.language),
        uuid = uuid,
        title = esc(&config.title),
        author = esc(&config.author),
        year = book.year,
        modified = modified,
        cover_meta = cover_meta,
        cover_manifest = cover_manifest,
        cover_spine = cover_spine,
        manifest = manifest,
        spine = spine,
    )
}

/// EPUB 3 navigation document (table of contents). The rubric labels follow
/// the book language (`en` → "Chapter"/"Contents", anything else → "Розділ"/"Зміст").
fn nav_xhtml(book: &Book, has_cover: bool) -> String {
    let (toc_title, chapter_word, cover_word) = if book.language == "en" {
        ("Contents", "Chapter", "Cover")
    } else {
        ("Зміст", "Розділ", "Обкладинка")
    };
    let mut items = String::new();
    if has_cover {
        items.push_str(&format!(
            "     <li><a href=\"cover.xhtml\">{}</a></li>\n",
            cover_word
        ));
    }
    for meta in &book.chapters {
        items.push_str(&format!(
            "     <li><a href=\"chapter-{:02}.xhtml\">{chapter_word} {} — {}</a></li>\n",
            meta.number,
            meta.number,
            esc(&meta.title),
            chapter_word = chapter_word,
        ));
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"{lang}\">\n\
         <head>\n\
         \x20 <meta charset=\"utf-8\"/>\n\
         \x20 <title>{toc_title}</title>\n\
         \x20 <link rel=\"stylesheet\" type=\"text/css\" href=\"styles.css\"/>\n\
         </head>\n\
         <body>\n\
         \x20 <nav epub:type=\"toc\" id=\"toc\">\n\
         \x20\x20 <h1>{toc_title}</h1>\n\
         \x20\x20 <ol>\n{items}\
         \x20\x20 </ol>\n\
         \x20 </nav>\n\
         </body>\n\
         </html>\n",
        lang = esc(&book.language),
        toc_title = toc_title,
        items = items,
    )
}

/// One chapter rendered as a valid XHTML page; rubric labels follow `language`.
fn chapter_xhtml(chapter: &Chapter, language: &str) -> String {
    let chapter_word = if language == "en" {
        "Chapter"
    } else {
        "Розділ"
    };
    let title_esc = esc(&chapter.title);
    let body_html = render_markdown(&chapter.content);

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <!DOCTYPE html>\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"{lang}\">\n\
         <head>\n\
         \x20 <meta charset=\"utf-8\"/>\n\
         \x20 <title>{chapter_word} {num} — {title}</title>\n\
         \x20 <link rel=\"stylesheet\" type=\"text/css\" href=\"styles.css\"/>\n\
         </head>\n\
         <body>\n\
         \x20 <h1>{chapter_word} {num}: {title}</h1>\n\
         {body}\
         </body>\n\
         </html>\n",
        lang = esc(language),
        chapter_word = chapter_word,
        num = chapter.number,
        title = title_esc,
        body = body_html,
    )
}

/// Minimal, safe markdown → XHTML renderer for chapter prose.
///
/// Supported: `##` / `###` headings, `>` blockquotes, ``` code fences,
/// blank-line paragraphs, inline `code`, *emphasis*, **strong**.
/// All other text is XML-escaped.
fn render_markdown(md: &str) -> String {
    let mut out = String::with_capacity(md.len() + 256);
    let mut para: Vec<&str> = Vec::new();
    let mut quote: Vec<&str> = Vec::new();
    let mut code: Vec<&str> = Vec::new();
    let mut in_code = false;

    let flush = |out: &mut String, para: &mut Vec<&str>| {
        if !para.is_empty() {
            let text = para.join(" ");
            if text.trim().is_empty() {
                para.clear();
                return;
            }
            out.push_str("  <p>");
            out.push_str(&inline_html(&text));
            out.push_str("</p>\n");
            para.clear();
        }
    };

    for raw in md.lines() {
        let line = raw.trim_end();
        if in_code {
            // Closing fence
            if line.trim_start().starts_with("```") {
                in_code = false;
                out.push_str("  <pre><code>");
                let n = code.len();
                if n > 0 {
                    // drop the trailing newline we added to each code line
                    let joined = code.join("\n");
                    let stripped = joined.trim_end();
                    out.push_str(&esc(stripped));
                }
                out.push_str("</code></pre>\n");
                code.clear();
            } else {
                code.push(line);
            }
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            // opening of a code fence
            flush(&mut out, &mut para);
            if !quote.is_empty() {
                flush_quote(&mut out, &mut quote);
            }
            in_code = true;
        } else if trimmed.starts_with(">") {
            // blockquote (poetic) line
            flush(&mut out, &mut para);
            let rest = trimmed.trim_start_matches('>').trim();
            quote.push(rest);
        } else if trimmed.starts_with("### ") {
            flush(&mut out, &mut para);
            if !quote.is_empty() {
                flush_quote(&mut out, &mut quote);
            }
            out.push_str("  <h3>");
            out.push_str(&inline_html(trimmed.trim_start_matches("### ").trim()));
            out.push_str("</h3>\n");
        } else if trimmed.starts_with("## ") {
            flush(&mut out, &mut para);
            if !quote.is_empty() {
                flush_quote(&mut out, &mut quote);
            }
            out.push_str("  <h2>");
            out.push_str(&inline_html(trimmed.trim_start_matches("## ").trim()));
            out.push_str("</h2>\n");
        } else if trimmed.starts_with("---") {
            flush(&mut out, &mut para);
            if !quote.is_empty() {
                flush_quote(&mut out, &mut quote);
            }
            out.push_str("  <hr/>\n");
        } else if line.trim().is_empty() {
            // paragraph / quote boundary
            if !quote.is_empty() {
                flush_quote(&mut out, &mut quote);
            }
            flush(&mut out, &mut para);
        } else {
            para.push(trimmed);
        }
    }

    // trailing blocks
    if in_code {
        out.push_str("  <pre><code>");
        let joined = code.join("\n");
        out.push_str(&esc(joined.trim_end()));
        out.push_str("</code></pre>\n");
    }
    if !quote.is_empty() {
        flush_quote(&mut out, &mut quote);
    }
    flush(&mut out, &mut para);

    out
}

fn flush_quote(out: &mut String, quote: &mut Vec<&str>) {
    if quote.is_empty() {
        return;
    }
    out.push_str("  <blockquote>");
    for (i, q) in quote.iter().enumerate() {
        if i > 0 {
            out.push_str("<br/>");
        }
        out.push_str(&inline_html(q));
    }
    out.push_str("</blockquote>\n");
    quote.clear();
}

/// Inline markdown: escape, then wrap `code`, **strong**, *emphasis*.
fn inline_html(text: &str) -> String {
    let escaped = esc(text);
    let mut out = String::with_capacity(escaped.len());
    let bytes = escaped.as_bytes();
    let mut i = 0;
    let n = bytes.len();
    while i < n {
        // `code` span
        if bytes[i] == b'`' {
            let mut j = i + 1;
            while j < n && bytes[j] != b'`' {
                j += 1;
            }
            if j < n {
                out.push_str("<code>");
                out.push_str(&escaped[i + 1..j]);
                out.push_str("</code>");
                i = j + 1;
                continue;
            }
        }
        // **strong**
        if bytes[i] == b'*' && i + 1 < n && bytes[i + 1] == b'*' {
            let mut j = i + 2;
            while j + 1 < n && !(bytes[j] == b'*' && bytes[j + 1] == b'*') {
                j += 1;
            }
            if j + 1 < n {
                out.push_str("<strong>");
                out.push_str(&escaped[i + 2..j]);
                out.push_str("</strong>");
                i = j + 2;
                continue;
            }
        }
        // *emphasis*
        if bytes[i] == b'*' {
            let mut j = i + 1;
            while j < n && bytes[j] != b'*' {
                j += 1;
            }
            if j < n {
                out.push_str("<em>");
                out.push_str(&escaped[i + 1..j]);
                out.push_str("</em>");
                i = j + 1;
                continue;
            }
        }
        // Copy one UTF-8 character as-is
        let len = utf8_len(bytes[i]);
        out.push_str(&escaped[i..i + len]);
        i += len;
    }
    out
}

/// Length (in bytes) of the UTF-8 sequence starting at `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
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

/// Verify a produced EPUB with the `zip` reader: the fixed skeleton, the
/// stored `mimetype` first entry, and every chapter page present.
/// Returns a human-readable listing (Rust-first validation — no external tools).
pub fn check_epub(
    epub_path: &str,
    chapters: &[ChapterMeta],
    cover: Option<&str>,
) -> Result<String, String> {
    let path = Path::new(epub_path);
    let file = std::fs::File::open(path).map_err(|e| format!("EPUB file not found: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Not a valid ZIP: {}", e))?;

    if archive.is_empty() {
        return Err("EPUB archive is empty".to_string());
    }

    let mut listing = String::new();

    // Entry 0 must be mimetype, stored, exact content.
    {
        let mut first = archive
            .by_index(0)
            .map_err(|e| format!("Cannot read first entry: {}", e))?;
        if first.name() != "mimetype" {
            return Err(format!(
                "First entry is {} — must be mimetype (invalid EPUB)",
                first.name()
            ));
        }
        if first.compression() != CompressionMethod::Stored {
            return Err(format!(
                "mimetype is not stored (uncompressed): {:?}",
                first.compression()
            ));
        }
        let mut content = Vec::new();
        first
            .read_to_end(&mut content)
            .map_err(|e| format!("Cannot read mimetype: {}", e))?;
        if content != b"application/epub+xml" {
            return Err("mimetype content is not application/epub+xml".to_string());
        }
        listing.push_str("OK   mimetype (stored, application/epub+xml)\n");
    }

    let mut required = vec![
        "META-INF/container.xml".to_string(),
        "OEBPS/content.opf".to_string(),
        "OEBPS/nav.xhtml".to_string(),
        "OEBPS/styles.css".to_string(),
    ];
    if let Some(name) = cover {
        required.push(format!("OEBPS/{name}"));
        required.push("OEBPS/cover.xhtml".to_string());
    }
    let names: Vec<String> = (0..archive.len())
        .map(|i| {
            let f = archive.by_index(i);
            f.map(|x| x.name().to_string()).unwrap_or_default()
        })
        .collect();

    for want in &required {
        if !names.contains(want) {
            return Err(format!("Missing required entry: {}", want));
        }
        listing.push_str(&format!("OK   {}\n", want));
    }

    for meta in chapters {
        let want = format!("OEBPS/chapter-{:02}.xhtml", meta.number);
        if !names.iter().any(|n| n == &want) {
            return Err(format!("Missing chapter entry: {}", want));
        }
        listing.push_str(&format!("OK   {}  ({})\n", want, meta.title));
    }

    for name in &names {
        if name == "mimetype" {
            continue;
        }
        if !required.iter().any(|r| r == name) && !name.starts_with("OEBPS/chapter-") {
            listing.push_str(&format!("note extra entry: {}\n", name));
        }
    }

    listing.push_str(&format!(
        "{} entries total, all checks passed\n",
        names.len()
    ));
    Ok(listing)
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
            language: "uk".to_string(),
            chapters: vec![
                ChapterMeta {
                    number: 1,
                    title: "Глава & Перша".to_string(),
                    file: "chapters/01.md".to_string(),
                },
                ChapterMeta {
                    number: 2,
                    title: "Друга".to_string(),
                    file: "chapters/02.md".to_string(),
                },
            ],
        }
    }

    fn sample_chapters() -> Vec<Chapter> {
        vec![
            Chapter {
                number: 1,
                title: "Глава & Перша".to_string(),
                content: "## Початок\n\nТекст з `code` та *курсивом*.\n\n> Рядок поетичний"
                    .to_string(),
            },
            Chapter {
                number: 2,
                title: "Друга".to_string(),
                content: "Просто текст.\n\n```rust\nlet x = 1;\n```".to_string(),
            },
        ]
    }

    #[test]
    fn esc_escapes_special_chars() {
        assert_eq!(esc("a&b<c>d\"e'f"), "a&amp;b&lt;c&gt;d&quot;e&apos;f");
    }

    #[test]
    fn render_markdown_headings_and_paragraphs() {
        let html = render_markdown("## Заголовок\n\nАбзац з @ знаком та *ем* і **болд**.");
        assert!(html.contains("<h2>Заголовок</h2>"));
        assert!(html.contains("<em>ем</em>"));
        assert!(html.contains("<strong>болд</strong>"));
        assert!(html.contains("@"));
    }

    #[test]
    fn render_markdown_code_fence_is_escaped() {
        let html = render_markdown("```rust\nlet a = 1 < 2;\n```");
        assert!(html.contains("<pre><code>let a = 1 &lt; 2;</code></pre>"));
    }

    #[test]
    fn render_markdown_blockquote() {
        let html = render_markdown("Звичайний абзац.\n\n> Один рядок\n> Другий рядок");
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("Один рядок<br/>Другий рядок"));
    }

    #[test]
    fn container_xml_points_at_opf() {
        let c = container_xml();
        assert!(c.contains("OEBPS/content.opf"));
        assert!(c.contains("application/oebps-package+xml"));
    }

    #[test]
    fn nav_has_toc_and_chapter_links() {
        let n = nav_xhtml(&sample_book(), false);
        assert!(n.contains("chapter-01.xhtml"));
        assert!(n.contains("chapter-02.xhtml"));
        assert!(n.contains("epub:type=\"toc\""));
    }

    #[test]
    fn nav_links_cover_when_present() {
        let n = nav_xhtml(&sample_book(), true);
        assert!(n.contains("cover.xhtml"));
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
        let o = content_opf(&cfg, &sample_book(), &sample_chapters());
        assert!(o.contains("<dc:title>Test Book</dc:title>"));
        assert!(o.contains("<dc:language>uk</dc:language>"));
        assert!(o.contains("styles.css"));
        assert!(o.contains("<itemref idref=\"ch01\"/>"));
        assert!(o.contains("urn:uuid:"));
        assert!(!o.contains("properties=\"cover-image\""));
    }

    #[test]
    fn nav_and_chapter_use_language_labels() {
        let n = nav_xhtml(&sample_book(), false);
        assert!(n.contains("<h1>Зміст</h1>"));
        assert!(n.contains("Розділ 1 — Глава &amp; Перша"));

        let mut en = sample_book();
        en.language = "en".to_string();
        let n = nav_xhtml(&en, true);
        assert!(n.contains("<h1>Contents</h1>"));
        assert!(n.contains("Chapter 1 — Глава &amp; Перша"));
        assert!(n.contains(">Cover</a>"));

        let ch = chapter_xhtml(&sample_chapters()[0], "en");
        assert!(ch.contains("<h1>Chapter 1: Глава &amp; Перша</h1>"));
        assert!(ch.contains("xml:lang=\"en\""));
        assert!(!ch.contains("Розділ"));
    }

    #[test]
    fn opf_cover_meta_manifest_spine() {
        let cfg = EpubConfig {
            title: "Test Book".to_string(),
            author: "Author".to_string(),
            output_path: "build/test.epub".to_string(),
            cover_image: Some("assets/cover.PNG".to_string()),
            language: "uk".to_string(),
        };
        let o = content_opf(&cfg, &sample_book(), &sample_chapters());
        assert!(o.contains("<meta name=\"cover\" content=\"cover-image\"/>"));
        assert!(
            o.contains("href=\"cover.png\" media-type=\"image/png\" properties=\"cover-image\"")
        );
        assert!(o.contains("href=\"cover.xhtml\" media-type=\"application/xhtml+xml\""));
        assert!(o.contains("<itemref idref=\"cover-page\"/>"));
    }

    #[test]
    fn cover_storage_name_normalizes() {
        assert_eq!(cover_storage_name("c.jpg").as_deref(), Some("cover.jpg"));
        assert_eq!(cover_storage_name("x.PNG").as_deref(), Some("cover.png"));
        assert_eq!(cover_storage_name("y.bmp"), None);
        assert_eq!(cover_storage_name("noext"), None);
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
