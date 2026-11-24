//! 显存实测校准
//! VRAM calibration from real measurements.
//!
//! 显存估算原本纯按 GGUF 元数据推算 KV，对带自定义注意力或逐层滑窗的架构会偏差很大
//! （gemma 静默滑窗按全注意力算高估一倍多；Inkling 反而低估 5 GB）。
//!
//! 这里在模型真正加载完成后量一次真实占用，按模型累积成「上下文 → 显存」的采样点。
//! 有两点以上时直接用实测直线外推，比任何公式都准；
//! 只有一点时用公式反推差额（extra）做近似。

use std::sync::Mutex;

use crate::json::Json;

/// 一个实测点：这个上下文下整卡多占了多少显存
#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub ctx: u32,
    pub mb: u32,
}

/// 某个模型的校准数据
#[derive(Clone)]
pub struct Calib {
    /// 历次实测点（按 ctx 升序，最多保留 8 个）
    pub points: Vec<Point>,
    /// 只有一点时用的近似：公式之外每 token 额外显存（MB/token）
    pub extra_mb_per_token: f64,
    /// 采样时的启动指纹：这些一变，旧的实测点就不再适用
    pub ngl: i64,
    pub ctk: String,
    pub ctv: String,
    pub ncmoe: i64,
    pub vision: bool,
    pub batch: i64,
    pub ubatch: i64,
    pub ts: u64,
}

/// 启动指纹：决定显存占用的参数组合
#[derive(Clone, PartialEq)]
pub struct Fingerprint {
    pub ngl: i64,
    pub ctk: String,
    pub ctv: String,
    pub ncmoe: i64,
    pub vision: bool,
    /// 批大小会显著改变计算缓冲（256K 时 ub 512 要 25 GB，ub 64 只要 4 GB），
    /// 所以必须一起记，否则不同批大小的测量点混用会算错
    pub batch: i64,
    pub ubatch: i64,
}

impl Fingerprint {
    pub fn matches(&self, c: &Calib) -> bool {
        self.ngl == c.ngl
            && self.ctk == c.ctk
            && self.ctv == c.ctv
            && self.ncmoe == c.ncmoe
            && self.vision == c.vision
            && self.batch == c.batch
            && self.ubatch == c.ubatch
    }
}

/// 估算公式版本。公式一改，单点近似就不成立；实测点不受影响。
const FORMULA_VERSION: i64 = 2;

/// 每个模型最多保留多少个实测点
const MAX_POINTS: usize = 8;

fn path() -> std::path::PathBuf {
    crate::util::data_dir().join("vram-calib.json")
}

fn cell() -> &'static Mutex<Json> {
    static C: std::sync::OnceLock<Mutex<Json>> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        let mut j = Json::obj();
        if let Ok(text) = std::fs::read_to_string(path()) {
            if let Ok(parsed) = crate::json::parse(&text) {
                if matches!(parsed, Json::Obj(_)) {
                    j = parsed;
                }
            }
        }
        Mutex::new(j)
    })
}

/// 读取某个模型的校准数据（没有则 None）
pub fn get(model_id: &str) -> Option<Calib> {
    let j = cell().lock().ok()?;
    let o = j.get(model_id)?;

    // 实测点：新格式
    let mut points: Vec<Point> = Vec::new();
    if let Some(arr) = o.get("points").and_then(|v| v.as_arr()) {
        for p in arr {
            let ctx = p.i("ctx", 0).max(0) as u32;
            let mb = p.i("mb", 0).max(0) as u32;
            if ctx > 0 && mb > 0 {
                points.push(Point { ctx, mb });
            }
        }
    }
    // 兼容旧格式（只有单点 + extra）
    if points.is_empty() {
        let ctx = o.i("sampleCtx", 0).max(0) as u32;
        let mb = o.i("sampleMb", 0).max(0) as u32;
        if ctx > 0 && mb > 0 {
            points.push(Point { ctx, mb });
        }
    }
    if points.is_empty() {
        return None;
    }
    points.sort_by_key(|p| p.ctx);
    points.dedup_by_key(|p| p.ctx);

    // 公式改过之后，单点近似那套就不再可信；但实测点照样能用
    let formula_ok = o.i("formula", 0) == FORMULA_VERSION;
    let extra = if formula_ok {
        o.get("extraMbPerToken")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    } else {
        0.0
    };

    Some(Calib {
        points,
        extra_mb_per_token: extra,
        ngl: o.i("ngl", 0),
        ctk: o.s("ctk", ""),
        ctv: o.s("ctv", ""),
        ncmoe: o.i("ncmoe", 0),
        vision: o.b("vision", false),
        batch: o.i("batch", 0),
        ubatch: o.i("ubatch", 0),
        ts: o.i("ts", 0).max(0) as u64,
    })
}

/// 用全部实测点拟合一条直线：返回 (每 token 多少 MB, 截距 MB)
/// 少于两个不同上下文的点时返回 None（此时退回公式 + 单点近似）
pub fn line(model_id: &str) -> Option<(f64, f64)> {
    let c = get(model_id)?;
    let pts: Vec<Point> = {
        let mut v = c.points.clone();
        v.sort_by_key(|p| p.ctx);
        v.dedup_by_key(|p| p.ctx);
        v
    };
    if pts.len() < 2 {
        return None;
    }
    let n = pts.len() as f64;
    let sx: f64 = pts.iter().map(|p| p.ctx as f64).sum();
    let sy: f64 = pts.iter().map(|p| p.mb as f64).sum();
    let sxx: f64 = pts.iter().map(|p| (p.ctx as f64).powi(2)).sum();
    let sxy: f64 = pts
        .iter()
        .map(|p| p.ctx as f64 * p.mb as f64)
        .sum();
    let denom = n * sxx - sx * sx;
    if denom.abs() < 1e-6 {
        return None;
    }
    let slope = (n * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / n;
    if !slope.is_finite() || !intercept.is_finite() || slope <= 0.0 {
        return None;
    }
    Some((slope, intercept))
}

/// 记一个实测点（同 ctx 覆盖），并保存
pub fn put_point(model_id: &str, ctx: u32, mb: u32, fp: &Fingerprint, extra_mb_per_token: f64) {
    if model_id.is_empty() || ctx == 0 || mb == 0 {
        return;
    }
    let Ok(mut j) = cell().lock() else { return };

    // 指纹变了就清空旧点：换了 KV 类型/层数，之前的测量不再可比
    let mut points: Vec<Point> = Vec::new();
    if let Some(o) = j.get(model_id) {
        let same = o.i("ngl", 0) == fp.ngl
            && o.s("ctk", "") == fp.ctk
            && o.s("ctv", "") == fp.ctv
            && o.i("ncmoe", 0) == fp.ncmoe
            && o.b("vision", false) == fp.vision
            && o.i("batch", 0) == fp.batch
            && o.i("ubatch", 0) == fp.ubatch;
        if same {
            if let Some(arr) = o.get("points").and_then(|v| v.as_arr()) {
                for p in arr {
                    let c = p.i("ctx", 0).max(0) as u32;
                    let m = p.i("mb", 0).max(0) as u32;
                    if c > 0 && m > 0 {
                        points.push(Point { ctx: c, mb: m });
                    }
                }
            }
        }
    }
    points.retain(|p| p.ctx != ctx);
    points.push(Point { ctx, mb });
    points.sort_by_key(|p| p.ctx);
    while points.len() > MAX_POINTS {
        points.remove(0);
    }

    let mut arr = Vec::new();
    for p in points.iter() {
        let mut o = Json::obj();
        o.set("ctx", Json::Num(p.ctx as f64));
        o.set("mb", Json::Num(p.mb as f64));
        arr.push(o);
    }

    let mut o = Json::obj();
    o.set("points", Json::Arr(arr));
    o.set("extraMbPerToken", Json::Num(extra_mb_per_token));
    o.set("sampleCtx", Json::Num(ctx as f64));
    o.set("sampleMb", Json::Num(mb as f64));
    o.set("ngl", Json::Num(fp.ngl as f64));
    o.set("ctk", Json::str(&fp.ctk));
    o.set("ctv", Json::str(&fp.ctv));
    o.set("ncmoe", Json::Num(fp.ncmoe as f64));
    o.set("vision", Json::Bool(fp.vision));
    o.set("batch", Json::Num(fp.batch as f64));
    o.set("ubatch", Json::Num(fp.ubatch as f64));
    o.set("ts", Json::Num(crate::util::now_secs() as f64));
    o.set("formula", Json::Num(FORMULA_VERSION as f64));
    j.set(model_id, o);
    let _ = std::fs::write(path(), j.to_pretty());
}

// ---------------------------------------------------------------------------
// 待采样状态：启动时记下基线，服务起来后再量一次
// ---------------------------------------------------------------------------

struct Pending {
    model_id: String,
    /// 绝对路径，用来重新读 GGUF 元数据算理论值
    model_path: String,
    file_bytes: u64,
    ctx: u32,
    fp: Fingerprint,
    flash_attn: bool,
    baseline_mb: u64,
    waited: u32,
    /// 上一次采样值，用来判断显存是否还在增长（模型还在加载）
    last_mb: u64,
    /// 连续「没再增长」的次数
    stable: u32,
    /// 至少涨这么多 MB 才认为模型真的装进去了
    min_mb: u64,
    /// 视觉投影文件占用（要从未知里扣掉，否则会被算成每 token 成本）
    mmproj_mb: u64,
}

static PENDING: std::sync::OnceLock<Mutex<Option<Pending>>> = std::sync::OnceLock::new();

fn pending_cell() -> &'static Mutex<Option<Pending>> {
    PENDING.get_or_init(|| Mutex::new(None))
}

/// 启动前调用：记下基线显存，服务就绪后再采样
pub fn begin(
    model_id: &str,
    model_path: &str,
    file_bytes: u64,
    ctx: u32,
    fp: Fingerprint,
    flash_attn: bool,
    baseline_mb: u64,
    mmproj_mb: u64,
) {
    let file_mb = (file_bytes / 1_048_576) as f64;
    let min_mb = (file_mb * 0.35).max(512.0) as u64;
    if let Ok(mut p) = pending_cell().lock() {
        *p = Some(Pending {
            model_id: model_id.to_string(),
            model_path: model_path.to_string(),
            file_bytes,
            ctx,
            fp,
            flash_attn,
            baseline_mb,
            waited: 0,
            last_mb: baseline_mb,
            stable: 0,
            min_mb,
            mmproj_mb,
        });
    }
}

/// 停止 / 切换模型时丢弃待采样
pub fn cancel() {
    if let Ok(mut p) = pending_cell().lock() {
        *p = None;
    }
}

/// 服务已就绪时每秒调用一次。等显存停止增长且超过文件大小 35% 再采样。
pub fn tick(running: bool) -> Option<(String, Calib)> {
    if !running {
        return None;
    }
    let sample = {
        let Ok(mut guard) = pending_cell().lock() else {
            return None;
        };
        let p = guard.as_mut()?;
        p.waited += 1;
        let now = match crate::monitor::gpu_mem_used_mb() {
            Some(v) => v,
            None => return None,
        };
        // 模型还在往显存里装：占用会持续上涨，这时候采样会严重偏低
        if now > p.last_mb + 64 {
            p.last_mb = now;
            p.stable = 0;
            if p.waited > 150 {
                guard.take();
            }
            return None;
        }
        p.stable += 1;
        if p.stable < 3 {
            p.last_mb = now;
            return None;
        }
        // 涨量没到预期说明还没装完，继续等
        if now < p.baseline_mb.saturating_add(p.min_mb) {
            p.stable = 0;
            p.last_mb = now;
            if p.waited > 300 {
                guard.take();
            }
            return None;
        }
        let s = guard.take()?;
        let mut s = s;
        s.last_mb = now;
        s
    };

    let used_whole = sample.last_mb;
    // 显存已经贴到上限，说明这次摆不下、驱动在偷偷换页，量到的数字不能用
    if let Some(total) = crate::monitor::gpu_mem_total_mb() {
        if total > 0 && (used_whole as f64) / (total as f64) >= 0.92 {
            return None;
        }
    }

    let used_mb = if sample.baseline_mb > 0 && used_whole > sample.baseline_mb {
        used_whole - sample.baseline_mb
    } else {
        used_whole
    } as f64;
    let used_mb = (used_mb - sample.mmproj_mb as f64).max(0.0);

    // 单点近似：用公式反推差额，供只有一个点的情况使用
    let info = match crate::gguf::read(&sample.model_path) {
        Some(i) => i,
        None => return None,
    };
    let est = crate::fit::estimate(
        &info,
        sample.file_bytes,
        sample.ctx,
        sample.fp.ngl.max(0) as u32,
        &sample.fp.ctk,
        &sample.fp.ctv,
        sample.flash_attn,
    );
    let theory_mb = est.total * 1024.0;
    let extra = if theory_mb > 0.0 && sample.ctx > 0 {
        (used_mb - theory_mb) / sample.ctx as f64
    } else {
        0.0
    };
    if !extra.is_finite() || !(-0.05..=0.5).contains(&extra) {
        return None;
    }

    put_point(
        &sample.model_id,
        sample.ctx,
        used_mb as u32,
        &sample.fp,
        extra,
    );
    let c = get(&sample.model_id)?;
    Some((sample.model_id.clone(), c))
}
