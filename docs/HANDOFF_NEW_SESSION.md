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
- **Latest work**: bands 233–260 — **RB-1…RB-29 landed**: web app (unified nav,
  3-pane Studio + meta panel + build-products, cover canvas + 🧊 /view3d with
  **EPUB→3D preflight** and KDP warning panel, products shelf), strict EPUB,
  print packages (interior/cover-wrap PDF, PDF/X-1a CMYK), standards/gates
  (head `e97f47f`, 92 tests). Key findings: EN cover 125 DPI (needs ≥300 for
  print), EN OPF lacks ISBN, HC pages 456 ESTIMATED (verify vs proof).
  Backlog: RB-16c raster/subset, RB-27b DJ-3D, ICC drop-in, preflight cover-file hint.
- **Dev plan / bands**: [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) — canon for
  formats & geometry; tickets on GSV board product `rebook` (RB-1…RB-29).
- **Kit registration**: row in
  [`S:/rust/GSV/docs/gsv/PRODUCTS.md`](../../GSV/docs/gsv/PRODUCTS.md).
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`.
- **NEXT pointer**: [`NEXT_SESSION_PROMPT.md`](./NEXT_SESSION_PROMPT.md).
