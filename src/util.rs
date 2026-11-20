//! 通用工具：路径解析、原子写入、系统语言探测、体积/时间格式化
//! Shared helpers: path resolution, atomic writes, OS locale detection, formatting.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 工作目录 = 程序所在目录（便于整包迁移，避免绝对路径）
/// Base dir = directory of the executable, so the whole folder stays portable.
pub fn base_dir() -> PathBuf {
    if let Ok(raw) = std::env::var("LLAMAUI_HOME") {
        let p = PathBuf::from(raw);
        if p.is_dir() {
            return p;
        }
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    // 开发布局下 exe 位于 target/release，向上寻找工程根（含 Cargo.toml）
    let mut cur = Some(exe_dir.as_path());
    for _ in 0..4 {
        let Some(dir) = cur else { break };
        if dir.join("Cargo.toml").is_file() {
            return dir.to_path_buf();
        }
        cur = dir.parent();
    }
    exe_dir
}

pub fn data_dir() -> PathBuf {
    let p = base_dir().join("data");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn logs_dir() -> PathBuf {
    let p = base_dir().join("logs");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn read_text(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// 追加一行诊断信息到 logs/debug.log（GUI 程序没有控制台，用它排查底层问题）
pub fn append_debug(line: &str) {
    let path = logs_dir().join("debug.log");
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{}", line);
    }
}

/// 原子写：先写临时文件再替换，避免断电/崩溃导致配置损坏
pub fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.flush()?;
    }
    // Windows 上 rename 到已存在文件会失败，先删除
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp, path)
}

/// 探测系统语言，返回 "zh-CN" 或 "en-US"
pub fn detect_system_lang() -> String {
    // 1) 显式环境变量优先
    for key in ["LLAMAUI_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.to_lowercase();
            if !v.is_empty() {
                return normalize_lang(&v);
            }
        }
    }
    // 2) 平台原生接口
    #[cfg(windows)]
    {
        // GetUserDefaultUILanguage: 0x0804 = 简体中文
        if let Some(id) = unsafe { win_lang_id() } {
            return if (id & 0x3ff) == 0x04 { "zh-CN".into() } else { "en-US".into() };
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if s.contains("zh") {
                return "zh-CN".into();
            }
            if s.contains("en") {
                return "en-US".into();
            }
        }
    }
    "en-US".into()
}

#[cfg(windows)]
extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

#[cfg(windows)]
unsafe fn win_lang_id() -> Option<u16> {
    let id = GetUserDefaultUILanguage();
    if id == 0 {
        None
    } else {
        Some(id)
    }
}

pub fn normalize_lang(raw: &str) -> String {
    if raw.starts_with("zh") {
        "zh-CN".into()
    } else {
        "en-US".into()
    }
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut idx = 0;
    while v >= 1024.0 && idx < UNITS.len() - 1 {
        v /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", v, UNITS[idx])
    }
}

pub fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn now_secs() -> u64 {
    (now_ms() / 1000) as u64
}

/// 简单随机数（用于生成 API 密钥），基于时间与进程 id
pub fn pseudo_random_bytes(n: usize) -> Vec<u8> {
    let mut seed = now_ms() as u64 ^ ((std::process::id() as u64) << 32);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        // xorshift64*
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        out.push((seed & 0xff) as u8);
    }
    out
}

pub fn random_hex(n_bytes: usize) -> String {
    pseudo_random_bytes(n_bytes)
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// 生成形如 sk-llamaui-xxxxxxxxxxxxxxxx 的密钥
pub fn new_api_key() -> String {
    format!("sk-llamaui-{}", random_hex(16))
}

/// 计算本机可用 CPU 线程数
pub fn cpu_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// 将相对路径解析为绝对路径（相对工程根），llama-server 工作目录可能不同
pub fn absolutize(path: &str) -> String {
    let p = std::path::PathBuf::from(path);
    if p.is_absolute() {
        return path.to_string();
    }
    let joined = base_dir().join(&p);
    std::fs::canonicalize(&joined)
        .map(|c| c.to_string_lossy().to_string())
        .unwrap_or_else(|_| joined.to_string_lossy().to_string())
}

/// 把扫描目录解析成真实路径：相对路径按程序所在目录解析
pub fn resolve_dir(path: &str) -> PathBuf {
    let p = std::path::PathBuf::from(path);
    if p.is_absolute() {
        return p;
    }
    base_dir().join(p)
}

/// 若路径位于程序目录内，则转成相对路径（用于把绝对扫描目录迁移成相对写法）
pub fn to_relative_if_inside(path: &str) -> String {
    let p = std::path::PathBuf::from(path);
    if !p.is_absolute() {
        return path.to_string();
    }
    let base = base_dir();
    if let Ok(rel) = p.strip_prefix(&base) {
        return rel.to_string_lossy().to_string();
    }
    // 大小写/短路径差异时用 canonicalize 再试一次
    let canon_p = std::fs::canonicalize(&p).unwrap_or(p.clone());
    let canon_b = std::fs::canonicalize(&base).unwrap_or(base);
    match canon_p.strip_prefix(&canon_b) {
        Ok(rel) => rel.to_string_lossy().to_string(),
        Err(_) => path.to_string(),
    }
}

/// 若模型位于工程目录内，则存为相对路径，便于整包迁移
pub fn relativize(path: &str) -> String {
    let abs = std::fs::canonicalize(std::path::Path::new(path))
        .map(|c| c.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string());
    let base = match std::fs::canonicalize(base_dir()) {
        Ok(b) => b.to_string_lossy().to_string(),
        Err(_) => return abs,
    };
    let base = base.trim_end_matches(['\\', '/']).to_string();
    if let Some(rest) = abs.strip_prefix(&(base.clone() + std::path::MAIN_SEPARATOR_STR)) {
        return rest.to_string();
    }
    abs
}

/// 查找监听指定端口的进程号（跨平台，尽力而为）
pub fn pid_listening_on(port: u16) -> Option<u32> {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("netstat")
            .args(["-ano", "-p", "TCP"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 5 && cols[3].eq_ignore_ascii_case("LISTENING") {
                if cols[1].ends_with(&format!(":{}", port)) {
                    if let Ok(pid) = cols[4].parse::<u32>() {
                        if pid != 0 {
                            return Some(pid);
                        }
                    }
                }
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        let out = std::process::Command::new("lsof")
            .args(["-ti", &format!("tcp:{}", port), "-sTCP:LISTEN"])
            .output()
            .ok()?;
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .and_then(|l| l.trim().parse::<u32>().ok())
    }
}

/// 结束指定进程（Windows 结束整棵进程树）
pub fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}
