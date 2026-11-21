mod action;
mod app;
pub mod card;
pub mod config;
mod event;
pub mod key;
mod keymap;
pub mod layout;
pub mod refresh;
pub mod registry;
mod ui;

use crate::{
    action::Run,
    app::App,
    card::Card,
    config::Config,
    event::{Event, EventSource, Events},
    keymap::Keymap,
};
use ratatui::{Terminal, backend::Backend};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;

/// Entry point: start the cards, set up the terminal, run the loop, always restore.
/// Which cards exist is the caller's business: `app` builds them from the config via the registry,
/// one card per `[[card]]`, in config order.
pub async fn run(cards: Vec<Arc<dyn Card>>, config: &Config) -> anyhow::Result<()> {
    let placements = config.layout()?;
    let slots = cards.iter().zip(placements).map(|(card, placements)| (card.title().to_string(), placements));
    let mut app = App::new(config.ui.columns, slots);
    let keymap = Keymap::new(&config.cards)?;

    let (tx, rx) = mpsc::channel(64);
    let tasks = refresh::spawn_all(&cards, tx.clone());
    let run_action = |run: Run| {
        if let Some(card) = cards.get(run.slot) {
            refresh::spawn_action(run.slot, card.clone(), run.action, run.arg, tx.clone());
        }
    };

    let mut terminal = ratatui::init();
    let mut events = EventSource::new(Duration::from_millis(250), rx);
    let result = event_loop(&mut terminal, &mut events, &mut app, &keymap, run_action).await;

    ratatui::restore();

    tasks.iter().for_each(|task| task.abort());
    result
}

async fn event_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    events: &mut impl Events,
    app: &mut App,
    keymap: &Keymap,
    mut run_action: impl FnMut(Run),
) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
    while app.is_running() {
        terminal.draw(|frame| ui::render(app, frame))?;

        match events.next().await? {
            Event::Key(key) => {
                if let Some(action) = keymap.map(key, app.is_inside())
                    && let Some(run) = app.update(action)
                {
                    run_action(run);
                }
            }

            Event::Card(update) => app.apply(update),
            Event::Resize | Event::Tick => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::card::{CardView, Item, Line, State};
    use crate::refresh::Update;
    use anyhow::bail;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;

    /// Replays a fixed list of events, then errors (so a test can't hang).
    struct Scripted(VecDeque<Event>);

    impl Scripted {
        fn new(events: impl IntoIterator<Item = Event>) -> Self {
            Self(events.into_iter().collect())
        }
    }

    impl Events for Scripted {
        async fn next(&mut self) -> anyhow::Result<Event> {
            match self.0.pop_front() {
                Some(ev) => Ok(ev),
                None => bail!("script exhausted"),
            }
        }
    }

    fn key(c: char) -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
    }

    fn terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(40, 5)).unwrap()
    }

    #[tokio::test]
    async fn q_ends_the_loop() {
        let mut events = Scripted::new([key('q')]);
        event_loop(&mut terminal(), &mut events, &mut App::demo(), &Keymap::default(), |_| {}).await.unwrap();
    }

    #[tokio::test]
    async fn loop_survives_ticks_resizes_and_unknown_keys() {
        let mut events = Scripted::new([Event::Tick, Event::Resize, key('x'), key('q')]);
        event_loop(&mut terminal(), &mut events, &mut App::demo(), &Keymap::default(), |_| {}).await.unwrap();
        assert!(events.0.is_empty(), "loop stopped before reaching q");
    }

    #[tokio::test]
    async fn loop_keeps_running_without_quit() {
        let mut events = Scripted::new([key('x'), Event::Tick]);
        let err =
            event_loop(&mut terminal(), &mut events, &mut App::demo(), &Keymap::default(), |_| {}).await.unwrap_err();
        assert_eq!(err.to_string(), "script exhausted");
    }

    #[tokio::test]
    async fn card_updates_reach_the_app() {
        let view = CardView::new(State::Ok).line(Line::new("hi"));
        let mut events = Scripted::new([Event::Card(Update { slot: 1, view: view.clone() }), key('q')]);
        let mut app = App::demo();
        event_loop(&mut terminal(), &mut events, &mut app, &Keymap::default(), |_| {}).await.unwrap();
        assert_eq!(app.cards()[1].view, Some(view));
    }

    #[tokio::test]
    async fn card_keys_run_actions() {
        let config = Config::parse(
            "[[card]]\nid = \"a\"\ntype = \"x\"\n[[card]]\nid = \"b\"\ntype = \"x\"\nkeys = { toggle = \"t\", \"toggle:AirPods\" = \"u\" }",
        )
        .unwrap();
        let keymap = Keymap::new(&config.cards).unwrap();
        let mut events = Scripted::new([key('t'), key('u'), key('q')]);
        let mut app = App::demo();

        let mut ran = Vec::new();
        event_loop(&mut terminal(), &mut events, &mut app, &keymap, |run| ran.push(run)).await.unwrap();

        let toggle = |arg: Option<&str>| Run { slot: 1, action: "toggle".to_string(), arg: arg.map(String::from) };
        assert_eq!(ran, [toggle(None), toggle(Some("AirPods"))]);
        assert_eq!(app.focused(), 1);
    }

    #[tokio::test]
    async fn rows_are_picked_inside_a_card() {
        let rows = CardView::new(State::Ok)
            .item(Item::new("aa-01", "AirPods").action("toggle"))
            .item(Item::new("aa-02", "Keyboard").action("toggle"));
        let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let mut events = Scripted::new([
            Event::Card(Update { slot: 0, view: rows }),
            enter,    // go inside
            key('j'), // second row
            key(' '), // run it
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            key('j'), // outside again: moves focus down
            key('q'),
        ]);
        let mut app = App::demo();

        let mut ran = Vec::new();
        event_loop(&mut terminal(), &mut events, &mut app, &Keymap::default(), |run| ran.push(run)).await.unwrap();

        assert_eq!(ran, [Run { slot: 0, action: "toggle".to_string(), arg: Some("aa-02".to_string()) }]);
        assert_eq!((app.is_inside(), app.focused()), (false, 3));
    }
}
