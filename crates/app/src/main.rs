use pultr_platform::Platform;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match parse(std::env::args().skip(1))? {
        Command::Version => {
            println!("pultr {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Run { mock } => {
            let platform = if mock { pultr_mock::platform() } else { platform() };
            pultr_core::run(pultr_cards::default_cards(&platform)).await
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
    Run { mock: bool },
}

/// Hand-rolled until `clap` arrives in M8.
fn parse(args: impl IntoIterator<Item = String>) -> anyhow::Result<Command> {
    let mut mock = false;
    for arg in args {
        match arg.as_str() {
            "--version" | "-V" => return Ok(Command::Version),
            "--mock" => mock = true,
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }
    Ok(Command::Run { mock })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> anyhow::Result<Command> {
        parse(args.iter().map(|a| a.to_string()))
    }

    #[test]
    fn no_args_runs_for_real() {
        assert_eq!(parse_strs(&[]).unwrap(), Command::Run { mock: false });
    }

    #[test]
    fn mock_flag() {
        assert_eq!(parse_strs(&["--mock"]).unwrap(), Command::Run { mock: true });
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
}
