# List recipes
default:
    @just --list

# Run the TUI (pass args: just run --version)
run *args:
    cargo run -p pultr -- {{args}}

# Format code
fmt:
    cargo fmt --all

# Everything CI checks: run this before pushing
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo package -p pultr --locked

# Run tests only
test:
    cargo test --workspace

# Review changed insta snapshots (once tests use insta)
snap:
    cargo insta review

# Release a new version: just release 0.1.0
release version:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ "$(git branch --show-current)" == "main" ]] || { echo "not on main"; exit 1; }
    [[ -z "$(git status --porcelain)" ]] || { echo "working tree not clean"; exit 1; }
    git pull --ff-only
    cargo set-version --workspace {{version}}
    cargo check -q
    just check
    git commit -am "chore: release v{{version}}"
    git tag "v{{version}}"
    git push
    git push origin "v{{version}}"
    echo "Released v{{version}}: follow it with: gh run watch"
