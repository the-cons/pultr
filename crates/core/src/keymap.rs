//! Key → Action: the app's own keys plus every card's `keys` from the config.
//!
//! Built once at startup. Two things on one key is a config error, never a guess.

use std::collections::HashMap;

use anyhow::bail;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    action::{Action, Direction},
    config::CardConfig,
    key::Key,
};

pub struct Keymap {
    bindings: HashMap<Key, Action>,
}

impl Keymap {
    /// Built-in keys first, then each card's `keys` in config order.
    pub fn new(cards: &[CardConfig]) -> anyhow::Result<Self> {
        let mut bindings = HashMap::new();
        let mut owners: HashMap<Key, String> = HashMap::new(); // who has the key, for error messages

        let built_in = built_in().into_iter().map(|(key, action, name)| (key, action, format!("built-in `{name}`")));
        let from_cards = cards.iter().enumerate().flat_map(|(slot, card)| {
            card.keys.iter().map(move |(action, &key)| {
                (key, Action::Run { slot, action: action.clone() }, format!("`{}.{action}`", card.id))
            })
        });

        for (key, action, name) in built_in.chain(from_cards) {
            if let Some(taken) = owners.get(&key) {
                bail!("key `{key}` is bound twice: {taken} and {name}");
            }
            owners.insert(key, name);
            bindings.insert(key, action);
        }
        Ok(Self { bindings })
    }

    pub fn map(&self, event: KeyEvent) -> Option<Action> {
        self.bindings.get(&Key::from_event(event)).cloned()
    }
}

/// The app's own keys. Fixed for now; a `[keys]` section can rebind them later.
fn built_in() -> Vec<(Key, Action, &'static str)> {
    let none = KeyModifiers::NONE;
    let char = |c| Key::new(KeyCode::Char(c), none);
    let left = Action::Move(Direction::Left);
    let down = Action::Move(Direction::Down);
    let up = Action::Move(Direction::Up);
    let right = Action::Move(Direction::Right);

    vec![
        (char('q'), Action::Quit, "quit"),
        (Key::new(KeyCode::Esc, none), Action::Quit, "quit"),
        (Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL), Action::Quit, "quit"),
        (Key::new(KeyCode::Left, none), left.clone(), "move left"),
        (char('h'), left, "move left"),
        (Key::new(KeyCode::Down, none), down.clone(), "move down"),
        (char('j'), down, "move down"),
        (Key::new(KeyCode::Up, none), up.clone(), "move up"),
        (char('k'), up, "move up"),
        (Key::new(KeyCode::Right, none), right.clone(), "move right"),
        (char('l'), right, "move right"),
    ]
}

#[cfg(test)]
impl Default for Keymap {
    /// Only the built-in keys.
    fn default() -> Self {
        Self::new(&[]).expect("built-in keys don't clash")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    fn keymap(config: &str) -> anyhow::Result<Keymap> {
        Keymap::new(&Config::parse(config)?.cards)
    }

    // quit_keys, modified_q_does_not_quit, capital_q_does_not_quit, plain_c_does_not_quit,
    // unbound_keys_map_to_nothing, kind_is_ignored_here, arrows_and_hjkl_move_focus:
    // keep them exactly as they are. The `map` helper above makes them work unchanged.

    #[test]
    fn card_keys_run_actions_on_their_slot() {
        let keymap = keymap(
            r#"
            [[card]]
            id = "battery"
            type = "battery"

            [[card]]
            id   = "music"
            type = "media"
            keys = { play_pause = "space", next = "N" }
            "#,
        )
        .unwrap();

        let run = |action: &str| Some(Action::Run { slot: 1, action: action.to_string() });
        assert_eq!(keymap.map(key(KeyCode::Char(' '), KeyModifiers::NONE)), run("play_pause"));
        assert_eq!(keymap.map(key(KeyCode::Char('N'), KeyModifiers::SHIFT)), run("next"));
        assert_eq!(keymap.map(key(KeyCode::Char('q'), KeyModifiers::NONE)), Some(Action::Quit)); // built-ins still there
    }

    #[test]
    fn card_keys_clash_with_each_other() {
        let err = keymap(
            r#"
            [[card]]
            id = "a"
            type = "x"
            keys = { toggle = "t" }

            [[card]]
            id = "b"
            type = "x"
            keys = { toggle = "t" }
            "#,
        )
        .err()
        .unwrap();
        assert_eq!(err.to_string(), "key `t` is bound twice: `a.toggle` and `b.toggle`");
    }

    #[test]
    fn card_keys_cannot_take_built_in_keys() {
        let err = keymap("[[card]]\nid = \"m\"\ntype = \"x\"\nkeys = { prev = \"h\" }").err().unwrap();
        assert_eq!(err.to_string(), "key `h` is bound twice: built-in `move left` and `m.prev`");
    }
}
