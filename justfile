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
    git diff --quiet || git commit -am "chore: release v{{version}}"
    git tag "v{{version}}"
    git push
    git push origin "v{{version}}"
    echo "Released v{{version}}: follow it with: gh run watch"

# Start a feature branch from fresh main: just start feat/bluetooth-card
start branch:
    git switch main
    git pull --ff-only
    git switch -c "{{branch}}"

# Run checks, push the branch and open a PR
pr:
    just check
    git push -u origin HEAD
    gh pr create --fill

# Wait for CI, squash-merge the PR, back to updated main
merge:
    gh pr checks --watch --fail-fast
    gh pr merge --squash --delete-branch
    git switch main
    git pull --ff-only
    git fetch --prune
