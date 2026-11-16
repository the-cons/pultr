use crate::action::Action;

/// All UI state lives here. Nothing in here touches the terminal or the OS.
pub struct App {
    running: bool,
}

impl App {
    pub fn new() -> Self {
        Self { running: true }
    }
    pub fn is_running(&self) -> bool {
        self.running
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
}