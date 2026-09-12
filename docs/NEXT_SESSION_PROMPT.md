# Rebook — Next Session Prompt

1. **State: bands 233–263 landed (RB-1…RB-38), 98 tests green, head `82954a5`.**
   App = portable web service on **127.0.0.1:8090**. Server lifecycle is pure
   MSYS2 bash: `bash /s/rust/rebook/scripts/rebuild-restart.sh` after any code
   change (UI pages are `include_str!` — no restart = no visible change). Do
   NOT kill it with a visible `cmd start` window — the user closes it and the
   server dies; the scripts use `nohup` + hidden.
2. **Interior PDF = own engine** (`src/interior_pdf.rs`, default): justification,
   widow/orphan ≥2, footer numbers, per-book subset font. genpdf fallback:
   `REBOOK_PDF_ENGINE=genpdf`. Verify the Studio page-view (V) numbers match a
   real build ±1 on the owner's draft; the engine counts browser-lineflow pages,
   the PDF counts genpdf-free lineflow — small drift is possible.
3. **Known live data**: EN ebook cover was 125 DPI (needs ≥300 print), EN OPF has
   no ISBN — re-export print art; HC pages still ESTIMATED for `en/` book.
4. **Backlog / next candidates**: FOGRA39 ICC drop-in for strict Ingram X-1a
   (env `REBOOK_ICC_CMYK` already honored, file not shipped); CMYK JPEG passthrough
   for raster on X-1a wraps (currently skipped → convert tool); Studio page-view
   per-book continuous numbering (now per-chapter); hyphenation in interior
   (rustybuzz is already a transitive dep); spread mirror for RTL?
5. Keep green: `cargo fmt -- --check` → `cargo clippy --all-targets` (product
   warnings = 0) → `cargo test` (98). Commits in `S:/rust/rebook`;
   `git commit -F target/msg.txt` (PowerShell eats inline messages).
6. **Push done at close**: bands 261–263 are ahead of `origin/master` — push
   when the owner confirms.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) · [`PRINT_PDF_RESEARCH.md`](./PRINT_PDF_RESEARCH.md).
