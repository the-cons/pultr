mod action;
mod app;
pub mod card;
mod event;
mod keymap;
mod ui;

use crate::{
    app::App,
    event::{Event, EventSource, Events},
};
use ratatui::{Terminal, backend::Backend};
use std::time::Duration;

/// Entry point: set up the terminal, run the loop, always restore.
pub async fn run() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let mut events = EventSource::new(Duration::from_millis(250));
    let result = event_loop(&mut terminal, &mut events).await;
    ratatui::restore();
    result
}

async fn event_loop<B: Backend>(terminal: &mut Terminal<B>, events: &mut impl Events) -> anyhow::Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut app = App::new();
    while app.is_running() {
        terminal.draw(|frame| ui::render(&app, frame))?;

        match events.next().await? {
            Event::Key(key) => {
                if let Some(action) = keymap::map(key) {
                    app.update(action);
                }
            }

            Event::Resize | Event::Tick => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use anyhow::bail;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;

    use super::*;

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
        event_loop(&mut terminal(), &mut events).await.unwrap();
    }

    #[tokio::test]
    async fn loop_survives_ticks_resizes_and_unknown_keys() {
        let mut events = Scripted::new([Event::Tick, Event::Resize, key('x'), key('q')]);
        event_loop(&mut terminal(), &mut events).await.unwrap();
        assert!(events.0.is_empty(), "loop stopped before reaching q");
    }

    #[tokio::test]
    async fn loop_keeps_running_without_quit() {
        let mut events = Scripted::new([key('x'), Event::Tick]);
        let err = event_loop(&mut terminal(), &mut events).await.unwrap_err();
        assert_eq!(err.to_string(), "script exhausted");
    }
}
