# Rebook — HANDOFF (new session)

- **Root**: `S:/rust/rebook` (Cargo project = repo root).
- **What it is**: small, strict **EPUB 3.2** book tool — plain Markdown → KDP-ready
  EPUB, local previewer on **port 8090**, every Kindle rule checked before export.
  100% Rust, MIT. Docs: [`README.md`](../README.md) ·
  [`PRODUCTION_GUIDE.md`](../PRODUCTION_GUIDE.md) ·
  [`CONCEPT.md`](./CONCEPT.md) · [`ROADMAP.md`](./ROADMAP.md) ·
  [`VULKAN_RESEARCH.md`](./VULKAN_RESEARCH.md) ·
  [`COLORING_CARS.md`](./COLORING_CARS.md);
  authoring rules live in `.cursor/rules/epub-kdp-guide.mdc`.
- **Book sources**: `chapters/` + `book.json` (sample book); coloring paperback
  roster `samples/coloring-cars.json`; build: `cargo run -- build-epub`.
- **Latest work**: **RB-40…52 landed**. RB-52 = Studio uk draft + SVG includes
  (`coloring-draft`, fork en). RB-51 = SVG plate kit. RB-50 = coloring canon
  (8.5×11, 24 cars, 46 plates, 104 p). Prior: cover sniff, OPF ISBN,
  GSV nav, write-then-translate, Studio pages, hyphenation, FOGRA39, print-art
  DPI, view3d CSS pages, AI translate + box FS.
  Three KDP types = ebook / paperback / hardcover. Gate: fmt · clippy 0 ·
  **cargo test**.
- **Live server**: `127.0.0.1:8090` — UI is `include_str!`-embedded, so a rebuild
  is not live until restart. **bash-only helpers** (no PowerShell):
  `bash /s/rust/rebook/scripts/restart-server.sh` (kill+nohup start) and
  `scripts/rebuild-restart.sh` (kill, `cargo build`, start).
- **Dev plan / bands**: [`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md);
  tickets on GSV board product `rebook` (hub `:9999`).
- **Kit registration**: row in
  [`S:/rust/GSV/docs/gsv/PRODUCTS.md`](../../GSV/docs/gsv/PRODUCTS.md).
- **Tests**: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo test`.
- **NEXT pointer**: [`NEXT_SESSION_PROMPT.md`](./NEXT_SESSION_PROMPT.md).
