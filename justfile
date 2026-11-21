# List recipes
default:
    @just --list

# Run the TUI (pass args: just run --version)
run *args:
    cargo run -p pultr -- {{ args }}

# Format code
fmt:
    cargo fmt --all

# Everything CI checks: run this before pushing
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace
    # Full verify only works once the version is new, see `release`
    cargo package --workspace --locked --no-verify

# Run tests only
test *args:
    cargo nextest run --workspace {{ args }}

# Release a new version: just release 0.1.0
release version:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ "$(git branch --show-current)" == "main" ]] || { echo "not on main"; exit 1; }
    [[ -z "$(git status --porcelain)" ]] || { echo "working tree not clean"; exit 1; }
    git pull --ff-only
    just check
    trap 'echo "release failed before pushing; reset with: git reset --hard origin/main"' ERR
    cargo set-version --workspace {{ version }}
    cargo check -q
    git diff --quiet || git commit -am "chore: release v{{ version }}"
    cargo package --workspace --locked   # full verify of exactly the commit that gets tagged
    git tag "v{{ version }}"
    git push
    git push origin "v{{ version }}"
    echo "Released v{{ version }}: follow it with: gh run watch"

# Start a feature branch from fresh main: just start feat/bluetooth-card
start branch:
    git switch main
    git pull --ff-only
    git switch -c "{{ branch }}"

# Run checks, push the branch and open a PR
pr:
    @git diff --quiet && git diff --cached --quiet || { echo "uncommitted changes: commit first"; exit 1; }
    just check
    git push -u origin HEAD
    gh pr create --fill

# Wait for CI, squash-merge the PR, back to updated main
merge:
    #!/usr/bin/env bash
    set -euo pipefail
    # A fresh PR has no checks for a few seconds; wait for them to appear (max ~60 s).
    for _ in {1..20}; do
        [[ "$(gh pr checks --json name --jq length 2>/dev/null || echo 0)" -gt 0 ]] && break
        echo "waiting for CI to start…"
        sleep 3
    done
    gh pr checks --watch --fail-fast
    gh pr merge --squash --delete-branch
    git switch main
    git pull --ff-only
    git fetch --prune

# Run tests, then review any changed snapshots
snap:
    cargo insta test --workspace --review --test-runner nextest
