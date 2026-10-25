# Coloring / KDP paperback — product bar (all titles)

Author book files live under `workspace/` (**gitignored**). The repo ships the
**tool** only. This bar applies to every new coloring book or similar KDP
paperback the owner asks for — not only Classic American Iron.

## Volume

| Rule | Value | Why |
|------|-------|-----|
| Minimum interior pages | **70** (even) | Below this the book feels thin next to shipped titles; spine text also needs **> 79** if initials go on the wrap |
| Preferred target | **~90** | Matches recent shipped bar (landmarks / pets); room for two poses or color+line without padding junk |
| Hard floor (KDP) | 24 (trim-dependent) | Printer minimum — **not** our product minimum |

Page math = even(`front_matter` + content plates + `back_matter`). Prefer more
subjects or more plates per subject over blank filler. KDP rejects excessive
blank pages.

## Quality (same bar every time)

- **Art:** clean, detailed line work (or color plates when the SKU is premium
  color). No muddy gray wash on contour pages; binarize / art-check before
  `coloring-kdp`. Contour stroke ≥ **0.75 pt** (aim ~1.25 pt).
- **Cover:** realistic or clearly product-grade wrap; English listing title on
  front; back description + empty barcode corner; spine initials only when
  pages **> 79**.
- **Copy:** breed/subject blurbs with correct place of origin (kid-clear,
  fact-check soft folklore); KDP keywords = **one word per box**; no brand
  names in keywords/description.
- **Preview:** local `/kdp` Print Previewer after rebuild (`include_str!` UI —
  restart the server). Order a proof before Publish when unsure.

What **changes** per title is **content presentation** (theme, roster,
line-only vs color+line, one vs two poses, front-matter tone) — **not** the
volume/quality floor above.

## Modes the tool already supports

| Mode | Roster flags | Typical page recipe |
|------|--------------|---------------------|
| Color plate + line | default ink/paper premium | 1 identity + 1 line (or 4 views) per subject |
| Line-only (kids B&W) | `line_only: true`, ink `black`, paper `white` | 1 or **2** contour pages per subject (poses) |
| One-plate place books | `plates: 1`, `year: 0`, place in `model` | TOC blurbs + index by name/place |

## Git

Never commit `workspace/`, built PDFs, or author art. Push **software + docs**
only.
