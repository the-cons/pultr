use crate::{
    action::{Action, Direction},
    card::CardView,
    layout::Placement,
    refresh::Update,
};

pub struct CardSlot {
    pub title: String,
    pub placement: Placement,
    pub view: Option<CardView>,
}

/// All UI state lives here. Nothing in here touches the terminal or the OS.
pub struct App {
    running: bool,
    columns: usize,
    cards: Vec<CardSlot>,
    focused: usize,
}

impl App {
    /// One slot per title, in grid order. No views until the first refresh lands.
    pub fn new(columns: usize, cards: impl IntoIterator<Item = (String, Placement)>) -> Self {
        let cards = cards.into_iter().map(|(title, placement)| CardSlot { title, placement, view: None }).collect();
        Self { running: true, columns, cards, focused: 0 }
    }

    pub fn columns(&self) -> usize {
        self.columns
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
            // Running is the event loop's job (it has the cards). Here: focus the card, so you see who reacted.
            Action::Run { slot, .. } if slot < self.cards.len() => self.focused = slot,
            Action::Run { .. } => {}
        }
    }

    /// A refresh task delivered a new view. Unknown slots are ignored.
    pub fn apply(&mut self, update: Update) {
        if let Some(slot) = self.cards.get_mut(update.slot) {
            slot.view = Some(update.view);
        }
    }
    /// Nearest card in `dir`: fewest steps that way first, then fewest steps sideways,
    /// then the earlier card. Nothing there → focus stays.
    fn move_focus(&mut self, dir: Direction) {
        let Some(from) = self.cards.get(self.focused).map(|slot| slot.placement) else { return };
        let nearest = self
            .cards
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| from.distance(slot.placement, dir).map(|steps| (steps, i)))
            .min();
        if let Some((_, i)) = nearest {
            self.focused = i;
        }
    }
}

/// Six named, empty cards: the same grid the tests have always used.
#[cfg(test)]
impl App {
    pub(crate) fn demo() -> Self {
        let names = ["bluetooth", "sidecar", "audio", "media", "battery", "system"].map(String::from);
        Self::new(3, names.into_iter().zip(crate::layout::row_major(6, 3)))
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
    fn running_an_action_focuses_its_card() {
        let mut app = App::demo();
        app.update(Action::Run { slot: 4, action: "toggle".to_string(), arg: None });
        assert_eq!(app.focused(), 4);

        app.update(Action::Run { slot: 99, action: "toggle".to_string(), arg: None }); // no such card: ignored
        assert_eq!(app.focused(), 4);
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
        assert_eq!(moved(app, Direction::Down), 3);

        let mut app = app_at(3);
        app.cards.truncate(4);
        assert_eq!(moved(app, Direction::Right), 1);
    }

    /// `[col, row, cols, rows]` per card.
    fn app_with(rects: &[[usize; 4]], focused: usize) -> App {
        let cards = rects
            .iter()
            .enumerate()
            .map(|(i, &[col, row, cols, rows])| (i.to_string(), Placement { col, row, cols, rows }));
        let mut app = App::new(3, cards);
        app.focused = focused;
        app
    }

    #[test]
    fn moves_across_spanning_cards() {
        //   0 0 1
        //   2 3 3
        let grid = [[0, 0, 2, 1], [2, 0, 1, 1], [0, 1, 1, 1], [1, 1, 2, 1]];
        assert_eq!(moved(app_with(&grid, 0), Direction::Right), 1);
        assert_eq!(moved(app_with(&grid, 0), Direction::Down), 2); // 2 and 3 both below: earlier wins
        assert_eq!(moved(app_with(&grid, 1), Direction::Down), 3); // only 3 is straight below
        assert_eq!(moved(app_with(&grid, 1), Direction::Left), 0);
        assert_eq!(moved(app_with(&grid, 3), Direction::Up), 0); // 0 and 1 both above: earlier wins
        assert_eq!(moved(app_with(&grid, 2), Direction::Right), 3);
    }

    #[test]
    fn jumps_over_empty_cells() {
        //   0 . 1
        //   . . .
        //   2 . .
        let grid = [[0, 0, 1, 1], [2, 0, 1, 1], [0, 2, 1, 1]];
        assert_eq!(moved(app_with(&grid, 0), Direction::Right), 1);
        assert_eq!(moved(app_with(&grid, 0), Direction::Down), 2);
        assert_eq!(moved(app_with(&grid, 1), Direction::Down), 2); // nothing straight below: nearest diagonal
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
