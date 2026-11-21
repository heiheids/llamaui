//! 模型库：目录扫描、导入列表持久化、选择状态
//! Model library: directory scanning, persisted import list, selection state.

use crate::json::{self, Json};
use crate::util;
use std::fs;
use std::path::{Path, PathBuf};

const LIB_FILE: &str = "models.json";
const MAX_DEPTH: usize = 4;
const MAX_SCAN_FILES: usize = 4000;

/// 扫描同步的结果
#[derive(Default, Clone, Debug)]
pub struct SyncReport {
    pub added: usize,
    pub removed: usize,
    pub updated: usize,
    pub removed_names: Vec<String>,
    /// 因为「不是对话模型」（扩散/图像等）而被跳过或移除的条目
    pub skipped: usize,
    pub skipped_names: Vec<String>,
}

/// 导入结果
#[derive(Default, Clone, Debug)]
pub struct ImportReport {
    pub added: usize,
    pub skipped: usize,
    pub skipped_names: Vec<String>,
}

#[derive(Clone)]
pub struct Model {
    pub id: String,
    /// 是否 MoE（专家混合）模型
    pub moe: bool,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub added_at: u64,
    /// 是否为识图（多模态）模型：同目录存在 mmproj 或文件名含视觉关键词
    pub vision: bool,
    /// 上次加载失败（如架构不被支持）：列表里标红
    pub unsupported: bool,
    /// 模型原生上下文长度（来自 GGUF 元数据，0 表示未知）
    pub ctx_native: u32,
    /// 配套的视觉投影文件（mmproj）路径；空表示不是识图模型
    pub mmproj: String,
}

pub struct Library {
    pub items: Vec<Model>,
    path: PathBuf,
}

impl Library {
    pub fn load() -> Library {
        let path = util::data_dir().join(LIB_FILE);
        let mut items = Vec::new();
        if let Some(text) = util::read_text(&path) {
            if let Ok(v) = json::parse(&text) {
                if let Some(arr) = v.get("items").and_then(|a| a.as_arr()) {
                    for o in arr {
                        let p = o.s("path", "");
                        if p.is_empty() {
                            continue;
                        }
                        items.push(Model {
                            id: o.s("id", ""),
                            name: o.s("name", ""),
                            size: o.i("size", 0) as u64,
                            added_at: o.i("addedAt", 0) as u64,
                            moe: o.b("moe", false),
                            vision: o.b("vision", false),
                            unsupported: o.b("unsupported", false),
                            ctx_native: o.i("ctxNative", 0) as u32,
                            mmproj: o.s("mmproj", ""),
                            path: p,
                        });
                    }
                }
            }
        }
        // 补齐缺失字段 / 过滤已删除文件（相对路径按工程根解析后再判断）
        items.retain(|m| Path::new(&util::absolutize(&m.path)).is_file());
        for m in items.iter_mut() {
            if m.id.is_empty() {
                m.id = make_id(&m.path);
            }
            if m.name.is_empty() {
                m.name = file_stem(&m.path);
            }
            m.size = fs::metadata(util::absolutize(&m.path))
                .map(|md| md.len())
                .unwrap_or(m.size);
        }
        Library { items, path }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let mut items = Vec::new();
        for m in &self.items {
            let mut o = Json::obj();
            o.set("id", Json::str(&m.id));
            o.set("name", Json::str(&m.name));
            o.set("path", Json::str(&m.path));
            o.set("size", Json::Num(m.size as f64));
            o.set("addedAt", Json::Num(m.added_at as f64));
            o.set("moe", Json::Bool(m.moe));
o.set("vision", Json::Bool(m.vision));
            o.set("unsupported", Json::Bool(m.unsupported));
            o.set("ctxNative", Json::Num(m.ctx_native as f64));
            o.set("mmproj", Json::str(&m.mmproj));
            items.push(o);
        }
        let mut root = Json::obj();
        root.set("items", Json::Arr(items));
        util::write_atomic(&self.path, &root.to_pretty())
    }

    pub fn find(&self, id: &str) -> Option<&Model> {
        self.items.iter().find(|m| m.id == id)
    }

    pub fn list_json(&self) -> Json {
        let arr = self
            .items
            .iter()
            .map(|m| {
                let mut o = Json::obj();
                o.set("id", Json::str(&m.id));
                o.set("name", Json::str(&m.name));
                o.set("path", Json::str(&m.path));
                o.set("size", Json::Num(m.size as f64));
                o.set("sizeText", Json::str(util::human_size(m.size)));
                o.set("exists", Json::Bool(Path::new(&util::absolutize(&m.path)).is_file()));
                o.set("moe", Json::Bool(m.moe));
o.set("vision", Json::Bool(m.vision));
                o.set("unsupported", Json::Bool(m.unsupported));
            o.set("ctxNative", Json::Num(m.ctx_native as f64));
            o.set("mmproj", Json::str(&m.mmproj));
                o.set("addedAt", Json::Num(m.added_at as f64));
                o
            })
            .collect();
        Json::Arr(arr)
    }

    /// 导入一个或多个路径；目录则递归扫描（含子文件夹）
    pub fn import_paths(&mut self, paths: &[String]) -> ImportReport {
        let mut found: Vec<PathBuf> = Vec::new();
        for p in paths {
            let pb = PathBuf::from(p);
            if pb.is_dir() {
                scan_dir(&pb, 0, &mut found);
            } else if pb.is_file() && is_gguf(&pb) {
                found.push(pb);
            }
        }
        let mut rep = ImportReport::default();
        for f in found {
            // mmproj 是视觉投影文件，属于配套资源而不是独立模型
            if is_projector(&f) {
                continue;
            }
            let path = util::relativize(&f.to_string_lossy().to_string());
            if self.items.iter().any(|m| m.path.eq_ignore_ascii_case(&path)) {
                continue;
            }
            // 读一下 GGUF 头：拿原生上下文，同时判断是不是对话模型
            let info = crate::gguf::read(&f.to_string_lossy());
            if let Some(i) = info.as_ref() {
                if !crate::gguf::is_chat_model(i) {
                    // 扩散/图像/视频模型：llama.cpp 跑不了，不入库
                    rep.skipped += 1;
                    rep.skipped_names.push(file_stem(&path));
                    continue;
                }
            }
            let size = fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
            let vision = looks_like_vision(&f);
            let ctx_native = info.as_ref().map(|i| i.ctx_native).unwrap_or(0);
            // 同目录的视觉投影文件（识图模型启动时必须带上）
            let mmproj = find_mmproj(&f)
                .map(|p| util::relativize(&p.to_string_lossy().to_string()))
                .unwrap_or_default();
            self.items.push(Model {
                id: make_id(&path),
                moe: info.as_ref().map(|i| i.is_moe()).unwrap_or(false),
                name: file_stem(&path),
                path,
                size,
                added_at: util::now_secs(),
                vision,
                unsupported: false,
                ctx_native,
                mmproj,
            });
            rep.added += 1;
        }
        rep
    }

    /// 与磁盘对齐：删除文件已不存在的条目、刷新已存在条目的元数据。
    /// 只在「父目录仍可访问」时才判为失效，避免移动硬盘/网络盘离线时误删。
    pub fn sync_with_disk(&mut self) -> SyncReport {
        let mut report = SyncReport::default();
        let mut keep: Vec<Model> = Vec::with_capacity(self.items.len());
        for mut m in std::mem::take(&mut self.items) {
            let abs = util::absolutize(&m.path);
            let p = Path::new(&abs);
            if !p.is_file() {
                // 父目录都打不开 → 认为是盘/目录离线，保留条目
                let parent_ok = p.parent().map(|d| d.is_dir()).unwrap_or(false);
                if parent_ok {
                    report.removed += 1;
                    report.removed_names.push(m.name.clone());
                    continue;
                }
                keep.push(m);
                continue;
            }
            // 文件还在：如果是扩散/图像模型（不应入库的），顺手清掉
            if let Some(info) = crate::gguf::read(&abs) {
                if !crate::gguf::is_chat_model(&info) {
                    report.skipped += 1;
                    report.skipped_names.push(m.name.clone());
                    continue;
                }
            }
            let mut changed = false;
            // 大小
            if let Ok(meta) = fs::metadata(p) {
                let size = meta.len();
                if size != m.size {
                    m.size = size;
                    changed = true;
                }
            }
            // 视觉投影文件（同目录新增/删除了 mmproj）
            let new_mmproj = find_mmproj(p)
                .map(|q| util::relativize(&q.to_string_lossy().to_string()))
                .unwrap_or_default();
            if !new_mmproj.eq_ignore_ascii_case(&m.mmproj) {
                m.mmproj = new_mmproj.clone();
                changed = true;
            }
            let vision = looks_like_vision(p) || !m.mmproj.is_empty();
            if vision != m.vision {
                m.vision = vision;
                changed = true;
            }
            // 原生上下文长度（之前读失败的补一次）
            if m.ctx_native == 0 {
                if let Some(info) = crate::gguf::read(&abs) {
                    if info.ctx_native > 0 {
                        m.ctx_native = info.ctx_native;
                        changed = true;
                    }
                }
            }
            if changed {
                report.updated += 1;
            }
            keep.push(m);
        }
        self.items = keep;
        report
    }

    /// 补齐缺失的原生上下文长度与 mmproj 路径（老库或手动导入的条目）
    pub fn fill_native_ctx(&mut self) -> bool {
        let mut changed = false;
        for m in self.items.iter_mut() {
            if m.ctx_native == 0 {
                let abs = util::absolutize(&m.path);
                if let Some(info) = crate::gguf::read(&abs) {
                    if info.ctx_native > 0 {
                        m.ctx_native = info.ctx_native;
                        changed = true;
                    }
                }
            }
            if m.mmproj.is_empty() {
                let abs = util::absolutize(&m.path);
                if let Some(p) = find_mmproj(Path::new(&abs)) {
                    m.mmproj = util::relativize(&p.to_string_lossy().to_string());
                    m.vision = true;
                    changed = true;
                }
            }
        }
        changed
    }

    /// 标记某模型“加载失败/架构不支持”，返回是否有变化
    pub fn set_unsupported(&mut self, id: &str, flag: bool) -> bool {
        let mut changed = false;
        for m in self.items.iter_mut() {
            if m.id == id && m.unsupported != flag {
                m.unsupported = flag;
                changed = true;
            }
        }
        changed
    }

    /// 重新判定所有条目的识图标记（文件增删后调用）
    pub fn refresh_vision_flags(&mut self) -> bool {
        let mut changed = false;
        for m in self.items.iter_mut() {
            let v = looks_like_vision(Path::new(&util::absolutize(&m.path)));
            if v != m.vision {
                m.vision = v;
                changed = true;
            }
        }
        changed
    }

    pub fn refresh_moe_flags(&mut self) -> bool {
        let mut changed = false;
        for m in self.items.iter_mut() {
            let abs = util::absolutize(&m.path);
            if let Some(info) = crate::gguf::read(&abs) {
                let v = info.is_moe();
                if v != m.moe {
                    m.moe = v;
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|m| m.id != id);
        self.items.len() != before
    }

    /// 默认扫描位置：程序目录下的 models / gguf，以及**任何含 llama-server 的子目录**。
    /// 返回相对路径（相对程序所在目录）；不写死版本号，换新版目录名也能识别。
    pub fn default_scan_dirs() -> Vec<String> {
        let base = util::base_dir();
        let mut dirs = Vec::new();
        for name in ["models", "gguf"] {
            if base.join(name).is_dir() {
                dirs.push(name.to_string());
            }
        }
        dirs.extend(llama_dirs());
        dirs
    }
}

/// 相对路径 = 程序目录内的目录（不可删除）
pub fn is_builtin_scan_dir(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty() && !std::path::Path::new(t).is_absolute()
}

/// 找出程序目录下所有含 llama-server 可执行文件的子目录（相对名，已排序）
pub fn llama_dirs() -> Vec<String> {
    let base = util::base_dir();
    let exe = if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    };
    let mut names: Vec<String> = Vec::new();
    let Ok(rd) = fs::read_dir(&base) else {
        return names;
    };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let p = e.path();
        if p.join(exe).is_file() || p.join("bin").join(exe).is_file() {
            names.push(name);
        }
    }
    names.sort();
    names
}

fn is_gguf(p: &Path) -> bool {
    p.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("gguf"))
        .unwrap_or(false)
}

/// 是否为视觉投影文件（mmproj-*.gguf 之类）
fn is_projector(p: &Path) -> bool {
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name.starts_with("mmproj") || name.contains("mmproj")
}

/// 在模型同目录（以及上一级）里找视觉投影文件，返回最合适的一个
pub fn find_mmproj(model: &Path) -> Option<std::path::PathBuf> {
    let dir = model.parent()?;
    for d in [dir, dir.parent().unwrap_or(dir)] {
        let mut best: Option<std::path::PathBuf> = None;
        let mut best_len: u64 = u64::MAX;
        if let Ok(rd) = fs::read_dir(d) {
            for e in rd.flatten() {
                let p = e.path();
                if !p.is_file() {
                    continue;
                }
                let name = e.file_name().to_string_lossy().to_lowercase();
                if !name.contains("mmproj") || !name.ends_with(".gguf") {
                    continue;
                }
                // 多个候选取最小的（通常是 F16，比 BF16/F32 稳定）
                let len = fs::metadata(&p).map(|m| m.len()).unwrap_or(u64::MAX);
                if best.is_none() || len < best_len {
                    best = Some(p);
                    best_len = len;
                }
            }
        }
        if best.is_some() {
            return best;
        }
    }
    None
}

/// 判断是否为识图（多模态）模型：
/// 1) 同目录存在 mmproj 投影文件；2) 文件名含常见视觉模型关键词。
fn looks_like_vision(p: &Path) -> bool {
    // 1) 同目录的 mmproj
    if let Some(dir) = p.parent() {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if name.contains("mmproj") && name.ends_with(".gguf") {
                    return true;
                }
            }
        }
    }
    // 2) 名称关键词
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    const KEYS: &[&str] = &[
        "vision",
        "llava",
        "minicpm-v",
        "qwen-vl",
        "qwen2-vl",
        "qwen2.5-vl",
        "qwen3-vl",
        "moondream",
        "internvl",
        "cogvlm",
        "glm-4v",
        "pixtral",
        "idefics",
        "smolvlm",
        "gemma-3",
        "paligemma",
        "phi-3-vision",
        "phi-4-multimodal",
        "-vl-",
        "vl-",
    ];
    KEYS.iter().any(|k| stem.contains(k))
}

fn file_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn make_id(path: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in path.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("m{:016x}", h)
}

fn scan_dir(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_DEPTH || out.len() >= MAX_SCAN_FILES {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        if out.len() >= MAX_SCAN_FILES {
            return;
        }
        let p = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            // 跳过隐藏目录与常见无关目录，减少无谓遍历
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            scan_dir(&p, depth + 1, out);
        } else if ft.is_file() && is_gguf(&p) {
            out.push(p);
        }
    }
}
