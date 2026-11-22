use std::time::{Duration, Instant};

use crate::{
    action::{Action, Direction, Run},
    card::{CardView, Item},
    layout::Placement,
    refresh::Update,
};

/// How long a failed action's message stays on its card.
pub const FAILURE_SHOWN: Duration = Duration::from_secs(6);

pub struct CardSlot {
    pub title: String,
    pub placement: Placement,
    pub view: Option<CardView>,
    /// Index into the view's items. Kept while you're elsewhere, so you come back to the same row.
    pub selected: usize,
    /// Actions on this card still running. More than one is possible: two keys pressed quickly.
    pub busy: usize,
    /// The last action that failed, shown until it expires or the next action starts.
    pub failure: Option<Failure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub message: String,
    pub until: Instant,
}

impl CardSlot {
    /// The rows of the latest view; none before the first refresh.
    pub fn items(&self) -> &[Item] {
        self.view.as_ref().map_or(&[], |view| &view.items)
    }
}

/// All UI state lives here. Nothing in here touches the terminal or the OS.
pub struct App {
    running: bool,
    columns: usize,
    cards: Vec<CardSlot>,
    focused: usize,
    /// Inside the focused card: keys pick rows instead of moving between cards.
    inside: bool,
}

impl App {
    /// One slot per title, in grid order. No views until the first refresh lands.
    pub fn new(columns: usize, cards: impl IntoIterator<Item = (String, Placement)>) -> Self {
        let cards = cards
            .into_iter()
            .map(|(title, placement)| CardSlot { title, placement, view: None, selected: 0, busy: 0, failure: None })
            .collect();
        Self { running: true, columns, cards, focused: 0, inside: false }
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
    pub fn is_inside(&self) -> bool {
        self.inside
    }

    /// Changes UI state. Returns the card action to run, if there is one: running it is the
    /// event loop's job, because the loop has the cards. Its card counts as busy until [`App::finish`].
    pub fn update(&mut self, action: Action) -> Option<Run> {
        let run = self.decide(action)?;
        if let Some(slot) = self.cards.get_mut(run.slot) {
            slot.busy += 1;
            slot.failure = None; // a new try: the old message is out of date
        }
        Some(run)
    }

    /// An action on `slot` finished. A failure shows on the card for [`FAILURE_SHOWN`].
    pub fn finish(&mut self, slot: usize, error: Option<String>, now: Instant) {
        let Some(slot) = self.cards.get_mut(slot) else { return };
        slot.busy = slot.busy.saturating_sub(1);
        if let Some(message) = error {
            slot.failure = Some(Failure { message, until: now + FAILURE_SHOWN });
        }
    }

    /// Clears failure messages that have been shown long enough. Called on every tick.
    pub fn expire(&mut self, now: Instant) {
        for slot in &mut self.cards {
            if slot.failure.as_ref().is_some_and(|f| f.until <= now) {
                slot.failure = None;
            }
        }
    }

    fn decide(&mut self, action: Action) -> Option<Run> {
        match action {
            Action::Quit => self.running = false,
            Action::Move(dir) => {
                self.inside = false;
                self.move_focus(dir);
            }
            Action::Enter => self.inside = self.cards.get(self.focused).is_some_and(|slot| !slot.items().is_empty()),
            Action::Back => self.inside = false,
            Action::SelectNext => self.select(|i, last| (i + 1).min(last)),
            Action::SelectPrev => self.select(|i, _| i.saturating_sub(1)),
            Action::Activate => return self.activate(),
            // Focus the card, so you see who reacted.
            Action::Run(run) if run.slot < self.cards.len() => {
                if run.slot != self.focused {
                    self.inside = false;
                }
                self.focused = run.slot;
                return Some(run);
            }
            Action::Run(_) => {}
        }
        None
    }

    /// A refresh task delivered a new view. Unknown slots are ignored.
    /// The selection follows its row by id, so a reordered list doesn't move it to another device.
    pub fn apply(&mut self, update: Update) {
        let Some(slot) = self.cards.get_mut(update.slot) else { return };
        let selected_id = slot.items().get(slot.selected).map(|item| item.id.clone());
        slot.view = Some(update.view);

        let items = slot.items();
        if let Some(i) = selected_id.and_then(|id| items.iter().position(|item| item.id == id)) {
            slot.selected = i;
        } else if !items.is_empty() {
            slot.selected = slot.selected.min(items.len() - 1); // row gone: stay near it
        } // no rows (e.g. an error view): keep the index for when they come back
    }

    /// Moves the selection inside the focused card. `step(current, last)` → new index.
    fn select(&mut self, step: impl Fn(usize, usize) -> usize) {
        if !self.inside {
            return;
        }
        let Some(slot) = self.cards.get_mut(self.focused) else { return };
        let Some(last) = slot.items().len().checked_sub(1) else { return }; // no rows
        slot.selected = step(slot.selected.min(last), last);
    }

    /// The selected row's action, with the row's id as argument.
    fn activate(&self) -> Option<Run> {
        if !self.inside {
            return None;
        }
        let slot = self.cards.get(self.focused)?;
        let item = slot.items().get(slot.selected)?;
        Some(Run { slot: self.focused, action: item.action.clone()?, arg: Some(item.id.clone()) })
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
    use crate::card::State;

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
        let run = |slot| Run { slot, action: "toggle".to_string(), arg: None };
        assert_eq!(app.update(Action::Run(run(4))), Some(run(4)));
        assert_eq!(app.focused(), 4);

        assert_eq!(app.update(Action::Run(run(99))), None); // no such card: ignored
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

    // ── inside a card ────────────────────────────────────

    /// Rows `a`, `b`, `c`: `a` and `c` toggle, `b` has no action.
    fn rows(ids: &[&str]) -> CardView {
        ids.iter().fold(CardView::new(State::Ok), |view, &id| {
            let item = Item::new(id, id.to_uppercase());
            view.item(if id == "b" { item } else { item.action("toggle") })
        })
    }

    /// Demo app with rows `a`, `b`, `c` in card 0, already inside it.
    fn inside() -> App {
        let mut app = App::demo();
        app.apply(Update { slot: 0, view: rows(&["a", "b", "c"]) });
        app.update(Action::Enter);
        app
    }

    fn selected(app: &App) -> usize {
        app.cards()[app.focused()].selected
    }

    fn toggle(slot: usize, id: &str) -> Option<Run> {
        Some(Run { slot, action: "toggle".to_string(), arg: Some(id.to_string()) })
    }

    #[test]
    fn enter_only_goes_into_cards_with_rows() {
        assert!(inside().is_inside());

        let mut app = App::demo(); // no views yet, so no rows
        app.update(Action::Enter);
        assert!(!app.is_inside());
    }

    #[test]
    fn back_and_move_leave_the_card() {
        let mut app = inside();
        app.update(Action::Back);
        assert!(!app.is_inside());

        let mut app = inside();
        app.update(Action::Move(Direction::Right));
        assert!(!app.is_inside());
        assert_eq!(app.focused(), 1);
    }

    #[test]
    fn select_stops_at_both_ends() {
        let mut app = inside();
        app.update(Action::SelectPrev);
        assert_eq!(selected(&app), 0);
        for _ in 0..5 {
            app.update(Action::SelectNext);
        }
        assert_eq!(selected(&app), 2);
        app.update(Action::SelectPrev);
        assert_eq!(selected(&app), 1);
    }

    #[test]
    fn select_needs_to_be_inside() {
        let mut app = inside();
        app.update(Action::Back);
        app.update(Action::SelectNext);
        assert_eq!(selected(&app), 0);
    }

    #[test]
    fn activate_runs_the_rows_action_with_its_id() {
        let mut app = inside();
        assert_eq!(app.update(Action::Activate), toggle(0, "a"));

        app.update(Action::SelectNext);
        assert_eq!(app.update(Action::Activate), None); // `b` has no action

        app.update(Action::SelectNext);
        assert_eq!(app.update(Action::Activate), toggle(0, "c"));

        app.update(Action::Back);
        assert_eq!(app.update(Action::Activate), None); // not inside
    }

    #[test]
    fn selection_follows_its_row_across_refreshes() {
        let mut app = inside();
        app.update(Action::SelectNext); // on `b`

        app.apply(Update { slot: 0, view: rows(&["c", "a", "b"]) }); // reordered
        assert_eq!(selected(&app), 2);

        app.apply(Update { slot: 0, view: rows(&["a"]) }); // `b` is gone: nearest row
        assert_eq!(selected(&app), 0);
    }

    #[test]
    fn error_view_keeps_the_selection_for_later() {
        let mut app = inside();
        app.update(Action::SelectNext);
        app.update(Action::SelectNext); // on `c`

        app.apply(Update { slot: 0, view: CardView::error("boom") });
        assert!(app.is_inside());
        assert_eq!(app.update(Action::Activate), None); // nothing to run on

        app.apply(Update { slot: 0, view: rows(&["a", "b", "c"]) });
        assert_eq!(app.update(Action::Activate), toggle(0, "c"));
    }

    // ── running actions ──────────────────────────────────

    fn run(slot: usize) -> Action {
        Action::Run(Run { slot, action: "toggle".to_string(), arg: None })
    }

    #[test]
    fn running_an_action_makes_its_card_busy_until_done() {
        let mut app = App::demo();
        let now = Instant::now();
        app.update(run(2));
        app.update(run(2)); // pressed twice
        assert_eq!(app.cards()[2].busy, 2);

        app.finish(2, None, now);
        assert_eq!(app.cards()[2].busy, 1);
        app.finish(2, None, now);
        app.finish(2, None, now); // one too many: no underflow
        assert_eq!(app.cards()[2].busy, 0);
        assert_eq!(app.cards()[2].failure, None);
    }

    #[test]
    fn failure_shows_for_a_while() {
        let mut app = App::demo();
        let now = Instant::now();
        app.update(run(1));
        app.finish(1, Some("Failed to connect".to_string()), now);
        assert_eq!(app.cards()[1].failure.as_ref().map(|f| f.message.as_str()), Some("Failed to connect"));

        app.expire(now + FAILURE_SHOWN - Duration::from_millis(1));
        assert!(app.cards()[1].failure.is_some());
        app.expire(now + FAILURE_SHOWN);
        assert!(app.cards()[1].failure.is_none());
    }

    #[test]
    fn next_action_clears_the_old_failure() {
        let mut app = App::demo();
        app.update(run(1));
        app.finish(1, Some("boom".to_string()), Instant::now());
        app.update(run(1));
        assert!(app.cards()[1].failure.is_none());
    }

    #[test]
    fn finish_for_unknown_slot_is_ignored() {
        App::demo().finish(99, Some("boom".to_string()), Instant::now());
    }

    #[test]
    fn card_key_on_another_card_leaves_this_one() {
        let mut app = inside();
        let run = Run { slot: 3, action: "toggle".to_string(), arg: None };
        app.update(Action::Run(run));
        assert_eq!((app.focused(), app.is_inside()), (3, false));

        let mut app = inside();
        app.update(Action::Run(Run { slot: 0, action: "toggle".to_string(), arg: None }));
        assert!(app.is_inside()); // same card: stay
    }
}
