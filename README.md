# llamaUI — a native Windows GUI for llama.cpp's `llama-server`
<img width="1182" height="920" alt="2026-11-21_055014" src="https://github.com/user-attachments/assets/980c3291-ea42-484c-8faa-c203ab0e5389" />

[简体中文](README.zh-CN.md) · English

llamaUI is a small, dependency-free Win32 desktop application that manages a local
[`llama.cpp`](https://github.com/ggml-org/llama.cpp) `llama-server` for you: pick a model,
start/stop the server, expose it on your LAN, hand out an API key, and chat in the browser.

It is written in Rust with **zero third-party crates** — every Win32 call is a hand-written
`extern "system"` binding. The result is a single ~1 MB `.exe` that needs no runtime, no
installer, and no network access to run.

```
┌──────────────────────────────────────────────────────────────┐
│ ● Running    Model: qwen3.5-9b   This run in/out: 33/32  …    │
│ http://192.168.1.10:9931/v1  PID 12345   CPU 8%  RAM …       │
├──────────────┬───────────────────────────────────────────────┤
│ Model        │ Alias / Context / GPU layers                  │
│ library      │ Threads / Slots / Flash Attention             │
│ (ownerdraw)  │ [x] Vision  [x] Jinja  [ ] LAN  [x] API key   │
│              │ …                                             │
├──────────────┴───────────────────────────────────────────────┤
│ Start / Stop · Arguments preview · Explain args · Log pane    │
└──────────────────────────────────────────────────────────────┘
```

## Features

**Model library**
- Import a folder (recursive) or re-scan all configured folders
- Each scan is a two-way sync: new files are added, vanished files are removed,
  sizes / vision sidecars / native context are refreshed
- Vision models are detected automatically (`mmproj*.gguf` sibling) and marked
- Models whose architecture is not supported by the bundled build are marked red
- Right-click a model → open its folder in Explorer, or remove it from the library
- Native context length is read straight from the GGUF header and auto-fills the context field

**Server control**
- Locates `llama-server.exe` automatically (program folder, any sub-folder, `PATH`);
  a “Browse…” button lets you point at a different location
- Start / stop, live log pane, argument preview
- Copies the LAN endpoint and API key to the clipboard in one click
- Optional LAN exposure and optional API-key enforcement (`Authorization: Bearer …`)
- Shows this run's cumulative token usage (prompt / generated), parsed from the server log

**Understanding the arguments**
- **Explain args** window: every argument actually passed, one per line, with a Chinese
  explanation and the official English help text
- Unknown/new parameters can be translated on demand by the local model itself
  (`/completion`), results cached in `locales/params.zh-CN.json` and never auto-overwritten

**Forward compatibility with llama.cpp upgrades**
- On startup llamaUI parses `llama-server --help` into a parameter catalog (cached)
- If the build changed, it reports *added / removed / changed* parameters in the log
- Parameters the new build no longer accepts are dropped before launch, so an upgrade
  does not break startup
- Parameters llamaUI does not model yet appear in an “auto-discovered” form
  (paged, filterable) and are written to `config.auto.*`

**Chat**
- Built-in web chat page (`/chat.html`) served by the panel, proxied to `/v1/chat/completions`
- Image input: paste, drag & drop, or pick a file (vision models only)
- Long messages are streamed, server-sent events handled by the panel

## Requirements

- Windows 10/11 (x64)
- A `llama.cpp` build that provides `llama-server.exe`
- Rust toolchain only if you want to build from source (GNU or MSVC both work)

## Quick start

1. Put `llama-server.exe` (and its DLLs) next to `llamaui.exe`, or in any sub-folder
   such as `llama-b13000\`, or configure the path under **Advanced → llama-server path**.
2. Launch `llamaui.exe`.
3. **Import folder** → choose a folder containing `.gguf` models (sub-folders are scanned).
4. Pick a model, adjust context length if needed, press **Start service**.
5. Copy the endpoint / API key, or press **Open chat page**.

If llamaUI cannot find `llama-server`, the log says so on the first line and points you
at **Advanced → llama-server path → Browse…**, or at downloading llama.cpp yourself.

Detailed manual: [docs/USAGE.md](docs/USAGE.md) · [中文使用说明](docs/USAGE.zh-CN.md)

## Build from source

```bash
cargo build --release
# binary: target/release/llamaui.exe  (copy next to your llama.cpp folder)
```

The build script embeds the icon, a Common-Controls v6 manifest and version info via
`windres` (`assets/app.rc`). No external crates are downloaded.

Technical documentation: [docs/TECHNICAL.md](docs/TECHNICAL.md) · [中文技术说明](docs/TECHNICAL.zh-CN.md)

## Privacy

llamaUI is fully offline by design:

- No telemetry, no analytics, no auto-update, no outbound connection except the local
  `llama-server` you launched and the browser you open yourself
- API keys are generated locally and stored only in `data/config.json`
- The repository contains **no** models, no llama.cpp binaries, no logs, no user data
  and no screenshots (screenshots would reveal local paths)

See [docs/TECHNICAL.md § Security and privacy](docs/TECHNICAL.md#security-and-privacy).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

This program links to `llama.cpp` at runtime by launching it as a separate process;
`llama.cpp` itself is distributed under the MIT license by its own authors and is **not**
included in this repository.

## Author

黑山大叔 · <https://space.bilibili.com/19428259> · <https://github.com/heiheids>
