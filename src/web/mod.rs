//! 管理面板 Web 服务：仅监听本机，提供静态资源与 JSON API
//! Panel web server: localhost-only, serves static assets and the JSON API.

pub mod api;
pub mod assets;
pub mod client;
pub mod http;

use std::sync::atomic::{AtomicU16, Ordering};

/// 实际绑定的面板端口（供窗口程序的“测试聊天”按钮使用）
static PANEL_PORT: AtomicU16 = AtomicU16::new(8790);

pub fn set_panel_port(port: u16) {
    PANEL_PORT.store(port, Ordering::SeqCst);
}

pub fn panel_port() -> u16 {
    PANEL_PORT.load(Ordering::SeqCst)
}

use api::Shared;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

// 聊天请求可能带 base64 图片，放宽到 16MB（仅本机 127.0.0.1 访问）
const MAX_BODY: usize = 16 * 1024 * 1024;

/// 绑定端口，占用时自动向后寻找可用端口
pub fn bind(desired: u16) -> std::io::Result<(TcpListener, u16)> {
    let mut port = desired.max(1024);
    for _ in 0..40 {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(l) => return Ok((l, port)),
            Err(_) => port = port.saturating_add(1),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrInUse,
        "no free port",
    ))
}

pub fn serve(listener: TcpListener, shared: Shared) {
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let shared = Arc::clone(&shared);
                std::thread::spawn(move || handle_conn(s, shared));
            }
            Err(_) => continue,
        }
    }
}

fn handle_conn(mut stream: TcpStream, shared: Shared) {
    let req = match http::read_request(&stream, MAX_BODY) {
        Ok(r) => r,
        Err(_) => {
            let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            return;
        }
    };

    let resp = if req.path.starts_with("/api/") {
        api::handle(&shared, &req)
    } else {
        serve_static(&req)
    };
    http::write_response(&mut stream, &resp);
}

fn serve_static(req: &http::Request) -> http::Response {
    let mut path = req.path.clone();
    if path == "/" {
        path = "/index.html".into();
    }
    if path == "/chat" {
        path = "/chat.html".into();
    }
    match assets::get(&path) {
        Some((mime, bytes)) => http::Response::bytes(bytes, mime),
        None => {
            let mut body = String::from("404 Not Found: ");
            body.push_str(&path);
            http::Response::text(&body, 404)
        }
    }
}
