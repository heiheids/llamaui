# llamaUI — llama.cpp `llama-server` 的原生 Windows 图形管理器

简体中文 · [English](README.md)

llamaUI 是一个零依赖的 Win32 桌面程序，用来管理本机 [`llama.cpp`](https://github.com/ggml-org/llama.cpp)
的 `llama-server`：选模型、启停服务、开放到局域网、下发 API 密钥，并在浏览器里直接聊天。

用 Rust 写成，**不引入任何第三方 crate**，所有 Win32 调用都是手写的 `extern "system"` 绑定。
产物是单个约 1 MB 的 `.exe`，不需要运行时、不需要安装、不联网也能跑。

## 功能

**模型库**
- 「导入文件夹」（递归扫描）与「扫描目录」
- 每次扫描都是双向同步：新增文件入列、文件消失的条目自动移除，
  大小 / 视觉投影 / 原生上下文一并刷新
- 自动识别识图模型（同目录 `mmproj*.gguf`）并打勾标记
- 当前 llama.cpp 构建不支持的架构会标红（例如 `unknown model architecture`）
- 右键模型 → 「打开所在目录」/「从模型库移除」
- 直接从 GGUF 头读取原生上下文长度，自动填入上下文输入框

**服务控制**
- 自动定位 `llama-server.exe`（程序目录、任意子目录、`PATH`），也可用「浏览…」指定别处
- 启停、实时日志窗格、启动参数预览
- 一键复制局域网接口地址与 API 密钥
- 可选开放局域网、可选强制 API 密钥（`Authorization: Bearer …`）
- 顶部显示本次运行的累计 token 用量（输入 / 输出），从服务日志解析

**看懂参数**
- **参数解读** 窗口：把本次实际传入的参数逐条列出，配中文解读与官方英文原文
- 没写过的参数可以点「生成中文解读」，用**本地模型自己翻译**，结果缓存在
  `locales/params.zh-CN.json`；人工填写过的条目永不被覆盖

**跟随 llama.cpp 版本演进**
- 启动时解析 `llama-server --help` 生成参数表并缓存
- 检测到版本变化时，日志报告 *新增 / 移除 / 说明变更* 的参数
- 新版已不接受的参数会在启动前自动剔除，避免升级后服务起不来
- 本程序尚未建模的新参数会出现在「自动发现的新参数」窗口（可翻页、可筛选），
  保存后落到 `config.auto.*` 并自动加入启动参数

**聊天**
- 面板自带网页聊天页 `/chat.html`，代理到 `/v1/chat/completions`
- 支持图片输入：粘贴、拖拽、选择文件（仅识图模型）
- 流式输出

## 环境要求

- Windows 10/11（x64）
- 一份带 `llama-server.exe` 的 `llama.cpp` 构建
- 仅编译时才需要 Rust 工具链（GNU / MSVC 均可）

## 快速开始

1. 把 `llama-server.exe` 及其 DLL 放到 `llamaui.exe` 同级目录，
   或任意子目录（如 `llama-b13000\`），或在「高级设置 → llama-server 路径」里指定。
2. 运行 `llamaui.exe`。
3. 点「导入文件夹」，选择存放 `.gguf` 的目录（含子文件夹）。
4. 选中模型，必要时调整上下文长度，点「启动服务」。
5. 复制接口地址 / 密钥，或点「打开测试聊天」。

若找不到 `llama-server`，程序会在日志第一行明确提示，并指引你去
「高级设置 → llama-server 路径 → 浏览…」指定其他位置，或自行下载 llama.cpp。

详细说明：[docs/USAGE.zh-CN.md](docs/USAGE.zh-CN.md) · [English manual](docs/USAGE.md)

## 从源码构建

```bash
cargo build --release
# 产物：target/release/llamaui.exe（复制到 llama.cpp 目录旁即可）
```

构建脚本通过 `windres`（`assets/app.rc`）嵌入图标、Common-Controls v6 清单与版本信息，
不下载任何外部依赖。

技术说明：[docs/TECHNICAL.zh-CN.md](docs/TECHNICAL.zh-CN.md) · [English](docs/TECHNICAL.md)

## 隐私

llamaUI 完全离线：

- 没有遥测、统计、自动更新；除了你自己启动的本地 `llama-server` 和你自己打开的浏览器，
  不发起任何外部连接
- API 密钥在本地生成，只存在 `data/config.json`
- 仓库里**不含**模型、不含 llama.cpp 二进制、不含日志、不含用户数据，也不含截图
  （截图会暴露本机路径）

详见 [docs/TECHNICAL.zh-CN.md § 安全与隐私](docs/TECHNICAL.zh-CN.md#安全与隐私)。

## 许可

GPL-3.0-or-later，见 [LICENSE](LICENSE)。

本程序以**独立进程**方式在运行时调用 `llama.cpp`；`llama.cpp` 由其作者以 MIT 许可发布，
本仓库不包含其任何代码或二进制。

## 作者

黑山大叔 · <https://space.bilibili.com/19428259> · <https://github.com/heiheids>
