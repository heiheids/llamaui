//! 淡蓝色扁平主题：配色、圆角自绘按钮、分隔线
//! Light blue flat theme: palette, rounded owner-drawn buttons, separators.
//!
//! COLORREF 采用 0x00BBGGRR 排列（注意与 RGB 相反）。

use super::win::*;

// ---------------- 配色 ----------------
/// 主背景 #F2F7FD
pub const BG: u32 = 0x00FD_F7F2;
/// 顶栏 / 卡片 #E7F0FC
pub const BAR: u32 = 0x00FC_F0E7;
/// 输入框、列表底色 #FFFFFF
pub const FIELD: u32 = 0x00FF_FFFF;
/// 主色蓝 #2E6BE6
pub const ACCENT: u32 = 0x00E6_6B2E;
/// 主色按下态 #2457C0
pub const ACCENT_DOWN: u32 = 0x00C0_5724;
/// 主色悬停 #4A80EE
pub const ACCENT_HOVER: u32 = 0x00EE_804A;
/// 主文字 #26313D
pub const TEXT: u32 = 0x003D_3126;
/// 次文字 #5C6B7A
pub const TEXT_DIM: u32 = 0x007A_6B5C;
/// 边框 #D5E2F2
pub const BORDER: u32 = 0x00F2_E2D5;
/// 输入框描边（比分隔线略深，扁平风格下才看得清）#C9DAF0
pub const FRAME: u32 = 0x00F0_DAC9;
/// 日志区底色 #F7FAFE
pub const LOG_BG: u32 = 0x00FE_FAF7;
/// 危险 / 错误色 #D64545
pub const DANGER: u32 = 0x0045_45D6;
/// 提醒色（橙色，用于“建议走 KVMem”的大模型标记）
pub const WARN: u32 = 0x0015_8FE6;
/// 成功色 #1E9E6A
pub const OK: u32 = 0x006A_9E1E;

/// 按钮视觉类型
#[derive(Clone, Copy, PartialEq)]
pub enum Btn {
    /// 实心蓝底白字（主操作）
    Primary,
    /// 白底蓝字带边框（次操作）
    Ghost,
    /// 浅蓝底深色字（弱操作）
    Soft,
}

impl Btn {
    pub fn of(id: usize) -> Btn {
        use super::layout::*;
        match id {
            ID_START | ID_SAVE | ID_CHAT | ID_ADV_SAVE => Btn::Primary,
            ID_COPY_EP | ID_COPY_KEY | ID_LOG_CLEAR => Btn::Soft,
            _ => Btn::Ghost,
        }
    }
}

fn rgb_fill(hdc: HDC, rc: &RECT, color: u32) {
    unsafe {
        let br = CreateSolidBrush(color);
        FillRect(hdc, rc, br);
        DeleteObject(br);
    }
}

/// 圆角矩形填充 + 可选描边
fn round_box(hdc: HDC, rc: &RECT, color: u32, radius: i32, border: Option<u32>) {
    unsafe {
        let br = CreateSolidBrush(color);
        let old_br = SelectObject(hdc, br);
        let pen = match border {
            Some(c) => CreatePen(0, 1, c),
            None => GetStockObject(NULL_PEN),
        };
        let old_pen = SelectObject(hdc, pen);
        RoundRect(
            hdc,
            rc.left,
            rc.top,
            rc.right,
            rc.bottom,
            radius,
            radius,
        );
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_br);
        if border.is_some() {
            DeleteObject(pen);
        }
        DeleteObject(br);
    }
}

/// 绘制一个自绘按钮（扁平、直角）
pub fn paint_button(hdc: HDC, rc: RECT, text: &str, kind: Btn, state: u32, font: HFONT) {
    let disabled = state & ODS_DISABLED != 0;
    let pressed = state & ODS_SELECTED != 0;

    let mut rect = rc;
    rect.right -= 1;
    rect.bottom -= 1;

    let (bg, fg, border) = match kind {
        Btn::Primary => {
            let bg = if disabled {
                0x00F5_DC_C8 // #C8DCF5 浅蓝
            } else if pressed {
                ACCENT_DOWN
            } else {
                ACCENT
            };
            (bg, FIELD, None)
        }
        Btn::Ghost => {
            let bg = if pressed { 0x00FB_F1_E9 } else { FIELD };
            (
                bg,
                if disabled { 0x00C6_BB_B0 } else { ACCENT },
                Some(if disabled { 0x00F2_E2_D5 } else { BORDER }),
            )
        }
        Btn::Soft => {
            let bg = if disabled {
                0x00F7_F4_F2
            } else if pressed {
                0x00FA_E9_DC // #DCE9FA
            } else {
                0x00FD_F2_EA // #EAF2FD
            };
            (
                bg,
                if disabled { 0x00C6_BB_B0 } else { TEXT },
                Some(BORDER),
            )
        }
    };

    flat_box(hdc, &rect, bg, border);

    unsafe {
        SetBkMode(hdc, 1);
        SetTextColor(hdc, fg);
        SelectObject(hdc, font);
        let mut tr = rect;
        tr.left += 8;
        tr.right -= 8;
        let w = wide(text);
        DrawTextW(
            hdc,
            w.as_ptr(),
            (w.len() - 1) as i32,
            &mut tr,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
    }
}

/// 直角实心矩形 + 可选 1px 描边
fn flat_box(hdc: HDC, rc: &RECT, fill: u32, border: Option<u32>) {
    unsafe {
        let br = CreateSolidBrush(fill);
        let old_br = SelectObject(hdc, br);
        let pen = match border {
            Some(c) => CreatePen(0, 1, c),
            None => GetStockObject(NULL_PEN),
        };
        let old_pen = SelectObject(hdc, pen);
        Rectangle(hdc, rc.left, rc.top, rc.right, rc.bottom);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_br);
        if border.is_some() {
            DeleteObject(pen);
        }
        DeleteObject(br);
    }
}

/// 列表项绘制：选中高亮 + 可选“识图模型”小图标 + 单行省略
pub fn paint_list_item(
    hdc: HDC,
    rc: RECT,
    text: &str,
    selected: bool,
    vision: bool,
    unsupported: bool,
    moe: bool,
    kv_big: bool,
    list_w: i32,
    font: HFONT,
) {
    let bg = if selected { ACCENT } else { FIELD };
    // 不受支持（加载会失败）的模型用红色标出
    let fg = if selected {
        FIELD
    } else if unsupported {
        DANGER
    } else {
        TEXT
    };
    let icon = if selected { FIELD } else { ACCENT };

    unsafe {
        let br = CreateSolidBrush(bg);
        FillRect(hdc, &rc, br);
        DeleteObject(br);
    }

    let mut tx = rc.left + 8;
    if vision {
        draw_image_icon(hdc, rc.left + 7, rc.top + (rc.bottom - rc.top - 14) / 2, icon);
        tx = rc.left + 27;
    }
    // 右侧徽标区：从右往左依次排布，正文在徽标之前截断
    let rx = list_w.min(rc.right);
    let mut cursor = rx - 6;
    // MoE 徽标（贴着右边）
    if moe {
        let badge = RECT {
            left: cursor - 38,
            top: rc.top + 4,
            right: cursor,
            bottom: rc.top + 18,
        };
        unsafe {
            let br = CreateSolidBrush(if selected { FIELD } else { ACCENT });
            FillRect(hdc, &badge, br);
            DeleteObject(br);
            SetBkMode(hdc, 1);
            SetTextColor(hdc, if selected { ACCENT } else { FIELD });
            SelectObject(hdc, font);
            let t = wide("MoE");
            let mut tr = badge;
            tr.left += 3;
            DrawTextW(hdc, t.as_ptr(), -1, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            SetTextColor(hdc, fg);
        }
        cursor = badge.left - 6;
    }
    // 大模型（>8 GB，建议走 KVMem 引擎）
    if kv_big {
        let badge = RECT {
            left: cursor - 36,
            top: rc.top + 4,
            right: cursor,
            bottom: rc.top + 18,
        };
        unsafe {
            let br = CreateSolidBrush(if selected { FIELD } else { WARN });
            FillRect(hdc, &badge, br);
            DeleteObject(br);
            SetBkMode(hdc, 1);
            SetTextColor(hdc, if selected { WARN } else { FIELD });
            SelectObject(hdc, font);
            let t = wide("KV");
            let mut tr = badge;
            tr.left += 5;
            DrawTextW(hdc, t.as_ptr(), -1, &mut tr, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            SetTextColor(hdc, fg);
        }
        cursor = badge.left - 6;
    }
    let badge_right = cursor;

    unsafe {
        SetBkMode(hdc, 1);
        SetTextColor(hdc, fg);
        SelectObject(hdc, font);
        let mut tr = RECT {
            left: tx,
            top: rc.top,
            right: badge_right,
            bottom: rc.bottom,
        };
        let w = wide(text);
        DrawTextW(
            hdc,
            w.as_ptr(),
            (w.len() - 1) as i32,
            &mut tr,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
    }
}

/// 画一个 14x14 的“图片”小标志（相框 + 山 + 太阳）
fn draw_image_icon(hdc: HDC, x: i32, y: i32, color: u32) {
    unsafe {
        let pen = CreatePen(0, 1, color);
        let old_pen = SelectObject(hdc, pen);
        let old_br = SelectObject(hdc, GetStockObject(HOLLOW_BRUSH));
        Rectangle(hdc, x, y, x + 13, y + 13);
        // 山
        let br = CreateSolidBrush(color);
        SelectObject(hdc, br);
        let pts = [
            POINT { x: x + 1, y: y + 11 },
            POINT { x: x + 6, y: y + 5 },
            POINT { x: x + 11, y: y + 11 },
        ];
        Polygon(hdc, pts.as_ptr(), 3);
        // 太阳
        Ellipse(hdc, x + 8, y + 2, x + 12, y + 6);
        DeleteObject(br);
        SelectObject(hdc, old_br);
        SelectObject(hdc, old_pen);
        DeleteObject(pen);
    }
}

/// 给输入框 / 列表画一圈淡蓝细边框（扁平风格，取代系统的深色描边）
pub fn draw_field_frames(hdc: HDC, parent: HWND) {
    unsafe {
        EnumChildWindows(parent, frame_cb, hdc as LPARAM);
    }
}

unsafe extern "system" fn frame_cb(child: HWND, lp: LPARAM) -> i32 {
    let hdc = lp as HDC;
    let mut buf = [0u16; 32];
    let n = GetClassNameW(child, buf.as_mut_ptr(), buf.len() as i32);
    if n <= 0 {
        return 1;
    }
    let name = String::from_utf16_lossy(&buf[..n as usize]);
    // 只处理输入框与列表框；按钮 / 标签由各自逻辑绘制
    if !name.eq_ignore_ascii_case("Edit") && !name.eq_ignore_ascii_case("ListBox") {
        return 1;
    }
    if IsWindowVisible(child) == 0 {
        return 1;
    }
    let mut r = RECT::default();
    if GetWindowRect(child, &mut r) == 0 {
        return 1;
    }
    let mut tl = POINT { x: r.left, y: r.top };
    let mut br = POINT { x: r.right, y: r.bottom };
    let parent = GetParent(child);
    ScreenToClient(parent, &mut tl);
    ScreenToClient(parent, &mut br);

    let line = |x0: i32, y0: i32, x1: i32, y1: i32| {
        let rr = RECT {
            left: x0,
            top: y0,
            right: x1,
            bottom: y1,
        };
        let brush = CreateSolidBrush(FRAME);
        FillRect(hdc, &rr, brush);
        DeleteObject(brush);
    };
    // 输入框使用系统默认外观，客户区覆盖整个控件，父窗口不需要再补任何底色。
    // 之前在这里往“窗口矩形与客户区之间那一圈”刷白，圈宽由控件自己决定
    // （单行实测上 0 / 下 12、多行右 20 / 下 7），看起来就是一圈宽窄不均的白边。
    // 现在只沿控件外沿画一圈等宽 1px 的细边框。
    line(tl.x - 1, tl.y - 1, br.x + 1, tl.y);
    line(tl.x - 1, br.y, br.x + 1, br.y + 1);
    line(tl.x - 1, tl.y, tl.x, br.y);
    line(br.x, tl.y, br.x + 1, br.y);
    1
}

/// 顶栏“当前模型”一行：文字过长时在原位横向滚动
pub fn paint_model_line(hdc: HDC, rect: RECT, text: &str, offset: i32, font: HFONT) {
    unsafe {
        // 先把这一块底色刷干净（文字滚动后不会留下残影）
        let br = CreateSolidBrush(BAR);
        FillRect(hdc, &rect, br);
        DeleteObject(br);

        if text.is_empty() {
            return;
        }
        let saved = SaveDC(hdc);
        SetBkMode(hdc, 1);
        SetTextColor(hdc, TEXT_DIM);
        SelectObject(hdc, font);
        IntersectClipRect(hdc, rect.left, rect.top, rect.right, rect.bottom);
        let w = wide(text);
        let mut y = rect.top + (rect.bottom - rect.top - 18) / 2;
        if y < rect.top {
            y = rect.top;
        }
        TextOutW(hdc, rect.left - offset, y, w.as_ptr(), (w.len() - 1) as i32);
        RestoreDC(hdc, saved);
    }
}

/// 量一段文字的像素宽度
pub fn text_width(hdc: HDC, text: &str, font: HFONT) -> i32 {
    unsafe {
        let old = SelectObject(hdc, font);
        let w = wide(text);
        let mut size = SIZE::default();
        GetTextExtentPoint32W(hdc, w.as_ptr(), (w.len() - 1) as i32, &mut size);
        SelectObject(hdc, old);
        size.cx
    }
}

/// 绘制平铺背景卡片
pub fn fill_card(hdc: HDC, rc: &RECT, color: u32) {
    rgb_fill(hdc, rc, color);
}

/// 一条 1px 分隔线
pub fn hline(hdc: HDC, x0: i32, y: i32, x1: i32) {
    let rc = RECT {
        left: x0,
        top: y,
        right: x1,
        bottom: y + 1,
    };
    rgb_fill(hdc, &rc, BORDER);
}
