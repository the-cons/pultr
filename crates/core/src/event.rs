use std::time::Duration;

use anyhow::bail;
use crossterm::event::{Event as TermEvent, EventStream, KeyEvent, KeyEventKind};
use futures::StreamExt;
use tokio::time::{Interval, MissedTickBehavior, interval};

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Resize,
    Tick,
}

pub struct EventSource {
    terminal: EventStream,
    tick: Interval,
}

impl EventSource {
    pub fn new(tick_rate: Duration) -> Self {
        let mut tick = interval(tick_rate);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        Self {
            terminal: EventStream::new(),
            tick,
        }
    }

}

impl Events for EventSource {
    /// Waits for the next event that matters. Never blocks the thread.
    async fn next(&mut self) -> anyhow::Result<Event> {
        loop {
            tokio::select! {
                _ = self.tick.tick() => return Ok(Event::Tick),
                ev = self.terminal.next() => match ev {
                    Some(Ok(ev)) => if let Some(ev) = translate(ev) { return Ok(ev) },
                    Some(Err(e)) => return Err(e.into()),
                    None => bail!("terminal event stream closed"),
                },
            }
        }
    }
}

/// Raw terminal event → our Event. `None` means "ignore it".
fn translate(ev: TermEvent) -> Option<Event> {
    match ev {
        TermEvent::Key(key) if key.kind == KeyEventKind::Press => Some(Event::Key(key)),
        TermEvent::Resize(..) => Some(Event::Resize),
        _ => None, // key release/repeat, mouse, focus, paste
    }
}

pub(crate) trait Events {
    async fn next(&mut self) -> anyhow::Result<Event>;
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

    fn key(kind: KeyEventKind) -> TermEvent {
        let mut k = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        k.kind = kind;
        TermEvent::Key(k)
    }

    #[test]
    fn key_press_passes_through() {
        assert!(matches!(translate(key(KeyEventKind::Press)), Some(Event::Key(_))));
    }

    #[test]
    fn key_release_and_repeat_are_dropped() {
        assert!(translate(key(KeyEventKind::Release)).is_none());
        assert!(translate(key(KeyEventKind::Repeat)).is_none());
    }

    #[test]
    fn resize_passes_through() {
        assert!(matches!(translate(TermEvent::Resize(80, 24)), Some(Event::Resize)));
    }

    #[test]
    fn focus_and_paste_are_dropped() {
        assert!(translate(TermEvent::FocusGained).is_none());
        assert!(translate(TermEvent::Paste("hi".into())).is_none());
    }
}