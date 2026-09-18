# Rebook — Next Session Prompt

1. **Owner product pick: rebook.** Window `S:/rust/rebook`. `agi` = keep-live +
   tickets then RB-47.
2. **State: RB-40…46 landed** (cover sniff, OPF ISBN, GSV nav chrome,
   write-then-translate, Studio book-wide pages, interior hyphenation,
   FOGRA39 ICC drop-in + CMYK JPEG on X-1a). Prior RB-1…RB-38. **121 tests**, clippy 0.
   App = portable web on **127.0.0.1:8090**. Restart after code change:
   `bash /s/rust/rebook/scripts/rebuild-restart.sh` (`include_str!` UI).
3. **Concept:** three KDP types (ebook / paperback / hardcover) + `/view3d`.
   Author writes in source language, forks translation. Vulkan is **out**.
4. **Next band = RB-47:** eBook cover DPI ≥300 for print-art; EN OPF ISBN on
   live books. Then RB-48 ebook flip, RB-49 AI translate.
5. **Known live data:** EN ebook cover was 125 DPI (needs ≥300 print);
   re-export print art after ISBN-in-OPF; HC pages still ESTIMATED for `en/`.
6. Keep green: `cargo fmt -- --check` → `cargo clippy --all-targets` →
   `cargo test`. `git commit -F target/msg.txt`.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`ROADMAP.md`](./ROADMAP.md) · [`CONCEPT.md`](./CONCEPT.md).
