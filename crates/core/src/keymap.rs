//! Key → Action: the app's own keys plus every card's `keys` from the config.
//!
//! Built once at startup. Two things on one key is a config error, never a guess.
//! Inside a card, a few keys mean something else (rows instead of cards); everything else works as usual.

use std::collections::HashMap;

use anyhow::bail;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    action::{self, Action, Direction, Run},
    config::CardConfig,
    key::Key,
};

pub struct Keymap {
    bindings: HashMap<Key, Action>,
    /// Checked first while inside a card.
    inside: HashMap<Key, Action>,
}

impl Keymap {
    /// Built-in keys first, then each card's `keys` in config order.
    pub fn new(cards: &[CardConfig]) -> anyhow::Result<Self> {
        let mut bindings = HashMap::new();
        let mut owners: HashMap<Key, String> = HashMap::new(); // who has the key, for error messages

        for card in cards {
            for name in card.keys.keys() {
                let (action, arg) = action::split(name);
                if action.is_empty() || arg == Some("") {
                    bail!("card `{}`: `{name}` is not an action (write `toggle` or `toggle:AirPods Pro`)", card.id);
                }
            }
        }

        let built_in = built_in().into_iter().map(|(key, action, name)| (key, action, format!("built-in `{name}`")));
        let from_cards = cards.iter().enumerate().flat_map(|(slot, card)| {
            card.keys.iter().map(move |(name, &key)| {
                let (action, arg) = action::split(name);
                let run = Action::Run(Run { slot, action: action.to_string(), arg: arg.map(String::from) });
                (key, run, format!("`{}.{name}`", card.id))
            })
        });

        for (key, action, name) in built_in.chain(from_cards) {
            if let Some(taken) = owners.get(&key) {
                bail!("key `{key}` is bound twice: {taken} and {name}");
            }
            owners.insert(key, name);
            bindings.insert(key, action);
        }
        Ok(Self { bindings, inside: inside().into_iter().collect() })
    }

    /// `inside`: whether the user is inside a card right now.
    pub fn map(&self, event: KeyEvent, inside: bool) -> Option<Action> {
        let key = Key::from_event(event);
        let inside_action = if inside { self.inside.get(&key) } else { None };
        inside_action.or_else(|| self.bindings.get(&key)).cloned()
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
        (Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL), Action::Quit, "quit"),
        (Key::new(KeyCode::Enter, none), Action::Enter, "open card"),
        (Key::new(KeyCode::Esc, none), Action::Back, "back"),
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

/// Inside a card these win over everything else. `space` is the only one a card key can also have:
/// it keeps working everywhere except inside a card.
fn inside() -> Vec<(Key, Action)> {
    let none = KeyModifiers::NONE;
    let char = |c| Key::new(KeyCode::Char(c), none);
    vec![
        (char('j'), Action::SelectNext),
        (Key::new(KeyCode::Down, none), Action::SelectNext),
        (char('k'), Action::SelectPrev),
        (Key::new(KeyCode::Up, none), Action::SelectPrev),
        (Key::new(KeyCode::Enter, none), Action::Activate),
        (char(' '), Action::Activate),
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

    fn press(keymap: &Keymap, code: KeyCode) -> Option<Action> {
        keymap.map(key(code, KeyModifiers::NONE), false)
    }

    fn press_inside(keymap: &Keymap, code: KeyCode) -> Option<Action> {
        keymap.map(key(code, KeyModifiers::NONE), true)
    }

    #[test]
    fn quit_keys() {
        let keymap = Keymap::default();
        assert_eq!(press(&keymap, KeyCode::Char('q')), Some(Action::Quit));
        assert_eq!(keymap.map(key(KeyCode::Char('c'), KeyModifiers::CONTROL), false), Some(Action::Quit));
        assert_eq!(press(&keymap, KeyCode::Char('c')), None);
        assert_eq!(press(&keymap, KeyCode::Char('Q')), None);
    }

    #[test]
    fn arrows_and_hjkl_move_focus() {
        let keymap = Keymap::default();
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
            assert_eq!(press(&keymap, code), Some(Action::Move(dir)), "{code:?}");
        }
    }

    #[test]
    fn enter_opens_and_esc_goes_back() {
        let keymap = Keymap::default();
        assert_eq!(press(&keymap, KeyCode::Enter), Some(Action::Enter));
        assert_eq!(press(&keymap, KeyCode::Esc), Some(Action::Back)); // no longer quits
        assert_eq!(press_inside(&keymap, KeyCode::Esc), Some(Action::Back));
    }

    #[test]
    fn inside_a_card_keys_pick_rows() {
        let keymap = Keymap::default();
        assert_eq!(press_inside(&keymap, KeyCode::Char('j')), Some(Action::SelectNext));
        assert_eq!(press_inside(&keymap, KeyCode::Down), Some(Action::SelectNext));
        assert_eq!(press_inside(&keymap, KeyCode::Char('k')), Some(Action::SelectPrev));
        assert_eq!(press_inside(&keymap, KeyCode::Up), Some(Action::SelectPrev));
        assert_eq!(press_inside(&keymap, KeyCode::Enter), Some(Action::Activate));
        assert_eq!(press_inside(&keymap, KeyCode::Char(' ')), Some(Action::Activate));
        // the rest works as usual: leave sideways, quit
        assert_eq!(press_inside(&keymap, KeyCode::Char('l')), Some(Action::Move(Direction::Right)));
        assert_eq!(press_inside(&keymap, KeyCode::Char('q')), Some(Action::Quit));
    }

    #[test]
    fn card_key_on_space_is_shadowed_only_inside() {
        let keymap = keymap("[[card]]\nid = \"m\"\ntype = \"x\"\nkeys = { play = \"space\", next = \"n\" }").unwrap();
        let play = Some(Action::Run(Run { slot: 0, action: "play".to_string(), arg: None }));
        assert_eq!(press(&keymap, KeyCode::Char(' ')), play);
        assert_eq!(press_inside(&keymap, KeyCode::Char(' ')), Some(Action::Activate));
        assert!(matches!(press_inside(&keymap, KeyCode::Char('n')), Some(Action::Run(_)))); // other card keys still work
    }

    #[test]
    fn enter_and_esc_are_taken() {
        let err = keymap("[[card]]\nid = \"m\"\ntype = \"x\"\nkeys = { go = \"enter\" }").err().unwrap();
        assert_eq!(err.to_string(), "key `enter` is bound twice: built-in `open card` and `m.go`");
    }

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

        let run = |action: &str| Some(Action::Run(Run { slot: 1, action: action.to_string(), arg: None }));
        assert_eq!(press(&keymap, KeyCode::Char(' ')), run("play_pause"));
        assert_eq!(keymap.map(key(KeyCode::Char('N'), KeyModifiers::SHIFT), false), run("next"));
        assert_eq!(press(&keymap, KeyCode::Char('q')), Some(Action::Quit)); // built-ins still there
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
    fn card_keys_with_an_argument() {
        let keymap = keymap(
            r#"
            [[card]]
            id   = "bt"
            type = "bluetooth"
            keys = { "toggle:AirPods Pro" = "a", "toggle:Magic Keyboard" = "b", power = "p" }
            "#,
        )
        .unwrap();

        let run = |action: &str, arg: Option<&str>| {
            Some(Action::Run(Run { slot: 0, action: action.to_string(), arg: arg.map(String::from) }))
        };
        assert_eq!(press(&keymap, KeyCode::Char('a')), run("toggle", Some("AirPods Pro")));
        assert_eq!(press(&keymap, KeyCode::Char('b')), run("toggle", Some("Magic Keyboard")));
        assert_eq!(press(&keymap, KeyCode::Char('p')), run("power", None));
    }

    #[test]
    fn argument_shows_in_clash_messages() {
        let err = keymap("[[card]]\nid = \"bt\"\ntype = \"x\"\nkeys = { \"toggle:AirPods\" = \"q\" }").err().unwrap();
        assert_eq!(err.to_string(), "key `q` is bound twice: built-in `quit` and `bt.toggle:AirPods`");
    }

    #[test]
    fn empty_action_or_argument_is_an_error() {
        for name in ["toggle:", ":AirPods", ""] {
            let config = format!("[[card]]\nid = \"bt\"\ntype = \"x\"\nkeys = {{ \"{name}\" = \"a\" }}");
            let err = keymap(&config).err().unwrap();
            assert_eq!(
                err.to_string(),
                format!("card `bt`: `{name}` is not an action (write `toggle` or `toggle:AirPods Pro`)")
            );
        }
    }

    #[test]
    fn card_keys_cannot_take_built_in_keys() {
        let err = keymap("[[card]]\nid = \"m\"\ntype = \"x\"\nkeys = { prev = \"h\" }").err().unwrap();
        assert_eq!(err.to_string(), "key `h` is bound twice: built-in `move left` and `m.prev`");
    }
}
