//! Bluetooth through `blueutil` (`brew install blueutil`): macOS has no Bluetooth command of its own.

use std::{
    env,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use pultr_platform::{Bluetooth, BtDevice, DeviceId, Error, Result};
use serde::Deserialize;
use tokio::process::Command;

/// Shown in the card when `blueutil` isn't installed.
const MISSING: &str = "brew install blueutil";

pub struct BlueUtil {
    /// `None` = not installed. Every call then answers `Unavailable` with the fix.
    path: Option<PathBuf>,
}

impl BlueUtil {
    /// Looks for `blueutil` once, at startup.
    pub fn find() -> Self {
        Self { path: find_in(search_dirs()) }
    }

    /// Runs `blueutil <args>` and returns what it printed. A non-zero exit becomes `Failed`
    /// with blueutil's own message, e.g. `Device not found by name: Beats`.
    async fn run(&self, args: &[&str]) -> Result<String> {
        let path = self.path.as_ref().ok_or_else(|| Error::Unavailable(MISSING.to_string()))?;
        // kill_on_drop: when the core gives up waiting (timeout), the process goes too.
        let output = Command::new(path).args(args).kill_on_drop(true).output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stderr.is_empty() { format!("blueutil failed ({})", output.status) } else { stderr };
            return Err(Error::Failed(message));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[async_trait]
impl Bluetooth for BlueUtil {
    async fn powered(&self) -> Result<bool> {
        parse_power(&self.run(&["--power"]).await?)
    }

    async fn set_powered(&self, on: bool) -> Result<()> {
        self.run(&["--powered", if on { "1" } else { "0" }]).await.map(drop)
    }

    async fn devices(&self) -> Result<Vec<BtDevice>> {
        parse_devices(&self.run(&["--paired", "--format", "json"]).await?)
    }

    async fn connect(&self, id: &DeviceId) -> Result<()> {
        self.run(&["--connect", id.as_str()]).await.map(drop)
    }

    async fn disconnect(&self, id: &DeviceId) -> Result<()> {
        self.run(&["--disconnect", id.as_str()]).await.map(drop)
    }
}

/// `blueutil --power` prints `1` or `0`.
fn parse_power(out: &str) -> Result<bool> {
    match out.trim() {
        "1" => Ok(true),
        "0" => Ok(false),
        other => Err(Error::Parse(format!("blueutil --power: {other}"))),
    }
}

/// One entry of `blueutil --paired --format json`. It sends more fields; we only read these.
#[derive(Deserialize)]
struct RawDevice {
    address: String,
    /// `null` for devices that never told macOS their name.
    name: Option<String>,
    connected: bool,
}

fn parse_devices(out: &str) -> Result<Vec<BtDevice>> {
    let raw: Vec<RawDevice> = serde_json::from_str(out).map_err(|e| Error::Parse(format!("blueutil --paired: {e}")))?;
    let devices = raw
        .into_iter()
        .map(|d| BtDevice {
            name: d.name.unwrap_or_else(|| d.address.clone()), // no name: the address is all we have
            id: DeviceId::new(&d.address),
            connected: d.connected,
        })
        .collect();
    Ok(devices)
}

/// `$PATH`, then Homebrew's two homes: a terminal opened from somewhere odd may have a short PATH.
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = env::var_os("PATH").map(|p| env::split_paths(&p).collect()).unwrap_or_default();
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    dirs
}

/// First directory that has a `blueutil` file in it.
fn find_in(dirs: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    dirs.into_iter().map(|dir| dir.join("blueutil")).find(|path| Path::is_file(path))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn power() {
        assert!(parse_power("1\n").unwrap());
        assert!(!parse_power("0").unwrap());
        assert_eq!(parse_power("on").unwrap_err().to_string(), "unexpected output: blueutil --power: on");
    }

    /// Shaped like real blueutil output, including the fields we don't read.
    const PAIRED: &str = r#"[
        {"address":"aa-bb-cc-dd-ee-01","recentAccessDate":"2026-11-21T10:00:00+01:00","favourite":false,
         "paired":true,"connected":true,"slave":false,"RSSI":-50,"rawRSSI":-60,"name":"AirPods Pro"},
        {"address":"AA-BB-CC-DD-EE-02","recentAccessDate":null,"favourite":false,
         "paired":true,"connected":false,"name":null}
    ]"#;

    #[test]
    fn devices() {
        let devices = parse_devices(PAIRED).unwrap();
        assert_eq!(
            devices,
            [
                BtDevice { id: DeviceId::new("aa-bb-cc-dd-ee-01"), name: "AirPods Pro".to_string(), connected: true },
                BtDevice {
                    id: DeviceId::new("aa-bb-cc-dd-ee-02"),
                    name: "AA-BB-CC-DD-EE-02".to_string(),
                    connected: false
                },
            ]
        );
        assert_eq!(devices[1].id.as_str(), "aa-bb-cc-dd-ee-02"); // normalised
    }

    #[test]
    fn no_paired_devices() {
        assert!(parse_devices("[]").unwrap().is_empty());
    }

    #[test]
    fn garbage_is_a_parse_error() {
        let err = parse_devices("Bluetooth is off").unwrap_err();
        assert!(err.to_string().starts_with("unexpected output: blueutil --paired:"), "{err}");
    }

    #[test]
    fn finds_blueutil_in_the_first_dir_that_has_it() {
        let root = env::temp_dir().join(format!("pultr-find-test-{}", std::process::id()));
        let (empty, has_it) = (root.join("empty"), root.join("bin"));
        fs::create_dir_all(&empty).unwrap();
        fs::create_dir_all(&has_it).unwrap();
        fs::write(has_it.join("blueutil"), "").unwrap();

        assert_eq!(find_in([empty.clone(), has_it.clone()]), Some(has_it.join("blueutil")));
        assert_eq!(find_in([empty]), None);
        assert_eq!(find_in([root.join("does-not-exist")]), None);
    }

    #[tokio::test]
    async fn without_blueutil_everything_is_unavailable() {
        let bt = BlueUtil { path: None };
        assert!(matches!(bt.powered().await, Err(Error::Unavailable(m)) if m == MISSING));
        assert!(matches!(bt.connect(&DeviceId::new("aa")).await, Err(Error::Unavailable(_))));
    }

    #[tokio::test]
    async fn failure_carries_the_tools_message() {
        // `sh` stands in for blueutil: same contract, message on stderr, non-zero exit.
        let bt = BlueUtil { path: Some(PathBuf::from("/bin/sh")) };
        let err = bt.run(&["-c", "echo 'Device not found by name: Beats' >&2; exit 1"]).await.unwrap_err();
        assert_eq!(err.to_string(), "Device not found by name: Beats");

        let err = bt.run(&["-c", "exit 3"]).await.unwrap_err();
        assert!(err.to_string().starts_with("blueutil failed (exit status: 3"), "{err}");
    }

    /// Talks to your real Bluetooth. Run on a Mac with: `just test --run-ignored only --no-capture`
    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore = "needs blueutil and real Bluetooth"]
    async fn real_blueutil() {
        let bt = BlueUtil::find();
        assert!(bt.path.is_some(), "blueutil not found");
        let powered = bt.powered().await.unwrap();
        let devices = bt.devices().await.unwrap();
        println!("powered: {powered}, paired: {devices:#?}");
    }
}
