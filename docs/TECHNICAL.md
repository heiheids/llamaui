# llamaUI — Technical Notes

> 简体中文：[TECHNICAL.zh-CN.md](TECHNICAL.zh-CN.md) · Overview: [../README.md](../README.md)

llamaUI is a Win32 desktop front-end for `llama.cpp`'s `llama-server`. It does not link
against llama.cpp; it launches `llama-server.exe` as a child process, manages its
configuration and exposes a small local HTTP panel plus a chat page.

- Language: Rust 2021, **no third-party crates** (`[dependencies]` is empty)
- Size: ~10.6 kLOC across 24 modules
- Artifact: one self-contained `.exe` (~1 MB with `opt-level="s"`, LTO, `panic="abort"`)

---

## 1. Why zero dependencies

The program is a GUI tool that must stay tiny, start instantly and work on an offline
machine. Pulling in a GUI toolkit (egui/iced/tao+wry) would add tens of megabytes of
dependencies and a build chain that needs network access. Instead every Win32 call is a
hand-written `extern "system"` declaration in `src/gui/win.rs` (user32, gdi32, comctl32,
shell32, ole32, comdlg32, kernel32). The cost is that widgets are owner-drawn, which is
also what gives the app its uniform flat look.

---

## 2. Module map

| Module | Lines | Responsibility |
|---|---:|---|
| `src/main.rs` | 223 | entry point: single-instance, startup migrations, panel server thread, message loop |
| `src/gui/win.rs` | 1001 | raw Win32 bindings, window helpers, DPI scaling, dialogs, clipboard |
| `src/gui/mod.rs` | 554 | `Gui` state, window procedure, subclass procs (EDIT / model listbox / scan listbox) |
| `src/gui/layout.rs` | 736 | control creation and geometry, `make_*` factories, theme wiring |
| `src/gui/actions.rs` | 2756 | every user action: import/scan, start/stop, save config, advanced page, param help, translate |
| `src/gui/theme.rs` | 389 | palette, owner-draw button/listbox/model-line painting, frame drawing |
| `src/settings.rs` | 592 | field descriptors (`FIELDS`), defaults, `build_args()`, preview |
| `src/paramspec.rs` | 595 | `llama-server --help` parser, catalog cache, version diff, launch-time validation |
| `src/paramhelp.rs` | 253 | hand-written Chinese/English descriptions for known parameters |
| `src/translate.rs` | 319 | local-model translation of parameter docs + cache |
| `src/process.rs` | 529 | exe discovery, child process lifecycle, log ring buffer, status JSON |
| `src/models.rs` | 475 | model library, recursive scanning, disk sync, GGUF sidecar detection |
| `src/gguf.rs` | 148 | minimal GGUF header reader (architecture, context length, name) |
| `src/store.rs` | 174 | `config.json` / `models.json` persistence and defaults merge |
| `src/json.rs` | 405 | JSON value, parser, serializer (no serde) |
| `src/i18n.rs` | 63 | locale loading from `locales/*.json` |
| `src/util.rs` | 306 | paths (relative/absolute), time, PID lookup, key generation |
| `src/sysinfo.rs` | 194 | CPU / RAM / GPU status via Win32 + `nvidia-smi` |
| `src/web/mod.rs` | 90 | panel HTTP server thread |
| `src/web/http.rs` | 189 | request parsing, response writing, chunked encoding |
| `src/web/api.rs` | 414 | panel API and `/v1` proxy to llama-server |
| `src/web/client.rs` | 144 | minimal HTTP client (used for the proxy and health checks) |
| `src/web/assets.rs` | 34 | embedded chat assets (HTML/CSS/JS) |
| `src/shutdown.rs` | 51 | graceful shutdown of the child process |

---

## 3. Threads and state

```
main thread (GUI)                panel thread            child process
─────────────────                ────────────            ─────────────
window proc / WM_TIMER(1 s)  →   GET /api/status         llama-server.exe
Mutex<App> (try_lock)        →   GET /api/models              │
background translate thread  →   POST /v1/chat/completions   │
                                 GET /chat.html        ←──────┘
```

- The whole application state lives in `Gui`, reachable through a `OnceLock<Mutex<Gui>>`
  and the helper `with_gui()`. **`with_gui` uses `try_lock().ok()`**: setting control text
  re-enters the window procedure (`EN_CHANGE` etc.), and a blocking lock would dead-lock the
  GUI thread. Nested echoes are simply dropped.
- The panel server runs on its own thread and shares `App` (the model library + config)
  behind the same `Mutex`.
- Translation runs on a detached thread; progress is published in a small `Mutex<TrState>`
  and polled by the 1-second timer, so the UI never blocks on the model.

---

## 4. Rendering and layout

- `WM_ERASEBKGND` paints the background: header bar, body, separators. For secondary
  windows it fills the flat background and asks `theme::draw_field_frames()` to draw a
  1 px frame around every child EDIT/LISTBOX.
- **Important**: the parent must not fill a child control's whole rectangle, or the child's
  text is erased. `frame_cb` fills only the *ring* between the control's window rect and its
  client rect, then strokes the frame.
- Buttons and the model listbox are owner-drawn (`WM_DRAWITEM`), which is what makes the flat
  rectangular style possible. The listbox draws two lines per row: name and size, plus a
  checkbox glyph for vision models and red text for unsupported architectures.
- Single-line EDIT text is vertically centred by subclassing the control and insetting the
  client rect in `WM_NCCALCSIZE` (6 px top/bottom, 1 px sides) plus `EM_SETMARGINS` for the
  left padding. Win32 EDIT has no vertical alignment property; this is the standard trick.
- All geometry is expressed in 96-DPI units and scaled with `layout::scale()`/`s()`.
  The manifest declares per-monitor DPI awareness.

---

## 5. Model library

- `Library::import_paths()` walks folders recursively (depth ≤ 4, ≤ 4000 files), skipping
  `.`-prefixed directories, `target`, `node_modules`. `mmproj*.gguf` files are excluded from
  the model list and instead attached to their sibling models.
- `Library::sync_with_disk()` makes scanning a two-way sync: entries whose file vanished are
  removed **only if the parent folder is still reachable** (so an unplugged drive does not
  wipe the library), and surviving entries get size / mmproj / vision / native-context
  refreshed.
- Native context comes from `gguf.rs`, which parses just enough of the GGUF metadata block
  (`general.architecture`, `<arch>.context_length`).
- Paths inside the program folder are stored **relative** (`util::relativize`), so the whole
  folder can be moved or copied to another machine.

---

## 6. Process lifecycle

`Launcher::resolve_exe()` search order: configured path → program folder and up to 4 parents
→ any sub-folder (this is what makes versioned `llama-bXXXXX\` folders work) → `bin\`
sub-folders → `PATH`.

On start:

1. `settings::build_args()` produces the raw argument vector
2. `paramspec::filter_args()` validates it against the parsed `--help` catalog
   (drops parameters this build no longer accepts, logs unknown ones)
3. if something already listens on the port, it is terminated first
4. the child is spawned with `CREATE_NO_WINDOW`; stdout+stderr are pumped into a ring buffer
   and mirrored to the log pane
5. phase transitions are exposed as JSON through `/api/status` and shown in the header

`Launcher::adopt()` handles the case where a previous instance left a server running:
if the port answers, the running process is adopted instead of starting a second one.

---

## 7. Parameter pipeline and forward compatibility

This is the part that keeps the app working across llama.cpp upgrades.

```
FIELDS (settings.rs)  ──┐
config.auto.*         ──┼→ build_args() → filter_args() → spawn
adv.extraArgs         ──┘         ▲
                                   │
                     paramspec::current()  ← llama-server --help
```

- On startup `paramspec::discover()` runs `llama-server --version` and `--help`, parses the
  help text into `{flag, aliases, value placeholder, kind, allowed values, default, env,
  section, description}` and caches it in `data/param-cache.json`.
- The cache is keyed by **exe size + mtime**, so parsing happens only when the binary changes.
- `parse_help()` is alignment-based (the help text uses a fixed column layout), tolerant of
  continuation lines, and never drops a line it cannot parse.
- `diff()` compares the previous catalog with the new one and reports added / removed /
  changed parameters in the log.
- `filter_args()` removes parameters that llamaUI knows but the new build dropped — the
  difference between “an upgrade prints a warning” and “an upgrade makes the server refuse to
  start”.
- Parameters llamaUI does not model appear in the **auto-discovered** window and are stored
  as `config.auto.<flag>`, then appended to the argument vector at the end.

---

## 8. Translation cache

`locales/params.zh-CN.json`:

```json
{
  "meta": { "version": "…", "model": "local", "updatedAt": 1763… },
  "params": {
    "--cors-methods": { "hash": "0e2a4946", "zh": "允许 CORS 的 HTTP 方法列表…", "by": "local" }
  }
}
```

- Keyed by flag; `hash` is an FNV-1a of the English description, so a re-translation happens
  only when llama.cpp changes that text.
- `by` is `local` (model) or `human` (hand-written). **Human entries are never overwritten.**
- Translation goes through `/completion` (raw completion), not `/chat/completions`: thinking
  models spend their whole token budget on `reasoning_content` and return an empty
  `content`, which breaks JSON extraction.
- The model output is parsed strictly first; if it was truncated by `n_predict`, a tolerant
  scanner extracts complete `"flag": "text"` pairs and keeps what is valid.

---

## 9. Panel HTTP server

- Small hand-written HTTP/1.1 server (no async runtime): one thread, thread-per-connection.
- Routes: `/api/status`, `/api/models`, `/api/scan`, `/api/start`, `/api/stop`,
  `/api/config`, `/v1/*` (proxied to llama-server, `Authorization` injected),
  `/chat.html` + assets.
- `MAX_BODY` is 16 MB, which is what allows image payloads through the proxy.
- The proxy forwards `messages` untouched so multimodal content arrays pass through.

---

## 10. Build pipeline

- `build.rs` runs `windres` over `assets/app.rc` to embed the icon (16/24/32/48/64 BMP +
  128/256 PNG), the Common-Controls v6 manifest and `VERSIONINFO`. The resource file is
  UTF-8 with BOM and `#pragma code_page(65001)`; `windres` is also given
  `--codepage 65001`, otherwise non-ASCII version strings are mojibake.
- `Cargo.toml` release profile: `opt-level="s"`, `lto=true`, `codegen-units=1`,
  `strip=true`, `panic="abort"` → small, fast-starting binary.
- No build-time network access, no `build.rs` dependency on external crates.

---

## 11. i18n

- UI strings live **only** in `locales/zh-CN.json` and `locales/en-US.json`; source code
  contains no user-visible literals (a couple of log lines written in Chinese for the
  operator are the exception, they are diagnostic).
- `i18n::t(lang, key)` falls back to the key itself when a translation is missing, so adding a
  language never breaks the UI.
- Values may be plain strings or arrays (used for combo box option labels), which is why
  `FIELDS` stores *label keys* rather than literal text.

---

## 12. Security and privacy

- **No telemetry.** The only outbound connections are to `127.0.0.1:<port>` (your own
  llama-server) and whatever browser you open yourself.
- **API keys** are generated locally (`sk-llamaui-` + 16 random hex chars from the OS RNG)
  and stored in plain text in `data/config.json`. They are redacted in the UI, in the
  argument preview and in the explain-args window.
- **LAN exposure is opt-in.** The server binds `127.0.0.1` unless “Allow LAN access” is
  enabled, and API-key enforcement is a separate opt-in.
- **The repository contains no user data**: `data/`, `logs/`, `models/`, llama.cpp binaries,
  build output, local toolchains and screenshots are all excluded by `.gitignore`.
- **No hardcoded absolute paths** in the source: all paths are derived from
  `util::base_dir()` (the executable's folder) or from configuration.

---

## 13. Known limitations

- Windows only (Win32 API throughout); the model-management half is portable in principle
  but the GUI is not.
- The `--help` parser assumes llama.cpp's aligned help layout. If upstream changes the
  format drastically, the parser degrades to “raw text entries” rather than crashing.
- Semantic changes that are not visible in the help text (e.g. a flag keeping its name but
  changing units) cannot be detected automatically.
- Screenshots are deliberately not shipped in the repository, since they reveal local paths.
