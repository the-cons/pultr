# Cards

A **card** is the unit of functionality: a box on screen with status and actions.

## Anatomy

```
┌ Title ─────────────────── state dot ┐
│ status lines / gauge / list         │
│                                     │
│ [key] action  [key] action          │  ← footer: card's actions
└─────────────────────────────────────┘
```

- **Focus**: arrows / `hjkl` move between cards; the focused card has a highlighted border.
- **Enter** goes inside a card that has a list (e.g. all paired devices): `j`/`k` or ↑/↓ pick a row (`▸`), `space`/`Enter` runs the row's action, `Esc` goes back. Moving sideways (`h`/`l`) also leaves.
- **Global keys**: actions with a `key` in config work from anywhere, focused or not.
- **States**: `ok` · `active` · `warning` · `error` · `unavailable` (greyed out, with the reason).

## Built-in cards

Legend: **v1** = first release, **v1.x** = soon after, **later** = maybe plugin.

### Connectivity

| Card | Shows | Actions | Release |
|---|---|---|---|
| `bluetooth` | Radio on/off, every paired device as a row (connected?) | power, on, off; per device: toggle, connect, disconnect | v1 |
| `wifi` | SSID, signal, IP | on/off | v1 |
| `vpn` | Tailscale / WireGuard status | up/down | v1.x |

### Displays

| Card | Shows | Actions | Release |
|---|---|---|---|
| `sidecar` (macOS) | iPad available / connected | connect, disconnect | v1 |
| `displays` | Connected displays, resolution | toggle external (Linux), mirror | v1.x |
| `brightness` | Display / keyboard brightness | up, down | v1.x |
| `appearance` | Dark / light | toggle | v1.x |

### Audio & media

| Card | Shows | Actions | Release |
|---|---|---|---|
| `audio-output` | Current output device | cycle, pick from list | v1 |
| `audio-input` | Current input device, mute | cycle, mute | v1 |
| `volume` | Volume gauge | up, down, mute | v1 |
| `media` | Now playing, position | play/pause, next, prev, play playlist | v1 |

### System

| Card | Shows | Actions | Release |
|---|---|---|---|
| `battery` | %, charging, time remaining | — | v1 |
| `system` | CPU, memory, disk, uptime | — | v1 |
| `power` | Keep-awake state | keep awake, sleep, lock | v1.x |
| `focus` | DND / Focus mode | toggle | v1.x |
| `clipboard` | Last N entries | copy again | later |

### Generic

| Card | Shows | Actions | Release |
|---|---|---|---|
| `shell` | Output of a command | any commands | v1 |
| `scene` | Scene name, last run | run | v1 |
| `plugin` | Whatever the plugin returns | whatever the plugin declares | v1.x |

## Card details

### `bluetooth`

```toml
[[card]]
id   = "bluetooth"
type = "bluetooth"
keys = { power = "B", "toggle:AirPods Pro" = "a" }
```

- Every paired device is a row. Inside the card (`Enter`), `space` toggles the selected device.
- `toggle`, `connect` and `disconnect` take the device after a `:`: its name as the card shows it (any case) or its address (`aa-bb-…` or `AA:BB:…`).
- `connect` switches Bluetooth on first if it's off. `connect`/`disconnect` do nothing if the device is already there.

## Ideas for later / plugins

Homebrew or apt updates · Docker containers · Git repo status · Pomodoro / timers · next calendar event · weather · Home Assistant · Spotify · cloud cost/status.

## Writing a built-in card

1. Add a module in `crates/cards/src/<name>.rs` implementing `Card`.
2. Take only the port(s) it needs (`Arc<dyn Bluetooth>`), never an adapter.
3. If the port is `None`, return a `CardView` with `State::Unavailable` and a reason.
4. Register the `type` string in the card registry.
5. Add a snapshot test using the `mock` adapter.
6. Document it in this file.
