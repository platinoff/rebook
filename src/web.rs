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
use crate::viewer::{DEFAULT_ADDR, LoadedBook, route};

/// Shared server state: every book discovered on disk + the drafts root.
pub struct AppState {
    /// Books on the shelf.
    pub books: Vec<LoadedBook>,
    /// `workspace/drafts` directory (Studio area).
    pub drafts_root: std::path::PathBuf,
}

const STUDIO_HTML: &str = include_str!("../ui/studio.html");
const COVER_HTML: &str = include_str!("../ui/cover.html");

/// Build the router over a frozen set of discovered books.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/studio", get(studio_page))
        .route("/cover", get(cover_page))
        .route("/api/books", get(api_books))
        .route("/api/cover-template", get(api_cover_template))
        .route("/api/drafts", get(api_drafts_list).post(api_drafts_create))
        .route(
            "/api/drafts/{id}",
            get(api_draft_get).delete(api_draft_delete),
        )
        .route("/api/drafts/{id}/chapter/{num}", put(api_draft_put_chapter))
        .route("/api/drafts/{id}/promote", post(api_draft_promote))
        .route("/api/ai", post(api_ai))
        .fallback(viewer_fallback)
        .with_state(state)
}

/// Run the previewer/service until the process exits. Bind only to loopback.
pub async fn serve_web(books: Vec<LoadedBook>, addr: &str) -> Result<(), String> {
    let drafts_root = std::path::PathBuf::from("workspace/drafts");
    let state = Arc::new(AppState { books, drafts_root });
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

async fn studio_page() -> Html<&'static str> {
    Html(STUDIO_HTML)
}

async fn cover_page() -> Html<&'static str> {
    Html(COVER_HTML)
}

async fn api_books(State(st): State<Arc<AppState>>) -> Response {
    let mut items = String::from("[");
    for (i, b) in st.books.iter().enumerate() {
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
    if st.books.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            "Не знайдено жодного *.epub — запустіть build-epub або покладіть файл у каталог.",
        )
            .into_response();
    }
    let (status, body, mime) = route(path, &st.books);
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
    let body = format!(
        "{{\"meta\":{},\"chapters\":[{}]}}",
        meta.to_json(),
        parts.join(",")
    );
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

/// Body for `POST /api/ai`.
#[derive(serde::Deserialize)]
struct AiIn {
    mode: String,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    context: String,
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
    match crate::ai::complete(&endpoint, &payload).await {
        Ok(text) => text_response(StatusCode::OK, "text/plain; charset=utf-8", text),
        Err(e) => text_response(StatusCode::BAD_GATEWAY, "text/plain; charset=utf-8", e),
    }
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
            drafts_root: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("web-drafts"),
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
    }

    #[tokio::test]
    async fn api_books_and_viewer_fallback() {
        let (s, b) = get(router(fixture_state()), "/api/books").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("\"id\":\"test\""));
        let (s, b) = get(router(fixture_state()), "/test/chapter/1").await;
        assert_eq!(s, StatusCode::OK);
        assert!(b.contains("<h1>Y</h1>"));
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
}
