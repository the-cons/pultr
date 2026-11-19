use crate::app::{App, COLUMNS};
use crate::card::{self, CardView, Emphasis};
use ratatui::Frame;
use ratatui::layout::{Layout, Rect};
use ratatui::prelude::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};

/// Pure view: reads `App`, draws a frame. Never mutates state.
pub fn render(app: &App, frame: &mut Frame) {
    let outer = Block::bordered().title(" pultr ").title_bottom(" q quit ");
    let inner = outer.inner(frame.area());
    frame.render_widget(outer, frame.area());

    let cells = grid(inner, app.cards().len(), COLUMNS);
    for (i, (slot, cell)) in app.cards().iter().zip(cells).enumerate() {
        let mut block = Block::bordered().title(format!(" {} ", slot.title));
        if i == app.focused() {
            block = block.border_type(BorderType::Thick).border_style(Style::new().fg(Color::Cyan));
        }

        let body = block.inner(cell);
        frame.render_widget(block, cell);
        if let Some(view) = &slot.view {
            render_view(view, body, frame);
        }
    }
}

/// The inside of one card. Lines only for now.
fn render_view(view: &CardView, area: Rect, frame: &mut Frame) {
    let lines: Vec<Line> = view.lines.iter().map(to_tui_line).collect();
    frame.render_widget(Paragraph::new(lines), area);
}

/// Our `card::Line` -> ratatui's `Line`. Same name, different jobs.
fn to_tui_line(line: &card::Line) -> Line<'_> {
    let style = match line.emphasis {
        Emphasis::Normal => Style::new(),
        Emphasis::Strong => Style::new().add_modifier(Modifier::BOLD),
        Emphasis::Dim => Style::new().fg(Color::DarkGray),
    };

    Line::styled(line.text.as_str(), style)
}

/// Splits `area` into `count` equal cells
/// The last row may be partly empty.
fn grid(area: Rect, count: usize, columns: usize) -> Vec<Rect> {
    if count == 0 || columns == 0 {
        return Vec::new();
    }
    let rows = count.div_ceil(columns);
    let row_areas = Layout::vertical(vec![Constraint::Fill(1); rows]).split(area);
    row_areas
        .iter()
        .flat_map(|row| Layout::horizontal(vec![Constraint::Fill(1); columns]).split(*row).to_vec())
        .take(count)
        .collect()
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::action::{Action, Direction};
    use crate::card::State;
    use crate::refresh::Update;

    fn draw(width: u16, height: u16) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| render(&App::demo(), f)).unwrap();
        terminal
    }

    #[test]
    fn renders_frame() {
        insta::assert_snapshot!(draw(60, 14).backend());
    }

    #[test]
    fn survives_tiny_terminal() {
        // Rendering into a 1x1 or 0x0 area must not panic.
        draw(1, 1);
        draw(0, 0);
    }

    #[test]
    fn grid_has_one_cell_per_card() {
        let area = Rect::new(0, 0, 90, 30);
        assert_eq!(grid(area, 6, 3).len(), 6);
        assert_eq!(grid(area, 4, 3).len(), 4); // second row only half full
        assert!(grid(area, 0, 3).is_empty());
        assert!(grid(area, 6, 0).is_empty());
    }

    #[test]
    fn grid_fills_rows_left_to_right() {
        let cells = grid(Rect::new(0, 0, 90, 30), 6, 3);
        // first row: same y, increasing x
        assert_eq!(cells[0].y, cells[2].y);
        assert!(cells[0].x < cells[1].x && cells[1].x < cells[2].x);
        // second row starts below the first, back at the left edge
        assert!(cells[3].y > cells[0].y);
        assert_eq!(cells[3].x, cells[0].x);
    }

    #[test]
    fn grid_cells_stay_inside_the_area() {
        let area = Rect::new(2, 1, 61, 17); // odd sizes on purpose
        for cell in grid(area, 5, 3) {
            assert_eq!(area.intersection(cell), cell, "{cell:?} leaks out of {area:?}");
        }
    }

    #[test]
    fn focused_card_has_thick_border() {
        let mut app = App::demo();
        app.update(Action::Move(Direction::Right));
        let mut terminal = Terminal::new(TestBackend::new(60, 14)).unwrap();
        terminal.draw(|f| render(&app, f)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn card_shows_its_lines() {
        let mut app = App::demo();
        let view = CardView::new(State::Ok).line(card::Line::strong("82 %")).line(card::Line::dim("charging"));
        app.apply(Update { slot: 4, view });
        let mut terminal = Terminal::new(TestBackend::new(60, 14)).unwrap();
        terminal.draw(|f| render(&app, f)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }
}
