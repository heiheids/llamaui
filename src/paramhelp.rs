//! llama-server 参数说明（中英对照）
//! 依据随附的 llama-b12549 官方 `llama-server --help` 文案整理，
//! 用于「启动参数预览 → 参数解读」窗口。

pub struct Item {
    /// 该参数的所有写法（短/长），用于匹配
    pub flags: &'static [&'static str],
    /// 中文解读
    pub zh: &'static str,
    /// 英文（摘自官方 help，做了压缩）
    pub en: &'static str,
}

pub const ITEMS: &[Item] = &[
    Item {
        flags: &["-m", "--model"],
        zh: "要加载的模型文件路径（.gguf）。",
        en: "path to the model file",
    },
    Item {
        flags: &["--mmproj"],
        zh: "视觉投影文件（多模态模型专用）。识图模型必须带上它，否则发送图片会报 image input is not supported。",
        en: "path to a multimodal projector file (required for vision models)",
    },
    Item {
        flags: &["-a", "--alias", "--alias"],
        zh: "模型别名，接口里用这个名字调用；留空则用模型文件名。",
        en: "set an alias for the model (used as the model name in the API)",
    },
    Item {
        flags: &["-c", "--ctx-size"],
        zh: "上下文窗口大小（token 数）。需要容纳提示词 + 历史 + 生成内容；智能体框架通常要 67000 以上。",
        en: "size of the prompt context (default: 0 = loaded from model)",
    },
    Item {
        flags: &["-ngl", "--gpu-layers", "--n-gpu-layers"],
        zh: "放到显存里的层数。99 表示尽量全部放 GPU；显存不够时调小。",
        en: "max. number of layers to store in VRAM ('auto', 'all' or a number)",
    },
    Item {
        flags: &["-t", "--threads"],
        zh: "生成阶段使用的 CPU 线程数。0 表示由程序自动决定。",
        en: "number of CPU threads to use during generation",
    },
    Item {
        flags: &["-tb", "--threads-batch"],
        zh: "批处理/提示词处理阶段使用的 CPU 线程数。",
        en: "number of threads to use during batch and prompt processing",
    },
    Item {
        flags: &["-np", "--parallel"],
        zh: "并发槽位数（同时能处理几个请求）。1 表示一次只处理一个。",
        en: "number of server slots (default: -1 = auto)",
    },
    Item {
        flags: &["-fa", "--flash-attn"],
        zh: "Flash Attention：on / off / auto。开启可显著降低长上下文显存占用，auto 由程序按模型决定。",
        en: "set Flash Attention use ('on', 'off', or 'auto')",
    },
    Item {
        flags: &["--jinja", "--no-jinja"],
        zh: "使用模型自带的 Jinja 对话模板（推荐开启，否则工具调用、思考模式等可能不生效）。",
        en: "enable Jinja chat template (--no-jinja to disable)",
    },
    Item {
        flags: &["-lm", "--load-mode"],
        zh: "模型加载模式，auto 表示自动选择（取决于显存与模型大小）。",
        en: "model loading mode (default: auto)",
    },
    Item {
        flags: &["--embeddings"],
        zh: "以向量（embedding）模式运行，此时不产生对话文本。",
        en: "run in embedding mode only",
    },
    Item {
        flags: &["--no-warmup"],
        zh: "跳过启动时的预热推理，启动更快，但第一个请求会稍慢。",
        en: "skip warmup after loading",
    },
    Item {
        flags: &["--host"],
        zh: "监听地址。127.0.0.1 只允许本机访问；0.0.0.0 开放到局域网。",
        en: "ip address to listen on (default: 127.0.0.1)",
    },
    Item {
        flags: &["--port"],
        zh: "监听端口，接口地址就是 http://地址:端口/v1。",
        en: "port to listen on (default: 8080)",
    },
    Item {
        flags: &["--api-key"],
        zh: "接口密钥。设置后请求必须带 Authorization: Bearer <密钥>，否则返回 401。",
        en: "API key(s) required to access the server",
    },
    Item {
        flags: &["-to", "--timeout"],
        zh: "服务端读写超时（秒）。长文本生成建议保持较大值，例如 3600。",
        en: "server read/write timeout in seconds (default: 3600)",
    },
    Item {
        flags: &["--threads-http"],
        zh: "HTTP 处理线程数（并发连接数相关）。-1 表示自动。",
        en: "number of threads used to process HTTP requests",
    },
    Item {
        flags: &["--ui", "--no-ui"],
        zh: "是否启用 llama.cpp 自带的网页聊天界面。本程序用 --no-ui 关闭，改用自己打开浏览器聊天页。",
        en: "enable or disable the built-in web UI",
    },
    Item {
        flags: &["--metrics"],
        zh: "暴露 /metrics 监控指标（Prometheus 格式）。",
        en: "enable prometheus /metrics endpoint",
    },
    Item {
        flags: &["--slots", "--no-slots"],
        zh: "是否暴露 /slots 接口（查看每个并发槽位状态）。",
        en: "enable or disable the /slots endpoint",
    },
    Item {
        flags: &["-sm", "--split-mode"],
        zh: "多卡切分方式：none（只用一个 GPU）/ layer（按层切分）/ row（按行切分）/ tensor。",
        en: "how to split the model across GPUs",
    },
    Item {
        flags: &["-ts", "--tensor-split"],
        zh: "各显卡的显存分配比例，逗号分隔，例如 0.7,0.3。",
        en: "fraction of the model to offload to each GPU",
    },
    Item {
        flags: &["-mg", "--main-gpu"],
        zh: "主显卡序号（从 0 开始），在 split-mode 为 none 时决定用哪张卡。",
        en: "the GPU to use for the model / for intermediate results",
    },
    Item {
        flags: &["--temp", "--temperature"],
        zh: "采样温度。越低越稳定保守（0.1~0.3），越高越发散（0.8~1.2）。",
        en: "sampling temperature (default: 0.8)",
    },
    Item {
        flags: &["--top-p"],
        zh: "核采样阈值：只从累计概率前 top-p 的候选里取词，0.95 是常用值。",
        en: "top-p sampling (default: 0.95)",
    },
    Item {
        flags: &["--top-k"],
        zh: "只从概率最高的前 K 个候选里取词。40 是常用值，0 表示关闭。",
        en: "top-k sampling (default: 40, 0 = disabled)",
    },
    Item {
        flags: &["--min-p"],
        zh: "最小概率阈值：低于「最高概率 × min-p」的候选直接丢弃，0.05 是常用值。",
        en: "min-p sampling (default: 0.05)",
    },
    Item {
        flags: &["--repeat-penalty"],
        zh: "重复惩罚系数。大于 1 抑制重复（常用 1.1），1.0 表示不惩罚。",
        en: "penalize repeated tokens (default: 1.0)",
    },
    Item {
        flags: &["-s", "--seed"],
        zh: "随机种子。-1 表示每次随机；固定数值可复现同一输出。",
        en: "RNG seed (-1 = random)",
    },
    Item {
        flags: &["-b", "--batch-size"],
        zh: "逻辑批大小：一次最多处理的 token 数。2048 是默认值。",
        en: "logical maximum batch size (default: 2048)",
    },
    Item {
        flags: &["-ub", "--ubatch-size"],
        zh: "物理批大小：真正一次送进计算图的数量。默认 512，显存紧张时调小。",
        en: "physical maximum batch size (default: 512)",
    },
    Item {
        flags: &["-ctk", "--cache-type-k"],
        zh: "K 缓存的数据类型（f16 / q8_0 / q4_0 …）。改小可省显存，但可能影响质量。",
        en: "KV cache data type for K (default: f16)",
    },
    Item {
        flags: &["-ctv", "--cache-type-v"],
        zh: "V 缓存的数据类型（f16 / q8_0 / q4_0 …）。改小可省显存，但可能影响质量。",
        en: "KV cache data type for V (default: f16)",
    },
    Item {
        flags: &["--prio"],
        zh: "进程/线程优先级：-1 低、0 普通、1 中、2 高、3 实时。",
        en: "set process/thread priority (-1..3)",
    },
    Item {
        flags: &["--numa"],
        zh: "NUMA 策略，多路 CPU 服务器上用于优化内存访问；普通台式机保持 auto 即可。",
        en: "NUMA optimization strategy",
    },
    Item {
        flags: &["-ncmoe", "--n-cpu-moe"],
        zh: "把前 N 层的 MoE 专家权重留在内存（CPU）里，缓解显存不足，适合 MoE 大模型。",
        en: "keep MoE weights of the first N layers on the CPU",
    },
    Item {
        flags: &["-rea", "--reasoning"],
        zh: "是否启用思考（推理）模式：on / off / auto。auto 由对话模板判断。",
        en: "use reasoning/thinking in the chat ('on', 'off', or 'auto')",
    },
    Item {
        flags: &["--reasoning-budget"],
        zh: "思考过程的 token 预算：-1 不限制，0 立即结束思考，N>0 为上限。",
        en: "token budget for thinking (-1 = unrestricted, 0 = end immediately)",
    },
    Item {
        flags: &["--reasoning-format"],
        zh: "思考内容返回格式：none 混在正文里、deepseek 放到 reasoning_content 字段、deepseek-legacy 两者都带。",
        en: "how thought tags are parsed and returned",
    },
    Item {
        flags: &["--chat-template-file"],
        zh: "自定义聊天模板文件（Jinja），用于模型没自带模板的情况。",
        en: "path to a custom chat template file",
    },
    Item {
        flags: &["--cors-origins"],
        zh: "允许跨域访问的来源列表（浏览器直连时用），默认不开放。",
        en: "CORS allowed origins",
    },
    Item {
        flags: &["-n", "--predict", "--n-predict"],
        zh: "最多生成多少 token，-1 表示不限制（直到自然结束）。",
        en: "number of tokens to predict (-1 = infinity)",
    },
    Item {
        flags: &["--keep"],
        zh: "从开头保留多少个 token 不被裁掉，用于长对话裁剪策略。",
        en: "number of tokens to keep from the initial prompt",
    },
    Item {
        flags: &["--no-mmap", "--mlock"],
        zh: "内存映射相关：--no-mmap 禁用 mmap（加载慢但省内存），--mlock 锁定内存防止被换出。",
        en: "memory mapping / locking options",
    },
    Item {
        flags: &["-cb", "--cont-batching"],
        zh: "连续批处理：多个请求可以边生成边拼接处理，提升并发吞吐。",
        en: "enable continuous batching",
    },
];

/// 按 flag 查解读（忽略大小写，兼容 `--flag=value` 写法）
pub fn lookup(arg: &str) -> Option<&'static Item> {
    let name = arg.split('=').next().unwrap_or(arg);
    ITEMS
        .iter()
        .find(|it| it.flags.iter().any(|f| f.eq_ignore_ascii_case(name)))
}
