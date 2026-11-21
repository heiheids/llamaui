//! llama-server 进程生命周期管理（启动 / 停止 / 日志 / 健康探测）
//! llama-server process lifecycle: launch, stop, log capture, health probing.

use crate::json::Json;
use crate::settings;
use crate::util;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const LOG_RING: usize = 1200;

#[derive(Clone, PartialEq)]
pub enum Phase {
    Stopped,
    Starting,
    Running,
    Error,
}

pub struct ServerState {
    pub phase: Phase,
    pub pid: Option<u32>,
    pub started_at: u64,
    pub error: String,
    pub args: Vec<String>,
    pub log_path: String,
    pub exit_code: Option<i32>,
    /// 本次启动以来累计的 token 用量（从 llama-server 日志里累加）
    pub tokens_prompt: u64,
    pub tokens_gen: u64,
}

impl ServerState {
    fn new() -> ServerState {
        ServerState {
            phase: Phase::Stopped,
            pid: None,
            started_at: 0,
            error: String::new(),
            args: Vec::new(),
            log_path: String::new(),
            exit_code: None,
            tokens_prompt: 0,
            tokens_gen: 0,
        }
    }
}

pub struct Launcher {
    pub state: Arc<Mutex<ServerState>>,
    pub logs: Arc<Mutex<VecDeque<String>>>,
    child: Option<Child>,
    pub port: u16,
    pub host: String,
}

impl Launcher {
    pub fn new() -> Launcher {
        Launcher {
            state: Arc::new(Mutex::new(ServerState::new())),
            logs: Arc::new(Mutex::new(VecDeque::new())),
            child: None,
            port: 0,
            host: String::new(),
        }
    }

    /// 累加一次请求的 token 用量（由日志解析得到）
    pub fn add_tokens(&self, prompt: u64, gen: u64) {
        if let Ok(mut st) = self.state.lock() {
            st.tokens_prompt = st.tokens_prompt.saturating_add(prompt);
            st.tokens_gen = st.tokens_gen.saturating_add(gen);
        }
    }

    pub fn log_count(&self) -> u64 {
        self.logs.lock().map(|l| l.len() as u64).unwrap_or(0)
    }

    /// 追加一行日志到内存环 + 文件
    pub fn push_log(&self, line: &str, log_path: &str) {
        let stamp = format_timestamp();
        let entry = format!("[{}] {}", stamp, line);
        if let Ok(mut ring) = self.logs.lock() {
            ring.push_back(entry.clone());
            while ring.len() > LOG_RING {
                ring.pop_front();
            }
        }
        if !log_path.is_empty() {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
            {
                let _ = writeln!(f, "{}", entry);
            }
        }
    }

    pub fn snapshot_logs(&self, since: usize) -> Vec<String> {
        self.logs
            .lock()
            .map(|l| l.iter().skip(since).cloned().collect())
            .unwrap_or_default()
    }

    pub fn status_json(&self, cfg: &Json) -> Json {
        let st = match self.state.lock() {
            Ok(s) => s.clone_state(),
            Err(_) => ("stopped".to_string(), None, 0, String::new(), None, String::new(), 0, 0),
        };
        let port = cfg.i("network.port", 9931);
        let host = settings::bind_host(cfg);
        let mut o = Json::obj();
        o.set("phase", Json::str(st.0));
        o.set("pid", st.1.map(|p| Json::Num(p as f64)).unwrap_or(Json::Null));
        o.set("startedAt", Json::Num(st.2 as f64));
        o.set("uptime", Json::Num(if st.2 == 0 { 0.0 } else { (util::now_secs() - st.2) as f64 }));
        o.set("error", Json::str(st.3));
        o.set("exitCode", st.4.map(|c| Json::Num(c as f64)).unwrap_or(Json::Null));
        o.set("host", Json::str(host.clone()));
        o.set("port", Json::Num(port as f64));
        o.set("logPath", Json::str(st.5));
        o.set("tokensPrompt", Json::Num(st.6 as f64));
        o.set("tokensGen", Json::Num(st.7 as f64));
        o.set("logCount", Json::Num(self.log_count() as f64));
        o.set("argsPreview", Json::str(settings::preview_args(cfg, "<model>")));
        let mut urls = Json::obj();
        urls.set("base", Json::str(format!("http://{}:{}/v1", display_host(&host), port)));
        urls.set("health", Json::str(format!("http://{}:{}/health", display_host(&host), port)));
        urls.set("local", Json::str(format!("http://127.0.0.1:{}/v1", port)));
        o.set("endpoints", urls);
        o
    }

    /// 当前选用的推理引擎（显示值："llama.cpp" 或 "KVMem"）
    pub fn engine_of(cfg: &Json) -> String {
        let e = cfg.s("launch.engine", "llama.cpp");
        if e.trim().is_empty() {
            "llama.cpp".to_string()
        } else {
            e.trim().to_string()
        }
    }

    /// 当前是否选中 KVMem 引擎（llama.cpp 后端，吃 GGUF）
    pub fn is_kvmem(cfg: &Json) -> bool {
        let e = Self::engine_of(cfg).to_ascii_lowercase();
        e.contains("kvmem") && !e.contains("ninfer")
    }

    /// 当前是否选中 NInfer 引擎（吃 .ninfer）
    pub fn is_ninfer(cfg: &Json) -> bool {
        Self::engine_of(cfg).to_ascii_lowercase().contains("ninfer")
    }

    /// 在程序目录内自动查找 KVMem 引擎（llama-kvmem-server.exe），找不到返回空串
    pub fn detect_kvmem_exe() -> String {
        let names = ["llama-kvmem-server.exe", "llama-kvmem-server"];
        let base = util::base_dir();
        let mut stack = vec![(base.clone(), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            if depth > 3 {
                continue;
            }
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let path = e.path();
                if path.is_file() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if names.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                        return path.to_string_lossy().to_string();
                    }
                } else if path.is_dir() {
                    stack.push((path, depth + 1));
                }
            }
        }
        String::new()
    }

    /// 在程序目录内自动查找 NInfer 引擎（infer-kvmem-server.exe）
    pub fn detect_ninfer_exe() -> String {
        Self::detect_exe_named(&["infer-kvmem-server.exe", "ninfer-kvmem-server.exe", "ninfer-serve.exe"])
    }

    fn detect_exe_named(names: &[&str]) -> String {
        let base = util::base_dir();
        let mut stack = vec![(base.clone(), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            if depth > 4 {
                continue;
            }
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let path = e.path();
                if path.is_file() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if names.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                        return path.to_string_lossy().to_string();
                    }
                } else if path.is_dir() {
                    stack.push((path, depth + 1));
                }
            }
        }
        String::new()
    }

    /// NInfer 引擎的 exe 路径
    pub fn ninfer_exe(cfg: &Json) -> String {
        let p = cfg.s("app.ninferExe", "");
        let t = p.trim();
        if !t.is_empty() && std::path::Path::new(t).is_file() {
            return t.to_string();
        }
        String::new()
    }

    /// KVMem 版引擎的 exe 路径（未配置或文件不存在时返回空串）
    pub fn kvmem_exe(cfg: &Json) -> String {
        let p = cfg.s("app.kvmemExe", "");
        let t = p.trim();
        if !t.is_empty() && std::path::Path::new(t).is_file() {
            return t.to_string();
        }
        String::new()
    }

    /// 定位推理引擎可执行文件（按 launch.engine 在 llama / kvmem 之间切换）
    pub fn resolve_exe(cfg: &Json) -> Result<PathBuf, String> {
        // NInfer 引擎：需要 infer-kvmem-server.exe
        if Self::is_ninfer(cfg) {
            let ni = Self::ninfer_exe(cfg);
            if !ni.is_empty() {
                return Ok(PathBuf::from(ni));
            }
            return Err("err.ninferNotFound".to_string());
        }
        // 选了 KVMem 引擎：必须有自己的 llama-kvmem-server.exe
        if Self::is_kvmem(cfg) {
            let kv = Self::kvmem_exe(cfg);
            if !kv.is_empty() {
                return Ok(PathBuf::from(kv));
            }
            return Err("err.kvmemNotFound".to_string());
        }
        let custom = cfg.s("app.llamaExe", "");
        if !custom.trim().is_empty() {
            let p = PathBuf::from(custom.trim());
            if p.is_file() {
                return Ok(p);
            }
            return Err(format!("err.exeNotFound:{}", custom));
        }
        let base = util::base_dir();
        let exe_name = if cfg!(windows) {
            "llama-server.exe"
        } else {
            "llama-server"
        };
        // 从程序目录向上回溯数层，兼容「exe 在 target/release」与「exe 在工程根」两种布局
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut cur = Some(base.as_path());
        for _ in 0..4 {
            if let Some(dir) = cur {
                roots.push(dir.to_path_buf());
                cur = dir.parent();
            }
        }
        let mut candidates: Vec<PathBuf> = Vec::new();
        for root in roots {
            candidates.push(root.join(exe_name));
            candidates.push(root.join("bin").join(exe_name));
            // 工程内随附的 llama 目录（llama-b12549 之类）
            if let Ok(rd) = std::fs::read_dir(&root) {
                let mut dirs: Vec<PathBuf> = rd
                    .flatten()
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .map(|e| e.path())
                    .collect();
                dirs.sort();
                for d in dirs {
                    candidates.push(d.join(exe_name));
                    candidates.push(d.join("bin").join(exe_name));
                }
            }
        }
        for c in candidates {
            if c.is_file() {
                return Ok(c);
            }
        }
        // PATH 中查找
        if let Ok(path) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path) {
                let c = dir.join(exe_name);
                if c.is_file() {
                    return Ok(c);
                }
            }
        }
        Err("err.exeNotFound:llama-server".into())
    }

    /// 启动时自动识别 llama-server：识别到就写进配置，识别不到返回 None
    pub fn autodetect_exe(cfg: &mut Json) -> Option<String> {
        let cur = cfg.s("app.llamaExe", "");
        if !cur.trim().is_empty() && std::path::Path::new(cur.trim()).is_file() {
            return Some(cur.trim().to_string());
        }
        // 配置为空或路径已失效：重新在程序目录附近找
        let mut probe = cfg.clone();
        probe.set_path("app.llamaExe", Json::str(""));
        match Launcher::resolve_exe(&probe) {
            Ok(p) => {
                let s = p.to_string_lossy().to_string();
                cfg.set_path("app.llamaExe", Json::str(&s));
                Some(s)
            }
            Err(_) => None,
        }
    }

    /// 面板启动时接管已在运行的 llama-server（例如上次未正常退出）
    pub fn adopt(&mut self, cfg: &Json) {
        let port = cfg.i("network.port", 9931) as u16;
        let host = if settings::bind_host(cfg) == "0.0.0.0" {
            "127.0.0.1".to_string()
        } else {
            settings::bind_host(cfg)
        };
        if !tcp_alive(&host, port) {
            return;
        }
        let pid = util::pid_listening_on(port);
        self.port = port;
        self.host = host;
        if let Ok(mut st) = self.state.lock() {
            st.phase = Phase::Running;
            st.pid = pid;
            st.started_at = util::now_secs();
            st.error.clear();
        }
        self.push_log(
            &format!("adopted existing server on port {} (pid {:?})", port, pid),
            "",
        );
    }

    pub fn start(&mut self, cfg: &Json, model_path: &str) -> Result<(), String> {
        self.stop();
        // 每次启动重新累计本轮用量
        if let Ok(mut st) = self.state.lock() {
            st.tokens_prompt = 0;
            st.tokens_gen = 0;
        }
        let exe = Launcher::resolve_exe(cfg)?;
        // 发射前按 llama-server 自己的参数表校验：已移除的参数自动剔除，未知的留警告
        let raw_args = settings::build_args(cfg, model_path);
        let (args, arg_warnings) = crate::paramspec::filter_args(raw_args);
        for w in arg_warnings.iter() {
            self.push_log(w, "");
        }
        let port = cfg.i("network.port", 9931) as u16;
        // 端口若被上个实例占用，先结束占用者，避免新进程绑定失败
        if let Some(pid) = util::pid_listening_on(port) {
            self.push_log(&format!("port {} busy, terminating pid {}", port, pid), "");
            util::kill_pid(pid);
            std::thread::sleep(Duration::from_millis(700));
        }
        let host = settings::bind_host(cfg);
        let log_path = util::logs_dir()
            .join(format!("server-{}.log", util::now_secs()))
            .to_string_lossy()
            .to_string();
        let log_name = PathBuf::from(&log_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        {
            let mut st = self.state.lock().map_err(|_| "err.lock".to_string())?;
            st.phase = Phase::Starting;
            st.error.clear();
            st.exit_code = None;
            st.args = args.clone();
            st.log_path = log_name.clone();
            st.started_at = 0;
            st.pid = None;
        }
        self.port = port;
        self.host = host.clone();

        let mut cmd = Command::new(&exe);
        cmd.args(&args);
        // 以可执行文件所在目录为工作目录：DLL 与相对资源按此处解析
        if let Some(dir) = exe.parent() {
            cmd.current_dir(dir);
        }
        // KVMem 的 worker 依赖同包内的 CUDA 运行库（runtime\cuda），
        // 直接把 worker 启动会缺 DLL，所以把这些目录补进子进程的 PATH
        {
            let mut extra: Vec<std::path::PathBuf> = Vec::new();
            if let Some(dir) = exe.parent() {
                extra.push(dir.to_path_buf());
                let mut anc = dir.parent();
                for _ in 0..3 {
                    if let Some(a) = anc {
                        for sub in ["runtime\\cuda", "runtime", "workers\\llamacpp"] {
                            let p = a.join(sub);
                            if p.is_dir() {
                                extra.push(p);
                            }
                        }
                        anc = a.parent();
                    }
                }
            }
            if !extra.is_empty() {
                let cur = std::env::var("PATH").unwrap_or_default();
                let mut merged = String::new();
                for p in extra.iter() {
                    merged.push_str(&p.to_string_lossy());
                    merged.push(';');
                }
                merged.push_str(&cur);
                cmd.env("PATH", merged);
            }
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            // 创建独立进程组，便于整组结束（含子进程）
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            cmd.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
        }

        self.push_log(&format!("launch: {} {}", exe.display(), args.join(" ")), &log_path);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("err.spawnFailed:{}", e))?;

        let pid = child.id();
        if let Ok(mut st) = self.state.lock() {
            st.pid = Some(pid);
            st.phase = Phase::Starting;
        }

        // 捕获 stdout / stderr
        let state_arc = Arc::clone(&self.state);
        let logs_arc = Arc::clone(&self.logs);
        let out_path = log_path.clone();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        if let Some(out) = stdout {
            spawn_reader(out, "out", state_arc.clone(), logs_arc.clone(), out_path.clone());
        }
        if let Some(err) = stderr {
            spawn_reader(err, "err", state_arc, logs_arc, out_path);
        }

        self.child = Some(child);
        self.device_ready_probe();
        Ok(())
    }

    /// 轮询端口，直到服务就绪或超时
    fn device_ready_probe(&self) {
        let host = if self.host == "0.0.0.0" || self.host.is_empty() {
            "127.0.0.1".to_string()
        } else {
            self.host.clone()
        };
        let port = self.port;
        let state_arc = Arc::clone(&self.state);
        std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(180);
            loop {
                {
                    let Ok(st) = state_arc.lock() else { return };
                    if st.phase != Phase::Starting {
                        return;
                    }
                }
                if tcp_alive(&host, port) {
                    if let Ok(mut st) = state_arc.lock() {
                        if st.phase == Phase::Starting {
                            st.phase = Phase::Running;
                            st.started_at = util::now_secs();
                        }
                    }
                    return;
                }
                if Instant::now() > deadline {
                    if let Ok(mut st) = state_arc.lock() {
                        if st.phase == Phase::Starting {
                            st.phase = Phase::Error;
                            st.error = "err.startTimeout".into();
                        }
                    }
                    return;
                }
                std::thread::sleep(Duration::from_millis(400));
            }
        });
    }

    pub fn stop(&mut self) {
        let mut killed = false;
        if let Some(mut child) = self.child.take() {
            let pid = child.id();
            #[cfg(windows)]
            {
                // 使用 taskkill 结束整棵进程树，避免 loader 子进程残留
                let _ = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            #[cfg(not(windows))]
            {
                let _ = child.kill();
            }
            let _ = child.wait();
            killed = true;
        }
        // 接管来的实例没有子进程句柄，按记录的 PID 结束
        if !killed {
            if let Ok(mut st) = self.state.lock() {
                if let Some(pid) = st.pid {
                    util::kill_pid(pid);
                }
            }
        }
        if let Ok(mut st) = self.state.lock() {
            st.phase = Phase::Stopped;
            st.pid = None;
            st.started_at = 0;
            st.args.clear();
        }
    }

    /// 面板退出时兜底，保证不留下孤儿进程
    pub fn shutdown(&mut self) {
        self.stop();
    }
}

impl Drop for Launcher {
    fn drop(&mut self) {
        let _ = self.child.take().map(|mut c| {
            let _ = c.kill();
            let _ = c.wait();
        });
    }
}

/// 读取子进程输出流并转发到日志环
fn spawn_reader<R: std::io::Read + Send + 'static>(
    stream: R,
    tag: &'static str,
    state: Arc<Mutex<ServerState>>,
    logs: Arc<Mutex<VecDeque<String>>>,
    log_path: String,
) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            let entry = format!("[{}] {}", tag, line);
            if let Ok(mut ring) = logs.lock() {
                ring.push_back(entry.clone());
                while ring.len() > LOG_RING {
                    ring.pop_front();
                }
            }
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                let _ = writeln!(f, "{}", entry);
            }
        }
        // 流结束表示进程退出
        if let Ok(mut st) = state.lock() {
            if st.phase != Phase::Stopped {
                st.phase = Phase::Error;
                st.error = "err.processExited".into();
                st.pid = None;
            }
        }
    });
}

fn tcp_alive(host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;
    let Ok(mut addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    match addrs.next() {
        Some(a) => std::net::TcpStream::connect_timeout(&a, Duration::from_millis(500)).is_ok(),
        None => false,
    }
}

/// 对外展示用地址：绑定全部接口时显示本机局域网 IP
pub fn display_host(bind: &str) -> String {
    if bind != "0.0.0.0" && !bind.is_empty() {
        return bind.to_string();
    }
    lan_ip().unwrap_or_else(|| "127.0.0.1".to_string())
}

/// 探测本机局域网 IPv4（通过连接外部地址获取默认出口）
pub fn lan_ip() -> Option<String> {
    use std::net::UdpSocket;
    let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else {
        return None;
    };
    if sock.connect("8.8.8.8:80").is_err() {
        return None;
    }
    sock.local_addr().ok().map(|a| a.ip().to_string())
}

fn format_timestamp() -> String {
    // 仅需可读时间戳，避免引入时间库
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs = now % 86400;
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
}

type StateSnapshot = (String, Option<u32>, u64, String, Option<i32>, String, u64, u64);

impl ServerState {
    fn clone_state(&self) -> StateSnapshot {
        let phase = match self.phase {
            Phase::Stopped => "stopped",
            Phase::Starting => "starting",
            Phase::Running => "running",
            Phase::Error => "error",
        };
        (
            phase.to_string(),
            self.pid,
            self.started_at,
            self.error.clone(),
            self.exit_code,
            self.log_path.clone(),
            self.tokens_prompt,
            self.tokens_gen,
        )
    }
}
