//! 极简 HTTP 客户端：用于代理测试聊天与健康检查
//! Tiny HTTP client used for the test-chat proxy and health checks.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub struct ClientResponse {
    pub code: u16,
    pub body: String,
}

/// 向 127.0.0.1:<port> 发起请求；host 仅用于展示
pub fn request(
    host: &str,
    port: u16,
    method: &str,
    path: &str,
    api_key: Option<&str>,
    body: Option<&str>,
) -> Result<ClientResponse, String> {
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| "no address".to_string())?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|e| format!("connect: {}", e))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(300)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;

    let mut req = String::new();
    req.push_str(&format!("{} {} HTTP/1.1\r\n", method, path));
    req.push_str(&format!("Host: {}:{}\r\n", host, port));
    req.push_str("Connection: close\r\n");
    req.push_str("Accept: application/json\r\n");
    if let Some(k) = api_key {
        if !k.is_empty() {
            req.push_str(&format!("Authorization: Bearer {}\r\n", k));
        }
    }
    if let Some(b) = body {
        req.push_str("Content-Type: application/json\r\n");
        req.push_str(&format!("Content-Length: {}\r\n", b.as_bytes().len()));
    }
    req.push_str("\r\n");
    if let Some(b) = body {
        req.push_str(b);
    }

    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("write: {}", e))?;
    stream.flush().ok();

    let mut raw: Vec<u8> = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|e| format!("read: {}", e))?;

    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> Result<ClientResponse, String> {
    let sep = find_subslice(raw, b"\r\n\r\n").ok_or_else(|| "no header end".to_string())?;
    let head = String::from_utf8_lossy(&raw[..sep]).to_string();
    let rest = &raw[sep + 4..];

    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or("");
    let code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse::<u16>().ok())
        .unwrap_or(0);

    let mut chunked = false;
    let mut content_length: Option<usize> = None;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_lowercase();
            let val = v.trim().to_string();
            match key.as_str() {
                "content-type" => {}
                "transfer-encoding" => {
                    if val.to_lowercase().contains("chunked") {
                        chunked = true;
                    }
                }
                "content-length" => content_length = val.parse().ok(),
                _ => {}
            }
        }
    }

    let body_bytes: Vec<u8> = if chunked {
        decode_chunked(rest)
    } else if let Some(n) = content_length {
        rest.iter().take(n).copied().collect()
    } else {
        rest.to_vec()
    };

    Ok(ClientResponse {
        code,
        body: String::from_utf8_lossy(&body_bytes).to_string(),
    })
}

fn decode_chunked(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let Some(pos) = find_subslice(&data[i..], b"\r\n") else {
            break;
        };
        let size_line = String::from_utf8_lossy(&data[i..i + pos]).to_string();
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let Ok(size) = usize::from_str_radix(size_hex, 16) else {
            break;
        };
        if size == 0 {
            break;
        }
        let start = i + pos + 2;
        let end = (start + size).min(data.len());
        out.extend_from_slice(&data[start..end]);
        i = end + 2;
    }
    out
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}
