mod action;
mod app;
pub mod card;
pub mod config;
mod event;
mod keymap;
pub mod refresh;
pub mod registry;
mod ui;

use crate::{
    app::App,
    card::Card,
    event::{Event, EventSource, Events},
};
use ratatui::{Terminal, backend::Backend};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;

/// Entry point: start the cards, set up the terminal, run the loop, always restore.
/// Which cards exist is the caller's business: `app` builds them from the config via the registry.
pub async fn run(cards: Vec<Arc<dyn Card>>) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel(64);
    let tasks = refresh::spawn_all(&cards, tx);
    let mut app = App::new(cards.iter().map(|card| card.title().to_string()));

    let mut terminal = ratatui::init();
    let mut events = EventSource::new(Duration::from_millis(250), rx);
    let result = event_loop(&mut terminal, &mut events, &mut app).await;
    ratatui::restore();

    tasks.iter().for_each(|task| task.abort());
    result
}

async fn event_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    events: &mut impl Events,
    app: &mut App,
) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
    while app.is_running() {
        terminal.draw(|frame| ui::render(app, frame))?;

        match events.next().await? {
            Event::Key(key) => {
                if let Some(action) = keymap::map(key) {
                    app.update(action);
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
    use crate::card::{CardView, Line, State};
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
        event_loop(&mut terminal(), &mut events, &mut App::demo()).await.unwrap();
    }

    #[tokio::test]
    async fn loop_survives_ticks_resizes_and_unknown_keys() {
        let mut events = Scripted::new([Event::Tick, Event::Resize, key('x'), key('q')]);
        event_loop(&mut terminal(), &mut events, &mut App::demo()).await.unwrap();
        assert!(events.0.is_empty(), "loop stopped before reaching q");
    }

    #[tokio::test]
    async fn loop_keeps_running_without_quit() {
        let mut events = Scripted::new([key('x'), Event::Tick]);
        let err = event_loop(&mut terminal(), &mut events, &mut App::demo()).await.unwrap_err();
        assert_eq!(err.to_string(), "script exhausted");
    }

    #[tokio::test]
    async fn card_updates_reach_the_app() {
        let view = CardView::new(State::Ok).line(Line::new("hi"));
        let mut events = Scripted::new([Event::Card(Update { slot: 1, view: view.clone() }), key('q')]);
        let mut app = App::demo();
        event_loop(&mut terminal(), &mut events, &mut app).await.unwrap();
        assert_eq!(app.cards()[1].view, Some(view));
    }
}
