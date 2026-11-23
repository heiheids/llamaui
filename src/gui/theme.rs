//! 淡蓝色扁平主题：配色、圆角自绘按钮、分隔线
//! Light blue flat theme: palette, rounded owner-drawn buttons, separators.
//!
//! COLORREF 采用 0x00BBGGRR 排列（注意与 RGB 相反）。

use super::win::*;

// ---------------- 配色 ----------------
/// 主背景 #F2F7FD
pub const BG: u32 = 0x00FD_F7F2;
/// 顶栏 / 卡片：与页面底色一致，顶部不再用更深一档的色块
pub const BAR: u32 = 0x00FD_F7F2;
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
/// 实测/正确状态用的绿色
pub const TEXT_OK: u32 = 0x0016_A34A;
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
    // 客户区被内缩后，窗口矩形与客户区之间那圈内边距要由父窗口补成输入框底色，
    // 否则会露出页面底色，看着像框里缺了一块。补完再沿外沿画 1px 边框。
    unsafe {
        let mut cr = RECT::default();
        if GetClientRect(child, &mut cr) != 0 {
            let mut c0 = POINT { x: 0, y: 0 };
            let mut c1 = POINT {
                x: cr.right,
                y: cr.bottom,
            };
            ClientToScreen(child, &mut c0);
            ClientToScreen(child, &mut c1);
            ScreenToClient(parent, &mut c0);
            ScreenToClient(parent, &mut c1);
            let white = CreateSolidBrush(FIELD);
            let mut strips = vec![
                // 上
                RECT { left: tl.x, top: tl.y, right: br.x, bottom: c0.y },
                // 下
                RECT { left: tl.x, top: c1.y, right: br.x, bottom: br.y },
                // 左
                RECT { left: tl.x, top: c0.y, right: c0.x, bottom: c1.y },
            ];
            // 带垂直滚动条时右侧边条就是滚动条本体，涂了会把滚动条盖掉
            let has_vscroll = (GetWindowLongPtrW(child, GWL_STYLE) as u32 & WS_VSCROLL) != 0;
            if !has_vscroll {
                strips.push(RECT {
                    left: c1.x,
                    top: c0.y,
                    right: br.x,
                    bottom: c1.y,
                });
            }
            for s in strips.iter() {
                FillRect(hdc, s, white);
            }
            DeleteObject(white);
        }
    }
    // 沿控件外沿再画一圈等宽 1px 的细边框
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

// ---------------------------------------------------------------------------
// 数据监控窗口的绘制
// ---------------------------------------------------------------------------

/// 在 (x, y) 处写一行文字，返回占用高度
fn draw_text(hdc: HDC, x: i32, y: i32, text: impl AsRef<str>, color: u32, font: HFONT) -> i32 {
    let text = text.as_ref();
    unsafe {
        let saved = SaveDC(hdc);
        SetBkMode(hdc, 1);
        SetTextColor(hdc, color);
        SelectObject(hdc, font);
        let w = wide(text);
        TextOutW(hdc, x, y, w.as_ptr(), (w.len() - 1) as i32);
        let mut size = SIZE::default();
        GetTextExtentPoint32W(hdc, w.as_ptr(), (w.len() - 1) as i32, &mut size);
        RestoreDC(hdc, saved);
        size.cy.max(1)
    }
}

/// 一块带边框的面板：填底色 + 1px 边框
fn panel_frame(hdc: HDC, rc: &RECT) {
    rgb_fill(hdc, rc, FIELD);
    unsafe {
        let edge = CreateSolidBrush(FRAME);
        let strips = [
            RECT { left: rc.left, top: rc.top, right: rc.right, bottom: rc.top + 1 },
            RECT { left: rc.left, top: rc.bottom - 1, right: rc.right, bottom: rc.bottom },
            RECT { left: rc.left, top: rc.top, right: rc.left + 1, bottom: rc.bottom },
            RECT { left: rc.right - 1, top: rc.top, right: rc.right, bottom: rc.bottom },
        ];
        for s in strips.iter() {
            FillRect(hdc, s, edge);
        }
        DeleteObject(edge);
    }
}

/// 数字格式化：带一位小数
fn fmt1(v: f64) -> String {
    if v <= 0.0 {
        "—".to_string()
    } else {
        format!("{:.1}", v)
    }
}

/// 监控窗口整体绘制（三个实时框 + 请求历史表）
pub fn paint_monitor(
    hdc: HDC,
    rect: RECT,
    snap: &crate::monitor::Snapshot,
    font: HFONT,
    font_bold: HFONT,
    tr: &dyn Fn(&str) -> String,
) {
    // 背景
    rgb_fill(hdc, &rect, BG);

    let left = rect.left + 16;
    let right = rect.right - 16;
    let mut y = rect.top + 12;

    // ---- 头部：模型 / 引擎 / 端口 / 上下文 / 槽位 / build ----
    let head_model = if snap.model.is_empty() {
        "—".to_string()
    } else {
        snap.model.clone()
    };
    let head = format!(
        "{}  {}    {}: {}    {}: {}    {}: {}",
        tr("monitor.model"),
        head_model,
        tr("monitor.engine"),
        if snap.engine.is_empty() { "—" } else { &snap.engine },
        tr("monitor.port"),
        if snap.port > 0 { snap.port.to_string() } else { "—".to_string() },
        tr("monitor.ctx"),
        if snap.ctx > 0 { snap.ctx.to_string() } else { "—".to_string() },
    );
    draw_text(hdc, left, y, &head, TEXT, font_bold);
    y += 22;
    let sub = format!(
        "{}: {}    {}: {}    {}: {}",
        tr("monitor.slots"),
        if snap.slots > 0 { snap.slots.to_string() } else { "—".to_string() },
        tr("monitor.uptime"),
        fmt_uptime(snap.uptime),
        tr("monitor.build"),
        if snap.build.is_empty() { "—" } else { &snap.build },
    );
    draw_text(hdc, left, y, &sub, TEXT_DIM, font);
    y += 24;
    hline(hdc, left, y, right);
    y += 10;

    // ---- 三框同一行 ----
    let gap = 12;
    let total_w = right - left;
    let box_w = (total_w - gap * 2) / 3;
    let box_h = 132;

    // GPU
    let gpu_rc = RECT { left, top: y, right: left + box_w, bottom: y + box_h };
    panel_frame(hdc, &gpu_rc);
    let mut ty = y + 8;
    draw_text(hdc, left + 10, ty, tr("monitor.gpu"), TEXT, font_bold);
    ty += 26;
    if snap.gpu.ok {
        let rows = [
            (tr("monitor.gpuUtil"), format!("{} %", snap.gpu.util)),
            (
                tr("monitor.vram"),
                format!("{:.1} / {:.1} GB", snap.gpu.mem_used as f64 / 1024.0, snap.gpu.mem_total as f64 / 1024.0),
            ),
            (
                tr("monitor.gpuTemp"),
                if snap.gpu.temp >= 0 { format!("{} °C", snap.gpu.temp) } else { "—".to_string() },
            ),
            (
                tr("monitor.gpuPower"),
                if snap.gpu.power_w > 0 { format!("{} W", snap.gpu.power_w) } else { "—".to_string() },
            ),
        ];
        for (k, v) in rows {
            draw_text(hdc, left + 10, ty, &k, TEXT_DIM, font);
            let vw = text_width(hdc, &v, font);
            draw_text(hdc, left + box_w - 10 - vw, ty, &v, TEXT, font);
            ty += 22;
        }
    } else {
        draw_text(hdc, left + 10, ty, tr("monitor.nvmlNA"), TEXT_DIM, font);
    }

    // 实时 · 输入
    let p_rc = RECT {
        left: left + box_w + gap,
        top: y,
        right: left + box_w * 2 + gap,
        bottom: y + box_h,
    };
    panel_frame(hdc, &p_rc);
    let px = p_rc.left + 10;
    let mut ty = y + 8;
    draw_text(hdc, px, ty, tr("monitor.rtPrompt"), TEXT, font_bold);
    ty += 26;
    let phase_txt = match snap.rt.phase {
        crate::monitor::Phase::Prompt => tr("monitor.phasePrompt"),
        crate::monitor::Phase::Decode => tr("monitor.phaseDecode"),
        crate::monitor::Phase::Idle => tr("monitor.phaseIdle"),
    };
    let cache_pct = if snap.rt.prompt_tokens > 0 {
        snap.rt.cache_hit as f64 * 100.0 / snap.rt.prompt_tokens as f64
    } else {
        0.0
    };
    let prows = [
        (tr("monitor.phase"), phase_txt),
        (tr("monitor.promptTokens"), fmt_u64(snap.rt.prompt_tokens)),
        (tr("monitor.promptTps"), fmt_tps(snap.rt.prompt_tps)),
        (
            tr("monitor.cacheHit"),
            format!("{} ({:.0}%)", fmt_u64(snap.rt.cache_hit), cache_pct),
        ),
    ];
    for (k, v) in prows {
        draw_text(hdc, px, ty, &k, TEXT_DIM, font);
        let vw = text_width(hdc, &v, font);
        draw_text(hdc, p_rc.right - 10 - vw, ty, &v, TEXT, font);
        ty += 22;
    }

    // 实时 · 生成
    let d_rc = RECT {
        left: left + box_w * 2 + gap * 2,
        top: y,
        right,
        bottom: y + box_h,
    };
    panel_frame(hdc, &d_rc);
    let dx = d_rc.left + 10;
    let mut ty = y + 8;
    draw_text(hdc, dx, ty, tr("monitor.rtDecode"), TEXT, font_bold);
    ty += 26;
    let dfields = [
        (tr("monitor.genTokens"), fmt_u64(snap.rt.gen_tokens)),
        (tr("monitor.genTps"), fmt_tps(snap.rt.gen_tps)),
        (
            tr("monitor.remaining"),
            if snap.rt.remaining >= 0 { snap.rt.remaining.to_string() } else { "—".to_string() },
        ),
        (tr("monitor.ctxPeak"), fmt_u64(snap.rt.ctx_peak)),
    ];
    for (k, v) in dfields {
        draw_text(hdc, dx, ty, &k, TEXT_DIM, font);
        let vw = text_width(hdc, &v, font);
        draw_text(hdc, d_rc.right - 10 - vw, ty, &v, TEXT, font);
        ty += 22;
    }

    y += box_h + 12;

    // ---- 请求历史 ----
    let hist_bottom = rect.bottom - 12;
    let hist_rc = RECT { left, top: y, right, bottom: hist_bottom };
    if hist_rc.bottom > hist_rc.top + 40 {
        panel_frame(hdc, &hist_rc);
        let hx = hist_rc.left + 10;
        let mut hy = hist_rc.top + 8;
        draw_text(hdc, hx, hy, tr("monitor.history"), TEXT, font_bold);
        hy += 24;

        // 表头：列定义（左对齐文字列 + 右对齐数值列）
        let cols = [
            tr("monitor.colIndex"),
            tr("monitor.colTime"),
            tr("monitor.colPrompt"),
            tr("monitor.colGen"),
            tr("monitor.colPrefill"),
            tr("monitor.colDecode"),
            tr("monitor.colTotal"),
            tr("monitor.colMtp"),
        ];
        let widths = [40, 80, 80, 80, 110, 100, 90, 70];
        // 表头底色
        let head_rc = RECT {
            left: hist_rc.left + 1,
            top: hy - 2,
            right: hist_rc.right - 1,
            bottom: hy + 20,
        };
        rgb_fill(hdc, &head_rc, LOG_BG);
        let mut cx = hx;
        for (i, c) in cols.iter().enumerate() {
            draw_text(hdc, cx, hy, c, TEXT_DIM, font);
            cx += widths[i];
        }
        hy += 24;
        hline(hdc, hist_rc.left + 1, hy - 4, hist_rc.right - 1);

        if snap.hist.is_empty() {
            draw_text(hdc, hx, hy, tr("monitor.historyEmpty"), TEXT_DIM, font);
        } else {
            let max_rows = ((hist_rc.bottom - hy - 8) / 20).max(0) as usize;
            let start = snap.hist.len().saturating_sub(max_rows);
            for (n, row) in snap.hist[start..].iter().enumerate() {
                let mut cx = hx;
                let idx = (start + n + 1).to_string();
                let cells = [
                    idx,
                    if row.time.is_empty() { "—".to_string() } else { row.time.clone() },
                    row.prompt.to_string(),
                    row.gen.to_string(),
                    fmt1(row.prefill_tps),
                    fmt1(row.decode_tps),
                    format!("{:.1}s", row.total_s),
                    match row.mtp {
                        Some(v) => format!("{:.0}%", v * 100.0),
                        None => "—".to_string(),
                    },
                ];
                for (i, cell) in cells.iter().enumerate() {
                    draw_text(hdc, cx, hy, cell, TEXT, font);
                    cx += widths[i];
                }
                hy += 20;
            }
        }
    }
}

/// 速度：没有数据就是「—」，不要出现「—/s」
fn fmt_tps(v: f64) -> String {
    if v <= 0.0 {
        "—".to_string()
    } else {
        format!("{:.1}/s", v)
    }
}

fn fmt_u64(v: u64) -> String {
    if v == 0 {
        "0".to_string()
    } else {
        v.to_string()
    }
}

fn fmt_uptime(secs: u64) -> String {
    if secs == 0 {
        "—".to_string()
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
