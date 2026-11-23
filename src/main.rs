//! llamaUI —— llama.cpp 模型加载管理器
//! 作者 黑山大叔 | 主页 https://space.bilibili.com/19428259 | GitHub https://github.com/heiheids
//!
//! 入口：初始化配置与模型库，启动本地管理面板，按需拉起 llama-server。
//! 界面是原生 Windows 窗口程序，不弹出控制台窗口。
#![cfg_attr(windows, windows_subsystem = "windows")]

mod fit;
mod gguf;
mod gui;
mod i18n;
mod json;
mod models;
mod monitor;
mod paramhelp;
mod paramspec;
mod process;
mod settings;
mod shutdown;
mod store;
mod sysinfo;
mod translate;
mod util;
mod vramcal;
mod web;

use std::sync::{Arc, Mutex};

fn main() {
    let mut app = web::api::App::new();

    // 首次运行：把工程目录内的模型文件夹写入扫描目录
    if app
        .store
        .cfg
        .path("app.scanDirs")
        .and_then(|v| v.as_arr())
        .map(|a| a.is_empty())
        .unwrap_or(true)
    {
        let dirs = models::Library::default_scan_dirs();
        if !dirs.is_empty() {
            app.store.cfg.set_path(
                "app.scanDirs",
                json::Json::Arr(dirs.into_iter().map(json::Json::Str).collect()),
            );
            let _ = app.store.save();
        }
    }

    // 若上次退出异常，llama-server 可能仍在监听：直接接管并显示为运行中
    app.launcher.adopt(&app.store.cfg);

    // 补齐模型的原生上下文长度（老库没有这个字段）
    if app.lib.fill_native_ctx() {
        let _ = app.lib.save();
    }

    // 扫描目录：程序目录内的绝对路径迁移成相对写法，并补上自带的默认目录
    {
        let mut dirs: Vec<String> = app
            .store
            .cfg
            .path("app.scanDirs")
            .and_then(|v| v.as_arr())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let before: Vec<String> = dirs.clone();
        // 程序目录内部的绝对路径 → 相对路径
        for d in dirs.iter_mut() {
            *d = util::to_relative_if_inside(d);
        }
        for d in models::Library::default_scan_dirs() {
            if !dirs.iter().any(|x| x.eq_ignore_ascii_case(&d)) {
                dirs.push(d);
            }
        }
        if dirs != before {
            app.store.cfg.set_path(
                "app.scanDirs",
                json::Json::Arr(dirs.iter().map(|d| json::Json::str(d)).collect()),
            );
            let _ = app.store.save();
        }
    }

    // 自动识别 llama-server，并把绝对路径写回配置
    match process::Launcher::autodetect_exe(&mut app.store.cfg) {
        Some(p) => {
            let _ = app.store.save();
            app.launcher
                .push_log(&format!("llama-server: {}", p), "");
            // 参数自描述：解析 --help 并与上一版对比，实现版本自适配
            let prev = paramspec::cached();
            if let Some(doc) = paramspec::discover(&p) {
                let ver = if doc.version.is_empty() {
                    "llama-server".to_string()
                } else {
                    doc.version.clone()
                };
                match prev {
                    Some(old) if old.help_hash != doc.help_hash && !old.params.is_empty() => {
                        let d = paramspec::diff(&old, &doc);
                        app.launcher.push_log(
                            &format!(
                                "检测到 llama-server 参数变化（{}）：新增 {} 个、移除 {} 个、说明变更 {} 个",
                                ver,
                                d.added.len(),
                                d.removed.len(),
                                d.changed.len()
                            ),
                            "",
                        );
                        if !d.added.is_empty() {
                            app.launcher.push_log(
                                &format!("新增参数：{}", d.added.join(" ")),
                                "",
                            );
                            // P3：只在换版本时提示一次，不自动翻译
                            app.launcher.push_log(
                                "新参数的英文说明已解析：可在「高级设置 → 自动发现的新参数」里直接设置，\
                                 并在「参数解读」中查看原文；需要中文时点该窗口的「生成中文解读」",
                                "",
                            );
                        }
                        if !d.removed.is_empty() {
                            app.launcher.push_log(
                                &format!(
                                    "已移除参数：{}（若配置里仍在用，启动时会自动忽略）",
                                    d.removed.join(" ")
                                ),
                                "",
                            );
                        }
                    }
                    _ => {
                        let extra = paramspec::auto_params().len();
                        app.launcher.push_log(
                            &format!(
                                "参数表已解析：{} 个参数，其中 {} 个是本程序尚未建模的新参数（{}）",
                                doc.params.len(),
                                extra,
                                ver
                            ),
                            "",
                        );
                    }
                }
            } else {
                app.launcher
                    .push_log("未能解析 llama-server --help，参数校验暂不可用", "");
            }
        }
        None => {
            app.launcher.push_log(
                "未发现 llama-server：可到「高级设置 → llama-server 路径」点「浏览…」指定其他位置，\
                 或自行下载 llama.cpp 的 Windows 版本后把 llama-server.exe 放到程序目录（或 llama-b12549 之类的子目录）",
                "",
            );
        }
    }

    let desired = app.store.cfg.i("app.panelPort", 8790) as u16;
    let (listener, port) = match web::bind(desired) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[llamaUI] failed to bind panel port: {}", e);
            std::process::exit(1);
        }
    };

    let url = format!("http://127.0.0.1:{}", port);
    println!("[llamaUI] panel: {}", url);
    println!("[llamaUI] data dir: {}", util::data_dir().display());
    println!("[llamaUI] author: 黑山大叔  https://space.bilibili.com/19428259");

    web::set_panel_port(port);
    let shared: web::api::Shared = Arc::new(Mutex::new(app));
    shutdown::install(Arc::clone(&shared));

    // CPU / 内存 / 显卡状态采样（后台线程，界面只读快照）
    sysinfo::start();

    // 对话页与 JSON API 仍由内置 HTTP 服务提供，供“测试聊天”按钮打开
    let server_shared = Arc::clone(&shared);
    std::thread::spawn(move || {
        web::serve(listener, server_shared);
    });

    #[cfg(windows)]
    {
        // 原生桌面窗口：界面不在浏览器里打开
        gui::run(shared);
    }
    #[cfg(not(windows))]
    {
        // 其它系统暂以浏览器面板作为功能对等回退（界面层与核心逻辑解耦）
        if shared
            .lock()
            .map(|a| a.store.cfg.b("app.autoOpenBrowser", true))
            .unwrap_or(true)
        {
            open_browser(&url);
        }
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }
}

/// 打开系统默认浏览器（不依赖第三方库）
fn open_browser(url: &str) {
    #[cfg(windows)]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
