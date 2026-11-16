use crate::action::Action;

/// All UI state lives here. Nothing in here touches the terminal or the OS.
pub struct App {
    running: bool,
}

impl App {
    pub fn new() -> Self { Self { running: true } }
    pub fn is_running(&self) -> bool { self.running }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
        }
    }
}
