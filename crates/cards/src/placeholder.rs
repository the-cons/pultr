//! Stand-in for card types that are planned but not built yet. Shrinks as real cards arrive.

use std::time::Duration;

use async_trait::async_trait;
use pultr_core::{
    card::{ActionSpec, Card, CardView, Line, State},
    config::CardConfig,
};

pub struct Placeholder {
    id: String,
    title: String,
    /// Whatever the config put keys on, so example configs load before the real card exists.
    actions: Vec<ActionSpec>,
}

impl Placeholder {
    pub fn new(config: &CardConfig) -> Self {
        let actions =
            config.keys.keys().map(|id| ActionSpec { id: id.clone(), label: id.clone(), key: None }).collect();
        Self { id: config.id.clone(), title: config.title_or(&config.kind), actions }
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

    fn actions(&self) -> Vec<ActionSpec> {
        self.actions.clone()
    }

    fn refresh_interval(&self) -> Option<Duration> {
        None // nothing changes, one view is enough
    }

    async fn refresh(&self) -> anyhow::Result<CardView> {
        Ok(CardView::new(State::Ok).line(Line::dim("coming soon")))
    }

    async fn run(&self, _action: &str, _arg: Option<&str>) -> anyhow::Result<()> {
        Ok(()) // nothing to do yet; the card stays "coming soon"
    }
}
