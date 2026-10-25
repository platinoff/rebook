# Rebook — HANDOFF (new session)

- **Root**: `S:/rust/rebook` (Cargo project = repo root).
- **What it is**: small, strict **EPUB 3.2** book tool — plain Markdown → KDP-ready
  EPUB, local previewer on **port 8090**, every Kindle rule checked before export.
  100% Rust, MIT. Docs: [`README.md`](../README.md) ·
  [`PRODUCTION_GUIDE.md`](../PRODUCTION_GUIDE.md) ·
  [`CONCEPT.md`](./CONCEPT.md) · [`ROADMAP.md`](./ROADMAP.md) ·
  [`VULKAN_RESEARCH.md`](./VULKAN_RESEARCH.md) ·
  [`COLORING_CARS.md`](./COLORING_CARS.md) ·
  [`COLORING_PRODUCT_BAR.md`](./COLORING_PRODUCT_BAR.md);
  authoring rules live in `.cursor/rules/epub-kdp-guide.mdc` and
  `.cursor/rules/coloring-product-bar.mdc`.
- **Book sources**: `chapters/` + `book.json` (sample book); coloring paperback
  roster `samples/coloring-cars.json`; live author titles under `workspace/`
  (gitignored). Build: `cargo run -- build-epub` / `--book DIR coloring-kdp`.
- **Product bar (every new coloring / KDP paperback):** minimum **70** interior
  pages (prefer **~90**); same art/cover/copy quality as shipped titles; only
  **content presentation** changes per book. Details:
  [`COLORING_PRODUCT_BAR.md`](./COLORING_PRODUCT_BAR.md).
- **Latest work**: coloring pipeline supports `line_only`, `plates` 1/2/4,
  local `/kdp` Print Previewer with side page nav (UI is `include_str!` — rebuild
  + restart after UI edits). RB-40…55 + RB-62 software landed (one-plate front/
  back, name/place index). Author books stay local — never commit `workspace/`.
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
