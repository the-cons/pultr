//! Stand-in cards until real ones exist. Shrinks as `cards` grows.

use crate::card::{Card, CardView, Line, State};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};

const IDS: [&str; 6] = ["bluetooth", "sidecar", "audio", "media", "battery", "system"];

pub fn cards() -> Vec<Arc<dyn Card>> {
    IDS.into_iter().map(|id| Arc::new(Placeholder(id)) as Arc<dyn Card>).collect()
}

struct Placeholder(&'static str);

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
