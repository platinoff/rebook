# Rebook — Next Session Prompt

1. **Owner product pick: rebook.** Window `S:/rust/rebook`. `agi` = keep-live +
   GSV tickets (product `rebook`), take the next open RB in order.
2. **State: RB-40…55 landed.** Coloring paperback #1 is live on KDP. Pipeline:
   `coloring-draft` → `coloring-kdp` (interior.pdf 130 p + one-JPEG wrap, no
   fonts) → local Print Previewer `/kdp`. Portable binary: `init`, `--dir`,
   `REBOOK_HOME`.
3. **Next band = RB-56…61** (book #2 on the same machine). Book plan, roster,
   covers and listing copy live in `workspace/plans/` — **gitignored**. Never
   commit book text or put it in ticket bodies; git gets rebook software only.
4. **Lessons baked in:** KDP rejects brand names in keywords/description; Helvetica
   in the wrap PDF stalls the previewer; Cover slot ≠ manuscript slot; 79 pages is
   the spine-text minimum, not a cap.
5. App = portable web on **127.0.0.1:8090** (`cargo run -- view`). UI is
   `include_str!`, restart after code change.
6. Keep green: `cargo fmt -- --check` → `cargo clippy --all-targets` →
   `cargo test`.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`ROADMAP.md`](./ROADMAP.md) · [`CONCEPT.md`](./CONCEPT.md).
