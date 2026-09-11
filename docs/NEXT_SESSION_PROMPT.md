# Rebook — Next Session Prompt

1. `cargo test` in `S:/rust/rebook` — keep green; EPUB path check with
   `cargo run -- build-epub` against the sample book (`chapters/` + `book.json`).
2. KDP strictness: previewer rules at `127.0.0.1:8090` must stay in lockstep with
   `.cursor/rules/epub-kdp-guide.mdc` (single source for Kindle checks).
3. One commit in `S:/rust/rebook`, push its origin remote at close; no `git add -A`,
   no `build/` / `target/` staging.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`PRODUCTION_GUIDE.md`](../PRODUCTION_GUIDE.md).
