//! 设置定义与命令行参数生成
//! Settings definition (single source of truth) and llama-server argument builder.
//!
//! 每个字段都带有 i18n 标签键，界面据此渲染，绝不在代码里硬编码文案。

use crate::json::Json;

/// 字段控件类型
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Text,
    MultilineText,
    Int,
    Float,
    Bool,
    Select,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::MultilineText => "multiline",
            Kind::Int => "int",
            Kind::Float => "float",
            Kind::Bool => "bool",
            Kind::Select => "select",
        }
    }
}

pub struct Field {
    pub key: &'static str,
    pub kind: Kind,
    pub section: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub options: &'static [&'static str],
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub advanced: bool,
}

const fn f(
    key: &'static str,
    kind: Kind,
    section: &'static str,
    label: &'static str,
    hint: &'static str,
    advanced: bool,
) -> Field {
    Field {
        key,
        kind,
        section,
        label,
        hint,
        options: &[],
        min: None,
        max: None,
        step: None,
        advanced,
    }
}

const fn bool_field(
    key: &'static str,
    section: &'static str,
    label: &'static str,
    hint: &'static str,
    advanced: bool,
) -> Field {
    Field {
        key,
        kind: Kind::Bool,
        section,
        label,
        hint,
        options: &[],
        min: None,
        max: None,
        step: None,
        advanced,
    }
}

const fn sel(
    key: &'static str,
    section: &'static str,
    label: &'static str,
    hint: &'static str,
    options: &'static [&'static str],
    advanced: bool,
) -> Field {
    Field {
        key,
        kind: Kind::Select,
        section,
        label,
        hint,
        options,
        min: None,
        max: None,
        step: None,
        advanced,
    }
}

const fn num(
    key: &'static str,
    section: &'static str,
    label: &'static str,
    hint: &'static str,
    min: f64,
    max: f64,
    step: f64,
    advanced: bool,
) -> Field {
    Field {
        key,
        kind: Kind::Int,
        section,
        label,
        hint,
        options: &[],
        min: Some(min),
        max: Some(max),
        step: Some(step),
        advanced,
    }
}


const fn flt(
    key: &'static str,
    section: &'static str,
    label: &'static str,
    hint: &'static str,
    min: f64,
    max: f64,
    step: f64,
    advanced: bool,
) -> Field {
    Field {
        key,
        kind: Kind::Float,
        section,
        label,
        hint,
        options: &[],
        min: Some(min),
        max: Some(max),
        step: Some(step),
        advanced,
    }
}

pub const SELECT_YES_NO: &[&str] = &["auto", "on", "off"];
/// 推理引擎：llama.cpp 原生，或 KVMem（需自带 llama-kvmem-server.exe）
pub const SELECT_ENGINE: &[&str] = &["llama.cpp", "KVMem", "NInfer"];
pub const SELECT_LOAD_MODE: &[&str] = &["auto", "none", "mmap", "mlock", "mmap+mlock"];
pub const SELECT_SPLIT: &[&str] = &["auto", "none", "layer", "row", "tensor"];
pub const SELECT_CACHE: &[&str] = &["f16", "f32", "bf16", "q8_0", "q5_1", "q5_0", "q4_1", "q4_0", "iq4_nl"];
pub const SELECT_REASONING: &[&str] = &["auto", "on", "off"];
pub const SELECT_NUMA: &[&str] = &["auto", "distribute", "isolate", "numactl"];
/// RoPE 频率缩放方式（llama.cpp 的 --rope-scaling 取值）
pub const SELECT_ROPE: &[&str] = &["auto", "none", "linear", "yarn"];


/// 全部设置字段（顺序即界面顺序）
pub const FIELDS: &[Field] = &[
    // ---------- 常用：模型加载 ----------
    f("launch.alias", Kind::Text, "common", "field.alias", "hint.alias", false),
    bool_field("launch.vision", "common", "field.vision", "hint.vision", false),
    f("launch.mmproj", Kind::Text, "advanced", "field.mmproj", "hint.mmproj", true),
    num("launch.ctxSize", "common", "field.ctxSize", "hint.ctxSize", 0.0, 1048576.0, 256.0, false),
    // 勾上后「上下文长度」始终跟随所选模型的原生上限，切换模型时自动改写
    bool_field("app.ctxAuto", "common", "field.ctxAuto", "hint.ctxAuto", false),
    num("launch.nGpuLayers", "common", "field.nGpuLayers", "hint.nGpuLayers", 0.0, 999.0, 1.0, false),
    sel("launch.engine", "common", "field.engine", "hint.engine", SELECT_ENGINE, true),
    num("launch.threads", "common", "field.threads", "hint.threads", 0.0, 512.0, 1.0, false),
    num("launch.parallel", "common", "field.parallel", "hint.parallel", 1.0, 64.0, 1.0, false),
    sel("launch.flashAttn", "common", "field.flashAttn", "hint.flashAttn", SELECT_YES_NO, false),
    f("launch.jinja", Kind::Bool, "common", "field.jinja", "hint.jinja", false),
    f("launch.mtp", Kind::Bool, "common", "field.mtp", "hint.mtp", false),
    // ---------- 常用：网络 / API ----------
    f("network.lanOpen", Kind::Bool, "network", "field.lanOpen", "hint.lanOpen", false),
    f("network.host", Kind::Text, "network", "field.host", "hint.host", false),
    num("network.port", "network", "field.port", "hint.port", 1.0, 65535.0, 1.0, false),
    f("network.apiKeyEnabled", Kind::Bool, "network", "field.apiKeyEnabled", "hint.apiKeyEnabled", false),
    f("network.apiKeys", Kind::MultilineText, "network", "field.apiKeys", "hint.apiKeys", false),
    // ---------- 高级：模型 / 显存 ----------
    num("launch.batchSize", "advanced", "field.batchSize", "hint.batchSize", 1.0, 65536.0, 64.0, true),
    num("launch.ubatchSize", "advanced", "field.ubatchSize", "hint.ubatchSize", 1.0, 65536.0, 64.0, true),
    sel("launch.loadMode", "advanced", "field.loadMode", "hint.loadMode", SELECT_LOAD_MODE, true),
    sel("gpu.splitMode", "advanced", "field.splitMode", "hint.splitMode", SELECT_SPLIT, true),
    f("gpu.tensorSplit", Kind::Text, "advanced", "field.tensorSplit", "hint.tensorSplit", true),
    num("gpu.mainGpu", "advanced", "field.mainGpu", "hint.mainGpu", 0.0, 64.0, 1.0, true),
    num("gpu.nCpuMoe", "advanced", "field.nCpuMoe", "hint.nCpuMoe", 0.0, 999.0, 1.0, true),
    sel("cache.typeK", "advanced", "field.cacheTypeK", "hint.cacheTypeK", SELECT_CACHE, true),
    sel("cache.typeV", "advanced", "field.cacheTypeV", "hint.cacheTypeV", SELECT_CACHE, true),
    // ---------- 高级：采样 ----------
    flt("sampling.temp", "advanced", "field.temp", "hint.temp", 0.0, 5.0, 0.05, true),
    num("sampling.topK", "advanced", "field.topK", "hint.topK", 0.0, 1000.0, 1.0, true),
    flt("sampling.topP", "advanced", "field.topP", "hint.topP", 0.0, 1.0, 0.01, true),
    flt("sampling.minP", "advanced", "field.minP", "hint.minP", 0.0, 1.0, 0.01, true),
    flt("sampling.repeatPenalty", "advanced", "field.repeatPenalty", "hint.repeatPenalty", 0.0, 4.0, 0.05, true),
    num("sampling.seed", "advanced", "field.seed", "hint.seed", -1.0, 4294967295.0, 1.0, true),
    // ---------- 高级：服务 ----------
    num("network.timeout", "advanced", "field.timeout", "hint.timeout", 1.0, 86400.0, 30.0, true),
    num("network.threadsHttp", "advanced", "field.threadsHttp", "hint.threadsHttp", -1.0, 512.0, 1.0, true),
    f("network.builtinUi", Kind::Bool, "advanced", "field.builtinUi", "hint.builtinUi", true),
    f("network.metrics", Kind::Bool, "advanced", "field.metrics", "hint.metrics", true),
    f("network.slots", Kind::Bool, "advanced", "field.slots", "hint.slots", true),
    f("launch.embeddings", Kind::Bool, "advanced", "field.embeddings", "hint.embeddings", true),
    f("launch.noWarmup", Kind::Bool, "advanced", "field.noWarmup", "hint.noWarmup", true),
    f(
        "app.autoTranslate",
        Kind::Bool,
        "advanced",
        "field.autoTranslate",
        "hint.autoTranslate",
        true,
    ),
    sel("adv.reasoning", "advanced", "field.reasoning", "hint.reasoning", SELECT_REASONING, true),
    num("adv.reasoningBudget", "advanced", "field.reasoningBudget", "hint.reasoningBudget", -1.0, 100000.0, 1.0, true),
    // 输出上限（tokens）：0 = 自动。思考型模型自动给一个宽裕值，
    // 否则它可能把预算全花在思考上，返回空答案
    num("adv.maxOutput", "advanced", "field.maxOutput", "hint.maxOutput", 0.0, 262144.0, 1024.0, true),
    // RoPE 缩放：把上下文扩到原生之上时，用它代替「裸外推」
    sel("adv.ropeScaling", "advanced", "field.ropeScaling", "hint.ropeScaling", SELECT_ROPE, true),
    num("adv.ropeScale", "advanced", "field.ropeScale", "hint.ropeScale", 0.0, 64.0, 0.5, true),
    num("adv.yarnOrigCtx", "advanced", "field.yarnOrigCtx", "hint.yarnOrigCtx", 0.0, 1048576.0, 1024.0, true),
    f("adv.chatTemplateFile", Kind::Text, "advanced", "field.chatTemplateFile", "hint.chatTemplateFile", true),
    num("adv.prio", "advanced", "field.prio", "hint.prio", -1.0, 3.0, 1.0, true),
    sel("adv.numa", "advanced", "field.numa", "hint.numa", SELECT_NUMA, true),
    f("adv.corsOrigins", Kind::Text, "advanced", "field.corsOrigins", "hint.corsOrigins", true),
    f("adv.extraArgs", Kind::MultilineText, "advanced", "field.extraArgs", "hint.extraArgs", true),
];

/// 默认配置：所有键都显式给出，保证首次启动即可用
pub fn defaults() -> Json {
    let mut c = Json::obj();
    c.set_path("app.lang", Json::str("auto"));
    c.set_path("app.autoOpenBrowser", Json::Bool(true));
    c.set_path("app.panelPort", Json::Num(8790.0));
    c.set_path("app.llamaExe", Json::str(""));
    c.set_path("app.scanDirs", Json::Arr(vec![]));
    c.set_path("app.selectedModel", Json::str(""));
    // 上下文长度默认自动跟随模型上限；用户手动改并保存后置为 false
    c.set_path("app.ctxAuto", Json::Bool(true));
    c.set_path("launch.alias", Json::str(""));
    c.set_path("launch.vision", Json::Bool(true));
    c.set_path("launch.mmproj", Json::str(""));
    c.set_path("launch.ctxSize", Json::Num(32768.0));
    c.set_path("launch.nGpuLayers", Json::Num(99.0));
    c.set_path("launch.threads", Json::Num(0.0));
    c.set_path("launch.parallel", Json::Num(1.0));
    c.set_path("launch.flashAttn", Json::str("auto"));
    c.set_path("launch.jinja", Json::Bool(true));
    c.set_path("launch.batchSize", Json::Num(2048.0));
    c.set_path("launch.ubatchSize", Json::Num(512.0));
    c.set_path("launch.loadMode", Json::str("auto"));
    c.set_path("launch.embeddings", Json::Bool(false));
    c.set_path("launch.noWarmup", Json::Bool(false));
    c.set_path("app.autoTranslate", Json::Bool(false));
    c.set_path("network.lanOpen", Json::Bool(false));
    c.set_path("network.host", Json::str("127.0.0.1"));
    c.set_path("network.port", Json::Num(9931.0));
    c.set_path("network.apiKeyEnabled", Json::Bool(true));
    c.set_path("network.apiKeys", Json::Arr(vec![Json::str(crate::util::new_api_key())]));
    c.set_path("network.timeout", Json::Num(3600.0));
    c.set_path("network.threadsHttp", Json::Num(-1.0));
    c.set_path("network.builtinUi", Json::Bool(false));
    c.set_path("network.metrics", Json::Bool(false));
    c.set_path("network.slots", Json::Bool(true));
    c.set_path("gpu.splitMode", Json::str("auto"));
    c.set_path("gpu.tensorSplit", Json::str(""));
    c.set_path("gpu.mainGpu", Json::Num(0.0));
    c.set_path("gpu.nCpuMoe", Json::Num(0.0));
    c.set_path("cache.typeK", Json::str("f16"));
    c.set_path("cache.typeV", Json::str("f16"));
    c.set_path("sampling.temp", Json::Num(0.8));
    c.set_path("sampling.topK", Json::Num(40.0));
    c.set_path("sampling.topP", Json::Num(0.95));
    c.set_path("sampling.minP", Json::Num(0.05));
    c.set_path("sampling.repeatPenalty", Json::Num(1.1));
    c.set_path("sampling.seed", Json::Num(-1.0));
    c.set_path("adv.reasoning", Json::str("auto"));
    c.set_path("adv.reasoningBudget", Json::Num(-1.0));
    // 0 = 自动：思考型模型给宽裕的输出上限，普通模型不干预
    c.set_path("adv.maxOutput", Json::Num(0.0));
    c.set_path("adv.ropeScaling", Json::str("auto"));
    c.set_path("adv.ropeScale", Json::Num(0.0));
    c.set_path("adv.yarnOrigCtx", Json::Num(0.0));
    c.set_path("adv.chatTemplateFile", Json::str(""));
    c.set_path("adv.prio", Json::Num(0.0));
    c.set_path("adv.numa", Json::str("auto"));
    c.set_path("adv.corsOrigins", Json::str(""));
    c.set_path("adv.extraArgs", Json::str(""));
    c
}

/// 供界面使用的字段元数据
pub fn schema_json() -> Json {
    let mut fields = Vec::new();
    for fl in FIELDS {
        let mut o = Json::obj();
        o.set("key", Json::str(fl.key));
        o.set("kind", Json::str(fl.kind.as_str()));
        o.set("section", Json::str(fl.section));
        o.set("label", Json::str(fl.label));
        o.set("hint", Json::str(fl.hint));
        o.set("advanced", Json::Bool(fl.advanced));
        if !fl.options.is_empty() {
            o.set(
                "options",
                Json::Arr(fl.options.iter().map(|s| Json::str(*s)).collect()),
            );
        }
        if let Some(v) = fl.min {
            o.set("min", Json::Num(v));
        }
        if let Some(v) = fl.max {
            o.set("max", Json::Num(v));
        }
        if let Some(v) = fl.step {
            o.set("step", Json::Num(v));
        }
        fields.push(o);
    }
    Json::Arr(fields)
}

/// 绑定地址：自定义优先，其次局域网开关
/// 默认值是 127.0.0.1；打开“允许局域网访问”且地址仍为默认时改绑 0.0.0.0，
/// 这样开关依然有效，用户填了具体地址则完全以填写的为准。
pub fn bind_host(cfg: &Json) -> String {
    let custom = cfg.s("network.host", "").trim().to_string();
    let lan = cfg.b("network.lanOpen", false);
    if custom.is_empty() {
        return if lan { "0.0.0.0".to_string() } else { "127.0.0.1".to_string() };
    }
    if lan && (custom == "127.0.0.1" || custom == "localhost") {
        return "0.0.0.0".to_string();
    }
    custom
}

pub fn api_keys(cfg: &Json) -> Vec<String> {
    cfg.path("network.apiKeys")
        .and_then(|v| v.as_arr())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn push_opt(args: &mut Vec<String>, flag: &str, value: &str) {
    let v = value.trim();
    if !v.is_empty() {
        args.push(flag.to_string());
        args.push(v.to_string());
    }
}

fn push_num(args: &mut Vec<String>, flag: &str, cfg: &Json, key: &str, skip: f64) {
    let v = cfg.i(key, skip as i64) as f64;
    if v != skip {
        args.push(flag.to_string());
        args.push(format!("{}", cfg.i(key, skip as i64)));
    }
}

/// NInfer 引擎（infer-kvmem-server.exe）的命令行。
/// 模型是**位置参数**，不接受 -m；参数集也和 llama.cpp 完全不同，
/// 所以单独构建，绝不混入 llama 的参数。
fn build_ninfer_args(cfg: &Json, model_path: &str) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    a.push(model_path.to_string());

    a.push("--host".into());
    a.push(bind_host(cfg));
    a.push("--port".into());
    a.push(cfg.i("network.port", 9931).to_string());
    let keys = api_keys(cfg);
    if cfg.b("network.apiKeyEnabled", true) && !keys.is_empty() {
        a.push("--api-key".into());
        a.push(keys.join(","));
    }
    a.push("--no-webui".into());

    let alias = cfg.s("launch.alias", "");
    if !alias.trim().is_empty() {
        a.push("--model-id".into());
        a.push(alias.trim().to_string());
    }
    let np = cfg.i("launch.parallel", 1).clamp(1, 8);
    if np > 1 {
        a.push("--max-concurrency".into());
        a.push(np.to_string());
    }

    // KVMem：B + R 必须 <= max-context
    if cfg.b("kvmem.enabled", true) {
        a.push("--kvmem".into());
    }
    let budget = cfg.i("kvmem.budget", 32768).max(64);
    let reserve = cfg.i("kvmem.reserve", 4096).max(64);
    a.push("--kvmem-budget".into());
    a.push(budget.to_string());
    a.push("--kvmem-gen-reserve".into());
    a.push(reserve.to_string());
    let host = cfg.i("kvmem.hostMib", 8192);
    if host > 0 {
        a.push("--kvmem-host-mib".into());
        a.push(host.to_string());
    }
    let conv = cfg.i("kvmem.conversations", 1);
    if conv > 1 {
        a.push("--kvmem-sessions".into());
        a.push(conv.to_string());
    }
    let disk = cfg.i("kvmem.diskMib", 0);
    if disk > 0 {
        a.push("--kvmem-disk-mib".into());
        a.push(disk.to_string());
        let dir = cfg.s("kvmem.diskPath", "");
        if !dir.trim().is_empty() {
            a.push("--kvmem-disk-path".into());
            a.push(crate::util::absolutize(dir.trim()));
        }
    }
    let ctx = cfg.i("launch.ctxSize", 8192);
    let need = budget + reserve + 64;
    a.push("--max-context".into());
    a.push(ctx.max(need).to_string());
    a
}

/// 思考型模型自动使用的输出上限（tokens）。
/// 这类模型先写思考再写答案，上限给小了会「只有思考、没有答案」。
const THINKING_MAX_OUTPUT: i32 = 16384;

/// 判断模型是不是思考型（读 GGUF 里的 chat template 标记）。
/// 参数预览用的 "<model>" 占位符没有真实文件，一律按普通模型处理。
fn model_is_thinking(model_path: &str) -> bool {
    if model_path == "<model>" {
        return false;
    }
    crate::gguf::read(model_path)
        .map(|i| i.thinking)
        .unwrap_or(false)
}

/// KVMem 引擎（llama-kvmem-server.exe）的命令行。
/// 它只认长参数（--ctx-size / --gpu-layers / --flash-attn ...），
/// 另有一整套 --kvmem-* 控制 KV 历史在显存/内存/SSD 之间怎么放。
fn build_kvmem_args(cfg: &Json, model_path: &str) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    a.push("-m".into());
    a.push(model_path.to_string());

    let mmproj = cfg.s("launch.mmproj", "");
    let mmproj = mmproj.trim().to_string();
    if cfg.b("launch.vision", true) && !mmproj.is_empty() {
        a.push("--mmproj".into());
        a.push(crate::util::absolutize(&mmproj));
    }

    let alias = {
        let raw = cfg.s("launch.alias", "");
        if raw.trim().is_empty() {
            std::path::Path::new(model_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            raw.trim().to_string()
        }
    };
    if !alias.is_empty() {
        a.push("--alias".into());
        a.push(alias);
    }

    a.push("--ctx-size".into());
    a.push(cfg.i("launch.ctxSize", 8192).to_string());
    a.push("--gpu-layers".into());
    a.push(cfg.i("launch.nGpuLayers", 99).to_string());

    let fa = cfg.s("launch.flashAttn", "auto");
    if fa != "auto" {
        a.push("--flash-attn".into());
        a.push(fa);
    }
    a.push("--cache-type-k".into());
    a.push(cfg.s("cache.typeK", "f16"));
    a.push("--cache-type-v".into());
    a.push(cfg.s("cache.typeV", "f16"));
    if cfg.b("launch.jinja", true) {
        a.push("--jinja".into());
    }
    let np = cfg.i("launch.parallel", 1);
    if np > 1 {
        a.push("--parallel".into());
        a.push(np.to_string());
    }
    let thr = cfg.i("launch.threads", 0);
    if thr > 0 {
        a.push("--threads".into());
        a.push(thr.to_string());
    }

    // ---- KVMem 核心参数 ----
    if cfg.b("kvmem.enabled", true) {
        a.push("--kvmem".into());
    }
    let budget = cfg.i("kvmem.budget", 32768);
    if budget > 0 {
        a.push("--kvmem-budget".into());
        a.push(budget.to_string());
    }
    let reserve = cfg.i("kvmem.reserve", 4096);
    if reserve > 0 {
        a.push("--kvmem-gen-reserve".into());
        a.push(reserve.to_string());
    }
    let host = cfg.i("kvmem.hostMib", 8192);
    if host > 0 {
        a.push("--kvmem-host-mib".into());
        a.push(host.to_string());
    }
    let disk = cfg.i("kvmem.diskMib", 0);
    if disk > 0 {
        a.push("--kvmem-disk-mib".into());
        a.push(disk.to_string());
        let dir = cfg.s("kvmem.diskPath", "");
        let dir = dir.trim().to_string();
        if !dir.is_empty() {
            a.push("--kvmem-disk-path".into());
            a.push(crate::util::absolutize(&dir));
        }
    }
    let conv = cfg.i("kvmem.conversations", 1);
    if conv > 1 {
        a.push("--kvmem-conversations".into());
        a.push(conv.to_string());
    }

    // 网络与采样
    a.push("--host".into());
    a.push(bind_host(cfg));
    a.push("--port".into());
    a.push(cfg.i("network.port", 9931).to_string());
    let keys = api_keys(cfg);
    if cfg.b("network.apiKeyEnabled", true) && !keys.is_empty() {
        a.push("--api-key".into());
        a.push(keys.join(","));
    }
    a.push("--no-ui".into());
    push_f("--temp", cfg, "sampling.temp", 0.8, &mut a);
    push_f("--top-p", cfg, "sampling.topP", 0.95, &mut a);
    push_f("--min-p", cfg, "sampling.minP", 0.05, &mut a);
    push_f("--repeat-penalty", cfg, "sampling.repeatPenalty", 1.1, &mut a);
    a
}

/// 浮点参数：与默认值不同才写入
fn push_f(flag: &str, cfg: &Json, key: &str, def: f64, a: &mut Vec<String>) {
    let v = cfg.path(key).and_then(|x| x.as_f64()).unwrap_or(def);
    if (v - def).abs() > 1e-9 {
        a.push(flag.to_string());
        a.push(format!("{}", v));
    }
}

/// 依据配置与模型路径生成启动参数（按引擎分派）
pub fn build_args(cfg: &Json, model_path: &str) -> Vec<String> {
    // NInfer 引擎：模型是位置参数、参数集完全不同，单独构建
    if crate::process::Launcher::is_ninfer(cfg) {
        return build_ninfer_args(cfg, model_path);
    }
    // KVMem 引擎（llama-kvmem-server.exe）用长参数 + 一整套 --kvmem-*，
    // 和 llama-server 的短参数不兼容，单独构建
    if crate::process::Launcher::is_kvmem(cfg) {
        return build_kvmem_args(cfg, model_path);
    }
    let mut a: Vec<String> = Vec::new();
    a.push("-m".into());
    a.push(model_path.to_string());

    // 识图模型必须同时加载视觉投影文件（mmproj），否则带图片的请求会 500
    // 「开启识图」没勾选时就不带，按纯文本模型启动
    let mmproj = cfg.s("launch.mmproj", "");
    let mmproj = mmproj.trim().to_string();
    if cfg.b("launch.vision", true) && !mmproj.is_empty() {
        a.push("--mmproj".into());
        a.push(crate::util::absolutize(&mmproj));
    }

    // 别名留空时使用模型文件名，避免 API 中暴露完整路径
    let alias = {
        let raw = cfg.s("launch.alias", "");
        if raw.trim().is_empty() {
            std::path::Path::new(model_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            raw
        }
    };
    push_opt(&mut a, "--alias", &alias);
    let ctx_want = cfg.i("launch.ctxSize", 4096);
    a.push("-c".into());
    a.push(ctx_want.to_string());

    // ---- RoPE 缩放 ----
    // 把上下文开到模型原生上限之上时，裸外推会让质量明显变差；
    // 这里可以直接选 linear / yarn 让 llama.cpp 做频率缩放。
    // 倍率留 0 就按「目标上下文 ÷ 原生上下文」自动算。
    let rope = cfg.s("adv.ropeScaling", "auto");
    if rope != "auto" {
        let native = if model_path == "<model>" {
            0u32
        } else {
            crate::gguf::read(model_path)
                .map(|i| i.ctx_native)
                .unwrap_or(0)
        };
        a.push("--rope-scaling".into());
        a.push(rope.clone());
        if rope != "none" {
            let mut factor = cfg
                .path("adv.ropeScale")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if factor <= 0.0 && native > 0 && ctx_want > native as i64 {
                factor = ctx_want as f64 / native as f64;
            }
            if factor > 0.0 {
                a.push("--rope-scale".into());
                a.push(format!("{:.3}", factor));
            }
            // YaRN 需要知道模型原始训练上下文；0 = 用模型自己的。
            // 只有 yarn 才需要这个参数，linear 传了没意义
            if rope == "yarn" {
                let orig = cfg.i("adv.yarnOrigCtx", 0);
                let orig = if orig > 0 { orig } else { native as i64 };
                if orig > 0 {
                    a.push("--yarn-orig-ctx".into());
                    a.push(orig.to_string());
                }
            }
        }
    }
    a.push("-ngl".into());
    a.push(cfg.i("launch.nGpuLayers", 99).to_string());
    push_num(&mut a, "-t", cfg, "launch.threads", 0.0);
    a.push("-np".into());
    a.push(cfg.i("launch.parallel", 1).to_string());

    let fa = cfg.s("launch.flashAttn", "auto");
    if fa != "auto" {
        a.push("-fa".into());
        a.push(fa);
    }
    if cfg.b("launch.jinja", true) {
        a.push("--jinja".into());
    } else {
        a.push("--no-jinja".into());
    }
    // MTP（多 token 预测）：模型自带 NextN 层时启用，能明显提升生成速度。
    // 很多模型没有 NextN 层，这时传 --spec-type draft-mtp 会让 llama-server
    // 直接启动失败（failed to create MTP context），所以先看一眼 GGUF 头。
    if cfg.b("launch.mtp", false) {
        let has_mtp = if model_path == "<model>" {
            // 参数预览之类的场景没有真实路径，按开关显示
            true
        } else {
            crate::gguf::read(model_path)
                .map(|i| i.nextn_layers > 0)
                .unwrap_or(false)
        };
        if has_mtp {
            a.push("--spec-type".into());
            a.push("draft-mtp".into());
        }
    }

    // 输出上限：0 = 自动。思考型模型（chat template 里带 think/reasoning）
    // 需要足够的输出预算，否则容易把 token 全花在思考上、正文返回空
    let mo = cfg.i("adv.maxOutput", 0);
    let out_n: i64 = if mo > 0 {
        mo
    } else if mo == 0 && model_is_thinking(model_path) {
        THINKING_MAX_OUTPUT as i64
    } else {
        0
    };
    if out_n > 0 {
        a.push("-n".into());
        a.push(out_n.to_string());
    }

    push_num(&mut a, "-b", cfg, "launch.batchSize", 2048.0);
    push_num(&mut a, "-ub", cfg, "launch.ubatchSize", 512.0);
    let lm = cfg.s("launch.loadMode", "auto");
    if lm != "auto" {
        a.push("-lm".into());
        a.push(lm);
    }
    if cfg.b("launch.embeddings", false) {
        a.push("--embeddings".into());
    }
    if cfg.b("launch.noWarmup", false) {
        a.push("--no-warmup".into());
    }

    // 网络
    a.push("--host".into());
    a.push(bind_host(cfg));
    a.push("--port".into());
    a.push(cfg.i("network.port", 9931).to_string());
    let keys = api_keys(cfg);
    if cfg.b("network.apiKeyEnabled", true) && !keys.is_empty() {
        a.push("--api-key".into());
        a.push(keys.join(","));
    }
    push_num(&mut a, "-to", cfg, "network.timeout", 3600.0);
    let th = cfg.i("network.threadsHttp", -1);
    if th >= 0 {
        a.push("--threads-http".into());
        a.push(th.to_string());
    }
    if !cfg.b("network.builtinUi", false) {
        a.push("--no-ui".into());
    }
    if cfg.b("network.metrics", false) {
        a.push("--metrics".into());
    }
    if !cfg.b("network.slots", true) {
        a.push("--no-slots".into());
    }

    // GPU
    let sm = cfg.s("gpu.splitMode", "auto");
    if sm != "auto" {
        a.push("-sm".into());
        a.push(sm);
    }
    push_opt(&mut a, "-ts", &cfg.s("gpu.tensorSplit", ""));
    push_num(&mut a, "-mg", cfg, "gpu.mainGpu", 0.0);
    push_num(&mut a, "-ncmoe", cfg, "gpu.nCpuMoe", 0.0);
    push_opt(&mut a, "-ctk", &cfg.s("cache.typeK", "f16"));
    push_opt(&mut a, "-ctv", &cfg.s("cache.typeV", "f16"));

    // 采样默认值
    let temp = cfg.path("sampling.temp").and_then(|v| v.as_f64()).unwrap_or(0.8);
    a.push("--temp".into());
    a.push(format!("{}", temp));
    push_num(&mut a, "--top-k", cfg, "sampling.topK", 40.0);
    let topp = cfg.path("sampling.topP").and_then(|v| v.as_f64()).unwrap_or(0.95);
    a.push("--top-p".into());
    a.push(format!("{}", topp));
    let minp = cfg.path("sampling.minP").and_then(|v| v.as_f64()).unwrap_or(0.05);
    a.push("--min-p".into());
    a.push(format!("{}", minp));
    let rp = cfg
        .path("sampling.repeatPenalty")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.1);
    a.push("--repeat-penalty".into());
    a.push(format!("{}", rp));
    let seed = cfg.i("sampling.seed", -1);
    if seed >= 0 {
        a.push("-s".into());
        a.push(seed.to_string());
    }

    // 其它高级项
    let rea = cfg.s("adv.reasoning", "auto");
    if rea != "auto" {
        a.push("-rea".into());
        a.push(rea);
    }
    let rb = cfg.i("adv.reasoningBudget", -1);
    if rb >= 0 {
        a.push("--reasoning-budget".into());
        a.push(rb.to_string());
    }
    push_opt(&mut a, "--chat-template-file", &cfg.s("adv.chatTemplateFile", ""));
    push_num(&mut a, "--prio", cfg, "adv.prio", 0.0);
    let numa = cfg.s("adv.numa", "auto");
    if numa != "auto" {
        a.push("--numa".into());
        a.push(numa);
    }
    push_opt(&mut a, "--cors-origins", &cfg.s("adv.corsOrigins", ""));

    // 附加参数：按空白切分，支持引号包裹含空格的值
    for tok in split_args(&cfg.s("adv.extraArgs", "")) {
        a.push(tok);
    }
    // 自动发现的新参数：config.auto.<flag> → --flag [value]
    if let Some(Json::Obj(map)) = cfg.path("auto") {
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        for k in keys {
            let v = &map[k];
            let flag = format!("--{}", k);
            match v {
                Json::Bool(true) => a.push(flag),
                Json::Bool(false) | Json::Null => {}
                Json::Str(s) => {
                    let t = s.trim();
                    if !t.is_empty() {
                        a.push(flag);
                        a.push(t.to_string());
                    }
                }
                Json::Num(n) => {
                    a.push(flag);
                    a.push(format!("{}", *n as i64));
                }
                _ => {}
            }
        }
    }

    a
}

/// 拆分附加参数字符串，支持 "双引号" 与 '单引号'
pub fn split_args(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut has = false;
    for ch in raw.chars() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                } else {
                    cur.push(ch);
                }
            }
            None => match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                    has = true;
                }
                c if c.is_whitespace() => {
                    if has || !cur.is_empty() {
                        out.push(cur.clone());
                        cur.clear();
                        has = false;
                    }
                }
                c => {
                    cur.push(c);
                    has = true;
                }
            },
        }
    }
    if has || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// 面向用户的命令行预览（脱敏 API 密钥）
pub fn preview_args(cfg: &Json, model_path: &str) -> String {
    let args = build_args(cfg, model_path);
    let mut shown: Vec<String> = Vec::new();
    let mut mask_next = false;
    for a in args {
        if mask_next {
            shown.push("<redacted>".into());
            mask_next = false;
            continue;
        }
        if a == "--api-key" {
            mask_next = true;
        }
        shown.push(a);
    }
    shown.join(" ")
}
