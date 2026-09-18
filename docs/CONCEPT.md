# Rebook — концепція продукту

`rebook` — один портативний Rust-сервіс (`127.0.0.1:8090`): пишеш книгу,
бачиш її як читач і як друкарня, вантажиш на **Amazon KDP** без локального
AZW3/KFX.

Інтерфейс — **uk ⇄ en** (чіп у навігації, `localStorage rb.lang`). Автор
пише **спочатку рідною мовою**, потім форкає видання під переклад.

Канон UI-підказок скопійовано з GSV Galaxy (`title` + `data-tip` + `#rbTip`
на pointerover) — не вигадуємо другий тултіп-рушій.

## Три типи книг (окремі KDP-продукти)

Кожна мова × кожен формат = **окремий KDP title** (окремий ISBN для друку).

| Тип | Що вантажимо | Віртуальний перегляд | Геометрія |
|-----|----------------|----------------------|-----------|
| **eBook** | EPUB 3.2 (`mimetype` STORED first, OPF + `nav.xhtml`, cover JPEG/PNG ≥1000 px, ISBN як `urn:isbn:` якщо є) | Полиця `/` + глава + `/view3d` mode `ebook` (обкладинка, далі CSS-листання) | front + pages, ratio ≥ 1.6:1, ideal 1600×2560 |
| **Paperback** | zip: `interior.pdf` + `cover-wrap.pdf` + barcode + `manifest.json` | `/view3d` mode `pb` spread (verso/recto) + Studio page-view (V) | bleed 0.125″, spine `pages×paper`, gutter table |
| **Hardcover** | zip: interior + case-laminate wrap (KDP 2026: **без** cloth/dust на KDP) | `/view3d` mode `hc` (wrap або spread); Ingram jacket — mode `dj` | wrap 0.51″, hinge 0.4″, pages 75–550 even |

Dust jacket (`dj`) лишається **Ingram**-ціллю, не KDP hardcover.

## Пиши рідною → переклади

1. Studio: нова чернетка, `language=uk` (або навпаки).
2. Текст, обкладинка, формати (ebook / pb / hc) — у **джерельній** мові.
3. Кнопка **⇄ Перекласти видання** → `POST /api/drafts/{id}/translate/{en|uk}`:
   копіює markdown як є, ставить `translation_of`, `source_language`,
   **скидає ISBN** (друк — окремий код на видання).
4. Перекладаєш у новій чернетці. AI-assist (`REBOOK_AI_*`) — опційно, env-gated.
5. Build products окремо для кожної мови.

`book.json` може нести `isbn`; EPUB OPF пише `urn:isbn:` + ONIX list 15.

## Віртуальний перегляд (як IDE, не як Canva-реклама)

Кожна сторінка сервісу має **ту саму системну рамку**:

- sticky nav (Полиця / Studio / Обкладинки / 3D / Продукти)
- hover-підказки на вкладках (GSV `data-tip`)
- uk/en чіп, спільний `rb.lang`
- Studio: 3-pane IDE (rail глав, редактор, preview + page-view як у друку)

3D-стенд (`/view3d`) показує **реальну KDP-геометрію** (trim, spine, bleed,
hinge, flaps) з EPUB-обкладинки полиці — ebook front + CSS pages,
paperback/hardcover open spread (verso/recto), Ingram jacket.

GPU: CSS 3D зараз. Vulkan у цей веб-сервіс **не** кладемо — див.
[`VULKAN_RESEARCH.md`](./VULKAN_RESEARCH.md).
