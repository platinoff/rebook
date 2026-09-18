# Rebook — Next Session Prompt

1. **Owner product pick: rebook.** Window `S:/rust/rebook`. `agi` = keep-live +
   tickets then RB-49.
2. **State: RB-40…48 landed** (cover sniff, OPF ISBN, GSV nav chrome,
   write-then-translate, Studio book-wide pages, interior hyphenation,
   FOGRA39 ICC, print-art ≥300 DPI, EN `opf:isbn`, `/view3d` CSS pages +
   verso/recto spread). Prior RB-1…RB-38. App = portable web on
   **127.0.0.1:8090**. Restart after code change:
   `bash /s/rust/rebook/scripts/rebuild-restart.sh` (`include_str!` UI).
3. **Concept:** three KDP types (ebook / paperback / hardcover) + `/view3d`.
   Author writes in source language, forks translation. Vulkan is **out**.
4. **Next band = RB-49:** AI-переклад чернетки (env-gated `/api/ai`). Then
   RB-50 EPUBCheck v2.
5. **Known live data:** EN print-art must be re-exported ≥1800×2700 px for 6×9;
   set `en/book.json` isbn and rebuild EPUB so OPF has `urn:isbn:`; HC pages
   still ESTIMATED for `en/`.
6. Keep green: `cargo fmt -- --check` → `cargo clippy --all-targets` →
   `cargo test`. `git commit -F target/msg.txt`.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`ROADMAP.md`](./ROADMAP.md) · [`CONCEPT.md`](./CONCEPT.md).
