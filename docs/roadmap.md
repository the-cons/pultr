# Roadmap

Small steps. Each milestone ends with something you can run. Tick boxes as you go.

## M0 — Skeleton (an evening or two)

- [x] Reserve the name: publish an empty `pultr` crate on crates.io, create the GitHub repo
- [x] Create repo, licence (MIT OR Apache-2.0), `.gitignore`, README
- [x] Cargo workspace with empty crates: `platform`, `core`, `cards`, `mock`, `macos`, `app`
- [x] GitHub Actions: build + clippy + test on `macos-latest` and `ubuntu-latest`
- [x] `pultr --version` runs

**Done when:** CI is green.

## M1 — A box on screen

- [x] ratatui + crossterm app loop, quit with `q`
- [x] Fixed grid layout with hard-coded placeholder cards
- [x] Focus movement with arrows / `hjkl`, highlighted border

**Done when:** you can move between empty cards.

## M2 — Card trait + mock data

- [x] `CardView`, `State`, `Card` trait in `core`
- [x] Per-card refresh task + channel to UI
- [x] `Battery` port in `platform`, mock implementation
- [x] `battery` card rendering mock data
- [x] First `insta` snapshot test

**Done when:** `pultr --mock` shows a live-updating battery card.

## M3 — Config

- [x] Load `config.toml` (serde), XDG/`directories` path
- [x] Cards created from config via registry (`type` → constructor)
- [x] `pos`, `span`, auto-placement
- [x] Per-card `keys`, global key dispatch, conflict detection
- [x] `config.example.toml` in repo

**Done when:** layout and keys come from your own config file.

## M4 — First real macOS feature: Bluetooth ⭐

- [ ] `Bluetooth` port
- [ ] macOS adapter using `blueutil`
- [ ] `bluetooth` card: status + toggle
- [ ] Action tasks with "working…" state and timeout
- [ ] Unavailable state when `blueutil` is missing

**Done when:** one key connects your headphones. *This is the moment the tool becomes useful.*

## M5 — Sidecar ⭐

- [ ] `Displays` port with optional second-screen capability
- [ ] macOS adapter via `SidecarLauncher`, behind feature `sidecar`
- [ ] `sidecar` card: connect/disconnect

**Done when:** one key puts the iPad next to your Mac.

## M6 — Audio & media

- [ ] `Audio` port + `SwitchAudioSource` adapter; `audio-output`, `volume` cards
- [ ] `Media` port + Music.app AppleScript adapter; `media` card

## M7 — Status cards

- [ ] `system` card (`sysinfo`)
- [ ] `wifi` card
- [ ] real `battery` adapter on macOS

## M8 — CLI & palette

- [ ] Action addresses `<card>.<action>`
- [ ] `pultr run`, `pultr status --json`, `pultr list actions`
- [ ] `/` fuzzy action palette (`nucleo`)
- [ ] Enter → card detail view with item list

## M9 — Shell cards, scenes, templates

- [ ] `shell` card type
- [ ] Scenes + `pultr scene`
- [ ] `templates/` + `pultr init --template`
- [ ] `pultr doctor`

## 🎉 v0.1 release

- [ ] Hot reload, themes
- [ ] README with `vhs` demo GIF (recorded with `--mock`)
- [ ] `cargo-dist` release builds + Homebrew tap
- [ ] Announce

## M10 — Plugins

- [ ] Plugin host: spawn, JSON-RPC over stdio, timeouts, restart backoff
- [ ] `plugin` card type
- [ ] Example plugin in Python, one in Rust
- [ ] Protocol documented and versioned (`protocol = 1`)

## M11 — Linux

- [ ] `linux` crate: BlueZ (`bluer`), MPRIS, PipeWire, UPower, NetworkManager
- [ ] `displays` card
- [ ] Platform-marked templates
- [ ] Linux release binaries

## v1.x backlog

Appearance · Focus/DND · brightness · keep-awake · VPN · clipboard · native macOS adapters (`objc2`) · plugin installer · Rust plugin SDK.

## Open questions

- ~~Final project name~~ → **pultr** (from *Pult*: control desk). Still to reserve: GitHub repo, crates.io, Homebrew tap.
- Should global hotkeys outside the terminal be in scope (e.g. a small daemon), or left to skhd / Raycast / Shortcuts calling `pultr run`?
- Do scenes need conditions ("only if iPad is nearby")?
