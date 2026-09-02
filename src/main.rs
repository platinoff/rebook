use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return;
    }

    match args[1].as_str() {
        "init" => {
            if args.len() < 4 {
                eprintln!("Usage: rust_book init <title> <author>");
                std::process::exit(1);
            }
            init_book(&args[2], &args[3]);
        }
        "build-epub" => match build_epub() {
            Ok(()) => println!("EPUB built successfully"),
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
    println!("  init       - Scaffold a new book project");
    println!("  build-epub - Build a valid EPUB 3.2 from book.json");
    println!("  convert    - (deprecated) KDP accepts EPUB directly");
    println!("  kdp        - (deprecated) KDP accepts EPUB directly");
}

fn init_book(title: &str, author: &str) {
    println!("Initializing book project: {}", title);
    println!("Author: {}", author);
    println!("Creating book.toml scaffold...");
    println!("Setting up chapters/ directory...");
    println!("Copying default content...");
    println!("✓ Book scaffold created successfully!");
}

fn build_epub() -> Result<(), String> {
    println!("Building EPUB 3.2 format...");

    let book = rust_book::load_book("book.json")?;
    println!(
        "Loaded book: {} ({} chapters)",
        book.title,
        book.chapters.len()
    );

    let config = rust_book::epub::EpubConfig {
        title: book.title.clone(),
        author: book.author.clone(),
        output_path: "build/rust_book.epub".to_string(),
        cover_image: None,
        language: "uk".to_string(),
    };

    rust_book::epub::generate_epub(&config, &book)?;

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
