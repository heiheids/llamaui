//! 多语言：语言文件内嵌，运行时按语言环境选择或手动切换
//! i18n: embedded locale files, selected by OS locale or explicit setting.

use crate::json::{self, Json};
use crate::util;

pub const LANGS: &[(&str, &str)] = &[("zh-CN", "简体中文"), ("en-US", "English")];

const ZH: &str = include_str!("../locales/zh-CN.json");
const EN: &str = include_str!("../locales/en-US.json");

pub fn load(lang: &str) -> Json {
    let raw = if util::normalize_lang(lang) == "zh-CN" { ZH } else { EN };
    json::parse(raw).unwrap_or_else(|_| {
        let mut o = Json::obj();
        o.set("meta", {
            let mut m = Json::obj();
            m.set("broken", Json::Bool(true));
            m
        });
        o
    })
}

/// 依据配置解析实际语言代码
pub fn resolve(cfg: &Json) -> String {
    let want = cfg.s("app.lang", "auto");
    if want == "auto" || want.is_empty() {
        util::detect_system_lang()
    } else {
        util::normalize_lang(&want)
    }
}

/// 取文案；缺失时回退为键名，便于发现未翻译项
pub fn t(lang: &str, key: &str) -> String {
    let tree = load(lang);
    match tree.path(key) {
        Some(Json::Str(s)) => s.clone(),
        _ => {
            let fallback = load("en-US");
            match fallback.path(key) {
                Some(Json::Str(s)) => s.clone(),
                _ => key.to_string(),
            }
        }
    }
}

pub fn bundle_json(lang: &str) -> Json {
    let mut o = Json::obj();
    o.set("lang", Json::str(util::normalize_lang(lang)));
    o.set("strings", load(lang));
    let mut avail = Vec::new();
    for (code, name) in LANGS {
        let mut it = Json::obj();
        it.set("code", Json::str(*code));
        it.set("name", Json::str(*name));
        avail.push(it);
    }
    o.set("available", Json::Arr(avail));
    o
}
