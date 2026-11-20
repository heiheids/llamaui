//! 参数解读的中文翻译缓存 + 本地模型自译。
//!
//! 设计约束（与规划一致）：
//! * 只在用户点「生成中文解读」时运行，绝不自动偷跑算力
//! * 翻译结果写 `locales/params.zh-CN.json`，人工可覆盖（by=human 永不覆盖）
//! * 按「说明哈希」判断是否过期：llama.cpp 改了文案才需要重译
//! * 失败不致命：没服务、模型拒绝、JSON 不合法 → 返回错误，界面照常显示英文

use crate::json::Json;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Entry {
    pub hash: String,
    pub zh: String,
    /// local = 本地模型译，human = 人工填写
    pub by: String,
}

pub fn cache_path() -> PathBuf {
    crate::util::base_dir().join("locales").join("params.zh-CN.json")
}

pub fn load() -> BTreeMap<String, Entry> {
    let mut out = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(cache_path()) else {
        return out;
    };
    let Ok(v) = crate::json::parse(&text) else {
        return out;
    };
    let Some(Json::Obj(map)) = v.path("params") else {
        return out;
    };
    for (k, e) in map.iter() {
        out.insert(
            k.clone(),
            Entry {
                hash: e.s("hash", ""),
                zh: e.s("zh", ""),
                by: e.s("by", "local"),
            },
        );
    }
    out
}

pub fn save(map: &BTreeMap<String, Entry>, version: &str, model: &str) {
    let mut params = Json::obj();
    for (k, e) in map.iter() {
        if e.zh.trim().is_empty() {
            continue;
        }
        let mut o = Json::obj();
        o.set("hash", Json::str(&e.hash));
        o.set("zh", Json::str(&e.zh));
        o.set("by", Json::str(&e.by));
        params.set(k, o);
    }
    let mut root = Json::obj();
    let mut meta = Json::obj();
    meta.set("version", Json::str(version));
    meta.set("model", Json::str(model));
    meta.set("updatedAt", Json::Num(crate::util::now_secs() as f64));
    root.set("meta", meta);
    root.set("params", params);
    let path = cache_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, root.to_pretty());
}

/// 取某个参数的中文解读（哈希不匹配视为过期，返回 None）
pub fn lookup(flag: &str, desc_hash: &str) -> Option<String> {
    let map = load();
    let e = map.get(flag)?;
    if e.hash != desc_hash || e.zh.trim().is_empty() {
        return None;
    }
    Some(e.zh.clone())
}

/// 待翻译项
pub struct Pending {
    pub flag: String,
    pub hash: String,
    pub desc: String,
    pub default: String,
    pub allowed: Vec<String>,
}

/// 从给定的参数表里挑出「还没有新鲜中文」的条目（人工译文视为已存在）
pub fn pending(specs: &[(String, String, String, Vec<String>)]) -> Vec<Pending> {
    let map = load();
    let mut out = Vec::new();
    for (flag, desc, default, allowed) in specs.iter() {
        let hash = format!("{:08x}", fnv1a(desc.as_bytes()) as u32);
        match map.get(flag) {
            Some(e) if e.hash == hash && !e.zh.trim().is_empty() => continue,
            _ => {}
        }
        out.push(Pending {
            flag: flag.clone(),
            hash,
            desc: desc.clone(),
            default: default.clone(),
            allowed: allowed.clone(),
        });
    }
    out
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// 用本地 llama-server 翻译一批参数说明。
/// 返回 (成功数, 失败说明)。
pub fn translate_batch(
    host: &str,
    port: u16,
    api_key: &str,
    items: &[Pending],
    version: &str,
    model: &str,
) -> (usize, String) {
    if items.is_empty() {
        return (0, String::new());
    }
    // 每条：--flag: 英文说明 (默认: x; 可选: a/b)
    let mut lines = String::new();
    for p in items.iter() {
        lines.push_str(&format!("{}: {}", p.flag, p.desc));
        if !p.default.is_empty() {
            lines.push_str(&format!(" (默认: {})", p.default));
        }
        if !p.allowed.is_empty() {
            lines.push_str(&format!(" (可选: {})", p.allowed.join("/")));
        }
        lines.push('\n');
    }
    // 用 /completion（非 chat）而不是 /chat/completions：
    // 思考型模型的 chat 接口会把 token 全花在 reasoning 上、content 为空，
    // 纯补全接口绕开模板与思考，对"翻译一小段"这种工具调用更可靠。
    let prompt = format!(
        "你是 llama.cpp 参数文档的翻译员。把参数说明译成简洁技术中文（每条不超过 80 字，保留数值与默认值）。\n\
         直接输出 JSON 对象，键是参数名（含前导 --），值是中文说明；不要解释、不要空话、不要重复原文。\n\n{}\n\nJSON:\n",
        lines
    );
    let mut body = Json::obj();
    body.set("prompt", Json::str(&prompt));
    body.set("temperature", Json::Num(0.2));
    body.set("n_predict", Json::Num(768.0));
    body.set("stream", Json::Bool(false));

    let body_str = body.to_compact();
    let resp = match crate::web::client::request(
        host,
        port,
        "POST",
        "/completion",
        Some(api_key),
        Some(&body_str),
    ) {
        Ok(r) => r,
        Err(e) => return (0, format!("请求失败: {}", e)),
    };
    if resp.code != 200 {
        return (0, format!("HTTP {}", resp.code));
    }
    let Ok(v) = crate::json::parse(&resp.body) else {
        return (0, "响应不是合法 JSON".to_string());
    };
    let content = v.s("content", "");
    // 先按严格 JSON 解析；失败（模型常被 n_predict 截断）则退回扫描 "键": "值" 对
    let strict = extract_json_object(&content);
    let mut map = load();
    let mut ok = 0usize;
    let mut pairs: Vec<(String, String)> = Vec::new();
    if let Some(obj) = strict {
        for p in items.iter() {
            let zh = obj.s(p.flag.as_str(), "");
            if !zh.trim().is_empty() {
                pairs.push((p.flag.clone(), zh));
            }
        }
    }
    if pairs.is_empty() {
        pairs = scan_pairs(&content);
    }
    for p in items.iter() {
        let Some((_, zh)) = pairs.iter().find(|(k, _)| k == &p.flag) else {
            continue;
        };
        let zh = zh.trim();
        if zh.is_empty() {
            continue;
        }
        // 人工填写的条目绝不覆盖
        if let Some(old) = map.get(&p.flag) {
            if old.by == "human" {
                continue;
            }
        }
        map.insert(
            p.flag.clone(),
            Entry {
                hash: p.hash.clone(),
                zh: zh.to_string(),
                by: "local".to_string(),
            },
        );
        ok += 1;
    }
    if ok > 0 {
        save(&map, version, model);
        (ok, String::new())
    } else {
        (
            0,
            format!(
                "模型返回的 JSON 没有匹配的键（请求 {} 条）：{}",
                items.len(),
                content.chars().take(80).collect::<String>()
            ),
        )
    }
}

/// 从模型输出里抠出第一个 JSON 对象（容忍 ```json 包裹与前后废话）
fn extract_json_object(s: &str) -> Option<Json> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end <= start {
        return None;
    }
    crate::json::parse(&s[start..=end]).ok()
}

/// 容错扫描：把输出里的 `"--flag": "中文"` 逐对抠出来。
/// 模型被 n_predict 截断时（最后一条没闭合）也能救回前面完整的条目。
fn scan_pairs(s: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = s.chars().collect();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '"' {
            i += 1;
            continue;
        }
        // 读键
        let Some((key, next)) = read_quoted(&chars, i) else {
            i += 1;
            continue;
        };
        let mut j = next;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if j >= chars.len() || chars[j] != ':' {
            i = next;
            continue;
        }
        j += 1;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if j >= chars.len() || chars[j] != '"' {
            i = next;
            continue;
        }
        let Some((val, next2)) = read_quoted(&chars, j) else {
            i = next;
            continue;
        };
        if key.starts_with("--") && !val.trim().is_empty() {
            out.push((key, val));
        }
        i = next2;
    }
    out
}

/// 从 chars[i] == '"' 开始读一个字符串，返回 (内容, 结束后的下标)
fn read_quoted(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut i = start + 1;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                let next = chars.get(i + 1)?;
                match next {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    other => out.push(*other),
                }
                i += 2;
            }
            '"' => return Some((out, i + 1)),
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    // 没闭合（被截断）：把已读到的内容返回，但标记为失败由调用方丢弃
    None
}
