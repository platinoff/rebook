# Rebook — Development Plan (band 233, started 2026-09-11)

Product: **one portable web service** (single Rust binary, loopback `127.0.0.1:8090`)
with two worlds:

- **Studio** — draft area. Books-in-progress, AI-assisted writing, files in
  `.md` / `.mdc` / `.html` / `.svg` / images. Nothing leaves the machine.
- **Shelf** — product area. Finished books per target format: **eBook**
  (strict EPUB), **Paperback** and **Hardcover** (KDP print packages —
  zip with full-wrap cover, interior, barcode, manifest), Canva-like
  **Cover Studio** for cover artwork.

Tickets live on the GSV board (product `rebook`, `RB-1…RB-10`); this file is
the canon for formats and geometry.

## 1. Standardization matrix (verified 2026-09-11)

### 1.1 eBook (KDP)

| Item | Standard |
|------|----------|
| Upload | EPUB (any 3.x; fixed-layout only via KPF/Kindle Create), DOCX; manuscript ≤ **650 MB** |
| Cover | front only, **JPEG/TIFF**, ideal **1600×2560 px** (ratio ≥ **1.6:1** H:W), min 625×1000, max 10 000 px, **< 50 MB** |
| EPUB core | EPUB 3.3 = W3C REC (Jan 2026 edition), backward-compatible with 3.2; gate = **EPUBCheck 5.3.0**; rebook emits strict 3.2/3.3-compatible (mimetype first+stored, `container.xml`, `nav.xhtml`, OPF `version="3.0"` + `dcterms:modified`) |
| ISBN in OPF | `<dc:identifier id="pub-id">urn:isbn:978…</dc:identifier>` + `identifier-type` refinement |
| DRM | per-book toggle; **since 2026-01-20** DRM-free buyers can download EPUB/PDF — our EPUB is a distribution artifact, keep it pristine |
| Limit | 10 new titles / format / week |

### 1.2 Paperback (KDP print)

Spine width (`pages` rounded **up to even**):

| Paper | Formula (in) |
|-------|--------------|
| B&W white | `pages × 0.002252` |
| B&W cream | `pages × 0.0025` |
| B&W groundwood | `pages × 0.00235` |
| Premium color / std color white | `pages × 0.002347` |

Full-wrap cover file (one flattened PDF, ≥300 DPI, sRGB or CMYK accepted):

```
width  = 0.125 + back(=trimW) + spine + front(=trimW) + 0.125   (bleed 0.125" each outer side)
height = 0.125 + trimH + 0.125
px = inches × 300          (e.g. 6×9, 300p white: spine 0.6756 → 12.9256×9.25" → 3878×2775 px)
```

- Trim sizes: 5×8 … 8.5×11 (17 standard; B&W white/groundwood 24–828 pages,
  cream 24–776, premium color 24–828 (590 on A4/8.5″), std color 72–600;
  custom W 4–8.5″, H 6–11.69″).
- Inside (gutter): 24–150p **0.375″**, 151–300p **0.5″**, 301–500p **0.625″**,
  501–700p **0.75″**, 701–828p **0.875″**; outside ≥ 0.25″ (0.375″ with bleed).
- Spine **text** only when pages > **79**, ≥ 0.0625″ from spine edges.
- Barcode zone: KDP stamps **2″×1.2″** bottom-right unless supplied
  (min 1.4″×0.8″, 100 %K on white, ≥0.25″ from spine/trim).
- Interior: single pages (no spreads), min font 7 pt, line ≥ 0.75 pt,
  images ≥300 DPI, bleed page = trim +0.125″ W × +0.25″ H.

### 1.3 Hardcover (KDP 2026 — GA, case laminate only)

- **No dust jackets / cloth** on KDP: art printed on board (gloss or matte).
- Trims: 5.5×8.5, 6×9, 6.14×9.21, 7×10, 8.25×11; pages **75–550** (even);
  B&W white/cream, premium color white; separate ISBN per edition.
- Cover file: back+spine+front + **0.51″ wrap** each edge; **0.4″ hinge**
  dead zone each side of spine; text safe ≥ **0.635″** from book edge;
  barcode ≥ 0.76″ from bottom; ≤ 40 MB recommended, 300 DPI.
- Spine constant **not published** — approximation `pages×0.002347 + 0.06″`
  (board allowance) **[UNVERIFIED — always offer the KDP-generated template]**.
- Alternatives (later export targets): **IngramSpark** case laminate
  (`board = trim−0.185″ W / +0.25″ H`, wrap 0.625″, stepped spine,
  PDF/X + CMYK, ≤240 % ink) + dust jacket (flaps **3.25″**, folds 0.25″);
  **Lulu** casewrap/linen (PB spine `pages/444 + 0.06″`, HC stepped table);
  **Blurb** calculator-only.

### 1.4 Print barcode

ISBN → **EAN-13** (978/979 prefix, ISBN-10 → 978 + recompute check digit,
mod-10 weights 1/3). Place 100 %K on white in the reserved zone.

## 2. Architecture

```
rust_book (single portable exe)
 ├─ standards.rs   # RB-2: trims, papers, spine/cover/gutter math, EAN-13 (pure, no io)
 ├─ barcode.rs     # RB-4: EAN-13 SVG
 ├─ cover.rs       # RB-3/RB-9: wrap templates + cover layer model (cover.json)
 ├─ studio/        # RB-6/RB-7: drafts CRUD (workspace/drafts/**), AI adapter (env-gated)
 ├─ shelf/         # RB-8: build targets -> products/{ebook,epub.zip, paperback/, hardcover/}
 ├─ epub.rs        # existing strict EPUB writer (keep)
 ├─ web.rs         # RB-5: axum app: /, /studio, /cover, /check + /api/* (embedded UI)
 └─ viewer.rs      # folds into web.rs (routes stay compatible)
```

- **Portable release**: one exe + embedded UI assets, no node/npm, no DLLs
  beyond the GNU runtime we already ship; `target/live/` copy pattern per kit.
- UI allowed formats: `.rs .md .mdc .json .js .wasm .svg .html` (kit rule);
  thin vanilla JS glue, no framework.
- AI: OpenAI-compatible endpoint via env only (`REBOOK_AI_ENDPOINT/MODEL`);
  off ⇒ UI hides assist. No vendor lock-in, no secrets in repo.
- Data layout (all gitignored): `workspace/drafts/<slug>/` (authoring),
  `products/<slug>/` (finished: `ebook.epub`, `paperback/*.{pdf,svg}`,
  `hardcover/*`, `manifest.json`, `*.zip` upload bundles).

## 3. Bands (GSV board tickets)

| RB | Ticket | Scope |
|----|--------|-------|
| 1 | `t-…601655093100` | research + this plan ✅ |
| 2 | `t-…611123412400` | `standards.rs` + tests ✅ next |
| 3 | `t-…621690388600` | `cover-template` CLI (SVG wrap) |
| 4 | `t-…632451724500` | EAN-13 barcode SVG generator |
| 5 | `t-…642571635600` | axum web shell, embedded UI, /studio /cover /shelf |
| 6 | `t-…650602281400` | drafts workspace + autosave + promote-to-build |
| 7 | `t-…659648547100` | AI-assist adapter (env-gated) |
| 8 | `t-…668519925600` | Shelf product packages + KDP checklist |
| 9 | `t-…677334791500` | Cover Studio (layers, presets, export) |
| 10 | `t-…685135350300` | KDP gate v2 (print validation in /check) |

Order 1→10 is dependency-bound (3,4 need 2; 5 needs nothing but lands with 6;
9 needs 3+4+5; 10 needs 2+8). No mid-drain push; one commit per band.

## 4. Wave 2 (done 2026-09-11, bands 241–244)

| RB | Scope | ✅ |
|----|-------|----|
| 11 | live shelf rescan + slug de-dup (AppState.root snapshot) | `ef03639` |
| 12 | SSE streaming `/api/ai` (`stream_to`, studio typing UI) | `5975176` |
| 13 | `/products` shelf UI: badges + gate status + zip/epub downloads | `aece872` |
| 14 | PDF export research → [`PRINT_PDF_RESEARCH.md`](./PRINT_PDF_RESEARCH.md) | `b167b08` |

## 5. Wave 3 (queued — owner crate gate first)

RB-15 interior PDF (KDP RGB, genpdf) → RB-16 wrap-cover PDF (svg2pdf/printpdf)
→ RB-17 IngramSpark PDF/X-CMYK. Crate pick required before implementation
(`printpdf`/`genpdf`/`pdf-writer` — see research note); tickets on GSV board
product `rebook`.

**RB-15 landed** `ce50d4c` (band 245, genpdf 0.2): custom trim size via
`set_paper_size(Size)` (mm), margins from the gutter table, PageBreak per
chapter, `REBOOK_FONT_TTF*` overrides → system Times/Georgia/Arial. Known
gaps rolled into RB-16: page numbers (recto), font subsetting (11.8 MB
bloat), justify alignment (genpdf has Left/Center/Right only).

**RB-16 API intel (probed 2026-09-12):** genpdf 0.2 ships printpdf ^0.3.4
(OLD API); crates.io printpdf 0.12.8 is the NEW 2nd-iteration API (different
types — do not mix). `genpdf::render` Area exposes `print_str`, `draw_line`,
`add_image` — NO filled-rect/rotation → cover-wrap artwork should go through
**svg2pdf + classic printpdf 0.3.4 (matching genpdf's dependency)** or a
custom `Element` painting rects as dense `draw_line` fills; decision in-band.

**RB-16 BLOCKED-pending-choice (band 246):** Route **A** = printpdf 0.12.8
+ `svg2pdf` feature feeding our composed cover SVG (risk: font resolution for
Georgia/Cyrillic + data-URI raster). Route **B** = own minimal PDF writer
(~300 LOC: rects `re f`, BT/Td text, barcode as path fills, inline images)
— full control, vector-only first. Gate either way: visual diff vs the
KDP-generated template before close.

**RB-16 RESOLVED → route B (owner pick).** Landed band 247 (rebook `1d09895`,
ticket RB-16v2 done): `src/pdfwriter.rs` + `src/coverpdf.rs`, `cover-template
--pdf`. Band 248 **RB-16b** done (rebook `687c76a`): `src/ttf.rs` TrueType
parser + Type0/Identity-H embed (Cyrillic ✓, spine 90° ✓, vector EAN-13 ✓,
Arial auto-picked around the stubbed times.ttf). Raster `front_image` + font
subsetting → RB-16c. **RB-17** (Ingram PDF/X-CMYK) now builds on
pdfwriter (OutputIntent + CMYK `k` ops) — after RB-16c.

**Wave-3 status (2026-09-12):** RB-15 ✅ `ce50d4c` · RB-16v2 ✅ `1d09895` ·
RB-16b ✅ `687c76a` · **RB-17 ✅ `4e82549`** — PDF/X-1a CMYK wrapper done
(OutputIntent/XMP/TrimBox/ink-TAC, `--pdf --cmyk`; strict FOGRA39 ICC via
`REBOOK_ICC_CMYK`, not shipped). Remaining polish: RB-16c raster+subset,
page-numbers in interior, live rescan of /products.
