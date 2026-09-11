//! AI assist adapter: OpenAI-compatible chat completions for the Studio.
//!
//! Zero extra HTTP dependencies — a hand-rolled HTTP/1.1 POST over a tokio
//! `TcpStream`, because the target endpoints are local OpenAI-compatible
//! servers (llama-rs, OmniRoute: `http://127.0.0.1:…/v1/chat/completions`).
//! `https://` is refused (no TLS stack here, and none should leak secrets).
//!
//! Off by default: unset `REBOOK_AI_ENDPOINT` ⇒ the adapter reports
//! `enabled() == false` and the API answers 503. The token
//! (`REBOOK_AI_TOKEN`) is only ever sent as a header, never returned.

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Assist modes offered in the Studio.
pub const MODES: &[&str] = &["continue", "rewrite", "summarize", "outline", "cover-brief"];

/// Per-mode system instructions (uk prose, markdown output).
pub fn system_prompt(mode: &str) -> &'static str {
    match mode {
        "continue" => {
            "Ти — співавтор книги. Продовжуй текст у тому ж стилі й мові, без заголовків, лише прозу."
        }
        "rewrite" => {
            "Перепиши наданий уривок чистіше й точніше, зберігаючи зміст, стиль і мову оригіналу."
        }
        "summarize" => "Стисло перекажи уривок українською (або мовою тексту) у 3–6 речень.",
        "outline" => {
            "Склади план розділу у markdown (## / ### заголовки + тези) за темою й контекстом."
        }
        "cover-brief" => {
            "Опиши арт-бриф для обкладинки: настрій, композицію, палітру, типографіку — текстом."
        }
        _ => "Допоможи автору книги українською markdown-ом.",
    }
}

/// Endpoint parts after URL validation (host, port, path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// HTTP host (no scheme), as passed to connect.
    pub host: String,
    /// TCP port (80 default; https rejected).
    pub port: u16,
    /// Request path (e.g. /v1/chat/completions).
    pub path: String,
}

/// Parse `http://host[:port]/path`; reject anything else.
pub fn parse_endpoint(url: &str) -> Result<Endpoint, String> {
    let rest = url.strip_prefix("http://").ok_or_else(|| {
        "REBOOK_AI_ENDPOINT must be http:// (local OpenAI-compatible host; https not supported)"
            .to_string()
    })?;
    if rest.is_empty() {
        return Err("empty endpoint".to_string());
    }
    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, format!("/{p}")),
        None => (rest, "/v1/chat/completions".to_string()),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (
            h.to_string(),
            p.parse::<u16>().map_err(|_| format!("bad port in {url}"))?,
        ),
        None => (authority.to_string(), 80),
    };
    if host.is_empty() {
        return Err(format!("no host in {url}"));
    }
    Ok(Endpoint { host, port, path })
}

/// True when the env gate is on.
pub fn enabled() -> bool {
    std::env::var("REBOOK_AI_ENDPOINT").is_ok_and(|v| !v.trim().is_empty())
}

/// Build the OpenAI-compatible chat body (mode-aware, with optional context).
pub fn chat_body(mode: &str, prompt: &str, context: &str, model: &str) -> Value {
    let mut user = String::from("");
    if !context.trim().is_empty() {
        user.push_str("Контекст:\n");
        user.push_str(context.trim());
        user.push_str("\n\nЗавдання: ");
    }
    user.push_str(prompt.trim());
    json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt(mode)},
            {"role": "user", "content": user},
        ],
        "temperature": 0.7,
        "max_tokens": 1024
    })
}

/// Extract assistant text from a chat-completions JSON response.
pub fn extract_content(v: &Value) -> Option<String> {
    v.get("choices")?
        .get(0)?
        .get("message")?
        .get("content")?
        .as_str()
        .map(|s| s.to_string())
}

/// POST the body to the endpoint and collect deltas, return the assistant text.
pub async fn complete(endpoint: &str, body: &Value) -> Result<String, String> {
    let mut parts = String::new();
    stream_completion(endpoint, body, &mut |chunk| {
        parts.push_str(&chunk);
        Ok(())
    })
    .await?;
    Ok(parts)
}

/// One parsed upstream SSE line.
#[derive(Debug, PartialEq, Eq)]
pub enum SseEvent {
    /// A non-empty content delta.
    Delta(String),
    /// `data: [DONE]`.
    Done,
}

/// Interpret one upstream `text/event-stream` line from a chat-completions
/// stream: role-only deltas and blank data are noise; `[DONE]` terminates.
pub fn sse_parse_line(line: &str) -> Option<SseEvent> {
    let payload = line.trim().strip_prefix("data:")?.trim();
    if payload.is_empty() {
        return None;
    }
    if payload == "[DONE]" {
        return Some(SseEvent::Done);
    }
    let v: Value = serde_json::from_str(payload).ok()?;
    let delta = v
        .get("choices")?
        .get(0)?
        .get("delta")?
        .get("content")?
        .as_str()?;
    if delta.is_empty() {
        return None;
    }
    Some(SseEvent::Delta(delta.to_string()))
}

/// Stream a chat completion (`stream: true` upstream), invoking `on_chunk`
/// per content delta. Backs both [`complete`] and the SSE endpoint.
pub async fn stream_completion(
    endpoint: &str,
    body: &Value,
    on_chunk: &mut (dyn FnMut(String) -> Result<(), String> + Send),
) -> Result<(), String> {
    let ep = parse_endpoint(endpoint)?;
    let mut payload = body.clone();
    payload["stream"] = Value::Bool(true);
    let payload = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let stream = TcpStream::connect((ep.host.as_str(), ep.port))
        .await
        .map_err(|e| format!("connect {host}:{port}: {e}", host = ep.host, port = ep.port))?;
    tokio::time::timeout(
        std::time::Duration::from_secs(180),
        pump(stream, &ep, &payload, on_chunk),
    )
    .await
    .map_err(|_| "AI stream timed out".to_string())?
}

async fn pump(
    stream: TcpStream,
    ep: &Endpoint,
    payload: &[u8],
    on_chunk: &mut (dyn FnMut(String) -> Result<(), String> + Send),
) -> Result<(), String> {
    let mut reader = open_stream(stream, ep, payload).await?;
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        match sse_parse_line(&line) {
            Some(SseEvent::Done) => break,
            Some(SseEvent::Delta(t)) => on_chunk(t)?,
            None => {}
        }
    }
    Ok(())
}

/// Forward a chat-completion stream into an mpsc channel (true incremental
/// deltas; a final `Err` carries the failure). Backs the SSE endpoint.
pub async fn stream_to(
    endpoint: &str,
    body: &Value,
    tx: &tokio::sync::mpsc::Sender<Result<String, String>>,
) {
    let out = run_stream_to(endpoint, body, tx).await;
    if let Err(e) = out {
        let _ = tx.send(Err(e)).await;
    }
}

async fn run_stream_to(
    endpoint: &str,
    body: &Value,
    tx: &tokio::sync::mpsc::Sender<Result<String, String>>,
) -> Result<(), String> {
    let ep = parse_endpoint(endpoint)?;
    let mut payload = body.clone();
    payload["stream"] = Value::Bool(true);
    let payload = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let stream = TcpStream::connect((ep.host.as_str(), ep.port))
        .await
        .map_err(|e| format!("connect {host}:{port}: {e}", host = ep.host, port = ep.port))?;
    tokio::time::timeout(
        std::time::Duration::from_secs(180),
        pump_to(stream, &ep, &payload, tx),
    )
    .await
    .map_err(|_| "AI stream timed out".to_string())?
}

async fn pump_to(
    stream: TcpStream,
    ep: &Endpoint,
    payload: &[u8],
    tx: &tokio::sync::mpsc::Sender<Result<String, String>>,
) -> Result<(), String> {
    let mut reader = open_stream(stream, ep, payload).await?;
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        match sse_parse_line(&line) {
            Some(SseEvent::Done) => break,
            Some(SseEvent::Delta(t)) => {
                if tx.send(Ok(t)).await.is_err() {
                    break;
                }
            }
            None => {}
        }
    }
    Ok(())
}

/// Connect, send the request head+body and skip the response headers,
/// returning a buffered reader positioned at the SSE body.
async fn open_stream(
    stream: TcpStream,
    ep: &Endpoint,
    payload: &[u8],
) -> Result<tokio::io::BufReader<TcpStream>, String> {
    let mut io = stream;
    let auth = match std::env::var("REBOOK_AI_TOKEN") {
        Ok(t) if !t.trim().is_empty() => format!("authorization: bearer {}\r\n", t.trim()),
        _ => String::new(),
    };
    let head = format!(
        "POST {path} HTTP/1.1\r\nhost: {host}\r\ncontent-type: application/json\r\ncontent-length: {len}\r\n{auth}connection: close\r\n\r\n",
        path = ep.path,
        host = ep.host,
        len = payload.len(),
        auth = auth
    );
    io.write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    io.write_all(payload).await.map_err(|e| e.to_string())?;
    let mut reader = tokio::io::BufReader::new(io);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("AI upstream closed during headers".to_string());
        }
        if line.starts_with("HTTP/") && !line.contains(" 200") {
            return Err(format!("AI upstream status: {}", line.trim()));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
    }
    Ok(reader)
}

/// Split a raw HTTP/1.1 response (with optional chunked transfer) into
/// `(status_ok, body)`. Kept for tests of the non-SSE fallback path.
#[cfg(test)]
fn split_response(raw: &[u8]) -> Result<(bool, String), String> {
    let sep = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "malformed HTTP response (no header terminator)".to_string())?;
    let head = String::from_utf8_lossy(&raw[..sep]).into_owned();
    let status_ok = head.split_whitespace().nth(1) == Some("200");
    let code = head.split_whitespace().nth(1).unwrap_or("?").to_string();
    let body = &raw[sep + 4..];
    let body = if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        decode_chunked(body)?
    } else {
        body.to_vec()
    };
    if !status_ok {
        return Ok((
            false,
            format!("HTTP {code}: {}", String::from_utf8_lossy(&body)),
        ));
    }
    Ok((true, String::from_utf8_lossy(&body).into_owned()))
}

#[cfg(test)]
fn decode_chunked(mut data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let nl = data
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("truncated chunk size")?;
        let size = usize::from_str_radix(String::from_utf8_lossy(&data[..nl]).trim(), 16)
            .map_err(|_| "bad chunk size")?;
        data = &data[nl + 2..];
        if size == 0 {
            break;
        }
        if data.len() < size {
            return Err("truncated chunk".to_string());
        }
        out.extend_from_slice(&data[..size]);
        data = &data[size..];
        if data.starts_with(b"\r\n") {
            data = &data[2..];
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_parsing() {
        let e = parse_endpoint("http://127.0.0.1:20128/v1/chat/completions").unwrap();
        assert_eq!(e.host, "127.0.0.1");
        assert_eq!(e.port, 20128);
        assert_eq!(e.path, "/v1/chat/completions");
        let e = parse_endpoint("http://localhost:8080").unwrap();
        assert_eq!((e.port, e.path.as_str()), (8080, "/v1/chat/completions"));
        assert!(parse_endpoint("https://openai.example/v1").is_err());
        assert!(parse_endpoint("ftp://x").is_err());
    }

    #[test]
    fn modes_and_body_shape() {
        assert_eq!(MODES.len(), 5);
        assert!(system_prompt("outline").contains("markdown"));
        let body = chat_body("continue", "продовж", "уривок", "llama");
        assert_eq!(body["model"], "llama");
        assert_eq!(body["messages"][0]["role"], "system");
        assert!(
            body["messages"][1]["content"]
                .as_str()
                .unwrap()
                .contains("уривок")
        );
    }

    #[test]
    fn extract_and_chunked() {
        let v: Value =
            serde_json::from_str(r#"{"choices":[{"message":{"content":"halt"}}]}"#).unwrap();
        assert_eq!(extract_content(&v).unwrap(), "halt");
        assert_eq!(extract_content(&json!({})), None);
        let raw = b"2\r\nab\r\n1\r\nc\r\n0\r\n\r\n";
        assert_eq!(decode_chunked(raw).unwrap(), b"abc");
        let resp = b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
        let (ok, body) = split_response(resp).unwrap();
        assert!(ok);
        assert_eq!(body, "hello");
        let resp = b"HTTP/1.1 500 Oops\r\ncontent-length: 0\r\n\r\n";
        let (ok, _) = split_response(resp).unwrap();
        assert!(!ok);
    }

    #[test]
    fn disabled_by_default() {
        // edition 2024: env mutation is unsafe; isolated one-field read-only test.
        unsafe { std::env::remove_var("REBOOK_AI_ENDPOINT") };
        assert!(!enabled());
    }

    #[test]
    fn sse_lines_parse() {
        assert_eq!(
            sse_parse_line(r#"data: {"choices":[{"delta":{"content":"Pri"}}]}"#),
            Some(SseEvent::Delta("Pri".into()))
        );
        assert_eq!(
            sse_parse_line(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#),
            None
        ); // role-only delta is noise
        assert_eq!(sse_parse_line("data: [DONE]"), Some(SseEvent::Done));
        assert_eq!(sse_parse_line(": keepalive"), None);
        assert_eq!(sse_parse_line(""), None);
    }

    #[tokio::test]
    async fn stream_to_forwards_deltas_and_done() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            // read request headers+body until blank line + a bit of body
            let mut buf = vec![0u8; 4096];
            let _ = tokio::io::AsyncReadExt::read(&mut sock, &mut buf).await;
            sock.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n")
                .await
                .unwrap();
            for l in [
                "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"Pri\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"vit\"}}]}\n\n",
                "data: [DONE]\n\n",
            ] {
                sock.write_all(l.as_bytes()).await.unwrap();
            }
            sock.flush().await.unwrap();
        });
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let ep = format!("http://{addr}/v1/chat/completions");
        let body = chat_body("continue", "", "context", "test");
        stream_to(&ep, &body, &tx).await;
        drop(tx);
        let mut got = String::new();
        while let Some(Ok(chunk)) = rx.recv().await {
            got.push_str(&chunk);
        }
        assert_eq!(got, "Privit");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn complete_collects_stream() {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = tokio::io::AsyncReadExt::read(&mut sock, &mut buf).await;
            use tokio::io::AsyncWriteExt;
            sock.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"A\"}}]}\n\ndata: [DONE]\n\n")
                .await
                .unwrap();
            sock.flush().await.unwrap();
        });
        let ep = format!("http://{addr}/");
        let body = chat_body("summarize", "", "x", "t");
        assert_eq!(complete(&ep, &body).await.unwrap(), "A");
        server.await.unwrap();
    }
}
