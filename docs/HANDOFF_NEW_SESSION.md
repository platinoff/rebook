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
- **Latest work**: bands 233–263 — **RB-1…RB-38 landed**. Since RB-29:
  **RB-30** touchpad/keyboard everywhere (pinch, Ctrl+wheel, arrows/PgUp/0 on both
  3D stages, pinch + Ctrl+± on the cover canvas, pointer-based chapter reorder);
  **RB-31** Studio sticky status bar (row:col, words/chars, ≈pages, editor/preview
  zoom Ctrl+wheel/±/0, survives zen); **RB-32** Studio **page-by-page view** — CSS
  multi-column flow sliced per print page with a recto/verso spread, ⏮◀▶⏭ +
  «page N / M» jump, mirrors interior metrics; **RB-33/34** interior PDF now uses
  an **own engine** (justification via TJ, widow/orphan control, footer page
  numbers, per-book **font subsetting** — a real book dropped 3.75 MB → 0.78 MB;
  `REBOOK_PDF_ENGINE=genpdf` keeps the legacy path); **RB-35** clippy-clean;
  **RB-36** preflight reports **which cover file** was found (EPUB entry path +
  draft `cover.png|jpg`); **RB-37** dust-jacket 3D (jacket wrap + fore-edge flaps
  over the hc case on /view3d and the cover mini-3D); **RB-38** JPEG front art
  rides the cover-wrap PDF as a DCTDecode XObject (PNG → convert-to-JPEG report;
  CMYK/X-1a stays vector-only). Head `82954a5`, **98 tests**.
- **Live server**: `127.0.0.1:8090` — UI is `include_str!`-embedded, so a rebuild
  is not live until restart. **bash-only helpers** (no PowerShell):
  `bash /s/rust/rebook/scripts/restart-server.sh` (kill+nohup start) and
  `scripts/rebuild-restart.sh` (kill, `cargo build`, start). The CLI tool wrapper
  may print `ChildProcess.kill` on detached start — the server still comes up.
- **Dev plan / bands**: [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) — canon for
  formats & geometry; tickets on GSV board product `rebook` (RB-1…RB-38).
- **Kit registration**: row in
  [`S:/rust/GSV/docs/gsv/PRODUCTS.md`](../../GSV/docs/gsv/PRODUCTS.md).
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`.
- **NEXT pointer**: [`NEXT_SESSION_PROMPT.md`](./NEXT_SESSION_PROMPT.md).
