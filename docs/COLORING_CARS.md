# Classic American Iron — coloring paperback (KDP research)

Working title (uk, write-first): **Класичні американські тачки**.
KDP listing (en fork): **Classic American Iron**.

This is a **paperback coloring book**, not a prose EPUB. eBook is optional later;
the sellable object is a **B&W interior + color wrap** on Amazon KDP.

Canon files: [`samples/coloring-cars.json`](../samples/coloring-cars.json),
[`samples/coloring-mark.svg`](../samples/coloring-mark.svg), [`src/coloring.rs`](../src/coloring.rs).

## How others actually print these

Checked against KDP paperback help (2026) and competitor coloring practice
(muscle-car adult books, single-sided interiors, 8.5×11 workbooks):

| Choice | What we do | Why |
|--------|------------|-----|
| Trim | **8.5×11** (already in `PAPERBACK_TRIMS`) | Default adult coloring size; 6×9 feels like a novel, not a plate |
| Paper / ink | **white + black** | Cream hides pencil; color interior is the wrong SKU |
| Bleed | **No** | Art stays ≥0.25″ from trim; bleed only if art hits the cut |
| Gutter | KDP table (`gutter_in`, 0.375″ at 24–150 p) | Binding eats the inside; plates must not dive into the spine |
| Line weight | **≥0.75 pt** (KDP graphic minimum); aim 1–1.5 pt | Hairlines vanish on KDP toner |
| “Single-sided” | Art on **recto**; verso is **caption**, not empty | KDP rejects *excessive blank pages*; markers still hit a non-art back |
| Page count | **even**, ≥24 | Printers sheet signatures; our roster → **104** pages |
| Spine text | allowed (**104 > 79**) | KDP `SPINE_TEXT_MIN_PAGES`; short title on the wrap spine |
| Raster | ≥300 DPI if any bitmap; **SVG plates** stay vector | `print_art_ok` already gates photos; line art should not be JPEGs |

Competitors with 24–30 cars often print **one view per car**. That flattens a
’59 Cadillac and a ’34 coupe to the same value. We vary plates by **shape
complexity** (see roster `tier`).

KDP community note: if every even page is *truly blank*, buyers and QA both
complain. Verso copy (make · model · year · custom mark) is the product, not
filler.

## Pages per car (not uniform)

Each **plate** = 2 PDF pages (verso caption + recto SVG).

| Tier | Plates | Views | Who |
|------|--------|-------|-----|
| **hero** | 3 | ¾ front, rear/signature, badge or cockpit | fins, split-window, cobra pipes, hidden lamps, shaker, Boss scoop |
| **signature** | 2 | profile + front | Deuce, Bel Air, GTO ’64, Mustang GT, Camaro, Chevelle, Challenger, Trans Am, ’49 Mercury |
| **simple** | 1 | one clean ¾ or side | 3-window, Skylark, Impala ’58, GT40, 442, Javelin, GTO ’72, Riviera |

Roster: **6 hero + 10 signature + 8 simple = 24 cars, 46 plates**.
Interior: 9 front (ends on a recto so the first caption is verso) + 92 plate pages
+ 2 back = 103 → **104** even. White 8.5×11 allows up to 590 pages.

Front matter (9): half-title, title, copyright / AI-disclosure slot, “this book
belongs to”, how-to, roster TOC (spread), section opener «Тачки».
Back (2): index by year + make. Trailing even pad is blank.

## Caption + custom mark

Every verso prints, in the source language then the EN fork:

`МАРКА · МОДЕЛЬ · РІК` / `MAKE · MODEL · YEAR`

plus the garage crest [`coloring-mark.svg`](../samples/coloring-mark.svg)
(stroke only, colorable). No gray fills. No photos.

## Pipeline (bands)

1. **RB-50** (this file + `coloring.rs`) — research + roster math.
2. **RB-51** (`coloring_svg.rs` + `cargo run -- coloring-plates`) — SVG kit.
3. **RB-52** (`coloring_draft.rs` + `cargo run -- coloring-draft`) — Studio `uk`
   + SVG includes; ⇄ `en`.
4. **RB-53** — paperback wrap (color cover, barcode, spine text ok at 104 p).
5. **RB-54** — `/view3d` 8.5×11 spread proof.
6. **RB-55** — `print/check` + KDP checklist **before** publish.

EPUBCheck v2 waits as **RB-56**.

## Sources (summarized, not copied)

- Amazon KDP: Paperback Submission Guidelines — page count, margins/gutter,
  0.75 pt line minimum, even pages, no excessive blanks.
- Industry 2026 coloring practice: 8.5×11, no bleed unless art hits the trim,
  single-sided *art* with a used verso, 50–70 designs typical; we ship 46
  plates / 24 cars so hero cars get extra views instead of padding clones.
