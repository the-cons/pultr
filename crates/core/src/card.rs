//! The card contract: what every card (built-in, shell, plugin) must provide.
//!
//! Cards never draw. They return a [`CardView`] describing *what* to show;
//! the core decides *how* it looks, so every card stays consistent.

use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait Card: Send + Sync {
    /// Stable id, used in action addresses like `airpods.toggle`.
    fn id(&self) -> &str;

    /// Shown in the card's border.
    fn title(&self) -> &str;

    /// Actions this card offers. Status-only cards keep the default.
    fn actions(&self) -> Vec<ActionSpec> {
        Vec::new()
    }

    /// How often to call [`Card::refresh`]. `None` = the card pushes updates itself.
    fn refresh_interval(&self) -> Option<Duration>;

    /// Ask the system for the current state.
    async fn refresh(&self) -> anyhow::Result<CardView>;

    /// Run one of the actions from [`Card::actions`]. `arg` is what came after the `:`
    /// (`AirPods Pro` in `toggle:AirPods Pro`); most actions take none.
    async fn run(&self, action: &str, _arg: Option<&str>) -> anyhow::Result<()> {
        anyhow::bail!("card `{}` has no action `{action}`", self.id())
    }
}

/// Describes one action a card offers.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ActionSpec {
    pub id: String,
    pub label: String,
    /// Default key; config can override it later.
    pub key: Option<char>,
}

/// Overall condition of a card. Drives the border color / state dot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Ok,
    Active,
    Warning,
    Error,
    Unavailable,
}

/// How a line of text should stand out.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum Emphasis {
    #[default]
    Normal,
    Strong,
    Dim,
}

/// One line of card text. Our own type, not ratatui's, so plugins can send it later.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub emphasis: Emphasis,
}

impl Line {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), emphasis: Emphasis::Normal }
    }

    pub fn strong(text: impl Into<String>) -> Self {
        Self { text: text.into(), emphasis: Emphasis::Strong }
    }

    pub fn dim(text: impl Into<String>) -> Self {
        Self { text: text.into(), emphasis: Emphasis::Dim }
    }
}

/// One row of a card's list: a paired device, an audio output…
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Stable across refreshes; actions on this row get it as their argument.
    pub id: String,
    /// Left side: what it is.
    pub label: String,
    /// Right side: how it is, e.g. `connected`. Coloured by `state`.
    pub detail: String,
    pub state: State,
}

impl Item {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self { id: id.into(), label: label.into(), detail: String::new(), state: State::Ok }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }
}

/// Structured content of a card: what to show, not how.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CardView {
    pub state: State,
    pub lines: Vec<Line>,
    /// Progress bar, 0.0-1.0.
    pub gauge: Option<f32>,
    /// A list below the lines, one row per item.
    pub items: Vec<Item>,
}

impl CardView {
    pub fn new(state: State) -> Self {
        Self { state, ..Self::default() }
    }

    /// Something went wrong; the message is shown inside the card.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(State::Error).line(Line::new(message))
    }

    /// The feature doesn't exist here (missing tool, wrong OS…).
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(State::Unavailable).line(Line::dim(reason))
    }

    pub fn line(mut self, line: Line) -> Self {
        self.lines.push(line);
        self
    }

    pub fn item(mut self, item: Item) -> Self {
        self.items.push(item);
        self
    }

    /// Clamped to 0.0–1.0; NaN becomes 0.0.
    pub fn gauge(mut self, ratio: f32) -> Self {
        self.gauge = Some(if ratio.is_nan() { 0.0 } else { ratio.clamp(0.0, 1.0) });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn builder_collects_lines_in_order() {
        let view = CardView::new(State::Active).line(Line::strong("82 %")).line(Line::new("charging"));
        assert_eq!(view.state, State::Active);
        assert_eq!(view.lines, vec![Line::strong("82 %"), Line::new("charging")]);
    }

    #[test]
    fn items_keep_their_order_and_fields() {
        let view = CardView::new(State::Ok)
            .item(Item::new("aa-01", "AirPods").detail("connected").state(State::Active))
            .item(Item::new("aa-02", "Keyboard"));

        assert_eq!(
            view.items[0],
            Item {
                id: "aa-01".to_string(),
                label: "AirPods".to_string(),
                detail: "connected".to_string(),
                state: State::Active,
            }
        );
        assert_eq!((view.items[1].detail.as_str(), view.items[1].state), ("", State::Ok)); // defaults
    }

    #[test]
    fn gauge_is_clamped() {
        assert_eq!(CardView::default().gauge(0.5).gauge, Some(0.5));
        assert_eq!(CardView::default().gauge(-1.0).gauge, Some(0.0));
        assert_eq!(CardView::default().gauge(7.0).gauge, Some(1.0));
        assert_eq!(CardView::default().gauge(f32::NAN).gauge, Some(0.0));
    }

    #[test]
    fn error_and_unavailable_carry_their_message() {
        let err = CardView::error("timed out");
        assert_eq!(err.state, State::Error);
        assert_eq!(err.lines, vec![Line::new("timed out")]);

        let na = CardView::unavailable("blue util not found");
        assert_eq!(na.state, State::Unavailable);
        assert_eq!(na.lines[0].emphasis, Emphasis::Dim);
    }

    /// Minimal card: proves the trait works behind `Arc<dyn Card>`.
    struct Fixed;

    #[async_trait]
    impl Card for Fixed {
        fn id(&self) -> &str {
            "fixed"
        }
        fn title(&self) -> &str {
            "Fixed"
        }
        fn refresh_interval(&self) -> Option<Duration> {
            Some(Duration::from_secs(1))
        }
        async fn refresh(&self) -> anyhow::Result<CardView> {
            Ok(CardView::new(State::Ok).line(Line::new("hello")))
        }
    }

    #[tokio::test]
    async fn card_works_as_trait_object() {
        let card: Arc<dyn Card> = Arc::new(Fixed);
        let view = card.refresh().await.unwrap();
        assert_eq!(view.lines, vec![Line::new("hello")]);
        assert!(card.actions().is_empty());
    }

    #[tokio::test]
    async fn unknown_action_is_an_error() {
        let err = Fixed.run("explode", None).await.unwrap_err();
        assert_eq!(err.to_string(), "card `fixed` has no action `explode`");
    }
}
