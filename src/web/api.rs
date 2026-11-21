//! API 路由与处理逻辑
//! HTTP API routes and handlers.

use super::client;
use super::http::{Request, Response};
use crate::i18n;
use crate::json::{self, Json};
use crate::models::Library;
use crate::process::Launcher;
use crate::settings;
use crate::store::Store;
use crate::util;
use std::sync::{Arc, Mutex};

pub struct App {
    pub store: Store,
    pub lib: Library,
    pub launcher: Launcher,
}

pub type Shared = Arc<Mutex<App>>;

impl App {
    pub fn new() -> App {
        let store = Store::load();
        let lib = Library::load();
        let mut app = App {
            store,
            lib,
            launcher: Launcher::new(),
        };
        // 启动时若未选择模型，则默认选中库中第一个，避免"空选择"状态
        if app.store.selected_model().is_empty() {
            if let Some(first) = app.lib.items.first().map(|m| m.id.clone()) {
                app.store.set_selected(&first);
                let _ = app.store.save();
            }
        }
        app
    }

    fn lang(&self) -> String {
        i18n::resolve(&self.store.cfg)
    }
}

pub fn handle(shared: &Shared, req: &Request) -> Response {
    let Ok(mut app) = shared.lock() else {
        return err("err.lock", "en-US", "zh-CN");
    };
    let lang = app.lang();
    let path = req.path.trim_end_matches('/').to_string();
    let path = if path.is_empty() { "/".to_string() } else { path };
    let method = req.method.as_str();

    match (method, path.as_str()) {
        ("GET", "/api/bootstrap") => bootstrap(&mut app, &lang),
        ("GET", "/api/status") => {
            let since = req
                .query
                .get("since")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            status_payload(&app, since)
        }
        ("GET", "/api/models") => ok({
            let mut o = Json::obj();
            o.set("models", app.lib.list_json());
            o.set("selected", Json::str(app.store.selected_model()));
            o.set("scanDirs", scan_dirs(&app));
            o
        }),
        ("POST", "/api/models/import") => import_models(&mut app, req, &lang),
        ("POST", "/api/models/scan") => scan_models(&mut app, &lang),
        ("POST", "/api/models/remove") => remove_model(&mut app, req),
        ("POST", "/api/models/select") => select_model(&mut app, req),
        ("POST", "/api/config") => save_config(&mut app, req, &lang),
        ("POST", "/api/keys/new") => new_key(&mut app),
        ("POST", "/api/server/start") => start_server(&mut app, req, &lang),
        ("POST", "/api/server/stop") => {
            app.launcher.stop();
            app.launcher.push_log("server stopped by panel", "");
            ok(Json::Bool(true))
        }
        ("POST", "/api/chat") => proxy_chat(&app, req, &lang),
        ("GET", "/api/log") => {
            let since = req
                .query
                .get("since")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            let mut o = Json::obj();
            o.set("lines", Json::Arr(app.launcher.snapshot_logs(since).into_iter().map(Json::Str).collect()));
            o.set("total", Json::Num(app.launcher.log_count() as f64));
            ok(o)
        }
        ("POST", "/api/log/clear") => {
            if let Ok(mut logs) = app.launcher.logs.lock() {
                logs.clear();
            }
            ok(Json::Bool(true))
        }
        ("GET", "/api/preview") => {
            let model = app.store.selected_model();
            let path = app.lib.find(&model).map(|m| m.path.clone()).unwrap_or_default();
            let mut o = Json::obj();
            o.set("args", Json::str(settings::preview_args(&app.store.cfg, &path)));
            o.set("exe", Json::str(
                Launcher::resolve_exe(&app.store.cfg)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ));
            ok(o)
        }
        _ => err_localized("err.methodNotAllowed", &lang),
    }
}

fn ok(data: Json) -> Response {
    let mut o = Json::obj();
    o.set("ok", Json::Bool(true));
    o.set("data", data);
    Response::json(&o, 200)
}

fn err(code: &str, lang_a: &str, lang_b: &str) -> Response {
    let lang = if util::normalize_lang(lang_a) == "zh-CN" { lang_a } else { lang_b };
    err_localized(code, lang)
}

fn err_localized(code: &str, lang: &str) -> Response {
    let mut o = Json::obj();
    o.set("ok", Json::Bool(false));
    o.set("code", Json::str(code));
    o.set("message", Json::str(i18n::t(lang, code)));
    Response::json(&o, 400)
}

fn bootstrap(app: &mut App, _lang: &str) -> Response {
    let lang = app.lang();
    let mut o = Json::obj();
    // 聊天页据此判断能否发图片：开启识图且模型带 mmproj
    let vision = app.store.cfg.b("launch.vision", true)
        && !app.store.cfg.s("launch.mmproj", "").trim().is_empty();
    let mut cfg = app.store.cfg.clone();
    cfg.set("vision", Json::Bool(vision));
    o.set("config", cfg);
    o.set("schema", settings::schema_json());
    o.set("models", app.lib.list_json());
    o.set("selected", Json::str(app.store.selected_model()));
    o.set("lang", Json::str(lang.clone()));
    o.set("i18n", i18n::bundle_json(&lang));
    o.set("languages", i18n::bundle_json(&lang).get("available").cloned().unwrap_or(Json::Arr(vec![])));
    o.set("status", app.launcher.status_json(&app.store.cfg));
    o.set("scanDirs", scan_dirs(app));
    o.set("systemLang", Json::str(util::detect_system_lang()));
    o.set("cpuThreads", Json::Num(util::cpu_threads() as f64));
    o.set("version", Json::str(env!("CARGO_PKG_VERSION")));
    ok(o)
}

fn status_payload(app: &App, since: usize) -> Response {
    let mut o = Json::obj();
    o.set("status", app.launcher.status_json(&app.store.cfg));
    o.set("lines", Json::Arr(app.launcher.snapshot_logs(since).into_iter().map(Json::Str).collect()));
    o.set("total", Json::Num(app.launcher.log_count() as f64));
    o.set("selected", Json::str(app.store.selected_model()));
    ok(o)
}

fn scan_dirs(app: &App) -> Json {
    let mut dirs: Vec<String> = app
        .store
        .cfg
        .path("app.scanDirs")
        .and_then(|v| v.as_arr())
        .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_default();
    for d in Library::default_scan_dirs() {
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }
    Json::Arr(dirs.into_iter().map(Json::Str).collect())
}

/// 首次导入后自动选中首个模型，减少一次手动点击
fn auto_select(app: &mut App) {
    if !app.store.selected_model().is_empty() {
        return;
    }
    if let Some(first) = app.lib.items.first() {
        let id = first.id.clone();
        app.store.set_selected(&id);
        let _ = app.store.save();
    }
}

fn import_models(app: &mut App, req: &Request, lang: &str) -> Response {
    let body = match json::parse(&req.body_text()) {
        Ok(v) => v,
        Err(_) => return err_localized("err.badPayload", lang),
    };
    let paths: Vec<String> = body
        .get("paths")
        .and_then(|v| v.as_arr())
        .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_default();
    if paths.is_empty() {
        return err_localized("err.badPayload", lang);
    }
    let added = app.lib.import_paths(&paths).added;
    let _ = app.lib.save();
    auto_select(app);
    let mut o = Json::obj();
    o.set("added", Json::Num(added as f64));
    o.set("models", app.lib.list_json());
    o.set("selected", Json::str(app.store.selected_model()));
    ok(o)
}

fn scan_models(app: &mut App, lang: &str) -> Response {
    let dirs: Vec<String> = scan_dirs(app)
        .as_arr()
        .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_default();
    let mut added = 0usize;
    for d in dirs {
        let real = crate::util::resolve_dir(&d);
        if real.is_dir() {
            added += app.lib.import_paths(&[real.to_string_lossy().to_string()]).added;
        }
    }
    // 与磁盘对齐：清理失效条目、刷新元数据
    let report = app.lib.sync_with_disk();
    let _ = app.lib.save();
    auto_select(app);
    let mut o = Json::obj();
    o.set("added", Json::Num(added as f64));
    o.set("removed", Json::Num(report.removed as f64));
    o.set("updated", Json::Num(report.updated as f64));
    o.set("models", app.lib.list_json());
    o.set("selected", Json::str(app.store.selected_model()));
    o.set("message", Json::str(format!("{}", added)));
    if added == 0 && app.lib.items.is_empty() {
        o.set("hint", Json::str(i18n::t(lang, "label.emptyModels")));
    }
    ok(o)
}

fn remove_model(app: &mut App, req: &Request) -> Response {
    let body = json::parse(&req.body_text()).unwrap_or(Json::Null);
    let id = body.s("id", "");
    if id.is_empty() {
        return err_localized("err.badPayload", &app.lang());
    }
    app.lib.remove(&id);
    let _ = app.lib.save();
    if app.store.selected_model() == id {
        app.store.set_selected("");
        let _ = app.store.save();
    }
    ok(app.lib.list_json())
}

fn select_model(app: &mut App, req: &Request) -> Response {
    let body = json::parse(&req.body_text()).unwrap_or(Json::Null);
    let id = body.s("id", "");
    if app.lib.find(&id).is_none() {
        return err_localized("err.modelMissing", &app.lang());
    }
    app.store.set_selected(&id);
    let _ = app.store.save();
    ok(Json::str(id))
}

fn save_config(app: &mut App, req: &Request, lang: &str) -> Response {
    let body = match json::parse(&req.body_text()) {
        Ok(v) => v,
        Err(_) => return err_localized("err.badPayload", lang),
    };
    if let Err(e) = app.store.merge_update(&body) {
        let code = e.split(':').next().unwrap_or("err.badPayload").to_string();
        return err_localized(&code, lang);
    }
    if let Err(_) = app.store.save() {
        return err_localized("err.io", lang);
    }
    let mut o = Json::obj();
    o.set("config", app.store.cfg.clone());
    ok(o)
}

fn new_key(app: &mut App) -> Response {
    let key = util::new_api_key();
    let mut keys = settings::api_keys(&app.store.cfg);
    keys.push(key.clone());
    app.store.cfg.set_path(
        "network.apiKeys",
        Json::Arr(keys.iter().map(|k| Json::Str(k.clone())).collect()),
    );
    let _ = app.store.save();
    let mut o = Json::obj();
    o.set("key", Json::str(key));
    o.set("config", app.store.cfg.clone());
    ok(o)
}

fn start_server(app: &mut App, req: &Request, lang: &str) -> Response {
    let body = json::parse(&req.body_text()).unwrap_or(Json::Null);
    let id = {
        let requested = body.s("id", "");
        if requested.is_empty() {
            app.store.selected_model()
        } else {
            requested
        }
    };
    if id.is_empty() {
        return err_localized("err.modelNotSelected", lang);
    }
    let Some(model) = app.lib.find(&id).cloned() else {
        return err_localized("err.modelMissing", lang);
    };
    if !std::path::Path::new(&model.path).is_file() {
        return err_localized("err.modelMissing", lang);
    }
    app.store.set_selected(&id);
    let _ = app.store.save();
    let cfg = app.store.cfg.clone();
    // 启动前转为绝对路径：llama-server 的工作目录是它自身所在目录
    let abs_model = util::absolutize(&model.path);
    match app.launcher.start(&cfg, &abs_model) {
        Ok(_) => {
            let mut o = Json::obj();
            o.set("status", app.launcher.status_json(&app.store.cfg));
            ok(o)
        }
        Err(e) => {
            let code = e.split(':').next().unwrap_or("err.spawnFailed").to_string();
            app.launcher.push_log(&format!("start failed: {}", e), "");
            err_localized(&code, lang)
        }
    }
}

/// 代理聊天请求到 llama-server，避免浏览器跨域问题
fn proxy_chat(app: &App, req: &Request, lang: &str) -> Response {
    let body = req.body_text();
    let parsed = match json::parse(&body) {
        Ok(v) => v,
        Err(_) => return err_localized("err.badPayload", lang),
    };
    let mut payload = Json::obj();
    let model = app.store.selected_model();
    let alias = app.store.cfg.s("launch.alias", "");
    let model_name = if !alias.is_empty() {
        alias
    } else {
        app.lib.find(&model).map(|m| m.path.clone()).unwrap_or_default()
    };
    payload.set("model", Json::str(model_name));
    payload.set(
        "messages",
        parsed.get("messages").cloned().unwrap_or(Json::Arr(vec![])),
    );
    payload.set("stream", Json::Bool(false));
    for key in ["temperature", "top_p", "top_k", "max_tokens", "min_p", "repeat_penalty"] {
        if let Some(v) = parsed.get(key) {
            payload.set(key, v.clone());
        }
    }

    let port = app.store.cfg.i("network.port", 9931) as u16;
    let keys = settings::api_keys(&app.store.cfg);
    let key = if app.store.cfg.b("network.apiKeyEnabled", true) {
        keys.first().cloned()
    } else {
        None
    };

    match client::request(
        "127.0.0.1",
        port,
        "POST",
        "/v1/chat/completions",
        key.as_deref(),
        Some(&payload.to_compact()),
    ) {
        Ok(resp) => {
            let mut parsed_resp = json::parse(&resp.body).unwrap_or(Json::Null);
            if resp.code >= 400 {
                if let Json::Obj(m) = &mut parsed_resp {
                    m.insert("ok".into(), Json::Bool(false));
                    m.insert("code".into(), Json::str("err.upstream"));
                }
                let mut wrapper = Json::obj();
                wrapper.set("ok", Json::Bool(false));
                wrapper.set("code", Json::str("err.upstream"));
                wrapper.set("message", Json::str(resp.body.clone()));
                return Response::json(&wrapper, 502);
            }
            ok(parsed_resp)
        }
        Err(e) => {
            app.launcher.push_log(&format!("chat proxy error: {}", e), "");
            let mut o = Json::obj();
            o.set("ok", Json::Bool(false));
            o.set("code", Json::str("err.network"));
            o.set("message", Json::str(i18n::t(lang, "err.network")));
            Response::json(&o, 502)
        }
    }
}
