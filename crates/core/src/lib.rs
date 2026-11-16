mod action;
mod app;
mod event;
mod keymap;
mod ui;

use std::time::Duration;
use ratatui::DefaultTerminal;
use crate::{app::App, event::{Event, EventSource}};

/// Entry point: set up the terminal, run the loop, always restore.
pub async fn run() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal).await;
    ratatui::restore();
    result
}

async fn event_loop(terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
    let mut app = App::new();
    let mut events = EventSource::new(Duration::from_millis(250));

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
