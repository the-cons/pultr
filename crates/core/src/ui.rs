use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Layout, Rect};
use ratatui::prelude::Constraint;
use ratatui::widgets::Block;

const COLUMNS: usize = 3;

/// Pure view: reads `App`, draws a frame. Never mutates state.
pub fn render(app: &App, frame: &mut Frame) {
    let outer = Block::bordered().title(" pultr ").title_bottom(" q quit ");
    let inner = outer.inner(frame.area());
    frame.render_widget(outer, frame.area());

    let cells = grid(inner, app.cards().len(), COLUMNS);
    for (title, cell) in app.cards().iter().zip(cells) {
        let card = Block::bordered().title(format!(" {title} "));
        frame.render_widget(card, cell);
    }
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

    fn draw(width: u16, height: u16) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| render(&App::new(), f)).unwrap();
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
}
