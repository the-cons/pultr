# Platforms

How each port is implemented per OS. Start with shelling out to proven tools; replace with native APIs only where it's worth it (speed, fewer dependencies).

## macOS

| Port | v1 implementation | Native later | Notes |
|---|---|---|---|
| Bluetooth | `blueutil --format json` | `IOBluetooth` via `objc2` | No built-in CLI for Bluetooth |
| Audio output/input | `SwitchAudioSource` | CoreAudio | `brew install switchaudio-osx` |
| Volume | `osascript -e 'set volume …'` | CoreAudio | |
| Media | `osascript` → Music.app | MediaRemote (private) | Only Music.app via AppleScript; Spotify has its own AppleScript dictionary |
| Sidecar | `SidecarLauncher` | `SidecarCore` (private) via `objc2` | **Private API — can break on macOS updates.** Behind Cargo feature `sidecar` |
| Displays | `system_profiler SPDisplaysDataType -json` | CoreGraphics | |
| Battery | `pmset -g batt` / `battery` crate | IOKit | |
| System | `sysinfo` crate | | |
| Wi-Fi | `ipconfig getsummary en0`, `networksetup` | CoreWLAN | SSID needs Location permission on recent macOS — show "hidden" otherwise |
| Appearance | `osascript` → System Events | | |
| Focus/DND | `shortcuts run "<name>"` | | User creates the shortcut; `pultr doctor` explains |
| Keep awake | `caffeinate` child process | IOKit power assertions | |
| Brightness | — | private DisplayServices / `brightness` CLI | v1.x |

Helper tools are optional. `pultr doctor` lists which are missing and which cards that affects.

## Linux

Nearly everything goes through **D-Bus**, which makes Linux the cleaner platform.

| Port | Implementation | Notes |
|---|---|---|
| Bluetooth | BlueZ via `bluer` | Signals give push updates for free |
| Audio | PipeWire: `wpctl` (v1), native later | PulseAudio fallback: `pactl` |
| Media | MPRIS via `mpris` / `zbus` | Works with Spotify, mpv, browsers, most players |
| Displays | `wlr-randr` (wlroots), `gnome-randr`/Mutter D-Bus, `xrandr` (X11) | Fragmented — detect compositor at startup |
| Second screen | — | No Sidecar equivalent; `displays` card instead |
| Battery | UPower D-Bus | `/sys/class/power_supply` fallback |
| System | `sysinfo` crate | |
| Wi-Fi | NetworkManager D-Bus | `iwd` fallback later |
| Appearance | `org.freedesktop.appearance` portal / `gsettings` | Desktop-dependent |
| Focus/DND | `makoctl`, `dunstctl`, GNOME `gsettings` | Desktop-dependent |
| Keep awake | `systemd-inhibit` | |
| Brightness | `brightnessctl` / logind `SetBrightness` | |

## Capability detection

Each adapter's `platform()` probes once at startup:

```rust
pub fn platform() -> Platform {
    Platform {
        bluetooth: which("blueutil").ok().map(|p| Arc::new(BlueUtil::new(p)) as _),
        // …
    }
}
```

Unavailable → `None` → cards show `unavailable` with a reason (e.g. "install blueutil").

## Supported versions (target)

- macOS 14 Sonoma and newer, Apple Silicon and Intel.
- Linux with systemd, D-Bus and PipeWire (Fedora 39+, Ubuntu 24.04+, Arch). GNOME, KDE and wlroots compositors.

## CI

- `macos-latest` and `ubuntu-latest` runners: build, `clippy -D warnings`, tests.
- Adapter integration tests are `#[ignore]` by default (need real hardware); run locally with `cargo test -- --ignored`.
