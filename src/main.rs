use std::path::{Path, PathBuf};

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let args = apply_global_flags(&raw);
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
        "interior-pdf" => match build_interior(&args[2..]) {
            Ok(path) => println!("✓ Interior PDF: {path}"),
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
        "coloring-plates" => {
            let dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "samples/coloring/plates".to_string());
            match coloring_plates(&dir) {
                Ok(n) => println!("✓ {n} SVG plates → {dir}"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "coloring-draft" => {
            let dir = args.get(2).cloned().unwrap_or_else(|| {
                rust_book::paths::home()
                    .join("workspace")
                    .join("drafts")
                    .to_string_lossy()
                    .into_owned()
            });
            match coloring_draft(&dir) {
                Ok((uk, en)) => println!("✓ Studio {uk} + {en} → {dir}"),
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "coloring-kdp" => {
            let dir = args.get(2).cloned().unwrap_or_else(|| {
                rust_book::paths::home()
                    .join("build")
                    .join("coloring-kdp")
                    .to_string_lossy()
                    .into_owned()
            });
            match coloring_kdp(&dir) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        "init" => match init_project(&args[2..]) {
            Ok(dir) => println!("✓ book project ready in {}", dir.display()),
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        },
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_help();
        }
    }
}

/// `--dir` / `-C` set the portable home before any command runs.
fn apply_global_flags(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0usize;
    while i < args.len() {
        if (args[i] == "--dir" || args[i] == "-C") && i + 1 < args.len() {
            rust_book::paths::set_home_override(PathBuf::from(&args[i + 1]));
            i += 2;
            continue;
        }
        out.push(args[i].clone());
        i += 1;
    }
    out
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
        let home = rust_book::paths::home();
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let candidates = [home.clone(), cwd];
        for root in &candidates {
            let json = root.join("book.json");
            if json.is_file() {
                return BookProject {
                    base: root.clone(),
                    json,
                    epub: root.join("build").join("book.epub"),
                };
            }
        }
        for root in &candidates {
            let json = root.join("samples").join("book.json");
            if json.is_file() {
                return BookProject {
                    base: root.join("samples"),
                    json,
                    epub: root.join("samples").join("book.epub"),
                };
            }
        }
        BookProject {
            base: home.join("samples"),
            json: home.join("samples").join("book.json"),
            epub: home.join("samples").join("book.epub"),
        }
    }

    fn label(&self, extra_base: &str) -> PathBuf {
        self.base.join(extra_base)
    }
}

fn print_help() {
    println!("rebook — portable EPUB 3.2 / KDP book tool");
    println!("==========================================");
    println!("Copy rust_book.exe into a folder, then:");
    println!("  rust_book init              scaffold book.json + chapters/");
    println!("  rust_book build-epub        write build/book.epub");
    println!("  rust_book view              http://127.0.0.1:8090/  (loopback only)");
    println!();
    println!("Global: --dir PATH  or  -C PATH  or  REBOOK_HOME  (portable data folder)");
    println!("Your book = book.json + chapters/*.md [+ cover.png] in that folder.");
    println!();
    println!("Commands:");
    println!("  init [--sample] [DIR]  New book project (optional MIT sample chapters)");
    println!("  build-epub             EPUB 3.2 from book.json + chapters/");
    println!("                         Optional: --cover <path>");
    println!("  md                     One markdown export of the whole book");
    println!("  check                  Validate the built EPUB");
    println!("  view [--port N]        Local previewer + Studio (127.0.0.1)");
    println!("  cover-template …       Full-wrap print SVG (KDP/Ingram geometry)");
    println!("  shelf                  product.json → ebook / paperback / hardcover");
    println!("  check-print [DIR]      Print-gate on a products folder");
    println!("  interior-pdf [TRIM]    Interior PDF for the resolved book");
    println!("  convert · kdp          Deprecated — upload the EPUB to KDP");
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
        isbn: book.isbn.clone(),
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
    let cover_entry = resolve_cover(args, &proj).and_then(|p| {
        std::fs::read(&p)
            .ok()
            .and_then(|b| rust_book::epub::sniff_image_kind(&b).map(|k| format!("cover.{k}")))
    });
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

/// RB-15: render the interior print PDF for the resolved project.
fn build_interior(args: &[String]) -> Result<String, String> {
    let proj = BookProject::resolve();
    let book = rust_book::load_book(&proj.json)?;
    let chapters = rust_book::load_chapters(&proj.base, &book)?;
    let mut trim = "6x9";
    let mut out = rust_book::paths::home()
        .join("build")
        .join("interior.pdf")
        .to_string_lossy()
        .into_owned();
    let mut i = 0usize;
    let mut positionals: Vec<&str> = Vec::new();
    while i < args.len() {
        if args[i] == "--out" {
            out = args.get(i + 1).cloned().ok_or("--out needs a value")?;
            i += 2;
        } else {
            positionals.push(args[i].as_str());
            i += 1;
        }
    }
    if let Some(t) = positionals.first() {
        trim = t;
    } else if let Ok(bytes) = std::fs::read(proj.label("product.json"))
        && let Ok(cfg) = serde_json::from_slice::<rust_book::shelf::ProductConfig>(&bytes)
    {
        trim = Box::leak(cfg.trim.into_boxed_str());
    }
    let tr = rust_book::standards::find_trim(rust_book::standards::PAPERBACK_TRIMS, trim)
        .ok_or_else(|| format!("unknown trim {trim}"))?;
    rust_book::interior::render_interior_pdf(&book, &chapters, tr, std::path::Path::new(&out))?;
    Ok(out)
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
    let mut as_pdf = false;
    for a in &args[2..] {
        if a == "--pdf" {
            as_pdf = true;
        }
    }
    if as_pdf {
        let mut title = String::new();
        let mut author = String::new();
        let mut i2 = 0usize;
        while i2 < args.len() {
            match args[i2].as_str() {
                "--title" if i2 + 1 < args.len() => {
                    title = args[i2 + 1].clone();
                    i2 += 2;
                }
                "--author" if i2 + 1 < args.len() => {
                    author = args[i2 + 1].clone();
                    i2 += 2;
                }
                _ => i2 += 1,
            }
        }
        let mut doc = rust_book::coverdoc::CoverDoc::new(
            &title,
            &author,
            mode.tag(),
            label,
            pages,
            match paper {
                rust_book::standards::Paper::White => "white",
                rust_book::standards::Paper::Cream => "cream",
                rust_book::standards::Paper::Groundwood => "ground",
                rust_book::standards::Paper::PremiumColor => "premium",
            },
        );
        doc.isbn = isbn.map(str::to_string);
        let as_cmyk = args.iter().any(|a| a == "--cmyk");
        let pdf_path = out
            .unwrap_or_else(|| format!("build/cover_{}_{}_{}.pdf", trim.label, pages, mode.tag()));
        let opts = rust_book::coverpdf::WrapOpts {
            cmyk: as_cmyk,
            pdfx: as_cmyk,
            icc: rust_book::icc::discover_cmyk_icc(),
        };
        let rep = rust_book::coverpdf::render_wrap_pdf_opts(
            &doc,
            std::path::Path::new(&pdf_path),
            &opts,
        )?;
        println!(
            "  wrap PDF {} ({} bytes, placed={} embedded={} barcode={} bars cmyk={} ink_max={:.0}%)",
            pdf_path,
            rep.bytes,
            rep.text_placed,
            rep.text_embedded,
            rep.barcode_bars,
            rep.cmyk,
            rep.ink_max_pct
        );
        return Ok(pdf_path);
    }
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
    let home = rust_book::paths::home();
    if rust_book::paths::is_unsafe_home(&home) {
        return Err(
            "refusing to use a system folder as the book home. Run from a dedicated \
             directory, or set REBOOK_HOME / --dir"
                .into(),
        );
    }
    println!("data home: {}", home.display());

    // Out-of-the-box: seed the MIT sample next to the exe if nothing is there.
    let proj = BookProject::resolve();
    if !proj.json.is_file() {
        write_embedded_sample(&home.join("samples"))?;
    }
    if !proj.epub.exists() {
        println!("No EPUB found — building {}", proj.epub.display());
        build_epub(&[])?;
    }

    let books = rust_book::viewer::discover_books_many(&rust_book::paths::scan_roots())?;
    if books.is_empty() {
        println!("No *.epub under {}", home.display());
        println!("Run: rust_book init   then   rust_book build-epub");
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to start async runtime: {e}"))?;
    rt.block_on(rust_book::web::serve_web(books, &addr))
}

fn init_project(args: &[String]) -> Result<PathBuf, String> {
    let mut sample = false;
    let mut dir: Option<PathBuf> = None;
    for a in args {
        if a == "--sample" {
            sample = true;
        } else if a.starts_with('-') {
            return Err(format!("unknown init flag: {a}"));
        } else {
            dir = Some(PathBuf::from(a));
        }
    }
    let root = dir.unwrap_or_else(rust_book::paths::home);
    if rust_book::paths::is_unsafe_home(&root) {
        return Err("refusing to init in a system folder — pick an empty directory".into());
    }
    std::fs::create_dir_all(&root).map_err(|e| format!("mkdir {}: {e}", root.display()))?;
    if sample {
        write_embedded_sample(&root)?;
        return Ok(root);
    }
    let json = root.join("book.json");
    if json.is_file() {
        return Err(format!("{} already exists", json.display()));
    }
    let chapters = root.join("chapters");
    std::fs::create_dir_all(&chapters).map_err(|e| format!("mkdir chapters: {e}"))?;
    std::fs::write(
        chapters.join("01.md"),
        "# Chapter 1\n\nWrite your book here.\n",
    )
    .map_err(|e| format!("write chapter: {e}"))?;
    let stub = r#"{
  "title": "My Book",
  "author": "Author Name",
  "edition": 1,
  "year": 2026,
  "format": "EPUB 3.2",
  "language": "en",
  "chapters": [
    { "number": 1, "title": "Chapter 1", "file": "chapters/01.md" }
  ]
}
"#;
    std::fs::write(&json, stub).map_err(|e| format!("write book.json: {e}"))?;
    println!("  book.json");
    println!("  chapters/01.md");
    println!(
        "Next: edit chapters, then  rust_book --dir \"{}\" build-epub",
        root.display()
    );
    Ok(root)
}

fn write_embedded_sample(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir.join("chapters")).map_err(|e| format!("mkdir sample: {e}"))?;
    let files: &[(&str, &str)] = &[
        ("book.json", include_str!("../samples/book.json")),
        ("chapters/01.md", include_str!("../samples/chapters/01.md")),
        ("chapters/02.md", include_str!("../samples/chapters/02.md")),
        ("chapters/03.md", include_str!("../samples/chapters/03.md")),
    ];
    for (name, body) in files {
        let path = rust_book::paths::safe_under(dir, name)?;
        if !path.is_file() {
            std::fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
            println!("  {}", path.display());
        }
    }
    Ok(())
}

fn coloring_plates(dir: &str) -> Result<usize, String> {
    let roster = rust_book::coloring::load_roster()?;
    rust_book::coloring::kdp_ok(&roster)?;
    rust_book::coloring_svg::write_plates(&roster, Path::new(dir))
}

fn coloring_draft(dir: &str) -> Result<(String, String), String> {
    let (uk, en) = rust_book::coloring_draft::seed(Path::new(dir))?;
    Ok((uk.id, en.id))
}

fn coloring_kdp(dir: &str) -> Result<(), String> {
    let files = rust_book::coloring_kdp::package(Path::new(dir))?;
    println!("✓ interior {}", files.interior.display());
    println!("✓ wrap     {}", files.wrap.display());
    println!("✓ listing  {}", files.listing.display());
    println!();
    print!(
        "{}",
        std::fs::read_to_string(&files.listing).map_err(|e| e.to_string())?
    );
    Ok(())
}
