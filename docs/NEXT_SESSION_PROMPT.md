# Rebook — Next Session Prompt

1. **Owner product pick: rebook.** Window `S:/rust/rebook`. `agi` = keep-live +
   GSV tickets (product `rebook`), take the next open RB in order.
2. **Product bar (mandatory for any new coloring / KDP paperback):** ≥ **70**
   interior pages (prefer **~90**); same quality as shipped titles; only content
   presentation differs. Canon:
   [`COLORING_PRODUCT_BAR.md`](./COLORING_PRODUCT_BAR.md) ·
   `.cursor/rules/coloring-product-bar.mdc`.
3. **State:** coloring pipeline = `line_only` + plates 1/2/4 → `coloring-kdp` →
   `/kdp` previewer (side ‹› + arrow keys; UI embedded — rebuild+restart).
   RB-40…55 + RB-62 software landed. Portable binary: `init`, `--dir`,
   `REBOOK_HOME`.
4. **Author content** lives in `workspace/` — **gitignored**. Never commit book
   text/art/PDFs or put them in ticket bodies. Push tool + docs only.
5. **Lessons baked in:** KDP rejects brand names in keywords/description;
   Helvetica in the wrap PDF stalls the previewer; Cover slot ≠ manuscript;
   79 pages = spine-text **minimum**, not a cap; shared `assets/cover-*.png`
   collide across books — keep covers in `art/` or book-prefixed names; sync
   must not wipe painted covers.
6. App = portable web on **127.0.0.1:8090**. Keep green: `cargo fmt -- --check`
   → `cargo clippy --all-targets` → `cargo test`.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`COLORING_PRODUCT_BAR.md`](./COLORING_PRODUCT_BAR.md) ·
[`ROADMAP.md`](./ROADMAP.md) · [`CONCEPT.md`](./CONCEPT.md).
