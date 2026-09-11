# Rebook — Next Session Prompt

1. **Plan RB-1…RB-10 fully landed (bands 233–240)** — app: portable single web
   service (`view` → http://127.0.0.1:8090/, axum) with `/` shelf+reader,
   `/studio` drafts (md/mdc, autosave, promote→strict EPUB), `/cover`
   (KDP-wrap template generator + Cover Studio layers, SVG/PNG), `/check`
   KDP gate. CLI: `build-epub`, `shelf`, `cover-template`, `check-print`.
2. Next wave candidates: SSE streaming for `/api/ai`; /shelf UI polish +
   live rescan of `products/` without restart; PDF export of print interiors
   (needs a writer crate decision); PNG rasterizer for cover art export;
   slug de-duplication (two books can collide on one id).
3. Keep green: `cargo fmt -- --check` → `cargo clippy --all-targets` →
   `cargo test`; one commit in `S:/rust/rebook`, push origin at close.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) · [`PRODUCTION_GUIDE.md`](../PRODUCTION_GUIDE.md).
