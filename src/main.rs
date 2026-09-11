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
        "cover-template" => match cover_template(&args[2..]) {
            Ok(path) => println!("✓ Template written: {path}"),
            Err(e) => eprintln!("Error: {}", e),
        },
        "shelf" => match build_shelf() {
            Ok(()) => {}
            Err(e) => eprintln!("Error: {}", e),
        },
        "check-print" => {
            let dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "products".to_string());
            match check_print(&dir) {
                Ok(()) => {}
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
    println!("  cover-template [TRIM] [PAGES] [white|cream|ground|premium]");
    println!("               [--mode pb|hc|dj] [--out FILE.svg] [--isbn ISBN]");
    println!("               Full-wrap print template SVG at 300 DPI (KDP/Ingram geometry)");
    println!(
        "  shelf      - Build all product formats (product.json targets: ebook/paperback/hardcover)"
    );
    println!("  check-print [products] - KDP print-gate v2: verify packages vs standards");
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

/// KDP gate v2: verify built print packages under a products dir.
fn check_print(dir: &str) -> Result<(), String> {
    let root = Path::new(dir);
    if !root.exists() {
        return Err(format!("no {dir} — run `shelf` first"));
    }
    let mut any = false;
    let mut failed = 0usize;
    let mut books: Vec<_> = std::fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    books.sort();
    for book in books {
        for fmt in ["paperback", "hardcover"] {
            let pkg = book.join(fmt);
            if !pkg.join("manifest.json").exists() {
                continue;
            }
            any = true;
            println!("── {} ──", pkg.display());
            for item in rust_book::shelf::verify_package(&pkg)? {
                let mark = if item.ok { "OK  " } else { "FAIL" };
                if !item.ok {
                    failed += 1;
                }
                println!("  {mark} {} · {}", item.name, item.detail);
            }
        }
    }
    if !any {
        println!("Немає print-пакетів у {dir} (targets: paperback/hardcover у product.json).");
        return Ok(());
    }
    if failed == 0 {
        println!("✓ print gate: усі перевірки green");
    } else {
        println!("✗ print gate: {failed} FAIL");
    }
    Ok(())
}

/// Build every targeted product format for the resolved book project.
fn build_shelf() -> Result<(), String> {
    let proj = BookProject::resolve();
    let book = rust_book::load_book(&proj.json)?;
    let chapters = rust_book::load_chapters(&proj.base, &book)?;
    let cfg_path = proj.label("product.json");
    let cfg = if cfg_path.exists() {
        let bytes = std::fs::read(&cfg_path).map_err(|e| format!("read product.json: {e}"))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("bad product.json: {e}"))?
    } else {
        println!(
            "No product.json — default targets [ebook], trim 6x9, paper white, pages estimated."
        );
        rust_book::shelf::ProductConfig::default()
    };
    let paths = rust_book::shelf::build_product(&proj.base, &book, &chapters, &cfg)?;
    println!(
        "✓ Shelf built into {} (pages {}{})",
        paths.dir.display(),
        paths.pages,
        if paths.pages_estimated {
            " ESTIMATED"
        } else {
            ""
        }
    );
    for p in [&paths.ebook, &paths.paperback, &paths.hardcover]
        .into_iter()
        .flatten()
    {
        println!("  {}", p.display());
    }
    Ok(())
}

/// Generate a full-wrap cover template SVG from the KDP/Ingram geometry.
fn cover_template(args: &[String]) -> Result<String, String> {
    let mut positional: Vec<&str> = Vec::new();
    let mut mode = "pb";
    let mut out: Option<String> = None;
    let mut isbn: Option<&str> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                mode = args
                    .get(i + 1)
                    .map(String::as_str)
                    .ok_or("--mode needs a value")?;
                i += 2;
            }
            "--out" => {
                out = Some(args.get(i + 1).cloned().ok_or("--out needs a value")?);
                i += 2;
            }
            "--isbn" => {
                isbn = Some(
                    args.get(i + 1)
                        .map(String::as_str)
                        .ok_or("--isbn needs a value")?,
                );
                i += 2;
            }
            other => {
                positional.push(other);
                i += 1;
            }
        }
    }
    let mode = rust_book::cover::Mode::parse(mode)
        .ok_or_else(|| format!("unknown --mode (pb|hc|dj): {mode}"))?;
    let label = positional.first().copied().unwrap_or("6x9");
    let table = if mode == rust_book::cover::Mode::CaseLaminate {
        rust_book::standards::HARDCOVER_TRIMS
    } else {
        rust_book::standards::PAPERBACK_TRIMS
    };
    let trim = rust_book::standards::find_trim(table, label)
        .ok_or_else(|| format!("unknown trim {label} for mode {}", mode.tag()))?;
    let pages: u32 = positional
        .get(1)
        .map(|s| s.parse())
        .transpose()
        .map_err(|_| "PAGES must be a number")?
        .unwrap_or(300);
    let paper = match positional.get(2).copied().unwrap_or("white") {
        "white" => rust_book::standards::Paper::White,
        "cream" => rust_book::standards::Paper::Cream,
        "ground" => rust_book::standards::Paper::Groundwood,
        "premium" => rust_book::standards::Paper::PremiumColor,
        other => {
            return Err(format!(
                "unknown paper: {other} (white|cream|ground|premium)"
            ));
        }
    };

    let tpl = rust_book::cover::template(trim, pages, paper, mode)?;
    let svg = rust_book::cover::template_svg_with_isbn(&tpl, isbn);
    let path =
        out.unwrap_or_else(|| format!("build/cover_{}_{}_{}.svg", trim.label, pages, mode.tag()));
    if let Some(parent) = Path::new(&path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    std::fs::write(&path, svg).map_err(|e| format!("write {path}: {e}"))?;
    println!(
        "  {} {}p {:?} · {:.4}x{:.4}in · spine {:.4}in",
        trim.label, tpl.pages, paper, tpl.size.w, tpl.size.h, tpl.spine
    );
    Ok(path)
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
    rt.block_on(rust_book::web::serve_web(books, &addr))
}
