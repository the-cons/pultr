use crate::{
    action::{Action, Direction},
    card::CardView,
    refresh::Update,
};

/// Cards per row. Fixed for now;
pub const COLUMNS: usize = 3;

pub struct CardSlot {
    pub title: String,
    pub view: Option<CardView>,
}

/// All UI state lives here. Nothing in here touches the terminal or the OS.
pub struct App {
    running: bool,
    cards: Vec<CardSlot>,
    focused: usize,
}

impl App {
    /// One slot per title, in grid order. No views until the first refresh lands.
    pub fn new(titles: impl IntoIterator<Item = String>) -> Self {
        let cards = titles.into_iter().map(|title| CardSlot { title, view: None }).collect();
        Self { running: true, cards, focused: 0 }
    }
    pub fn is_running(&self) -> bool {
        self.running
    }
    pub fn cards(&self) -> &[CardSlot] {
        &self.cards
    }

    pub fn focused(&self) -> usize {
        self.focused
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::Move(dir) => self.move_focus(dir),
        }
    }

    /// A refresh task delivered a new view. Unknown slots are ignored.
    pub fn apply(&mut self, update: Update) {
        if let Some(slot) = self.cards.get_mut(update.slot) {
            slot.view = Some(update.view);
        }
    }

    fn move_focus(&mut self, dir: Direction) {
        let (i, len) = (self.focused, self.cards.len());
        let target = match dir {
            Direction::Left if i % COLUMNS > 0 => i - 1,
            Direction::Right if i % COLUMNS < COLUMNS - 1 => i + 1,
            Direction::Up if i >= COLUMNS => i - COLUMNS,
            Direction::Down if i / COLUMNS < len.saturating_sub(1) / COLUMNS => (i + COLUMNS).min(len - 1),
            _ => i,
        };

        if target < len {
            self.focused = target;
        }
    }
}

/// Six named, empty cards: the same grid the tests have always used.
#[cfg(test)]
impl App {
    pub(crate) fn demo() -> Self {
        Self::new(["bluetooth", "sidecar", "audio", "media", "battery", "system"].map(String::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_running() {
        assert!(App::demo().is_running());
    }

    #[test]
    fn quit_stops_the_app() {
        let mut app = App::demo();
        app.update(Action::Quit);
        assert!(!app.is_running());
    }

    #[test]
    fn starts_with_placeholder_cards() {
        assert_eq!(App::demo().cards().len(), 6);
    }

    fn app_at(focused: usize) -> App {
        let mut app = App::demo();
        app.focused = focused;
        app
    }

    fn moved(mut app: App, dir: Direction) -> usize {
        app.update(Action::Move(dir));
        app.focused()
    }

    #[test]
    fn focus_starts_on_first_card() {
        assert_eq!(App::demo().focused(), 0);
    }

    // Grid with 6 cards, 3 columns:
    //   0 1 2
    //   3 4 5

    #[test]
    fn moves_in_all_directions() {
        assert_eq!(moved(app_at(4), Direction::Left), 3);
        assert_eq!(moved(app_at(4), Direction::Right), 5);
        assert_eq!(moved(app_at(4), Direction::Up), 1);
        assert_eq!(moved(app_at(1), Direction::Down), 4);
    }

    #[test]
    fn stops_at_the_edges() {
        assert_eq!(moved(app_at(0), Direction::Left), 0);
        assert_eq!(moved(app_at(3), Direction::Left), 3); // no wrap to previous row
        assert_eq!(moved(app_at(2), Direction::Right), 2); // no wrap to next row
        assert_eq!(moved(app_at(1), Direction::Up), 1);
        assert_eq!(moved(app_at(4), Direction::Down), 4);
    }

    #[test]
    fn half_full_last_row() {
        // 4 cards:
        //   0 1 2
        //   3
        let mut app = app_at(2);
        app.cards.truncate(4);
        assert_eq!(moved(app, Direction::Down), 3); // lands on the last card

        let mut app = app_at(3);
        app.cards.truncate(4);
        assert_eq!(moved(app, Direction::Right), 3); // nothing to the right
    }

    #[test]
    fn no_cards_no_panic() {
        for dir in [Direction::Left, Direction::Right, Direction::Up, Direction::Down] {
            let mut app = App::demo();
            app.cards.clear();
            assert_eq!(moved(app, dir), 0);
        }
    }
}
