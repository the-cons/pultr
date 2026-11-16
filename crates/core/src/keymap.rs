use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::action::Action;

/// Key -> Action. Hard-coded for now.
pub fn map(key: KeyEvent) -> Option<Action> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, _) => Some(Action::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Quit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyEventKind;
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn quit_keys() {
        for k in [
            key(KeyCode::Char('q'), KeyModifiers::NONE),
            key(KeyCode::Esc, KeyModifiers::NONE),
            key(KeyCode::Char('c'), KeyModifiers::CONTROL),
        ] {
            assert_eq!(map(k), Some(Action::Quit), "{k:?}");
        }
    }

    #[test]
    fn modified_q_does_not_quit() {
        assert_eq!(map(key(KeyCode::Char('q'), KeyModifiers::CONTROL)), None);
        assert_eq!(map(key(KeyCode::Char('q'), KeyModifiers::ALT)), None);
    }

    #[test]
    fn capital_q_does_not_quit() {
        assert_eq!(map(key(KeyCode::Char('Q'), KeyModifiers::SHIFT)), None);
    }

    #[test]
    fn plain_c_does_not_quit() {
        assert_eq!(map(key(KeyCode::Char('c'), KeyModifiers::NONE)), None);
    }

    #[test]
    fn unbound_keys_map_to_nothing() {
        assert_eq!(map(key(KeyCode::Char('x'), KeyModifiers::NONE)), None);
        assert_eq!(map(key(KeyCode::Enter, KeyModifiers::NONE)), None);
    }

    #[test]
    fn kind_is_ignored_here() {
        let mut k = key(KeyCode::Char('q'), KeyModifiers::NONE);
        k.kind = KeyEventKind::Release;
        assert_eq!(map(k), Some(Action::Quit));
    }
}