# EPUB Viewer — план реалізації (за стандартизацією KDP)

**Мета:** локальний просмоторщик `build/rust_book.epub`, який показує книгу так, як її
«побачить» KDP (Kindle Direct Publishing), і перевіряє EPUB-файл на відповідність
вимогам KDP EPUB 3.2.

**Команда:** `rust_book view [--port 8090]` → відкривається `http://127.0.0.1:8090/`.

## Як працює

1. Програма читає `build/rust_book.epub` через `zip` (crate уже в проєкті, offline).
2. Розкриває в пам'яті: `META-INF/container.xml`, `OEBPS/content.opf`,
   `OEBPS/nav.xhtml`, `OEBPS/styles.css` та всі `OEBPS/chapter-XX.xhtml`.
3. Токіо `TcpListener` на `127.0.0.1:<port>` відповідає на HTTP-запити (без azum —
   мінімальний власний HTTP/1.1 відповідник; жодних нових залежностей).
4. Маршрути:
   - `GET /` — головна: назва, автор, зміст (TOC) + панель KDP-сумісності.
   - `GET /chapter/{id}` — одна глава, очищена для рендеру (KDP-допустимий XHTML).
   - `GET /check` — повний звіт KDP-перевірки (JSON + HTML-таблиця).
   - `GET /styles.css` — офовий CSS з EPUB.
   - `GET /health` — ознака роботи сервера.

## KDP-перевірка (`/check`)

Усі правила якісні, дзволені лише Rust-перевірки (без зовнішніх інструментів):

1. **`mimetype`** — перший запис, `Stored` (без стиск), точний вміст
   `application/epub+xml`. (вже є в `check_epub`).
2. **Обов'язкові записи** — `META-INF/container.xml`, `OEBPS/content.opf`,
   `OEBPS/nav.xhtml`, `OEBPS/styles.css` + шаблон `OEBPS/chapter-*.xhtml`.
3. **`content.opf`** — має `dc:title`, `dc:creator`, `dc:language`, `dc:identifier`
   (urn:uuid), `dc:date`, `meta dcterms:modified` (ISO), `<spine>`.
4. **Навігація** — `nav.xhtml` містить `epub:type="toc"` та `<a href="chapter-...">`.
5. **Не дозволено KDP:** жодного `<script>`, `<iframe>`, `<video>`, `<audio>`,
   `<form>`, CSS `@import` / `url(...)` зовнішніх, зовнішніх посилань `http(s)://`,
   JavaScript-обробників `on*`. Перевіряємо всі XHTML-сторінки й стилі.
6. **XML добре сформований** — кожен XHTML розбирається як рядок; ігноруємо неекрановані
   `&`, `<`, `>` у текстах (перевірка на «сирі» амперсанди поза `&amp;`-формами).
7. **Кількість глав** з бібліотеки `book.json` має дорівнювати числу записів
   `OEBPS/chapter-*.xhtml` та `<itemref>` у spine.

## Чому так

- **Rust 100%** — немає Python/JS-інфраструктури, усе в межах crates, що вже є.
- **Offline** — `cargo --offline`, жодних мережевих залежностей.
- **Локально** — лише `127.0.0.1`, нічого не публікується.
- **KDP-стандарт** — перевіряємо те, що відкине KDP, до завантаження файлу.

## Файли

- `src/viewer.rs` — нова логіка: `read_epub`, `serve`, `kdp_check`, `render_index/chapter`.
- `src/lib.rs` — `pub mod viewer;`.
- `src/main.rs` — команда `view`.

## Критерій готовності

`cargo fmt` + `clippy` чисто; `cargo test --offline` зелений; `cargo build --release`
збирається; `cargo run --release -- view` піднімає сервер, `/check` дає «PASS»/
розгорнутий звіт, глави рендеряться в браузері; EPUB переходить усі правила KDP.