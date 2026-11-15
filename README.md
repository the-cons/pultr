# pultr

*Pult* (German/Luxembourgish): a control desk or mixing console.

A card-based terminal dashboard for your machine's settings and status.
Open a terminal, press one key, and your headphones connect, your iPad becomes a second screen, or your music pauses.

```
┌ AirPods ───────────┐┌ iPad ──────────────┐┌ Battery ───────────┐
│ ● connected  84%   ││ ○ not connected    ││ 72%  charging      │
│ [a] toggle         ││ [i] connect        ││ 1:40 to full       │
└────────────────────┘└────────────────────┘└────────────────────┘
┌ Now Playing ───────────────────────────────┐┌ Output ──────────┐
│ ▶ Track – Artist           2:14 / 4:03     ││ AirPods          │
│ [space] play/pause  [n] next  [p] prev     ││ [o] switch       │
└────────────────────────────────────────────┘└──────────────────┘
```

## Highlights

- **Cards** — every feature is a card with its own status and actions. You choose which cards appear and where.
- **One-key favourites** — bind any action to a single key, usable from anywhere in the UI.
- **Scenes** — one key runs several actions ("desk mode": connect iPad, switch audio, enable Focus).
- **Templates** — start from a ready-made layout (`pultr init --template desk`).
- **Shell cards** — turn any command into a card without writing Rust.
- **Plugins** — write cards in any language via a small JSON-RPC protocol.
- **CLI mode** — every action also works headless: `pultr run airpods.toggle`.
- **macOS and Linux** — platform code is isolated behind adapters.

## Status

Pre-alpha / design phase. See [docs/roadmap.md](docs/roadmap.md).

## Documentation

| Doc | What's in it |
|---|---|
| [Vision](docs/vision.md) | Goals, non-goals, principles |
| [Architecture](docs/architecture.md) | Crates, ports & adapters, runtime model |
| [Cards](docs/cards.md) | Card model and every built-in card |
| [Configuration](docs/configuration.md) | Config file format, keys, scenes, templates |
| [Plugins](docs/plugins.md) | Shell cards and the plugin protocol |
| [Platforms](docs/platforms.md) | macOS / Linux implementation per feature |
| [Roadmap](docs/roadmap.md) | Milestones in small steps |
| [Contributing](CONTRIBUTING.md) | How to help |

## Requirements (planned)

- macOS 14+ or a modern Linux desktop (systemd, D-Bus, PipeWire)
- Optional helpers on macOS: `blueutil`, `switchaudio-osx`, `SidecarLauncher`
  — missing helpers only disable the cards that need them.

## License

MIT OR Apache-2.0
