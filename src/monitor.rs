//! 模型运行数据监控（移植自 LlamaDashboard 的监控面板）
//! Model runtime monitoring, ported from LlamaDashboard.
//!
//! 数据来源全部在本机：
//!   - `GET /slots`  实时槽位（llama-server 默认开启）
//!   - `GET /props`  静态信息（build_info / 总槽位 / 上下文）
//!   - 日志文件      每次请求的结算块（精确毫秒、MTP 接受率）
//!   - NVML          显卡利用率、显存、温度、功耗
//!
//! 采集在后台线程完成，界面只读取快照，避免阻塞主线程。

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// 对外数据结构
// ---------------------------------------------------------------------------

/// 当前阶段
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Prompt,
    Decode,
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Idle
    }
}

/// 显卡采样（NVML 不可用时 ok = false）
#[derive(Clone, Default)]
pub struct Gpu {
    pub ok: bool,
    pub util: u32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub temp: i32,
    pub power_w: u32,
}

/// 实时面板数据
#[derive(Clone, Default)]
pub struct Realtime {
    pub phase: Phase,
    pub prompt_tokens: u64,
    pub prompt_tps: f64,
    pub gen_tokens: u64,
    pub gen_tps: f64,
    pub cache_hit: u64,
    pub remaining: i64,
    pub ctx_peak: u64,
}

/// 一条已完成的请求
#[derive(Clone, Default)]
pub struct HistoryRow {
    pub time: String,
    pub prompt: u64,
    pub gen: u64,
    pub prefill_tps: f64,
    pub decode_tps: f64,
    pub total_s: f64,
    /// MTP / 推测解码接受率（0..1），None = 未启用或没有统计
    pub mtp: Option<f64>,
}

/// 界面读取的完整快照
#[derive(Clone, Default)]
pub struct Snapshot {
    pub running: bool,
    pub model: String,
    pub engine: String,
    pub port: u16,
    pub ctx: u64,
    pub slots: u32,
    pub uptime: u64,
    pub build: String,
    pub gpu: Gpu,
    pub rt: Realtime,
    pub hist: Vec<HistoryRow>,
    /// /slots 是否取到了
    pub slots_ok: bool,
    /// /slots 的错误说明（空 = 正常）
    pub slots_err: String,
    pub props_ok: bool,
}

// ---------------------------------------------------------------------------
// 由界面维护的上下文（端口 / 密钥 / 是否运行 / 日志路径）
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct MonCtx {
    pub port: u16,
    pub key: String,
    pub running: bool,
    pub log_path: String,
    /// 服务未运行时 /props 取不到，用配置里的值兜底显示
    pub ctx_cfg: u64,
    pub slots_cfg: u32,
}

fn ctx_cell() -> &'static Mutex<MonCtx> {
    static C: OnceLock<Mutex<MonCtx>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(MonCtx::default()))
}

/// 主窗口每秒调用一次，把当前连接信息同步给采集线程
pub fn update_ctx(port: u16, key: &str, running: bool, log_path: &str, ctx_cfg: u64, slots_cfg: u32) {
    if let Ok(mut c) = ctx_cell().lock() {
        c.port = port;
        c.key = key.to_string();
        c.running = running;
        c.log_path = log_path.to_string();
        c.ctx_cfg = ctx_cfg;
        c.slots_cfg = slots_cfg;
    }
}

fn snap_cell() -> &'static Mutex<Snapshot> {
    static S: OnceLock<Mutex<Snapshot>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Snapshot::default()))
}

/// 界面读取最近一次快照
pub fn snapshot() -> Snapshot {
    snap_cell()
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}

static RUNNING: AtomicBool = AtomicBool::new(false);

/// 启动采集线程（重复调用无副作用）
pub fn start() {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(run_loop);
}

/// 请求采集线程退出
pub fn stop() {
    RUNNING.store(false, Ordering::SeqCst);
}

pub fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// NVML 动态绑定
// ---------------------------------------------------------------------------

#[repr(C)]
struct NvmlUtilization {
    gpu: u32,
    memory: u32,
}

#[repr(C)]
struct NvmlMemory {
    total: u64,
    free: u64,
    used: u64,
}

type NvmlDevice = *mut std::ffi::c_void;

/// 动态加载的 NVML 函数表。任何一步失败就整体不可用，
/// 界面显示「不可用」而不是报错。
struct Nvml {
    #[cfg(windows)]
    module: usize,
    device: NvmlDevice,
    get_util: unsafe extern "system" fn(NvmlDevice, *mut NvmlUtilization) -> i32,
    get_mem: unsafe extern "system" fn(NvmlDevice, *mut NvmlMemory) -> i32,
    get_temp: unsafe extern "system" fn(NvmlDevice, u32, *mut u32) -> i32,
    get_power: unsafe extern "system" fn(NvmlDevice, *mut u32) -> i32,
    shutdown: unsafe extern "system" fn() -> i32,
}

impl Nvml {
    #[cfg(windows)]
    fn load() -> Option<Nvml> {
        unsafe extern "system" {
            fn LoadLibraryW(name: *const u16) -> usize;
            fn GetProcAddress(module: usize, name: *const u8) -> *const core::ffi::c_void;
            fn FreeLibrary(module: usize) -> i32;
        }
        const NVML_SUCCESS: i32 = 0;

        let name: Vec<u16> = "nvml.dll\0".encode_utf16().collect();
        let module = unsafe { LoadLibraryW(name.as_ptr()) };
        if module == 0 {
            return None;
        }

        unsafe fn sym(module: usize, name: &str) -> Option<*const core::ffi::c_void> {
            unsafe extern "system" {
                fn GetProcAddress(module: usize, name: *const u8) -> *const core::ffi::c_void;
            }
            let mut buf = Vec::with_capacity(name.len() + 1);
            buf.extend_from_slice(name.as_bytes());
            buf.push(0);
            let p = unsafe { GetProcAddress(module, buf.as_ptr()) };
            if p.is_null() {
                None
            } else {
                Some(p)
            }
        }

        // 必需符号：初始化、取卡、取指标
        let init = unsafe { sym(module, "nvmlInit_v2")? };
        let count = unsafe { sym(module, "nvmlDeviceGetCount_v2")? };
        let handle_by_index = unsafe { sym(module, "nvmlDeviceGetHandleByIndex_v2")? };
        let get_util = unsafe { sym(module, "nvmlDeviceGetUtilizationRates")? };
        let get_mem = unsafe { sym(module, "nvmlDeviceGetMemoryInfo")? };
        let get_temp = unsafe { sym(module, "nvmlDeviceGetTemperature")? };
        let get_power = unsafe { sym(module, "nvmlDeviceGetPowerUsage")? };
        let shutdown = unsafe { sym(module, "nvmlShutdown")? };

        let init: unsafe extern "system" fn() -> i32 = unsafe { std::mem::transmute(init) };
        let count: unsafe extern "system" fn(*mut u32) -> i32 = unsafe { std::mem::transmute(count) };
        let handle_by_index: unsafe extern "system" fn(u32, *mut NvmlDevice) -> i32 =
            unsafe { std::mem::transmute(handle_by_index) };
        let get_util: unsafe extern "system" fn(NvmlDevice, *mut NvmlUtilization) -> i32 =
            unsafe { std::mem::transmute(get_util) };
        let get_mem: unsafe extern "system" fn(NvmlDevice, *mut NvmlMemory) -> i32 =
            unsafe { std::mem::transmute(get_mem) };
        let get_temp: unsafe extern "system" fn(NvmlDevice, u32, *mut u32) -> i32 =
            unsafe { std::mem::transmute(get_temp) };
        let get_power: unsafe extern "system" fn(NvmlDevice, *mut u32) -> i32 =
            unsafe { std::mem::transmute(get_power) };
        let shutdown: unsafe extern "system" fn() -> i32 = unsafe { std::mem::transmute(shutdown) };

        if unsafe { init() } != NVML_SUCCESS {
            unsafe { FreeLibrary(module) };
            return None;
        }
        let mut n: u32 = 0;
        if unsafe { count(&mut n) } != NVML_SUCCESS || n == 0 {
            unsafe {
                shutdown();
                FreeLibrary(module);
            }
            return None;
        }
        let mut device: NvmlDevice = std::ptr::null_mut();
        if unsafe { handle_by_index(0, &mut device) } != NVML_SUCCESS {
            unsafe {
                shutdown();
                FreeLibrary(module);
            }
            return None;
        }

        Some(Nvml {
            module,
            device,
            get_util,
            get_mem,
            get_temp,
            get_power,
            shutdown,
        })
    }

    #[cfg(not(windows))]
    fn load() -> Option<Nvml> {
        None
    }

    fn sample(&self) -> Gpu {
        const NVML_TEMPERATURE_GPU: u32 = 0;
        let mut g = Gpu {
            ok: true,
            temp: -1,
            ..Default::default()
        };
        let mut u = NvmlUtilization { gpu: 0, memory: 0 };
        if unsafe { (self.get_util)(self.device, &mut u) } == 0 {
            g.util = u.gpu;
        }
        let mut m = NvmlMemory {
            total: 0,
            free: 0,
            used: 0,
        };
        if unsafe { (self.get_mem)(self.device, &mut m) } == 0 {
            g.mem_total = m.total / 1_048_576;
            g.mem_used = m.used / 1_048_576;
        }
        let mut t: u32 = 0;
        if unsafe { (self.get_temp)(self.device, NVML_TEMPERATURE_GPU, &mut t) } == 0 {
            g.temp = t as i32;
        }
        let mut p: u32 = 0;
        if unsafe { (self.get_power)(self.device, &mut p) } == 0 {
            g.power_w = p / 1000;
        }
        g
    }
}

impl Drop for Nvml {
    fn drop(&mut self) {
        unsafe { (self.shutdown)() };
        #[cfg(windows)]
        unsafe {
            unsafe extern "system" {
                fn FreeLibrary(module: usize) -> i32;
            }
            FreeLibrary(self.module);
        }
    }
}

// ---------------------------------------------------------------------------
// 日志结算块解析
// ---------------------------------------------------------------------------

/// 同一个 task 累积中的一条请求
#[derive(Default)]
struct Pending {
    prompt: u64,
    prompt_ms: f64,
    gen: u64,
    gen_ms: f64,
    mtp: Option<f64>,
}

/// 从日志行里取 "…… = 123.45 ms / 67 tokens" 的毫秒与 token 数
fn parse_time_tokens(line: &str) -> Option<(f64, u64)> {
    let eq = line.find('=')?;
    let rest = &line[eq + 1..];
    let ms_pos = rest.find(" ms")?;
    let ms: f64 = rest[..ms_pos].trim().parse().ok()?;
    let after = &rest[ms_pos..];
    let slash = after.find('/')?;
    let tail = &after[slash + 1..];
    let tok_pos = tail.find("tokens")?;
    let digits: String = tail[..tok_pos]
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    let n: u64 = digits.parse().ok()?;
    Some((ms, n))
}

/// `draft acceptance = 0.92857 (39 accepted / 42 generated)`
fn parse_draft_acceptance(line: &str) -> Option<f64> {
    let key = "draft acceptance =";
    let pos = line.find(key)?;
    let rest = &line[pos + key.len()..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '(')
        .unwrap_or(rest.len());
    rest[..end].trim().parse::<f64>().ok()
}

/// 从 `task 123` 里取 task id
fn parse_task_id(line: &str) -> Option<u64> {
    let pos = line.find("| task ")?;
    let rest = &line[pos + "| task ".len()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// 解析一行日志，维护 task 累积表；遇到 total time 就结算一条历史
fn feed_line(line: &str, pending: &mut BTreeMap<u64, Pending>, out: &mut Vec<HistoryRow>, time: &str) {
    let Some(task) = parse_task_id(line) else {
        return;
    };
    if line.contains("prompt eval time") {
        if let Some((ms, n)) = parse_time_tokens(line) {
            let e = pending.entry(task).or_default();
            e.prompt = n;
            e.prompt_ms = ms;
        }
    } else if line.contains("eval time =") {
        if let Some((ms, n)) = parse_time_tokens(line) {
            let e = pending.entry(task).or_default();
            e.gen = n;
            e.gen_ms = ms;
        }
    } else if let Some(acc) = parse_draft_acceptance(line) {
        let e = pending.entry(task).or_default();
        e.mtp = Some(acc);
    } else if line.contains("total time =") {
        if let Some(e) = pending.remove(&task) {
            let total_ms = parse_time_tokens(line).map(|(ms, _)| ms).unwrap_or(e.prompt_ms + e.gen_ms);
            if e.prompt > 0 || e.gen > 0 {
                out.push(HistoryRow {
                    time: time.to_string(),
                    prompt: e.prompt,
                    gen: e.gen,
                    prefill_tps: if e.prompt_ms > 0.0 {
                        e.prompt as f64 / (e.prompt_ms / 1000.0)
                    } else {
                        0.0
                    },
                    decode_tps: if e.gen_ms > 0.0 {
                        e.gen as f64 / (e.gen_ms / 1000.0)
                    } else {
                        0.0
                    },
                    total_s: total_ms / 1000.0,
                    mtp: e.mtp,
                });
            }
        }
    }
}

/// 增量读取日志文件，返回新解析出的历史条目
fn read_new_log_lines(path: &str, offset: &mut u64, pending: &mut BTreeMap<u64, Pending>) -> Vec<HistoryRow> {
    use std::io::{Read, Seek, SeekFrom};
    let mut out = Vec::new();
    if path.is_empty() {
        return out;
    }
    // 服务状态里存的是日志「文件名」（server-<stamp>.log），实际文件在 logs/ 下；
    // 绝对路径则原样使用。
    let full = {
        let p = std::path::Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            crate::util::logs_dir().join(p)
        }
    };
    let Ok(mut f) = std::fs::File::open(&full) else {
        return out;
    };
    let Ok(meta) = f.metadata() else { return out };
    let len = meta.len();
    // 文件被重建（每次启动新建日志）：从头再来
    if len < *offset {
        *offset = 0;
        pending.clear();
    }
    if len == *offset {
        return out;
    }
    if f.seek(SeekFrom::Start(*offset)).is_err() {
        return out;
    }
    // 只读新增部分，且单次最多读 256 KB，避免一次性吃太多
    let want = (len - *offset).min(256 * 1024) as usize;
    let mut buf = vec![0u8; want];
    let Ok(n) = f.read(&mut buf) else { return out };
    if n == 0 {
        return out;
    }
    let text = String::from_utf8_lossy(&buf[..n]).to_string();
    // 最后一行可能不完整，下次从它重新开始
    let consumed = match text.rfind('\n') {
        Some(p) => p + 1,
        None => 0,
    };
    if consumed == 0 {
        return out;
    }
    *offset += consumed as u64;
    for line in text[..consumed].lines() {
        let time = elapsed_stamp(line);
        feed_line(line, pending, &mut out, &time);
    }
    out
}

/// 从 llama-server 的相对时间前缀取「分:秒」，例如 `0.27.184.061` → `00:27`
/// （格式是 时.分.秒.毫秒，从服务启动算起）。
fn elapsed_stamp(line: &str) -> String {
    for tok in line.split_whitespace() {
        let parts: Vec<&str> = tok.split('.').collect();
        if parts.len() != 4 {
            continue;
        }
        if !parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        {
            continue;
        }
        let h: u64 = parts[0].parse().unwrap_or(0);
        let m: u64 = parts[1].parse().unwrap_or(0);
        let s: u64 = parts[2].parse().unwrap_or(0);
        let total = h * 3600 + m * 60 + s;
        return format!("{:02}:{:02}", total / 60, total % 60);
    }
    String::new()
}

// ---------------------------------------------------------------------------
// /slots 与 /props 解析
// ---------------------------------------------------------------------------

struct SlotAgg {
    phase: Phase,
    /// 当前任务 id（变化说明换了新请求，增量要重新起算）
    task: u64,
    /// 输入 token（处理中才有值；空闲时 None = 保持上次显示）
    prompt: Option<u64>,
    /// 已生成 token（处理中才有值）
    gen: Option<u64>,
    cache: u64,
    remaining: i64,
    ctx_used: u64,
    n_ctx: u64,
    total_slots: u32,
}

fn fetch_slots(port: u16, key: &str) -> Result<SlotAgg, String> {
    let resp = crate::web::client::request(
        "127.0.0.1",
        port,
        "GET",
        "/slots",
        if key.is_empty() { None } else { Some(key) },
        None,
    )?;
    if resp.code == 401 || resp.code == 403 {
        return Err("key".to_string());
    }
    if resp.code == 404 {
        return Err("off".to_string());
    }
    if resp.code != 200 {
        return Err(format!("http {}", resp.code));
    }
    let json = crate::json::parse(&resp.body).map_err(|_| "parse".to_string())?;
    let arr = json.as_arr().ok_or("shape".to_string())?;
    let mut agg = SlotAgg {
        phase: Phase::Idle,
        task: 0,
        prompt: None,
        gen: None,
        cache: 0,
        remaining: -1,
        ctx_used: 0,
        n_ctx: 0,
        total_slots: arr.len() as u32,
    };
    for s in arr.iter() {
        let processing = s.b("is_processing", false);
        // llama-server 把「已生成 / 剩余」放在 next_token[0] 里，不在顶层；
        // 部分 build 也会给顶层同名字段，两个都试。
        let nt = s
            .path("next_token")
            .and_then(|v| v.as_arr())
            .and_then(|a| a.first());
        let decoded = nt
            .map(|t| t.i("n_decoded", 0))
            .unwrap_or_else(|| s.i("n_decoded", 0))
            .max(0) as u64;
        let remain = nt
            .map(|t| t.i("n_remain", -1))
            .unwrap_or_else(|| s.i("n_remaining", -1));
        // n_prompt_tokens 是「上下文已用」，llama-server 会随生成增长，
        // 减去已生成才是本次输入长度（kvmem 版不增长，减完仍然正确）
        let ctx_tokens = s.i("n_prompt_tokens", 0).max(0) as u64;
        let prompt = ctx_tokens.saturating_sub(decoded);

        if processing && decoded > 0 {
            agg.phase = Phase::Decode;
        } else if processing && agg.phase == Phase::Idle {
            agg.phase = Phase::Prompt;
        }
        if processing {
            agg.gen = Some(agg.gen.unwrap_or(0) + decoded);
            agg.prompt = Some(agg.prompt.unwrap_or(0) + prompt);
        }
        let task = s.i("id_task", 0).max(0) as u64;
        if task > agg.task {
            agg.task = task;
        }
        agg.cache += s.i("n_prompt_tokens_cache", 0).max(0) as u64;
        if remain >= 0 {
            agg.remaining = if agg.remaining < 0 {
                remain
            } else {
                agg.remaining.min(remain)
            };
        }
        let ctx_used = s.i("n_ctx_used", 0).max(0) as u64;
        let used = if ctx_used > 0 { ctx_used } else { ctx_tokens };
        agg.ctx_used = agg.ctx_used.max(used);
        let n_ctx = s.i("n_ctx", 0).max(0) as u64;
        if n_ctx > agg.n_ctx {
            agg.n_ctx = n_ctx;
        }
    }
    Ok(agg)
}

fn fetch_props(port: u16, key: &str) -> Option<(String, u32, u64)> {
    let resp = crate::web::client::request(
        "127.0.0.1",
        port,
        "GET",
        "/props",
        if key.is_empty() { None } else { Some(key) },
        None,
    )
    .ok()?;
    if resp.code != 200 {
        return None;
    }
    let json = crate::json::parse(&resp.body).ok()?;
    let build = json.s("build_info", "");
    let slots = json.i("total_slots", 0).max(0) as u32;
    let ctx = json
        .path("default_generation_settings")
        .map(|d| d.i("n_ctx", 0))
        .unwrap_or(0)
        .max(0) as u64;
    Some((build, slots, ctx))
}

// ---------------------------------------------------------------------------
// 采集主循环
// ---------------------------------------------------------------------------

fn run_loop() {
    let nvml = Nvml::load();
    let mut prev_gen: u64 = 0;
    let mut prev_prompt: u64 = 0;
    let mut last_task: u64 = 0;
    // 请求结束后 /slots 不再返回 token 数，用这些记住最后一次的值
    let mut last_prompt: u64 = 0;
    let mut last_gen: u64 = 0;
    let mut last_remain: i64 = -1;
    let mut last_ctx_peak: u64 = 0;
    let mut gen_tps: f64 = 0.0;
    let mut prompt_tps: f64 = 0.0;
    let mut last = Instant::now();
    let mut log_offset: u64 = 0;
    let mut pending: BTreeMap<u64, Pending> = BTreeMap::new();
    let mut hist: Vec<HistoryRow> = Vec::new();
    let mut build = String::new();
    let mut props_slots: u32 = 0;
    let mut props_ctx: u64 = 0;
    let mut tick: u64 = 0;

    while RUNNING.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(1000));
        tick = tick.wrapping_add(1);

        let (port, key, running, log_path, ctx_cfg, slots_cfg) = match ctx_cell().lock() {
            Ok(c) => (
                c.port,
                c.key.clone(),
                c.running,
                c.log_path.clone(),
                c.ctx_cfg,
                c.slots_cfg,
            ),
            Err(_) => continue,
        };

        let mut snap = Snapshot {
            running,
            port,
            ..Default::default()
        };
        snap.gpu = match &nvml {
            Some(n) => n.sample(),
            None => Gpu::default(),
        };

        // ---- 实时槽位 ----
        let mut slots_ok = false;
        if running && port > 0 {
            match fetch_slots(port, &key) {
                Ok(agg) => {
                    slots_ok = true;
                    let dt = last.elapsed().as_secs_f64().max(0.001);
                    // 换了请求（task 变化）就把增量基准归零，避免算出负数或跳变
                    if agg.task != last_task {
                        last_task = agg.task;
                        prev_gen = 0;
                        prev_prompt = 0;
                    }
                    // 输出速度：只在生成阶段、计数前进时更新；否则保持上次值（冻结）
                    if let Some(gen) = agg.gen {
                        if agg.phase == Phase::Decode && gen >= prev_gen {
                            let d = (gen - prev_gen) as f64;
                            if d > 0.0 {
                                gen_tps = d / dt;
                            }
                        }
                        prev_gen = gen;
                    }
                    if let Some(p) = agg.prompt {
                        if agg.phase == Phase::Prompt && p >= prev_prompt {
                            let d = (p - prev_prompt) as f64;
                            if d > 0.0 {
                                prompt_tps = d / dt;
                            }
                        }
                        prev_prompt = p;
                        // 请求结束后 /slots 不再给 token 数，保留最后一次的值
                        last_prompt = p;
                    }
                    if let Some(g) = agg.gen {
                        last_gen = g;
                    }
                    if agg.remaining >= 0 {
                        last_remain = agg.remaining;
                    }
                    if agg.ctx_used > 0 {
                        last_ctx_peak = agg.ctx_used;
                    }
                    snap.rt = Realtime {
                        phase: agg.phase,
                        prompt_tokens: last_prompt,
                        // 速度不做阶段清零：请求结束后保留最后一次有效值（冻结），
                        // 否则一闪而过的请求在面板上永远看不到速度
                        prompt_tps,
                        gen_tokens: last_gen,
                        gen_tps,
                        cache_hit: agg.cache,
                        remaining: last_remain,
                        ctx_peak: last_ctx_peak,
                    };
                    if agg.n_ctx > 0 {
                        props_ctx = agg.n_ctx;
                    }
                    if agg.total_slots > 0 {
                        props_slots = agg.total_slots;
                    }
                }
                Err(e) => snap.slots_err = e,
            }
            // /props 变化慢，5 秒一次
            if tick % 5 == 0 {
                if let Some((b, s, c)) = fetch_props(port, &key) {
                    build = b;
                    if s > 0 {
                        props_slots = s;
                    }
                    if c > 0 {
                        props_ctx = c;
                    }
                }
            }
        } else {
            prev_gen = 0;
            prev_prompt = 0;
            last_task = 0;
            last_prompt = 0;
            last_gen = 0;
            last_remain = -1;
            last_ctx_peak = 0;
            gen_tps = 0.0;
            prompt_tps = 0.0;
        }
        snap.slots_ok = slots_ok;
        snap.props_ok = !build.is_empty();
        snap.build = build.clone();
        // /props 拿不到时（服务没在跑）用配置值，避免整行显示「—」
        snap.slots = if props_slots > 0 { props_slots } else { slots_cfg };
        snap.ctx = if props_ctx > 0 { props_ctx } else { ctx_cfg };
        last = Instant::now();

        // ---- 日志结算块 ----
        let new_rows = read_new_log_lines(&log_path, &mut log_offset, &mut pending);
        if !new_rows.is_empty() {
            hist.extend(new_rows);
            // 只保留最近 50 条
            if hist.len() > 50 {
                let cut = hist.len() - 50;
                hist.drain(0..cut);
            }
        }
        snap.hist = hist.clone();

        if let Ok(mut s) = snap_cell().lock() {
            // 运行态与模型名由界面补充
            snap.model = s.model.clone();
            snap.engine = s.engine.clone();
            snap.uptime = s.uptime;
            *s = snap;
        }
    }

    // 退出前把快照清空，下次打开窗口重新开始
    if let Ok(mut s) = snap_cell().lock() {
        *s = Snapshot::default();
    }
}

/// 界面每秒同步一次模型名 / 引擎 / 运行时长（这几项只有界面知道）
pub fn update_labels(model: &str, engine: &str, uptime: u64) {
    if let Ok(mut s) = snap_cell().lock() {
        s.model = model.to_string();
        s.engine = engine.to_string();
        s.uptime = uptime;
    }
}

/// 供界面直接写入运行态（服务停止后立刻反映，不等下一 tick）
pub fn set_running(running: bool) {
    if let Ok(mut s) = snap_cell().lock() {
        s.running = running;
        if !running {
            s.rt = Realtime::default();
            s.slots_ok = false;
            s.slots_err.clear();
        }
    }
}

fn _unused(_: Arc<Mutex<()>>) {}

/// 一次性读取整卡已用显存（MB）。校准只需在启动前后各调用一次，
/// 因此这里现加载现释放，不占用长期句柄。
pub fn gpu_mem_used_mb() -> Option<u64> {
    let n = Nvml::load()?;
    let g = n.sample();
    if !g.ok || g.mem_total == 0 {
        return None;
    }
    Some(g.mem_used)
}