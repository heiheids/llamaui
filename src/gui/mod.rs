//! 原生窗口程序入口（Win32）：窗口、消息循环与全局状态
//! Native desktop entry: windows, message loop and global state.

pub mod actions;
pub mod layout;
pub mod theme;
pub mod win;

use crate::settings::Kind;
use crate::web::api::Shared;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use win::*;

pub const CLASS_NAME: &str = "LlamaUiMain";
pub const TIMER_TICK: usize = 1;
pub const TIMER_MARQUEE: usize = 2;
pub const WM_PAINT: u32 = 0x000F;

/// 扫描目录列表框的子类化过程：右键弹「删除目录」菜单
pub unsafe extern "system" fn scan_list_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    if msg == WM_CONTEXTMENU {
        with_gui(|g| actions::scan_dir_menu(g, hwnd, lp));
        return 0;
    }
    let old = SCAN_LIST_OLD.load(Ordering::Relaxed);
    if old == 0 {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    CallWindowProcW(old, hwnd, msg, wp, lp)
}

/// 模型列表的子类化过程：只旁听鼠标移动，用来弹“不支持加载该模型”提示
/// 输入框的子类化过程：把客户区向内收，让文字“上下居中 + 左右留边”
/// Win32 单行 EDIT 天然把文字画在客户区顶部，只能靠收窄客户区把它压到中间。
pub unsafe extern "system" fn edit_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    let old = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if old == 0 {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    CallWindowProcW(old, hwnd, msg, wp, lp)
}

pub unsafe extern "system" fn list_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    if msg == WM_MOUSEMOVE {
        // 登记一次“鼠标移出”通知，移出后自动收起提示
        if !LIST_TRACKING.swap(true, Ordering::Relaxed) {
            let mut tme = TRACKMOUSEEVENT {
                cb_size: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                flags: TME_LEAVE,
                hwnd,
                hover_time: 0,
            };
            TrackMouseEvent(&mut tme);
        }
        with_gui(|g| actions::on_list_hover(g, lp));
    } else if msg == WM_MOUSELEAVE {
        LIST_TRACKING.store(false, Ordering::Relaxed);
        with_gui(|g| actions::hide_hover(g));
    }
    let old = LIST_OLD_PROC.load(Ordering::Relaxed);
    if old == 0 {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    CallWindowProcW(old, hwnd, msg, wp, lp)
}

/// 一个设置项对应的控件
/// 自动发现的参数控件（来自 llama-server --help，键名在运行时才知道）
pub struct AutoCtl {
    /// 配置键（去掉前导 -），落在 config.auto.<key>
    pub key: String,
    /// 原始 flag，用于显示与生成参数
    pub flag: String,
    pub kind: crate::settings::Kind,
    pub hwnd: HWND,
    pub label: HWND,
}

pub struct Ctl {
    pub key: &'static str,
    pub kind: Kind,
    pub hwnd: HWND,
    pub label: HWND,
    pub options: &'static [&'static str],
}

/// 主窗口与附属窗口集合
pub struct Gui {
    pub shared: Shared,
    pub hwnd: HWND,
    pub font: HFONT,
    pub font_bold: HFONT,
    pub font_mono: HFONT,
    pub title: String,
    pub subtitle: String,

    pub list: HWND,
    pub fields: Vec<Ctl>,
    pub args_view: HWND,

    pub btn_import: HWND,
    pub btn_scan: HWND,
    pub btn_start: HWND,
    pub btn_stop: HWND,
    pub btn_chat: HWND,
    pub btn_adv: HWND,
    pub btn_save: HWND,
    pub btn_newkey: HWND,
    pub btn_copy_ep: HWND,
    pub btn_copy_key: HWND,
    pub lang_combo: HWND,

    pub lbl_models: HWND,
    pub lbl_common: HWND,
    pub lbl_network: HWND,
    pub lbl_args: HWND,
    pub lbl_hint: HWND,
    pub lbl_log: HWND,
    /// 日志上方的警示条（爆显存风险等）
    pub lbl_warn: HWND,

    pub st_pill: HWND,
    pub st_host: HWND,
    pub st_uptime: HWND,
    pub st_endpoint: HWND,
    pub st_cpu: HWND,
    pub st_ram: HWND,
    pub st_gpu: HWND,

    pub adv_hwnd: HWND,
    pub adv_fields: Vec<Ctl>,
    pub btn_adv_save: HWND,
    pub btn_adv_close: HWND,
    /// 高级设置里 llama-server 路径旁的“浏览…”按钮
    pub btn_adv_browse: HWND,
    /// 「参数解读」按钮与解读窗口
    pub btn_param_help: HWND,
    /// 「显存估算」按钮与窗口
    pub btn_estimate: HWND,
    pub fit_hwnd: HWND,
    /// 已经就显存风险写过日志（避免刷屏）
    pub vram_warned: bool,
    pub help_hwnd: HWND,
    /// 参数解读窗口里的文本控件与「生成中文解读」按钮
    pub help_view: HWND,
    pub btn_help_translate: HWND,
    /// 高级设置里「自动发现的新参数」控件
    pub auto_fields: Vec<AutoCtl>,
    /// 该分区的标题标签
    pub lbl_auto: HWND,
    /// 扫描目录列表（高级设置）
    pub scan_list: HWND,
    pub lbl_scan_dirs: HWND,
    pub btn_scan_dir_add: HWND,
    /// 「设置…」按钮与自动发现参数窗口
    pub btn_auto: HWND,
    pub auto_hwnd: HWND,
    /// 自动发现窗口当前页码（每页 18 个）
    pub auto_page: usize,
    /// 自动发现窗口的筛选关键字
    pub auto_filter: String,

    pub log_edit: HWND,
    pub btn_log_clear: HWND,
    /// 「llama 下载」按钮：打开 llama.cpp releases 页面
    pub btn_llama_dl: HWND,
    /// 「KVMem 下载」与「帮助」按钮
    pub btn_kvmem_dl: HWND,
    pub btn_help: HWND,
    pub tip: HWND,
    /// 悬停提示窗（如“不支持加载该模型”）
    pub hover_wnd: HWND,
    /// 顶栏“当前模型”区域（父窗口自绘，文字过长时在原位横向滚动）
    pub model_rect: RECT,
    pub model_text: String,
    pub marquee_off: i32,
    pub model_scroll: bool,

    pub model_ids: Vec<String>,
    pub selected_id: String,
    pub lib_sig: String,
    /// 列表重建期间屏蔽选择通知
    pub suppress_sel: bool,
    pub log_seen: usize,
    pub lang: String,
    pub model_full: String,
    pub brush_bg: HBRUSH,
    pub brush_bar: HBRUSH,
    pub brush_field: HBRUSH,
    pub brush_log: HBRUSH,
}

pub static GUI: OnceLock<Mutex<Gui>> = OnceLock::new();

/// 背景画刷：控件创建过程中会触发 WM_CTLCOLORSTATIC，此时 GUI 锁已被持有，
/// 因此画刷必须用无锁的原子量保存，否则会自锁死。
pub static BRUSH: AtomicIsize = AtomicIsize::new(0);
pub static BRUSH_BAR: AtomicIsize = AtomicIsize::new(0);
pub static BRUSH_FIELD: AtomicIsize = AtomicIsize::new(0);
pub static BRUSH_LOG: AtomicIsize = AtomicIsize::new(0);
/// 状态圆点颜色（随服务状态变化）
pub static PILL_COLOR: AtomicU32 = AtomicU32::new(0x007A_6B5C);
/// 供 WM_DRAWITEM / WM_CTLCOLOREDIT 无锁读取的字号与控件句柄
pub static FONT_UI: AtomicIsize = AtomicIsize::new(0);
pub static HWND_ARGS: AtomicIsize = AtomicIsize::new(0);
pub static HWND_LOG: AtomicIsize = AtomicIsize::new(0);
/// 主窗口句柄：只有主窗口才绘制顶栏
pub static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);
/// 列表项“识图模型”标记位（第 i 位对应第 i 项），供 WM_DRAWITEM 无锁读取
pub static VISION_BITS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// 列表项“MoE 模型”标记位
pub static MOE_BITS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// 列表项“大模型（>8 GB，建议 KVMem）”标记位
pub static BIG_BITS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// 列表项“不受支持（加载会失败）”标记位
pub static UNSUPPORTED_BITS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// 列表控件原本的窗口过程（子类化用）
pub static SCAN_LIST_OLD: AtomicIsize = AtomicIsize::new(0);
static LIST_OLD_PROC: AtomicIsize = AtomicIsize::new(0);
/// 是否已登记列表的“鼠标移出”通知
pub static LIST_TRACKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 访问全局界面状态。
/// 注意：控件更新会同步回调 WM_COMMAND（如 EN_CHANGE），此时锁已被持有，
/// 必须用 try_lock 丢弃这类嵌套回声，否则 std::sync::Mutex 会自锁死。
pub fn with_gui<F: FnOnce(&mut Gui)>(f: F) {
    if let Some(cell) = GUI.get() {
        if let Some(mut g) = cell.try_lock().ok() {
            f(&mut g);
        }
    }
}

pub fn i18n(g: &Gui, key: &str) -> String {
    crate::i18n::t(&g.lang, key)
}

/// 启动界面：创建窗口并进入消息循环（阻塞）
pub fn run(shared: Shared) {
    init_common_controls();
    let hwnd = create_main_window();
    if hwnd == 0 {
        return;
    }
    // 首次布局与数据填充
    layout::build(shared, hwnd);
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        SetTimer(hwnd, TIMER_TICK, 1000, std::ptr::null());
        // 模型名滚动用的高频定时器（80ms 一格）
        SetTimer(hwnd, TIMER_MARQUEE, 80, std::ptr::null());
    }
    with_gui(|g| {
        actions::refresh_models(g);
        actions::refresh_status(g);
    });

    let mut msg: MSG = unsafe { std::mem::zeroed() };
    loop {
        let r = unsafe { GetMessageW(&mut msg, 0, 0, 0) };
        if r <= 0 {
            break;
        }
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    // 退出前停止 llama-server，避免孤儿进程
    with_gui(|g| {
        if let Ok(mut app) = g.shared.lock() {
            app.launcher.shutdown();
            let _ = app.store.save();
        }
    });
}

fn create_main_window() -> HWND {
    let brush = unsafe { CreateSolidBrush(theme::BG) };
    register_window_class(CLASS_NAME, wnd_proc, brush);
    // WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_THICKFRAME
    let style = 0x00C0_0000 | 0x0008_0000 | 0x0002_0000 | 0x0001_0000 | 0x0004_0000 | 0x0200_0000; // 含 WS_CLIPCHILDREN
    let title = crate::i18n::t(&crate::util::detect_system_lang(), "app.title");
    // 1196x927 → 客户区 1180x888，正好容纳 layout 设计稿
    let hwnd = create_window(CLASS_NAME, &title, 0, style, 60, 30, 1196, 927, 0, 0);
    if hwnd != 0 {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
    hwnd
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wp & 0xFFFF) as usize;
            let code = ((wp >> 16) & 0xFFFF) as u32;
            with_gui(|g| actions::on_command(g, hwnd, id, code));
            0
        }
        WM_TIMER => {
            if wp == TIMER_MARQUEE {
                with_gui(|g| actions::on_marquee_tick(g));
            } else {
                with_gui(|g| actions::on_tick(g));
            }
            0
        }
        // 顶栏“当前模型”由父窗口自绘（可滚动），其余交给默认处理
        WM_PAINT => {
            if hwnd == MAIN_HWND.load(Ordering::Relaxed) {
                let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
                let hdc = unsafe {
                    BeginPaint(hwnd, &mut ps as *mut PAINTSTRUCT as *mut std::ffi::c_void)
                };
                // 先把这次需要重绘的区域整块刷成背景色。
                // 父窗口本身只画“当前模型”那一行，其它区域靠这里兜底；
                // 少了这一步，控件移动/缩窄后留下的旧像素会一直残留在界面上。
                unsafe {
                    let br = CreateSolidBrush(theme::BG);
                    FillRect(hdc, &ps.rc_paint, br);
                    DeleteObject(br);
                }
                with_gui(|g| {
                    let font = FONT_UI.load(Ordering::Relaxed) as HFONT;
                    theme::paint_model_line(hdc, g.model_rect, &g.model_text, g.marquee_off, font);
                });
                unsafe {
                    EndPaint(hwnd, &ps as *const PAINTSTRUCT as *const std::ffi::c_void);
                    // 输入框外沿那圈 1px 边框要用整窗 DC 来画：
                    // BeginPaint 的 DC 被裁剪到失效区，紧贴控件的 1px 常落在裁剪区之外，
                    // 结果边框时有时无。窗口带 WS_CLIPCHILDREN，这个 DC 只覆盖控件以外的区域。
                    let dc = GetDC(hwnd);
                    theme::draw_field_frames(dc, hwnd);
                    ReleaseDC(hwnd, dc);
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_ERASEBKGND => {
            let hdc = wp as HDC;
            let (cw, ch) = client_size(hwnd);
            let all = RECT {
                left: 0,
                top: 0,
                right: cw,
                bottom: ch,
            };
            let is_main = hwnd == MAIN_HWND.load(Ordering::Relaxed);
            unsafe {
                if !is_main {
                    // 二级窗口（高级设置）整体淡蓝底 + 输入框细边框
                    FillRect(hdc, &all, BRUSH.load(Ordering::Relaxed) as HBRUSH);
                    theme::draw_field_frames(hdc, hwnd);
                    return 1;
                }
                let k = layout::scale(hwnd);
                let header_h = layout::s(layout::HEADER_H, k);
                let mut bar = all;
                bar.bottom = header_h;
                let mut body = all;
                body.top = header_h;
                FillRect(hdc, &body, BRUSH.load(Ordering::Relaxed) as HBRUSH);
                FillRect(hdc, &bar, BRUSH_BAR.load(Ordering::Relaxed) as HBRUSH);
                // 顶栏 / 日志区各一条细分隔线
                let pen = CreateSolidBrush(theme::BORDER);
                let line = RECT {
                    left: 0,
                    top: header_h,
                    right: cw,
                    bottom: header_h + 1,
                };
                FillRect(hdc, &line, pen);
                let ly = layout::s(686, k);
                let line2 = RECT {
                    left: 0,
                    top: ly,
                    right: cw,
                    bottom: ly + 1,
                };
                FillRect(hdc, &line2, pen);
                DeleteObject(pen);
                // 输入框 / 列表框统一画淡蓝细边框（扁平风格）
                theme::draw_field_frames(hdc, hwnd);
            }
            1
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wp as HDC;
            let ctl = lp as HWND;
            let id = GetDlgCtrlID(ctl) as usize;
            let (text, brush) = if id == layout::ID_PILL {
                (
                    PILL_COLOR.load(Ordering::Relaxed),
                    BRUSH_BAR.load(Ordering::Relaxed),
                )
            } else if (900..=910).contains(&id) {
                (theme::TEXT_DIM, BRUSH_BAR.load(Ordering::Relaxed))
            } else {
                (theme::TEXT, BRUSH.load(Ordering::Relaxed))
            };
            unsafe {
                SetBkMode(hdc, 1);
                SetTextColor(hdc, text);
            }
            brush as LRESULT
        }
        WM_CTLCOLORBTN => {
            let hdc = wp as HDC;
            unsafe {
                SetBkMode(hdc, 1);
                SetTextColor(hdc, theme::TEXT);
            }
            BRUSH.load(Ordering::Relaxed) as LRESULT
        }
        WM_CTLCOLOREDIT => {
            let hdc = wp as HDC;
            let ctl = lp as HWND;
            // 只读的“参数预览 / 运行日志”用更淡的底色，与可编辑输入框区分
            let alt = ctl == HWND_ARGS.load(Ordering::Relaxed)
                || ctl == HWND_LOG.load(Ordering::Relaxed);
            let brush = if alt {
                BRUSH_LOG.load(Ordering::Relaxed)
            } else {
                BRUSH_FIELD.load(Ordering::Relaxed)
            };
            unsafe {
                SetBkMode(hdc, 1);
                SetTextColor(hdc, theme::TEXT);
            }
            brush as LRESULT
        }
        WM_CTLCOLORLISTBOX => {
            let hdc = wp as HDC;
            unsafe {
                SetBkMode(hdc, 1);
                SetTextColor(hdc, theme::TEXT);
            }
            BRUSH_FIELD.load(Ordering::Relaxed) as LRESULT
        }
        WM_DRAWITEM => {
            let dis = &*(lp as *const DRAWITEMSTRUCT);
            // ODT_BUTTON = 4：自绘按钮
            if dis.ctl_type == 4 {
                let text = get_text(dis.hwnd_item);
                let kind = theme::Btn::of(dis.ctl_id as usize);
                let font = FONT_UI.load(Ordering::Relaxed) as HFONT;
                theme::paint_button(dis.hdc, dis.rc_item, &text, kind, dis.item_state, font);
                return 1;
            }
            // ODT_LISTBOX = 2：模型列表（识图模型带小图标、不受支持的标红）
            if dis.ctl_type == 2 {
                let idx = dis.item_id as usize;
                let text = list_text(dis.hwnd_item, idx);
                let selected = dis.item_state & ODS_SELECTED != 0;
                let bits = VISION_BITS.load(Ordering::Relaxed);
                let vision = idx < 64 && (bits >> idx) & 1 == 1;
                let bad = idx < 64 && (UNSUPPORTED_BITS.load(Ordering::Relaxed) >> idx) & 1 == 1;
                let font = FONT_UI.load(Ordering::Relaxed) as HFONT;
                let moe = idx < 64 && (MOE_BITS.load(Ordering::Relaxed) >> idx) & 1 == 1;
                let (lw, _lh) = client_size(dis.hwnd_item);
                let (lw, _lh) = client_size(dis.hwnd_item);
                let big = idx < 64 && (BIG_BITS.load(Ordering::Relaxed) >> idx) & 1 == 1;
                theme::paint_list_item(dis.hdc, dis.rc_item, &text, selected, vision, bad, moe, big, lw, font);
                return 1;
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        // 模型列表右键：删除该项
        WM_CONTEXTMENU => {
            let target = wp as HWND;
            let is_list = with_gui(|g| {
                if target == g.list {
                    actions::model_context_menu(g, hwnd, lp);
                }
            });
            let _ = is_list;
            0
        }
        WM_CLOSE => {
            // 二级窗口（高级设置）关闭时只销毁自己，不要连带结束服务
            if hwnd != MAIN_HWND.load(Ordering::Relaxed) {
                // 关窗（X 或返回）后必须把句柄清零，否则下次点“高级设置”会去 show 一个已销毁的窗口
                with_gui(|g| {
                    if g.adv_hwnd == hwnd {
                        g.adv_hwnd = 0;
                        // 控件已随窗口销毁，句柄必须清掉，
                        // 否则“保存设置”会去读已销毁控件 → 读到空 → 写成 0
                        g.adv_fields.clear();
                    }
                    if g.help_hwnd == hwnd {
                        g.help_hwnd = 0;
                    }
                    if g.auto_hwnd == hwnd {
                        g.auto_hwnd = 0;
                        g.auto_fields.clear();
                    // 显存估算窗口：X 关闭也要清句柄，否则再次点“显存估算”打不开
                    if g.fit_hwnd == hwnd {
                        g.fit_hwnd = 0;
                    }
                    }
                });
                DestroyWindow(hwnd);
                return 0;
            }
            with_gui(|g| {
                if g.adv_hwnd != 0 {
                    DestroyWindow(g.adv_hwnd);
                    g.adv_hwnd = 0;
                }
                if let Ok(mut app) = g.shared.lock() {
                    app.launcher.shutdown();
                    let _ = app.store.save();
                }
            });
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            // 只有主窗口退出才结束消息循环（高级设置等窗口共用同一个窗口过程）
            if hwnd == MAIN_HWND.load(Ordering::Relaxed) {
                PostQuitMessage(0);
            } else {
                // 二级窗口被销毁：同步清掉句柄
                with_gui(|g| {
                    if g.adv_hwnd == hwnd {
                        g.adv_hwnd = 0;
                        g.adv_fields.clear();
                    }
                    if g.help_hwnd == hwnd {
                        g.help_hwnd = 0;
                    }
                    if g.auto_hwnd == hwnd {
                        g.auto_hwnd = 0;
                        g.auto_fields.clear();
                    // 显存估算窗口：X 关闭也要清句柄，否则再次点“显存估算”打不开
                    if g.fit_hwnd == hwnd {
                        g.fit_hwnd = 0;
                    }
                    }
                });
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// 供控件创建后调用：注册全局状态
pub fn install(gui: Gui) {
    let _ = GUI.set(Mutex::new(gui));
}

/// 便捷取值：当前配置快照
pub fn cfg(g: &Gui) -> crate::json::Json {
    g.shared
        .lock()
        .map(|a| a.store.cfg.clone())
        .unwrap_or_else(|_| crate::settings::defaults())
}
