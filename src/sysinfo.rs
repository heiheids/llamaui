//! 系统状态采样：CPU 占用率 / 内存占用 / 显卡温度与显存
//! System status sampling: CPU load, memory usage, GPU temperature and VRAM.
//!
//! 采样在后台线程完成，界面只读取快照，避免主线程被 nvidia-smi 之类的外部调用阻塞。
//! Sampling happens on a worker thread; the UI only reads a snapshot.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub cpu_percent: u32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    /// 低于 0 表示无法读取
    pub gpu_temp: i32,
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub gpu_known: bool,
}

static SNAPSHOT: OnceLock<Mutex<Sample>> = OnceLock::new();
static STARTED: AtomicBool = AtomicBool::new(false);

fn cell() -> &'static Mutex<Sample> {
    SNAPSHOT.get_or_init(|| Mutex::new(Sample { gpu_temp: -1, ..Default::default() }))
}

/// 启动后台采样线程（重复调用无副作用）
pub fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let mut prev = unsafe { system_times() };
        let mut tick: u64 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let now = unsafe { system_times() };
            let cpu = cpu_percent(prev, now);
            prev = now;

            let (ram_used, ram_total) = unsafe { memory_mb() };

            // 显卡数据变化慢，每 3 秒查询一次即可
            let gpu = if tick % 3 == 0 { gpu_query() } else { None };

            if let Ok(mut s) = cell().lock() {
                s.cpu_percent = cpu;
                s.ram_used_mb = ram_used;
                s.ram_total_mb = ram_total;
                if let Some(g) = gpu {
                    s.gpu_temp = g.0;
                    s.vram_used_mb = g.1;
                    s.vram_total_mb = g.2;
                    s.gpu_known = g.0 >= 0 || g.2 > 0;
                }
            }
            tick = tick.wrapping_add(1);
        }
    });
}

/// 读取最近一次采样
pub fn sample() -> Sample {
    cell().lock().map(|s| *s).unwrap_or_default()
}

fn cpu_percent(a: Times, b: Times) -> u32 {
    let idle = b.idle.saturating_sub(a.idle);
    let kernel = b.kernel.saturating_sub(a.kernel);
    let user = b.user.saturating_sub(a.user);
    let total = kernel.saturating_add(user);
    if total == 0 {
        return 0;
    }
    let busy = total.saturating_sub(idle);
    ((busy as f64 / total as f64) * 100.0).round().clamp(0.0, 100.0) as u32
}

#[derive(Clone, Copy, Default)]
struct Times {
    idle: u64,
    kernel: u64,
    user: u64,
}

#[cfg(windows)]
#[repr(C)]
struct FileTime {
    low: u32,
    high: u32,
}

#[cfg(windows)]
impl FileTime {
    fn value(&self) -> u64 {
        ((self.high as u64) << 32) | self.low as u64
    }
}

#[cfg(windows)]
extern "system" {
    fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    fn GlobalMemoryStatusEx(buf: *mut MemoryStatusEx) -> i32;
}

#[cfg(windows)]
#[repr(C)]
struct MemoryStatusEx {
    length: u32,
    memory_load: u32,
    total_phys: u64,
    avail_phys: u64,
    total_page_file: u64,
    avail_page_file: u64,
    total_virtual: u64,
    avail_virtual: u64,
    avail_extended_virtual: u64,
}

#[cfg(windows)]
unsafe fn system_times() -> Times {
    let mut idle = FileTime { low: 0, high: 0 };
    let mut kernel = FileTime { low: 0, high: 0 };
    let mut user = FileTime { low: 0, high: 0 };
    if GetSystemTimes(&mut idle, &mut kernel, &mut user) == 0 {
        return Times::default();
    }
    Times {
        idle: idle.value(),
        kernel: kernel.value(),
        user: user.value(),
    }
}

#[cfg(not(windows))]
unsafe fn system_times() -> Times {
    Times::default()
}

#[cfg(windows)]
unsafe fn memory_mb() -> (u64, u64) {
    let mut m = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page_file: 0,
        avail_page_file: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended_virtual: 0,
    };
    if GlobalMemoryStatusEx(&mut m) == 0 {
        return (0, 0);
    }
    let total = m.total_phys / 1_048_576;
    let used = total.saturating_sub(m.avail_phys / 1_048_576);
    (used, total)
}

#[cfg(not(windows))]
unsafe fn memory_mb() -> (u64, u64) {
    (0, 0)
}

/// 查询显卡温度 / 显存占用（MB）：优先 nvidia-smi
fn gpu_query() -> Option<(i32, u64, u64)> {
    #[cfg(windows)]
    const NO_WINDOW: u32 = 0x0800_0000;

    let mut cmd = std::process::Command::new("nvidia-smi");
    cmd.args([
        "--query-gpu=temperature.gpu,memory.used,memory.total",
        "--format=csv,noheader,nounits",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?.trim();
    let mut parts = line.split(',').map(|p| p.trim());
    let temp = parts.next().and_then(|v| v.parse::<i32>().ok()).unwrap_or(-1);
    let used = parts.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let total = parts.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    Some((temp, used, total))
}
