use std::path::PathBuf;

use anyhow::Context;
use pultr_core::config::Config;
use pultr_platform::Platform;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match parse(std::env::args().skip(1))? {
        Command::Version => {
            println!("pultr {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Run { mock, config } => {
            // Config and cards are ready before the TUI starts, so mistakes are plain errors on stderr.
            let config = match pultr_core::config::load(config.as_deref())? {
                Some(config) => config,
                None => Config::parse(pultr_cards::DEFAULT_CONFIG)?,
            };
            let platform = if mock { pultr_mock::platform() } else { platform() };
            let cards = pultr_cards::registry(&platform).build_all(&config.cards)?;
            pultr_core::run(cards, &config).await
        }
    }
}

/// Real adapters arrive with M4 (macOS) and M11 (Linux). Until then this machine offers nothing.
fn platform() -> Platform {
    Platform::default()
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Version,
    Run { mock: bool, config: Option<PathBuf> },
}

/// Hand-rolled until `clap` arrives in M8.
fn parse(args: impl IntoIterator<Item = String>) -> anyhow::Result<Command> {
    let mut mock = false;
    let mut config = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => return Ok(Command::Version),
            "--mock" => mock = true,
            "--config" => config = Some(args.next().context("--config needs a path")?.into()),
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }
    Ok(Command::Run { mock, config })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> anyhow::Result<Command> {
        parse(args.iter().map(|a| a.to_string()))
    }

    #[test]
    fn no_args_runs_for_real() {
        assert_eq!(parse_strs(&[]).unwrap(), Command::Run { mock: false, config: None });
    }

    #[test]
    fn mock_flag() {
        assert_eq!(parse_strs(&["--mock"]).unwrap(), Command::Run { mock: true, config: None });
    }

    #[test]
    fn version_wins() {
        assert_eq!(parse_strs(&["-V"]).unwrap(), Command::Version);
        assert_eq!(parse_strs(&["--mock", "--version"]).unwrap(), Command::Version);
    }

    #[test]
    fn unknown_argument_is_an_error() {
        assert_eq!(parse_strs(&["--nope"]).unwrap_err().to_string(), "unknown argument: --nope");
    }

    #[test]
    fn config_flag() {
        let expected = Command::Run { mock: true, config: Some(PathBuf::from("my.toml")) };
        assert_eq!(parse_strs(&["--config", "my.toml", "--mock"]).unwrap(), expected);
    }

    #[test]
    fn config_flag_needs_a_path() {
        assert_eq!(parse_strs(&["--config"]).unwrap_err().to_string(), "--config needs a path");
    }
}
