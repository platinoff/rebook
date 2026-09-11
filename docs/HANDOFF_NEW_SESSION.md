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
- **Latest work**: bands 233–238 — RB-1…RB-8 done (plan+standards, cover-template,
  EAN-13, axum shell `/studio /cover /api`, drafts+promote, AI assist `/api/ai`,
  shelf packages). Remaining: RB-9 Cover Studio layers, RB-10 KDP gate v2
  (see [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) §3).
  [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) (KDP ebook/paperback/hardcover matrix,
  Ingram/Lulu formulas, 10-band roadmap); `src/standards.rs` (RB-2): spine/cover/
  gutter/trim/EAN-13 math + 10 tests.
- **Dev plan / bands**: [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) — canon for
  formats & geometry; tickets on GSV board product `rebook` (RB-1…RB-10).
- **Kit registration**: row in
  [`S:/rust/GSV/docs/gsv/PRODUCTS.md`](../../GSV/docs/gsv/PRODUCTS.md).
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`.
- **NEXT pointer**: [`NEXT_SESSION_PROMPT.md`](./NEXT_SESSION_PROMPT.md).
