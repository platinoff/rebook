use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "build-epub" => match build_epub(&args) {
            Ok(()) => println!("EPUB built successfully"),
            Err(e) => eprintln!("Error: {}", e),
        },
        "md" => match render_book_md() {
            Ok(()) => println!("rust_pered_sn_book.md written"),
            Err(e) => eprintln!("Error: {}", e),
        },
        "check" => match check(&args) {
            Ok(listing) => println!("{}", listing),
            Err(e) => eprintln!("Error: {}", e),
        },
        "convert" => {
            if args.len() < 4 {
                eprintln!("Usage: rust_book convert <epub_path> <azw3_path>");
                std::process::exit(1);
            }
            match convert_azw3(&args[2], &args[3]) {
                Ok(()) => println!("Conversion successful"),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        "kdp" => {
            if args.len() < 3 {
                eprintln!("Usage: rust_book kdp <azw3_path>");
                std::process::exit(1);
            }
            match kdp_build(&args[2]) {
                Ok(()) => println!("KDP build successful"),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        "view" => {
            let port = if args.len() >= 3 && args[2] == "--port" && args.len() >= 4 {
                args[3].clone()
            } else {
                "8090".to_string()
            };
            match view(&port) {
                Ok(()) => println!("Viewer stopped"),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_help();
        }
    }
}

/// A resolved book project: where `book.json` lives and where the EPUB goes.
///
/// If a `book.json` exists in the working directory we treat it as the user's
/// own book project. Otherwise we fall back to the bundled MIT sample so the
/// tool runs out of the box on a fresh clone.
#[derive(Clone)]
struct BookProject {
    base: std::path::PathBuf,
    json: std::path::PathBuf,
    epub: std::path::PathBuf,
}

impl BookProject {
    fn resolve() -> BookProject {
        if Path::new("book.json").exists() {
            BookProject {
                base: std::path::PathBuf::from("."),
                json: std::path::PathBuf::from("book.json"),
                epub: std::path::PathBuf::from("build/rust_book.epub"),
            }
        } else {
            BookProject {
                base: std::path::PathBuf::from("samples"),
                json: std::path::PathBuf::from("samples/book.json"),
                epub: std::path::PathBuf::from("samples/rust_book.epub"),
            }
        }
    }

    fn label(&self, extra_base: &str) -> std::path::PathBuf {
        if self.base == std::path::Path::new(".") {
            std::path::PathBuf::from(extra_base)
        } else {
            self.base.join(extra_base)
        }
    }
}

fn print_help() {
    println!("rebook - local EPUB 3.2 book tool");
    println!("==================================");
    println!("Available commands:");
    println!("  build-epub - Build a valid EPUB 3.2 (book.json + chapters/, or bundled sample)");
    println!("               Optional: --cover <path> (png/jpg/webp/gif/svg;");
    println!("               auto-detects cover.png/.jpg/.jpeg/.webp next to book.json)");
    println!("  md         - Render the whole book to a single markdown file");
    println!("  check      - Validate the built EPUB (Rust-only, zip crate)");
    println!("  view       - Local KDP EPUB previewer server (http://127.0.0.1:8090/)");
    println!("  convert    - (deprecated) KDP accepts EPUB directly");
    println!("  kdp        - (deprecated) KDP accepts EPUB directly");
}

fn build_epub(args: &[String]) -> Result<(), String> {
    println!("Building EPUB 3.2 format...");
    let proj = BookProject::resolve();

    let book = rust_book::load_book(&proj.json)?;
    let chapters = rust_book::load_chapters(&proj.base, &book)?;
    let mut total = 0usize;
    for ch in &chapters {
        total += ch.word_count();
        println!("  розділ {:02}: {} слів", ch.number, ch.word_count());
    }
    println!(
        "Loaded book: {} ({} · {} розділів, {} слів)",
        book.title,
        book.author,
        chapters.len(),
        total
    );

    let config = rust_book::epub::EpubConfig {
        title: book.title.clone(),
        author: book.author.clone(),
        output_path: proj.epub.to_string_lossy().into_owned(),
        cover_image: resolve_cover(args, &proj),
        language: book.language.clone(),
    };

    rust_book::epub::generate_epub(&config, &book, &chapters)?;

    if proj.epub.exists() {
        println!("✓ EPUB generated successfully: {}", proj.epub.display());
    }

    Ok(())
}

/// Regenerate a single aggregated markdown export from `book.json` +
/// `chapters/*.md` so the human-readable book stays in sync with the EPUB.
fn render_book_md() -> Result<(), String> {
    let proj = BookProject::resolve();
    let book = rust_book::load_book(&proj.json)?;
    let chapters = rust_book::load_chapters(&proj.base, &book)?;

    let stem = proj
        .json
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "book".to_string());
    let out_name = proj.label(&format!("{stem}.md"));

    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", book.title));
    out.push_str(&format!("Автор: {}\n", book.author));
    out.push_str(&format!("Видання: {}, {} рік\n\n", book.edition, book.year));

    for ch in &chapters {
        out.push_str("\n---\n\n");
        out.push_str(&format!("## Глава {}: {}\n\n", ch.number, ch.title));
        out.push_str(ch.content.trim_end());
        out.push('\n');
    }

    std::fs::write(&out_name, out)
        .map_err(|e| format!("Failed to write {}: {}", out_name.display(), e))?;
    println!("Written: {}", out_name.display());
    Ok(())
}

/// Rust-only EPUB validation.
fn check(args: &[String]) -> Result<String, String> {
    let proj = BookProject::resolve();
    let book = rust_book::load_book(&proj.json)?;
    let chapters = rust_book::load_chapters(&proj.base, &book)?;
    let cover_entry = resolve_cover(args, &proj)
        .as_deref()
        .and_then(rust_book::epub::cover_storage_name);
    let listing = rust_book::epub::check_epub(
        &proj.epub.to_string_lossy(),
        &book.chapters,
        cover_entry.as_deref(),
    )?;
    let total: usize = chapters.iter().map(|c| c.word_count()).sum();
    Ok(format!(
        "{}\nChapter words: {} ({} chapters)\n",
        listing,
        total,
        chapters.len()
    ))
}

/// Cover resolution: an explicit `--cover <path>` wins; otherwise look for a
/// `cover.png` / `cover.jpg` / `cover.jpeg` / `cover.webp` next to `book.json`.
fn resolve_cover(args: &[String], proj: &BookProject) -> Option<String> {
    let mut i = 0usize;
    while i + 1 < args.len() {
        if args[i] == "--cover" {
            return Some(args[i + 1].clone());
        }
        i += 1;
    }
    for candidate in ["cover.png", "cover.jpg", "cover.jpeg", "cover.webp"] {
        let path = proj.label(candidate);
        if path.exists() {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    None
}

/// KDP accepts a valid EPUB directly — no local AZW3/KFX conversion is needed.
/// Upload the `build/rust_book.epub` produced by `build-epub` to KDP.
fn convert_azw3(epub_path: &str, _azw3_path: &str) -> Result<(), String> {
    println!("EPUB: {}", epub_path);
    if !Path::new(epub_path).exists() {
        return Err(format!("EPUB file not found: {}", epub_path));
    }
    Err(
        "Local AZW3/KFX conversion is not needed: Amazon KDP accepts a valid EPUB 3.2 \
         directly and converts it server-side. Upload build/rust_book.epub to KDP instead."
            .to_string(),
    )
}

/// KDP path is a direct EPUB upload; there is no local AZW3 build step.
fn kdp_build(_azw3_path: &str) -> Result<(), String> {
    Err(
        "KDP upload is done through Kindle Direct Publishing with the EPUB file \
         (build/rust_book.epub); there is no local AZW3 build step."
            .to_string(),
    )
}

/// Start the local KDP EPUB previewer server. Bind only to loopback.
fn view(port: &str) -> Result<(), String> {
    let addr = format!("127.0.0.1:{port}");

    // Out-of-the-box: if the resolved project has no EPUB yet, build it first
    // (this makes the bundled sample instantly viewable on a fresh clone).
    let proj = BookProject::resolve();
    if !proj.epub.exists() {
        println!("No EPUB found — building {}", proj.epub.display());
        build_epub(&[])?;
    }

    // Discover every *.epub on disk (and adjacent book.json / auto-parse).
    let root = std::path::Path::new(".");
    let books = rust_book::viewer::discover_books(root)?;
    if books.is_empty() {
        println!("Не знайдено жодного *.epub у поточному каталозі.");
        println!(
            "Покладіть EPUB (напр. {}/*.epub) і запустіть заново.",
            proj.epub.parent().unwrap().display()
        );
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to start async runtime: {e}"))?;
    rt.block_on(rust_book::viewer::serve(books, &addr))
}
