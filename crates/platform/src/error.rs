/// What can go wrong when asking the system. Adapters map their failures onto this.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The system answered, but not in a way we understand.
    #[error("unexpected output: {0}")]
    Parse(String),

    /// Running a helper tool or system call failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// The system understood, but said no: "device not paired", "Bluetooth is off"…
    #[error("{0}")]
    Failed(String),

    /// This can't work on this machine as it is. The message says how to fix it: "brew install blueutil".
    #[error("{0}")]
    Unavailable(String),
}

/// `Result` with our `Error` as  the default error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_readable() {
        assert_eq!(Error::Parse("42%;huh".into()).to_string(), "unexpected output: 42%;huh");

        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "pmset not found");
        assert_eq!(Error::from(io).to_string(), "pmset not found");
        assert_eq!(Error::Unavailable("brew install blueutil".into()).to_string(), "brew install blueutil");
    }
}
