# Rebook — Next Session Prompt

1. **Waves 1–2 done (RB-1…RB-14, bands 233–244)**: portable web service with
   `/` shelf, `/studio`, `/cover`, `/products` (+APIs incl. SSE `/api/ai`),
   strict EPUB, shelf/print packages + KDP gates.
2. **Wave 3 = print PDF — owner must pick crates first** (see
   [`PRINT_PDF_RESEARCH.md`](./PRINT_PDF_RESEARCH.md)): RB-15 genpdf interior
   (KDP RGB) → RB-16 svg2pdf/printpdf wrap PDF → RB-17 Ingram PDF/X-CMYK.
   Backlog: live rescan of `products/` in `/products` UI already done; PNG
   rasterizer for Cover export; UI i18n.
3. Keep green: `cargo fmt -- --check` → `cargo clippy --all-targets` →
   `cargo test`; one commit in `S:/rust/rebook`, push origin at close.

Sources: [`HANDOFF_NEW_SESSION.md`](./HANDOFF_NEW_SESSION.md) ·
[`REBOOK_DEV_PLAN.md`](./REBOOK_DEV_PLAN.md) · [`PRODUCTION_GUIDE.md`](../PRODUCTION_GUIDE.md).
