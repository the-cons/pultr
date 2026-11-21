//! One key, as written in the config: `a`, `A`, `ctrl-a`, `alt-1`, `space`, `enter`, `f5`.

use std::{fmt, str::FromStr};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct Key {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl Key {
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// The key a terminal event stands for. Shift lives in the character (`A`, `?`),
    /// because terminals disagree on whether they also report the SHIFT flag.
    pub fn from_event(event: KeyEvent) -> Self {
        let mut modifiers = event.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT);
        if let KeyCode::Char(_) = event.code {
            modifiers.remove(KeyModifiers::SHIFT);
        }

        Self { code: event.code, modifiers }
    }
}

impl FromStr for Key {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut modifiers = KeyModifiers::NONE;
        let mut rest = s;
        loop {
            if let Some(r) = rest.strip_prefix("ctrl-") {
                modifiers |= KeyModifiers::CONTROL;
                rest = r;
            } else if let Some(r) = rest.strip_prefix("alt-") {
                modifiers |= KeyModifiers::ALT;
                rest = r;
            } else {
                break;
            }
        }

        let code = match rest {
            "space" => KeyCode::Char(' '),
            "enter" => KeyCode::Enter,
            "tab" => KeyCode::Tab,
            "esc" => KeyCode::Esc,
            "backspace" => KeyCode::Backspace,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            _ => match (function_key(rest), single_char(rest)) {
                (Some(n), _) => KeyCode::F(n),
                (_, Some(c)) => KeyCode::Char(c),
                _ => return Err(format!("unknown key `{s}` (examples: a, A, ctrl-a, alt-1, space, enter, f5)")),
            },
        };
        Ok(Self { code, modifiers })
    }
}

/// `f1`–`f12`. A lone `f` is just the letter.
fn function_key(s: &str) -> Option<u8> {
    s.strip_prefix('f')?.parse().ok().filter(|n| (1..=12).contains(n))
}

/// Exactly one visible character.
fn single_char(s: &str) -> Option<char> {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if !c.is_whitespace() => Some(c),
        _ => None,
    }
}

/// Lets serde read a `Key` from a TOML string, with our error message.
impl TryFrom<String> for Key {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// Written the way the config spells it, so error messages match what the user typed.
impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            f.write_str("ctrl-")?;
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            f.write_str("alt-")?;
        }
        match self.code {
            KeyCode::Char(' ') => f.write_str("space"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::Enter => f.write_str("enter"),
            KeyCode::Tab => f.write_str("tab"),
            KeyCode::Esc => f.write_str("esc"),
            KeyCode::Backspace => f.write_str("backspace"),
            KeyCode::Up => f.write_str("up"),
            KeyCode::Down => f.write_str("down"),
            KeyCode::Left => f.write_str("left"),
            KeyCode::Right => f.write_str("right"),
            KeyCode::F(n) => write!(f, "f{n}"),
            other => write!(f, "{other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> Key {
        s.parse().unwrap()
    }

    #[test]
    fn parses_the_documented_syntax() {
        assert_eq!(key("a"), Key::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(key("A"), Key::new(KeyCode::Char('A'), KeyModifiers::NONE));
        assert_eq!(key("ctrl-a"), Key::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(key("alt-1"), Key::new(KeyCode::Char('1'), KeyModifiers::ALT));
        assert_eq!(key("ctrl-alt-x"), Key::new(KeyCode::Char('x'), KeyModifiers::CONTROL | KeyModifiers::ALT));
        assert_eq!(key("space"), Key::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(key("enter"), Key::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(key("f5"), Key::new(KeyCode::F(5), KeyModifiers::NONE));
        assert_eq!(key("f"), Key::new(KeyCode::Char('f'), KeyModifiers::NONE)); // the letter, not a function key
        assert_eq!(key("-"), Key::new(KeyCode::Char('-'), KeyModifiers::NONE));
    }

    #[test]
    fn rejects_nonsense() {
        for bad in ["", "ab", "shift-a", "ctrl-", "f13", "f0", "hyper-a"] {
            let err = bad.parse::<Key>().unwrap_err();
            assert!(err.starts_with(&format!("unknown key `{bad}`")), "{bad}: {err}");
        }
    }

    #[test]
    fn display_round_trips() {
        for s in ["a", "A", "ctrl-a", "alt-1", "ctrl-alt-x", "space", "enter", "esc", "left", "f12", "?"] {
            assert_eq!(key(s).to_string(), s);
        }
    }

    #[test]
    fn events_match_config_keys() {
        let event = |code, modifiers| Key::from_event(KeyEvent::new(code, modifiers));
        assert_eq!(event(KeyCode::Char('a'), KeyModifiers::NONE), key("a"));
        assert_eq!(event(KeyCode::Char('a'), KeyModifiers::CONTROL), key("ctrl-a"));
        // Shift: some terminals send the flag, some don't. Both mean `A`.
        assert_eq!(event(KeyCode::Char('A'), KeyModifiers::SHIFT), key("A"));
        assert_eq!(event(KeyCode::Char('A'), KeyModifiers::NONE), key("A"));
        // Other flags (super, hyper…) aren't part of a key.
        assert_eq!(event(KeyCode::Char('a'), KeyModifiers::SUPER), key("a"));
    }
}
