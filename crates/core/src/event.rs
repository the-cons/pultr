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

    /// Waits for the next event that matters. Never blocks the thread.
    pub async fn next(&mut self) -> anyhow::Result<Event> {
        loop {
            tokio::select! {
                _ = self.tick.tick() => return Ok(Event::Tick),
                ev = self.terminal.next() => match ev {
                    Some(Ok(TermEvent::Key(key))) if key.kind == KeyEventKind::Press => {
                        return Ok(Event::Key(key));
                    }
                    Some(Ok(TermEvent::Resize(..))) => return Ok(Event::Resize),
                    Some(Ok(_)) => continue,
                    Some(Err(e)) => return Err(e.into()),
                    None => bail!("terminal event stream closed"),
                },
            }
        }
    }
}
