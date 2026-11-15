# Contributing

Thanks for your interest! The project is early; issues and ideas are as welcome as code.

## Ways to help

- **Templates** — add a layout to `templates/` (mark supported platforms).
- **Plugins** — build one and link it in an issue; good ones get listed in the README.
- **Platform adapters** — especially Linux desktop variants (KDE, GNOME, wlroots).
- **Built-in cards** — see [docs/cards.md](docs/cards.md) for the checklist.

## Development

```bash
cargo run -p pultr -- --mock      # run with fake data, no real devices needed
cargo test                       # unit + snapshot tests
cargo insta review               # review changed snapshots
cargo clippy --all-targets -- -D warnings
cargo fmt
```

## Rules of the road

- `core` and `cards` must never depend on `macos` / `linux` — only on `platform`.
- Every new port method needs a `mock` implementation.
- Platform-specific code goes behind `#[cfg(target_os = "…")]` inside its adapter crate only.
- New cards need a snapshot test and an entry in `docs/cards.md`.
- Keep PRs small; one feature per PR.

## Commit style

Conventional commits: `feat(cards): add volume card`, `fix(macos): handle missing blueutil`.
