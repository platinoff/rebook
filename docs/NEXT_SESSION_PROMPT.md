# Rebook — Next Session Prompt

1. **Owner product pick: rebook.** Window `S:/rust/rebook`. `agi` = keep-live +
   tickets then RB-53 (paperback wrap for Classic American Iron).
2. **State: RB-40…52 landed.** Studio seed: `cargo run -- coloring-draft`.
   SVG plates via `cargo run -- coloring-plates`.
   Canon: [`COLORING_CARS.md`](./COLORING_CARS.md) (8.5×11, 24 cars, 46 plates, 104 p).
   Prior: cover sniff, OPF ISBN, GSV nav, write-then-translate, Studio pages,
   hyphenation, FOGRA39, print-art DPI, view3d CSS pages, AI translate + box FS.
   App = portable web on **127.0.0.1:8090**. Restart after code change:
   `bash /s/rust/rebook/scripts/rebuild-restart.sh` (`include_str!` UI).
3. **Concept:** three KDP types (ebook / paperback / hardcover) + `/view3d`.
   Coloring book = paperback SKU. Author writes uk, forks en. Vulkan is **out**.
4. **Next band = RB-53:** paperback wrap (color cover, barcode, spine text).
   Then RB-54 3D → RB-55 preflight before publish.
5. **Known live data:** EN print-art must be re-exported ≥1800×2700 px for 6×9;
   set `en/book.json` isbn and rebuild EPUB so OPF has `urn:isbn:`; HC pages
   still ESTIMATED for `en/`.
6. Keep green: `cargo fmt -- --check` → `cargo clippy --all-targets` →
   `cargo test`. `git commit -F target/msg.txt`.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`ROADMAP.md`](./ROADMAP.md) · [`CONCEPT.md`](./CONCEPT.md) ·
[`COLORING_CARS.md`](./COLORING_CARS.md).
