//! The one portable web service: axum router over the viewer plus the
//! product areas — `/` shelf & reader (viewer routes), `/studio`, `/cover`,
//! and `/api/*` (books JSON, live cover-template SVG). Loopback only.
//!
//! UI pages are embedded at compile time (`include_str!`), so the release is
//! a single self-contained executable — no sidecar assets, no node.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path as AxPath, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router, serve};
use tokio::net::TcpListener;

use crate::cover::{Mode, template, template_svg_with_isbn};
use crate::standards::{HARDCOVER_TRIMS, PAPERBACK_TRIMS, Paper, find_trim};
use crate::viewer::{DEFAULT_ADDR, LoadedBook, route, slug_is_safe};

/// Shared server state: initial shelf snapshot + scan root for live rescan.
pub struct AppState {
    /// Books discovered at startup (fallback when the root is unreadable).
    pub books: Vec<LoadedBook>,
    /// Directory walked per request (live shelf).
    pub root: std::path::PathBuf,
    /// `workspace/drafts` directory (Studio area).
    pub drafts_root: std::path::PathBuf,
    /// `products` directory (Shelf product area).
    pub products_root: std::path::PathBuf,
}

impl AppState {
    /// Fresh shelf snapshot: rescan the root, fall back to startup books.
    pub fn snapshot(&self) -> Vec<LoadedBook> {
        crate::viewer::discover_books(&self.root).unwrap_or_else(|_| self.books.clone())
    }

    /// RB-39: configured products dir + a sibling `products/` for every shelf
    /// book (`.\\en\\build\\x.epub` → `en/products`) — the live server must see
    /// per-book build trees regardless of cwd, not just a relative root.
    pub fn product_roots(&self) -> Vec<std::path::PathBuf> {
        let mut roots = vec![self.products_root.clone()];
        for b in self.snapshot() {
            let p = std::path::Path::new(&b.path);
            if let Some(book_dir) = p.parent().and_then(|x| x.parent()) {
                let cand = book_dir.join("products");
                if !roots.contains(&cand) {
                    roots.push(cand);
                }
            }
        }
        roots
    }

    /// Resolve `<root>/<slug>/<file>` against every known products root
    /// (ebook files live one level deeper in `<slug>/ebook/`).
    pub fn find_product_file(&self, slug: &str, file: &str) -> Option<std::path::PathBuf> {
        self.product_roots()
            .into_iter()
            .flat_map(|r| {
                [
                    r.join(slug).join(file),
                    r.join(slug).join("ebook").join(file),
                ]
            })
            .find(|p| p.is_file())
    }

    /// Resolve a print package dir (`<root>/<slug>/<format>`) across roots.
    pub fn find_package_dir(&self, slug: &str, format: &str) -> Option<std::path::PathBuf> {
        self.product_roots()
            .into_iter()
            .map(|r| r.join(slug).join(format))
            .find(|p| p.join("manifest.json").is_file())
    }
}

const STUDIO_HTML: &str = include_str!("../ui/studio.html");
const COVER_HTML: &str = include_str!("../ui/cover.html");
const PRODUCTS_HTML: &str = include_str!("../ui/products.html");
const VIEW3D_HTML: &str = include_str!("../ui/view3d.html");

/// Build the router over a frozen set of discovered books.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/studio", get(studio_page))
        .route("/cover", get(cover_page))
        .route("/products", get(products_page))
        .route("/view3d", get(view3d_page))
        .route("/api/products", get(api_products))
        .route(
            "/api/products/download/{slug}/{file}",
            get(api_product_download),
        )
        .route("/api/books", get(api_books))
        .route("/api/books/{id}/preflight", get(api_book_preflight))
        .route("/api/books/{id}/interior", get(api_book_interior))
        .route("/api/cover-template", get(api_cover_template))
        .route("/api/drafts", get(api_drafts_list).post(api_drafts_create))
        .route(
            "/api/drafts/{id}",
            get(api_draft_get).delete(api_draft_delete),
        )
        .route(
            "/api/drafts/{id}/chapter/{num}",
            put(api_draft_put_chapter).delete(api_draft_delete_chapter),
        )
        .route("/api/drafts/{id}/reorder", put(api_draft_reorder))
        .route("/api/drafts/{id}/meta", put(api_draft_meta))
        .route("/api/drafts/{id}/cover-img", post(api_draft_cover_img))
        .route("/api/drafts/{id}/build", post(api_draft_build))
        .route("/api/drafts/{id}/promote", post(api_draft_promote))
        .route(
            "/api/drafts/{id}/translate/{lang}",
            post(api_draft_translate),
        )
        .route(
            "/api/drafts/{id}/cover",
            get(api_draft_get_cover).put(api_draft_put_cover),
        )
        .route("/api/cover", post(api_cover))
        .route("/api/cover/dims", get(api_cover_dims))
        .route("/api/print/check/{slug}/{format}", get(api_print_check))
        .route("/api/ai", get(api_ai_status).post(api_ai))
        .fallback(viewer_fallback)
        .with_state(state)
}

/// Run the previewer/service until the process exits. Bind only to loopback.
pub async fn serve_web(books: Vec<LoadedBook>, addr: &str) -> Result<(), String> {
    let drafts_root = std::path::PathBuf::from("workspace/drafts");
    let state = Arc::new(AppState {
        books,
        root: std::path::PathBuf::from("."),
        drafts_root,
        products_root: std::path::PathBuf::from("products"),
    });
    println!("rebook web service: http://{addr}/");
    println!("  / — полиця/читалка · /studio — чернетки · /cover — обкладинки");
    for b in &state.books {
        println!("  /{id} — {title}", id = b.id, title = b.book.title);
    }
    println!("Зупинити — Ctrl+C.");
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Не вдалося слухати {addr}: {e}"))?;
    let app = router(state);
    serve(listener, app)
        .await
        .map_err(|e| format!("server: {e}"))
}

async fn health() -> &'static str {
    "ok"
}

async fn studio_page() -> Html<String> {
    Html(crate::viewer::inject_nav(STUDIO_HTML, "/studio"))
}

async fn cover_page() -> Html<String> {
    Html(crate::viewer::inject_nav(COVER_HTML, "/cover"))
}

async fn products_page() -> Html<String> {
    Html(crate::viewer::inject_nav(PRODUCTS_HTML, "/products"))
}

/// RB-28: dedicated 3D viewer with a book picker (discoverability fix).
async fn view3d_page() -> Html<String> {
    Html(crate::viewer::inject_nav(VIEW3D_HTML, "/view3d"))
}

/// RB-29 pre-KDP preflight for one shelf book: `?mode=hc|pb|ebook&trim=6x9&paper=white&pages=456`.
async fn api_book_preflight(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let Some(b) = st.snapshot().into_iter().find(|b| b.id == id) else {
        return (StatusCode::NOT_FOUND, "no such book").into_response();
    };
    let epub = match crate::viewer::Epub::from_path(&b.path) {
        Ok(e) => e,
        Err(e) => return draft_err(e),
    };
    let mode = match q.get("mode").map(String::as_str) {
        Some("pb") | Some("paperback") => "paperback",
        Some("ebook") => "ebook",
        _ => "hardcover",
    };
    let trim = q.get("trim").map(String::as_str).unwrap_or("6x9");
    let paper = match q.get("paper").map(String::as_str).unwrap_or("white") {
        "cream" => Paper::Cream,
        "ground" => Paper::Groundwood,
        "premium" => Paper::PremiumColor,
        _ => Paper::White,
    };
    let pages: u32 = q.get("pages").and_then(|s| s.parse().ok()).unwrap_or(300);
    let pf = crate::preflight::run(&epub, &b.book, mode, trim, paper, pages);
    match serde_json::to_string(&pf) {
        Ok(json) => text_response(StatusCode::OK, "application/json; charset=utf-8", json),
        Err(e) => draft_err(e.to_string()),
    }
}

/// RB-48: chapter HTML for CSS page-flip / recto-verso on `/view3d`.
async fn api_book_interior(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
) -> Response {
    let Some(b) = st.snapshot().into_iter().find(|b| b.id == id) else {
        return (StatusCode::NOT_FOUND, "no such book").into_response();
    };
    let chapters = crate::viewer::interior_chapters(&b.epub, &b.book);
    let body = serde_json::json!({
        "id": b.id,
        "title": b.book.title,
        "author": b.book.author,
        "language": b.book.language,
        "chapters": chapters,
    });
    match serde_json::to_string(&body) {
        Ok(json) => text_response(StatusCode::OK, "application/json; charset=utf-8", json),
        Err(e) => draft_err(e.to_string()),
    }
}

/// JSON list of built products (folders under `products/`).
async fn api_products(State(st): State<Arc<AppState>>) -> Response {
    let items = crate::shelf::list_products_in(&st.product_roots());
    let body = serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
    text_response(StatusCode::OK, "application/json; charset=utf-8", body)
}

/// Serve one built file from a product folder (`<slug>.epub`, `<slug>-pb.zip`,
/// `<slug>-hc.zip`) — slug-fenced, exact file whitelist.
async fn api_product_download(
    State(st): State<Arc<AppState>>,
    AxPath((slug, file)): AxPath<(String, String)>,
) -> Response {
    let allowed = (slug_is_safe(&slug) && file == format!("{slug}.epub"))
        || (slug_is_safe(&slug)
            && (file == format!("{slug}-pb.zip") || file == format!("{slug}-hc.zip")));
    if !allowed {
        return draft_err("unknown product file".to_string());
    }
    let path = match st.find_product_file(&slug, &file) {
        Some(p) => p,
        None => return draft_err("product file not found".to_string()),
    };
    match std::fs::read(&path) {
        Ok(bytes) => {
            let mime = if file.ends_with(".epub") {
                "application/epub+zip"
            } else {
                "application/zip"
            };
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{file}\""),
                )
                .body(axum::body::Body::from(bytes))
                .unwrap_or_default()
        }
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn api_books(State(st): State<Arc<AppState>>) -> Response {
    let mut items = String::from("[");
    for (i, b) in st.snapshot().iter().enumerate() {
        if i > 0 {
            items.push(',');
        }
        items.push_str(&format!(
            "{{\"id\":\"{}\",\"title\":\"{}\",\"author\":\"{}\",\"language\":\"{}\",\"path\":\"{}\",\"chapters\":{}}}",
            json_esc(&b.id),
            json_esc(&b.book.title),
            json_esc(&b.book.author),
            json_esc(&b.book.language),
            json_esc(&b.path),
            b.book.chapters.len()
        ));
    }
    items.push(']');
    text_response(StatusCode::OK, "application/json; charset=utf-8", items)
}

/// Build a plain response with an explicit status/content-type.
fn text_response(code: StatusCode, mime: &str, body: String) -> Response {
    Response::builder()
        .status(code)
        .header(header::CONTENT_TYPE, mime)
        .body(axum::body::Body::from(body))
        .unwrap_or_default()
}

/// `GET /api/cover-template?trim=6x9&pages=300&paper=white&mode=pb[&isbn=…]`
/// → guides-only full-wrap SVG at 300 DPI.
async fn api_cover_template(Query(q): Query<HashMap<String, String>>) -> Response {
    match build_template_from_query(&q) {
        Ok(svg) => {
            let mut resp = text_response(StatusCode::OK, "image/svg+xml", svg);
            resp.headers_mut().insert(
                header::CONTENT_DISPOSITION,
                axum::http::HeaderValue::from_static("inline; filename=\"cover-template.svg\""),
            );
            resp
        }
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

fn build_template_from_query(q: &HashMap<String, String>) -> Result<String, String> {
    let mode = q.get("mode").map(String::as_str).unwrap_or("pb");
    let mode = Mode::parse(mode).ok_or_else(|| format!("unknown mode: {mode}"))?;
    let label = q.get("trim").map(String::as_str).unwrap_or("6x9");
    let table = if mode == Mode::CaseLaminate {
        HARDCOVER_TRIMS
    } else {
        PAPERBACK_TRIMS
    };
    let trim = find_trim(table, label)
        .ok_or_else(|| format!("unknown trim {label} for mode {}", mode.tag()))?;
    let pages: u32 = q
        .get("pages")
        .map(|s| s.parse())
        .transpose()
        .map_err(|_| "pages must be a number".to_string())?
        .unwrap_or(300);
    let paper = match q.get("paper").map(String::as_str).unwrap_or("white") {
        "white" => Paper::White,
        "cream" => Paper::Cream,
        "ground" => Paper::Groundwood,
        "premium" => Paper::PremiumColor,
        other => return Err(format!("unknown paper: {other}")),
    };
    let t = template(trim, pages, paper, mode)?;
    Ok(template_svg_with_isbn(
        &t,
        q.get("isbn").map(String::as_str),
    ))
}

async fn viewer_fallback(State(st): State<Arc<AppState>>, uri: axum::http::Uri) -> Response {
    let path = uri.path();
    let books = st.snapshot();
    if books.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            "Не знайдено жодного *.epub — запустіть build-epub або покладіть файл у каталог.",
        )
            .into_response();
    }
    let (status, body, mime) = route(path, &books);
    let code = status
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .map(StatusCode::from_u16)
        .and_then(Result::ok)
        .unwrap_or(StatusCode::OK);
    text_response(code, mime, body)
}

fn json_esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Body for `POST /api/drafts`.
#[derive(serde::Deserialize)]
struct DraftIn {
    title: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    language: String,
}

/// Body for `PUT /api/drafts/{id}/chapter/{num}`.
#[derive(serde::Deserialize)]
struct ChapterIn {
    #[serde(default)]
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    ext: String,
}

fn draft_err(e: String) -> Response {
    text_response(StatusCode::BAD_REQUEST, "text/plain; charset=utf-8", e)
}

async fn api_drafts_list(State(st): State<Arc<AppState>>) -> Response {
    match crate::drafts::list(&st.drafts_root) {
        Ok(items) => {
            let body = serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
            text_response(StatusCode::OK, "application/json; charset=utf-8", body)
        }
        Err(e) => draft_err(e),
    }
}

async fn api_drafts_create(State(st): State<Arc<AppState>>, Json(body): Json<DraftIn>) -> Response {
    match crate::drafts::create(&st.drafts_root, &body.title, &body.author, &body.language) {
        Ok(meta) => (
            StatusCode::CREATED,
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            meta.to_json(),
        )
            .into_response(),
        Err(e) => draft_err(e),
    }
}

async fn api_draft_get(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
) -> Response {
    let meta = match crate::drafts::load(&st.drafts_root, &id) {
        Ok(m) => m,
        Err(e) => return draft_err(e),
    };
    let mut parts: Vec<String> = Vec::new();
    for c in &meta.chapters {
        let content =
            crate::drafts::chapter_content(&st.drafts_root, &id, c.number).unwrap_or_default();
        parts.push(format!(
            "{{\"number\":{},\"title\":\"{}\",\"file\":\"{}\",\"content\":\"{}\"}}",
            c.number,
            json_esc(&c.title),
            json_esc(&c.file),
            json_esc(&content)
        ));
    }
    let body = {
        let dir = crate::drafts::draft_dir(&st.drafts_root, &id);
        let cover = ["cover.png", "cover.jpg"]
            .iter()
            .find(|f| dir.join(f).exists())
            .map(|f| format!("\"{f}\""))
            .unwrap_or_else(|| "null".to_string());
        format!(
            "{{\"meta\":{},\"chapters\":[{}],\"cover\":{}}}",
            meta.to_json(),
            parts.join(","),
            cover
        )
    };
    text_response(StatusCode::OK, "application/json; charset=utf-8", body)
}

async fn api_draft_put_chapter(
    State(st): State<Arc<AppState>>,
    AxPath((id, num)): AxPath<(String, u32)>,
    Json(body): Json<ChapterIn>,
) -> Response {
    let ext = if body.ext.is_empty() {
        "md"
    } else {
        body.ext.as_str()
    };
    match crate::drafts::save_chapter(&st.drafts_root, &id, num, &body.title, &body.content, ext) {
        Ok(meta) => text_response(
            StatusCode::OK,
            "application/json; charset=utf-8",
            meta.to_json(),
        ),
        Err(e) => draft_err(e),
    }
}

async fn api_draft_promote(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
) -> Response {
    match crate::drafts::promote(&st.drafts_root, &id) {
        Ok(path) => text_response(
            StatusCode::OK,
            "text/plain; charset=utf-8",
            path.to_string_lossy().into_owned(),
        ),
        Err(e) => draft_err(e),
    }
}

async fn api_draft_translate(
    State(st): State<Arc<AppState>>,
    AxPath((id, lang)): AxPath<(String, String)>,
) -> Response {
    match crate::drafts::fork_translation(&st.drafts_root, &id, &lang) {
        Ok(meta) => text_response(
            StatusCode::OK,
            "application/json; charset=utf-8",
            meta.to_json(),
        ),
        Err(e) => draft_err(e),
    }
}

async fn api_draft_delete(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
) -> Response {
    match crate::drafts::delete(&st.drafts_root, &id) {
        Ok(()) => text_response(
            StatusCode::OK,
            "text/plain; charset=utf-8",
            "deleted".to_string(),
        ),
        Err(e) => draft_err(e),
    }
}

/// `GET /api/print/check/{slug}/{format}` → KDP print-gate v2 JSON items
/// for a built package under `products/` (slug/format strictly validated).
async fn api_print_check(
    State(st): State<Arc<AppState>>,
    AxPath((slug, format)): AxPath<(String, String)>,
) -> Response {
    if crate::viewer::slug_is_safe(&slug) && (format == "paperback" || format == "hardcover") {
        let dir = match st.find_package_dir(&slug, &format) {
            Some(d) => d,
            None => return draft_err(format!("no {} package for {slug}", format)),
        };
        match crate::shelf::verify_package(&dir) {
            Ok(items) => {
                let body = serde_json::to_string(&items).unwrap_or_else(|_| "[]".into());
                text_response(StatusCode::OK, "application/json; charset=utf-8", body)
            }
            Err(e) => draft_err(e),
        }
    } else {
        draft_err("bad slug or format".to_string())
    }
}

/// Body for `POST /api/ai`.
#[derive(serde::Deserialize)]
struct AiIn {
    mode: String,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    context: String,
    #[serde(default)]
    stream: bool,
}

async fn api_draft_delete_chapter(
    State(st): State<Arc<AppState>>,
    AxPath((id, num)): AxPath<(String, u32)>,
) -> Response {
    match crate::drafts::delete_chapter(&st.drafts_root, &id, num) {
        Ok(meta) => text_response(
            StatusCode::OK,
            "application/json; charset=utf-8",
            meta.to_json(),
        ),
        Err(e) => draft_err(e),
    }
}

async fn api_draft_reorder(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
    body: String,
) -> Response {
    let order: Vec<u32> = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return draft_err(format!("body must be [1,3,2…]: {e}")),
    };
    match crate::drafts::reorder_chapters(&st.drafts_root, &id, &order) {
        Ok(meta) => text_response(
            StatusCode::OK,
            "application/json; charset=utf-8",
            meta.to_json(),
        ),
        Err(e) => draft_err(e),
    }
}

/// RB-26: front-matter patch for a draft (all fields optional).
#[derive(serde::Deserialize)]
struct MetaIn {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    formats: Option<Vec<String>>,
    #[serde(default)]
    trim: Option<String>,
    #[serde(default)]
    pages: Option<u32>,
    #[serde(default)]
    isbn: Option<String>,
}

async fn api_draft_meta(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
    Json(b): Json<MetaIn>,
) -> Response {
    let patch = crate::drafts::MetaPatch {
        title: b.title.as_deref(),
        author: b.author.as_deref(),
        language: b.language.as_deref(),
        formats: b.formats.as_deref(),
        trim: b.trim.as_deref(),
        pages: b.pages,
        isbn: b.isbn.as_deref(),
    };
    match crate::drafts::save_meta(&st.drafts_root, &id, &patch) {
        Ok(meta) => text_response(
            StatusCode::OK,
            "application/json; charset=utf-8",
            meta.to_json(),
        ),
        Err(e) => draft_err(e),
    }
}

#[derive(serde::Deserialize)]
struct CoverImgIn {
    data: String,
}

async fn api_draft_cover_img(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
    Json(b): Json<CoverImgIn>,
) -> Response {
    match crate::drafts::save_cover_img(&st.drafts_root, &id, &b.data) {
        Ok(()) => text_response(
            StatusCode::OK,
            "text/plain; charset=utf-8",
            "saved".to_string(),
        ),
        Err(e) => draft_err(e),
    }
}

async fn api_draft_build(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
) -> Response {
    match crate::drafts::build_products(&st.drafts_root, &id) {
        Ok(files) => {
            let body = serde_json::to_string(&files).unwrap_or_else(|_| "[]".into());
            text_response(StatusCode::OK, "application/json; charset=utf-8", body)
        }
        Err(e) => draft_err(e),
    }
}

/// `POST /api/cover` with a CoverDoc JSON → composed artwork SVG.
async fn api_cover(Json(doc): Json<crate::coverdoc::CoverDoc>) -> Response {
    match doc.render_svg() {
        Ok(svg) => text_response(StatusCode::OK, "image/svg+xml", svg),
        Err(e) => draft_err(e),
    }
}

/// RB-27: geometry for the 3D mockup — spine/panel sizes in inches.
async fn api_cover_dims(Query(q): Query<HashMap<String, String>>) -> Response {
    let mode = q.get("mode").map(String::as_str).unwrap_or("hc");
    let m = Mode::parse(mode).unwrap_or(Mode::CaseLaminate);
    let table = if m == Mode::CaseLaminate {
        HARDCOVER_TRIMS
    } else {
        PAPERBACK_TRIMS
    };
    let label = q.get("trim").map(String::as_str).unwrap_or("6x9");
    let Some(t) = find_trim(table, label) else {
        return draft_err(format!("unknown trim {label} for {mode}"));
    };
    let pages: u32 = q.get("pages").and_then(|s| s.parse().ok()).unwrap_or(300);
    let paper = match q.get("paper").map(String::as_str).unwrap_or("white") {
        "cream" => Paper::Cream,
        "ground" => Paper::Groundwood,
        "premium" => Paper::PremiumColor,
        _ => Paper::White,
    };
    let (w, h) = match template(t, pages, paper, m) {
        Ok(tpl) => (tpl.size.w, tpl.size.h),
        Err(e) => return draft_err(e),
    };
    let spine = if m == Mode::CaseLaminate {
        crate::standards::hardcover_spine_approx(pages, paper)
    } else {
        crate::standards::spine_width(pages, paper)
    };
    // book box = trim-sized faces; wrap canvas reported too
    let body = format!(
        "{{\"trim\":\"{}\",\"trim_w_in\":{},\"trim_h_in\":{},\"spine_in\":{:.6},\"wrap_w_in\":{:.4},\"wrap_h_in\":{:.4},\"pages\":{}}}",
        t.label,
        t.w,
        t.h,
        spine,
        w,
        h,
        crate::standards::even_pages(pages)
    );
    text_response(StatusCode::OK, "application/json; charset=utf-8", body)
}

async fn api_draft_get_cover(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
) -> Response {
    match crate::drafts::load_cover(&st.drafts_root, &id) {
        Some(json) => text_response(StatusCode::OK, "application/json; charset=utf-8", json),
        None => (StatusCode::NOT_FOUND, "no cover.json for this draft").into_response(),
    }
}

async fn api_draft_put_cover(
    State(st): State<Arc<AppState>>,
    AxPath((id,)): AxPath<(String,)>,
    body: String,
) -> Response {
    match crate::drafts::save_cover(&st.drafts_root, &id, &body) {
        Ok(()) => text_response(
            StatusCode::OK,
            "text/plain; charset=utf-8",
            "saved".to_string(),
        ),
        Err(e) => draft_err(e),
    }
}

/// Probe whether the local AI adapter is on (never leaks the endpoint URL).
async fn api_ai_status() -> Response {
    let body = serde_json::json!({
        "enabled": crate::ai::enabled(),
        "modes": crate::ai::MODES,
    });
    text_response(
        StatusCode::OK,
        "application/json; charset=utf-8",
        body.to_string(),
    )
}

/// OpenAI-compatible assist; 503 when REBOOK_AI_ENDPOINT is unset (offline-safe).
async fn api_ai(Json(body): Json<AiIn>) -> Response {
    if !crate::ai::enabled() {
        return text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "text/plain; charset=utf-8",
            "AI disabled: set REBOOK_AI_ENDPOINT to a local OpenAI-compatible server (http://…)"
                .to_string(),
        );
    }
    if !crate::ai::MODES.contains(&body.mode.as_str()) {
        return draft_err(format!(
            "unknown mode {:?} (use /{}/)",
            body.mode,
            crate::ai::MODES.join("/")
        ));
    }
    let endpoint = std::env::var("REBOOK_AI_ENDPOINT").unwrap_or_default();
    let model = std::env::var("REBOOK_AI_MODEL").unwrap_or_else(|_| "local".to_string());
    let payload = crate::ai::chat_body(&body.mode, &body.prompt, &body.context, &model);
    if body.stream {
        return sse_assist(endpoint, payload);
    }
    match crate::ai::complete(&endpoint, &payload).await {
        Ok(text) => text_response(StatusCode::OK, "text/plain; charset=utf-8", text),
        Err(e) => text_response(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", e),
    }
}

/// RB-12: true incremental deltas as Server-Sent Events (`data:` = JSON
/// string per delta; `event: error` carries upstream failures).
fn sse_assist(endpoint: String, payload: serde_json::Value) -> Response {
    use axum::response::sse::{Event, Sse};
    use tokio_stream::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, String>>(64);
    tokio::spawn(async move {
        crate::ai::stream_to(&endpoint, &payload, &tx).await;
    });
    let stream = ReceiverStream::new(rx).map(|item| match item {
        Ok(delta) => Event::default().json_data(delta),
        Err(e) => Event::default().event("error").json_data(e),
    });
    Sse::new(stream).into_response()
}

/// Default bind helper for callers that want the canonical address.
pub fn default_addr() -> String {
    DEFAULT_ADDR.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::Epub;
    use crate::{Book, ChapterMeta};
    use tower::ServiceExt;

    fn fixture_state() -> Arc<AppState> {
        let entries = vec![
            ("mimetype".to_string(), b"application/epub+xml".to_vec()),
            (
                "OEBPS/chapter-01.xhtml".to_string(),
                b"<body><h1>Y</h1></body>".to_vec(),
            ),
        ];
        let epub = Epub::from_entries(entries);
        let book = Book {
            title: "Тест".to_string(),
            author: "А".to_string(),
            edition: 1,
            year: 2026,
            format: "EPUB 3.2".to_string(),
            language: "uk".to_string(),
            isbn: None,
            chapters: vec![ChapterMeta {
                number: 1,
                title: "Y".to_string(),
                file: "OEBPS/chapter-01.xhtml".to_string(),
            }],
        };
        Arc::new(AppState {
            books: vec![LoadedBook {
                id: "test".to_string(),
                path: "t.epub".to_string(),
                book,
                epub,
            }],
            root: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("web-shelf-missing"),
            drafts_root: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("web-drafts"),
            products_root: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("web-products-missing"),
        })
    }

    async fn get(app: Router, uri: &str) -> (StatusCode, String) {
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(uri)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 4_000_000)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn health_studio_cover_routes() {
        let app = router(fixture_state());
        let (s, b) = get(app, "/health").await;
        assert_eq!((s.as_u16(), b.as_str()), (200, "ok"));
        let (s, b) = get(router(fixture_state()), "/studio").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("id=\"studio-app\""));
        let (s, b) = get(router(fixture_state()), "/cover").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("id=\"cover-app\""));
        let (s, b) = get(router(fixture_state()), "/api/ai").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("\"enabled\""));
        assert!(b.contains("translate"));
        unsafe { std::env::remove_var("REBOOK_AI_ENDPOINT") };
        let (s, _) = json_call(
            router(fixture_state()),
            "POST",
            "/api/ai",
            r#"{"mode":"translate","prompt":"en","context":"x"}"#,
        )
        .await;
        assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn api_books_and_viewer_fallback() {
        let (s, b) = get(router(fixture_state()), "/api/books").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("\"id\":\"test\""));
        let (s, b) = get(router(fixture_state()), "/test/chapter/1").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("<h1>Y</h1>"));
        let (s, b) = get(router(fixture_state()), "/api/books/test/interior").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("\"id\":\"test\""));
        assert!(b.contains("<h1>Y</h1>"));
        assert!(b.contains("\"number\":1"));
        let (s, _) = get(router(fixture_state()), "/api/books/nope/interior").await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        let (s, _) = get(router(fixture_state()), "/nope").await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn cover_template_api() {
        let (s, b) = get(
            router(fixture_state()),
            "/api/cover-template?trim=6x9&pages=300&paper=white&mode=pb&isbn=978-3-16-148410-0",
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("viewBox=\"0 0 3878 2775\""));
        assert!(b.contains("id=\"barcode\""));
        let (s, b) = get(
            router(fixture_state()),
            "/api/cover-template?trim=9x9&pages=300",
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert!(b.contains("unknown trim"));
    }

    #[tokio::test]
    async fn cover_api_composes_artwork() {
        let (s, body) = json_call(
            router(fixture_state()),
            "POST",
            "/api/cover",
            r##"{"mode":"ebook","trim":"6x9","pages":300,"paper":"white","bg_front":"#223344","bg_back":"#223344","title":{"text":"Hi <&>","x":0,"y":0,"pt":40,"color":"#fff","spine":false},"author":{"text":"by A","x":0,"y":0,"pt":18,"color":"#eee","spine":false}}"##,
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(body.contains("viewBox=\"0 0 1600 2560\""));
        assert!(body.contains("Hi &lt;&amp;&gt;"));
    }

    #[tokio::test]
    async fn shelf_rescans_live() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("web-rescan");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let st = Arc::new(AppState {
            books: vec![],
            root: root.clone(),
            drafts_root: root.join("drafts"),
            products_root: root.join("products-none"),
        });
        let (s, body) = get(router(st.clone()), "/api/books").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(body, "[]");

        // build a book into the shelf while the router is alive — no restart.
        let book = Book {
            title: "Late Arrival".to_string(),
            author: "Z".to_string(),
            edition: 1,
            year: 2026,
            format: "EPUB 3.2".to_string(),
            language: "uk".to_string(),
            isbn: None,
            chapters: vec![ChapterMeta {
                number: 1,
                title: "One".to_string(),
                file: "c1.md".to_string(),
            }],
        };
        let chapters = vec![crate::Chapter {
            number: 1,
            title: "One".to_string(),
            content: "text".to_string(),
        }];
        let cfg = crate::epub::EpubConfig {
            title: book.title.clone(),
            author: book.author.clone(),
            output_path: root.join("late.epub").to_string_lossy().into_owned(),
            cover_image: None,
            language: "uk".to_string(),
            isbn: None,
        };
        crate::epub::generate_epub(&cfg, &book, &chapters).unwrap();
        let (s, body) = get(router(st.clone()), "/api/books").await;
        assert_eq!(s, StatusCode::OK);
        assert!(body.contains("late-arrival"));
        // fallback route serves the new book without restart too
        let (s, _) = get(router(st.clone()), "/late-arrival/").await;
        assert_eq!(s, StatusCode::OK);
    }

    #[test]
    fn slug_dedup_same_title() {
        // two epubs with identical titles → ids `same-book` and `same-book-2`.
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("dedup-shelf");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let book = Book {
            title: "Same Book".to_string(),
            author: "Q".to_string(),
            edition: 1,
            year: 2026,
            format: "EPUB 3.2".to_string(),
            language: "uk".to_string(),
            isbn: None,
            chapters: vec![ChapterMeta {
                number: 1,
                title: "One".to_string(),
                file: "c1.md".to_string(),
            }],
        };
        let ch = vec![crate::Chapter {
            number: 1,
            title: "One".to_string(),
            content: "x".to_string(),
        }];
        for name in ["a.epub", "b.epub"] {
            let cfg = crate::epub::EpubConfig {
                title: book.title.clone(),
                author: book.author.clone(),
                output_path: dir.join(name).to_string_lossy().into_owned(),
                cover_image: None,
                language: "uk".to_string(),
                isbn: None,
            };
            crate::epub::generate_epub(&cfg, &book, &ch).unwrap();
        }
        let books = crate::viewer::discover_books(&dir).unwrap();
        assert_eq!(books.len(), 2);
        assert_eq!(books[0].id, "same-book");
        assert_eq!(books[1].id, "same-book-2");
    }

    #[test]
    fn product_roots_add_per_book_products_dir() {
        let epub = crate::viewer::Epub::from_entries(vec![(
            "mimetype".to_string(),
            b"application/epub+xml".to_vec(),
        )]);
        let st = AppState {
            books: vec![LoadedBook {
                id: "x".into(),
                path: "en/build/web-prod.epub".into(),
                book: Book {
                    title: "X".into(),
                    author: "A".into(),
                    edition: 1,
                    year: 2026,
                    format: "EPUB 3.2".into(),
                    language: "uk".into(),
                    isbn: None,
                    chapters: vec![],
                },
                epub,
            }],
            root: std::path::PathBuf::from("target/no-such-shelf-root"),
            drafts_root: std::path::PathBuf::from("target/d"),
            products_root: std::path::PathBuf::from("products"),
        };
        let roots = st.product_roots();
        assert!(
            roots
                .iter()
                .any(|r| r == &std::path::PathBuf::from("en/products")),
            "en/build/x.epub must contribute en/products: {roots:?}"
        );
    }

    #[tokio::test]
    async fn products_api_and_download() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("web-products");
        let _ = std::fs::remove_dir_all(&root);
        let book = Book {
            title: "Web Prod".to_string(),
            author: "A".to_string(),
            edition: 1,
            year: 2026,
            format: "EPUB 3.2".to_string(),
            language: "uk".to_string(),
            isbn: None,
            chapters: vec![ChapterMeta {
                number: 1,
                title: "One".to_string(),
                file: "c1.md".to_string(),
            }],
        };
        let chapters = vec![crate::Chapter {
            number: 1,
            title: "One".to_string(),
            content: "abc ".repeat(400),
        }];
        let cfg = crate::shelf::ProductConfig {
            targets: vec!["ebook".to_string(), "paperback".to_string()],
            trim: "6x9".to_string(),
            pages: Some(200),
            paper: crate::shelf::PaperName::White,
            isbn: None,
        };
        crate::shelf::build_product(&root, &book, &chapters, &cfg).unwrap();
        // build_product nests under products/
        let st = Arc::new(AppState {
            books: vec![],
            root: root.clone(),
            drafts_root: root.join("d"),
            products_root: root.join("products"),
        });
        let (s, body) = get(router(st.clone()), "/api/products").await;
        assert_eq!(s, StatusCode::OK);
        assert!(body.contains("web-prod"));
        assert!(body.contains("paperback"));
        let (s, _) = get(
            router(st.clone()),
            "/api/products/download/web-prod/web-prod-pb.zip",
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        let (s, _) = get(
            router(st.clone()),
            "/api/products/download/web-prod/web-prod.epub",
        )
        .await;
        assert_eq!(s, StatusCode::OK, "ebook lives one level deeper");
        let (s, _) = get(
            router(st.clone()),
            "/api/products/download/web-prod/..%2Fsecret",
        )
        .await;
        assert_ne!(s, StatusCode::OK);
        let (s, _) = get(router(st.clone()), "/products").await;
        assert_eq!(s, StatusCode::OK);
    }

    #[tokio::test]
    async fn meta_cover_build_endpoints() {
        let st0 = fixture_state();
        let st = Arc::new(AppState {
            books: st0.books.clone(),
            root: st0.root.clone(),
            drafts_root: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("web-drafts-meta"),
            products_root: st0.products_root.clone(),
        });
        let _ = std::fs::remove_dir_all(&st.drafts_root);
        let (s, _) = json_call(
            router(st.clone()),
            "POST",
            "/api/drafts",
            r#"{"title":"Meta Test","author":"A","language":"uk"}"#,
        )
        .await;
        assert_eq!(s, StatusCode::CREATED);
        json_call(
            router(st.clone()),
            "PUT",
            "/api/drafts/meta-test/chapter/1",
            r#"{"title":"One","content":"word word word word word word word word word word"}"#,
        )
        .await;
        let (s, b) = json_call(
            router(st.clone()),
            "PUT",
            "/api/drafts/meta-test/meta",
            r#"{"author":"Artem","language":"en","formats":["ebook","hardcover"],"pages":80,"isbn":"978-3-16-148410-0"}"#,
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("\"language\":\"en\""));
        assert!(b.contains("hardcover"));
        let (s, _) = json_call(
            router(st.clone()),
            "PUT",
            "/api/drafts/meta-test/meta",
            r#"{"formats":["audiobook"]}"#,
        )
        .await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        let (s, _) = json_call(
            router(st.clone()),
            "POST",
            "/api/drafts/meta-test/cover-img",
            r#"{"data":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg=="}"#,
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(st.drafts_root.join("meta-test/cover.png").exists());
        let (s, b) = json_call(
            router(st.clone()),
            "POST",
            "/api/drafts/meta-test/build",
            "",
        )
        .await;
        assert_eq!(s, StatusCode::OK, "{b}");
        assert!(b.contains("ebook/meta-test.epub"));
        assert!(b.contains("meta-test-hc.zip"));
    }

    #[tokio::test]
    async fn cover_dims_api_for_mockup() {
        let (s, b) = get(
            router(fixture_state()),
            "/api/cover/dims?mode=hc&trim=6x9&pages=200&paper=white",
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        let v: serde_json::Value = serde_json::from_str(&b).unwrap();
        assert_eq!(v["trim"], "6x9");
        assert!((v["spine_in"].as_f64().unwrap() - (200.0 * 0.002252 + 0.06)).abs() < 1e-6);
        assert_eq!(v["trim_w_in"], 6.0);
        let (s, _) = get(router(fixture_state()), "/api/cover/dims?mode=hc&trim=5x8").await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        // RB-37: dust-jacket wrap width must include flaps + hinges beyond the panel.
        let (s, b) = get(
            router(fixture_state()),
            "/api/cover/dims?mode=dj&trim=6x9&pages=200&paper=white",
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        let v: serde_json::Value = serde_json::from_str(&b).unwrap();
        assert!(
            v["wrap_w_in"].as_f64().unwrap() > 6.0 + 6.0,
            "dj wrap should exceed two 6in panels: {b}"
        );
    }

    #[tokio::test]
    async fn nav_and_view3d_pages() {
        let st = fixture_state();
        let (s, b) = get(router(st.clone()), "/view3d").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("rb-nav"));
        assert!(b.contains("id=\"bookSel\""));
        assert!(b.contains("id=\"leaf\""));
        assert!(b.contains("id=\"leafSpread\""));
        assert!(b.contains("id=\"stage\""));
        assert!(b.contains("class=\"rb-box\""));
        assert!(b.contains("rbBindBoxFs"));
        let (s, b) = get(router(st.clone()), "/studio").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("rb-nav"));
        assert!(b.contains("/view3d"));
        let (s, b) = get(router(st.clone()), "/books").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("rb-nav"));
        assert!(b.contains("🧊 3D"));
    }

    #[test]
    fn build_from_query_defaults() {
        let q = HashMap::new();
        let svg = build_template_from_query(&q).unwrap();
        assert!(svg.contains("6x9 300p"));
    }

    async fn json_call(
        app: Router,
        method: &'static str,
        uri: &str,
        body: &str,
    ) -> (StatusCode, String) {
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(axum::body::Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 4_000_000)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn drafts_roundtrip_over_http() {
        let st = fixture_state();
        let st = Arc::new(AppState {
            books: st.books.clone(),
            root: st.root.clone(),
            drafts_root: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("web-drafts-rt"),
            products_root: st.products_root.clone(),
        });
        let _ = std::fs::remove_dir_all(&st.drafts_root);
        let (s, body) = json_call(
            router(st.clone()),
            "POST",
            "/api/drafts",
            r#"{"title":"API Draft","author":"A","language":"en"}"#,
        )
        .await;
        assert_eq!(s, StatusCode::CREATED);
        assert!(body.contains("\"id\":\"api-draft\""));
        let (s, body) = json_call(
            router(st.clone()),
            "PUT",
            "/api/drafts/api-draft/chapter/1",
            r#"{"title":"One","content":"Hello chapter text"}"#,
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(body.contains("ch01.md"));
        let (s, body) = get(router(st.clone()), "/api/drafts/api-draft").await;
        assert_eq!(s, StatusCode::OK);
        assert!(body.contains("Hello chapter text"));
        let (s, body) = json_call(
            router(st.clone()),
            "POST",
            "/api/drafts/api-draft/promote",
            "",
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(body.ends_with("api-draft.epub"));
        assert!(std::path::Path::new(body.trim_end()).exists());
    }

    #[test]
    fn cover_html_blocks_native_get_and_has_en_labels() {
        let h = include_str!("../ui/cover.html");
        assert!(h.contains("onsubmit=\"event.preventDefault();\""));
        assert!(h.contains("method=\"post\""));
        assert!(h.contains("data-en=\"Generate\""));
        assert!(h.contains("class=\"rb-box\""));
        assert!(h.contains("fillTrims"));
        assert!(h.contains("data-en=\"Pages\""));
        assert!(h.contains("data-uk=\"Згенерувати\""));
    }

    #[test]
    fn studio_html_covers_uk_en_leftovers() {
        let h = include_str!("../ui/studio.html");
        assert!(h.contains("draftNone"));
        assert!(h.contains("data-i18n=\"esc\""));
        assert!(h.contains("data-ph-en=\"— chapter —\""));
        assert!(h.contains("Esc — close"));
        assert!(h.contains("HALF_TITLE"));
        assert!(h.contains("jumpBook"));
        assert!(h.contains("measureBook"));
        assert!(h.contains("pvBookOff"));
        assert!(h.contains("class=\"editwrap rb-box\""));
        assert!(h.contains("id=\"prevPane\""));
        assert!(h.contains("mode:'translate'"));
        assert!(h.contains("/api/ai"));
    }

    #[test]
    fn products_html_title_is_bilingual() {
        let h = include_str!("../ui/products.html");
        assert!(h.contains("data-en=\"Products — rebook\""));
        assert!(h.contains("data-uk=\"Продукти — rebook\""));
        assert!(h.contains("class=\"card rb-box\""));
    }
}
