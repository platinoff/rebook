# Rebook — roadmap (після RB-38)

Тікети живуть на **GSV board**, product `rebook` (`RB-*`). Цей файл — канон
черги. **RB-40…58 landed**. Portable binary = `init` + `--dir` / `REBOOK_HOME`.
Coloring book = `--book DIR` / `REBOOK_BOOK` (локальна тека: `roster.json`,
`listing.json`, `art/`, вихід у `DIR/build/kdp`).

## Landed (RB-40…52)

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
| **52** | Studio draft uk + SVG includes; ⇄ en (`coloring-draft`) | `classic-american-iron` + `-en`; verso МАРКА/MAKE |

Документи: [`CONCEPT.md`](./CONCEPT.md) · [`VULKAN_RESEARCH.md`](./VULKAN_RESEARCH.md) · [`COLORING_CARS.md`](./COLORING_CARS.md).

## Наступна черга (логічний порядок для `agi`)

Книжкові плани живуть **локально** у `workspace/plans/<book>/`, а теки для
`--book` у `workspace/books/<book>/` (усе gitignored). Тут і в тікетах — тільки ПЗ.

- ~~**RB-56**~~ landed — `--book DIR`: roster/art/listing/output з локальної теки; KDP-текст (опис, keywords, категорії, присвята) пішов з `src/coloring_kdp.rs` у `listing.json`.
- ~~**RB-57**~~ landed — `kdp_lint`: марки/моделі з ростера (з множиною), `listing.json` `deny`/`allow`, «free»/bestseller/#1, ≤7 keywords ≤50 символів. `coloring-kdp` падає, `/kdp` показує, `coloring-lint` — швидка перевірка.
- ~~**RB-58**~~ landed — `BodyKind` (`car`/`pickup`/`long-hood`/`cabover`, «heavy» = помилка) у `kdp_ok`; `frame_aspect` + `subject_box` на ракурс; без мастера нелегковий кузов отримує пунктирну рамку «art pending» замість легковика; `coloring-brief` пише `art-brief.json/.md` (2400×3200, рамка в px, custom build, `rule` з ростера) у `DIR/build/brief`; Studio-чернетка: id з `roster.id`, присвята/іменник з `listing.json` (`dedication_uk`, `noun_plural_uk`).
1. **RB-59…61** — книга №2 (вантажівки): арт-гейт 120/120 і KDP-пакет зібрані локально. Публікацію на KDP робить власник. У git лише ПЗ.
2. **RB-62** — книга №3, острівні ендеміки. Ресерч і ростер лише в `workspace/` (gitignored). Ліміт interior **130** сторінок. Спочатку зміст і тексти (назва, де живе, що їсть), разом із текстом обкладинки. Потім по одній тварині: кольорова плита, одразу її розмальовка. Не пакетом. Обкладинка: попереду тварина з листком, англійська назва; зад — опис і порожнє місце під штрихкод. У git і в тікети текст книги не класти.

Пізніше: `/view3d` 8.5×11 spread proof; EPUBCheck-еквівалент v2.

Не брати Vulkan у цю чергу (див. research).

## Ratio / gate

Rust **95–100%**. Перевірка: `cargo fmt -- --check` → `cargo clippy --all-targets`
→ `cargo test`. UI = vanilla HTML/JS у `ui/*.html` (`include_str!`).
