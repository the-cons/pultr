# Vision

## Problem

Everyday machine toggles — connect headphones, put the iPad next to the laptop, switch audio output, pause music — are buried in menus and settings panes. Each one is several clicks. Status information (battery, connected devices, what's playing) is scattered across the menu bar.

## Goal

One terminal UI where the things you do every day are **one keystroke away**, and the things you want to know are **visible at a glance**.

## Principles

1. **Few keystrokes.** The common path is a single key. Anything else is reachable through the fuzzy action palette (`/`).
2. **Configurable, not configured-for-you.** The layout is the user's. Defaults and templates exist to get started, not to dictate.
3. **Degrade gracefully.** A missing helper tool, unsupported OS feature or broken plugin disables one card — never the app.
4. **Core stays OS-agnostic.** All platform code lives in adapters. The core never calls a system command.
5. **Small core, open edges.** Built-in cards cover the essentials; shell cards and plugins cover everything else. The author should not have to build every feature.
6. **Headless parity.** Every action available in the TUI is available on the CLI, so it can be bound to hotkeys, Shortcuts or scripts.
7. **Fast.** Starts in well under 100 ms; status refresh never blocks input.

## Non-goals

- Replacing System Settings or a full Linux settings app. Only frequently used settings belong here.
- A GUI. This is a terminal app.
- Windows support (for now). The architecture allows it later.
- Running as root or changing security-sensitive settings (privacy permissions, SIP, firewall rules).

## Target users

- Developers who live in the terminal on macOS or Linux.
- People with multi-device desk setups (headphones, tablets, external displays) who switch often.
- Tinkerers who want to build their own status cards.
