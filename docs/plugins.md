# Plugins

There are two ways to extend the app without touching its source:

1. **Shell cards** — a few lines of TOML. Good for simple status + actions.
2. **Plugin cards** — a separate program speaking JSON-RPC over stdio. Any language.

Both implement the same `Card` trait internally, so they look and behave exactly like built-in cards.

## Why this design

| Option | Decision |
|---|---|
| Shell commands | ✅ simplest possible extension |
| External process + JSON-RPC (like LSP, Nushell plugins) | ✅ language-agnostic, crash-isolated |
| WASM (extism / wasmtime) | ❌ for now — plugins here mostly call system commands, which a sandbox makes harder. Revisit if sharing untrusted plugins becomes common |
| Embedded Lua | ❌ ties authors to one language |
| Native dynamic libraries | ❌ Rust has no stable ABI; a crash kills the app |

---

## Shell cards

```toml
[[card]]
id      = "docker"
type    = "shell"
title   = "Docker"
status  = "docker ps -q | wc -l | xargs printf '%s running'"
refresh = "10s"
actions = [
  { id = "stop", label = "stop all", run = "docker stop $(docker ps -q)", confirm = true },
  { id = "prune", label = "prune",   run = "docker system prune -f" },
]
keys = { stop = "S" }
```

- `status` output: each stdout line is a card line. Exit code ≠ 0 → card state `error`.
- Optional **structured output**: if `status_format = "json"`, stdout is parsed as a `CardView` (see below).
- `confirm = true` asks before running.
- Commands run via `sh -c` with a timeout (default 5 s, `timeout = "30s"` to override).

---

## Plugin cards

### Installation & discovery

```
~/.config/pultr/plugins/<name>/
  plugin.toml       # manifest
  <executable>      # anything runnable
```

```toml
# plugin.toml
name        = "pomodoro"
version     = "0.1.0"
description = "Pomodoro timer card"
command     = ["python3", "main.py"]
platforms   = ["macos", "linux"]    # optional
protocol    = 1
```

Use it in config:

```toml
[[card]]
id     = "pomo"
type   = "plugin"
plugin = "pomodoro"
pos    = [0, 2]
keys   = { start = "t" }
settings = { work_minutes = 25 }   # passed to the plugin as-is
```

One process is started per plugin **card** (so two cards of the same plugin are independent).

### Transport

- JSON-RPC 2.0, **one JSON object per line** on stdin/stdout.
- stderr is captured into the app log.
- Requests from the app have an `id`; notifications from the plugin don't.

### Lifecycle

```
app                                  plugin
 │── initialize {protocol, settings} ──▶│
 │◀──────── result {card description} ──│
 │── refresh ──────────────────────────▶│   (only if plugin wants polling)
 │◀──────────────── result {CardView} ──│
 │◀──── notification: update {CardView}│   (push, any time)
 │── action {id, arg?} ────────────────▶│
 │◀──────────────── result {ok|error} ──│
 │── shutdown ─────────────────────────▶│
```

### Messages

**`initialize`** (app → plugin)
```json
{"jsonrpc":"2.0","id":1,"method":"initialize",
 "params":{"protocol":1,"os":"macos","card_id":"pomo","settings":{"work_minutes":25}}}
```
Result:
```json
{"title":"Pomodoro",
 "actions":[{"id":"start","label":"start"},{"id":"stop","label":"stop"}],
 "refresh_ms":null}
```
`refresh_ms: null` → push-only; a number → app sends `refresh` at that interval.

**`refresh`** (app → plugin) → result is a `CardView`.

**`update`** (plugin → app, notification)
```json
{"jsonrpc":"2.0","method":"update","params":{ ...CardView... }}
```

**`action`** (app → plugin)
```json
{"jsonrpc":"2.0","id":7,"method":"action","params":{"id":"start","arg":null}}
```
Result: `{}` on success, or a JSON-RPC error with a human-readable `message`.

**`shutdown`** (app → plugin) — plugin should exit within 1 s, otherwise it is killed.

### CardView (JSON)

```json
{
  "state": "active",
  "lines": [
    {"text": "Focus — 18:42 left", "style": "bold"},
    {"text": "Session 2 of 4", "style": "dim"}
  ],
  "gauge": 0.25,
  "items": [
    {"id": "s1", "text": "Session 1 ✓"}
  ]
}
```
- `state`: `ok` | `active` | `warning` | `error` | `unavailable`
- `style`: `normal` | `bold` | `dim` | `ok` | `warning` | `error`
- All fields except `state` are optional.

### Robustness rules (enforced by the host)

- Plugin crashes → card shows `error`, host restarts it with backoff (1 s, 5 s, 30 s, then gives up until reload).
- Every request has a timeout (default 5 s).
- Malformed output → logged, card shows `error`, plugin keeps running.
- Updates are rate-limited (max ~20/s per card).

### Security

Plugins run as your user with full access, like any program you install. Only install plugins you trust. The app never runs plugins it wasn't configured to run.

### Minimal plugin (Python)

```python
#!/usr/bin/env python3
import json, sys

def send(msg): print(json.dumps(msg), flush=True)

count = 0
for line in sys.stdin:
    req = json.loads(line)
    m, rid = req.get("method"), req.get("id")
    if m == "initialize":
        send({"jsonrpc":"2.0","id":rid,"result":{
            "title":"Counter","actions":[{"id":"inc","label":"+1"}],"refresh_ms":None}})
        send({"jsonrpc":"2.0","method":"update","params":{"state":"ok","lines":[{"text":"0"}]}})
    elif m == "action":
        count += 1
        send({"jsonrpc":"2.0","id":rid,"result":{}})
        send({"jsonrpc":"2.0","method":"update","params":{"state":"ok","lines":[{"text":str(count)}]}})
    elif m == "shutdown":
        break
```

### Future

- `pultr plugin add <git-url>` installer.
- A Rust SDK crate (`pultr-plugin`) with typed messages.
- Plugin-defined settings schema for validation and `pultr init` prompts.
