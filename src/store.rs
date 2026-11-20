//! 配置存储：加载 / 合并默认值 / 原子保存
//! Config store: load, merge with defaults, atomic save.

use crate::json::{self, Json};
use crate::util;
use std::path::PathBuf;

pub struct Store {
    pub cfg: Json,
    path: PathBuf,
}

impl Store {
    pub fn load() -> Store {
        let path = util::data_dir().join("config.json");
        let mut cfg = load_or_default(&path);
        ensure_api_key(&mut cfg);
        ensure_host(&mut cfg);
        let st = Store { cfg, path };
        // 首次运行或旧版本缺键时立即落盘一次
        let _ = st.save();
        st
    }

    pub fn save(&self) -> std::io::Result<()> {
        util::write_atomic(&self.path, &self.cfg.to_pretty())
    }

    /// 用前端提交的部分配置进行深度合并（只接受 schema 中存在的键，防止越权写入）
    pub fn merge_update(&mut self, patch: &Json) -> Result<(), String> {
        let mut failed = Vec::new();
        if let Json::Obj(map) = patch {
            for (group, val) in map.iter() {
                for (key, v) in walk(group, val) {
                    if is_known_key(&key) || is_app_key(&key) {
                        self.cfg.set_path(&key, v);
                    } else {
                        failed.push(key);
                    }
                }
            }
        } else {
            return Err("err.badPayload".into());
        }
        normalize(&mut self.cfg);
        if !failed.is_empty() {
            return Err(format!("err.unknownKey:{}", failed.join(",")));
        }
        Ok(())
    }

    pub fn selected_model(&self) -> String {
        self.cfg.s("app.selectedModel", "")
    }

    pub fn set_selected(&mut self, id: &str) {
        self.cfg.set_path("app.selectedModel", Json::str(id));
    }
}

fn walk(prefix: &str, val: &Json) -> Vec<(String, Json)> {
    match val {
        Json::Obj(m) if !m.is_empty() => {
            let mut out = Vec::new();
            for (k, v) in m.iter() {
                let p = format!("{}.{}", prefix, k);
                out.extend(walk(&p, v));
            }
            out
        }
        other => vec![(prefix.to_string(), other.clone())],
    }
}

fn is_known_key(key: &str) -> bool {
    crate::settings::FIELDS.iter().any(|f| f.key == key)
}

fn is_app_key(key: &str) -> bool {
    matches!(
        key,
        "app.lang"
            | "app.autoOpenBrowser"
            | "app.panelPort"
            | "app.llamaExe"
            | "app.selectedModel"
            | "app.scanDirs"
    )
}

fn ensure_api_key(cfg: &mut Json) {
    let empty = crate::settings::api_keys(cfg).is_empty();
    if empty {
        cfg.set_path(
            "network.apiKeys",
            Json::Arr(vec![Json::str(util::new_api_key())]),
        );
    }
}

/// 绑定地址默认值：旧配置里为空时补成 127.0.0.1
fn ensure_host(cfg: &mut Json) {
    let cur = cfg.s("network.host", "");
    if cur.trim().is_empty() {
        cfg.set_path("network.host", Json::str("127.0.0.1"));
    }
}

/// 数值范围与空文本整理
fn normalize(cfg: &mut Json) {
    for f in crate::settings::FIELDS {
        let Some(cur) = cfg.path(f.key).cloned() else {
            continue;
        };
        match f.kind {
            crate::settings::Kind::Int | crate::settings::Kind::Float => {
                let mut n = cur.as_f64().unwrap_or(0.0);
                if let Some(min) = f.min {
                    n = n.max(min);
                }
                if let Some(max) = f.max {
                    n = n.min(max);
                }
                if f.kind == crate::settings::Kind::Int {
                    n = n.round();
                }
                cfg.set_path(f.key, Json::Num(n));
            }
            _ => {}
        }
    }
    // 端口冲突保护：面板端口不能与 llama-server 端口相同
    let panel = cfg.i("app.panelPort", 8790);
    let server = cfg.i("network.port", 9931);
    if panel == server {
        cfg.set_path("app.panelPort", Json::Num((panel + 1).min(65535) as f64));
    }
}

fn load_or_default(path: &PathBuf) -> Json {
    let mut cfg = crate::settings::defaults();
    if let Some(text) = util::read_text(path) {
        match json::parse(&text) {
            Ok(v) => deep_merge(&mut cfg, &v),
            Err(e) => {
                let _ = util::write_atomic(
                    &path.with_extension("broken.json"),
                    &format!("parse error: {}\n\n{}", e, text),
                );
            }
        }
    }
    cfg
}

/// 将存档深度合并进默认值（存档优先）
pub fn deep_merge(base: &mut Json, patch: &Json) {
    if let (Json::Obj(b), Json::Obj(p)) = (&mut *base, patch) {
        for (k, v) in p.iter() {
            match b.get_mut(k) {
                Some(Json::Obj(_)) if matches!(v, Json::Obj(_)) => {
                    let mut child = b.get(k).cloned().unwrap_or_else(Json::obj);
                    deep_merge(&mut child, v);
                    b.insert(k.clone(), child);
                }
                _ => {
                    b.insert(k.clone(), v.clone());
                }
            }
        }
    } else {
        *base = patch.clone();
    }
}
