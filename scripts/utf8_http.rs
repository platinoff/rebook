//! One-shot loopback JSON GET/POST as UTF-8 bytes. No Python, no code page.
//! Usage: utf8_http <get|post> <host> <port> <path> [json-body-file] <out-file>

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process;
use std::time::Duration;

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    process::exit(1);
}

fn http(host: &str, port: u16, method: &str, path: &str, body: &[u8]) -> Vec<u8> {
    let mut s = TcpStream::connect((host, port)).unwrap_or_else(|e| die(&e.to_string()));
    let _ = s.set_read_timeout(Some(Duration::from_secs(8)));
    let origin = format!("http://{host}:{port}");
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\nOrigin: {origin}\r\nConnection: close\r\nAccept: application/json\r\n"
    );
    if method == "POST" {
        req.push_str("Content-Type: application/json; charset=utf-8\r\n");
        req.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    req.push_str("\r\n");
    s.write_all(req.as_bytes()).unwrap_or_else(|e| die(&e.to_string()));
    if method == "POST" && !body.is_empty() {
        s.write_all(body).unwrap_or_else(|e| die(&e.to_string()));
    }
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).unwrap_or_else(|e| die(&e.to_string()));
    buf
}

fn http_body(raw: &[u8]) -> &[u8] {
    raw.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| &raw[i + 4..])
        .unwrap_or(raw)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let method = args.next().unwrap_or_else(|| die("method get|post"));
    let host = args.next().unwrap_or_else(|| die("host"));
    let port: u16 = args
        .next()
        .unwrap_or_else(|| die("port"))
        .parse()
        .unwrap_or_else(|_| die("port"));
    let path = args.next().unwrap_or_else(|| die("path"));
    let (body_path, out) = if method == "post" {
        (
            Some(args.next().unwrap_or_else(|| die("json body file"))),
            args.next().unwrap_or_else(|| die("out file")),
        )
    } else {
        (None, args.next().unwrap_or_else(|| die("out file")))
    };
    let req_body = match body_path {
        Some(p) => fs::read(&p).unwrap_or_else(|e| die(&e.to_string())),
        None => Vec::new(),
    };
    if !req_body.is_empty() && std::str::from_utf8(&req_body).is_err() {
        die("request body is not utf-8");
    }
    let raw = http(&host, port, &method.to_ascii_uppercase(), &path, &req_body);
    let body = http_body(&raw);
    if std::str::from_utf8(body).is_err() {
        die("response body is not utf-8");
    }
    fs::write(&out, body).unwrap_or_else(|e| die(&e.to_string()));
}
