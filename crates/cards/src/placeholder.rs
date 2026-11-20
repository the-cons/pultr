//! Stand-in for card types that are planned but not built yet. Shrinks as real cards arrive.

use std::time::Duration;

use async_trait::async_trait;
use pultr_core::{
    card::{Card, CardView, Line, State},
    config::CardConfig,
};

pub struct Placeholder {
    id: String,
    title: String,
}

impl Placeholder {
    pub fn new(config: &CardConfig) -> Self {
        Self { id: config.id.clone(), title: config.title_or(&config.kind) }
    }
}

#[async_trait]
impl Card for Placeholder {
    fn id(&self) -> &str {
        &self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn refresh_interval(&self) -> Option<Duration> {
        None // nothing changes, one view is enough
    }

    async fn refresh(&self) -> anyhow::Result<CardView> {
        Ok(CardView::new(State::Ok).line(Line::dim("coming soon")))
    }
}
