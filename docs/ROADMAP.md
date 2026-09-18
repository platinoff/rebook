# Rebook — roadmap (після RB-38)

Тікети живуть на **GSV board**, product `rebook` (`RB-*`). Цей файл — канон
черги. **RB-40…46 landed**. Next = RB-47.

## Landed (RB-40…46)

| RB | Що | Навіщо |
|----|----|--------|
| **40** | `sniff_image_kind` — обкладинка за magic bytes, не за розширенням | PNG названий `.jpg` стає `cover.png` у ZIP; KDP бачить правду |
| **41** | ISBN у OPF (`urn:isbn:` + ONIX 15); `kdp.rs` більше не кличе AZW3/boko | preflight `isbn:present` збігається з реальним EPUB |
| **42** | GSV-chrome: `data-tip` + `#rbTip`, uk/en навігація на кожній сторінці | IDE-рамка, не вигадувати тултіпи |
| **43** | Write-then-translate: `fork_translation` + Studio ⇄ + ISBN у меті | рідна мова → окреме видання, ISBN per-edition |
| **44** | Studio page-view: наскрізна нумерація книги (half-title = 1, футер з 2) | `.pgn`/`pvNo` як `interior_pdf` ±1; ←/→ через розділи |
| **45** | Knuth–Liang hyphenation у `interior_pdf::wrap` (`hypher` uk+en) | перенос по складах з `-`, не різати слово по літерах |
| **46** | FOGRA39 ICC drop-in (`REBOOK_ICC_CMYK` / `icc/*.icc`) + CMYK JPEG на X-1a | `/DestOutputProfile`; DeviceCMYK DCTDecode; ECI файл не в репо |

Документи: [`CONCEPT.md`](./CONCEPT.md) · [`VULKAN_RESEARCH.md`](./VULKAN_RESEARCH.md).

## Наступна черга (логічний порядок для `agi`)

1. **RB-47** — eBook cover DPI ≥300 для print-art; EN OPF ISBN на живих книгах
   власника (відомі дірки з NEXT).
2. **RB-48** — віртуальний стенд: ebook «листання» (CSS pages), не лише front;
   paperback spread з recto/verso як у Studio V.
3. **RB-49** — AI-переклад чернетки (env-gated `/api/ai`, не хмара за замовчуванням).
4. **RB-50** — EPUBCheck-еквівалент v2 (зовнішні URL, script, ISBN per format).

Не брати Vulkan у цю чергу (див. research).

## Ratio / gate

Rust **95–100%**. Перевірка: `cargo fmt -- --check` → `cargo clippy --all-targets`
→ `cargo test`. UI = vanilla HTML/JS у `ui/*.html` (`include_str!`).
