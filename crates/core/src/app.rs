use crate::action::Action;

/// Hard-coded until cards come from config.
const PLACEHOLDER_CARDS: [&str; 6] =
    ["bluetooth", "sidecar", "audio", "media", "battery", "system"];

/// All UI state lives here. Nothing in here touches the terminal or the OS.
pub struct App {
    running: bool,
    cards: Vec<&'static str>,
}

impl App {
    pub fn new() -> Self {
        Self { running: true, cards: PLACEHOLDER_CARDS.to_vec() }
    }
    pub fn is_running(&self) -> bool {
        self.running
    }
    pub fn cards(&self) -> &[&'static str] {
        &self.cards
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_running() {
        assert!(App::new().is_running());
    }

    #[test]
    fn quit_stops_the_app() {
        let mut app = App::new();
        app.update(Action::Quit);
        assert!(!app.is_running());
    }

    #[test]
    fn starts_with_placeholder_cards() {
        assert_eq!(App::new().cards().len(), 6);
    }
}
