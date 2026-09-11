# Rebook — HANDOFF (new session)

- **Root**: `S:/rust/rebook` (Cargo project = repo root).
- **What it is**: small, strict **EPUB 3.2** book tool — plain Markdown → KDP-ready
  EPUB, local previewer on **port 8090**, every Kindle rule checked before export.
  100% Rust, MIT. Docs: [`README.md`](../README.md) ·
  [`PRODUCTION_GUIDE.md`](../PRODUCTION_GUIDE.md) ·
  [`KDP_VIEWER_PLAN.md`](../KDP_VIEWER_PLAN.md); authoring rules live in
  `.cursor/rules/epub-kdp-guide.mdc`.
- **Book sources**: `chapters/` + `book.json` (sample book); build:
  `cargo run -- build-epub`.
- **Latest work**: bands 233–244 — **RB-1…RB-14 landed**: wave-1 plan/standards,
  cover-template, EAN-13, axum shell, drafts+promote, AI assist, shelf packages,
  Cover Studio, print gate v2; wave-2 live rescan + slug de-dup, SSE `/api/ai`,
  products shelf UI, PDF research. Next: wave-3 PDF (owner crate pick:
  genpdf interior → svg2pdf wrap → Ingram CMYK) — [`PRINT_PDF_RESEARCH.md`](./PRINT_PDF_RESEARCH.md).
  [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) (KDP ebook/paperback/hardcover matrix,
  Ingram/Lulu formulas, 10-band roadmap); `src/standards.rs` (RB-2): spine/cover/
  gutter/trim/EAN-13 math + 10 tests.
- **Dev plan / bands**: [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) — canon for
  formats & geometry; tickets on GSV board product `rebook` (RB-1…RB-10).
- **Kit registration**: row in
  [`S:/rust/GSV/docs/gsv/PRODUCTS.md`](../../GSV/docs/gsv/PRODUCTS.md).
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`.
- **NEXT pointer**: [`NEXT_SESSION_PROMPT.md`](./NEXT_SESSION_PROMPT.md).
