# llamaUI — 技术说明

> English: [TECHNICAL.md](TECHNICAL.md) · 项目介绍: [../README.zh-CN.md](../README.zh-CN.md)

llamaUI 是 `llama.cpp` 的 `llama-server` 的 Win32 桌面前端。它**不与 llama.cpp 链接**，
而是把 `llama-server.exe` 当作子进程启动，管理它的配置，并额外提供一个本地 HTTP 面板和聊天页。

- 语言：Rust 2021，**零第三方 crate**（`[dependencies]` 为空）
- 规模：24 个模块，约 10600 行
- 产物：单个自包含 `.exe`（`opt-level="s"` + LTO + `panic="abort"`，约 1 MB）

---

## 1. 为什么零依赖

这是个 GUI 小工具，必须体积小、启动快、离线可用。引入 GUI 框架（egui/iced/tao+wry）会带来
几十 MB 依赖和需要联网的构建链。因此所有 Win32 调用都在 `src/gui/win.rs` 里手写
`extern "system"` 声明（user32、gdi32、comctl32、shell32、ole32、comdlg32、kernel32）。
代价是控件全部自绘 —— 这也正是界面能保持统一扁平风格的原因。

---

## 2. 模块划分

| 模块 | 行数 | 职责 |
|---|---:|---|
| `src/main.rs` | 223 | 入口：单实例、启动迁移、面板线程、消息循环 |
| `src/gui/win.rs` | 1001 | 原始 Win32 绑定、窗口助手、DPI 缩放、对话框、剪贴板 |
| `src/gui/mod.rs` | 554 | `Gui` 状态、窗口过程、子类化过程（EDIT / 模型列表 / 扫描目录列表） |
| `src/gui/layout.rs` | 736 | 控件创建与布局、`make_*` 工厂、主题接线 |
| `src/gui/actions.rs` | 2756 | 所有用户动作：导入/扫描、启停、保存配置、高级设置、参数解读、翻译 |
| `src/gui/theme.rs` | 389 | 调色板、按钮/列表/模型行自绘、边框绘制 |
| `src/settings.rs` | 592 | 字段描述（`FIELDS`）、默认值、`build_args()`、参数预览 |
| `src/paramspec.rs` | 595 | `--help` 解析、参数表缓存、版本差异、发射前校验 |
| `src/paramhelp.rs` | 253 | 已知参数的手写中英说明 |
| `src/translate.rs` | 319 | 用本地模型翻译参数说明 + 缓存 |
| `src/process.rs` | 529 | exe 定位、子进程生命周期、日志环形缓冲、状态 JSON |
| `src/models.rs` | 475 | 模型库、递归扫描、磁盘同步、mmproj 识别 |
| `src/gguf.rs` | 148 | 极简 GGUF 头解析（架构、上下文长度、名称） |
| `src/store.rs` | 174 | `config.json` / `models.json` 持久化与默认值合并 |
| `src/json.rs` | 405 | JSON 值、解析器、序列化（不用 serde） |
| `src/i18n.rs` | 63 | 从 `locales/*.json` 加载文案 |
| `src/util.rs` | 306 | 路径（相对/绝对）、时间、PID 查询、密钥生成 |
| `src/sysinfo.rs` | 194 | CPU / 内存 / 显卡状态（Win32 + `nvidia-smi`） |
| `src/web/mod.rs` | 90 | 面板 HTTP 线程 |
| `src/web/http.rs` | 189 | 请求解析、响应写出、chunked 编码 |
| `src/web/api.rs` | 414 | 面板 API 与 `/v1` 代理 |
| `src/web/client.rs` | 144 | 极简 HTTP 客户端（代理与健康检查用） |
| `src/web/assets.rs` | 34 | 内嵌聊天页资源 |
| `src/shutdown.rs` | 51 | 子进程的优雅退出 |

---

## 3. 线程与状态

```
主线程 (GUI)                    面板线程                子进程
────────────                    ────────                ──────
窗口过程 / WM_TIMER(1 秒)   →   GET /api/status         llama-server.exe
Mutex<App>（try_lock）      →   GET /api/models              │
后台翻译线程                →   POST /v1/chat/completions   │
                                GET /chat.html         ←─────┘
```

- 全部状态放在 `Gui` 里，通过 `OnceLock<Mutex<Gui>>` 与 `with_gui()` 访问。
  **`with_gui` 用的是 `try_lock().ok()`**：设置控件文本会重入窗口过程（`EN_CHANGE` 等），
  用阻塞锁会把 GUI 线程锁死，所以嵌套回声直接丢弃。
- 面板服务器在自己的线程里跑，与 GUI 共享 `App`（模型库 + 配置），同一把 `Mutex`。
- 翻译跑在独立线程，进度写进 `Mutex<TrState>`，由 1 秒定时器轮询，
  所以界面永远不会等模型。

---

## 4. 绘制与布局

- `WM_ERASEBKGND` 负责背景：顶栏、主体、分隔线。二级窗口只填底色，
  再让 `theme::draw_field_frames()` 给每个子 EDIT/LISTBOX 画 1px 细边框。
- **关键点**：父窗口绝不能填满子控件的整个矩形，否则子控件文字会被抹掉。
  `frame_cb` 只填「控件窗口矩形」与「客户区矩形」之间的**环带**，然后描边。
- 按钮与模型列表是自绘（`WM_DRAWITEM`），扁平直角风格由此而来。
  列表每行画两行文字：模型名与大小，识图模型带复选框图标，不支持的架构用红字。
- 单行 EDIT 的文字垂直居中靠子类化 + 在 `WM_NCCALCSIZE` 里收窄客户区（上下 6px、左右 1px），
  再加 `EM_SETMARGINS` 控制左边距。Win32 EDIT 没有垂直对齐属性，这是标准做法。
- 所有几何以 96 DPI 为单位书写，通过 `layout::scale()`/`s()` 缩放；
  清单声明了 per-monitor DPI 感知。

---

## 5. 模型库

- `Library::import_paths()` 递归遍历（深度 ≤ 4、文件数 ≤ 4000），跳过 `.` 开头目录、
  `target`、`node_modules`。`mmproj*.gguf` 不进入模型列表，而是挂到同目录的模型上。
- `Library::sync_with_disk()` 让扫描变成双向同步：文件消失的条目会被移除，
  **但仅当父目录仍可访问**（移动硬盘拔掉不会清空模型库）；存活的条目会刷新
  大小 / mmproj / 识图标记 / 原生上下文。
- 原生上下文来自 `gguf.rs`：只解析 GGUF 元数据块里够用的部分
  （`general.architecture`、`<arch>.context_length`）。
- 程序目录内的路径以**相对路径**保存（`util::relativize`），整个目录可以整体搬走。

---

## 6. 进程生命周期

`Launcher::resolve_exe()` 的搜索顺序：配置路径 → 程序目录及向上 4 层 → **任意子目录**
（这就是 `llama-bXXXXX\` 这种带版本号目录能自动识别的原因）→ `bin\` 子目录 → `PATH`。

启动流程：

1. `settings::build_args()` 生成原始参数数组
2. `paramspec::filter_args()` 用 `--help` 参数表校验
   （剔除本版本已不接受的参数，未知参数记日志）
3. 端口若已被占用，先结束占用者
4. 以 `CREATE_NO_WINDOW` 启动子进程，stdout+stderr 泵入环形缓冲并镜像到日志窗格
5. 阶段变化通过 `/api/status` 以 JSON 暴露，并显示在顶栏

`Launcher::adopt()` 处理「上次异常退出、服务还在跑」的情况：端口能应答就接管，
不再启动第二个进程。

---

## 7. 参数流水线与版本自适应

这是本程序能跨 llama.cpp 版本继续工作的核心。

```
FIELDS (settings.rs)  ──┐
config.auto.*         ──┼→ build_args() → filter_args() → 启动
adv.extraArgs         ──┘         ▲
                                   │
                     paramspec::current()  ← llama-server --help
```

- 启动时 `paramspec::discover()` 执行 `llama-server --version` 与 `--help`，
  把帮助文本解析成 `{flag, 别名, 取值占位符, 类型, 可选值, 默认值, 环境变量, 小节, 说明}`，
  缓存在 `data/param-cache.json`。
- 缓存以**exe 大小 + 修改时间**为键，只有二进制变了才重新解析。
- `parse_help()` 基于列对齐（帮助文本是固定列布局），能拼接续行，
  解析不了的行**原样保留**而不是丢弃。
- `diff()` 对比上一版参数表，在日志里报告新增 / 移除 / 说明变更。
- `filter_args()` 剔除「本程序认识、但新版已删除」的参数 ——
  这决定了升级后是「日志里一条提示」还是「服务直接起不来」。
- 本程序未建模的参数出现在**自动发现**窗口，落值到 `config.auto.<flag>`，
  在参数数组末尾追加。

---

## 8. 翻译缓存

`locales/params.zh-CN.json`：

```json
{
  "meta": { "version": "…", "model": "local", "updatedAt": 1763… },
  "params": {
    "--cors-methods": { "hash": "0e2a4946", "zh": "允许 CORS 的 HTTP 方法列表…", "by": "local" }
  }
}
```

- 按 flag 作键；`hash` 是英文说明的 FNV-1a，所以只有 llama.cpp 改了那段文字才会重译。
- `by` 为 `local`（模型译）或 `human`（人工写）。**人工条目永不被覆盖。**
- 翻译走 `/completion`（原生补全）而不是 `/chat/completions`：
  思考型模型会把 token 全花在 `reasoning_content` 上、`content` 为空，导致 JSON 抽取失败。
- 模型输出先按严格 JSON 解析；若被 `n_predict` 截断，
  用容错扫描器把完整的 `"flag": "译文"` 对抠出来，能救多少救多少。

---

## 9. 面板 HTTP 服务

- 手写 HTTP/1.1 服务（没有异步运行时）：单线程 accept，每连接一个线程。
- 路由：`/api/status`、`/api/models`、`/api/scan`、`/api/start`、`/api/stop`、
  `/api/config`、`/v1/*`（代理到 llama-server 并注入 `Authorization`）、
  `/chat.html` 及静态资源。
- `MAX_BODY` 为 16 MB，这是图片请求能通过代理的原因。
- 代理不改写 `messages`，所以多模态内容数组可以原样透传。

---

## 10. 构建流程

- `build.rs` 调 `windres` 编译 `assets/app.rc`，嵌入图标（16/24/32/48/64 BMP + 128/256 PNG）、
  Common-Controls v6 清单与 `VERSIONINFO`。资源文件是带 BOM 的 UTF-8 并声明
  `#pragma code_page(65001)`，`windres` 另传 `--codepage 65001`，
  否则非 ASCII 的版本字符串会变成乱码。
- `Cargo.toml` 的 release 配置：`opt-level="s"`、`lto=true`、`codegen-units=1`、
  `strip=true`、`panic="abort"` —— 体积小、启动快。
- 构建期不访问网络，`build.rs` 不依赖任何外部 crate。

---

## 11. 多语言

- 界面文案**只**存在于 `locales/zh-CN.json` 与 `locales/en-US.json`，
  源码里没有面向用户的硬编码字符串
  （个别写给操作者看的诊断日志是例外，它们是给排查用的）。
- `i18n::t(lang, key)` 在缺翻译时回退成 key 本身，因此加语言不会把界面弄坏。
- 值可以是字符串或数组（下拉框选项用），所以 `FIELDS` 里存的是**文案键**而不是字面文本。

---

## 12. 安全与隐私

- **无遥测。** 唯一的对外连接是 `127.0.0.1:<端口>`（你自己启动的 llama-server）
  和你自己打开的浏览器。
- **API 密钥**在本地生成（`sk-llamaui-` + 16 位系统随机十六进制），
  明文存在 `data/config.json`。在界面、参数预览、参数解读窗口里都会打码显示。
- **局域网开放是可选项。** 默认监听 `127.0.0.1`，只有勾选「允许局域网访问」才改成
  `0.0.0.0`；强制密钥校验是另一个独立选项。
- **仓库不含任何用户数据**：`data/`、`logs/`、`models/`、llama.cpp 二进制、构建产物、
  本地工具链、截图全部被 `.gitignore` 排除。
- **源码里没有硬编码绝对路径**：所有路径都来自 `util::base_dir()`（可执行文件所在目录）
  或配置。

---

## 13. 已知局限

- 仅支持 Windows（全程 Win32 API）。模型管理那半理论上可移植，GUI 不可。
- `--help` 解析假设 llama.cpp 保持列对齐的帮助格式；若上游大改格式，
  解析器会退化成「原样文本条目」而不是崩溃。
- 帮助文本里看不出来的语义变更（比如参数名没变但单位变了）无法自动识别。
- 仓库刻意不附带截图，因为截图会暴露本机路径。
