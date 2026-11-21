/// What the user *wants*, independent of which key they pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    Move(Direction),
    /// Run `action` on the card in `slot`, e.g. `toggle`.
    Run {
        slot: usize,
        action: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
