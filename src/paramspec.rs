//! llama-server 参数自描述：解析 `--help`，缓存成 param-cache.json，
//! 并与本程序"已知参数"做差集，从而在 llama.cpp 升级后自动适配。
//!
//! 设计目标：
//! * 纯 std，无第三方依赖
//! * 解析失败不致命：解析不了的行原样保留，界面仍可显示
//! * 缓存以 exe 的 mtime+size 为键，只有换了版本才重新解析

use crate::json::Json;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

/// 从 --help 里解析出来的一个参数
#[derive(Clone, Debug)]
pub struct ParamSpec {
    /// 主写法（优先长参数，如 --ctx-size）
    pub flag: String,
    /// 其它写法（短参数、别名）
    pub aliases: Vec<String>,
    /// 参数占位符，如 `N`、`[on|off|auto]`、`{none,layer}`；无值参数为空
    pub value: String,
    /// 由占位符推断的类型：bool / int / enum / text
    pub kind: &'static str,
    /// 可选值（enum 时有效）
    pub allowed: Vec<String>,
    /// 官方英文说明
    pub desc: String,
    /// 默认值文本（解析不到为空）
    pub default: String,
    /// 环境变量名（如 LLAMA_ARG_CTX_SIZE），用作稳定的翻译键
    pub env: String,
    /// 所属小节
    pub section: String,
}

impl ParamSpec {
    pub fn takes_value(&self) -> bool {
        self.kind != "bool"
    }
    /// 配置里的键名（去掉前导 -）
    pub fn config_key(&self) -> String {
        self.flag.trim_start_matches('-').to_string()
    }
    /// 说明的短哈希，用于翻译缓存失效判断
    pub fn desc_hash(&self) -> String {
        format!("{:08x}", fnv1a(self.desc.as_bytes()) as u32)
    }
}

#[derive(Clone, Debug, Default)]
pub struct HelpDoc {
    /// llama-server --version 的首行
    pub version: String,
    /// 整份 help 的哈希
    pub help_hash: String,
    pub params: Vec<ParamSpec>,
}

impl HelpDoc {
    pub fn find(&self, arg: &str) -> Option<&ParamSpec> {
        let name = arg.split('=').next().unwrap_or(arg);
        self.params
            .iter()
            .find(|p| p.flag == name || p.aliases.iter().any(|a| a == name))
    }
    pub fn all_flags(&self) -> Vec<String> {
        let mut v = Vec::new();
        for p in self.params.iter() {
            v.push(p.flag.clone());
            v.extend(p.aliases.iter().cloned());
        }
        v
    }
}

#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

static CURRENT: OnceLock<HelpDoc> = OnceLock::new();
static KNOWN: OnceLock<Vec<String>> = OnceLock::new();

/// 当前生效的参数表（未初始化时返回 None）
pub fn current() -> Option<&'static HelpDoc> {
    CURRENT.get()
}

fn set_current(doc: HelpDoc) {
    let _ = CURRENT.set(doc);
}

/// 本程序"自己就知道"的参数集合：
/// paramhelp 表里列出的 + 默认配置生成出来的，二者的并集。
pub fn known_flags() -> &'static [String] {
    KNOWN.get_or_init(|| {
        let mut set: BTreeMap<String, ()> = BTreeMap::new();
        for item in crate::paramhelp::ITEMS.iter() {
            for f in item.flags.iter() {
                set.insert((*f).to_string(), ());
            }
        }
        // 用默认配置跑一遍参数生成，自动覆盖所有会发射的 flag
        let cfg = crate::settings::defaults();
        let args = crate::settings::build_args(&cfg, "<model>");
        let mut i = 0;
        while i < args.len() {
            let t = &args[i];
            if t.starts_with('-') {
                set.insert(t.split('=').next().unwrap_or(t).to_string(), ());
            }
            i += 1;
        }
        set.into_keys().collect()
    })
}

/// 比较两份参数表：新增 / 移除 / 说明变化
pub fn diff(old: &HelpDoc, new: &HelpDoc) -> Diff {
    let mut d = Diff::default();
    for p in new.params.iter() {
        match old.find(&p.flag) {
            None => d.added.push(p.flag.clone()),
            Some(o) => {
                if o.desc_hash() != p.desc_hash() {
                    d.changed.push(p.flag.clone());
                }
            }
        }
    }
    for p in old.params.iter() {
        if new.find(&p.flag).is_none() {
            d.removed.push(p.flag.clone());
        }
    }
    d
}

// ---------------- 解析 --help ----------------

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// 解析 llama-server --help 的文本
pub fn parse_help(text: &str) -> HelpDoc {
    let mut doc = HelpDoc {
        help_hash: format!("{:016x}", fnv1a(text.as_bytes())),
        ..Default::default()
    };
    let mut section = String::new();
    // 当前正在累积的条目下标
    let mut cur: Option<usize> = None;

    for raw in text.lines() {
        let line = raw.replace('\t', "    ");
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            cur = None;
            continue;
        }
        // 小节标题：----- common params -----
        let t = trimmed.trim();
        if t.starts_with("-----") && t.ends_with("-----") && t.len() > 10 {
            section = t.trim_matches('-').trim().to_string();
            cur = None;
            continue;
        }
        let indent = trimmed.len() - trimmed.trim_start().len();
        // 续行（缩进较深）→ 拼到上一条说明里
        if indent >= 20 {
            if let Some(idx) = cur {
                let add = trimmed.trim();
                let p: &mut ParamSpec = &mut doc.params[idx];
                // env 单独提取，避免混进说明
                if let Some(env) = extract_env(add) {
                    p.env = env;
                    continue;
                }
                if !p.desc.is_empty() {
                    p.desc.push(' ');
                }
                p.desc.push_str(add);
            }
            continue;
        }

        // 新条目：在左侧 flag 列里找 2 个以上空格的断点
        let Some(split) = find_split(trimmed) else {
            continue;
        };
        let left = trimmed[..split].trim();
        let right = trimmed[split..].trim();
        if !left.starts_with('-') {
            continue;
        }
        let (flags, value) = split_flags(left);
        if flags.is_empty() {
            continue;
        }
        // 主写法：优先最长的长参数
        let mut primary = flags[0].clone();
        for f in flags.iter() {
            if f.starts_with("--") && f.len() > primary.len() {
                primary = f.clone();
            }
        }
        let mut aliases: Vec<String> = flags
            .into_iter()
            .filter(|f| *f != primary)
            .collect();
        aliases.push(primary.clone());
        let (kind, allowed) = infer_kind(&value, right);
        let spec = ParamSpec {
            flag: primary,
            aliases,
            value,
            kind,
            allowed,
            desc: right.to_string(),
            default: extract_default(right),
            env: extract_env(right).unwrap_or_default(),
            section: section.clone(),
        };
        doc.params.push(spec);
        cur = Some(doc.params.len() - 1);
    }
    doc
}

/// 找到 flag 列与说明列的分界（≥2 个空格，且不在括号占位符里）
fn find_split(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'[' | b'<' | b'{' => depth += 1,
            b']' | b'>' | b'}' => depth -= 1,
            b' ' if depth <= 0 => {
                // 连续空格 ≥2
                if i + 1 < bytes.len() && bytes[i + 1] == b' ' && i >= 8 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// 把 `-c,  --ctx-size N` 拆成 (["-c","--ctx-size"], "N")
fn split_flags(left: &str) -> (Vec<String>, String) {
    let mut flags: Vec<String> = Vec::new();
    let mut value = String::new();
    for tok in left.split(&[' ', ','][..]) {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        if tok.starts_with('-') {
            flags.push(tok.to_string());
        } else if value.is_empty() {
            value = tok.to_string();
        } else if !value.ends_with(',') {
            // 形如 `N0,N1,N2,...`
            value.push_str(tok);
        }
    }
    // 占位符合并：`N0,N1,N2,...` 这类被逗号切开的情况由 split 已处理大半
    (flags, value)
}

fn infer_kind(value: &str, desc: &str) -> (&'static str, Vec<String>) {
    if value.is_empty() {
        return ("bool", Vec::new());
    }
    // 说明里明确列出可选值
    let allowed = extract_allowed(desc);
    if !allowed.is_empty() {
        return ("enum", allowed);
    }
    // [on|off|auto] / <0|1> / {none,layer,row}
    let inner = value
        .trim_start_matches(['[', '<', '{'])
        .trim_end_matches([']', '>', '}']);
    if inner.contains('|') {
        let opts: Vec<String> = inner
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return ("enum", opts);
    }
    // 纯大写短词 → 数字；其余按文本处理（文本最安全，不会误拦合法值）
    let numeric = !inner.is_empty()
        && inner.len() <= 12
        && inner
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && !matches!(
            inner,
            "TYPE" | "NAME" | "PATH" | "FNAME" | "FILE" | "MODE" | "FORMAT" | "MESSAGE"
                | "STRING" | "BASENAME" | "TOKENIZER" | "TEMPLATE" | "STATE" | "GPU" | "DEVICE"
                | "LORA" | "URL" | "HOST" | "COLOR"
        );
    if numeric {
        ("int", Vec::new())
    } else {
        ("text", Vec::new())
    }
}

fn extract_env(s: &str) -> Option<String> {
    let key = "(env: ";
    let start = s.find(key)? + key.len();
    let rest = &s[start..];
    let end = rest.find(')')?;
    let v = rest[..end].trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn extract_default(s: &str) -> String {
    let key = "(default:";
    let Some(start) = s.find(key) else {
        return String::new();
    };
    let rest = &s[start + key.len()..];
    let end = rest.find(')').unwrap_or(rest.len());
    rest[..end].trim().to_string()
}

fn extract_allowed(s: &str) -> Vec<String> {
    let key = "allowed values:";
    let Some(start) = s.find(key) else {
        return Vec::new();
    };
    let rest = &s[start + key.len()..];
    let end = rest.find("(default").unwrap_or(rest.len());
    rest[..end]
        .split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty() && x.len() < 24)
        .collect()
}

// ---------------- 运行 exe 读取 help ----------------

/// 读取 exe 的 --version 与 --help（带缓存键：路径+大小+修改时间）
pub fn discover(exe: &str) -> Option<HelpDoc> {
    let path = Path::new(exe);
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size = meta.len();
    let cache_path = crate::util::data_dir().join("param-cache.json");

    // 命中缓存就直接用
    if let Some(doc) = load_cache(&cache_path, exe, mtime, size) {
        set_current(doc.clone());
        return Some(doc);
    }

    let version = run_capture(exe, &["--version"])
        .map(|s| s.lines().next().unwrap_or("").trim().to_string())
        .unwrap_or_default();
    let help = run_capture(exe, &["--help"])?;
    if !help.contains("--") {
        return None;
    }
    let mut doc = parse_help(&help);
    doc.version = version;
    save_cache(&cache_path, exe, mtime, size, &doc);
    set_current(doc.clone());
    Some(doc)
}

/// 读取上一次缓存的参数表（不做任何发现）
pub fn cached() -> Option<HelpDoc> {
    let cache_path = crate::util::data_dir().join("param-cache.json");
    let text = std::fs::read_to_string(&cache_path).ok()?;
    let v = crate::json::parse(&text).ok()?;
    deserialize(&v)
}

fn run_capture(exe: &str, args: &[&str]) -> Option<String> {
    use std::process::{Command, Stdio};
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW：不要闪黑框
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().ok()?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    if s.trim().is_empty() {
        s = String::from_utf8_lossy(&out.stderr).to_string();
    }
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

// ---------------- 缓存读写 ----------------

fn load_cache(path: &Path, exe: &str, mtime: u64, size: u64) -> Option<HelpDoc> {
    let text = std::fs::read_to_string(path).ok()?;
    let v = crate::json::parse(&text).ok()?;
    if v.s("exe", "") != exe || v.i("mtime", 0) as u64 != mtime || v.i("size", 0) as u64 != size {
        return None;
    }
    deserialize(&v)
}

fn save_cache(path: &Path, exe: &str, mtime: u64, size: u64, doc: &HelpDoc) {
    let mut o = Json::obj();
    o.set("exe", Json::str(exe));
    o.set("mtime", Json::Num(mtime as f64));
    o.set("size", Json::Num(size as f64));
    o.set("version", Json::str(&doc.version));
    o.set("helpHash", Json::str(&doc.help_hash));
    let mut arr = Vec::new();
    for p in doc.params.iter() {
        let mut e = Json::obj();
        e.set("flag", Json::str(&p.flag));
        e.set(
            "aliases",
            Json::Arr(p.aliases.iter().map(|a| Json::str(a)).collect()),
        );
        e.set("value", Json::str(&p.value));
        e.set("kind", Json::str(p.kind));
        e.set(
            "allowed",
            Json::Arr(p.allowed.iter().map(|a| Json::str(a)).collect()),
        );
        e.set("desc", Json::str(&p.desc));
        e.set("default", Json::str(&p.default));
        e.set("env", Json::str(&p.env));
        e.set("section", Json::str(&p.section));
        arr.push(e);
    }
    o.set("params", Json::Arr(arr));
    o.set("parsedAt", Json::Num(crate::util::now_secs() as f64));
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
    let _ = std::fs::write(path, o.to_pretty());
}

fn deserialize(v: &Json) -> Option<HelpDoc> {
    let params = v.path("params")?.as_arr()?;
    let mut doc = HelpDoc {
        version: v.s("version", ""),
        help_hash: v.s("helpHash", ""),
        params: Vec::new(),
    };
    for p in params.iter() {
        doc.params.push(ParamSpec {
            flag: p.s("flag", ""),
            aliases: p
                .path("aliases")
                .and_then(|a| a.as_arr())
                .map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            value: p.s("value", ""),
            kind: leak_kind(&p.s("kind", "text")),
            allowed: p
                .path("allowed")
                .and_then(|a| a.as_arr())
                .map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            desc: p.s("desc", ""),
            default: p.s("default", ""),
            env: p.s("env", ""),
            section: p.s("section", ""),
        });
    }
    Some(doc)
}

/// kind 只需 4 个固定字符串，做成 static 以免到处分配
fn leak_kind(k: &str) -> &'static str {
    match k {
        "bool" => "bool",
        "int" => "int",
        "enum" => "enum",
        _ => "text",
    }
}

// ---------------- 自动发现：本程序还不认识的参数 ----------------

/// 这些参数对"启动服务器"没有意义（或由本程序自己管理），不放进自动发现分区
const AUTO_DENY: &[&str] = &[
    "-h", "--help", "--usage", "--version", "--cache-list", "--completion-bash",
    "-m", "--model", "--mmproj", "--host", "--port", "--api-key",
];

/// 返回"当前 llama-server 有、但我们还没建模"的参数（用于高级设置的自动发现分区）
pub fn auto_params() -> Vec<ParamSpec> {
    let Some(doc) = current() else {
        return Vec::new();
    };
    let known = known_flags();
    let mut out: Vec<ParamSpec> = doc
        .params
        .iter()
        .filter(|p| {
            !known.iter().any(|k| k == &p.flag)
                && !AUTO_DENY.iter().any(|d| *d == p.flag)
                && !p.flag.is_empty()
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| a.flag.cmp(&b.flag));
    out
}

/// 取某个 flag 的可选值（供自动发现分区的下拉框使用）
pub fn param_options(flag: &str) -> Vec<String> {
    current()
        .and_then(|d| d.find(flag))
        .map(|p| p.allowed.clone())
        .unwrap_or_default()
}

// ---------------- 发射前校验 ----------------

/// 按参数表过滤将要发射的参数：
/// * 本程序认识、但新版 llama-server 已经没有的 flag → 丢弃（连同它的值）并记录
/// * 完全没见过的 flag → 保留，但记录一条警告
pub fn filter_args(args: Vec<String>) -> (Vec<String>, Vec<String>) {
    let Some(doc) = current() else {
        return (args, Vec::new());
    };
    let known = known_flags();
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut warnings: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        let tok = args[i].clone();
        if tok.starts_with('-') {
            let name = tok.split('=').next().unwrap_or(&tok).to_string();
            let has_value = i + 1 < args.len() && !args[i + 1].starts_with('-');
            match doc.find(&name) {
                Some(_) => {
                    out.push(tok);
                    if has_value {
                        out.push(args[i + 1].clone());
                        i += 1;
                    }
                }
                None => {
                    let was_known = known.iter().any(|k| k == &name);
                    if was_known {
                        warnings.push(format!("{} 在当前 llama-server 中已不存在，已忽略", name));
                    } else {
                        warnings.push(format!("{} 不在当前 llama-server 参数表中", name));
                        out.push(tok);
                        if has_value {
                            out.push(args[i + 1].clone());
                            i += 1;
                        }
                    }
                }
            }
        } else {
            out.push(tok);
        }
        i += 1;
    }
    (out, warnings)
}
