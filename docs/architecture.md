# Architecture

## Overview

The app follows **Ports and Adapters** (hexagonal architecture):

- **Core** — cards, layout, config, keybindings, rendering. Knows nothing about operating systems.
- **Ports** — Rust traits describing what the core needs from the system (`Bluetooth`, `Audio`, `Media`, …).
- **Adapters** — per-OS implementations of those traits (`macos`, `linux`, `mock`).
- **Card sources** — built-in cards, shell cards, and plugin cards all implement the same `Card` trait.

```
              ┌──────────────────────────────┐
              │            app               │  main binary: CLI parsing, wiring
              └──────┬────────────────┬──────┘
                     │                │
           ┌─────────▼──────┐  ┌──────▼─────────┐
           │      core      │  │    plugins     │  shell cards, JSON-RPC bridge
           │ cards, layout, │  │  (impl Card)   │
           │ config, input  │  └────────────────┘
           └─────────┬──────┘
                     │ uses traits only
           ┌─────────▼──────┐
           │    platform    │  ports (traits) + shared types
           └──┬──────┬───┬──┘
              │      │   │
         ┌────▼─┐ ┌──▼──┐ ┌▼─────┐
         │macos │ │linux│ │ mock │  adapters
         └──────┘ └─────┘ └──────┘
```

## Workspace layout

```
crates/
  platform/   # traits (ports) and shared types: BtDevice, Track, AudioDevice…
  core/       # Card trait, card registry, layout engine, config, keymap, UI
  cards/      # built-in card implementations (bluetooth, media, battery…)
  plugins/    # shell card + plugin process host
  macos/      # macOS adapter
  linux/      # Linux adapter
  mock/       # fake adapter for tests, demos and screenshots
  app/        # binary: `pultr`
templates/    # starter configs: desk.toml, music.toml, minimal.toml, dev.toml
examples/
  plugins/    # example plugins (Python, shell, Rust)
docs/
```

Dependency rule: arrows only point **downwards**. `core` and `cards` depend on `platform`, never on `macos`/`linux`.

## Ports

Each port is a trait. Every field on `Platform` is an `Option`, because not every feature exists everywhere.

```rust
#[async_trait]
pub trait Bluetooth: Send + Sync {
    async fn powered(&self) -> Result<bool>;
    async fn set_powered(&self, on: bool) -> Result<()>;
    async fn devices(&self) -> Result<Vec<BtDevice>>;   // paired + connection state
    async fn connect(&self, id: &DeviceId) -> Result<()>;
    async fn disconnect(&self, id: &DeviceId) -> Result<()>;
}

pub struct Platform {
    pub bluetooth:  Option<Arc<dyn Bluetooth>>,
    pub audio:      Option<Arc<dyn Audio>>,
    pub media:      Option<Arc<dyn Media>>,
    pub display:    Option<Arc<dyn Displays>>,      // incl. Sidecar on macOS
    pub power:      Option<Arc<dyn Power>>,         // battery, keep-awake, sleep
    pub network:    Option<Arc<dyn Network>>,       // Wi-Fi
    pub system:     Option<Arc<dyn SystemInfo>>,    // CPU, mem, disk, uptime
    pub appearance: Option<Arc<dyn Appearance>>,    // dark mode
    pub focus:      Option<Arc<dyn Focus>>,         // DND / Focus
}
```

Wiring happens once, in `app`:

```rust
#[cfg(target_os = "macos")]
fn platform() -> Platform { pultr_macos::platform() }

#[cfg(target_os = "linux")]
fn platform() -> Platform { pultr_linux::platform() }
```

`--mock` swaps in `pultr_mock::platform()` on any OS.

Adapters detect capabilities at startup (e.g. "is `blueutil` on PATH?") and return `None` for anything unavailable.

## Card trait

```rust
#[async_trait]
pub trait Card: Send + Sync {
    fn id(&self) -> &str;
    fn title(&self) -> &str;
    fn actions(&self) -> Vec<ActionSpec>;            // id, label, default key
    fn refresh_interval(&self) -> Option<Duration>;  // None = push-only
    async fn refresh(&self) -> Result<CardView>;
    async fn run(&self, action: &str) -> Result<()>;
}
```

`CardView` is **structured content**, not raw terminal drawing:

```rust
pub struct CardView {
    pub state: State,            // Ok | Active | Warning | Error | Unavailable
    pub lines: Vec<Line>,        // text with optional emphasis
    pub gauge: Option<f32>,      // 0.0–1.0 progress bar
    pub items: Vec<ListItem>,    // selectable list (devices, outputs…)
}
```

The core owns rendering, so every card — built-in or plugin — looks consistent and respects the theme.

## Runtime model

- **tokio** runtime.
- One **refresh task per card**, ticking at the card's interval, sending `CardView`s to the UI over an `mpsc` channel.
- Push-capable cards (plugins, D-Bus signals on Linux) send updates whenever they like.
- **Actions** are spawned as tasks; the card shows a "working…" state until done, then refreshes immediately.
- The **UI loop** (ratatui + crossterm) only reads the latest views and handles input; it never awaits a system call.
- Every system call has a **timeout** (default 3 s). A timeout marks the card as `Error`, nothing else.

## Action addressing

Every action has a stable address: `<card-id>.<action-id>`, e.g. `airpods.toggle`, `music.next`.
Used by keybindings, scenes and the CLI (`pultr run airpods.toggle`).

## Error handling

- `anyhow` at the edges, typed errors (`thiserror`) in ports.
- Card errors are shown inside the card with a short message; full detail goes to the log (`~/.local/state/pultr/pultr.log`, or `~/Library/Logs/pultr/` on macOS).

## Testing

- `mock` adapter returns scripted data → snapshot tests of rendered cards (`insta`).
- Adapter tests run only on their OS (`#[cfg(target_os)]`) and are kept thin.
- Config parsing and scene expansion are pure and unit-tested.

## Key crates

| Purpose | Crate |
|---|---|
| TUI | `ratatui`, `crossterm` |
| Async | `tokio`, `async-trait` |
| Config | `serde`, `toml`, `directories` |
| CLI | `clap` |
| Fuzzy palette | `nucleo` |
| System info | `sysinfo`, `battery` |
| Linux D-Bus | `zbus`, `bluer` (BlueZ), `mpris` |
| macOS native (later) | `objc2`, `objc2-io-bluetooth` |
| Logging | `tracing`, `tracing-appender` |
| Snapshots | `insta` |
