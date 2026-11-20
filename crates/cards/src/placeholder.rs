//! Stand-in for cards that don't exist yet. Shrinks as real cards arrive; gone after M3.

use std::time::Duration;

use async_trait::async_trait;
use pultr_core::card::{Card, CardView, Line, State};

pub struct Placeholder(pub &'static str);

#[async_trait]
impl Card for Placeholder {
    fn id(&self) -> &str {
        self.0
    }

    fn title(&self) -> &str {
        self.0
    }

    fn refresh_interval(&self) -> Option<Duration> {
        None // nothing changes, one view is enough
    }

    async fn refresh(&self) -> anyhow::Result<CardView> {
        Ok(CardView::new(State::Ok).line(Line::dim("coming soon")))
    }
}
