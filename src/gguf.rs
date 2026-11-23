//! 读取 GGUF 文件头里的关键元数据（架构与原生上下文长度）。
//! 只解析元数据段，不加载张量，几百毫秒内完成。
//!
//! Read key GGUF metadata (architecture + native context length) without
//! loading tensors.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct GgufInfo {
    pub arch: String,
    /// 模型原生上下文长度（未声明的为 0）
    pub ctx_native: u32,
    /// rope 缩放后的原始上下文（存在 yarn 等缩放时参考）
    pub ctx_original: u32,
    pub name: String,
    /// 是否带分词器元数据。语言模型必有；扩散/图像模型没有
    pub has_tokenizer: bool,
    /// 层数
    pub block_count: u32,
    /// 专家总数（MoE 模型 > 0）
    pub expert_count: u32,
    /// 每次激活的专家数
    pub expert_used: u32,
    /// KV 头数
    pub n_head_kv: u32,
    /// 注意力头数（head_count_kv 缺失时用来估算）
    pub n_head: u32,
    /// K/V 的维度
    pub k_len: u32,
    pub v_len: u32,
    /// 每 N 层有一个全注意力层（混合 SSM 模型，0 = 没有）
    pub full_attn_interval: u32,
    /// 滑窗大小（0 = 没有滑窗）
    pub sliding_window: u32,
    /// 是否带逐层滑窗模式（true = 只有部分层是滑窗，KV 仍按完整上下文算）
    pub sliding_pattern: bool,
    /// 内置 MTP（多 token 预测）层数，0 = 不支持
    pub nextn_layers: u32,
    /// 是否思考型模型（chat template 里带 think / reasoning 之类的开关）
    pub thinking: bool,
}

impl GgufInfo {
    /// 是否 MoE 模型
    pub fn is_moe(&self) -> bool {
        self.expert_count > 1
    }
    /// 需要为每个 token 保存完整 KV 的层数。
    /// 混合架构（如 qwen35 的 SSM + attention）只有 1/full_attn_interval 的层需要。
    pub fn kv_layer_count(&self) -> u32 {
        let layers = self.block_count.max(1);
        if self.full_attn_interval > 1 {
            (layers + self.full_attn_interval - 1) / self.full_attn_interval
        } else {
            layers
        }
    }
}

/// 是否是「能对话/能补全」的语言模型。
/// 扩散类（Stable Diffusion / Qwen-Image / Flux / 视频模型等）虽然也是 GGUF，
/// 但 llama.cpp 跑不了它们，也不该出现在模型库里。
pub fn is_chat_model(info: &GgufInfo) -> bool {
    const NON_CHAT: &[&str] = &[
        "image",
        "flux",
        "diffus",
        "vae",
        "video",
        "stable-diffusion",
        "sd3",
        "sd15",
        "sdxl",
        "wan",
        "ltx",
        "mochi",
        "pixart",
        "lumina",
        "kolors",
        "hidream",
        "chroma",
        "auraflow",
        "cosmos",
    ];
    let arch = info.arch.to_ascii_lowercase();
    if NON_CHAT.iter().any(|k| arch.contains(k)) {
        return false;
    }
    // 没有分词器、也没有上下文长度的 GGUF 不是语言模型
    info.has_tokenizer || info.ctx_native > 0
}

pub fn read(path: &str) -> Option<GgufInfo> {
    let p = Path::new(path);
    if !p.is_file() {
        return None;
    }
    let file = File::open(p).ok()?;
    let mut r = BufReader::with_capacity(1 << 16, file);

    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).ok()?;
    if &magic != b"GGUF" {
        return None;
    }
    let version = read_u32(&mut r)?;
    if version < 2 {
        // v1 的元数据布局不同，直接放弃（老文件极少见）
        return None;
    }
    let _tensor_count = read_u64(&mut r)?;
    let kv_count = read_u64(&mut r)?;
    if kv_count > 100_000 {
        return None;
    }

    let mut info = GgufInfo::default();
    let mut arch = String::new();
    for _ in 0..kv_count {
        let key = read_string(&mut r)?;
        let vtype = read_u32(&mut r)?;
        // 只关心字符串和整数，其它类型按规范跳过
        match vtype {
            8 => {
                let val = read_string(&mut r)?;
                if key == "general.architecture" {
                    arch = val.clone();
                } else if key == "general.name" {
                    info.name = val;
                } else if key == "tokenizer.chat_template" {
                    // 思考型模型会在模板里暴露 think / reasoning 之类的开关或标记，
                    // 这类模型输出上限要给得足够大，否则只吐思考不吐答案
                    let low = val.to_lowercase();
                    info.thinking = low.contains("think")
                        || low.contains("reasoning")
                        || low.contains("深思")
                        || low.contains("思考");
                } else if key.starts_with("tokenizer.ggml.") {
                    info.has_tokenizer = true;
                }
            }
            4 => {
                let val = read_u32(&mut r)?;
                apply_u32_key(&mut info, &key, val);
            }
            10 => {
                let val = read_u64(&mut r)? as u32;
                apply_u32_key(&mut info, &key, val);
            }
            9 => {
                // 数组：head_count_kv 等可能按层存成数组，取第一个元素
                let atype = read_u32(&mut r)?;
                let alen = read_u64(&mut r)?;
                if alen > 10_000_000 {
                    return None;
                }
                // 逐层滑窗模式：这个键是数组，走不到下面的标量分支，
                // 但它决定 KV 是否按完整上下文计算，必须先记下来
                if key.ends_with(".attention.sliding_window_pattern") {
                    info.sliding_pattern = true;
                }
                let is_int = matches!(atype, 4 | 5 | 10 | 11);
                let mut first: Option<u32> = None;
                for i in 0..alen {
                    if is_int && first.is_none() {
                        first = Some(match atype {
                            4 | 5 => read_u32(&mut r)?,
                            _ => read_u64(&mut r)? as u32,
                        });
                    } else {
                        skip_value(&mut r, atype)?;
                    }
                    let _ = i;
                }
                if let Some(v) = first {
                    apply_u32_key(&mut info, &key, v);
                }
            }
            _ => skip_value(&mut r, vtype)?,
        }
    }
    info.arch = arch;
    if info.ctx_native == 0 && info.ctx_original > 0 {
        info.ctx_native = info.ctx_original;
    }
    Some(info)
}

fn read_u32<R: Read>(r: &mut R) -> Option<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).ok()?;
    Some(u32::from_le_bytes(b))
}

/// 把数值型元数据按后缀填进 GgufInfo
fn apply_u32_key(info: &mut GgufInfo, key: &str, val: u32) {
    if key.ends_with(".context_length") {
        info.ctx_native = val;
    } else if key.ends_with(".rope.scaling.original_context_length") {
        info.ctx_original = val;
    } else if key.ends_with(".block_count") {
        info.block_count = val;
    } else if key.ends_with(".expert_count") {
        info.expert_count = val;
    } else if key.ends_with(".expert_used_count") {
        info.expert_used = val;
    } else if key.ends_with(".attention.head_count_kv") {
        info.n_head_kv = val;
    } else if key.ends_with(".attention.head_count") {
        info.n_head = val;
    } else if key.ends_with(".attention.key_length") {
        info.k_len = val;
    } else if key.ends_with(".attention.value_length") {
        info.v_len = val;
    } else if key.ends_with(".full_attention_interval") {
        info.full_attn_interval = val;
    } else if key.ends_with(".nextn_predict_layers") {
        info.nextn_layers = val;
    } else if key.ends_with(".attention.sliding_window_pattern") {
        info.sliding_pattern = true;
    } else if key.ends_with(".attention.sliding_window") {
        info.sliding_window = val;
    }
}

fn read_u64<R: Read>(r: &mut R) -> Option<u64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).ok()?;
    Some(u64::from_le_bytes(b))
}

fn read_string<R: Read>(r: &mut R) -> Option<String> {
    let len = read_u64(r)? as usize;
    if len > 1 << 20 {
        return None;
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf).to_string())
}

/// 跳过一个值（按类型定长 / 数组逐个跳过）
fn skip_value<R: Read>(r: &mut R, vtype: u32) -> Option<()> {
    match vtype {
        0 | 1 | 7 => {
            let mut b = [0u8; 1];
            r.read_exact(&mut b).ok()?;
        }
        2 | 3 => {
            let mut b = [0u8; 2];
            r.read_exact(&mut b).ok()?;
        }
        4 | 5 | 6 => {
            let mut b = [0u8; 4];
            r.read_exact(&mut b).ok()?;
        }
        10 | 11 | 12 => {
            let mut b = [0u8; 8];
            r.read_exact(&mut b).ok()?;
        }
        8 => {
            let len = read_u64(r)? as usize;
            if len > 1 << 20 {
                return None;
            }
            let mut buf = vec![0u8; len];
            r.read_exact(&mut buf).ok()?;
        }
        9 => {
            let atype = read_u32(r)?;
            let alen = read_u64(r)?;
            if alen > 10_000_000 {
                return None;
            }
            for _ in 0..alen {
                skip_value(r, atype)?;
            }
        }
        _ => return None,
    }
    Some(())
}
