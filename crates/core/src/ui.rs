use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::widgets::{Block, Paragraph};

/// Pure view: reads `App`, draws a frame. Never mutates state.
pub fn render(_app: &App, frame: &mut Frame) {
    let text = Paragraph::new("press q to quit")
        .alignment(Alignment::Center)
        .block(Block::bordered().title(" pultr "));

    frame.render_widget(text, frame.area());
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn draw(width: u16, height: u16) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| render(&App::new(), f)).unwrap();
        terminal
    }

    #[test]
    fn renders_frame() {
        insta::assert_snapshot!(draw(40, 5).backend());
    }

    #[test]
    fn survives_tiny_terminal() {
        // Rendering into a 1x1 or 0x0 area must not panic.
        draw(1, 1);
        draw(0, 0);
    }
}
