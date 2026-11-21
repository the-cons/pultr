# Configuration

## Location

| OS | Path |
|---|---|
| macOS | `~/.config/pultr/config.toml` (XDG respected if set) |
| Linux | `$XDG_CONFIG_HOME/pultr/config.toml` → `~/.config/pultr/config.toml` |

The repo ships `config.example.toml`. Personal config (device addresses etc.) never lives in the repo.

The config is **hot-reloaded** on save. Invalid config keeps the last valid one and shows the error in a banner.

## Structure

```toml
[ui]
theme   = "default"     # default | mono | <custom theme name>
columns = 3
border  = "rounded"     # plain | rounded | double | none

[keys]                   # app-level keys (defaults shown)
palette = "/"
quit    = "q"
reload  = "r"
help    = "?"

# ── cards ──────────────────────────────────────────────

[[card]]
id     = "airpods"            # used in action addresses: airpods.toggle
type   = "bluetooth"
title  = "AirPods"
device = "aa-bb-cc-dd-ee-ff"
pos    = [0, 0]               # [column, row]
keys   = { toggle = "a" }     # global hotkeys for this card's actions

[[card]]
id     = "ipad"
type   = "sidecar"
device = "iPad"
pos    = [1, 0]
keys   = { toggle = "i" }

[[card]]
id   = "battery"
type = "battery"
pos  = [2, 0]

[[card]]
id   = "music"
type = "media"
pos  = [0, 1]
span = [2, 1]                 # columns, rows
keys = { play_pause = "space", next = "n", prev = "p" }

[[card]]
id    = "out"
type  = "audio-output"
pos   = [2, 1]
keys  = { cycle = "o" }
cycle = ["AirPods", "MacBook Pro Speakers", "Studio Display"]  # optional subset
```

### Common card fields

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Unique, used in action addresses |
| `type` | yes | Built-in type, `shell`, `scene` or `plugin` |
| `title` | no | Defaults to a type-specific title |
| `pos` | no | `[col, row]`; omitted → auto-placed in order |
| `span` | no | `[cols, rows]`, default `[1, 1]` |
| `keys` | no | `action = "key"` map of global hotkeys |
| `refresh` | no | Override refresh interval, e.g. `"5s"` |
| `hidden` | no | Keep the card (for CLI/scenes) but don't show it |

Card-specific fields are documented per type in [cards.md](cards.md).

Cards with pos are placed first. The rest fill free cells left to right, top to bottom, in config order; auto-placement never goes back to fill an earlier gap. Overlapping cards and cards outside the grid are errors at startup. ui.columns is 1–12; the grid has at most 12 rows.

### Key syntax

`"a"`, `"A"` (shift), `"ctrl-a"`, `"alt-1"`, `"space"`, `"enter"`, `"f5"`.
Conflicting keys are reported at load time.

Built-in keys can’t be used by cards: q, esc, ctrl-c (quit), arrows and h j k l (move). A card key must name an action the card offers. Pressing it runs the action and focuses the card. Rebinding the built-in keys ([keys]) isn’t supported yet.

An action can take an argument after a `:`, for actions that work on one item of a card: `keys = { "toggle:AirPods Pro" = "a" }`. TOML needs the quotes because of the `:` and the space. Everything after the first `:` is the argument, so addresses like `aa:bb:…` work.

## Scenes

A scene runs several actions in order.

```toml
[[scene]]
id    = "desk"
title = "Desk mode"
key   = "D"
steps = [
  "ipad.connect",
  "out.select:Studio Display",
  { action = "focus.on", continue_on_error = true },
  { sleep = "500ms" },
  "music.play",
]
```

- Steps are action addresses, optionally with an argument after `:`.
- A failed step stops the scene unless `continue_on_error = true`.
- A scene can appear as a card (`type = "scene"`, `scene = "desk"`) or just have a key.
- CLI: `pultr scene desk`.

## Templates

```bash
pultr init                       # interactive: pick a template
pultr init --template desk       # write config from template
pultr init --list                # list templates
```

Shipped templates (`templates/`):

| Template | Cards |
|---|---|
| `minimal` | battery, wifi, one bluetooth device |
| `desk` | sidecar, bluetooth (headphones, keyboard, mouse), audio output, media, scene "desk" |
| `music` | big media card, audio output, volume, playlists |
| `dev` | system, battery, shell cards for Docker and git |

Templates contain placeholders (`device = "<your-device>"`). `pultr init` offers to fill them by listing paired Bluetooth devices and audio outputs.

Templates are marked with supported platforms; cards that aren't available on the current OS are left out with a comment.

## Themes

```toml
# ~/.config/pultr/themes/mine.toml
[colors]
border        = "gray"
border_focus  = "cyan"
ok            = "green"
active        = "blue"
warning       = "yellow"
error         = "red"
unavailable   = "dark_gray"
```

## CLI

```bash
pultr                        # open the TUI
pultr run airpods.toggle     # run one action, exit
pultr scene desk             # run a scene
pultr status [--json]        # print all card states
pultr list actions           # list every action address
pultr doctor                 # check helpers, permissions, config
pultr --mock                 # fake data, for demos/tests
pultr --config <path>
```
