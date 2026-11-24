# llamaUI — User Manual

> 简体中文：[USAGE.zh-CN.md](USAGE.zh-CN.md) · Overview: [../README.md](../README.md)

Everything below follows the on-screen layout. All paths are placeholders.

---

## 1. First run

1. Put `llamaui.exe` next to your `llama.cpp` build, or in any folder.
2. Double-click it. No console window appears.
3. On start it looks for `llama-server.exe` in this order:
   1. the configured **llama-server path** (if set and still a file)
   2. the program folder and up to 4 parent folders
   3. **any sub-folder** of those (so versioned folders like `llama-b12549\`, `llama-b13000\` all match)
   4. a `bin\` sub-folder of those
   5. the system `PATH`
4. If nothing is found, the first log line says so and points at
   **Advanced → llama-server path → Browse…**, or at downloading llama.cpp yourself.
5. If found, the log shows `llama-server: <full path>` and its `--help` is parsed into a
   parameter catalog.

---

## 2. Model library (left)

### Import and scan

| Action | Meaning |
|---|---|
| **Import folder** | Recursively scan (≤ 4 levels, ≤ 4000 files) a folder for `.gguf` files, and register that folder in the scan list |
| **Scan folders** | Re-sync every folder in the scan list |

Scanning is a **two-way sync**; the log states the outcome:

```
scan finished: +2 added, -1 removed, 3 updated (total 10)
  removed: some-deleted-model
```

- `+N added` — new model files
- `-M removed` — the file is gone, so the entry is dropped. If its **parent folder is also
  inaccessible** (unplugged drive, offline share), the entry is kept to avoid data loss
- `K updated` — file size, sibling `mmproj`, vision flag or native context changed

`mmproj*.gguf` files are never listed as models; they are treated as the vision sidecar of
the models in the same folder.

### Markers

- ✅ checkbox icon — a vision sidecar was found, this is a vision model
- **Red text** — the current llama.cpp build does not support this architecture
  (log shows `unknown model architecture: 'xxx'`). Only a newer llama.cpp build or a
  different model helps; no setting fixes it
- Long names scroll in place

### Right-click menu

- **Open containing folder** — reveals and selects the `.gguf` in Explorer
- **Remove from library** — removes the entry only, the file stays on disk

---

## 3. Common settings (right)

| Field | Notes |
|---|---|
| **Model alias** | The model name used by the API. Empty = file name. Short names are friendlier for agent frameworks |
| **Context size (tokens)** | Usually the model's native maximum, auto-filled from the GGUF header. Agent frameworks often need ≥ 67000. Smaller saves VRAM |
| **Auto-fit to VRAM (--fit)** | Let llama.cpp decide layers and context from the free device memory; it really does estimate KV, compute buffers and per-layer weights. On this 22 GB card with ub 64 it picked 339200 tokens. When on, Context size and GPU layers are ignored |
| **VRAM headroom (MiB)** | How much VRAM `--fit` leaves unused, default 1024; try 256 to squeeze out more context |
| **Minimum fitted context** | Lowest context `--fit` may shrink to before it errors |
| **Context checkpoints** | Context snapshots kept per slot (`-ctxcp`, default 32, 0 disables) so long documents can roll back without re-prefill |
| **Prompt cache (MiB)** | Memory cap for the prompt cache (`-cram`, default 8192, -1 unlimited, 0 disables) |
| **Cache reuse chunk** | Minimum chunk reused from cache (`--cache-reuse`, 0 disables, 256 typical); llama.cpp disables it automatically when a vision projector is loaded || **MoE expert VRAM cache** | With experts in RAM, cache hot ones in VRAM (`--moe-cache-mib`); 0 disables || **Context follows model** | **Off by default.** Context is remembered per model: checked = that model's native value (capped at 262144); typed manually = your value is remembered; neither = **65536** (64K) by default, so a model never blows up VRAM just because nobody looked at the setting |
| **GPU layers** | 99 = put as much as possible on the GPU; lower it when VRAM is short |
| **CPU threads** | Threads for generation; 0 = auto |
| **Slots** | How many requests are handled concurrently; 1 = serial |
| **Flash Attention** | `auto` / `on` / `off`; enabling saves VRAM on long contexts |
| **Engine** | `llama.cpp` / `KVMem` / `NInfer`. Probed at startup, so engines that are not installed never appear |
| **Enable vision** | Adds `--mmproj` to the launch arguments. It re-syncs automatically per model, depending on whether that model has a sidecar |
| **Enable Jinja template** | Recommended; without it tool calls and thinking mode may not work |
| **Allow LAN access** | Listens on `0.0.0.0`; the top bar then shows the LAN URL and can copy it |
| **Require API key** | Requests must send `Authorization: Bearer <key>` |
| **Custom bind address / port** | Defaults `127.0.0.1` / `9931`. **Copy local address** next to it copies `http://127.0.0.1:<port>/v1` |
| **API keys** | One per line, the first is used. Buttons: **Copy**, **New key** |

The hint line under the form shows where the context value came from (auto-following the
model maximum, or manually set) and the current vision state.

**Save settings** writes the form back to `data/config.json`.

---

## 4. Start, stop, logs

- **Start service** — builds the arguments, launches `llama-server`, streams its log into
  the bottom-right pane
- **Stop service** — terminates the child process
- Switching models while running does **not** stop the server; the UI warns that the previous
  model is still loaded, and you press **Stop service** yourself (so a running conversation
  is never interrupted)
- **Clear log** — clears the pane only

The header shows: phase, current model, **this run's cumulative token usage (in/out)** and
uptime. Token usage is accumulated by parsing llama-server's own timing lines, so requests
from *any* client (the built-in chat page, your agent framework, curl…) are counted, and it
resets on every start.

---

### Data monitor

The **Data monitor** button in the header opens a separate window with live numbers for the
current run (closing it only stops sampling, the server keeps running):

- **GPU**: utilization, VRAM, temperature, power (via NVML; shows "unavailable" when the driver does not expose it)
- **Live · prompt**: phase, prompt tokens, prompt speed, KV cache hit
- **Live · decode**: generated tokens, output speed, remaining tokens, context peak
- **Request history**: prompt/output tokens, prefill and decode speed, total time, MTP acceptance

Numbers come from `GET /slots`, `GET /props` and the run log, so requests from *any* client
(the built-in chat page, your agent framework, curl) are counted. A dash means the value is
not available right now (not enabled, not exposed by the driver, or the server is not running).


## 5. Arguments preview and “Explain args”

The bottom-left pane shows the exact command line that will be passed (API key redacted).

**Explain args** opens a window that breaks it down argument by argument:

```
* -c 32768
   size of the prompt context; must hold prompt + history + output. Agent frameworks
   usually need >= 67000.
   EN: size of the prompt context (default: 0 = loaded from model)
```

- Arguments with a hand-written Chinese description show it plus the official English text
- Others show “（自动解析自当前 llama-server --help）” and the parsed English text,
  including default and allowed values
- The pane scrolls

### Generate Chinese descriptions

Press **生成中文解读**:

1. Only arguments of the current command line that have no Chinese description are processed
2. Translation is done by **your local llama-server** via `/completion` (bypassing the chat
   template and thinking mode)
3. Results are cached in `locales/params.zh-CN.json`, keyed by a hash of the English text, so
   nothing is re-translated unless llama.cpp changes the wording
4. It runs on a background thread; the window refreshes when done

To hand-edit: change that entry's `by` to `"human"`. Such entries are never overwritten.

---

## 6. Auto-discovered parameters

At the bottom of Advanced settings there is **Auto-discovered parameters N** + **Configure…**.
It opens a paged form:

- Lists parameters that the current llama-server accepts but llamaUI does not model yet
  (e.g. `--chat-template-kwargs`)
- Control type is inferred: no value → checkbox; allowed values → dropdown; otherwise text
- A **filter** box matches either the parameter name or its English description
  (try `cors`, `template`)
- **Save and close** stores values under `config.auto.<name>` and appends them to the launch
  arguments. Empty means “not used”

Parameters llamaUI already knows never appear here.

---

## 7. Chat page

**Open chat page** launches your browser at `http://127.0.0.1:<panel port>/chat.html`.

- The panel port differs from the llama-server port (defaults: panel `8790`, server `9931`;
  see `panel.port` in `data/config.json`)
- The page proxies to `llama-server`'s `/v1/chat/completions` and attaches the API key
- Images: pick a file, paste, or drag & drop (vision models only)
- Images larger than 1568 px on the long edge are downscaled
- If the model cannot accept images the page says so instead of failing silently

---

## 8. Advanced settings

Opened from the header. Three columns; the window **can be resized by dragging its edges**.
When the content does not fit, a scrollbar appears and **Back / Save** stay pinned to the bottom
edge, so they remain clickable at any screen size.

### Generation

`mmproj file`, `load mode`, `split mode`, `tensor split`, `main GPU`, `MoE CPU layers`,
`KV cache K/V type`, `batch size`, `ubatch size`, `temperature`, `Top-K`, `Top-P`, `Min-P`,
`repeat penalty`, `seed`, `read/write timeout`, `HTTP threads`

### Toggles

`built-in llama web UI`, `metrics`, `slots endpoint`, `embeddings only`, `skip warmup`,
`auto-translate docs on new version`

The last one is off by default. When enabled, and only after llama.cpp changed version,
llamaUI runs the translation once — provided the server is running and **idle**
(`/slots` shows no in-flight request). The version hash is recorded in
`app.autoTranslatedFor` so it never runs twice for the same build.

### Text

`reasoning mode`, `thinking budget`, `max output (tokens)`, `process priority`, `NUMA policy`, `custom chat template
file`, `CORS origins`, `extra command-line arguments`

**Extra command-line arguments** is the escape hatch: anything llamaUI does not know can be
typed here, space-separated, with double quotes for values containing spaces.

**Max output** defaults to 0 = auto: thinking models get 16384 tokens so they cannot spend the
whole budget on reasoning and return an empty answer; other models are left to the engine default.

### RoPE scaling

Pushing the context past what the model was trained on forces positional extrapolation and the
output quality drops sharply (llama.cpp only prints `possible training context overflow` and
starts anyway). These three fields replace plain extrapolation:

| Field | Meaning |
|---|---|
| **RoPE scaling** | `auto` / `none` / `linear` / `YaRN`. `auto` leaves the decision to the engine and the GGUF metadata |
| **RoPE scale factor** | 0 = derive it from `context / model native context` |
| **YaRN original context** | 0 = use the model's own training context; only used by YaRN |

### llama-server path

Leave empty for auto-detection, or press **Browse…** to pick the `.exe`.

### Model scan folders

A list:

- Relative paths (`models`, `llama-b12549`) are **built-in**, labeled “built-in”, and cannot
  be removed
- Absolute paths are yours (added via **Import folder** on the main page) and can be removed
  with a right-click
- **Any sub-folder containing `llama-server.exe` is added automatically**, so a llama.cpp
  upgrade with a different folder name is picked up without configuration

---

## 9. Files on disk

| File | Contents |
|---|---|
| `data/config.json` | all settings, API keys, scan folders, llama-server path |
| `data/models.json` | model library (path, size, native context, mmproj, markers) |
| `data/param-cache.json` | parameter catalog cache (invalidated by exe size + mtime; safe to delete) |
| `locales/params.zh-CN.json` | cached Chinese descriptions (editable; set `by` to `human` to lock) |
| `logs/` | server logs (if enabled) |

Deleting `data/` resets everything — library and keys included.

---

## 10. Troubleshooting

**Exits immediately / `unknown model architecture` in the log**
The current llama.cpp build cannot run that architecture. Update llama.cpp or use another
model; no setting changes this.

**Client complains about a tiny context (e.g. `4096 tokens`)**
Raise **Context size**. Agent frameworks often need ≥ 67000, and you need the VRAM/RAM for it.

**`image input is not supported`**
The model has no vision sidecar. Use a vision model, or stop sending images. The
“Enable vision” checkbox only takes effect for models that actually have an `mmproj`.

**Port already in use**
On start, llamaUI terminates whatever listens on that port and logs it; you can also change
the port.

**A parameter errors after a llama.cpp upgrade**
llamaUI drops parameters it knows about that the new build removed, and logs it. If you see
`xxx 不在当前 llama-server 参数表中`, the flag came from **Extra command-line arguments** —
delete it there.

**LAN clients cannot connect**
Enable **Allow LAN access**, and allow `llama-server.exe` plus the panel port through
Windows Firewall.
