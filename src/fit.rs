//! 显存估算与推荐配置。
//!
//! 所有数字都由「GGUF 元数据 + 当前机器的实际显存」推算，不写死任何硬件，
//! 换一台电脑会自动按那台机器的显存给出不同结论。
//!
//! 估算分三块：
//!   * 权重：模型文件大小
//!   * KV 缓存：层数 × KV 头数 × (K维+V维) × token 数 × 每元素字节
//!     —— 混合架构（SSM/滑窗）只算真正需要全量 KV 的层
//!   * 计算缓冲：与上下文相关的经验值（Flash Attention 开启时更小）
//!
//! 结果会标注为「估算」，实测与估算通常相差 5%~15%。

use crate::gguf::GgufInfo;

/// 每元素字节数（KV 量化）
pub fn kv_elem_bytes(type_k: &str) -> f64 {
    match type_k.trim().to_ascii_lowercase().as_str() {
        "f32" => 4.0,
        "bf16" => 2.0,
        "q8_0" => 34.0 / 32.0,  // 32 个元素 34 字节
        "q5_1" => 24.0 / 32.0,
        "q5_0" => 22.0 / 32.0,
        "q4_1" => 20.0 / 32.0,
        "q4_0" => 18.0 / 32.0,
        "iq4_nl" => 18.0 / 32.0,
        _ => 2.0, // f16 默认
    }
}

#[derive(Clone, Debug, Default)]
pub struct Estimate {
    /// 权重（GB）
    pub weights: f64,
    /// KV 缓存（GB）
    pub kv: f64,
    /// 计算缓冲（GB）
    pub compute: f64,
    /// 合计（GB）
    pub total: f64,
    /// 放不下的层数会被放到内存
    pub on_cpu_layers: u32,
}

/// 「全部层上 GPU」的写法。
/// llama.cpp 里 -ngl 恰好等于层数并不等于全量卸载：它会把输出层、MoE 专家
/// 留在 CPU 上。实测 24 层 MoE 模型 -ngl 24 只有 4.8 tok/s，-ngl 99 有 39 tok/s。
/// 所以凡是要「全放 GPU」的地方都写 99。
pub const ALL_LAYERS: u32 = 99;

/// 估算一次运行需要的显存
///
/// * `file_bytes`：模型文件大小
/// * `layers`：模型层数（0 = 未知，按 32 估）
/// * `gpu_layers`：放到 GPU 的层数（>= 层数即全部）
pub fn estimate(
    info: &GgufInfo,
    file_bytes: u64,
    ctx: u32,
    gpu_layers: u32,
    kv_type_k: &str,
    kv_type_v: &str,
    flash_attn: bool,
) -> Estimate {
    let layers = if info.block_count > 0 {
        info.block_count
    } else if gpu_layers > 0 && gpu_layers < 99 {
        gpu_layers
    } else {
        32
    };
    let on_gpu = gpu_layers.min(layers);
    let frac = if layers > 0 {
        on_gpu as f64 / layers as f64
    } else {
        1.0
    };

    // ---- 权重 ----
    let weights_all = file_bytes as f64 / 1e9;
    // 只把上 GPU 的那部分算进显存（输出层等按比例忽略）
    let weights = weights_all * frac;

    // ---- KV ----
    let kv_layers_total = info.kv_layer_count().max(1);
    let kv_layers = (kv_layers_total as f64 * frac).round().max(1.0) as u32;
    let n_kv = if info.n_head_kv > 0 {
        info.n_head_kv
    } else if info.n_head > 0 {
        (info.n_head / 8).max(1)
    } else {
        8
    };
    let (k_len, v_len) = if info.k_len > 0 {
        (info.k_len, if info.v_len > 0 { info.v_len } else { info.k_len })
    } else if info.n_head > 0 {
        // 没有显式声明时按 head_dim = embd/heads 估，embd 未知则用 128
        (128, 128)
    } else {
        (128, 128)
    };
    // 滑窗层的 KV 只需窗口大小
    let tokens = if info.sliding_window > 0 && info.full_attn_interval <= 1 {
        ctx.min(info.sliding_window)
    } else {
        ctx
    } as f64;
    let b_k = kv_elem_bytes(kv_type_k);
    let b_v = kv_elem_bytes(kv_type_v);
    let kv = kv_layers as f64 * n_kv as f64 * (k_len as f64 * b_k + v_len as f64 * b_v)
        * tokens
        / 1e9;

    // ---- 计算缓冲 ----
    // 经验值：FA 开启约 0.6~1.2 GB，关闭时随上下文增长更快
    let ctx_gb = ctx as f64 / 32768.0;
    let compute = if flash_attn {
        0.55 + 0.55 * ctx_gb
    } else {
        0.75 + 1.40 * ctx_gb
    };

    let total = weights + kv + compute;
    Estimate {
        weights,
        kv,
        compute,
        total,
        on_cpu_layers: layers.saturating_sub(on_gpu),
    }
}

/// 一个可选配置
#[derive(Clone, Debug)]
pub struct Profile {
    /// 展示名（如「推荐：全 GPU · 32k · q8_0 KV」）
    pub title: String,
    /// 一句话说明
    pub note: String,
    pub ctx: u32,
    pub ngl: u32,
    pub kv_type: String,
    pub flash_attn: bool,
    /// MoE 专用：放到内存的专家层数（0 = 不启用）
    pub n_cpu_moe: u32,
    pub est: Estimate,
    /// 是否放得下（占显存 ≤ 92%）
    pub fits: bool,
}

/// 按模型与显存生成候选配置。
///
/// `vram_total_gb`：检测到的显存（0 = 未检测到，只给保守推荐）
pub fn profiles(
    info: &GgufInfo,
    file_bytes: u64,
    cur_ctx: u32,
    vram_total_gb: f64,
) -> Vec<Profile> {
    let layers = if info.block_count > 0 {
        info.block_count
    } else {
        32
    };
    let ctx0 = if cur_ctx >= 2048 { cur_ctx } else { 8192 };
    let budget = if vram_total_gb > 0.0 {
        vram_total_gb * 0.92
    } else {
        0.0
    };
    let mut out: Vec<Profile> = Vec::new();

    // 生成一个候选配置
    let make = |title: &str,
                note: &str,
                ctx: u32,
                ngl: u32,
                kv: &str,
                fa: bool,
                n_cpu_moe: u32|
     -> Profile {
        let est = estimate(info, file_bytes, ctx, ngl, kv, kv, fa);
        let fits = budget <= 0.0 || est.total <= budget;
        Profile {
            title: title.to_string(),
            note: note.to_string(),
            ctx,
            ngl,
            kv_type: kv.to_string(),
            flash_attn: fa,
            n_cpu_moe,
            est,
            fits,
        }
    };

    // ① 当前设置的效果（作为对照，总是放第一个）
    out.push(make(
        "当前设置",
        "不改任何参数，先看现在这套要多少显存",
        ctx0,
        ALL_LAYERS,
        "f16",
        true,
        0,
    ));
    // ② 推荐：全 GPU + q8_0 KV，上下文自动收缩到放得下
    let mut rec_ctx = ctx0;
    let mut rec_fit = false;
    while rec_ctx >= 2048 {
        let e = estimate(info, file_bytes, rec_ctx, ALL_LAYERS, "q8_0", "q8_0", true);
        if budget <= 0.0 || e.total <= budget {
            rec_fit = true;
            break;
        }
        rec_ctx /= 2;
    }
    if rec_fit {
        let shrunk = rec_ctx < ctx0;
        let note = if shrunk {
            format!(
                "全部层上 GPU，KV 用 q8_0；上下文从 {} 收缩到 {} 才放得下",
                ctx0, rec_ctx
            )
        } else {
            "全部层上 GPU，KV 用 q8_0（质量几乎无损，显存省一半）".to_string()
        };
        out.push(make("推荐 · KV 量化", &note, rec_ctx, ALL_LAYERS, "q8_0", true, 0));
    }
    // ③ 长上下文：2 倍上下文 + q8_0
    let ctx2 = (ctx0.saturating_mul(2)).min(262144);
    if ctx2 > ctx0 {
        out.push(make(
            "长上下文",
            "上下文翻倍，仍用 q8_0 KV；显存不够就别选这个",
            ctx2,
            ALL_LAYERS,
            "q8_0",
            true,
            0,
        ));
    }
    // ④ 显存吃紧：部分层放内存（上下文仍自动收缩）
    if budget > 0.0 && !rec_fit {
        for keep in [0.85_f64, 0.7, 0.55] {
            let ngl = ((layers as f64 * keep) as u32).max(1);
            let mut c = ctx0;
            while c >= 2048 {
                let est = estimate(info, file_bytes, c, ngl, "q8_0", "q8_0", true);
                if est.total <= budget {
                    let note = format!(
                        "把 {} 层放内存，上下文 {}，生成速度会明显下降",
                        layers - ngl,
                        c
                    );
                    out.push(make(
                        "显存吃紧 · 部分放内存",
                        &note,
                        c,
                        ngl,
                        "q8_0",
                        true,
                        0,
                    ));
                    break;
                }
                c /= 2;
            }
            if out.iter().any(|p| p.title.contains("显存吃紧")) {
                break;
            }
        }
    }
    // ⑤ MoE 专用：专家权重留内存，注意力留显存
    if info.is_moe() {
        for half in [layers / 3, layers / 2, (layers * 2) / 3] {
            if half == 0 {
                continue;
            }
            let est = estimate(info, file_bytes, ctx0, ALL_LAYERS, "q8_0", "q8_0", true);
            // 专家权重约占 80%，留在内存后显存里只算剩下的部分
            let saved = est.weights * 0.8 * (half as f64 / layers as f64);
            let mut e = est.clone();
            e.total = (e.total - saved).max(e.kv + e.compute + e.weights * 0.2);
            e.weights = (e.weights - saved).max(0.0);
            if budget > 0.0 && e.total <= budget {
                let note = format!(
                    "MoE 专用：{} 层（共 {} 层）的专家权重放内存，注意力仍在 GPU，比整体降层划算",
                    half, layers
                );
                out.push(Profile {
                    title: "MoE · 专家放内存".to_string(),
                    note,
                    ctx: ctx0,
                    ngl: ALL_LAYERS,
                    kv_type: "q8_0".to_string(),
                    flash_attn: true,
                    n_cpu_moe: half,
                    est: e,
                    fits: true,
                });
                break;
            }
        }
    }
    // ⑥ 纯 CPU（仅在没有可用显存时提供）
    if budget <= 0.0 {
        out.push(make(
            "纯 CPU",
            "未检测到显卡信息；全部层用 CPU",
            8192,
            0,
            "f16",
            false,
            0,
        ));
    }
    out
}
