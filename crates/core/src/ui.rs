use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::widgets::{Block, Paragraph};
use crate::app::App;

/// Pure view: reads `App`, draws a frame. Never mutates state.
pub fn render(_app: &App, frame: &mut Frame) {
    let text = Paragraph::new("press q to quit")
        .alignment(Alignment::Center)
        .block(Block::bordered().title(" pultr "));

    frame.render_widget(text, frame.area());
}
