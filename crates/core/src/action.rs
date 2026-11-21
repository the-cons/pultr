/// What the user *wants*, independent of which key they pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    Move(Direction),
    /// Run `action` on the card in `slot`, e.g. `toggle`, with an optional argument: `AirPods Pro`.
    Run {
        slot: usize,
        action: String,
        arg: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// `toggle:AirPods Pro` → (`toggle`, `Some("AirPods Pro")`). Everything after the first `:` is the argument.
pub fn split(name: &str) -> (&str, Option<&str>) {
    match name.split_once(':') {
        Some((action, arg)) => (action, Some(arg)),
        None => (name, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_at_the_first_colon() {
        assert_eq!(split("toggle"), ("toggle", None));
        assert_eq!(split("toggle:AirPods Pro"), ("toggle", Some("AirPods Pro")));
        assert_eq!(split("connect:aa:bb:cc"), ("connect", Some("aa:bb:cc"))); // addresses keep their colons
        assert_eq!(split("toggle:"), ("toggle", Some("")));
    }
}
