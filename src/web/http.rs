//! 极简 HTTP/1.1 服务端实现（仅依赖 std）
//! Minimal HTTP/1.1 server helpers built on std only.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    #[allow(dead_code)] // 预留：面板令牌 / 反向代理鉴权
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
}

pub struct Response {
    pub code: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub extra_headers: Vec<(String, String)>,
}

impl Response {
    pub fn json(value: &crate::json::Json, code: u16) -> Response {
        Response {
            code,
            content_type: "application/json; charset=utf-8".into(),
            body: value.to_compact().into_bytes(),
            extra_headers: Vec::new(),
        }
    }

    pub fn text(msg: &str, code: u16) -> Response {
        Response {
            code,
            content_type: "text/plain; charset=utf-8".into(),
            body: msg.as_bytes().to_vec(),
            extra_headers: Vec::new(),
        }
    }

    pub fn bytes(data: &'static [u8], mime: &str) -> Response {
        Response {
            code: 200,
            content_type: mime.into(),
            body: data.to_vec(),
            extra_headers: vec![("Cache-Control".into(), "no-cache".into())],
        }
    }

}

/// 读取并解析请求；超时或格式错误时返回 Err
pub fn read_request(stream: &TcpStream, max_body: usize) -> Result<Request, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);

    let mut line = String::new();
    if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
        return Err("empty".into());
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let target = parts.next().unwrap_or("/").to_string();

    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), parse_query(q)),
        None => (target.clone(), HashMap::new()),
    };

    let mut headers = HashMap::new();
    loop {
        let mut h = String::new();
        let n = reader.read_line(&mut h).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        let trimmed = h.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }

    let len: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
        .min(max_body);
    let mut body = vec![0u8; len];
    if len > 0 {
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    }

    Ok(Request {
        method,
        path: percent_decode(&path),
        query,
        headers,
        body,
    })
}

pub fn write_response(stream: &mut TcpStream, resp: &Response) {
    let mut head = String::new();
    head.push_str(&format!("HTTP/1.1 {} {}\r\n", resp.code, status_text(resp.code)));
    head.push_str(&format!("Content-Type: {}\r\n", resp.content_type));
    head.push_str(&format!("Content-Length: {}\r\n", resp.body.len()));
    head.push_str("Connection: close\r\n");
    head.push_str("X-Content-Type-Options: nosniff\r\n");
    for (k, v) in &resp.extra_headers {
        head.push_str(&format!("{}: {}\r\n", k, v));
    }
    head.push_str("\r\n");
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&resp.body);
    let _ = stream.flush();
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "Status",
    }
}

pub fn parse_query(raw: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        map.insert(percent_decode(k), percent_decode(v));
    }
    map
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

