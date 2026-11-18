use crate::action::{Action, Direction};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Key -> Action. Hard-coded for now.
pub fn map(key: KeyEvent) -> Option<Action> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, _) => Some(Action::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Quit),
        (KeyCode::Left | KeyCode::Char('h'), KeyModifiers::NONE) => Some(Action::Move(Direction::Left)),
        (KeyCode::Down | KeyCode::Char('j'), KeyModifiers::NONE) => Some(Action::Move(Direction::Down)),
        (KeyCode::Up | KeyCode::Char('k'), KeyModifiers::NONE) => Some(Action::Move(Direction::Up)),
        (KeyCode::Right | KeyCode::Char('l'), KeyModifiers::NONE) => Some(Action::Move(Direction::Right)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

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

    #[test]
    fn arrows_and_hjkl_move_focus() {
        let none = KeyModifiers::NONE;
        for (code, dir) in [
            (KeyCode::Left, Direction::Left),
            (KeyCode::Char('h'), Direction::Left),
            (KeyCode::Down, Direction::Down),
            (KeyCode::Char('j'), Direction::Down),
            (KeyCode::Up, Direction::Up),
            (KeyCode::Char('k'), Direction::Up),
            (KeyCode::Right, Direction::Right),
            (KeyCode::Char('l'), Direction::Right),
        ] {
            assert_eq!(map(key(code, none)), Some(Action::Move(dir)), "{code:?}");
        }
    }
}
