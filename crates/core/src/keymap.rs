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