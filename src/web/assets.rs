//! 内嵌静态资源：单文件分发，无需相对路径依赖
//! Embedded static assets so the binary is self-contained.

macro_rules! asset {
    ($path:expr, $file:expr, $mime:expr) => {
        ($path, include_bytes!(concat!("../../", $file)) as &'static [u8], $mime)
    };
}

pub static FILES: &[(&str, &[u8], &str)] = &[
    asset!("/index.html", "assets/index.html", "text/html; charset=utf-8"),
    asset!("/chat.html", "assets/chat.html", "text/html; charset=utf-8"),
    asset!("/advanced.html", "assets/advanced.html", "text/html; charset=utf-8"),
    asset!("/css/base.css", "assets/css/base.css", "text/css; charset=utf-8"),
    asset!("/css/panel.css", "assets/css/panel.css", "text/css; charset=utf-8"),
    asset!("/css/chat.css", "assets/css/chat.css", "text/css; charset=utf-8"),
    asset!("/js/api.js", "assets/js/api.js", "text/javascript; charset=utf-8"),
    asset!("/js/i18n.js", "assets/js/i18n.js", "text/javascript; charset=utf-8"),
    asset!("/js/status.js", "assets/js/status.js", "text/javascript; charset=utf-8"),
    asset!("/js/models.js", "assets/js/models.js", "text/javascript; charset=utf-8"),
    asset!("/js/settings.js", "assets/js/settings.js", "text/javascript; charset=utf-8"),
    asset!("/js/panel.js", "assets/js/panel.js", "text/javascript; charset=utf-8"),
    asset!("/js/advanced.js", "assets/js/advanced.js", "text/javascript; charset=utf-8"),
    asset!("/js/chat.js", "assets/js/chat.js", "text/javascript; charset=utf-8"),
    asset!("/icon.png", "assets/icon.png", "image/png"),
    asset!("/favicon.ico", "assets/icon.ico", "image/x-icon"),
];

pub fn get(path: &str) -> Option<(&'static str, &'static [u8])> {
    FILES
        .iter()
        .find(|(p, _, _)| p.eq_ignore_ascii_case(path))
        .map(|(_, b, m)| (*m, *b))
}
