use crate::app::{App, COLUMNS};
use crate::card::{self, CardView, Emphasis, State};
use ratatui::Frame;
use ratatui::layout::{Layout, Rect};
use ratatui::prelude::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, LineGauge, Paragraph};

/// Pure view: reads `App`, draws a frame. Never mutates state.
pub fn render(app: &App, frame: &mut Frame) {
    let outer = Block::bordered().title(" pultr ").title_bottom(" q quit ");
    let inner = outer.inner(frame.area());
    frame.render_widget(outer, frame.area());

    let cells = grid(inner, app.cards().len(), COLUMNS);
    for (i, (slot, cell)) in app.cards().iter().zip(cells).enumerate() {
        let mut block = Block::bordered().title(format!(" {} ", slot.title));

        if let Some(view) = &slot.view {
            block = block.title_top(Line::styled(" ● ", Style::new().fg(state_color(view.state))).right_aligned());
        }
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

/// The inside of one card.
fn render_view(view: &CardView, area: Rect, frame: &mut Frame) {
    let lines: Vec<Line> = view.lines.iter().map(to_tui_line).collect();
    let text_height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let [text_area, gauge_area] =
        Layout::vertical([Constraint::Length(text_height), Constraint::Length(1)]).areas(area);

    frame.render_widget(Paragraph::new(lines), text_area);
    if let Some(ratio) = view.gauge {
        let gauge = LineGauge::default()
            .ratio(safe_ratio(ratio))
            .label("")
            .filled_style(Style::new().fg(state_color(view.state)));
        frame.render_widget(gauge, gauge_area);
    }
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

/// One color per state, used for the dot and the gauge.
fn state_color(state: State) -> Color {
    match state {
        State::Ok => Color::Green,
        State::Active => Color::Blue,
        State::Warning => Color::Yellow,
        State::Error => Color::Red,
        State::Unavailable => Color::DarkGray,
    }
}

/// `LineGauge` panics outside 0.0–1.0. `CardView::gauge()` clamps, but the field is
/// public, so the renderer protects itself too: a bad card must never crash the app.
fn safe_ratio(ratio: f32) -> f64 {
    if ratio.is_nan() { 0.0 } else { f64::from(ratio).clamp(0.0, 1.0) }
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

    #[test]
    fn gauge_ratio_is_always_safe() {
        assert_eq!(safe_ratio(0.5), 0.5);
        assert_eq!(safe_ratio(-3.0), 0.0);
        assert_eq!(safe_ratio(3.0), 1.0);
        assert_eq!(safe_ratio(f32::NAN), 0.0);
    }

    #[test]
    fn broken_gauge_value_does_not_panic() {
        let mut app = App::demo();
        let view = CardView { gauge: Some(f32::INFINITY), ..CardView::default() }; // bypasses the clamp
        app.apply(Update { slot: 0, view });
        let mut terminal = Terminal::new(TestBackend::new(60, 14)).unwrap();
        terminal.draw(|f| render(&app, f)).unwrap();
    }
}
