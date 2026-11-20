//! 退出清理：Ctrl+C / 关闭控制台时停止 llama-server，避免孤儿进程
//! Exit cleanup: stop llama-server on Ctrl+C or console close to avoid orphans.

use crate::web::api::Shared;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
static APP: OnceLock<Shared> = OnceLock::new();

#[cfg(windows)]
mod win {
    use super::STOP_REQUESTED;
    use std::sync::atomic::Ordering;

    pub type HandlerRoutine = unsafe extern "system" fn(u32) -> i32;

    #[link(name = "kernel32")]
    extern "system" {
        pub fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;

    }

    pub unsafe extern "system" fn on_ctrl(_event: u32) -> i32 {
        STOP_REQUESTED.store(true, Ordering::SeqCst);
        1 // 已处理，交由后台线程完成清理
    }
}

/// 注册退出钩子；调用前必须已完成 APP 初始化
pub fn install(app: Shared) {
    let _ = APP.set(app);
    #[cfg(windows)]
    unsafe {
        win::SetConsoleCtrlHandler(Some(win::on_ctrl), 1);
    }

    std::thread::spawn(move || loop {
        if STOP_REQUESTED.load(Ordering::SeqCst) {
            if let Some(shared) = APP.get() {
                if let Ok(mut a) = shared.lock() {
                    println!("[llamaUI] shutting down, stopping llama-server...");
                    a.launcher.shutdown();
                    let _ = a.store.save();
                }
            }
            std::process::exit(0);
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    });
}
