# Rebook — HANDOFF (new session)

- **Root**: `S:/rust/rebook` (Cargo project = repo root).
- **What it is**: small, strict **EPUB 3.2** book tool — plain Markdown → KDP-ready
  EPUB, local previewer on **port 8090**, every Kindle rule checked before export.
  100% Rust, MIT. Docs: [`README.md`](../README.md) ·
  [`PRODUCTION_GUIDE.md`](../PRODUCTION_GUIDE.md) ·
  [`CONCEPT.md`](./CONCEPT.md) · [`ROADMAP.md`](./ROADMAP.md) ·
  [`VULKAN_RESEARCH.md`](./VULKAN_RESEARCH.md);
  authoring rules live in `.cursor/rules/epub-kdp-guide.mdc`.
- **Book sources**: `chapters/` + `book.json` (sample book); build:
  `cargo run -- build-epub`.
- **Latest work**: **RB-40…47 landed** (this `agi` drain). Cover magic-byte sniff;
  OPF `urn:isbn:` + kdp.rs no AZW3; GSV-style nav `data-tip` + uk/en chrome;
  write-then-translate fork; Studio book-wide page numbers; Knuth–Liang
  hyphenation; FOGRA39 ICC drop-in (`REBOOK_ICC_CMYK` / `icc/*.icc`, ECI file
  not shipped) + CMYK JPEG passthrough on X-1a wrap; print-art DPI gate
  (`print_art_ok` ≥300, wrap skips sub-300 JPEG) + EN `kdp_check` `opf:isbn`
  from `book.json`. Prior: bands 233–263, RB-1…RB-38. Three KDP types =
  ebook / paperback / hardcover; virtual stand = `/view3d`. Gate: fmt ·
  clippy 0 · **cargo test**.
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
