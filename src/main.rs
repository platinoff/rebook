use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "build-epub" => match build_epub() {
            Ok(()) => println!("EPUB built successfully"),
            Err(e) => eprintln!("Error: {}", e),
        },
        "md" => match render_book_md() {
            Ok(()) => println!("rust_pered_sn_book.md written"),
            Err(e) => eprintln!("Error: {}", e),
        },
        "check" => match check() {
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

fn print_help() {
    println!("Rust перед сном - Book Creation Tool");
    println!("=====================================");
    println!("Available commands:");
    println!("  build-epub - Build a valid EPUB 3.2 from book.json + chapters/");
    println!("  md         - Regenerate rust_pered_sn_book.md from the chapters");
    println!("  check      - Validate build/rust_book.epub (Rust-only, zip crate)");
    println!("  view       - Local KDP EPUB previewer server (http://127.0.0.1:8090/)");
    println!("  convert    - (deprecated) KDP accepts EPUB directly");
    println!("  kdp        - (deprecated) KDP accepts EPUB directly");
}

fn build_epub() -> Result<(), String> {
    println!("Building EPUB 3.2 format...");

    let book = rust_book::load_book("book.json")?;
    let chapters = rust_book::load_chapters(".", &book)?;
    let mut total = 0usize;
    for ch in &chapters {
        total += ch.word_count();
        println!("  розділ {:02}: {} слів", ch.number, ch.word_count());
    }
    println!(
        "Loaded book: {} ({}. {} розділів, {} слів)",
        book.title,
        book.author,
        chapters.len(),
        total
    );

    let config = rust_book::epub::EpubConfig {
        title: book.title.clone(),
        author: book.author.clone(),
        output_path: "build/rust_book.epub".to_string(),
        cover_image: None,
        language: book.language.clone(),
    };

    rust_book::epub::generate_epub(&config, &book, &chapters)?;

    let epub_path = Path::new("build/rust_book.epub");
    if epub_path.exists() {
        println!("✓ EPUB generated successfully: {}", epub_path.display());
        println!(
            "   Location: {}",
            std::env::current_dir().unwrap_or_default().display()
        );
    }

    Ok(())
}

/// Regenerate `rust_pered_sn_book.md` from `book.json` + `chapters/*.md`
/// so the human-readable book stays in sync with the EPUB source.
fn render_book_md() -> Result<(), String> {
    let book = rust_book::load_book("book.json")?;
    let chapters = rust_book::load_chapters(".", &book)?;

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

    std::fs::write("rust_pered_sn_book.md", out)
        .map_err(|e| format!("Failed to write rust_pered_sn_book.md: {}", e))?;
    Ok(())
}

/// Rust-only EPUB validation.
fn check() -> Result<String, String> {
    let book = rust_book::load_book("book.json")?;
    let chapters = rust_book::load_chapters(".", &book)?;
    let listing = rust_book::epub::check_epub("build/rust_book.epub", &book.chapters)?;
    let total: usize = chapters.iter().map(|c| c.word_count()).sum();
    Ok(format!(
        "{}\nChapter words: {} ({} chapters)\n",
        listing,
        total,
        chapters.len()
    ))
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

    // Discover every *.epub on disk (and adjacent book.json / auto-parse).
    let root = std::path::Path::new(".");
    let books = rust_book::viewer::discover_books(root)?;
    if books.is_empty() {
        println!("Не знайдено жодного *.epub у поточному каталозі.");
        println!("Покладіть EPUB (напр. build/rust_book.epub) і запустіть заново.");
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to start async runtime: {e}"))?;
    rt.block_on(rust_book::viewer::serve(books, &addr))
}
