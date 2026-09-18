# Rebook — Next Session Prompt

1. **Owner product pick: rebook.** Window `S:/rust/rebook`. `agi` = keep-live +
   tickets (GSV `:9999` was down this drain — retry presence) then RB-44.
2. **State: RB-40…43 landed** (cover sniff, OPF ISBN, GSV nav chrome,
   write-then-translate). Prior RB-1…RB-38. **106 tests**, clippy 0.
   App = portable web on **127.0.0.1:8090**. Restart after code change:
   `bash /s/rust/rebook/scripts/rebuild-restart.sh` (`include_str!` UI).
3. **Concept:** three KDP types (ebook / paperback / hardcover) + `/view3d`.
   Author writes in source language, forks translation. Vulkan is **out**.
4. **Next band = RB-44:** Studio page-view continuous numbering (now
   per-chapter); verify ±1 vs `interior_pdf`. Then RB-45 hyphenation,
   RB-46 FOGRA39 ICC, RB-47 live-book cover DPI, RB-48 ebook flip.
5. **Known live data:** EN ebook cover was 125 DPI (needs ≥300 print);
   re-export print art after ISBN-in-OPF; HC pages still ESTIMATED for `en/`.
6. Keep green: `cargo fmt -- --check` → `cargo clippy --all-targets` →
   `cargo test`. `git commit -F target/msg.txt`.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`ROADMAP.md`](./ROADMAP.md) · [`CONCEPT.md`](./CONCEPT.md).
