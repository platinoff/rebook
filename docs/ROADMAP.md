# Rebook — roadmap (після RB-38)

Тікети живуть на **GSV board**, product `rebook` (`RB-*`). Цей файл — канон
черги. **RB-40…51 landed**. Next = RB-52 (Studio draft).

## Landed (RB-40…51)

| RB | Що | Навіщо |
|----|----|--------|
| **40** | `sniff_image_kind` — обкладинка за magic bytes, не за розширенням | PNG названий `.jpg` стає `cover.png` у ZIP; KDP бачить правду |
| **41** | ISBN у OPF (`urn:isbn:` + ONIX 15); `kdp.rs` більше не кличе AZW3/boko | preflight `isbn:present` збігається з реальним EPUB |
| **42** | GSV-chrome: `data-tip` + `#rbTip`, uk/en навігація на кожній сторінці | IDE-рамка, не вигадувати тултіпи |
| **43** | Write-then-translate: `fork_translation` + Studio ⇄ + ISBN у меті | рідна мова → окреме видання, ISBN per-edition |
| **44** | Studio page-view: наскрізна нумерація книги (half-title = 1, футер з 2) | `.pgn`/`pvNo` як `interior_pdf` ±1; ←/→ через розділи |
| **45** | Knuth–Liang hyphenation у `interior_pdf::wrap` (`hypher` uk+en) | перенос по складах з `-`, не різати слово по літерах |
| **46** | FOGRA39 ICC drop-in (`REBOOK_ICC_CMYK` / `icc/*.icc`) + CMYK JPEG на X-1a | `/DestOutputProfile`; DeviceCMYK DCTDecode; ECI файл не в репо |
| **47** | Print-art ≥300 DPI (`print_art_ok`); EN `opf:isbn` з `book.json` | eBook 1600×2560 ≠ 6×9 print; wrap не тягне 125 DPI JPEG |
| **48** | `/view3d` CSS pages + paperback spread (verso/recto) | eBook не лише front; розгортка як Studio V |
| **49** | AI `translate` + GSV box fullscreen (□ / Esc) | ⇄ fork, потім `/api/ai` якщо `REBOOK_AI_ENDPOINT`; бокси стенду/cover/studio/products |
| **50** | Coloring canon: Classic American Iron (`coloring.rs` + roster) | 8.5×11 B&W no-bleed; 24 тачки; plates 1/2/3; verso = марка·модель·рік |
| **51** | SVG plate kit (`coloring_svg.rs`, `coloring-plates`) | 8.5×11 pt, stroke 1.25, verso caption+mark, recto views; 104 p |

Документи: [`CONCEPT.md`](./CONCEPT.md) · [`VULKAN_RESEARCH.md`](./VULKAN_RESEARCH.md) · [`COLORING_CARS.md`](./COLORING_CARS.md).

## Наступна черга (логічний порядок для `agi`)

1. **RB-52** — Studio draft `uk` + SVG includes; ⇄ `en`.
2. **RB-53** — paperback wrap (color cover, barcode, spine text ok at 104 p).
3. **RB-54** — `/view3d` 8.5×11 spread proof.
4. **RB-55** — `print/check` + KDP checklist before publish.
5. **RB-56** — EPUBCheck-еквівалент v2 (зовнішні URL, script, ISBN per format).

Не брати Vulkan у цю чергу (див. research).

## Ratio / gate

Rust **95–100%**. Перевірка: `cargo fmt -- --check` → `cargo clippy --all-targets`
→ `cargo test`. UI = vanilla HTML/JS у `ui/*.html` (`include_str!`).
