//! 主窗口控件创建、布局与文案刷新（淡蓝色扁平风格）
//! Control creation, layout and text refresh for the main window (light blue flat theme).

use super::theme;
use super::win::*;
use super::{cfg, i18n, with_gui, Gui, Ctl};
use crate::settings::{Kind, FIELDS};
use crate::web::api::Shared;

// 控件 ID
pub const ID_LIST: usize = 100;
pub const ID_IMPORT: usize = 101;
pub const ID_SCAN: usize = 102;
pub const ID_START: usize = 103;
pub const ID_STOP: usize = 104;
pub const ID_CHAT: usize = 105;
pub const ID_ADV: usize = 106;
pub const ID_MONITOR: usize = 107;
pub const ID_SAVE: usize = 108;
pub const ID_NEWKEY: usize = 109;
pub const ID_LANG: usize = 110;
pub const ID_COPY_EP: usize = 111;
pub const ID_COPY_KEY: usize = 112;
/// 复制本机地址（127.0.0.1）
pub const ID_COPY_LOCAL: usize = 113;
pub const ID_LOG_CLEAR: usize = 400;
pub const ID_LLAMA_DL: usize = 401;
#[allow(dead_code)]
pub const ID_KVMEM_DL: usize = 402;
#[allow(dead_code)]
pub const ID_HELP: usize = 403;
pub const ID_ADV_SAVE: usize = 300;
pub const ID_ADV_CLOSE: usize = 301;
pub const ID_ADV_BROWSE: usize = 302;
pub const ID_PARAM_HELP: usize = 303;
pub const ID_ESTIMATE: usize = 314;
pub const ID_FIT_LIST: usize = 315;
pub const ID_FIT_APPLY: usize = 316;
pub const ID_FIT_CLOSE: usize = 317;
/// 估算窗口里的「实测校准」行（绿色显示）
pub const ID_FIT_MEASURED: usize = 318;
pub const ID_HELP_TRANSLATE: usize = 309;
pub const ID_AUTO_FILTER: usize = 310;
pub const ID_SCAN_ADD: usize = 311;
pub const ID_CTX_SCAN_DIR: usize = 312;
pub const ID_CTX_OPEN_DIR: usize = 313;
pub const ID_AUTO_BASE: usize = 3000;
pub const ID_AUTO_OPEN: usize = 305;
pub const ID_AUTO_SAVE: usize = 306;
pub const ID_AUTO_PREV: usize = 307;
pub const ID_AUTO_NEXT: usize = 308;
pub const ID_HELP_CLOSE: usize = 304;
pub const ID_FIELD_BASE: usize = 1000;
pub const ID_ADV_BASE: usize = 2000;

// 右键菜单命令
pub const ID_CTX_REMOVE: usize = 500;

// 顶栏标签专用 ID：用于在 WM_CTLCOLORSTATIC 里区分背景色
pub const ID_PILL: usize = 900;
pub const ID_ST_MODEL: usize = 901;
pub const ID_ST_HOST: usize = 902;
pub const ID_ST_UPTIME: usize = 903;
pub const ID_ST_ENDPOINT: usize = 904;
pub const ID_ST_CPU: usize = 905;
pub const ID_ST_RAM: usize = 906;
pub const ID_ST_GPU: usize = 907;

/// 顶栏高度（像素，96 DPI 基准），WM_ERASEBKGND 用它铺底色
pub const HEADER_H: i32 = 84;
/// 主窗口客户区基准尺寸
pub const CLIENT_W: i32 = 1180;
pub const CLIENT_H: i32 = 888;

/// 统一控件高度（输入框、按钮、下拉框一致，避免高矮不齐）
pub const CTL_H: i32 = 30;
/// 单行字段的标签+控件行高
pub const ROW_H: i32 = 62;
/// 右侧三列几何
pub const X0: i32 = 336;
pub const COL_W: i32 = 268;
pub const GAP: i32 = 12;

/// 界面缩放系数（96 DPI 为基准）
pub fn scale(hwnd: HWND) -> f32 {
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let dpi = if dpi == 0 { 96 } else { dpi };
    dpi as f32 / 96.0
}

pub fn s(v: i32, k: f32) -> i32 {
    (v as f32 * k).round() as i32
}

fn child_class(kind: Kind) -> &'static str {
    match kind {
        Kind::Bool => "BUTTON",
        Kind::Select => "COMBOBOX",
        _ => "EDIT",
    }
}

fn child_style(kind: Kind) -> u32 {
    let mut st = WS_CHILD | WS_VISIBLE | WS_TABSTOP;
    match kind {
        Kind::Bool => st |= BS_AUTOCHECKBOX,
        Kind::Select => st |= CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_VSCROLL,
        Kind::MultilineText => {
            st |= ES_MULTILINE | ES_AUTOVSCROLL;
        }
        _ => st |= ES_AUTOHSCROLL,
    }
    st
}

/// 普通标签（透明背景，颜色由 WM_CTLCOLORSTATIC 决定）
fn make_label(g: &Gui, text: &str) -> HWND {
    make_label_id(g, text, 0)
}

fn make_label_id(g: &Gui, text: &str, id: usize) -> HWND {
    // SS_NOPREFIX(0x0080)：让文案里的 "&" 正常显示，不被当成助记符
    let h = create_window(
        "STATIC",
        text,
        0,
        WS_CHILD | WS_VISIBLE | 0x0080,
        0,
        0,
        10,
        10,
        g.hwnd,
        id,
    );
    set_font(h, g.font);
    h
}

/// 单行、不换行的标签（超长时右侧显示省略号，全文由提示框给出）
fn make_label_nowrap(g: &Gui, text: &str, id: usize) -> HWND {
    // SS_NOPREFIX(0x0080) | SS_ENDELLIPSIS(0x4000) | SS_LEFTNOWORDWRAP(0x000C)
    let h = create_window(
        "STATIC",
        text,
        0,
        WS_CHILD | WS_VISIBLE | 0x0080 | 0x4000 | 0x000C,
        0,
        0,
        10,
        10,
        g.hwnd,
        id,
    );
    set_font(h, g.font);
    h
}

/// 自绘按钮（扁平、直角）
fn make_button(g: &Gui, text: &str, id: usize) -> HWND {
    let h = create_window(
        "BUTTON",
        text,
        0,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW,
        0,
        0,
        10,
        10,
        g.hwnd,
        id,
    );
    set_font(h, g.font);
    h
}

/// 给输入框挂上子类化过程（让文字上下居中、左右留边）
/// 扫描目录列表框：挂上右键菜单处理
pub fn subclass_scan_list(h: HWND) {
    if h == 0 {
        return;
    }
    unsafe {
        let old = SetWindowLongPtrW(h, GWLP_WNDPROC, super::scan_list_wnd_proc as isize);
        super::SCAN_LIST_OLD.store(old, std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn subclass_edit(h: HWND) {
    if h == 0 {
        return;
    }
    unsafe {
        let old = SetWindowLongPtrW(h, GWLP_WNDPROC, super::edit_wnd_proc as isize);
        SetWindowLongPtrW(h, GWLP_USERDATA, old);
        // 左右内边距交给 EDIT 自己处理，光标与选区也一起内缩
        SendMessageW(
            h,
            EM_SETMARGINS,
            EC_LEFTMARGIN | EC_RIGHTMARGIN,
            ((layout_margin()) | (layout_margin() << 16)) as LPARAM,
        );
    }
    // 客户区变化后需要重新计算一次
    unsafe {
        SetWindowPos(h, 0, 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0020); // NOSIZE|NOMOVE|FRAMECHANGED
    }
}

fn layout_margin() -> usize {
    // 文本左侧留白（EM_SETMARGINS），单位是像素
    8
}

fn make_field(g: &Gui, kind: Kind, options: &'static [&'static str], id: usize) -> HWND {
    // 输入框保持系统默认外观：客户区就是整个控件，不再由父窗口补白。
    // 之前自定义 WM_NCCALCSIZE 内缩客户区来“让文字居中”，结果父窗口只能
    // 往控件内边距里刷色，四周出现一圈宽窄不等、没有边界的白斑。
    let h = create_window(
        child_class(kind),
        "",
        0,
        child_style(kind),
        0,
        0,
        10,
        10,
        g.hwnd,
        id,
    );
    set_font(h, g.font);
    if kind == Kind::Select {
        for opt in options {
            combo_add(h, opt);
        }
    }
    if kind != Kind::Bool && kind != Kind::Select {
        subclass_edit(h);
    }
    h
}

/// 构建主窗口全部控件
pub fn build(shared: Shared, hwnd: HWND) {
    let lang = {
        let app = shared.lock().ok();
        match app {
            Some(a) => crate::i18n::resolve(&a.store.cfg),
            None => crate::util::detect_system_lang(),
        }
    };
    let title = crate::i18n::t(&lang, "app.title");
    set_text(hwnd, &title);

    let gui = Gui {
        shared,
        hwnd,
        font: 0,
        font_bold: 0,
        font_mono: 0,
        title,
        subtitle: String::new(),
        list: 0,
        fields: Vec::new(),
        args_view: 0,
        btn_import: 0,
        btn_scan: 0,
        btn_start: 0,
        btn_stop: 0,
        btn_chat: 0,
        btn_adv: 0,
        btn_monitor: 0,
        btn_save: 0,
        btn_newkey: 0,
        btn_copy_ep: 0,
        btn_copy_key: 0,
        btn_copy_local: 0,
        lang_combo: 0,
        lbl_models: 0,
        lbl_common: 0,
        lbl_network: 0,
        lbl_args: 0,
        lbl_hint: 0,
        lbl_log: 0,
        st_pill: 0,
        st_host: 0,
        st_uptime: 0,
        st_endpoint: 0,
        st_cpu: 0,
        st_ram: 0,
        st_gpu: 0,
        adv_hwnd: 0,
        adv_fields: Vec::new(),
        btn_adv_save: 0,
        btn_adv_close: 0,
        btn_adv_browse: 0,
        btn_param_help: 0,
        btn_estimate: 0,
        mon_hwnd: 0,
        adv_footer: 0,
        adv_scroll: 0,
        fit_hwnd: 0,
        vram_warned: false,
        help_hwnd: 0,
        help_view: 0,
        btn_help_translate: 0,
        btn_help_close: 0,
        auto_fields: Vec::new(),
        lbl_auto: 0,
        scan_list: 0,
        lbl_warn: 0,
        lbl_scan_dirs: 0,
        btn_scan_dir_add: 0,
        btn_auto: 0,
        auto_hwnd: 0,
        auto_page: 0,
        auto_filter: String::new(),
        log_edit: 0,
        btn_log_clear: 0,
        btn_llama_dl: 0,
        btn_kvmem_dl: 0,
        btn_help: 0,
        tip: 0,
        hover_wnd: 0,
        model_rect: RECT::default(),
        model_text: String::new(),
        marquee_off: 0,
        model_scroll: false,
        model_ids: Vec::new(),
        selected_id: String::new(),
        lib_sig: String::new(),
        suppress_sel: false,
        log_seen: 0,
        lang: lang.clone(),
        brush_bg: 0,
        brush_bar: 0,
        brush_field: 0,
        brush_log: 0,
        model_full: String::new(),
    };
    super::install(gui);

    with_gui(|g| {
        g.brush_bg = unsafe { CreateSolidBrush(theme::BG) };
        g.brush_bar = unsafe { CreateSolidBrush(theme::BAR) };
        g.brush_field = unsafe { CreateSolidBrush(theme::FIELD) };
        g.brush_log = unsafe { CreateSolidBrush(theme::LOG_BG) };
        super::BRUSH.store(g.brush_bg, std::sync::atomic::Ordering::Relaxed);
        super::BRUSH_BAR.store(g.brush_bar, std::sync::atomic::Ordering::Relaxed);
        super::BRUSH_FIELD.store(g.brush_field, std::sync::atomic::Ordering::Relaxed);
        super::BRUSH_LOG.store(g.brush_log, std::sync::atomic::Ordering::Relaxed);

        g.font = create_font(hwnd, 9, false);
        g.font_bold = create_font(hwnd, 10, true);
        g.font_mono = create_font_face(hwnd, 9, false, "Consolas");
        super::FONT_UI.store(g.font as isize, std::sync::atomic::Ordering::Relaxed);
        super::MAIN_HWND.store(hwnd, std::sync::atomic::Ordering::Relaxed);

        // 顶部状态区（“当前模型”一行由父窗口自绘，见 WM_PAINT，可横向滚动）
        g.st_pill = make_label_id(g, "", ID_PILL);
        g.st_host = make_label_id(g, "", ID_ST_HOST);
        g.st_uptime = make_label_id(g, "", ID_ST_UPTIME);
        g.st_endpoint = make_label_id(g, "", ID_ST_ENDPOINT);
        g.st_cpu = make_label_id(g, "", ID_ST_CPU);
        g.st_ram = make_label_id(g, "", ID_ST_RAM);
        g.st_gpu = make_label_id(g, "", ID_ST_GPU);
        g.btn_copy_ep = make_button(g, "", ID_COPY_EP);
        g.lang_combo = create_window(
            "COMBOBOX",
            "",
            0,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
            0,
            0,
            10,
            10,
            hwnd,
            ID_LANG,
        );
        set_font(g.lang_combo, g.font);
        g.btn_chat = make_button(g, "", ID_CHAT);
        g.btn_adv = make_button(g, "", ID_ADV);
	    g.btn_monitor = make_button(g, "", ID_MONITOR);

        // 左侧模型库（自绘列表：识图模型前面带小图标）
        g.lbl_models = make_label(g, "");
        g.list = create_window(
            "LISTBOX",
            "",
            0,
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_VSCROLL
                | LBS_NOTIFY
                | LBS_NOINTEGRALHEIGHT
                | LBS_OWNERDRAWFIXED
                | LBS_HASSTRINGS,
            0,
            0,
            10,
            10,
            hwnd,
            ID_LIST,
        );
        set_font(g.list, g.font);
        unsafe {
            SendMessageW(g.list, LB_SETITEMHEIGHT, 0, s(26, scale(hwnd)) as LPARAM);
            // 子类化：列表里的鼠标移动用来显示“不支持加载该模型”
            let old = SetWindowLongPtrW(g.list, GWLP_WNDPROC, super::list_wnd_proc as isize);
            super::LIST_OLD_PROC.store(old, std::sync::atomic::Ordering::Relaxed);
        }
        g.btn_import = make_button(g, "", ID_IMPORT);
        g.btn_scan = make_button(g, "", ID_SCAN);
        g.btn_start = make_button(g, "", ID_START);
        g.btn_stop = make_button(g, "", ID_STOP);
        g.lbl_args = make_label(g, "");
        g.btn_param_help = make_button(g, "", ID_PARAM_HELP);
        g.btn_estimate = make_button(g, "", ID_ESTIMATE);
        g.args_view = create_window(
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
        set_font(g.args_view, g.font_mono);
        subclass_edit(g.args_view);

        // 右侧设置区
        g.lbl_common = make_label(g, "");
        g.lbl_network = make_label(g, "");
        g.lbl_hint = make_label(g, "");
        g.btn_save = make_button(g, "", ID_SAVE);
        g.btn_newkey = make_button(g, "", ID_NEWKEY);
        g.btn_copy_key = make_button(g, "", ID_COPY_KEY);
        g.btn_copy_local = make_button(g, "", ID_COPY_LOCAL);

        // 启动时探测本机实际装了哪些推理引擎；没装的就不出现在下拉框里。
        // 结果借成 'static 切片，Ctl.options 的类型不用改。
        let engine_opts: &'static [&'static str] = {
            let list = match g.shared.lock() {
                Ok(app) => crate::process::Launcher::available_engines(&app.store.cfg),
                Err(_) => Vec::new(),
            };
            if list.is_empty() {
                crate::settings::SELECT_ENGINE
            } else {
                Box::leak(list.into_boxed_slice())
            }
        };

        for (idx, f) in FIELDS.iter().enumerate() {
            if f.section != "common" && f.section != "network" {
                continue;
            }
            let opts: &'static [&'static str] = if f.key == "launch.engine" {
                engine_opts
            } else {
                f.options
            };
            let ctl = Ctl {
                key: f.key,
                kind: f.kind,
                hwnd: make_field(g, f.kind, opts, ID_FIELD_BASE + idx),
                label: make_label(g, &i18n(g, f.label)),
                options: opts,
            };
            g.fields.push(ctl);
        }

        // 右下角运行日志
        g.lbl_log = make_label(g, "");
        g.lbl_warn = make_label(g, "");
        g.btn_log_clear = make_button(g, "", ID_LOG_CLEAR);
        g.btn_llama_dl = make_button(g, "", ID_LLAMA_DL);
        g.btn_kvmem_dl = make_button(g, "", ID_KVMEM_DL);
        g.btn_help = make_button(g, "", ID_HELP);
        g.log_edit = create_window(
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
        set_font(g.log_edit, g.font_mono);
        subclass_edit(g.log_edit);
        super::HWND_ARGS.store(g.args_view, std::sync::atomic::Ordering::Relaxed);
        super::HWND_LOG.store(g.log_edit, std::sync::atomic::Ordering::Relaxed);

        populate(g);
        apply_language(g);
        rebuild_tooltips(g);
        position_all(g);
    });
}

/// 用当前配置填充控件
pub fn populate(g: &mut Gui) {
    let c = cfg(g);
    for ctl in g.fields.iter() {
        // 配置里还没有这个键时按“空值”处理，让下拉/勾选框也能显示默认项
        let empty = crate::json::Json::str("");
        let v = c.path(ctl.key).unwrap_or(&empty);
        match ctl.kind {
            Kind::Bool => check_set(ctl.hwnd, v.as_bool().unwrap_or(false)),
            Kind::Select => {
                let cur = v.as_str().unwrap_or("");
                let mut idx = 0;
                let mut found = false;
                for (i, opt) in ctl.options.iter().enumerate() {
                    if *opt == cur {
                        idx = i;
                        found = true;
                    }
                }
                // 未设置过（如新增的 launch.engine）时落到第一项并写回配置；
                // 配置里的引擎本机没装（已从下拉框移除）时同样退到第一项，
                // 否则界面显示一个引擎、实际却按另一个引擎启动
                if cur.is_empty() || !found {
                    if let Some(first) = ctl.options.first() {
                        let f = first.to_string();
                        if let Ok(mut app) = g.shared.lock() {
                            app.store.cfg.set_path(ctl.key, crate::json::Json::str(&f));
                            // 立即落盘：否则下次启动还是读到那个本机没装的引擎
                            let _ = app.store.save();
                        }
                    }
                }
                // 选项文本本地化（值仍是 options 里的原字符串，保存不受影响）
                combo_reset(ctl.hwnd);
                for opt in ctl.options.iter() {
                    let label = i18n(g, &format!("opt.{}", opt));
                    let text = if label == format!("opt.{}", opt) {
                        opt.to_string()
                    } else {
                        label
                    };
                    combo_add(ctl.hwnd, &text);
                }
                combo_select(ctl.hwnd, idx);
            }
            Kind::Int => set_text(ctl.hwnd, &format!("{}", v.as_i64().unwrap_or(0))),
            Kind::Float => {
                let n = v.as_f64().unwrap_or(0.0);
                set_text(ctl.hwnd, &format!("{}", n))
            }
            Kind::MultilineText => {
                if ctl.key == "network.apiKeys" {
                    let keys = crate::settings::api_keys(&c);
                    set_text(ctl.hwnd, &keys.join("\r\n"));
                } else {
                    set_text(ctl.hwnd, v.as_str().unwrap_or(""));
                }
            }
            _ => set_text(ctl.hwnd, v.as_str().unwrap_or("")),
        }
    }
    if let Ok(app) = g.shared.lock() {
        g.selected_id = app.store.selected_model();
    }
    refresh_lang_combo(g);
    refresh_args_preview(g);
}

pub fn refresh_lang_combo(g: &mut Gui) {
    let cur = cfg(g).s("app.lang", "auto");
    combo_reset(g.lang_combo);
    combo_add(g.lang_combo, &format!("{}: auto", i18n(g, "label.language")));
    for (code, name) in crate::i18n::LANGS {
        combo_add(g.lang_combo, name);
        let _ = code;
    }
    let idx = match cur.as_str() {
        "zh-CN" => 1,
        "en-US" => 2,
        _ => 0,
    };
    combo_select(g.lang_combo, idx);
}

pub fn refresh_args_preview(g: &mut Gui) {
    let c = cfg(g);
    let path = g
        .shared
        .lock()
        .ok()
        .and_then(|a| a.lib.find(&g.selected_id).map(|m| m.path.clone()))
        .unwrap_or_default();
    set_text(g.args_view, &crate::settings::preview_args(&c, &path));
}

/// 刷新所有文案（切换语言或启动时调用）
pub fn apply_language(g: &mut Gui) {
    g.lang = {
        let c = cfg(g);
        crate::i18n::resolve(&c)
    };
    let title = i18n(g, "app.title");
    g.title = title.clone();
    set_text(g.hwnd, &format!("{} — {}", title, i18n(g, "app.subtitle")));
    set_text(g.lbl_models, &i18n(g, "section.models"));
    set_text(g.lbl_common, &i18n(g, "section.common"));
    set_text(g.lbl_network, &i18n(g, "section.network"));
    set_text(g.lbl_args, &i18n(g, "label.argsPreview"));
    set_text(g.btn_param_help, &i18n(g, "button.paramHelp"));
    set_text(g.btn_estimate, &i18n(g, "button.estimate"));
    set_text(g.lbl_log, &i18n(g, "label.runLog"));
    set_text(g.btn_import, &i18n(g, "button.importFolder"));
    set_text(g.btn_scan, &i18n(g, "button.scan"));
    set_text(g.btn_start, &i18n(g, "button.start"));
    set_text(g.btn_stop, &i18n(g, "button.stop"));
    set_text(g.btn_chat, &i18n(g, "button.chat"));
    set_text(g.btn_monitor, &i18n(g, "button.monitor"));
    set_text(g.btn_adv, &i18n(g, "button.advanced"));
    set_text(g.btn_save, &i18n(g, "button.save"));
    set_text(g.btn_newkey, &i18n(g, "button.newKey"));
    set_text(g.btn_copy_ep, &i18n(g, "button.copy"));
    set_text(g.btn_copy_local, &i18n(g, "button.copyLocal"));
    set_text(g.btn_copy_key, &i18n(g, "button.copy"));
    set_text(g.btn_log_clear, &i18n(g, "button.clearLog"));
    set_text(g.btn_llama_dl, &i18n(g, "button.llamaDownload"));
    set_text(g.btn_kvmem_dl, &i18n(g, "button.kvmemDownload"));
    set_text(g.btn_help, &i18n(g, "button.help"));
    set_text(g.st_cpu, &i18n(g, "status.cpuNone"));
    set_text(g.st_ram, &i18n(g, "status.ramNone"));
    set_text(g.st_gpu, &i18n(g, "status.gpuNone"));

    for ctl in g.fields.iter() {
        if let Some(f) = FIELDS.iter().find(|f| f.key == ctl.key) {
            let text = i18n(g, f.label);
            if ctl.kind == Kind::Bool {
                // 勾选框自带文字，文字就在方框右（即“条目左侧是勾选框”）
                set_text(ctl.hwnd, &text);
                set_text(ctl.label, "");
            } else {
                set_text(ctl.label, &text);
            }
        }
    }
    // 语言下拉的 “auto” 项也要跟随当前语言
    refresh_lang_combo(g);
    if g.adv_hwnd != 0 {
        super::actions::retranslate_advanced(g);
    }
    rebuild_tooltips(g);
}

/// 重建悬浮提示（语言切换后文案需要跟着变）
pub fn rebuild_tooltips(g: &mut Gui) {
    if g.tip != 0 {
        unsafe {
            DestroyWindow(g.tip);
        }
        g.tip = 0;
    }
    g.tip = create_tooltip(g.hwnd);
    if g.tip == 0 {
        return;
    }
    for ctl in g.fields.iter() {
        if let Some(f) = FIELDS.iter().find(|f| f.key == ctl.key) {
            tooltip_add(g.tip, ctl.label, &i18n(g, f.hint));
            tooltip_add(g.tip, ctl.hwnd, &i18n(g, f.hint));
        }
    }
    tooltip_add(g.tip, g.st_endpoint, &i18n(g, "hint.endpoint"));
    tooltip_add(g.tip, g.btn_copy_ep, &i18n(g, "hint.copyEndpoint"));
    tooltip_add(g.tip, g.btn_copy_key, &i18n(g, "hint.copyKey"));
    tooltip_add(g.tip, g.btn_newkey, &i18n(g, "hint.newKey"));
    tooltip_add(g.tip, g.lbl_log, &i18n(g, "hint.logPane"));
    tooltip_add(g.tip, g.btn_import, &i18n(g, "hint.importFolder"));
    tooltip_add(g.tip, g.btn_scan, &i18n(g, "hint.scan"));
    tooltip_add(g.tip, g.btn_start, &i18n(g, "hint.start"));
    tooltip_add(g.tip, g.btn_stop, &i18n(g, "hint.stop"));
    tooltip_add(g.tip, g.btn_chat, &i18n(g, "hint.chat"));
    tooltip_add(g.tip, g.btn_adv, &i18n(g, "hint.advanced"));
    tooltip_add(g.tip, g.btn_save, &i18n(g, "hint.save"));
    // 列表的提示改成“只在红色条目上弹一句不支持”，避免两个提示同时出现
}

/// 统一摆放控件位置
pub fn position_all(g: &mut Gui) {
    let k = scale(g.hwnd);
    let w = |v: i32| s(v, k);
    let h = |v: i32| s(v, k);

    // ---- 顶栏第一行：状态 + 主要操作 ----
    move_to(g.st_pill, w(16), h(14), w(96), h(26));
    // “当前模型”这一行由父窗口自己画，这里只记录区域
    g.model_rect = RECT {
        left: w(114),
        top: h(14),
        right: w(114) + w(360),
        bottom: h(14) + h(26),
    };
    move_to(g.st_host, w(482), h(14), w(150), h(26));
    move_to(g.st_uptime, w(640), h(14), w(110), h(26));
    move_to(g.lang_combo, w(758), h(14), w(110), h(200));
    move_to(g.btn_monitor, w(876), h(12), w(92), h(30));
    move_to(g.btn_chat, w(976), h(12), w(96), h(30));
    move_to(g.btn_adv, w(1080), h(12), w(84), h(30));

    // ---- 顶栏第二行：复制按钮在地址左侧 ----
    move_to(g.btn_copy_ep, w(16), h(44), w(56), h(CTL_H));
    move_to(g.st_endpoint, w(80), h(50), w(360), h(24));
    move_to(g.st_cpu, w(452), h(50), w(100), h(24));
    move_to(g.st_ram, w(560), h(50), w(170), h(24));
    move_to(g.st_gpu, w(738), h(50), w(426), h(24));

    // ---- 左侧模型库 ----
    move_to(g.lbl_models, w(16), h(96), w(200), h(20));
    move_to(g.list, w(16), h(120), w(300), h(350));
    move_to(g.btn_import, w(16), h(482), w(146), h(CTL_H));
    move_to(g.btn_scan, w(170), h(482), w(146), h(CTL_H));
    move_to(g.btn_start, w(16), h(524), w(300), h(CTL_H));
    move_to(g.btn_stop, w(16), h(566), w(300), h(CTL_H));
    move_to(g.lbl_args, w(16), h(608), w(104), h(18));
    move_to(g.btn_param_help, w(122), h(597), w(92), h(CTL_H));
    move_to(g.btn_estimate, w(220), h(597), w(96), h(CTL_H));
    move_to(g.args_view, w(16), h(630), w(300), h(242));

    // ---- 右侧：三列网格 ----
    move_to(g.lbl_common, w(X0), h(96), w(240), h(20));

    let mut plain: Vec<&Ctl> = Vec::new();
    let mut checks: Vec<&Ctl> = Vec::new();
    let mut net_plain: Vec<&Ctl> = Vec::new();
    let mut key_ctl: Option<&Ctl> = None;
    for ctl in g.fields.iter() {
        let section = FIELDS
            .iter()
            .find(|f| f.key == ctl.key)
            .map(|f| f.section)
            .unwrap_or("common");
        if ctl.kind == Kind::MultilineText {
            key_ctl = Some(ctl);
        } else if ctl.kind == Kind::Bool {
            checks.push(ctl);
        } else if section == "network" {
            net_plain.push(ctl);
        } else {
            plain.push(ctl);
        }
    }

    // 常用设置：两行三列
    let mut y = 124;
    for (i, ctl) in plain.iter().enumerate() {
        let (row, col) = (i / 3, i % 3);
        let cx = X0 + (col as i32) * (COL_W + GAP);
        let cy = y + (row as i32) * ROW_H;
        place_field(ctl, cx, cy, COL_W, k);
    }
    let rows = (plain.len() + 2) / 3;
    y += (rows as i32) * ROW_H;

    // 勾选项：横向一排，方框在文字左侧（单独占一行，不与上方控件重叠）
    // 数量超过 3 个时按可用宽度均分，避免越界
    let check_row_y = y + 4;
    let n = checks.len().max(1) as i32;
    let avail = 1164 - X0;
    let (slot, step) = if n <= 3 {
        (COL_W, COL_W + GAP)
    } else {
        let s = (avail - (n - 1) * GAP) / n;
        (s, s + GAP)
    };
    for (i, ctl) in checks.iter().enumerate() {
        let cx = X0 + (i as i32) * step;
        move_to(ctl.label, w(cx), h(check_row_y), 0, 0);
        move_to(ctl.hwnd, w(cx), h(check_row_y), w(slot - 8), h(CTL_H));
    }
    y = check_row_y + 34;

    // 网络与 API
    move_to(g.lbl_network, w(X0), h(y), w(240), h(20));
    y += 28;

    // 自定义地址 / 端口：与上方行同间距
    for (i, ctl) in net_plain.iter().enumerate() {
        let cx = X0 + (i as i32) * (COL_W + GAP);
        place_field(ctl, cx, y, COL_W, k);
    }
    // 「复制本机地址」放在第三列，与服务端口输入框同一行
    move_to(
        g.btn_copy_local,
        w(X0 + 2 * (COL_W + GAP)),
        h(y + 22),
        w(120),
        h(CTL_H),
    );
    y += ROW_H;

    // 密钥：紧跟自定义地址下方，复制 / 生成新密钥按钮等高对齐
    if let Some(ctl) = key_ctl {
        move_to(ctl.label, w(X0), h(y - 4), w(320), h(18));
        move_to(ctl.hwnd, w(X0), h(y + 18), w(420), h(CTL_H));
        move_to(g.btn_copy_key, w(X0 + 428), h(y + 18), w(86), h(CTL_H));
        move_to(g.btn_newkey, w(X0 + 522), h(y + 18), w(140), h(CTL_H));
        move_to(g.btn_save, w(1064), h(y + 18), w(100), h(CTL_H));
        y += 18 + CTL_H + 8;
    }

    move_to(g.lbl_hint, w(X0), h(y), w(700), h(20));
    y += 26;

    // ---- 右下角运行日志（标签在框外，避免标签底色盖住边框）----
    move_to(g.lbl_log, w(X0), h(y), w(240), h(20));
    move_to(g.btn_log_clear, w(1064), h(y - 2), w(100), h(CTL_H));
    move_to(g.btn_kvmem_dl, w(956), h(y - 2), w(100), h(CTL_H));
    move_to(g.btn_help, w(740), h(y - 2), w(88), h(CTL_H));
    move_to(g.btn_llama_dl, w(848), h(y - 2), w(100), h(CTL_H));
    // 顶部警示条（爆显存风险等），把日志区整体下移并缩短，给提示留出位置
    // 放在按钮行下方（按钮结束于 y+28），否则这条 1160 宽的标签会盖住按钮底部几行
    move_to(g.lbl_warn, w(X0), h(y + 30), w(1164 - X0), h(18));
    let log_top = y + 50;
    let log_h = 872 - log_top;
    move_to(g.log_edit, w(X0), h(log_top), w(828), h(log_h.max(80)));

    // 下拉框的闭合高度默认比输入框矮几像素，这里统一成 CTL_H
    for ctl in g.fields.iter() {
        if ctl.kind == Kind::Select && ctl.hwnd != 0 {
            combo_fit_height(ctl.hwnd, h(CTL_H));
        }
    }
    // 顶栏语言下拉框同样对齐（它不在 fields 里，单独处理）
    if g.lang_combo != 0 {
        combo_fit_height(g.lang_combo, h(CTL_H));
    }

    // 控件都挪好之后整窗重绘一次：否则父窗口上残留的旧边框会停在老位置
    unsafe {
        InvalidateRect(g.hwnd, std::ptr::null(), 1);
    }
}

/// 单个字段：标签在上、控件在下（高度统一）
fn place_field(ctl: &Ctl, cx: i32, cy: i32, col_w: i32, k: f32) {
    let w = |v: i32| s(v, k);
    let h = |v: i32| s(v, k);
    move_to(ctl.label, w(cx), h(cy), w(col_w), h(18));
    move_to(ctl.hwnd, w(cx), h(cy + 22), w(col_w - 14), h(CTL_H));
}
