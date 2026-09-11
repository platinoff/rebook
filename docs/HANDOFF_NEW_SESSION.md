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
- **Latest work**: band 234 — `cover-template` CLI (full-wrap SVG 300 DPI, pb/hc/dj,
  barcode zone; rebook `7ab79cf`). Earlier band 233 — wave-1 standardization research →
  [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) (KDP ebook/paperback/hardcover matrix,
  Ingram/Lulu formulas, 10-band roadmap); `src/standards.rs` (RB-2): spine/cover/
  gutter/trim/EAN-13 math + 10 tests.
- **Dev plan / bands**: [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) — canon for
  formats & geometry; tickets on GSV board product `rebook` (RB-1…RB-10).
- **Kit registration**: row in
  [`S:/rust/GSV/docs/gsv/PRODUCTS.md`](../../GSV/docs/gsv/PRODUCTS.md).
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`.
- **NEXT pointer**: [`NEXT_SESSION_PROMPT.md`](./NEXT_SESSION_PROMPT.md).
