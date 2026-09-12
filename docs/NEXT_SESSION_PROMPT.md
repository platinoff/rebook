# Rebook — Next Session Prompt

1. **State: bands 233–260 landed (RB-1…RB-29).** App = portable web service on
   **127.0.0.1:8090** (detached instance may still run — `Stop-Process rust_book`
   before `cargo build`; it locks the exe). Nav everywhere: Полиця / Studio /
   Cover / 🧊 3D / Продукти.
2. **Preflight first**: open `/view3d`, pick a published EPUB → real cover on the
   3D stand + KDP overlays/warnings. Known live issue: **EN ebook cover 751×1200
   = 125 DPI @6″** — re-export print art ≥1800px before ordering; EN OPF has no
   ISBN (KDP will stamp).
3. **Print packages**: `en/product.json` → `cargo run -- shelf` in `en/` — HC zip
   at `en/products/rust-before-sleep/rust-before-sleep-hc.zip`; pages ESTIMATED
   (456) — set exact `"pages"` from a KDP proof first.
4. **Backlog tickets (next bands):** RB-16c raster images inside cover-wrap PDF
   + font subsetting (11.7MB interior); RB-27b dust-jacket 3D; Ingram strict
   PDF/X needs an FOGRA39 ICC drop-in via `REBOOK_ICC_CMYK`; preflight UX: show
   which cover file was found; interior page numbers (recto).
5. Keep green: `cargo fmt -- --check` → `cargo clippy --all-targets` → `cargo
   test` (92). One commit in `S:/rust/rebook`, push origin at close.
6. Kit quirk: PowerShell eats quotes — **always commit with `git commit -F
   target/msg.txt`**; two old commits have truncated messages (`8002b24`,
   GSV `8024360`), amend+force-push only if the owner says.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) · [`PRINT_PDF_RESEARCH.md`](./PRINT_PDF_RESEARCH.md).
