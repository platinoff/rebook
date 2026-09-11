# RB-14 — pure-Rust print PDF export: research note (2026-09-11)

Goal: produce KDP/Ingram-ready **interior PDF** (single pages) and **flattened
wrap cover PDF** (300 DPI) without leaving Rust / adding C toolchains.

## Candidate crates (verified 2026)

| Crate | Role | Notes |
|-------|------|-------|
| `printpdf` (API 2nd iteration) | write PDF: pages, fonts, images, SVG (via `svg2pdf`), shading; conformance module (`PDF/X-3:2002`) | the workhorse; `image` feature for encode/decode; HTML layout is experimental (azul) — don't rely on it |
| `genpdf` | high-level document tree over printpdf + rusttype: page breaking, elements | fastest path to an interior book PDF; known bloat: embeds every glyph of every loaded font (~100–200 KiB/font) → subset or post-process; no CMYK story |
| `pdf-writer` (typst) | low-level typed PDF object writer; color model incl. **CMYK, ICC profiles, Separation/DeviceN** | no layout engine — good for emitting our own flattened cover/OutputIntent objects; typst itself has open PDF/X issue (#6012) — CMYK-only post-processing already passes IngramSpark preflight per that thread |

## What each store actually demands

- **KDP**: plain PDF (interior single pages, no spreads, fonts embedded, images
  ≥300 DPI; **RGB accepted** — KDP converts), cover = flattened one-piece PDF.
  → RGB path is enough; no PDF/X needed.
- **IngramSpark**: PDF/X‑1a:2001 or X‑3:2002, **all-CMYK**, ≤240% ink, 300 ppi.
  → needs ICC/CMYK conversion (color engine) + OutputIntent — heavier, separate band.

## Recommended wave-3 sequence (owner picks bands)

1. **RB-15 interior PDF (KDP)**: `genpdf` document tree from our chapter XHTML
   render pass; trim+gutter/bleed page sizes from `standards.rs` (gutter table,
   bleed page = trim+0.125×0.25), 7pt+ font check, page numbers (odd=recto).
   Acceptance: KDP-preview PDF + `check-print`-style page-count parity.
2. **RB-16 cover wrap PDF (KDP)**: our `cover.json`/template → `svg2pdf` +
   `printpdf`/`pdf-writer` → one flattened RGB PDF at exact wrap inches, 300 DPI,
   no crop marks; barcode vector (100%K on white) embedded via our `barcode.rs`.
3. **RB-17 Ingram PDF/X**: CMYK + OutputIntent (ICC soft-proof convert), ≤240%
   ink; evaluate `pdf-writer` color layer vs an ICC conversion crate; validate
   with an Ingram template pair. Gate: preflight pass before publishing.

Keep ratios: PDF writer stays Rust-only (95–100%); no external binaries.
