//! Win32 原生窗口层（仅 Windows，直接调用系统 API，不引入任何第三方 crate）
//! Native Win32 layer: raw FFI bindings and small helpers, std only.

#![allow(non_snake_case, dead_code)]

use std::ffi::c_void;

pub type HWND = isize;
pub type HINSTANCE = isize;
pub type HMENU = isize;
pub type HICON = isize;
pub type HFONT = isize;
pub type HBRUSH = isize;
pub type HDC = isize;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;

pub type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: u32,
    pub style: u32,
    pub lpfnWndProc: Option<WNDPROC>,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: isize,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
    pub hIconSm: HICON,
}

#[repr(C)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: u32,
    pub pt: POINT,
    pub lPrivate: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct OPENFILENAMEW {
    pub lStructSize: u32,
    pub hwndOwner: HWND,
    pub hInstance: HINSTANCE,
    pub lpstrFilter: *const u16,
    pub lpstrCustomFilter: *mut u16,
    pub nMaxCustFilter: u32,
    pub nFilterIndex: u32,
    pub lpstrFile: *mut u16,
    pub nMaxFile: u32,
    pub lpstrFileTitle: *mut u16,
    pub nMaxFileTitle: u32,
    pub lpstrInitialDir: *const u16,
    pub lpstrTitle: *const u16,
    pub Flags: u32,
    pub nFileOffset: u16,
    pub nFileExtension: u16,
    pub lpstrDefExt: *const u16,
    pub lCustData: LPARAM,
    pub lpfnHook: *const c_void,
    pub lpTemplateName: *const u16,
    pub pvReserved: *mut c_void,
    pub dwReserved: u32,
    pub FlagsEx: u32,
}

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassExW(class: *const WNDCLASSEXW) -> u16;
    pub fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    pub fn DefWindowProcW(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn ShowWindow(hwnd: HWND, cmd: i32) -> i32;
    pub fn UpdateWindow(hwnd: HWND) -> i32;
    pub fn DestroyWindow(hwnd: HWND) -> i32;
    pub fn GetMessageW(msg: *mut MSG, hwnd: HWND, min: u32, max: u32) -> i32;
    pub fn TranslateMessage(msg: *const MSG) -> i32;
    pub fn DispatchMessageW(msg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(code: i32);
    pub fn PostMessageW(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> i32;
    pub fn SendMessageW(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn SetWindowTextW(hwnd: HWND, text: *const u16) -> i32;
    pub fn GetWindowTextW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    pub fn GetWindowTextLengthW(hwnd: HWND) -> i32;
    pub fn SetTimer(hwnd: HWND, id: usize, ms: u32, proc: *const c_void) -> usize;
    pub fn KillTimer(hwnd: HWND, id: usize) -> i32;
    pub fn EnableWindow(hwnd: HWND, enable: i32) -> i32;
    pub fn MoveWindow(hwnd: HWND, x: i32, y: i32, w: i32, h: i32, repaint: i32) -> i32;
    pub fn SetWindowPos(
        hwnd: HWND,
        after: HWND,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    pub fn GetClientRect(hwnd: HWND, rect: *mut RECT) -> i32;
    pub fn LoadCursorW(instance: HINSTANCE, name: *const u16) -> isize;
    pub fn LoadIconW(instance: HINSTANCE, name: *const u16) -> HICON;
    pub fn MessageBoxW(hwnd: HWND, text: *const u16, caption: *const u16, flags: u32) -> i32;
    pub fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
    pub fn SetFocus(hwnd: HWND) -> HWND;
    pub fn GetDpiForWindow(hwnd: HWND) -> u32;
    pub fn InvalidateRect(hwnd: HWND, rect: *const RECT, erase: i32) -> i32;
    pub fn DrawTextW(hdc: HDC, text: *const u16, len: i32, rect: *mut RECT, format: u32) -> i32;
    pub fn FillRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> i32;
    pub fn LoadImageW(
        instance: HINSTANCE,
        name: *const u16,
        kind: u32,
        cx: i32,
        cy: i32,
        load: u32,
    ) -> isize;
    pub fn OpenClipboard(owner: HWND) -> i32;
    pub fn EmptyClipboard() -> i32;
    pub fn CloseClipboard() -> i32;
    pub fn SetClipboardData(format: u32, mem: isize) -> isize;
    pub fn GetDlgCtrlID(hwnd: HWND) -> i32;
    pub fn BeginPaint(hwnd: HWND, ps: *mut c_void) -> HDC;
    pub fn EndPaint(hwnd: HWND, ps: *const c_void) -> i32;
    pub fn GetParent(hwnd: HWND) -> HWND;
    pub fn IsWindowVisible(hwnd: HWND) -> i32;
    pub fn IsWindow(hwnd: HWND) -> i32;
    pub fn GetSystemMetrics(index: i32) -> i32;

    pub fn GetWindowRect(hwnd: HWND, rect: *mut RECT) -> i32;
    pub fn ScreenToClient(hwnd: HWND, pt: *mut POINT) -> i32;
    pub fn GetClassNameW(hwnd: HWND, buf: *mut u16, max: i32) -> i32;
    pub fn EnumChildWindows(parent: HWND, cb: CHILD_PROC, lp: LPARAM) -> i32;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn AppendMenuW(menu: HMENU, flags: u32, id: usize, text: *const u16) -> i32;
    pub fn TrackPopupMenu(
        menu: HMENU,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        hwnd: HWND,
        rect: *const RECT,
    ) -> i32;
    pub fn DestroyMenu(menu: HMENU) -> i32;
    pub fn GetCursorPos(pt: *mut POINT) -> i32;
    pub fn TrackMouseEvent(tme: *mut TRACKMOUSEEVENT) -> i32;
    pub fn GetDC(hwnd: HWND) -> HDC;
    pub fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    pub fn SetWindowLongPtrW(hwnd: HWND, index: i32, value: isize) -> isize;
    pub fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;

    pub fn CallWindowProcW(prev: isize, hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT;
    pub fn ClientToScreen(hwnd: HWND, pt: *mut POINT) -> i32;
}

pub const GWLP_WNDPROC: i32 = -4;
pub const GWLP_USERDATA: i32 = -21;
pub const WM_NCCALCSIZE: u32 = 0x0083;
pub const EM_SETMARGINS: u32 = 0x00D3;
pub const EC_LEFTMARGIN: usize = 0x0001;
pub const EC_RIGHTMARGIN: usize = 0x0002;

/// WM_NCCALCSIZE（wParam=TRUE）用的结构
#[repr(C)]
pub struct NCCALCSIZE_PARAMS {
    pub rgrc: [RECT; 3],
    pub lppos: *mut c_void,
}
pub const WS_EX_NOACTIVATE: u32 = 0x0800_0000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SIZE {
    pub cx: i32,
    pub cy: i32,
}

/// PAINTSTRUCT：hdc + BOOL + RECT + 两个 BOOL + 32 字节保留
#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub f_erase: i32,
    pub rc_paint: RECT,
    pub f_restore: i32,
    pub f_inc_update: i32,
    pub reserved: [u8; 32],
}

#[repr(C)]
pub struct TRACKMOUSEEVENT {
    pub cb_size: u32,
    pub flags: u32,
    pub hwnd: HWND,
    pub hover_time: u32,
}

pub const TME_LEAVE: u32 = 0x0000_0002;
pub const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
pub const WS_EX_TOPMOST: u32 = 0x0000_0008;

#[link(name = "shell32")]
extern "system" {
    pub fn SHBrowseForFolderW(bi: *mut BROWSEINFOW) -> *mut c_void;
    pub fn SHGetPathFromIDListW(pidl: *const c_void, path: *mut u16) -> i32;
    pub fn ShellExecuteW(
        hwnd: HWND,
        op: *const u16,
        file: *const u16,
        params: *const u16,
        dir: *const u16,
        show: i32,
    ) -> isize;
}

/// 用系统默认程序打开 URL 或文件
pub fn open_url(url: &str) {
    let op = wide("open");
    let u = wide(url);
    unsafe {
        ShellExecuteW(0, op.as_ptr(), u.as_ptr(), std::ptr::null(), std::ptr::null(), 1);
    }
}

/// 在资源管理器里定位并选中某个文件（文件不存在则打开其所在目录）
pub fn reveal_in_explorer(path: &str) {
    let p = std::path::Path::new(path);
    let op = wide("open");
    if p.is_file() {
        // explorer /select,"C:\path\file"
        let params = wide(&format!("/select,\"{}\"", path));
        let file = wide("explorer.exe");
        unsafe {
            ShellExecuteW(0, op.as_ptr(), file.as_ptr(), params.as_ptr(), std::ptr::null(), 1);
        }
        return;
    }
    // 文件不存在：退而打开父目录
    if let Some(dir) = p.parent() {
        if dir.is_dir() {
            let d = wide(&dir.to_string_lossy());
            unsafe {
                ShellExecuteW(0, op.as_ptr(), d.as_ptr(), std::ptr::null(), std::ptr::null(), 1);
            }
            return;
        }
    }
    if p.is_dir() {
        let d = wide(path);
        unsafe {
            ShellExecuteW(0, op.as_ptr(), d.as_ptr(), std::ptr::null(), std::ptr::null(), 1);
        }
    }
}

#[link(name = "ole32")]
extern "system" {
    pub fn CoTaskMemFree(p: *mut c_void);
}

#[repr(C)]
pub struct BROWSEINFOW {
    pub owner: HWND,
    pub pidl_root: *mut c_void,
    pub display_name: *mut u16,
    pub title: *const u16,
    pub flags: u32,
    pub callback: *const c_void,
    pub lparam: LPARAM,
    pub image: i32,
}

pub type CHILD_PROC = unsafe extern "system" fn(HWND, LPARAM) -> i32;

#[link(name = "comctl32")]
extern "system" {
    pub fn InitCommonControlsEx(icc: *const INITCOMMONCONTROLSEX) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GlobalAlloc(flags: u32, bytes: usize) -> isize;
    pub fn GlobalLock(mem: isize) -> *mut c_void;
    pub fn GlobalUnlock(mem: isize) -> i32;
    pub fn GetLastError() -> u32;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strikeout: u32,
        charset: u32,
        out_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> HFONT;
    pub fn DeleteObject(obj: isize) -> i32;
    pub fn CreateSolidBrush(color: u32) -> HBRUSH;
    pub fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    pub fn SetTextColor(hdc: HDC, color: u32) -> u32;
    pub fn SetBkColor(hdc: HDC, color: u32) -> u32;
    pub fn RoundRect(hdc: HDC, l: i32, t: i32, r: i32, b: i32, ew: i32, eh: i32) -> i32;
    pub fn Rectangle(hdc: HDC, l: i32, t: i32, r: i32, b: i32) -> i32;
    pub fn SelectObject(hdc: HDC, obj: isize) -> isize;
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn CreateCompatibleBitmap(hdc: HDC, w: i32, h: i32) -> isize;
    pub fn BitBlt(dst: HDC, x: i32, y: i32, w: i32, h: i32, src: HDC, sx: i32, sy: i32, rop: u32) -> i32;
    pub fn DeleteDC(hdc: HDC) -> i32;
    pub fn GetStockObject(kind: i32) -> isize;
    pub fn CreatePen(style: u32, width: i32, color: u32) -> isize;
    pub fn Polygon(hdc: HDC, pts: *const POINT, count: i32) -> i32;
    pub fn Ellipse(hdc: HDC, l: i32, t: i32, r: i32, b: i32) -> i32;
    pub fn IntersectClipRect(hdc: HDC, l: i32, t: i32, r: i32, b: i32) -> i32;
    pub fn TextOutW(hdc: HDC, x: i32, y: i32, text: *const u16, len: i32) -> i32;
    pub fn SaveDC(hdc: HDC) -> i32;
    pub fn RestoreDC(hdc: HDC, saved: i32) -> i32;
    pub fn GetTextExtentPoint32W(hdc: HDC, text: *const u16, len: i32, size: *mut SIZE) -> i32;
}

#[link(name = "comdlg32")]
extern "system" {
    pub fn GetOpenFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
}

// ---------------- 常量 ----------------
pub const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
pub const WS_POPUP: u32 = 0x8000_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_TABSTOP: u32 = 0x0001_0000;
pub const WS_VSCROLL: u32 = 0x0020_0000;
/// BitBlt 光栅操作：直接拷贝
pub const SRCCOPY: u32 = 0x00CC_0020;
/// 滚轮与竖向滚动
pub const WM_MOUSEWHEEL: u32 = 0x020A;
pub const WM_VSCROLL: u32 = 0x0115;
pub const SB_LINEUP: usize = 0;
pub const SB_LINEDOWN: usize = 1;
/// GetWindowLong 的样式索引
pub const GWL_STYLE: i32 = -16;
/// 系统垂直滚动条宽度
pub const SM_CXVSCROLL: i32 = 2;
pub const WS_BORDER: u32 = 0x0080_0000;
pub const WS_EX_CLIENTEDGE: u32 = 0x0000_0200;
pub const ES_MULTILINE: u32 = 0x0004;
pub const ES_AUTOVSCROLL: u32 = 0x0040;
pub const ES_AUTOHSCROLL: u32 = 0x0080;
pub const ES_READONLY: u32 = 0x0800;
pub const LBS_NOTIFY: u32 = 0x0001;
pub const LBS_OWNERDRAWFIXED: u32 = 0x0010;
pub const LBS_HASSTRINGS: u32 = 0x0040;
pub const LB_SETITEMHEIGHT: u32 = 0x01A0;
pub const LB_ITEMFROMPOINT: u32 = 0x01A9;
pub const LB_GETTEXTLEN_MSG: u32 = 0x018A;
pub const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
pub const BS_AUTOCHECKBOX: u32 = 0x0003;
pub const BS_GROUPBOX: u32 = 0x0007;
pub const CBS_DROPDOWNLIST: u32 = 0x0003;
pub const CBS_HASSTRINGS: u32 = 0x0200;
pub const SW_SHOW: i32 = 5;
pub const SW_HIDE: i32 = 0;
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_TIMER: u32 = 0x0113;
pub const WM_CTLCOLORSTATIC: u32 = 0x0138;
pub const WM_CTLCOLORBTN: u32 = 0x0135;
pub const WM_SETFONT: u32 = 0x0030;
pub const CB_ADDSTRING: u32 = 0x0143;
pub const CB_SETITEMHEIGHT: u32 = 0x0153;
pub const CB_GETITEMHEIGHT: u32 = 0x0154;
pub const CB_GETCURSEL: u32 = 0x0147;
pub const CB_SETCURSEL: u32 = 0x014E;
pub const CB_RESETCONTENT: u32 = 0x014B;
pub const CB_GETLBTEXT: u32 = 0x0148;
pub const CB_GETLBTEXTLEN: u32 = 0x0149;
pub const BM_GETCHECK: u32 = 0x00F0;
pub const BM_SETCHECK: u32 = 0x00F1;
pub const LB_ADDSTRING: u32 = 0x0180;
pub const LB_RESETCONTENT: u32 = 0x0184;
pub const LB_SETCURSEL: u32 = 0x0186;
// 注意：LB_GETCURSEL 是 0x0188（0x0187 是 LB_GETSEL，取“某个条目是否被选中”），
// 之前误用 LB_GETSEL 导致任何点击都被读成第 0 项，模型选择总被换回默认模型。
pub const LB_GETCURSEL: u32 = 0x0188;
pub const LB_GETSEL: u32 = 0x0187;
pub const LB_GETTEXTLEN: u32 = 0x018A;
pub const LB_GETTEXT: u32 = 0x0189;
pub const EM_SETSEL: u32 = 0x00B1;
pub const EM_SCROLLCARET: u32 = 0x00B7;
pub const IDC_ARROW: usize = 32512;
pub const COLOR_WINDOW: isize = 5;
pub const MB_OK: u32 = 0x0000;
pub const MB_ICONWARNING: u32 = 0x0030;
pub const MB_ICONINFORMATION: u32 = 0x0040;

// ---- 自绘按钮 / 消息补充 ----
pub const WM_PAINT: u32 = 0x000F;
pub const WM_ERASEBKGND: u32 = 0x0014;
pub const WM_DRAWITEM: u32 = 0x002B;
pub const WM_CONTEXTMENU: u32 = 0x007B;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_MOUSELEAVE: u32 = 0x02A3;
pub const WM_CTLCOLOREDIT: u32 = 0x0133;
pub const WM_CTLCOLORLISTBOX: u32 = 0x0134;
pub const BS_OWNERDRAW: u32 = 0x000B;
pub const ODS_SELECTED: u32 = 0x0001;
pub const ODS_DISABLED: u32 = 0x0004;
pub const ODS_FOCUS: u32 = 0x0010;
pub const DT_LEFT: u32 = 0x0000;
pub const DT_CENTER: u32 = 0x0001;
pub const DT_RIGHT: u32 = 0x0002;
pub const DT_VCENTER: u32 = 0x0004;
pub const DT_SINGLELINE: u32 = 0x0020;
pub const DT_WORDBREAK: u32 = 0x0010;
pub const DT_END_ELLIPSIS: u32 = 0x8000;
pub const HOLLOW_BRUSH: i32 = 5;
pub const NULL_PEN: i32 = 8;
pub const IMAGE_ICON: u32 = 1;
pub const LR_SHARED: u32 = 0x8000;
pub const CF_UNICODETEXT: u32 = 13;
pub const GMEM_MOVEABLE: u32 = 0x0002;
pub const ICC_WIN95_CLASSES: u32 = 0x0000_00FF;

/// 自绘控件信息（DRAWITEMSTRUCT）：x64 下 HWND 需 8 字节对齐
#[repr(C)]
pub struct DRAWITEMSTRUCT {
    pub ctl_type: u32,
    pub ctl_id: u32,
    pub item_id: u32,
    pub item_action: u32,
    pub item_state: u32,
    pub hwnd_item: HWND,
    pub hdc: HDC,
    pub rc_item: RECT,
    pub item_data: usize,
}

#[repr(C)]
pub struct INITCOMMONCONTROLSEX {
    pub size: u32,
    pub icc: u32,
}

#[repr(C)]
pub struct TOOLINFOW {
    pub cb_size: u32,
    pub flags: u32,
    pub hwnd: HWND,
    pub id: usize,
    pub rect: RECT,
    pub inst: HINSTANCE,
    pub text: *mut u16,
    pub lparam: LPARAM,
    pub reserved: *mut c_void,
}

pub const TTF_IDISHWND: u32 = 0x0001;
pub const TTF_SUBCLASS: u32 = 0x0010;
pub const TTM_ADDTOOLW: u32 = 0x0400 + 50;
pub const TTM_SETMAXTIPWIDTH: u32 = 0x0400 + 24;
// Unicode 版是 WM_USER + 57（WM_USER + 12 是 ANSI 版，会导致工具项错乱）
pub const TTM_UPDATETIPTEXTW: u32 = 0x0400 + 57;
pub const TTS_ALWAYSTIP: u32 = 0x0001;
pub const TTS_NOPREFIX: u32 = 0x0002;
pub const TOOLTIPS_CLASSW: &str = "tooltips_class32";

pub fn wide(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

pub fn get_module() -> HINSTANCE {
    unsafe { GetModuleHandleW(std::ptr::null()) }
}

/// 注册窗口类（重复注册已存在时返回旧的，不影响使用）
pub fn register_window_class(class_name: &str, proc: WNDPROC, background: HBRUSH) -> bool {
    let class = wide(class_name);
    let cursor = unsafe { LoadCursorW(0, IDC_ARROW as *const u16) };
    // 图标取自 exe 内嵌资源（build.rs 用 windres 打包 assets/icon.ico）
    let big_icon = load_app_icon(32);
    let small_icon = load_app_icon(16);
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: get_module(),
        hIcon: big_icon,
        hCursor: cursor,
        hbrBackground: background,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class.as_ptr(),
        hIconSm: small_icon,
    };
    unsafe { RegisterClassExW(&wc) != 0 }
}

pub fn create_window(
    class_name: &str,
    title: &str,
    ex_style: u32,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    parent: HWND,
    id: usize,
) -> HWND {
    let class = wide(class_name);
    let text = wide(title);
    unsafe {
        CreateWindowExW(
            ex_style,
            class.as_ptr(),
            text.as_ptr(),
            style,
            x,
            y,
            w,
            h,
            parent,
            id as HMENU,
            get_module(),
            std::ptr::null_mut(),
        )
    }
}

pub fn set_text(hwnd: HWND, text: &str) {
    if hwnd == 0 {
        return;
    }
    let w = wide(text);
    unsafe {
        SetWindowTextW(hwnd, w.as_ptr());
    }
}

pub fn get_text(hwnd: HWND) -> String {
    if hwnd == 0 {
        return String::new();
    }
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    let mut buf = vec![0u16; (len + 2) as usize];
    let got = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if got <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..got as usize])
}

pub fn set_font(hwnd: HWND, font: HFONT) {
    unsafe {
        SendMessageW(hwnd, WM_SETFONT, font as WPARAM, 1);
    }
}

pub fn enable(hwnd: HWND, on: bool) {
    unsafe {
        EnableWindow(hwnd, if on { 1 } else { 0 });
    }
}

pub fn combo_reset(hwnd: HWND) {
    unsafe {
        SendMessageW(hwnd, CB_RESETCONTENT, 0, 0);
    }
}

pub fn combo_add(hwnd: HWND, text: &str) {
    let w = wide(text);
    unsafe {
        SendMessageW(hwnd, CB_ADDSTRING, 0, w.as_ptr() as LPARAM);
    }
}

/// 当前选中项下标（无选中返回 0）
pub fn combo_index(hwnd: HWND) -> usize {
    let i = unsafe { SendMessageW(hwnd, 0x0147, 0, 0) }; // CB_GETCURSEL
    if i < 0 {
        0
    } else {
        i as usize
    }
}

/// 把下拉框“闭合状态”的高度调成 want 像素。
/// CBS_DROPDOWNLIST 的闭合高度 = 选择区项高 + 边框，光设窗口高度没用，
/// 必须用 CB_SETITEMHEIGHT(-1) 改选择区的项高。
pub fn combo_fit_height(hwnd: HWND, want: i32) {
    unsafe {
        let mut r = RECT::default();
        if GetWindowRect(hwnd, &mut r) == 0 {
            return;
        }
        let cur = r.bottom - r.top;
        let item = SendMessageW(hwnd, CB_GETITEMHEIGHT, usize::MAX, 0) as i32;
        if item <= 0 || cur <= 0 {
            return;
        }
        let border = cur - item; // 上下边框总高度，通常 6px
        let target_item = want - border;
        if target_item > 8 && target_item != item {
            SendMessageW(hwnd, CB_SETITEMHEIGHT, usize::MAX, target_item as LPARAM);
        }
    }
}

pub fn combo_select(hwnd: HWND, index: usize) {    unsafe {
        SendMessageW(hwnd, CB_SETCURSEL, index, 0);
    }
}

pub fn combo_sel(hwnd: HWND) -> Option<usize> {
    let r = unsafe { SendMessageW(hwnd, CB_GETCURSEL, 0, 0) };
    if r < 0 {
        None
    } else {
        Some(r as usize)
    }
}

pub fn combo_text(hwnd: HWND) -> String {
    let Some(idx) = combo_sel(hwnd) else {
        return String::new();
    };
    let len = unsafe { SendMessageW(hwnd, CB_GETLBTEXTLEN, idx, 0) };
    if len < 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    unsafe {
        SendMessageW(hwnd, CB_GETLBTEXT, idx, buf.as_mut_ptr() as LPARAM);
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

pub fn list_reset(hwnd: HWND) {
    unsafe {
        SendMessageW(hwnd, LB_RESETCONTENT, 0, 0);
    }
}

pub fn list_add(hwnd: HWND, text: &str) -> usize {
    let w = wide(text);
    unsafe { SendMessageW(hwnd, LB_ADDSTRING, 0, w.as_ptr() as LPARAM) as usize }
}

pub fn list_select(hwnd: HWND, index: usize) {
    unsafe {
        SendMessageW(hwnd, LB_SETCURSEL, index, 0);
    }
}

pub fn list_sel(hwnd: HWND) -> Option<usize> {
    let r = unsafe { SendMessageW(hwnd, LB_GETCURSEL, 0, 0) };
    if r < 0 {
        None
    } else {
        Some(r as usize)
    }
}

/// 取列表项文本
pub fn list_text(hwnd: HWND, index: usize) -> String {
    let len = unsafe { SendMessageW(hwnd, LB_GETTEXTLEN, index, 0) };
    if len < 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    unsafe {
        SendMessageW(hwnd, LB_GETTEXT, index, buf.as_mut_ptr() as LPARAM);
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

/// 由屏幕坐标求列表项索引（不在任何项上时返回 None）
pub fn list_index_from_point(hwnd: HWND, screen_x: i32, screen_y: i32) -> Option<usize> {
    let mut pt = POINT {
        x: screen_x,
        y: screen_y,
    };
    unsafe {
        ScreenToClient(hwnd, &mut pt);
    }
    // 低 16 位是索引，高 16 位表示是否落在条目上
    let r = unsafe { SendMessageW(hwnd, LB_ITEMFROMPOINT, 0, pt.x as LPARAM | ((pt.y as LPARAM) << 16)) };
    let idx = (r & 0xFFFF) as i32;
    let outside = (r >> 16) & 0xFFFF;
    if outside != 0 || idx < 0 {
        None
    } else {
        Some(idx as usize)
    }
}

pub fn check_get(hwnd: HWND) -> bool {
    unsafe { SendMessageW(hwnd, BM_GETCHECK, 0, 0) == 1 }
}

pub fn check_set(hwnd: HWND, on: bool) {
    unsafe {
        SendMessageW(hwnd, BM_SETCHECK, if on { 1 } else { 0 }, 0);
    }
}

pub fn msg_box(owner: HWND, text: &str, title: &str, flags: u32) {
    let t = wide(text);
    let c = wide(title);
    unsafe {
        MessageBoxW(owner, t.as_ptr(), c.as_ptr(), flags);
    }
}

/// 原生打开文件对话框（仅 .gguf）
pub fn open_file_dialog(owner: HWND, title: &str) -> Option<String> {
    unsafe {
        let mut file_buf = vec![0u16; 4096];
        // 过滤器：双方以 \0 分隔，末尾再补一个 \0
        let filter: Vec<u16> = "GGUF 模型 (*.gguf)\0*.gguf\0所有文件 (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let title_w = wide(title);
        let def_ext = wide("gguf");
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = owner;
        ofn.hInstance = get_module();
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrTitle = title_w.as_ptr();
        ofn.lpstrDefExt = def_ext.as_ptr();
        // OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_EXPLORER
        ofn.Flags = 0x0000_1000 | 0x0000_0800 | 0x0008_0000;
        if GetOpenFileNameW(&mut ofn) == 0 {
            return None;
        }
        let end = file_buf.iter().position(|c| *c == 0).unwrap_or(0);
        Some(String::from_utf16_lossy(&file_buf[..end]))
    }
}

/// 选择可执行文件（用于手动指定 llama-server）
pub fn open_exe_dialog(owner: HWND, title: &str) -> Option<String> {
    unsafe {
        let mut file_buf = vec![0u16; 4096];
        let filter: Vec<u16> =
            "可执行文件 (*.exe)\0*.exe\0所有文件 (*.*)\0*.*\0\0".encode_utf16().collect();
        let title_w = wide(title);
        let def_ext = wide("exe");
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = owner;
        ofn.hInstance = get_module();
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrTitle = title_w.as_ptr();
        ofn.lpstrDefExt = def_ext.as_ptr();
        ofn.Flags = 0x0000_1000 | 0x0000_0800 | 0x0008_0000;
        if GetOpenFileNameW(&mut ofn) == 0 {
            return None;
        }
        let end = file_buf.iter().position(|c| *c == 0).unwrap_or(0);
        Some(String::from_utf16_lossy(&file_buf[..end]))
    }
}

/// 创建界面字体（按 DPI 缩放）
pub fn create_font(hwnd: HWND, size_pt: i32, bold: bool) -> HFONT {
    create_font_face(hwnd, size_pt, bold, "Microsoft YaHei UI")
}

/// 指定字体的版本（日志/参数预览用等宽字体）
pub fn create_font_face(hwnd: HWND, size_pt: i32, bold: bool, name: &str) -> HFONT {
    let dpi = unsafe {
        let d = GetDpiForWindow(hwnd);
        if d == 0 {
            96
        } else {
            d
        }
    } as f32;
    let height = -((size_pt as f32) * dpi / 72.0) as i32;
    let face = wide(name);
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            if bold { 700 } else { 400 },
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            face.as_ptr(),
        )
    }
}

pub fn client_size(hwnd: HWND) -> (i32, i32) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        GetClientRect(hwnd, &mut rc);
    }
    (rc.right - rc.left, rc.bottom - rc.top)
}

pub fn move_to(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        MoveWindow(hwnd, x, y, w, h, 1);
    }
}

pub fn show(hwnd: HWND, visible: bool) {
    unsafe {
        ShowWindow(hwnd, if visible { SW_SHOW } else { SW_HIDE });
    }
}

/// 复制文本到剪贴板（CF_UNICODETEXT）
pub fn set_clipboard_text(owner: HWND, text: &str) {
    let data: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = data.len() * 2;
    unsafe {
        if OpenClipboard(owner) == 0 {
            return;
        }
        EmptyClipboard();
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if mem != 0 {
            let p = GlobalLock(mem);
            if !p.is_null() {
                std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, p as *mut u8, bytes);
                GlobalUnlock(mem);
                SetClipboardData(CF_UNICODETEXT, mem);
            }
        }
        CloseClipboard();
    }
}

/// 初始化常用控件（提示框需要）
pub fn init_common_controls() {
    let icc = INITCOMMONCONTROLSEX {
        size: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        icc: ICC_WIN95_CLASSES,
    };
    unsafe {
        InitCommonControlsEx(&icc);
    }
}

/// 创建一个提示框窗口（悬浮说明用）。失败时返回 0。
pub fn create_tooltip(owner: HWND) -> HWND {
    // 某些系统上未注册 Win95 控件类，这里再兜底注册一次
    init_common_controls();
    let class = wide(TOOLTIPS_CLASSW);
    let empty = wide("");
    let h = unsafe {
        CreateWindowExW(
            0x0000_0008, // WS_EX_TOPMOST
            class.as_ptr(),
            empty.as_ptr(),
            WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
            0,
            0,
            0,
            0,
            owner,
            0,
            get_module(),
            std::ptr::null_mut(),
        )
    };
    if h == 0 {
        // 记录失败原因，便于在日志里排查
        let err = unsafe { GetLastError() };
        crate::util::append_debug(&format!("create_tooltip failed: {}", err));
    }
    h
}

/// 给控件挂一条悬浮提示
pub fn tooltip_add(tip: HWND, ctl: HWND, text: &str) {
    if tip == 0 || ctl == 0 || text.is_empty() {
        return;
    }
    // TTM_ADDTOOLW 不保证复制字符串，这里把文案泄漏成静态内存，数量有限可接受
    let leaked: &'static mut [u16] = Box::leak(wide(text).into_boxed_slice());
    let info = TOOLINFOW {
        cb_size: std::mem::size_of::<TOOLINFOW>() as u32,
        flags: TTF_IDISHWND | TTF_SUBCLASS,
        hwnd: ctl,
        id: ctl as usize,
        rect: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        inst: get_module(),
        text: leaked.as_mut_ptr(),
        lparam: 0,
        reserved: std::ptr::null_mut(),
    };
    unsafe {
        SendMessageW(tip, TTM_SETMAXTIPWIDTH, 0, 320);
        SendMessageW(tip, TTM_ADDTOOLW, 0, &info as *const TOOLINFOW as LPARAM);
    }
}

/// 从 exe 资源里取出图标（资源 id = 1）
pub fn load_app_icon(size: i32) -> HICON {
    unsafe {
        LoadImageW(
            get_module(),
            1 as *const u16,
            IMAGE_ICON,
            size,
            size,
            0,
        )
    }
}

/// 更新已有提示条目的文案（用于动态内容，如当前模型全名）
pub fn tooltip_set_text(tip: HWND, ctl: HWND, text: &str) {
    if tip == 0 || ctl == 0 {
        return;
    }
    let leaked: &'static mut [u16] = Box::leak(wide(text).into_boxed_slice());
    let info = TOOLINFOW {
        cb_size: std::mem::size_of::<TOOLINFOW>() as u32,
        flags: TTF_IDISHWND,
        hwnd: ctl,
        id: ctl as usize,
        rect: RECT::default(),
        inst: 0,
        text: leaked.as_mut_ptr(),
        lparam: 0,
        reserved: std::ptr::null_mut(),
    };
    unsafe {
        SendMessageW(tip, TTM_UPDATETIPTEXTW, 0, &info as *const TOOLINFOW as LPARAM);
    }}

/// 选择文件夹（原生浏览目录对话框）
pub fn pick_folder(owner: HWND, title: &str) -> Option<String> {
    let mut buf = vec![0u16; 1024];
    let title_w = wide(title);
    let mut bi = BROWSEINFOW {
        owner,
        pidl_root: std::ptr::null_mut(),
        display_name: buf.as_mut_ptr(),
        title: title_w.as_ptr(),
        // BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE | BIF_EDITBOX
        flags: 0x0000_0001 | 0x0000_0040 | 0x0000_0010,
        callback: std::ptr::null(),
        lparam: 0,
        image: 0,
    };
    unsafe {
        let pidl = SHBrowseForFolderW(&mut bi);
        if pidl.is_null() {
            return None;
        }
        let mut path = vec![0u16; 1024];
        let ok = SHGetPathFromIDListW(pidl, path.as_mut_ptr());
        CoTaskMemFree(pidl);
        if ok == 0 {
            return None;
        }
        let end = path.iter().position(|c| *c == 0).unwrap_or(0);
        Some(String::from_utf16_lossy(&path[..end]))
    }
}

/// 在指定位置弹出右键菜单，返回被选中的命令 id（0 表示取消）
pub fn popup_menu(owner: HWND, items: &[(usize, String)], x: i32, y: i32) -> usize {
    unsafe {
        let menu = CreatePopupMenu();
        if menu == 0 {
            return 0;
        }
        for (id, text) in items {
            let w = wide(text);
            AppendMenuW(menu, 0, *id, w.as_ptr());
        }
        // TPM_RETURNCMD | TPM_RIGHTBUTTON
        let cmd = TrackPopupMenu(menu, 0x0100 | 0x0002, x, y, 0, owner, std::ptr::null());
        DestroyMenu(menu);
        if cmd <= 0 {
            0
        } else {
            cmd as usize
        }
    }
}

pub fn open_dark_ui_note() {}
