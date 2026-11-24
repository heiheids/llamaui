//! 显存实测校准
//! VRAM calibration from real measurements.
//!
//! 显存估算原本纯按 GGUF 元数据推算 KV 大小，对带自定义注意力的新架构
//! （如 inkling2_loop 的 qk_clip / relative_extent / attn_res / shortconv）
//! 会严重低估：Inkling-2-Mini 实测 22.0 GB，公式只算出 12.9 GB。
//!
//! 这里在模型真正加载完成后量一次真实占用，和同一套公式算出的理论值比对，
//! 反推出「公式没算到的每 token 显存」，按模型记下来供后续估算修正。

use std::sync::Mutex;

use crate::json::Json;

/// 一次校准结果
#[derive(Clone)]
pub struct Calib {
    /// 公式之外的每 token 额外显存（MB / token）
    pub extra_mb_per_token: f64,
    /// 采样时的上下文长度与整卡占用（仅用于展示）
    pub sample_ctx: u32,
    pub sample_mb: u32,
    /// 采样时的启动指纹：这些一变，校准就不再适用
    pub ngl: i64,
    pub ctk: String,
    pub ctv: String,
    pub ncmoe: i64,
    pub vision: bool,
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
}

impl Fingerprint {
    pub fn matches(&self, c: &Calib) -> bool {
        self.ngl == c.ngl
            && self.ctk == c.ctk
            && self.ctv == c.ctv
            && self.ncmoe == c.ncmoe
            && self.vision == c.vision
    }
}

/// 估算公式版本。公式一改，旧校准值就不再成立，必须重新测。
/// 1 = 计算缓冲线性增长（0.55 + 0.55×ctx/32k）
/// 2 = 计算缓冲近似固定（0.35 + 0.10×ctx/32k）
const FORMULA_VERSION: i64 = 2;

fn path() -> std::path::PathBuf {
    crate::util::data_dir().join("vram-calib.json")
}

fn cell() -> &'static Mutex<Json> {
    static C: std::sync::OnceLock<Mutex<Json>> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        let mut j = Json::obj();
        if let Ok(text) = std::fs::read_to_string(path()) {
            if let Ok(Json::Obj(_)) = crate::json::parse(&text) {
                if let Ok(parsed) = crate::json::parse(&text) {
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
    // 公式改过就用旧值没有意义，交给下次启动重新校准
    if o.i("formula", 0) != FORMULA_VERSION {
        return None;
    }
    let extra = o.get("extraMbPerToken")?.as_f64()?;
    Some(Calib {
        extra_mb_per_token: extra,
        sample_ctx: o.i("sampleCtx", 0).max(0) as u32,
        sample_mb: o.i("sampleMb", 0).max(0) as u32,
        ngl: o.i("ngl", 0),
        ctk: o.s("ctk", ""),
        ctv: o.s("ctv", ""),
        ncmoe: o.i("ncmoe", 0),
        vision: o.b("vision", false),
        ts: o.i("ts", 0).max(0) as u64,
    })
}

/// 写入校准结果并落盘
pub fn put(model_id: &str, c: &Calib) {
    if model_id.is_empty() {
        return;
    }
    let Ok(mut j) = cell().lock() else { return };
    let mut o = Json::obj();
    o.set("extraMbPerToken", Json::Num(c.extra_mb_per_token));
    o.set("sampleCtx", Json::Num(c.sample_ctx as f64));
    o.set("sampleMb", Json::Num(c.sample_mb as f64));
    o.set("ngl", Json::Num(c.ngl as f64));
    o.set("ctk", Json::str(&c.ctk));
    o.set("ctv", Json::str(&c.ctv));
    o.set("ncmoe", Json::Num(c.ncmoe as f64));
    o.set("vision", Json::Bool(c.vision));
    o.set("ts", Json::Num(c.ts as f64));
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
    let baseline = baseline_mb;
    // 模型权重是显存占用的大头，至少要涨到文件大小的 35% 才算装进去；
    // 否则会在「读盘/初始化」的平台期就采样，量出来只有几百 MB。
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
            baseline_mb: baseline,
            waited: 0,
            last_mb: baseline,
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

/// 服务已就绪时每秒调用一次。攒够 3 次 tick（等显存分配稳定）再采样并落盘。
/// 返回刚完成的校准，供界面提示。
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
        // 模型还在往显存里装：占用会持续上涨，这时候采样会严重偏低。
        // 一直等到连续两次几乎不再变化，才算加载完成。
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
        // 涨量没到预期说明模型还没装完，继续等
        if now < p.baseline_mb.saturating_add(p.min_mb) {
            p.stable = 0;
            p.last_mb = now;
            if p.waited > 300 {
                guard.take();
            }
            return None;
        }
        guard.take()?
    };

    let used_whole = sample.last_mb;
    // 显存已经贴到上限，说明这次摆不下、驱动在偷偷换页，
    // 量到的数字不能当校准样本（会把每 token 开销算小）
    if let Some(total) = crate::monitor::gpu_mem_total_mb() {
        if total > 0 && (used_whole as f64) / (total as f64) >= 0.92 {
            return None;
        }
    }
    // 启动前量过基线就用增量，否则退回整卡占用（会偏高，但至少不是低估）
    let used_mb = if sample.baseline_mb > 0 && used_whole > sample.baseline_mb {
        used_whole - sample.baseline_mb
    } else {
        used_whole
    } as f64;

    // 用同一套公式算理论值，差值就是公式没算到的部分
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
    let used_mb = (used_mb - sample.mmproj_mb as f64).max(0.0);
    if theory_mb <= 0.0 || sample.ctx == 0 {
        return None;
    }
    let extra = (used_mb - theory_mb) / sample.ctx as f64;
    // 明显异常（采样失败 / 别的进程占显存）就别记，免得污染后续估算
    if !extra.is_finite() || !(-0.05..=0.5).contains(&extra) {
        return None;
    }
    let c = Calib {
        extra_mb_per_token: extra,
        sample_ctx: sample.ctx,
        sample_mb: used_mb as u32,
        ngl: sample.fp.ngl,
        ctk: sample.fp.ctk.clone(),
        ctv: sample.fp.ctv.clone(),
        ncmoe: sample.fp.ncmoe,
        vision: sample.fp.vision,
        ts: crate::util::now_secs(),
    };
    put(&sample.model_id, &c);
    Some((sample.model_id.clone(), c))
}
