//! 界面动作：模型导入、启动停止、保存配置、高级设置页与运行日志
//! UI actions: import, start/stop, save config, advanced page and runtime log.

use super::layout::{self, *};
use super::theme;
use super::win::*;
use super::{cfg, i18n, with_gui, AutoCtl, Ctl, Gui};
use crate::json::Json;
use crate::settings::{Kind, FIELDS};
use crate::util;

/// WM_COMMAND 分发
pub fn on_command(g: &mut Gui, hwnd: HWND, id: usize, code: u32) {
    // 自动发现窗口的筛选框：输入即过滤
    if id == ID_AUTO_FILTER {
        if code == 0x0300 {
            if let Some(ctl) = AUTO_WIN_CTL.with(|c| *c.borrow()) {
                g.auto_filter = get_text(ctl.filter);
                g.auto_page = 0;
                snapshot_auto(g);
                build_auto_page(g);
                position_auto_window(g);
            }
        }
        return;
    }
    match id {
        ID_IMPORT => {
            if code == 0 {
                import_file(g, hwnd);
            }
        }
        ID_SCAN => {
            if code == 0 {
                scan_dirs(g);
            }
        }
        ID_START => {
            if code == 0 {
                start_server(g);
            }
        }
        ID_STOP => {
            if code == 0 {
                stop_server(g);
            }
        }
        ID_CHAT => {
            if code == 0 {
                open_chat(g);
            }
        }
        ID_ADV => {
            if code == 0 {
                open_advanced(g);
            }
        }
        ID_MONITOR => {
            if code == 0 {
                open_monitor_window(g);
            }
        }
        ID_SAVE => {
            if code == 0 {
                save_config(g, hwnd);
            }
        }
        ID_NEWKEY => {
            if code == 0 {
                new_key(g);
            }
        }
        ID_COPY_EP => {
            if code == 0 {
                copy_endpoint(g);
            }
        }
        ID_COPY_KEY => {
            if code == 0 {
                copy_key(g);
            }
        }
        ID_COPY_LOCAL => {
            if code == 0 {
                copy_local_url(g);
            }
        }
        ID_LANG => {
            // CBN_SELCHANGE = 1
            if code == 1 {
                switch_language(g);
            }
        }
        ID_LIST => {
            // LBN_SELCHANGE = 1（列表重建触发的通知不算用户换模型）
            if code == 1 && !g.suppress_sel {
                select_from_list(g);
            }
        }

        ID_ESTIMATE => {
            if code == 0 {
                open_fit_window(g);
            }
        }
        ID_FIT_APPLY => {
            if code == 0 {
                apply_fit_profile(g);
            }
        }
        ID_FIT_CLOSE => {
            if code == 0 && g.fit_hwnd != 0 {
                unsafe {
                    DestroyWindow(g.fit_hwnd);
                }
                g.fit_hwnd = 0;
            }
        }
        ID_AUTO_OPEN => {
            if code == 0 {
                open_auto_window(g);
            }
        }
        ID_AUTO_SAVE => {
            if code == 0 {
                save_auto_values(g);
                if g.auto_hwnd != 0 {
                    unsafe {
                        DestroyWindow(g.auto_hwnd);
                    }
                    g.auto_hwnd = 0;
                    g.auto_fields.clear();
                }
            }
        }
        ID_AUTO_PREV => {
            if code == 0 && g.auto_page > 0 {
                snapshot_auto(g);
                g.auto_page -= 1;
                build_auto_page(g);
                position_auto_window(g);
            }
        }
        ID_AUTO_NEXT => {
            if code == 0 {
                let total = filtered_auto_params(g).len();
                if (g.auto_page + 1) * AUTO_PER_PAGE < total {
                    snapshot_auto(g);
                    g.auto_page += 1;
                    build_auto_page(g);
                    position_auto_window(g);
                }
            }
        }
        ID_PARAM_HELP => {
            if code == 0 {
                open_param_help(g);
            }
        }
        ID_HELP_TRANSLATE => {
            if code == 0 {
                start_translate(g);
            }
        }
        ID_HELP_CLOSE => {
            if code == 0 && g.help_hwnd != 0 {
                unsafe {
                    DestroyWindow(g.help_hwnd);
                }
                g.help_hwnd = 0;
            }
        }
        ID_ADV_BROWSE => {
            if code == 0 {
                browse_llama_exe(g);
            }
        }
        ID_ADV_SAVE => {
            if code == 0 {
                save_config(g, hwnd);
            }
        }
        ID_ADV_CLOSE => {
            if code == 0 {
                if g.help_hwnd != 0 {
                    unsafe {
                        DestroyWindow(g.help_hwnd);
                    }
                    g.help_hwnd = 0;
                }
                if g.adv_hwnd != 0 {
                    unsafe {
                        DestroyWindow(g.adv_hwnd);
                    }
                    g.adv_hwnd = 0;
                    g.adv_fields.clear();
                }
            }
        }
        ID_LOG_CLEAR => {
            if code == 0 {
                if let Ok(app) = g.shared.lock() {
                    if let Ok(mut logs) = app.launcher.logs.lock() {
                        logs.clear();
                    }
                }
                g.log_seen = 0;
                set_text(g.log_edit, "");
            }
        }
        ID_LLAMA_DL => {
            if code == 0 {
                open_url("https://github.com/ggml-org/llama.cpp/releases");
            }
        }
        ID_KVMEM_DL => {
            if code == 0 {
                open_url("https://github.com/kvmem/kvmem/releases");
            }
        }
        ID_HELP => {
            if code == 0 {
                open_help_doc(g);
            }
        }
        layout::ID_CTX_REMOVE => {
            if code == 0 {
                remove_selected(g);
            }
        }
        layout::ID_CTX_OPEN_DIR => {
            if code == 0 {
                open_model_dir(g);
            }
        }
        _ => {
            // 勾选框被点击：立刻刷新提示行与启动参数预览，
            // 让「开启识图」勾选状态与文案保持一致
            if id >= layout::ID_FIELD_BASE && id < layout::ID_FIELD_BASE + FIELDS.len() {
                let idx = id - layout::ID_FIELD_BASE;
                if FIELDS[idx].kind == Kind::Bool {
                    let on = check_get(
                        g.fields
                            .iter()
                            .find(|c| c.key == FIELDS[idx].key)
                            .map(|c| c.hwnd)
                            .unwrap_or(0),
                    );
                    if let Ok(mut app) = g.shared.lock() {
                        app.store.cfg.set_path(FIELDS[idx].key, Json::Bool(on));
                    }
                    layout::refresh_args_preview(g);
                    show_ctx_hint(g);
                }
            }
        }
    }
}

pub fn on_tick(g: &mut Gui) {
    // 监控窗口开着时才同步采集上下文并刷新
    if g.mon_hwnd != 0 {
        sync_monitor(g);
    }
    // 后台翻译完成时刷新解读窗口
    poll_translate(g);
    // 服务空闲时按需自动生成中文解读（默认关闭，见高级设置）
    auto_translate_tick(g);
    // 模型库签名只看“库内容”，不掺入当前选中项：
    // 否则每次换模型都会重建列表，把用户刚点的选择冲掉。
    let sig = match g.shared.lock() {
        Ok(app) => library_signature(&app.lib),
        Err(_) => String::new(),
    };
    if sig != g.lib_sig {
        g.lib_sig = sig;
        refresh_models(g);
    }
    refresh_status(g);
    append_logs(g);
    check_vram_risk(g);
    // 服务真正跑起来后，清掉上次的“不支持”红标
    let running = match g.shared.lock() {
        Ok(app) => app.launcher.status_json(&app.store.cfg).s("phase", "") == "running",
        Err(_) => false,
    };
    // 服务跑起来后量一次真实显存，校准「显存估算」的公式误差
    if let Some((_, c)) = crate::vramcal::tick(running) {
        if let Ok(app) = g.shared.lock() {
            app.launcher.push_log(
                &format!(
                    "vram-calib: ctx={} 实测每 token 额外 {:.3} MB（公式未计入的部分）",
                    c.sample_ctx, c.extra_mb_per_token
                ),
                "",
            );
        }
    }
    if running {
        let id = g.selected_id.clone();
        let mut changed = false;
        if let Ok(mut app) = g.shared.lock() {
            changed = app.lib.set_unsupported(&id, false);
            if changed {
                let _ = app.lib.save();
            }
        }
        if changed {
            refresh_models(g);
        }
    }
}

/// 库内容签名：id + 名称 + 大小
fn library_signature(lib: &crate::models::Library) -> String {
    let mut out = String::new();
    for m in lib.items.iter() {
        out.push_str(&m.id);
        out.push(':');
        out.push_str(&m.name);
        out.push(':');
        out.push_str(&m.size.to_string());
        out.push('|');
    }
    out
}

// ---------------- 模型库 ----------------

pub fn refresh_models(g: &mut Gui) {
    let (items, selected) = match g.shared.lock() {
        Ok(app) => (app.lib.list_json(), app.store.selected_model()),
        Err(_) => (Json::Arr(vec![]), String::new()),
    };
    list_reset(g.list);
    g.model_ids.clear();
    let mut vision_bits: u64 = 0;
    let mut bad_bits: u64 = 0;
    let mut moe_bits: u64 = 0;
    let mut big_bits: u64 = 0;
    let mut sel_index: Option<usize> = None;
    // 重建列表会触发一次选择通知，这期间不要把它当成“用户换模型”
    g.suppress_sel = true;
    if let Some(arr) = items.as_arr() {
        for (i, m) in arr.iter().enumerate() {
            let id = m.s("id", "");
            let name = m.s("name", "");
            let size = m.s("sizeText", "");
            list_add(g.list, &format!("{}   {}", name, size));
            if m.b("vision", false) && i < 64 {
                vision_bits |= 1u64 << i;
            }
            if m.b("unsupported", false) && i < 64 {
                bad_bits |= 1u64 << i;
            }
            if m.b("moe", false) && i < 64 {
                moe_bits |= 1u64 << i;
            }
            // 权重 > 8 GB：建议走 KVMem 引擎，列表里给个 KV 标记
            if m.i("size", 0) > 8_000_000_000 && i < 64 {
                big_bits |= 1u64 << i;
            }
            if id == selected {
                sel_index = Some(i);
            }
            g.model_ids.push(id);
        }
        if arr.is_empty() {
            list_add(g.list, &i18n(g, "label.emptyModels"));
        }
    }
    super::VISION_BITS.store(vision_bits, std::sync::atomic::Ordering::Relaxed);
    super::UNSUPPORTED_BITS.store(bad_bits, std::sync::atomic::Ordering::Relaxed);
    super::MOE_BITS.store(moe_bits, std::sync::atomic::Ordering::Relaxed);
    super::BIG_BITS.store(big_bits, std::sync::atomic::Ordering::Relaxed);
    // 列表项在 LB_ADDSTRING 时可能已经画过一次（那时标记位还是旧的），强制重画
    unsafe {
        InvalidateRect(g.list, std::ptr::null(), 0);
    }
    // 选中项仍存在就保持；不存在则回落到第一项，并在日志里说明，
    // 不再“悄悄换成默认模型”。
    let mut final_sel = selected.clone();
    if sel_index.is_none() && !g.model_ids.is_empty() {
        let first = g.model_ids[0].clone();
        let name = items
            .as_arr()
            .and_then(|a| a.first())
            .map(|m| m.s("name", ""))
            .unwrap_or_default();
        if let Ok(mut app) = g.shared.lock() {
            app.store.set_selected(&first);
            let _ = app.store.save();
            app.launcher.push_log(
                &format!("selected model was missing, fell back to: {}", name),
                "",
            );
        }
        final_sel = first;
        sel_index = Some(0);
    }
    if let Some(i) = sel_index {
        list_select(g.list, i);
    }
    g.suppress_sel = false;
    g.selected_id = final_sel;
    sync_mmproj(g);
    layout::refresh_args_preview(g);
    show_ctx_hint(g);
}

/// 右键菜单：从模型库删除当前条目（只移除记录，不删磁盘文件）
pub fn model_context_menu(g: &mut Gui, hwnd: HWND, lp: LPARAM) {
    let (x, y) = if lp == -1 || (lp as isize) == 0 {
        let mut pt = POINT { x: 0, y: 0 };
        unsafe {
            GetCursorPos(&mut pt);
        }
        (pt.x, pt.y)
    } else {
        ((lp & 0xFFFF) as i32, ((lp >> 16) & 0xFFFF) as i32)
    };
    // 让右键点中的那一条先成为选中项，避免误删
    if let Some(idx) = list_index_from_point(g.list, x, y) {
        list_select(g.list, idx);
        if let Some(id) = g.model_ids.get(idx).cloned() {
            if let Ok(mut app) = g.shared.lock() {
                app.store.set_selected(&id);
                let _ = app.store.save();
            }
            g.selected_id = id;
            layout::refresh_args_preview(g);
        }
    }
    let item = i18n(g, "menu.removeModel");
    let open_item = i18n(g, "menu.openModelDir");
    let cmd = popup_menu(
        hwnd,
        &[
            (layout::ID_CTX_OPEN_DIR, open_item),
            (layout::ID_CTX_REMOVE, item),
        ],
        x,
        y,
    );
    if cmd == layout::ID_CTX_REMOVE {
        remove_selected(g);
    } else if cmd == layout::ID_CTX_OPEN_DIR {
        open_model_dir(g);
    }
}

/// 在资源管理器里定位当前选中模型的文件
fn open_model_dir(g: &mut Gui) {
    let path = match g.shared.lock() {
        Ok(app) => app
            .lib
            .find(&g.selected_id)
            .map(|m| m.path.clone())
            .unwrap_or_default(),
        Err(_) => String::new(),
    };
    if path.is_empty() {
        return;
    }
    let abs = crate::util::absolutize(&path);
    crate::gui::win::reveal_in_explorer(&abs);
}

fn remove_selected(g: &mut Gui) {
    let Some(idx) = list_sel(g.list) else { return };
    let Some(id) = g.model_ids.get(idx).cloned() else {
        return;
    };
    let name = match g.shared.lock() {
        Ok(mut app) => {
            let name = app.lib.find(&id).map(|m| m.name.clone()).unwrap_or_default();
            app.lib.remove(&id);
            let _ = app.lib.save();
            // 删掉的正是当前模型：清空选择，避免指向不存在的条目
            if app.store.selected_model() == id {
                app.store.set_selected("");
                let _ = app.store.save();
            }
            app.launcher
                .push_log(&format!("model removed from library: {}", name), "");
            name
        }
        Err(_) => return,
    };
    if name.is_empty() {
        return;
    }
    refresh_models(g);
    refresh_status(g);
}

/// 导入：选择一个目录，递归识别其中（含子文件夹）的所有 .gguf 模型
fn import_file(g: &mut Gui, hwnd: HWND) {
    let title = i18n(g, "button.importFolder");
    let Some(dir) = pick_folder(hwnd, &title) else {
        return;
    };
    let mut added_dir = false;
    let (added, total) = match g.shared.lock() {
        Ok(mut app) => {
            let rep = app.lib.import_paths(&[dir.clone()]);
            let n = rep.added;
            app.lib.refresh_vision_flags();
        app.lib.refresh_moe_flags();
            let _ = app.lib.save();
            // 导入的目录也登记进扫描列表，下次「扫描目录」还能带上它
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
            if !dirs.iter().any(|d| d.eq_ignore_ascii_case(&dir)) {
                dirs.push(dir.clone());
                app.store.cfg.set_path(
                    "app.scanDirs",
                    Json::Arr(dirs.iter().map(|d| Json::str(d)).collect()),
                );
                let _ = app.store.save();
                added_dir = true;
            }
            if app.store.selected_model().is_empty() {
                if let Some(first) = app.lib.items.first().map(|m| m.id.clone()) {
                    app.store.set_selected(&first);
                    let _ = app.store.save();
                }
            }
            app.launcher.push_log(
                &format!(
                    "import folder: {} (+{} models, {} skipped not-a-chat-model, added to scan list)",
                    dir, n, rep.skipped
                ),
                "",
            );
            (n, app.lib.items.len())
        }
        Err(_) => (0, 0),
    };
    if added_dir && g.adv_hwnd != 0 {
        populate_advanced(g);
    }
    refresh_models(g);
    if added == 0 {
        msg_box(
            hwnd,
            &i18n(g, "label.noModelInFolder"),
            &title,
            MB_OK | MB_ICONINFORMATION,
        );
    } else {
        set_text(g.lbl_hint, &format!("+{} / {}", added, total));
    }
}

fn scan_dirs(g: &mut Gui) {
    let mut added = 0usize;
    let mut skipped = 0usize;
    if let Ok(mut app) = g.shared.lock() {
        let mut dirs: Vec<String> = app
            .store
            .cfg
            .path("app.scanDirs")
            .and_then(|v| v.as_arr())
            .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_default();
        for d in crate::models::Library::default_scan_dirs() {
            if !dirs.contains(&d) {
                dirs.push(d);
            }
        }
        for d in dirs {
            let real = crate::util::resolve_dir(&d);
            if real.is_dir() {
                let r = app.lib.import_paths(&[real.to_string_lossy().to_string()]);
                added += r.added;
                skipped += r.skipped;
            }
        }
        // 与磁盘对齐：删掉文件已消失的条目、刷新大小/视图片/上下文
        let report = app.lib.sync_with_disk();
        // MoE 标记按 GGUF 元数据刷新（换库、换机器后自动重算）
        app.lib.refresh_moe_flags();
        // 当前选中的模型被删了 → 清掉选择，避免启动时指向不存在的文件
        let sel = app.store.selected_model();
        if !sel.is_empty() && app.lib.find(&sel).is_none() {
            if let Some(first) = app.lib.items.first().map(|m| m.id.clone()) {
                app.store.set_selected(&first);
            } else {
                app.store.set_selected("");
            }
            let _ = app.store.save();
        }
        let _ = app.lib.save();
        if app.store.selected_model().is_empty() {
            if let Some(first) = app.lib.items.first().map(|m| m.id.clone()) {
                app.store.set_selected(&first);
                let _ = app.store.save();
            }
        }
        app.launcher.push_log(
            &format!(
                "scan finished: +{} added, -{} removed, {} updated, {} skipped (not a chat model) (total {})",
                added,
                report.removed,
                report.updated,
                skipped + report.skipped,
                app.lib.items.len()
            ),
            "",
        );
        for n in report.removed_names.iter().take(5) {
            app.launcher
                .push_log(&format!("  removed: {}", n), "");
        }
        for n in report.skipped_names.iter().take(5) {
            app.launcher
                .push_log(&format!("  skipped (not a chat model): {}", n), "");
        }
    }
    refresh_models(g);
    set_text(g.lbl_hint, &format!("+{}", added));
}

fn select_from_list(g: &mut Gui) {
    let Some(idx) = list_sel(g.list) else { return };
    let Some(id) = g.model_ids.get(idx).cloned() else {
        return;
    };
    // 同一模型被重复“选中”（列表重建会触发一次选择通知）时不要停服
    let prev = g.selected_id.clone();
    let mut name = String::new();
    let mut was_running = false;
    let mut changed = false;
    let mut warn = String::new();
    if let Ok(mut app) = g.shared.lock() {
        changed = prev != id || app.store.selected_model() != id;
        app.store.set_selected(&id);
        let _ = app.store.save();
        name = app.lib.find(&id).map(|m| m.name.clone()).unwrap_or_default();
        was_running = app.launcher.status_json(&app.store.cfg).s("phase", "") != "stopped";
        if changed {
            app.launcher
                .push_log(&format!("model selected: {}", name), "");
        }
        // 切换模型**不**自动停服：正在运行的模型保持不变，
        // 需要用户手动点「停止服务」，避免误操作打断正在进行的对话/任务
        if was_running && changed {
            let running = app
                .launcher
                .status_json(&app.store.cfg)
                .s("argsPreview", "");
            app.launcher.push_log(
                "service keeps running the previous model; press Stop service to switch",
                "",
            );
            warn = running;
        }
    }
    g.selected_id = id;
    // 服务未运行时，按模型权重大小自动匹配引擎（>8 GB → KVMem，小模型留 llama.cpp）
    // 用户仍可在「引擎」下拉里手动覆盖
    if !was_running {
        auto_pick_engine(g);
    }
    sync_mmproj(g);
    layout::refresh_args_preview(g);
    refresh_status(g);
    show_ctx_hint(g);
    if !warn.is_empty() {
        let msg = i18n(g, "hint.modelChangedWhileRunning");
        set_text(g.lbl_hint, &msg);
    }
}

/// 选模型后同步“识图”相关配置：
/// 1) 有 mmproj 的模型自动勾选「开启识图」并填入投影文件，没有则取消
/// 2) 上下文长度若是自动模式，填成模型原生上限
pub fn sync_mmproj(g: &mut Gui) {
    let (mmproj, native, name) = g
        .shared
        .lock()
        .ok()
        .and_then(|a| {
            a.lib
                .find(&g.selected_id)
                .map(|m| (m.mmproj.clone(), m.ctx_native, m.name.clone()))
        })
        .unwrap_or_default();

    let mut app = match g.shared.lock() {
        Ok(a) => a,
        Err(_) => return,
    };

    // ---- 识图开关 ----
    let has_mmproj = !mmproj.is_empty();
    if let Some(ctl) = g.fields.iter().find(|c| c.key == "launch.mmproj") {
        set_text(ctl.hwnd, &mmproj);
    }
    let prev_mmproj = app.store.cfg.s("launch.mmproj", "");
    if prev_mmproj != mmproj {
        app.store.cfg.set_path("launch.mmproj", Json::str(&mmproj));
        // 换了模型就按新模型的识图能力重置开关
        app.store
            .cfg
            .set_path("launch.vision", Json::Bool(has_mmproj));
        let _ = app.store.save();
        if has_mmproj {
            app.launcher
                .push_log(&format!("vision projector: {}", mmproj), "");
        }
    }

    // 勾选框必须在配置同步“之后”再设置，否则会用到切换前的旧值
    let want = if has_mmproj {
        app.store.cfg.b("launch.vision", true)
    } else {
        false
    };
    if let Some(ctl) = g.fields.iter().find(|c| c.key == "launch.vision") {
        check_set(ctl.hwnd, want);
    }

    // ---- 上下文自动填充 ----
    if app.store.cfg.b("app.ctxAuto", true) && native > 0 {
        let want = auto_ctx_for(native);
        let cur = app.store.cfg.i("launch.ctxSize", 0);
        if cur != want as i64 {
            app.store.cfg.set_path("launch.ctxSize", Json::Num(want as f64));
            let _ = app.store.save();
            if want == native {
                app.launcher.push_log(
                    &format!("context size auto-set to {} (model max)", native),
                    "",
                );
            } else {
                app.launcher.push_log(
                    &format!(
                        "context size auto-set to {} (model max {} exceeds the {} cap)",
                        want, native, AUTO_CTX_MAX
                    ),
                    "",
                );
            }
        }
        if let Some(ctl) = g.fields.iter().find(|c| c.key == "launch.ctxSize") {
            set_text(ctl.hwnd, &want.to_string());
        }
        let _ = name;
    }
}

/// 自动跟随模型时的上下文长度上限。
/// 有些模型标称 1M 上下文，照搬会直接吃光显存，这里封顶到 256k；
/// 需要更长就取消勾选「上下文跟随模型」自己填。
const AUTO_CTX_MAX: u32 = 262144;

/// 跟随模型时应使用的上下文：取原生值，但不超过 AUTO_CTX_MAX
fn auto_ctx_for(native: u32) -> u32 {
    native.min(AUTO_CTX_MAX)
}

/// 提示当前模型的原生上下文，并在设置过小时给出建议
pub fn show_ctx_hint(g: &mut Gui) {
    let native = g
        .shared
        .lock()
        .ok()
        .and_then(|a| a.lib.find(&g.selected_id).map(|m| m.ctx_native))
        .unwrap_or(0);
    let cur = cfg(g).i("launch.ctxSize", 4096);
    let native_text = if native > 0 {
        native.to_string()
    } else {
        i18n(g, "status.unknown")
    };
    let mut text = i18n(g, "hint.ctxInfo")
        .replace("{cur}", &cur.to_string())
        .replace("{native}", &native_text);
    if cur < 32768 {
        text.push_str(&i18n(g, "hint.ctxTooSmall"));
    }
    // 自动跟随模型上限 / 手动固定
    let auto = g
        .shared
        .lock()
        .ok()
        .map(|a| a.store.cfg.b("app.ctxAuto", true))
        .unwrap_or(true);
    text.push_str(&i18n(
        g,
        if auto { "hint.ctxAutoOn" } else { "hint.ctxAutoOff" },
    ));
    // 识图提示：以「开启识图」勾选框的实际状态为准，不要只看 mmproj 是否存在
    let has_mmproj = g
        .shared
        .lock()
        .ok()
        .map(|a| !a.store.cfg.s("launch.mmproj", "").is_empty())
        .unwrap_or(false);
    if has_mmproj {
        let vision_on = vision_checked(g);
        text.push_str(&i18n(
            g,
            if vision_on {
                "hint.visionOn"
            } else {
                "hint.visionOffOnModel"
            },
        ));
    }
    set_text(g.lbl_hint, &text);
}

/// 「开启识图」当前是否勾选（优先读控件，回落到配置）
pub fn vision_checked(g: &Gui) -> bool {
    if let Some(ctl) = g.fields.iter().find(|c| c.key == "launch.vision") {
        if ctl.hwnd != 0 {
            return check_get(ctl.hwnd);
        }
    }
    g.shared
        .lock()
        .ok()
        .map(|a| a.store.cfg.b("launch.vision", true))
        .unwrap_or(true)
}

// ---------------- 复制 ----------------

fn copy_endpoint(g: &mut Gui) {
    let url = endpoint_url(g);
    set_clipboard_text(g.hwnd, &url);
    set_text(g.lbl_hint, &format!("{} {}", i18n(g, "status.copied"), url));
}

fn copy_key(g: &mut Gui) {
    let c = cfg(g);
    let keys = crate::settings::api_keys(&c);
    let text = keys.join("\r\n");
    if text.is_empty() {
        set_text(g.lbl_hint, &i18n(g, "err.noApiKey"));
        return;
    }
    set_clipboard_text(g.hwnd, &text);
    set_text(
        g.lbl_hint,
        &format!("{} {}", i18n(g, "status.copied"), i18n(g, "label.apiKey")),
    );
}

/// 对外接口地址：优先局域网 IP（开放局域网时），否则本机地址
/// 本机地址（固定 127.0.0.1），给只在本机跑客户端的场景用
pub fn local_url(g: &Gui) -> String {
    let c = cfg(g);
    let port = c.i("network.port", 9931);
    format!("http://127.0.0.1:{}/v1", port)
}

fn copy_local_url(g: &mut Gui) {
    let url = local_url(g);
    set_clipboard_text(g.hwnd, &url);
    set_text(g.lbl_hint, &format!("{} {}", i18n(g, "status.copied"), url));
}

pub fn endpoint_url(g: &Gui) -> String {
    let c = cfg(g);
    let port = c.i("network.port", 9931);
    let bind = crate::settings::bind_host(&c);
    let host = if bind == "0.0.0.0" || bind.is_empty() {
        crate::process::lan_ip().unwrap_or_else(|| "127.0.0.1".to_string())
    } else {
        bind
    };
    format!("http://{}:{}/v1", host, port)
}

// ---------------- 配置读写 ----------------

fn collect_patch(g: &Gui) -> Json {
    let mut patch = Json::obj();
    // 只有高级设置窗口真的开着时，才收集它的控件；
    // 否则会读到已销毁控件的空文本，把数值写成 0
    let mut all: Vec<&Ctl> = g.fields.iter().collect();
    if g.adv_hwnd != 0 {
        all.extend(g.adv_fields.iter());
    }
    // 自动发现的新参数：值写到 auto.<key>，空值表示不使用该参数
    if g.auto_hwnd != 0 {
        for ac in g.auto_fields.iter() {
            if ac.hwnd == 0 {
                continue;
            }
            let val = match ac.kind {
                Kind::Bool => Json::Bool(check_get(ac.hwnd)),
                Kind::Select => {
                    // 下拉显示的是本地化文案，值必须按下标回读 options
                    let idx = combo_index(ac.hwnd);
                    let opts: &'static [&'static str] = crate::settings::FIELDS
                        .iter()
                        .find(|f| f.key == ac.key)
                        .map(|f| f.options)
                        .unwrap_or(&[]);
                    match opts.get(idx) {
                        Some(v) => Json::str(*v),
                        None => {
                            let t = combo_text(ac.hwnd);
                            if t.trim().is_empty() {
                                continue;
                            }
                            Json::str(&t)
                        }
                    }
                }
                _ => {
                    let t = get_text(ac.hwnd);
                    if t.trim().is_empty() {
                        continue;
                    }
                    Json::str(t.trim())
                }
            };
            patch.set_path(&format!("auto.{}", ac.key), val);
        }
    }
    // 扫描目录来自列表控件（相对路径为程序自带）
    if g.adv_hwnd != 0 && g.scan_list != 0 {
        let mut dirs: Vec<String> = Vec::new();
        let suffix = format!("（{}）", i18n(g, "label.builtinDir"));
        let mut i = 0usize;
        loop {
            let t = list_text(g.scan_list, i);
            if t.is_empty() {
                break;
            }
            let raw_dir = t.trim_end_matches(&suffix).trim().to_string();
            if !raw_dir.is_empty() {
                dirs.push(raw_dir);
            }
            i += 1;
            if i > 500 {
                break;
            }
        }
        patch.set_path(
            "app.scanDirs",
            Json::Arr(dirs.iter().map(|d| Json::str(d)).collect()),
        );
    }
    for ctl in all {
        if ctl.hwnd == 0 {
            continue;
        }
        let value = match ctl.kind {
            Kind::Bool => Json::Bool(check_get(ctl.hwnd)),
            Kind::Select => {
                let idx = combo_index(ctl.hwnd);
                match ctl.options.get(idx) {
                    Some(v) => Json::str(*v),
                    None => Json::str(&combo_text(ctl.hwnd)),
                }
            }
            Kind::Int => {
                let raw = get_text(ctl.hwnd);
                Json::Num(raw.trim().parse::<i64>().unwrap_or(0) as f64)
            }
            Kind::Float => {
                let raw = get_text(ctl.hwnd);
                Json::Num(raw.trim().parse::<f64>().unwrap_or(0.0))
            }
            Kind::MultilineText => {
                let raw = get_text(ctl.hwnd);
                if ctl.key == "network.apiKeys" {
                    let keys: Vec<Json> = raw
                        .split(['\r', '\n', ',', ';'])
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(Json::str)
                        .collect();
                    Json::Arr(keys)
                } else if ctl.key == "app.scanDirs" {
                    let dirs: Vec<Json> = raw
                        .split(['\r', '\n', ';'])
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(Json::str)
                        .collect();
                    Json::Arr(dirs)
                } else {
                    Json::str(raw)
                }
            }
            _ => Json::str(get_text(ctl.hwnd)),
        };
        patch.set_path(ctl.key, value);
    }
    patch
}

fn save_config(g: &mut Gui, hwnd: HWND) {
    let mut patch = collect_patch(g);
    // 「上下文跟随模型」勾上时，不管用户填了什么，一律写回所选模型的原生上限。
    // 关掉时尊重用户手填的值。
    let auto = patch
        .path("app.ctxAuto")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if auto {
        if let Ok(app) = g.shared.lock() {
            let native = app
                .lib
                .find(&app.store.selected_model())
                .map(|m| m.ctx_native)
                .unwrap_or(0);
            if native > 0 {
                patch.set_path(
                    "launch.ctxSize",
                    crate::json::Json::Num(auto_ctx_for(native) as f64),
                );
            }
        }
    }
    let result = match g.shared.lock() {
        Ok(mut app) => match app.store.merge_update(&patch) {
            Ok(_) => {
                let native = app
                    .lib
                    .find(&app.store.selected_model())
                    .map(|m| m.ctx_native)
                    .unwrap_or(0);
                let auto = patch
                    .path("app.ctxAuto")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                app.store.cfg.set_path("app.ctxAuto", Json::Bool(auto));
                match app.store.save() {
                    Ok(_) => {
                        app.launcher.push_log("settings saved", "");
                        Ok(app.store.cfg.clone())
                    }
                    Err(_) => Err("err.io".to_string()),
                }
            }
            Err(e) => Err(e.split(':').next().unwrap_or("err.badPayload").to_string()),
        },
        Err(_) => Err("err.lock".to_string()),
    };
    match result {
        Ok(c) => {
            // 跟随模式把界面上的上下文长度同步成实际生效值
            if c.b("app.ctxAuto", false) {
                if let Some(ctl) = g.fields.iter().find(|f| f.key == "launch.ctxSize") {
                    set_text(ctl.hwnd, &c.i("launch.ctxSize", 0).to_string());
                }
            }
            set_text(g.lbl_hint, &format!("✓ {}", i18n(g, "button.save")));
            if crate::i18n::resolve(&c) != g.lang {
                layout::apply_language(g);
            }
            layout::refresh_args_preview(g);
            refresh_status(g);
            if g.adv_hwnd != 0 {
                populate_advanced(g);
            }
        }
        Err(code) => {
            set_text(g.lbl_hint, &i18n(g, &code));
            msg_box(
                hwnd,
                &i18n(g, &code),
                &i18n(g, "err.badPayload"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}

/// 生成新密钥：旧密钥全部清空，只保留新的一个
fn new_key(g: &mut Gui) {
    let key = util::new_api_key();
    if let Ok(mut app) = g.shared.lock() {
        app.store.cfg.set_path(
            "network.apiKeys",
            Json::Arr(vec![Json::str(&key)]),
        );
        let _ = app.store.save();
        app.launcher
            .push_log("new api key generated, previous keys cleared", "");
    }
    layout::populate(g);
    set_text(g.lbl_hint, &i18n(g, "status.newKey"));
    set_clipboard_text(g.hwnd, &key);
}

fn switch_language(g: &mut Gui) {
    let value = match combo_sel(g.lang_combo) {
        Some(0) => "auto".to_string(),
        Some(1) => "zh-CN".to_string(),
        Some(2) => "en-US".to_string(),
        _ => "auto".to_string(),
    };
    if let Ok(mut app) = g.shared.lock() {
        app.store.cfg.set_path("app.lang", Json::str(&value));
        let _ = app.store.save();
    }
    layout::apply_language(g);
    refresh_models(g);
    refresh_status(g);
    set_text(g.lbl_hint, "");
}

// ---------------- 服务控制 ----------------

pub fn refresh_status(g: &mut Gui) {
    let (phase, endpoint, model_name, uptime, pid, tok_prompt, tok_gen) = match g.shared.lock() {
        Ok(app) => {
            let st = app.launcher.status_json(&app.store.cfg);
            let name = app
                .lib
                .find(&app.store.selected_model())
                .map(|m| m.name.clone())
                .unwrap_or_default();
            (
                st.s("phase", "stopped"),
                st.path("endpoints").map(|e| e.s("base", "")).unwrap_or_default(),
                name,
                st.i("uptime", 0),
                st.i("pid", 0),
                st.i("tokensPrompt", 0),
                st.i("tokensGen", 0),
            )
        }
        Err(_) => ("stopped".into(), String::new(), String::new(), 0, 0, 0, 0),
    };

    let phase_text = match phase.as_str() {
        "running" => i18n(g, "status.running"),
        "starting" => i18n(g, "status.starting"),
        "error" => i18n(g, "status.error"),
        _ => i18n(g, "status.stopped"),
    };
    let dot = match phase.as_str() {
        "running" => theme::OK,
        "starting" => 0x0020_A0_E0, // #E0A020 琥珀色
        "error" => theme::DANGER,
        _ => 0x00A4_9A_90, // #909AA4 灰
    };
    super::PILL_COLOR.store(dot, std::sync::atomic::Ordering::Relaxed);
    set_text(g.st_pill, &format!("● {}", phase_text));
    let model_text = if model_name.is_empty() {
        i18n(g, "status.noModel")
    } else {
        format!("{}: {}", i18n(g, "status.model"), model_name)
    };
    // 名字过长时不换行、不弹窗：改成在原位横向滚动（见 on_marquee_tick）
    if g.model_full != model_name || g.model_text != model_text {
        g.model_full = model_name.clone();
        g.model_text = model_text;
        g.marquee_off = 0;
        g.model_scroll = model_needs_scroll(g);
        unsafe {
            InvalidateRect(g.hwnd, &g.model_rect, 0);
        }
    }

    // 顶部改为显示“本次运行 token 用量”（绑定地址不再展示）
    set_text(
        g.st_host,
        &format!(
            "{}: {} / {}",
            i18n(g, "status.tokens"),
            human_tokens(tok_prompt as u64),
            human_tokens(tok_gen as u64)
        ),
    );
    let up = if uptime > 0 {
        format!(
            "{}: {}h {}m",
            i18n(g, "status.uptime"),
            uptime / 3600,
            (uptime % 3600) / 60
        )
    } else {
        format!("{}: —", i18n(g, "status.uptime"))
    };
    set_text(g.st_uptime, &up);

    // 默认就显示可直接复制的局域网 / 本机地址
    let url = endpoint_url(g);
    let ep = if endpoint.is_empty() {
        url
    } else if pid > 0 {
        format!("{}  ·  PID {}", url, pid)
    } else {
        url
    };
    set_text(g.st_endpoint, &ep);

    // 硬件状态：CPU / 内存 / 显卡
    let s = crate::sysinfo::sample();
    set_text(g.st_cpu, &format!("{} {}%", i18n(g, "status.cpu"), s.cpu_percent));
    if s.ram_total_mb > 0 {
        set_text(
            g.st_ram,
            &format!(
                "{} {:.1} / {:.1} GB",
                i18n(g, "status.ram"),
                s.ram_used_mb as f64 / 1024.0,
                s.ram_total_mb as f64 / 1024.0
            ),
        );
    }
    if s.gpu_known && s.vram_total_mb > 0 {
        set_text(
            g.st_gpu,
            &format!(
                "{} {}°C  ·  {} {:.1} / {:.1} GB",
                i18n(g, "status.gpu"),
                s.gpu_temp,
                i18n(g, "status.vram"),
                s.vram_used_mb as f64 / 1024.0,
                s.vram_total_mb as f64 / 1024.0
            ),
        );
    } else if s.gpu_known {
        set_text(g.st_gpu, &format!("{} {}°C", i18n(g, "status.gpu"), s.gpu_temp));
    } else {
        set_text(g.st_gpu, &i18n(g, "status.gpuNone"));
    }

    let running = phase == "running" || phase == "starting";
    enable(g.btn_start, !running);
    enable(g.btn_stop, phase != "stopped");
    set_text(
        g.btn_start,
        &i18n(g, if phase == "starting" { "status.starting" } else { "button.start" }),
    );
}

fn start_server(g: &mut Gui) {
    let patch = collect_patch(g);
    let (cfg_snapshot, model_path, model_id) = match g.shared.lock() {
        Ok(mut app) => {
            let _ = app.store.merge_update(&patch);
            let _ = app.store.save();
            let id = app.store.selected_model();
            let path = app
                .lib
                .find(&id)
                .map(|m| util::absolutize(&m.path))
                .unwrap_or_default();
            (app.store.cfg.clone(), path, id)
        }
        Err(_) => return,
    };
    if model_id.is_empty() || model_path.is_empty() {
        msg_box(
            g.hwnd,
            &i18n(g, "err.modelNotSelected"),
            &i18n(g, "app.title"),
            MB_OK | MB_ICONWARNING,
        );
        return;
    }
    if !std::path::Path::new(&model_path).is_file() {
        let msg = format!("{}: {}", i18n(g, "err.modelMissing"), model_path);
        set_text(g.lbl_hint, &msg);
        if let Ok(app) = g.shared.lock() {
            app.launcher.push_log(&msg, "");
        }
        msg_box(g.hwnd, &msg, &i18n(g, "status.error"), MB_OK | MB_ICONWARNING);
        return;
    }
    // 先量基线显存：必须赶在 spawn 之前，否则模型显存已经算进去了
    let vram_baseline = crate::monitor::gpu_mem_used_mb().unwrap_or(0);
    let res = match g.shared.lock() {
        Ok(mut app) => {
            // 上下文过小时提前告警：智能体框架通常要预留大量压缩余量
            let ctx = cfg_snapshot.i("launch.ctxSize", 4096);
            if ctx < 32768 {
                let native = app
                    .lib
                    .find(&model_id)
                    .map(|m| m.ctx_native)
                    .unwrap_or(0);
                let msg = i18n(g, "warn.ctxTooSmall")
                    .replace("{cur}", &ctx.to_string())
                    .replace(
                        "{native}",
                        &(if native > 0 {
                            native.to_string()
                        } else {
                            i18n(g, "status.unknown")
                        }),
                    );
                app.launcher.push_log(&msg, "");
            }
            app.launcher.start(&cfg_snapshot, &model_path)
        }
        Err(_) => Err("err.lock".to_string()),
    };
    // 启动成功后记录校准基线：服务跑起来再量一次真实显存，
    // 用来修正「显存估算」对新架构的低估
    if res.is_ok() {
        let fp = crate::vramcal::Fingerprint {
            ngl: cfg_snapshot.i("launch.nGpuLayers", 99),
            ctk: cfg_snapshot.s("cache.typeK", "f16"),
            ctv: cfg_snapshot.s("cache.typeV", "f16"),
            ncmoe: cfg_snapshot.i("gpu.nCpuMoe", 0),
            vision: cfg_snapshot.b("launch.vision", true),
        };
        let size = match g.shared.lock() {
            Ok(app) => app.lib.find(&model_id).map(|m| m.size).unwrap_or(0),
            Err(_) => 0,
        };
        // 识图模型的 mmproj 也占显存，校准时要先扣掉，否则会被算成每 token 成本
        let mmproj_mb = if cfg_snapshot.b("launch.vision", true) {
            let p = crate::util::absolutize(&cfg_snapshot.s("launch.mmproj", ""));
            std::fs::metadata(&p).map(|m| m.len() / 1_048_576).unwrap_or(0)
        } else {
            0
        };
        crate::vramcal::begin(
            &model_id,
            &crate::util::absolutize(&model_path),
            size,
            cfg_snapshot.i("launch.ctxSize", 8192).max(1) as u32,
            fp,
            cfg_snapshot.s("launch.flashAttn", "auto") != "off",
            vram_baseline,
            mmproj_mb,
        );
    }
    if let Err(e) = res {
        let code = e.split(':').next().unwrap_or("err.spawnFailed").to_string();
        set_text(g.lbl_hint, &i18n(g, &code));
        if let Ok(app) = g.shared.lock() {
            app.launcher.push_log(&format!("start failed: {}", e), "");
        }
        msg_box(
            g.hwnd,
            &i18n(g, &code),
            &i18n(g, "status.error"),
            MB_OK | MB_ICONWARNING,
        );
    }
    refresh_status(g);
}

fn stop_server(g: &mut Gui) {
    if let Ok(mut app) = g.shared.lock() {
        app.launcher.stop();
        app.launcher.push_log("service stopped by user", "");
    }
    refresh_status(g);
}

fn open_chat(g: &mut Gui) {
    let port = crate::web::panel_port();
    let url = format!("http://127.0.0.1:{}/chat.html", port);
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn();
    }
    let _ = &g.shared;
}

// ---------------- 列表悬停提示（不支持加载的模型） ----------------

/// 列表内鼠标移动：悬停在红色（不支持）条目上时弹一句提示
pub fn on_list_hover(g: &mut Gui, lp: LPARAM) {
    let x = (lp & 0xFFFF) as i16 as i32;
    let y = ((lp >> 16) & 0xFFFF) as i16 as i32;
    let mut pt = POINT { x, y };
    unsafe {
        ClientToScreen(g.list, &mut pt);
    }
    let Some(idx) = list_index_from_point(g.list, pt.x, pt.y) else {
        hide_hover(g);
        return;
    };
    let bad = idx < 64 && (super::UNSUPPORTED_BITS.load(std::sync::atomic::Ordering::Relaxed) >> idx) & 1 == 1;
    if !bad {
        hide_hover(g);
        return;
    }
    let text = i18n(g, "hint.unsupported");
    if g.hover_wnd == 0 {
        // 不抢焦点的提示窗：WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE，且被主窗口拥有
        g.hover_wnd = create_window(
            "STATIC",
            "",
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            WS_POPUP | 0x0001 | 0x0080,
            0,
            0,
            10,
            24,
            g.hwnd,
            0,
        );
        set_font(g.hover_wnd, g.font);
    }
    set_text(g.hover_wnd, &text);
    // 位置跟着鼠标，但不超出主窗口，避免“跑到窗口外”
    let (cw, ch) = client_size(g.hwnd);
    let hdc = unsafe { GetDC(g.hwnd) };
    let avail = if hdc != 0 {
        let w = theme::text_width(
            hdc,
            &text,
            super::FONT_UI.load(std::sync::atomic::Ordering::Relaxed) as HFONT,
        );
        unsafe {
            ReleaseDC(g.hwnd, hdc);
        }
        w
    } else {
        160
    };
    let width = (avail + 20).clamp(120, 360);
    let mut sx = pt.x + 14;
    let mut sy = pt.y + 20;
    if sx + width > cw - 8 {
        sx = (cw - width - 8).max(8);
    }
    if sy + 26 > ch - 8 {
        sy = pt.y - 30;
    }
    // 屏幕坐标转客户区
    let mut origin = POINT { x: 0, y: 0 };
    unsafe {
        ClientToScreen(g.hwnd, &mut origin);
    }
    move_to(g.hover_wnd, sx - origin.x, sy - origin.y, width, 24);
    show(g.hover_wnd, true);
}

/// 收起悬停提示
pub fn hide_hover(g: &mut Gui) {
    if g.hover_wnd != 0 {
        show(g.hover_wnd, false);
    }
}

// ---------------- 顶栏“当前模型”横向滚动 ----------------

/// 文字是否超出显示区域
fn model_needs_scroll(g: &Gui) -> bool {
    if g.model_text.is_empty() {
        return false;
    }
    let hdc = unsafe { GetDC(g.hwnd) };
    if hdc == 0 {
        return false;
    }
    let w = theme::text_width(hdc, &g.model_text, g.font);
    unsafe {
        ReleaseDC(g.hwnd, hdc);
    }
    let avail = g.model_rect.right - g.model_rect.left - 4;
    w > avail
}

/// 滚动步进：每 80ms 前进 2px，滚完之后停顿一下再从头开始
pub fn on_marquee_tick(g: &mut Gui) {
    if !g.model_scroll || g.model_rect.right <= g.model_rect.left {
        return;
    }
    let hdc = unsafe { GetDC(g.hwnd) };
    if hdc == 0 {
        return;
    }
    let text_w = theme::text_width(hdc, &g.model_text, g.font);
    unsafe {
        ReleaseDC(g.hwnd, hdc);
    }
    let avail = g.model_rect.right - g.model_rect.left;
    // 滚出左边界后再停 24px，然后回到起点，形成循环
    g.marquee_off += 2;
    if g.marquee_off > text_w - avail + 24 {
        g.marquee_off = 0;
    }
    unsafe {
        InvalidateRect(g.hwnd, &g.model_rect, 0);
    }
}

// ---------------- 底部运行日志 ----------------

fn append_logs(g: &mut Gui) {
    let total = match g.shared.lock() {
        Ok(app) => app.launcher.log_count() as usize,
        Err(_) => return,
    };
    if total == g.log_seen {
        return;
    }
    // 日志被清空后计数会回退，这里直接重来
    if total < g.log_seen {
        g.log_seen = 0;
        set_text(g.log_edit, "");
    }
    let lines = match g.shared.lock() {
        Ok(app) => app.launcher.snapshot_logs(g.log_seen),
        Err(_) => return,
    };
    g.log_seen = total;
    if lines.is_empty() {
        return;
    }
    let mut text = get_text(g.log_edit);
    if text.is_empty() && lines.len() > 200 {
        // 只保留尾部，避免控件文本无限增长
        let start = lines.len() - 200;
        text = lines[start..].join("\r\n");
    } else {
        if !text.is_empty() {
            text.push_str("\r\n");
        }
        text.push_str(&lines.join("\r\n"));
    }
    set_text(g.log_edit, &text);
    unsafe {
        SendMessageW(g.log_edit, EM_SETSEL, usize::MAX, -1);
        SendMessageW(g.log_edit, EM_SCROLLCARET, 0, 0);
    }

    // 从引擎日志里累加本轮 token 用量
    let (mut tp, mut tg) = (0u64, 0u64);
    for line in lines.iter() {
        if let Some((p, g)) = parse_token_line(line) {
            tp += p;
            tg += g;
        }
    }
    if tp > 0 || tg > 0 {
        if let Ok(app) = g.shared.lock() {
            app.launcher.add_tokens(tp, tg);
        }
    }

    // 把常见的加载失败翻译成一句人话，直接写在日志里并提示到界面
    let mut bad_arch: Option<String> = None;
    // 另一类失败：架构名认识，但张量对不上（模型比 llama.cpp 新，或用了循环层等新特性）
    let mut bad_tensor: Option<String> = None;
    for line in lines.iter() {
        if let Some(arch) = unknown_arch_of(line) {
            bad_arch = Some(arch);
            break;
        }
        if bad_tensor.is_none() {
            if let Some(t) = tensor_mismatch_of(line) {
                bad_tensor = Some(t);
            }
        }
    }
    if let Some(arch) = bad_arch {
        let msg = i18n(g, "err.unknownArch").replace("{arch}", &arch);
        // 该模型在列表里标红，悬停时提示“不支持加载该模型”
        let id = g.selected_id.clone();
        let mut changed = false;
        if let Ok(mut app) = g.shared.lock() {
            changed = app.lib.set_unsupported(&id, true);
            if changed {
                let _ = app.lib.save();
            }
            app.launcher.push_log(&msg, "");
        }
        set_text(g.lbl_hint, &msg);
        if changed {
            refresh_models(g);
        }
    } else if let Some(tensor) = bad_tensor {
        // 架构认识但张量缺失：同样是"当前 llama.cpp 加载不了这个模型"
        let msg = i18n(g, "err.tensorMismatch").replace("{tensor}", &tensor);
        let id = g.selected_id.clone();
        let mut changed = false;
        if let Ok(mut app) = g.shared.lock() {
            changed = app.lib.set_unsupported(&id, true);
            if changed {
                let _ = app.lib.save();
            }
            app.launcher.push_log(&msg, "");
        }
        set_text(g.lbl_hint, &msg);
        if changed {
            refresh_models(g);
        }
    }
}

/// 识别「架构名能认出、但张量布局对不上」的加载失败。
/// 典型日志：`error loading model: check_tensor_dims: tensor 'blk.24.attn_q.weight' not found`
/// 返回缺失的张量名，便于在提示里写清楚。
fn tensor_mismatch_of(line: &str) -> Option<String> {
    let key = "check_tensor_dims:";
    if !line.contains(key) {
        return None;
    }
    let rest = &line[line.find(key)? + key.len()..];
    // 取 tensor '...' 里的名字
    if let Some(start) = rest.find("tensor '") {
        let tail = &rest[start + "tensor '".len()..];
        if let Some(end) = tail.find('\'') {
            return Some(tail[..end].to_string());
        }
    }
    if let Some(start) = rest.find("tensor ") {
        let tail = rest[start + "tensor ".len()..].trim();
        let name: String = tail
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

/// token 数量的简短显示：1234 → 1.2k，1234567 → 1.2M
fn human_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// 从引擎日志里提取一次请求的 token 数，返回 (prompt, gen)。
///
/// llama.cpp / KVMem 用计时行：
///   `prompt eval time =  146.88 ms /    30 tokens (...)`  → (30, 0)
///   `       eval time =  125.45 ms /     8 tokens (...)`  → (0, 8)
/// NInfer（ninfer-kvmem-server）不打计时行，只在请求结束时汇总一行：
///   `req#1 done | openai-chat | output limit | prompt 54 | output 16 | ...`
///                                                                  → (54, 16)
fn parse_token_line(line: &str) -> Option<(u64, u64)> {
    // NInfer：必须是 “done” 那行，避免和 “started” 行里的 max output 重复计数
    if line.contains("req#") && line.contains(" done |") {
        let p = digits_after(line, "prompt ")?;
        let g = digits_after(line, "output ")?;
        return if p == 0 && g == 0 { None } else { Some((p, g)) };
    }
    // llama.cpp / KVMem
    let is_prompt = line.contains("prompt eval time");
    if !is_prompt && !line.contains("eval time =") {
        return None;
    }
    let pos = line.find("tokens")?;
    let before = &line[..pos];
    let slash = before.rfind('/')?;
    let digits: String = before[slash + 1..]
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    let n = digits.parse::<u64>().ok()?;
    if n == 0 {
        return None;
    }
    Some(if is_prompt { (n, 0) } else { (0, n) })
}

/// 取 `key` 后面紧跟的数字；同一行里 key 可能先出现在不跟数字的地方
/// （例如 `output limit` 之于 `output 16`），所以逐个出现位置往后试。
fn digits_after(line: &str, key: &str) -> Option<u64> {
    let mut from = 0usize;
    while let Some(rel) = line[from..].find(key) {
        let pos = from + rel + key.len();
        let digits: String = line[pos..]
            .chars()
            .skip_while(|c| *c == ' ')
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(v) = digits.parse::<u64>() {
            return Some(v);
        }
        from = pos;
    }
    None
}

/// 从日志行里提取 "unknown model architecture: 'xxx'" 的架构名
fn unknown_arch_of(line: &str) -> Option<String> {
    let key = "unknown model architecture:";
    let pos = line.find(key)?;
    let rest = &line[pos + key.len()..];
    let start = rest.find('\'')? + 1;
    let tail = &rest[start..];
    let end = tail.find('\'')?;
    Some(tail[..end].to_string())
}

// ---------------- 高级设置窗口（二级页面） ----------------

const APP_EXTRA: &[(&str, Kind, &str, &'static [&'static str])] = &[
    ("app.llamaExe", Kind::Text, "label.llamaExe", &[]),
    // KVMem 引擎专用（仅当引擎选 KVMem 时生效）
    ("app.kvmemExe", Kind::Text, "label.kvmemExe", &[]),
    ("app.ninferExe", Kind::Text, "label.ninferExe", &[]),
    ("kvmem.budget", Kind::Int, "label.kvmemBudget", &[]),
    ("kvmem.reserve", Kind::Int, "label.kvmemReserve", &[]),
    ("kvmem.hostMib", Kind::Int, "label.kvmemHostMib", &[]),
    ("kvmem.diskMib", Kind::Int, "label.kvmemDiskMib", &[]),
    ("kvmem.conversations", Kind::Int, "label.kvmemConversations", &[]),
    ("kvmem.diskPath", Kind::Text, "label.kvmemDiskPath", &[]),
];

/// 打开「参数解读」窗口：把当前启动参数逐条拆开，配中英文说明
/// 显存风险检查：服务运行中且显存几乎占满时，在日志上方给出置顶警示
fn check_vram_risk(g: &mut Gui) {
    let running = match g.shared.lock() {
        Ok(app) => app.launcher.status_json(&app.store.cfg).s("phase", "") != "stopped",
        Err(_) => false,
    };
    let s = crate::sysinfo::sample();
    let risk = running
        && s.vram_total_mb > 0
        && s.vram_used_mb as f64 / s.vram_total_mb as f64 >= 0.95;
    if risk {
        let msg = i18n(g, "warn.vramRisk");
        set_text(g.lbl_warn, &msg);
        if !g.vram_warned {
            g.vram_warned = true;
            let used = s.vram_used_mb as f64 / 1024.0;
            let total = s.vram_total_mb as f64 / 1024.0;
            if let Ok(app) = g.shared.lock() {
                app.launcher.push_log(
                    &format!(
                        "WARNING: VRAM {:.1}/{:.1} GB - risk of running out of memory; \
                         enable KV quantization (-ctk q8_0 -ctv q8_0) or use a KVMem engine",
                        used, total
                    ),
                    "",
                );
            }
        }
    } else {
        set_text(g.lbl_warn, "");
        g.vram_warned = false;
    }
}

/// 打开帮助文档：按当前界面语言选中中文或英文说明
fn open_help_doc(g: &mut Gui) {
    let lang = crate::i18n::resolve(&cfg(g));
    let zh = lang.starts_with("zh");
    let name = if zh { "docs/USAGE.zh-CN.md" } else { "docs/USAGE.md" };
    let base = crate::util::base_dir();
    let path = base.join(name);
    if path.is_file() {
        open_url(&path.to_string_lossy());
    } else {
        // 文档缺失时退回在线说明
        open_url(if zh {
            "https://github.com/heiheids/llamaui/blob/main/docs/USAGE.zh-CN.md"
        } else {
            "https://github.com/heiheids/llamaui/blob/main/docs/USAGE.md"
        });
    }
}

/// 按模型权重自动挑选引擎：> 8 GB 走 KVMem，其余保持 llama.cpp。
/// KVMem 引擎未安装时保持 llama.cpp 并在日志说明，避免“点了起不来”。
fn auto_pick_engine(g: &mut Gui) {
    const BIG_MODEL_GB: f64 = 8.0;
    let (size, name, path) = match g.shared.lock() {
        Ok(app) => app
            .lib
            .find(&g.selected_id)
            .map(|m| (m.size, m.name.clone(), m.path.clone()))
            .unwrap_or((0, String::new(), String::new())),
        Err(_) => return,
    };
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let gb = size as f64 / 1e9;

    // 目标引擎：.ninfer 只能走 NInfer；GGUF 按大小选；其余回 llama.cpp
    let want = if ext == "ninfer" {
        "NInfer"
    } else if ext == "gguf" && gb > BIG_MODEL_GB {
        "KVMem"
    } else {
        "llama.cpp"
    };

    let mut app = match g.shared.lock() {
        Ok(a) => a,
        Err(_) => return,
    };
    // 先把两个引擎的 exe 路径补上（没有就探测）
    if crate::process::Launcher::kvmem_exe(&app.store.cfg).is_empty() {
        let found = crate::process::Launcher::detect_kvmem_exe();
        if !found.is_empty() {
            app.store.cfg.set_path("app.kvmemExe", Json::str(&found));
            let _ = app.store.save();
        }
    }
    if crate::process::Launcher::ninfer_exe(&app.store.cfg).is_empty() {
        let found = crate::process::Launcher::detect_ninfer_exe();
        if !found.is_empty() {
            app.store.cfg.set_path("app.ninferExe", Json::str(&found));
            let _ = app.store.save();
        }
    }
    let have = |e: &str| -> bool {
        match e {
            "NInfer" => !crate::process::Launcher::ninfer_exe(&app.store.cfg).is_empty(),
            "KVMem" => !crate::process::Launcher::kvmem_exe(&app.store.cfg).is_empty(),
            _ => true,
        }
    };
    if !have(want) {
        // 需要的引擎没装：保持原引擎，并提示一次去下载
        if !app.store.cfg.b("app.engineNoticeShown", false) {
            app.store
                .cfg
                .set_path("app.engineNoticeShown", Json::Bool(true));
            let _ = app.store.save();
            app.launcher.push_log(
                &format!(
                    "{} ({:.1} GB, .{}) needs the {} engine, which is not installed - \
                     use the Get KVMem button to download the matching package",
                    name, gb, ext, want
                ),
                "",
            );
            let msg = i18n(g, "hint.engineMissing");
            set_text(g.lbl_hint, &msg);
        }
        return;
    }
    let current = crate::process::Launcher::engine_of(&app.store.cfg);
    if current == want {
        return;
    }
    app.store
        .cfg
        .set_path("launch.engine", Json::str(want));
    let _ = app.store.save();
    app.launcher.push_log(
        &format!("engine switched to {} for {} ({:.1} GB)", want, name, gb),
        "",
    );
    drop(app);
    layout::populate(g);
}

/// 打开「显存估算」窗口：按当前机器实际显存，列出几种可选配置
fn open_fit_window(g: &mut Gui) {
    // 用户可能用右上角 X 关掉了窗口（句柄已失效），先验证还活着再复用
    if g.fit_hwnd != 0 {
        if unsafe { IsWindow(g.fit_hwnd) } != 0 {
            show(g.fit_hwnd, true);
            return;
        }
        g.fit_hwnd = 0;
    }
    let title = format!("{} — {}", i18n(g, "button.estimate"), i18n(g, "app.title"));
    // 带可调边框，用户可以拖动改变大小
    let style = 0x00C0_0000 | 0x0008_0000 | 0x0004_0000;
    let hwnd = create_window(super::CLASS_NAME, &title, 0, style, 200, 90, 980, 620, 0, 0);
    if hwnd == 0 {
        return;
    }
    g.fit_hwnd = hwnd;

    let info_lbl = create_window(
        "STATIC",
        "",
        0,
        WS_CHILD | WS_VISIBLE | 0x0080,
        0,
        0,
        10,
        10,
        hwnd,
        0,
    );
    set_font(info_lbl, g.font);

    let list = create_window(
        "LISTBOX",
        "",
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | WS_BORDER | LBS_NOTIFY | LBS_HASSTRINGS,
        0,
        0,
        10,
        10,
        hwnd,
        ID_FIT_LIST,
    );
    set_font(list, g.font);

    let note = create_window(
        "STATIC",
        "",
        0,
        WS_CHILD | WS_VISIBLE | 0x0080,
        0,
        0,
        10,
        10,
        hwnd,
        0,
    );
    set_font(note, g.font);

    let apply = create_window(
        "BUTTON",
        &i18n(g, "button.applyProfile"),
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_FIT_APPLY,
    );
    set_font(apply, g.font);
    let close = create_window(
        "BUTTON",
        &i18n(g, "button.close"),
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_FIT_CLOSE,
    );
    set_font(close, g.font);

    FIT_WIN.with(|c| {
        *c.borrow_mut() = Some(FitWin {
            info: info_lbl,
            list,
            note,
            apply,
            close,
        })
    });

    // 估算数据
    let profiles = build_fit_profiles(g);
    let (gpu_name, vram_total, vram_used) = crate::sysinfo::sample().gpu_summary();
    {
        let mut l = fit_state().lock().unwrap_or_else(|e| e.into_inner());
        l.profiles = profiles;
        l.selected = 0;
    }
    fill_fit_list(g);
    fit_window_to_screen(hwnd, 980, 560);
    position_fit_window(g, &format!("{} · {} · {:.1}/{:.1} GB", gpu_name, i18n(g, "label.vram"), vram_used, vram_total));
    show(hwnd, true);
}

/// 组装候选配置
fn build_fit_profiles(g: &Gui) -> Vec<crate::fit::Profile> {
    let (path, size, ctx) = match g.shared.lock() {
        Ok(app) => {
            let m = app.lib.find(&g.selected_id);
            (
                m.map(|x| x.path.clone()).unwrap_or_default(),
                m.map(|x| x.size).unwrap_or(0),
                app.store.cfg.i("launch.ctxSize", 8192) as u32,
            )
        }
        Err(_) => (String::new(), 0, 8192),
    };
    let abs = crate::util::absolutize(&path);
    let info = crate::gguf::read(&abs).unwrap_or_default();
    let (_, vram_total, _) = crate::sysinfo::sample().gpu_summary();
    // 有实测校准就带上修正值，否则退回纯公式
    let extra = crate::vramcal::get(&g.selected_id)
        .map(|c| c.extra_mb_per_token)
        .unwrap_or(0.0);
    crate::fit::profiles_cal(&info, size, ctx, vram_total, extra)
}

fn fill_fit_list(g: &mut Gui) {
    if g.fit_hwnd == 0 {
        return;
    }
    let Some(ctl) = FIT_WIN.with(|c| *c.borrow()) else {
        return;
    };
    let st = fit_state().lock().unwrap_or_else(|e| e.into_inner());
    list_reset(ctl.list);
    for p in st.profiles.iter() {
        let mark = if p.fits { "OK" } else { " X" };
        let kv = if p.kv_type == "f16" { "f16".to_string() } else { p.kv_type.clone() };
        let moe = if p.n_cpu_moe > 0 {
            format!(", --n-cpu-moe {}", p.n_cpu_moe)
        } else {
            String::new()
        };
        list_add(
            ctl.list,
            &format!(
                "[{}] {:<18} -c {} -ngl {}{} -ctk {}   ~{:.1} GB",
                mark, p.title, p.ctx, p.ngl, moe, kv, p.est.total
            ),
        );
    }
    if !st.profiles.is_empty() {
        list_select(ctl.list, st.selected.min(st.profiles.len() - 1));
        let p = &st.profiles[st.selected.min(st.profiles.len() - 1)];
        set_text(ctl.note, &p.note);
    }
    drop(st);
    unsafe {
        InvalidateRect(g.fit_hwnd, std::ptr::null(), 1);
    }
}

// ---------------------------------------------------------------------------
// 二级窗口通用：按屏幕工作区自适应 + 支持拖动改变大小
// ---------------------------------------------------------------------------

/// 屏幕可用工作区（不含任务栏）
fn work_area() -> (i32, i32, i32, i32) {
    let mut r = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut r as *mut RECT as *mut core::ffi::c_void,
            0,
        )
    };
    if ok == 0 || r.right <= r.left || r.bottom <= r.top {
        // 读不到就退回一个保守值，保证按钮不会跑到屏幕外
        return (0, 0, 1280, 800);
    }
    (r.left, r.top, r.right - r.left, r.bottom - r.top)
}

/// 把窗口初始尺寸按 DPI 缩放，并夹到屏幕工作区内并居中。
/// 这一步是小屏 / 高 DPI 机器上「底部按钮看不见」的根本修复：
/// 原来窗口尺寸是写死的物理像素，而布局会按 DPI 放大，两者对不上。
fn fit_window_to_screen(hwnd: HWND, want_w: i32, want_h: i32) {
    let k = layout::scale(hwnd);
    let sc = |v: i32| layout::s(v, k);
    let (ox, oy, sw, sh) = work_area();
    let mut w = sc(want_w);
    let mut h = sc(want_h);
    let max_w = sw - sc(40);
    let max_h = sh - sc(80);
    if w > max_w {
        w = max_w;
    }
    if h > max_h {
        h = max_h;
    }
    let x = ox + ((sw - w) / 2).max(0);
    let y = oy + ((sh - h) / 2 - sc(16)).max(0);
    unsafe {
        SetWindowPos(hwnd, 0, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

/// 二级窗口尺寸变化后重排控件；主窗口不参与（布局是固定设计稿）
/// 高级设置窗口的竖向滚动（滚轮 / 滚动条）
pub fn on_adv_scroll(g: &mut Gui, hwnd: HWND, wp: usize, mouse: bool) -> bool {
    if hwnd != g.adv_hwnd || hwnd == 0 {
        return false;
    }
    let k = layout::scale(hwnd);
    let sc = |v: i32| layout::s(v, k);
    let step = if mouse { sc(60) } else { sc(32) };
    let code = if mouse {
        // 滚轮：wp 高字是有符号的滚动量
        let delta = ((wp >> 16) & 0xFFFF) as i16;
        if delta > 0 { -3 } else { 3 }
    } else {
        match wp & 0xFFFF {
            0 => -1,               // SB_LINEUP
            1 => 1,                // SB_LINEDOWN
            2 => -4,               // SB_PAGEUP
            3 => 4,                // SB_PAGEDOWN
            4 | 5 => {             // SB_THUMBPOSITION / SB_THUMBTRACK
                let pos = ((wp >> 16) & 0xFFFF) as i32;
                let old = g.adv_scroll;
                g.adv_scroll = pos;
                if old != g.adv_scroll {
                    position_advanced(g);
                }
                return true;
            }
            _ => 0,
        }
    };
    if code == 0 {
        return false;
    }
    let ratio = if mouse {
        (sc(60) as f32 / k.max(0.1)).round() as i32 / 2
    } else {
        32
    };
    g.adv_scroll += code * ratio.max(1);
    if g.adv_scroll < 0 {
        g.adv_scroll = 0;
    }
    position_advanced(g);
    true
}

pub fn on_window_resize(g: &mut Gui, hwnd: HWND) {
    if hwnd == g.hwnd {
        return;
    }
    if hwnd == g.fit_hwnd {
        let header = FIT_WIN.with(|c| c.borrow().map(|c| get_text(c.info)).unwrap_or_default());
        position_fit_window(g, &header);
    } else if hwnd == g.help_hwnd {
        position_help_window(g);
    } else if hwnd == g.adv_hwnd {
        position_advanced(g);
    }
}

/// 各二级窗口的最小尺寸（逻辑像素），WM_GETMINMAXINFO 用
pub fn min_window_size(g: &Gui, hwnd: HWND) -> Option<(i32, i32)> {
    let (w, h) = if hwnd == g.fit_hwnd {
        (560, 340)
    } else if hwnd == g.help_hwnd {
        (520, 360)
    } else if hwnd == g.adv_hwnd {
        (760, 420)
    } else {
        return None;
    };
    let k = layout::scale(hwnd);
    Some((layout::s(w, k), layout::s(h, k)))
}

fn position_fit_window(g: &mut Gui, header: &str) {    let k = layout::scale(g.fit_hwnd);
    let sc = |v: i32| layout::s(v, k);
    let (cw, ch) = client_size(g.fit_hwnd);
    let Some(ctl) = FIT_WIN.with(|c| *c.borrow()) else {
        return;
    };
    set_text(ctl.info, header);

    // 底部按钮锚定在客户区下沿：窗口再小也点得到
    let margin = sc(16);
    let btn_h = sc(34);
    let btn_y = ch - sc(14) - btn_h;
    let note_h = sc(36);
    let note_y = btn_y - sc(10) - note_h;
    let info_h = sc(20);
    let info_y = sc(12);
    let list_y = info_y + info_h + sc(10);
    let list_h = (note_y - sc(10) - list_y).max(sc(60));

    move_to(ctl.info, margin, info_y, cw - margin * 2, info_h);
    move_to(ctl.list, margin, list_y, cw - margin * 2, list_h);
    move_to(ctl.note, margin, note_y, cw - margin * 2, note_h);
    move_to(ctl.apply, cw - margin - sc(284), btn_y, sc(140), btn_h);
    move_to(ctl.close, cw - margin - sc(132), btn_y, sc(132), btn_h);
}

/// 应用选中的配置
fn apply_fit_profile(g: &mut Gui) {
    let Some(ctl) = FIT_WIN.with(|c| *c.borrow()) else {
        return;
    };
    let Some(idx) = list_sel(ctl.list) else {
        return;
    };
    let st = fit_state().lock().unwrap_or_else(|e| e.into_inner());
    let Some(p) = st.profiles.get(idx).cloned() else {
        return;
    };
    drop(st);
    if let Ok(mut app) = g.shared.lock() {
        app.store
            .cfg
            .set_path("launch.ctxSize", Json::Num(p.ctx as f64));
        app.store
            .cfg
            .set_path("launch.nGpuLayers", Json::Num(p.ngl as f64));
        app.store
            .cfg
            .set_path("cache.typeK", Json::str(&p.kv_type));
        app.store
            .cfg
            .set_path("cache.typeV", Json::str(&p.kv_type));
        app.store
            .cfg
            .set_path("gpu.nCpuMoe", Json::Num(p.n_cpu_moe as f64));
        app.store
            .cfg
            .set_path("launch.flashAttn", Json::str(if p.flash_attn { "on" } else { "off" }));
        let _ = app.store.save();
        app.launcher.push_log(
            &format!(
                "profile applied: -c {} -ngl {} -ctk {} -ctv {}{}",
                p.ctx,
                p.ngl,
                p.kv_type,
                p.kv_type,
                if p.n_cpu_moe > 0 {
                    format!(" --n-cpu-moe {}", p.n_cpu_moe)
                } else {
                    String::new()
                }
            ),
            "",
        );
    }
    layout::populate(g);
    layout::refresh_args_preview(g);
    set_text(g.lbl_hint, &i18n(g, "hint.profileApplied"));
}

thread_local! {
    static FIT_WIN: std::cell::RefCell<Option<FitWin>> = const { std::cell::RefCell::new(None) };
}

#[derive(Clone, Copy)]
struct FitWin {
    info: HWND,
    list: HWND,
    note: HWND,
    apply: HWND,
    close: HWND,
}

#[derive(Default, Clone)]
struct FitState {
    profiles: Vec<crate::fit::Profile>,
    selected: usize,
}

static FIT_STATE: std::sync::OnceLock<std::sync::Mutex<FitState>> = std::sync::OnceLock::new();

fn fit_state() -> &'static std::sync::Mutex<FitState> {
    FIT_STATE.get_or_init(|| std::sync::Mutex::new(FitState::default()))
}

fn open_param_help(g: &mut Gui) {
    if g.help_hwnd != 0 {
        show(g.help_hwnd, true);
        return;
    }
    let title = format!("{} — {}", i18n(g, "button.paramHelp"), i18n(g, "app.title"));
    // 带可调边框，用户可以拖动改变大小
    let style = 0x00C0_0000 | 0x0008_0000 | 0x0004_0000;
    let hwnd = create_window(super::CLASS_NAME, &title, 0, style, 180, 90, 920, 720, 0, 0);
    if hwnd == 0 {
        return;
    }
    g.help_hwnd = hwnd;

    let view = create_window(
        "EDIT",
        "",
        0,
        WS_CHILD | WS_VISIBLE | WS_VSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY,
        0,
        0,
        10,
        10,
        hwnd,
        0,
    );
    set_font(view, g.font);
    layout::subclass_edit(view);
    g.help_view = view;

    let btn = create_window(
        "BUTTON",
        &i18n(g, "button.back"),
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_HELP_CLOSE,
    );
    set_font(btn, g.font);
    g.btn_help_close = btn;

    g.btn_help_translate = create_window(
        "BUTTON",
        &i18n(g, "button.translate"),
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_HELP_TRANSLATE,
    );
    set_font(g.btn_help_translate, g.font);

    set_text(view, &build_param_help_text(g));

    fit_window_to_screen(hwnd, 920, 700);
    position_help_window(g);
    show(hwnd, true);
}

/// 参数解读窗口的自适应布局：文本框占满上方，按钮贴底
fn position_help_window(g: &mut Gui) {
    if g.help_hwnd == 0 || unsafe { IsWindow(g.help_hwnd) } == 0 {
        return;
    }
    let k = layout::scale(g.help_hwnd);
    let sc = |v: i32| layout::s(v, k);
    let (cw, ch) = client_size(g.help_hwnd);

    let margin = sc(14);
    let btn_h = sc(34);
    let btn_y = ch - sc(12) - btn_h;
    let view_y = sc(12);
    let view_h = (btn_y - sc(10) - view_y).max(sc(120));

    move_to(g.help_view, margin, view_y, cw - margin * 2, view_h);
    move_to(g.btn_help_translate, cw - margin - sc(112) - sc(8) - sc(150), btn_y, sc(150), btn_h);
    move_to(
        g.btn_help_close,
        cw - margin - sc(112),
        btn_y,
        sc(112),
        btn_h,
    );
}

// ---------------- 参数解读：本地模型翻译 ----------------

#[derive(Default, Clone)]
struct TrState {
    running: bool,
    finished: bool,
    ok: usize,
    total: usize,
    error: String,
}

static TR_STATE: std::sync::OnceLock<std::sync::Mutex<TrState>> = std::sync::OnceLock::new();

fn tr_state() -> &'static std::sync::Mutex<TrState> {
    TR_STATE.get_or_init(|| std::sync::Mutex::new(TrState::default()))
}

/// 服务是否空闲（没有正在处理的槽位）。拿不到状态时按空闲处理，
/// 因为自动翻译每个版本只跑一次，代价可控。
fn server_idle(host: &str, port: u16, key: &str) -> bool {
    match crate::web::client::request(host, port, "GET", "/slots", Some(key), None) {
        Ok(r) if r.code == 200 => match crate::json::parse(&r.body) {
            Ok(crate::json::Json::Arr(items)) => {
                !items.iter().any(|s| s.b("is_processing", false))
            }
            _ => true,
        },
        _ => true,
    }
}

/// 每 30 秒检查一次：开启「新版本自动生成中文解读」且服务空闲时自动跑一次
pub fn auto_translate_tick(g: &mut Gui) {
    AUTO_TICK.with(|t| {
        let mut n = t.get();
        n += 1;
        if n < 30 {
            t.set(n);
            return;
        }
        t.set(0);
    });
    {
        let running = tr_state().lock().map(|s| s.running).unwrap_or(false);
        if running {
            return;
        }
    }
    let (enabled, host, port, key, phase, done_for) = {
        let c = cfg(g);
        let host = "127.0.0.1".to_string();
        let port = c.i("network.port", 9931) as u16;
        let key = c
            .path("network.apiKeys")
            .and_then(|a| a.as_arr())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let phase = g
            .shared
            .lock()
            .map(|a| a.launcher.status_json(&a.store.cfg).s("phase", ""))
            .unwrap_or_default();
        (
            c.b("app.autoTranslate", false),
            host,
            port,
            key,
            phase,
            c.s("app.autoTranslatedFor", ""),
        )
    };
    if !enabled || phase != "running" {
        return;
    }
    let hash = crate::paramspec::current()
        .map(|d| d.help_hash.clone())
        .unwrap_or_default();
    if hash.is_empty() || done_for == hash {
        return; // 这个版本已经自动译过
    }
    if !server_idle(&host, port, &key) {
        return;
    }
    // 有活可干才动手
    let has_pending = {
        let c = cfg(g);
        let path = g
            .shared
            .lock()
            .ok()
            .and_then(|a| a.lib.find(&g.selected_id).map(|m| m.path.clone()))
            .unwrap_or_default();
        let args = crate::settings::build_args(&c, &path);
        let mut specs = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for tok in args.iter() {
            if !tok.starts_with('-') || !seen.insert(tok.clone()) {
                continue;
            }
            if crate::paramhelp::lookup(tok).is_some() {
                continue;
            }
            if let Some(s) = crate::paramspec::current().and_then(|d| d.find(tok)) {
                specs.push((
                    s.flag.clone(),
                    s.desc.clone(),
                    s.default.clone(),
                    s.allowed.clone(),
                ));
            }
        }
        !crate::translate::pending(&specs).is_empty()
    };
    if !has_pending {
        // 没有待译项，记下这个版本已处理，避免每 30 秒重复检查
        mark_auto_translated(g, &hash);
        return;
    }
    if let Ok(app) = g.shared.lock() {
        app.launcher.push_log(&i18n(g, "hint.autoTranslating"), "");
    }
    AUTO_RUN.with(|f| f.set(true));
    start_translate(g);
}

/// 记下「这个 help 版本已自动译过」
fn mark_auto_translated(g: &mut Gui, hash: &str) {
    if hash.is_empty() {
        return;
    }
    if let Ok(mut app) = g.shared.lock() {
        if app.store.cfg.s("app.autoTranslatedFor", "") == hash {
            return;
        }
        app.store
            .cfg
            .set_path("app.autoTranslatedFor", Json::str(hash));
        let _ = app.store.save();
    }
}

thread_local! {
    /// 自动翻译的轮询计数（每 30 个 tick 检查一次）
    static AUTO_TICK: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// 本轮翻译是否由自动流程触发
    static AUTO_RUN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 点「生成中文解读」：在后台线程里调用本地 llama-server 逐批翻译
fn start_translate(g: &mut Gui) {
    {
        let st = tr_state().lock().map(|s| s.running).unwrap_or(false);
        if st {
            set_text(g.lbl_hint, &i18n(g, "hint.translating"));
            return;
        }
    }
    // 收集需要翻译的参数（当前启动参数里、没有手写中文的那些）
    let args: Vec<String> = {
        let c = cfg(g);
        let path = g
            .shared
            .lock()
            .ok()
            .and_then(|a| a.lib.find(&g.selected_id).map(|m| m.path.clone()))
            .unwrap_or_default();
        crate::settings::build_args(&c, &path)
    };
    let mut specs: Vec<(String, String, String, Vec<String>)> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for tok in args.iter() {
        if !tok.starts_with('-') || !seen.insert(tok.clone()) {
            continue;
        }
        if crate::paramhelp::lookup(tok).is_some() {
            continue; // 已有手写中文
        }
        if let Some(s) = crate::paramspec::current().and_then(|d| d.find(tok)) {
            specs.push((
                s.flag.clone(),
                s.desc.clone(),
                s.default.clone(),
                s.allowed.clone(),
            ));
        }
    }
    let pend = crate::translate::pending(&specs);
    if let Ok(app) = g.shared.lock() {
        app.launcher.push_log(
            &format!(
                "translate: {} 个参数待翻译 (候选 {})",
                pend.len(),
                specs.len()
            ),
            "",
        );
        for p in pend.iter().take(8) {
            app.launcher.push_log(&format!("  - {}", p.flag), "");
        }
    }
    if pend.is_empty() {
        set_text(g.lbl_hint, &i18n(g, "hint.translateNothing"));
        return;
    }

    // 取服务信息
    let (host, port, key) = {
        let c = cfg(g);
        let host = "127.0.0.1".to_string();
        let port = c.i("network.port", 9931) as u16;
        let key = c
            .path("network.apiKeys")
            .and_then(|a| a.as_arr())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        (host, port, key)
    };
    let version = crate::paramspec::current()
        .map(|d| d.version.clone())
        .unwrap_or_default();

    let total = pend.len();
    if let Ok(mut s) = tr_state().lock() {
        *s = TrState {
            running: true,
            finished: false,
            ok: 0,
            total,
            error: String::new(),
        };
    }
    set_text(
        g.lbl_hint,
        &format!("{} 0/{}", i18n(g, "hint.translating"), total),
    );

    std::thread::spawn(move || {
        let mut ok_total = 0usize;
        let mut err = String::new();
        for chunk in pend.chunks(6) {
            let (ok, e) = crate::translate::translate_batch(
                &host, port, &key, chunk, &version, "local",
            );
            ok_total += ok;
            let failed = !e.is_empty();
            if failed && err.is_empty() {
                err = e;
            }
            if let Ok(mut s) = tr_state().lock() {
                s.ok = ok_total;
            }
            if failed {
                break;
            }
        }
        if let Ok(mut s) = tr_state().lock() {
            s.running = false;
            s.finished = true;
            s.ok = ok_total;
            s.error = err;
        }
    });
}

/// 主循环里轮询翻译进度：完成后刷新解读窗口
pub fn poll_translate(g: &mut Gui) {
    let snap = match tr_state().lock() {
        Ok(mut s) => {
            if !s.finished {
                return;
            }
            let snap = s.clone();
            // 只处理一次
            s.finished = false;
            snap
        }
        Err(_) => return,
    };
    if snap.error.is_empty() {
        let msg = format!(
            "{} {}/{}",
            i18n(g, "hint.translateDone"),
            snap.ok,
            snap.total
        );
        set_text(g.lbl_hint, &msg);
        if let Ok(app) = g.shared.lock() {
            app.launcher.push_log(&msg, "");
        }
        if AUTO_RUN.with(|f| f.replace(false)) {
            let hash = crate::paramspec::current()
                .map(|d| d.help_hash.clone())
                .unwrap_or_default();
            mark_auto_translated(g, &hash);
        }
    } else {
        let msg = format!("{}: {}", i18n(g, "hint.translateFailed"), snap.error);
        set_text(g.lbl_hint, &msg);
        if let Ok(app) = g.shared.lock() {
            app.launcher.push_log(&msg, "");
        }
    }
    // 解读窗口开着就重绘
    if g.help_hwnd != 0 && g.help_view != 0 {
        let text = build_param_help_text(g);
        set_text(g.help_view, &text);
    }
}

/// 把当前参数逐条排版成“参数 + 中文 + 英文”的文本
fn build_param_help_text(g: &Gui) -> String {
    let args: Vec<String> = {
        let c = cfg(g);
        let path = g
            .shared
            .lock()
            .ok()
            .and_then(|a| a.lib.find(&g.selected_id).map(|m| m.path.clone()))
            .unwrap_or_default();
        crate::settings::build_args(&c, &path)
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    };

    let mut out = String::new();
    out.push_str(&i18n(g, "help.header"));
    out.push_str("\r\n\r\n");

    // 逐条解析：遇到 --flag 就开一组，后面的非 flag 值归到该组
    let mut i = 0usize;
    while i < args.len() {
        let tok = &args[i];
        if tok.starts_with('-') {
            let mut value = String::new();
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                value = args[i + 1].clone();
            }
            let line = if value.is_empty() {
                tok.clone()
            } else {
                format!("{} {}", tok, redact(&value))
            };
            out.push_str(&format!("* {}\r\n", line));
            match crate::paramhelp::lookup(tok) {
                Some(item) => {
                    out.push_str(&format!("   {}\r\n", item.zh));
                    out.push_str(&format!("   EN: {}\r\n", item.en));
                }
                None => {
                    // 本程序没有手写解读：优先用本地模型译好的中文，其次退回官方英文原文
                    let spec = crate::paramspec::current().and_then(|d| d.find(tok));
                    let hash = spec.as_ref().map(|s| s.desc_hash()).unwrap_or_default();
                    let cached = crate::translate::lookup(tok, &hash);
                    match (cached, spec) {
                        (Some(zh), Some(s)) => {
                            out.push_str(&format!("   {}\r\n", zh));
                            if !s.desc.is_empty() {
                                out.push_str(&format!("   EN: {}\r\n", s.desc));
                            }
                        }
                        (Some(zh), None) => {
                            out.push_str(&format!("   {}\r\n", zh));
                        }
                        (None, Some(s)) => {
                            out.push_str("   （自动解析自当前 llama-server --help）\r\n");
                            if !s.desc.is_empty() {
                                out.push_str(&format!("   EN: {}\r\n", s.desc));
                            }
                            let mut extra = String::new();
                            if !s.allowed.is_empty() {
                                extra.push_str(&format!("可选值: {}", s.allowed.join(" / ")));
                            }
                            if !s.default.is_empty() {
                                if !extra.is_empty() {
                                    extra.push_str("；");
                                }
                                extra.push_str(&format!("默认: {}", s.default));
                            }
                            if !extra.is_empty() {
                                out.push_str(&format!("   {}\r\n", extra));
                            }
                        }
                        (None, None) => {
                            out.push_str("   （暂无解读，详见 llama.cpp 官方文档）\r\n");
                        }
                    }
                }
            }
            i += if value.is_empty() { 1 } else { 2 };
        } else {
            // 前面是 -m 这类已被收进上一组的独立值，正常不会走到这里
            i += 1;
        }
    }
    out
}

/// 密钥等敏感值打码显示
fn redact(v: &str) -> String {
    if v.starts_with("sk-") && v.len() > 10 {
        format!("{}…{}", &v[..7], &v[v.len() - 4..])
    } else {
        v.to_string()
    }
}

/// 打开「自动发现的新参数」窗口：把 --help 里有、本程序还没建模的参数做成分页表单
fn open_auto_window(g: &mut Gui) {
    if g.auto_hwnd != 0 {
        show(g.auto_hwnd, true);
        return;
    }
    let title = format!("{} — {}", i18n(g, "label.autoParams"), i18n(g, "app.title"));
    let style = 0x00C0_0000 | 0x0008_0000;
    let hwnd = create_window(super::CLASS_NAME, &title, 0, style, 200, 80, 1120, 720, 0, 0);
    if hwnd == 0 {
        return;
    }
    g.auto_hwnd = hwnd;
    g.auto_page = 0;
    g.auto_fields.clear();
    AUTO_BUF.with(|b| b.borrow_mut().clear());

    let hint = create_window(
        "STATIC",
        &i18n(g, "hint.autoParams"),
        0,
        WS_CHILD | WS_VISIBLE | 0x0080,
        0,
        0,
        10,
        10,
        hwnd,
        0,
    );
    set_font(hint, g.font);

    // 筛选框
    let filter_label = create_window(
        "STATIC",
        &i18n(g, "label.autoFilter"),
        0,
        WS_CHILD | WS_VISIBLE | 0x0080,
        0,
        0,
        10,
        10,
        hwnd,
        0,
    );
    set_font(filter_label, g.font);
    let filter = create_window(
        "EDIT",
        "",
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL,
        0,
        0,
        10,
        10,
        hwnd,
        ID_AUTO_FILTER,
    );
    set_font(filter, g.font);
    layout::subclass_edit(filter);
    g.auto_filter.clear();

    let prev = create_window(
        "BUTTON",
        "<",
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_AUTO_PREV,
    );
    set_font(prev, g.font);
    let next = create_window(
        "BUTTON",
        ">",
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_AUTO_NEXT,
    );
    set_font(next, g.font);
    let save = create_window(
        "BUTTON",
        &i18n(g, "button.saveClose"),
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_AUTO_SAVE,
    );
    set_font(save, g.font);

    AUTO_WIN_CTL.with(|c| {
        *c.borrow_mut() = Some(AutoWinCtl {
            hint,
            prev,
            next,
            save,
            filter,
            filter_label,
        })
    });
    build_auto_page(g);
    position_auto_window(g);
    show(hwnd, true);
}

/// 每页显示多少个参数
const AUTO_PER_PAGE: usize = 18;

thread_local! {
    static AUTO_WIN_CTL: std::cell::RefCell<Option<AutoWinCtl>> =
        const { std::cell::RefCell::new(None) };
    /// 翻页时暂存已填写的值，避免换页丢输入
    static AUTO_BUF: std::cell::RefCell<std::collections::BTreeMap<String, Json>> =
        std::cell::RefCell::new(std::collections::BTreeMap::new());
}

/// 读当前页控件的值（空文本忽略）
fn read_auto_controls(g: &Gui) -> Vec<(String, Json)> {
    let mut out = Vec::new();
    for ac in g.auto_fields.iter() {
        if ac.hwnd == 0 {
            continue;
        }
        let val = match ac.kind {
            Kind::Bool => Json::Bool(check_get(ac.hwnd)),
            Kind::Select => {
                let t = combo_text(ac.hwnd);
                if t.trim().is_empty() {
                    continue;
                }
                Json::str(&t)
            }
            _ => {
                let t = get_text(ac.hwnd);
                if t.trim().is_empty() {
                    continue;
                }
                Json::str(t.trim())
            }
        };
        out.push((ac.key.clone(), val));
    }
    out
}

/// 把当前页的值存进缓冲区（翻页前调用）
fn snapshot_auto(g: &Gui) {
    let pairs = read_auto_controls(g);
    AUTO_BUF.with(|b| {
        let mut b = b.borrow_mut();
        for (k, v) in pairs {
            b.insert(k, v);
        }
    });
}

#[derive(Clone, Copy)]
struct AutoWinCtl {
    hint: HWND,
    prev: HWND,
    next: HWND,
    save: HWND,
    filter: HWND,
    filter_label: HWND,
}

/// 按关键字过滤后的参数列表（匹配 flag 或英文说明）
fn filtered_auto_params(g: &Gui) -> Vec<crate::paramspec::ParamSpec> {
    let all = crate::paramspec::auto_params();
    let q = g.auto_filter.trim().to_lowercase();
    if q.is_empty() {
        return all;
    }
    all.into_iter()
        .filter(|p| {
            p.flag.to_lowercase().contains(&q) || p.desc.to_lowercase().contains(&q)
        })
        .collect()
}

/// 按当前页重建控件
fn build_auto_page(g: &mut Gui) {
    for ac in g.auto_fields.iter() {
        if ac.hwnd != 0 {
            unsafe {
                DestroyWindow(ac.hwnd);
            }
        }
        if ac.label != 0 {
            unsafe {
                DestroyWindow(ac.label);
            }
        }
    }
    g.auto_fields.clear();
    let hwnd = g.auto_hwnd;
    if hwnd == 0 {
        return;
    }
    let all = filtered_auto_params(g);
    let start = g.auto_page * AUTO_PER_PAGE;
    let page: Vec<_> = all
        .iter()
        .skip(start)
        .take(AUTO_PER_PAGE)
        .cloned()
        .collect();
    for (i, p) in page.iter().enumerate() {
        let kind = match p.kind {
            "bool" => Kind::Bool,
            "enum" => Kind::Select,
            _ => Kind::Text,
        };
        let class = match kind {
            Kind::Bool => "BUTTON",
            Kind::Select => "COMBOBOX",
            _ => "EDIT",
        };
        let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP;
        style |= match kind {
            Kind::Bool => BS_AUTOCHECKBOX,
            Kind::Select => CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_VSCROLL,
            _ => ES_AUTOHSCROLL,
        };
        let ctl = create_window(class, "", 0, style, 0, 0, 10, 10, hwnd, ID_AUTO_BASE + i);
        set_font(ctl, g.font);
        if kind == Kind::Select {
            for opt in p.allowed.iter() {
                combo_add(ctl, opt);
            }
        } else if kind != Kind::Bool {
            layout::subclass_edit(ctl);
        }
        let mut text = p.flag.clone();
        if !p.allowed.is_empty() {
            text.push_str(&format!(" [{}]", p.allowed.join("/")));
        }
        let label = create_window(
            "STATIC",
            &text,
            0,
            WS_CHILD | WS_VISIBLE | 0x0080,
            0,
            0,
            10,
            10,
            hwnd,
            0,
        );
        set_font(label, g.font);
        g.auto_fields.push(AutoCtl {
            key: p.config_key(),
            flag: p.flag.clone(),
            kind,
            hwnd: ctl,
            label,
        });
    }
    fill_auto_controls(g);
    // 销毁旧控件后必须整体重绘，否则原位置会留下未擦除的旧像素
    if g.auto_hwnd != 0 {
        unsafe {
            InvalidateRect(g.auto_hwnd, std::ptr::null(), 1);
        }
    }
    // 页码提示
    let total = filtered_auto_params(g).len();
    let pages = total.div_ceil(AUTO_PER_PAGE).max(1);
    if let Some(ctl) = AUTO_WIN_CTL.with(|c| *c.borrow()) {
        set_text(
            ctl.hint,
            &format!(
                "{}  ({} / {})",
                i18n(g, "hint.autoParams"),
                g.auto_page + 1,
                pages
            ),
        );
    }
}

/// 把 config.auto.*（优先用翻页缓冲区）的值填进当前页控件
fn fill_auto_controls(g: &Gui) {
    let c = cfg(g);
    AUTO_BUF.with(|b| {
        let buf = b.borrow();
        for ac in g.auto_fields.iter() {
            let v = buf.get(&ac.key).cloned().or_else(|| {
                c.path("auto")
                    .and_then(|a| a.path(&ac.key))
                    .cloned()
            });
            let v = v.unwrap_or(Json::Null);
            match ac.kind {
                Kind::Bool => check_set(ac.hwnd, v.as_bool().unwrap_or(false)),
                Kind::Select => {
                    let cur = v.as_str().unwrap_or("");
                    let mut idx = 0;
                    for (i, opt) in crate::paramspec::param_options(&ac.flag).iter().enumerate() {
                        if opt == cur {
                            idx = i;
                        }
                    }
                    combo_select(ac.hwnd, idx);
                }
                _ => set_text(ac.hwnd, v.as_str().unwrap_or("")),
            }
        }
    });
}

/// 摆放自动发现窗口里的控件
fn position_auto_window(g: &mut Gui) {
    if g.auto_hwnd == 0 {
        return;
    }
    let k = layout::scale(g.auto_hwnd);
    let sc = |v: i32| layout::s(v, k);
    let (cw, _ch) = client_size(g.auto_hwnd);
    let x0 = 18;
    let col_w = ((cw - sc(36)) / 3).max(sc(260));
    let row_h = 74;
    let Some(ctl) = AUTO_WIN_CTL.with(|c| *c.borrow()) else {
        return;
    };
    move_to(ctl.filter_label, sc(x0), sc(46), sc(52), sc(18));
    move_to(ctl.filter, sc(x0 + 56), sc(44), sc(240), sc(28));
    move_to(ctl.hint, sc(x0 + 306), sc(47), sc(cw - sc(36) - 306), sc(24));
    let mut i = 0usize;
    for ac in g.auto_fields.iter() {
        let col = (i % 3) as i32;
        let row = (i / 3) as i32;
        let cx = x0 + col * (col_w + 10);
        let cyy = 84 + row * row_h;
        // 勾选项和高级设置窗口保持一致：方框在前、文字在后，同一行
        if ac.kind == Kind::Bool {
            move_to(ac.hwnd, sc(cx), sc(cyy + 2), sc(24), sc(24));
            move_to(ac.label, sc(cx + 26), sc(cyy + 5), sc(col_w - 36), sc(17));
        } else {
            move_to(ac.label, sc(cx), sc(cyy), sc(col_w - 10), sc(17));
            move_to(ac.hwnd, sc(cx), sc(cyy + 19), sc(col_w - 20), sc(26));
        }
        i += 1;
    }
    let rows = i.div_ceil(3).max(1) as i32;
    let cy = 84 + rows * row_h + 8;
    move_to(ctl.prev, sc(x0), sc(cy), sc(56), sc(30));
    move_to(ctl.next, sc(x0 + 64), sc(cy), sc(56), sc(30));
    move_to(ctl.save, sc(cw - sc(130)), sc(cy), sc(112), sc(32));
}

/// 把自动发现窗口的值写回配置（含翻页缓冲区里收集的其它页）
fn save_auto_values(g: &mut Gui) {
    snapshot_auto(g);
    let pairs: Vec<(String, Json)> = AUTO_BUF.with(|b| {
        b.borrow().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    });
    let n = pairs.len();
    if let Ok(mut app) = g.shared.lock() {

        for (k, v) in pairs.iter() {
            // 布尔 false 且配置里原本没有这个键 → 不写，保持配置干净
            if let Json::Bool(false) = v {
                let has = app.store.cfg.path(&format!("auto.{}", k)).is_some();
                if !has {
                    continue;
                }
            }
            app.store.cfg.set_path(&format!("auto.{}", k), v.clone());
        }
        let _ = app.store.save();
        app.launcher
            .push_log(&format!("auto params saved: {} item(s)", n), "");
    }
    refresh_status(g);
}

/// 「浏览…」：手动选择 llama-server 可执行文件
fn browse_llama_exe(g: &mut Gui) {
    let title = i18n(g, "label.llamaExe");
    let Some(path) = open_exe_dialog(g.adv_hwnd.max(g.hwnd), &title) else {
        return;
    };
    if let Some(ctl) = g.adv_fields.iter().find(|c| c.key == "app.llamaExe") {
        set_text(ctl.hwnd, &path);
    }
    if let Ok(mut app) = g.shared.lock() {
        app.store.cfg.set_path("app.llamaExe", Json::str(&path));
        let _ = app.store.save();
        app.launcher
            .push_log(&format!("llama-server path set: {}", path), "");
    }
    set_text(g.lbl_hint, &path);
}

/// 把配置里的扫描目录填进列表（相对路径显示成「./name（程序自带）」）
fn fill_scan_list(g: &mut Gui) {
    if g.scan_list == 0 {
        return;
    }
    let dirs = scan_dir_items(g);
    list_reset(g.scan_list);
    for d in dirs.iter() {
        list_add(g.scan_list, d);
    }
}

/// 读配置里的扫描目录（原始写法，保留相对/绝对）
fn scan_dir_items(g: &Gui) -> Vec<String> {
    let c = cfg(g);
    let mut dirs: Vec<String> = c
        .path("app.scanDirs")
        .and_then(|v| v.as_arr())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    // 补上程序自带但还没写进配置的目录（相对路径）
    for d in crate::models::Library::default_scan_dirs() {
        if !dirs.iter().any(|x| x.eq_ignore_ascii_case(&d)) {
            dirs.push(d);
        }
    }
    dirs.iter()
        .map(|d| {
            if crate::models::is_builtin_scan_dir(d) {
                format!("{}（{}）", d, i18n(g, "label.builtinDir"))
            } else {
                d.clone()
            }
        })
        .collect()
}

/// 从列表里取回原始路径（去掉「（程序自带）」后缀）
fn scan_item_raw(g: &Gui, idx: usize) -> String {
    let text = list_text(g.scan_list, idx);
    let suffix = format!("（{}）", i18n(g, "label.builtinDir"));
    text.trim_end_matches(&suffix).to_string()
}

/// 右键菜单：删除扫描目录（程序自带的相对目录不允许删）
pub fn scan_dir_menu(g: &mut Gui, hwnd: HWND, lp: LPARAM) {
    let (x, y) = if lp == -1 || (lp as isize) == 0 {
        let mut pt = POINT { x: 0, y: 0 };
        unsafe {
            GetCursorPos(&mut pt);
        }
        (pt.x, pt.y)
    } else {
        ((lp & 0xFFFF) as i32, ((lp >> 16) & 0xFFFF) as i32)
    };
    let Some(idx) = list_index_from_point(g.scan_list, x, y) else {
        return;
    };
    let raw = scan_item_raw(g, idx);
    if crate::models::is_builtin_scan_dir(&raw) {
        // 提示要弹窗：高级设置窗口盖住了主窗口的提示行
        let title = i18n(g, "label.scanDirs");
        msg_box(
            hwnd,
            &i18n(g, "hint.builtinDirLocked"),
            &title,
            MB_OK | MB_ICONINFORMATION,
        );
        return;
    }
    let item = i18n(g, "menu.removeDir");
    let cmd = popup_menu(hwnd, &[(layout::ID_CTX_SCAN_DIR, item)], x, y);
    if cmd == layout::ID_CTX_SCAN_DIR {
        remove_scan_dir(g, &raw);
    }
}

/// 从配置里移除一个扫描目录并刷新列表
fn remove_scan_dir(g: &mut Gui, raw: &str) {
    if let Ok(mut app) = g.shared.lock() {
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
        let before = dirs.len();
        dirs.retain(|d| !d.eq_ignore_ascii_case(raw));
        if dirs.len() != before {
            app.store.cfg.set_path(
                "app.scanDirs",
                Json::Arr(dirs.iter().map(|d| Json::str(d)).collect()),
            );
            let _ = app.store.save();
            app.launcher
                .push_log(&format!("scan dir removed: {}", raw), "");
        }
    }
    fill_scan_list(g);
    set_text(g.lbl_hint, &i18n(g, "hint.dirRemoved"));
}

// ---------------------------------------------------------------------------
// 数据监控窗口
// ---------------------------------------------------------------------------

/// 把当前连接信息（端口 / 密钥 / 运行态 / 日志路径）同步给采集线程，
/// 并让监控窗口重绘一次。主窗口每秒调用。
fn sync_monitor(g: &mut Gui) {
    let (port, key, running, model, engine, uptime, log_path, ctx_cfg, slots_cfg) =
        match g.shared.lock() {
            Ok(app) => {
                let st = app.launcher.status_json(&app.store.cfg);
                let name = app
                    .lib
                    .find(&app.store.selected_model())
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                let keys = crate::settings::api_keys(&app.store.cfg);
                (
                    st.i("port", 0).max(0) as u16,
                    keys.first().cloned().unwrap_or_default(),
                    st.s("phase", "") == "running",
                    name,
                    crate::process::Launcher::engine_of(&app.store.cfg),
                    st.i("uptime", 0).max(0) as u64,
                    st.s("logPath", ""),
                    app.store.cfg.i("launch.ctxSize", 0).max(0) as u64,
                    app.store.cfg.i("launch.parallel", 1).max(1) as u32,
                )
            }
            Err(_) => (
                0,
                String::new(),
                false,
                String::new(),
                String::new(),
                0,
                String::new(),
                0,
                0,
            ),
        };
    crate::monitor::update_ctx(port, &key, running, &log_path, ctx_cfg, slots_cfg);
    crate::monitor::update_labels(&model, &engine, uptime);
    if g.mon_hwnd != 0 && unsafe { IsWindow(g.mon_hwnd) } != 0 {
        unsafe {
            InvalidateRect(g.mon_hwnd, std::ptr::null(), 0);
        }
    }
}

fn open_monitor_window(g: &mut Gui) {
    // 已经开着就前置，不重复建窗
    if g.mon_hwnd != 0 {
        if unsafe { IsWindow(g.mon_hwnd) } != 0 {
            show(g.mon_hwnd, true);
            sync_monitor(g);
            return;
        }
        g.mon_hwnd = 0;
    }
    let title = format!("{} — {}", i18n(g, "button.monitor"), i18n(g, "app.title"));
    // 标题栏 + 关闭按钮 + 可调大小
    let style = 0x00C0_0000 | 0x0008_0000 | 0x0004_0000;
    let hwnd = create_window(super::CLASS_NAME, &title, 0, style, 220, 100, 920, 640, 0, 0);
    if hwnd == 0 {
        return;
    }
    g.mon_hwnd = hwnd;
    super::MON_HWND.store(hwnd as isize, std::sync::atomic::Ordering::Relaxed);
    crate::monitor::start();
    sync_monitor(g);
    show(hwnd, true);
    unsafe {
        InvalidateRect(hwnd, std::ptr::null(), 1);
    }
}

fn open_advanced(g: &mut Gui) {
    if g.adv_hwnd != 0 {
        show(g.adv_hwnd, true);
        return;
    }
    let title = format!("{} — {}", i18n(g, "section.advanced"), i18n(g, "app.title"));
    // 带可调边框 + 竖向滚动条：小屏上内容放不下时可以滚动，底部按钮始终可见
    let style = 0x00C0_0000 | 0x0008_0000 | 0x0004_0000 | WS_VSCROLL;
    let hwnd = create_window(super::CLASS_NAME, &title, 0, style, 150, 30, 1060, 950, 0, 0);
    if hwnd == 0 {
        return;
    }
    g.adv_hwnd = hwnd;

    let mut ctl_list: Vec<Ctl> = Vec::new();
    for (idx, f) in FIELDS.iter().enumerate() {
        if f.section != "advanced" {
            continue;
        }
        let (hwnd_ctl, label) = create_adv_field(
            g,
            hwnd,
            f.kind,
            f.options,
            ID_ADV_BASE + idx,
            f.label,
        );
        ctl_list.push(Ctl {
            key: f.key,
            kind: f.kind,
            hwnd: hwnd_ctl,
            label,
            options: f.options,
        });
    }
    for (i, (key, kind, label_key, opts)) in APP_EXTRA.iter().enumerate() {
        let (hwnd_ctl, label) =
            create_adv_field(g, hwnd, *kind, opts, ID_ADV_BASE + 500 + i, label_key);
        ctl_list.push(Ctl {
            key,
            kind: *kind,
            hwnd: hwnd_ctl,
            label,
            options: opts,
        });
    }
    g.adv_fields = ctl_list;

    // 底部按钮条的遮挡底板：内容滚动时会经过按钮下方，
    // 用一块与页面同色的 STATIC 压在上面，看起来才干净。
    // 必须在按钮之前创建，z 序才会落在按钮下方。
    g.adv_footer = create_window(
        "STATIC",
        "",
        0,
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        0,
        0,
        10,
        10,
        hwnd,
        0,
    );
    set_font(g.adv_footer, g.font);

    g.btn_adv_save = create_window(
        "BUTTON",
        &i18n(g, "button.save"),
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_ADV_SAVE,
    );
    set_font(g.btn_adv_save, g.font);
    g.btn_adv_close = create_window(
        "BUTTON",
        &i18n(g, "button.back"),
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_ADV_CLOSE,
    );
    set_font(g.btn_adv_close, g.font);

    // llama-server 路径旁的「浏览…」按钮
    g.btn_adv_browse = create_window(
        "BUTTON",
        &i18n(g, "button.browse"),
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_CLIPSIBLINGS | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_ADV_BROWSE,
    );
    set_font(g.btn_adv_browse, g.font);

    // ---- 模型扫描目录：列表 + 添加按钮（右键删除；相对路径为程序自带，不可删）----
    g.lbl_scan_dirs = create_window(
        "STATIC",
        &i18n(g, "label.scanDirs"),
        0,
        WS_CHILD | WS_VISIBLE | 0x0080 | WS_CLIPSIBLINGS,
        0,
        0,
        10,
        10,
        hwnd,
        0,
    );
    set_font(g.lbl_scan_dirs, g.font);
    g.scan_list = create_window(
        "LISTBOX",
        "",
        0,
        WS_CHILD
            | WS_VISIBLE
            | WS_TABSTOP
            | WS_CLIPSIBLINGS
            | WS_VSCROLL
            | WS_BORDER
            | 0x0020_0000
            | LBS_NOTIFY
            | LBS_HASSTRINGS,
        0,
        0,
        10,
        10,
        hwnd,
        0,
    );
    set_font(g.scan_list, g.font);
    subclass_scan_list(g.scan_list);

    fill_scan_list(g);

    // ---- 自动发现的新参数：这里只放入口，完整列表在独立窗口里 ----
    g.auto_fields.clear();
    let auto_total = crate::paramspec::auto_params().len();
    g.lbl_auto = create_window(
        "STATIC",
        &format!("{} {}", i18n(g, "label.autoParams"), auto_total),
        0,
        WS_CHILD | WS_VISIBLE | 0x0080 | WS_CLIPSIBLINGS,
        0,
        0,
        10,
        10,
        hwnd,
        0,
    );
    set_font(g.lbl_auto, g.font);
    g.btn_auto = create_window(
        "BUTTON",
        &i18n(g, "button.autoSettings"),
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_CLIPSIBLINGS | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        hwnd,
        ID_AUTO_OPEN,
    );
    set_font(g.btn_auto, g.font);
    if auto_total == 0 {
        show(g.lbl_auto, false);
        show(g.btn_auto, false);
    }

    // z 序：内容 < 底部底板 < 两个按钮。
    // 否则「设置…」这类后创建的控件会浮在底栏上面，滚动时很难看。
    unsafe {
        SetWindowPos(
            g.adv_footer,
            0,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        for h in [g.btn_adv_save, g.btn_adv_close] {
            SetWindowPos(h, 0, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }
    fit_window_to_screen(hwnd, 1060, 980);
    position_advanced(g);
    populate_advanced(g);
    show(hwnd, true);
}

fn create_adv_field(
    g: &Gui,
    parent: HWND,
    kind: Kind,
    options: &'static [&'static str],
    id: usize,
    label_key: &str,
) -> (HWND, HWND) {
    let class = match kind {
        Kind::Bool => "BUTTON",
        Kind::Select => "COMBOBOX",
        _ => "EDIT",
    };
    // WS_CLIPSIBLINGS：滚动时控件不能画到固定的底部按钮条上面
    let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_CLIPSIBLINGS;
    style |= match kind {
        Kind::Bool => BS_AUTOCHECKBOX,
        Kind::Select => CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_VSCROLL,
        Kind::MultilineText => ES_MULTILINE | ES_AUTOVSCROLL | WS_VSCROLL,
        _ => ES_AUTOHSCROLL,
    };
    let ctl = create_window(class, "", 0, style, 0, 0, 10, 10, parent, id);
    set_font(ctl, g.font);
    if kind == Kind::Select {
        for o in options {
            combo_add(ctl, o);
        }
    }
    // 高级设置的输入框同样把文字居中、留出左右边距
    if kind != Kind::Bool && kind != Kind::Select {
        layout::subclass_edit(ctl);
    }
    let text = i18n(g, label_key);
    let label = create_window(
        "STATIC",
        &text,
        0,
        WS_CHILD | WS_VISIBLE | 0x0080 | WS_CLIPSIBLINGS,
        0,
        0,
        10,
        10,
        parent,
        0,
    );
    set_font(label, g.font);
    (ctl, label)
}

fn position_advanced(g: &mut Gui) {
    if g.adv_hwnd == 0 || unsafe { IsWindow(g.adv_hwnd) } == 0 {
        return;
    }
    let k = layout::scale(g.adv_hwnd);
    let sc = |v: i32| layout::s(v, k);
    let x0 = 16;
    let margin = sc(16);
    let (cw, ch) = client_size(g.adv_hwnd);
    // 列宽跟着窗口宽度走：窗口拉窄时三列自动收窄，不会被裁掉
    let col_w = (((cw - margin * 2) - sc(16)) / 3).max(sc(190));
    let full_w = (cw - margin * 2).max(sc(240));

    // 底部按钮固定在窗口下沿，任何尺寸下都点得到
    let btn_h = sc(32);
    let btn_y = ch - sc(12) - btn_h;
    let view_bottom = btn_y - sc(10);

    let kinds: Vec<Kind> = g.adv_fields.iter().map(|c| c.kind).collect();
    let has_auto = crate::paramspec::auto_params().len() > 0;

    // 内容底部（不含底部按钮）
    let content_bottom = |row_h: i32, ml_h: i32, list_h: i32| -> i32 {
        let mut cy = 10;
        let mut col = 0;
        for kind in kinds.iter() {
            if *kind == Kind::MultilineText {
                if col != 0 {
                    col = 0;
                    cy += row_h;
                }
                cy += 18 + ml_h + 6;
                continue;
            }
            col += 1;
            if col >= 3 {
                col = 0;
                cy += row_h;
            }
        }
        if col != 0 {
            cy += row_h;
        }
        // 扫描目录块：标签 + 列表
        cy += 6 + 18 + 22 + list_h + 8;
        if has_auto {
            cy += 6 + 34;
        }
        cy
    };

    // ---- 固定用宽松尺寸，靠竖向滚动条容纳超出的内容 ----
    // 之前会在放不下时压缩行距和多行框高度，结果文字与输入框挤在一起、
    // 也显示不全。现在不再压缩：需要多高就按多高排，滚动查看。
    let row_h = 42;
    let ml_h = 56;
    let list_h = 60;
    let need = sc(content_bottom(row_h, ml_h, list_h));
    let max_scroll = (((need - view_bottom) as f32) / k.max(0.1)).round() as i32;
    let max_scroll = max_scroll.max(0);
    if g.adv_scroll > max_scroll {
        g.adv_scroll = max_scroll;
    }
    if g.adv_scroll < 0 {
        g.adv_scroll = 0;
    }
    let off = sc(g.adv_scroll);
    unsafe {
        SetScrollRange(g.adv_hwnd, SB_VERT, 0, max_scroll, 1);
        SetScrollPos(g.adv_hwnd, SB_VERT, g.adv_scroll, 1);
    }
    // 纵向位置：逻辑坐标 → 物理坐标再减去滚动偏移
    let vy = |y: i32| sc(y) - off;

    let mut cx = x0;
    let mut cy = 10;
    let mut col = 0;
    for ctl in g.adv_fields.iter() {
        if ctl.kind == Kind::MultilineText {
            if col != 0 {
                col = 0;
                cx = x0;
                cy += row_h;
            }
            move_to(ctl.label, sc(cx), vy(cy), sc(col_w), sc(17));
            move_to(ctl.hwnd, sc(x0), vy(cy + 18), full_w, sc(ml_h));
            cy += 18 + ml_h + 6;
            continue;
        }
        // 勾选项：方框在前、文字在后，排在同一个格子里
        if ctl.kind == Kind::Bool {
            move_to(ctl.hwnd, sc(cx), vy(cy + 1), sc(22), sc(22));
            move_to(ctl.label, sc(cx + 24), vy(cy + 4), sc(col_w - 24), sc(17));
        } else {
            move_to(ctl.label, sc(cx), vy(cy), sc(col_w), sc(17));
            let h = 26;
            if ctl.key == "app.llamaExe" {
                move_to(ctl.hwnd, sc(cx), vy(cy + 18), sc(col_w - 24 - 84), sc(h));
                move_to(
                    g.btn_adv_browse,
                    sc(cx + col_w - 24 - 78),
                    vy(cy + 18),
                    sc(74),
                    sc(h),
                );
            } else {
                move_to(ctl.hwnd, sc(cx), vy(cy + 18), sc(col_w - 24), sc(h));
            }
        }
        col += 1;
        if col >= 3 {
            col = 0;
            cx = x0;
            cy += row_h;
        } else {
            cx += col_w + 8;
        }
    }
    if col != 0 {
        cy += row_h;
    }

    // ---- 模型扫描目录 ----
    cy += 6;
    move_to(g.lbl_scan_dirs, sc(x0), vy(cy), full_w, sc(17));
    cy += 20;
    move_to(g.scan_list, sc(x0), vy(cy), full_w, sc(list_h));
    cy += list_h + 6;

    // ---- 自动发现的新参数 ----
    if has_auto {
        cy += 4;
        let total = crate::paramspec::auto_params().len();
        set_text(
            g.lbl_auto,
            &format!("{} {}: {}", i18n(g, "label.autoParams"), total, i18n(g, "label.autoParamsMore")),
        );
        move_to(g.lbl_auto, sc(x0), vy(cy + 4), sc(col_w * 2 + 30), sc(17));
        move_to(g.btn_auto, sc(x0 + col_w * 2 + 40), vy(cy), sc(112), sc(26));
    }

    // 底部按钮条底板：盖住滚动到按钮下方的内容
    let band_top = btn_y - sc(6);
    move_to(g.adv_footer, 0, band_top, cw, (ch - band_top).max(0));
    // 底部按钮：固定在客户区下沿
    move_to(g.btn_adv_save, cw - margin - sc(112), btn_y, sc(112), btn_h);
    move_to(
        g.btn_adv_close,
        cw - margin - sc(112) * 2 - sc(8),
        btn_y,
        sc(112),
        btn_h,
    );

    // 底板与按钮移动后必须重绘：它们互相压盖，不主动重画会留下上一帧的旧像素
    unsafe {
        for h in [g.adv_footer, g.btn_adv_save, g.btn_adv_close] {
            InvalidateRect(h, std::ptr::null(), 1);
        }
    }
}

fn populate_advanced(g: &mut Gui) {
    let c = cfg(g);
    for ctl in g.adv_fields.iter() {
        let Some(v) = c.path(ctl.key) else { continue };
        match ctl.kind {
            Kind::Bool => check_set(ctl.hwnd, v.as_bool().unwrap_or(false)),
            Kind::Select => {
                let cur = v.as_str().unwrap_or("");
                let mut idx = 0;
                for (i, opt) in ctl.options.iter().enumerate() {
                    if *opt == cur {
                        idx = i;
                    }
                }
                combo_select(ctl.hwnd, idx);
            }
            Kind::MultilineText => {
                if ctl.key == "app.scanDirs" {
                    let dirs: Vec<String> = v
                        .as_arr()
                        .map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()).collect())
                        .unwrap_or_default();
                    set_text(ctl.hwnd, &dirs.join("\r\n"));
                } else {
                    set_text(ctl.hwnd, v.as_str().unwrap_or(""));
                }
            }
            Kind::Int => set_text(ctl.hwnd, &format!("{}", v.as_i64().unwrap_or(0))),
            Kind::Float => set_text(ctl.hwnd, &format!("{}", v.as_f64().unwrap_or(0.0))),
            _ => set_text(ctl.hwnd, v.as_str().unwrap_or("")),
        }
    }
}

pub fn retranslate_advanced(g: &mut Gui) {
    for ctl in g.adv_fields.iter() {
        if let Some(f) = FIELDS.iter().find(|f| f.key == ctl.key) {
            set_text(ctl.label, &i18n(g, f.label));
        } else if ctl.key == "app.llamaExe" {
            set_text(ctl.label, &i18n(g, "label.llamaExe"));
        } else if ctl.key == "app.scanDirs" {
            set_text(ctl.label, &i18n(g, "label.scanDirs"));
        }
    }
    set_text(g.btn_adv_save, &i18n(g, "button.save"));
    set_text(g.btn_adv_close, &i18n(g, "button.back"));    if g.adv_hwnd != 0 {
        set_text(
            g.adv_hwnd,
            &format!("{} — {}", i18n(g, "section.advanced"), i18n(g, "app.title")),
        );
    }
}

/// 供 mod.rs 复用
pub fn tick_once() {
    with_gui(|g| on_tick(g));
}
