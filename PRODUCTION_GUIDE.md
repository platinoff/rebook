# Production Guide: "Rust перед сном"

## Overview
Electronic book "Rust перед сном" (Rust before sleep) by Artem Platinov, Edition 1, 2026.
14 chapters, 30 min reading per chapter = 30*14 min total, designed for relaxation before sleep. Book in Ukrainian languages, searching English official for needs.

## Current State
All source files created in `S:\rust\rebook`:
- `Rust_pered_sn_book.md` - Full markdown book (231 lines)
- `Rust_pered_sn_chapters.txt` - Chapter content (229 lines)
- `Rust_pered_sn_rules.txt` - Local creation rules (81 lines)
- `Rust_pered_sn_lib.txt` - Research summary (205 lines)

## How to Actually Produce the Book

### Step 1: Install Required Tools
```bash
cd /s/rust/GSV
cargo install bookbinder        # EPUB/PDF generation
cargo install bookbinder_epub    # EPUB rendering crate
cargo install boko               # EPUB → AZW3/KFX conversion
cargo install kindling-cli       # KDP/Kindle publishing
# Note: Requires LaTeX for PDF: install via MikTeX or TeX Live
```

### Step 2: Scaffold the Book Project
```bash
papyrust init my-rust-book
# Creates directory: my-rust-book/
# Contents:
#   book.toml          - Metadata (title, author, ISBN)
#   cover.jpg          - Cover image (recommended 600x600px)
#   chapters/          - Chapter Markdown files
#   front-matter/      - Optional front pages
#   back-matter/       - Optional back pages
```

### Step 3: Populate with Content
Copy the prepared chapters into the scaffold:

```bash
# Example: Copy chapter 1
cp content_from_Rust_pered_sn_book.md my-rust-book/chapters/01-chapter.md

# Edit book.toml:
title = "Rust перед сном"
author = "Artem Platinov"
edition = 1
isbn = "978-0-1234567-8-9"
# Trim size defaults: 5.5×8.5 inches (popular for novels)
```

### Step 4: Build EPUB
```bash
cd my-rust-book
papyrust build epub
# Output: build/my-rust-book.epub (EPUB 3.2 format)
# Validated against epubcheck
```

### Step 5: Convert for Kindle (AZW3/KFX)
```bash
boko convert build/my-rust-book.epub build/my-rust-book.azw3
# Output: Kindle-compatible AZW3/KFX format
# KFX preferred format as of 2026
# AZW3 = Kindle Format 8 (older)
# KFX = Kindle Format 10 (newer, with hyphenation/ligatures)
```

### Step 6: Optional - KDP Direct Upload
```bash
# Method A: Standard book (recommended 2026)
kindling-cli build build/my-rust-book.epub --no-embed-source
# Output: build/my-rust-book.azw3 (KF8-only, default for books)

# Method B: Dictionary (if applicable)
kindling-cli build build/my-rust-book.epub --legacy-mobi
# Output: Dual-format .mobi (.mobi7 + .kf8)

# Method C: With embedded source for Previewer
kindling-cli build build/my-rust-book.epub
# Output: .azw3 + embedded EPUB for Kindle Previewer compatibility
```

### Step 7: Validation & Quality Check
```bash
# Validate EPUB
papyrust validate

# Check AZW3 contents
boko info build/my-rust-book.azw3

# Verify cover displays correctly
# Check: no HTML > 30MB (Kindle limit since Aug 2022)
# Verify: EB Garamond font embedded in PDF if generating PDF
```

## Chapter Content Mapping
The 14 chapters from the research map to papyrust chapter files:

| Chapter | Topic | Approx. Words |
|---------|-------|--------------|
| 1 | Introduction | ~200 |
| 2 | Basics (variables, types) | ~250 |
| 3 | Ownership | ~250 |
| 4 | Error Handling (Result/Option) | ~250 |
| 5 | Async (async/await, Tokio) | ~250 |
| 6 | Pattern Matching | ~250 |
| 7 | Generics and Traits | ~250 |
| 8 | Concurrency (threads, channels) | ~250 |
| 9 | Macros | ~250 |
| 10 | Cargo and Crates | ~250 |
| 11 | Testing | ~250 |
| 12 | Web with Axum | ~250 (includes code example) |
| 13 | Book Creation (how this book was made) | ~250 |
| 14 | Conclusion | ~200 |

**Total: ~3,500-4,000 words across 14 chapters**

## Production Checklist
- [ ] Install: `cargo install bookbinder boko kindling-cli`
- [ ] Scaffold: `papyrust init my-rust-book`
- [ ] Copy chapters from `Rust_pered_sn_chapters.txt` / `rust_pered_sn_book.md`
- [ ] Edit `book.toml` with metadata (title, author, ISBN)
- [ ] Place cover image: `cover.jpg` (600x600px, sRGB)
- [ ] Build EPUB: `papyrust build epub`
- [ ] Convert to Kindle: `boco convert epub azw3`
- [ ] KDP upload: `kindling-cli build azw3`
- [ ] Validate: `papyrust validate` + `boko info azw3`

## Output Formats Summary
| Format | Tool | Description |
|--------|------|-------------|
| EPUB 3.2 | papyrust build epub | Standard e-reader format |
| AZW3/KFX | boko convert | Amazon Kindle format (2026 standard) |
| PDF | papyrust build pdf / bookbinder | Print/desktop reading |
| MOBI | kindling-cli --legacy-mobi | Legacy (deprecated Aug 2022) |

## Rust Toolchain Requirements
- Rust 1.85.0+ (stable, 2024 edition)
- `edition = "2024"` in all Cargo.toml files
- Code examples: `cargo check --edition 2024`
- Dependencies: axum 0.8+, tokio 1.49+, bookbinder latest

## Poetic Requirements (enforced by local rules)
- Each chapter: 1-2 poetic lines
- Themes: calm, sleep, stars, night, flow, growth, patience
- No dark/aggressive themes
- Poetry must relate to chapter's technical concept
- Example from Chapter 5: "await not as waiting, but as yielding"

## Research Sources (official only)
- doc.rust-lang.org (stable Rust book, English)
- edition-guide.rust-lang.org (Rust 2024, English)
- docs.axum.rs (Axum web framework, English)
- docs.rs/bookbinder (book generation, English)
- irvj.github.io/papyrust (book scaffolding, English)
- kindling-cli docs (Kindle publishing, English)
- Ukraine official docs for Ukrainian localization (if needed)

## License
MIT License (or specify appropriate license in book.toml)