use crate::app::App;
use crate::card::{self, CardView, Emphasis, Item, State};
use crate::layout::Placement;
use ratatui::Frame;
use ratatui::layout::{Layout, Rect};
use ratatui::prelude::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, LineGauge, Paragraph};

/// Pure view: reads `App`, draws a frame. Never mutates state.
pub fn render(app: &App, frame: &mut Frame) {
    let hint = if app.is_inside() { " j/k select · space run · esc back " } else { " q quit " };
    let outer = Block::bordered().title(" pultr ").title_bottom(hint);
    let inner = outer.inner(frame.area());
    frame.render_widget(outer, frame.area());

    let placements: Vec<Placement> = app.cards().iter().map(|slot| slot.placement).collect();
    let cells = grid(inner, app.columns(), &placements);
    for (i, (slot, cell)) in app.cards().iter().zip(cells).enumerate() {
        let mut block = Block::bordered().title(format!(" {} ", slot.title));

        if let Some(view) = &slot.view {
            block = block.title_top(Line::styled(" ● ", Style::new().fg(state_color(view.state))).right_aligned());
        }
        let inside = i == app.focused() && app.is_inside();
        if i == app.focused() {
            let color = if inside { Color::Magenta } else { Color::Cyan };
            block = block.border_type(BorderType::Thick).border_style(Style::new().fg(color));
        }

        // Bottom border: what's going on with actions. Running beats an old failure.
        if slot.busy > 0 {
            block = block.title_bottom(Line::styled(" working… ", Style::new().fg(Color::Blue)));
        } else if let Some(failure) = &slot.failure {
            block = block.title_bottom(Line::styled(format!(" {} ", failure.message), Style::new().fg(Color::Red)));
        }

        let body = block.inner(cell);
        frame.render_widget(block, cell);
        if let Some(view) = &slot.view {
            render_view(view, inside.then_some(slot.selected), body, frame);
        }
    }
}

/// The inside of one card. `selected`: the row to mark, only while the user is inside this card.
fn render_view(view: &CardView, selected: Option<usize>, area: Rect, frame: &mut Frame) {
    let lines: Vec<Line> = view.lines.iter().map(to_tui_line).collect();
    let text_height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let gauge_height = u16::from(view.gauge.is_some());
    let [text_area, gauge_area, list_area] =
        Layout::vertical([Constraint::Length(text_height), Constraint::Length(gauge_height), Constraint::Fill(1)])
            .areas(area);

    frame.render_widget(Paragraph::new(lines), text_area);
    if let Some(ratio) = view.gauge {
        let gauge = LineGauge::default()
            .ratio(safe_ratio(ratio))
            .label("")
            .filled_style(Style::new().fg(state_color(view.state)));
        frame.render_widget(gauge, gauge_area);
    }

    render_items(&view.items, selected, list_area, frame);
}

/// One row per item: label on the left, detail on the right. The selected row gets a `▸` and bold text.
/// More rows than space: the list scrolls just enough to keep the selected row in view.
fn render_items(items: &[Item], selected: Option<usize>, area: Rect, frame: &mut Frame) {
    let skip = scroll(selected, usize::from(area.height));
    for ((i, item), row) in items.iter().enumerate().skip(skip).zip(area.rows()) {
        let is_selected = selected == Some(i);
        let detail = Line::styled(item.detail.as_str(), detail_style(item.state));
        let detail_width = u16::try_from(detail.width()).unwrap_or(u16::MAX);
        let [gutter, label_area, detail_area] =
            Layout::horizontal([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(detail_width)])
                .spacing(1)
                .areas(row);

        let label_style = if is_selected { Style::new().add_modifier(Modifier::BOLD) } else { Style::new() };
        if is_selected {
            frame.render_widget(Line::styled("▸", Style::new().fg(Color::Magenta)), gutter);
        }
        frame.render_widget(Line::styled(item.label.as_str(), label_style), label_area);
        frame.render_widget(detail, detail_area);
    }
}

/// How many rows to skip so row `selected` is visible in `height` rows. Pure, so it's easy to test.
fn scroll(selected: Option<usize>, height: usize) -> usize {
    match selected {
        Some(i) if height > 0 => (i + 1).saturating_sub(height),
        _ => 0,
    }
}

/// `Ok` is the quiet normal case for a row, so it stays grey. Everything else gets its state colour.
fn detail_style(state: State) -> Style {
    match state {
        State::Ok => Style::new().fg(Color::DarkGray),
        other => Style::new().fg(state_color(other)),
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

/// One screen area per placement. Columns share the width equally, rows the height;
/// a spanning card gets the union of its cells.
fn grid(area: Rect, columns: usize, placements: &[Placement]) -> Vec<Rect> {
    let rows = placements.iter().map(|p| p.row + p.rows).max().unwrap_or(0);
    if columns == 0 || rows == 0 {
        return Vec::new();
    }
    let col_areas = Layout::horizontal(vec![Constraint::Fill(1); columns]).split(area);
    let row_areas = Layout::vertical(vec![Constraint::Fill(1); rows]).split(area);

    // Clamped: `layout::place` never produces out-of-grid cells, but a bad placement must not crash the app.
    let cell = |col: usize, row: usize| {
        let (c, r) = (col_areas[col.min(columns - 1)], row_areas[row.min(rows - 1)]);
        Rect::new(c.x, r.y, c.width, r.height)
    };
    placements
        .iter()
        .map(|p| {
            let (last_col, last_row) = (p.col + p.cols.max(1) - 1, p.row + p.rows.max(1) - 1);
            cell(p.col, p.row).union(cell(last_col, last_row))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};
    use std::time::Instant;

    use super::*;
    use crate::action::{Action, Direction, Run};
    use crate::card::State;
    use crate::layout::row_major;
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
        assert_eq!(grid(area, 3, &row_major(6, 3)).len(), 6);
        assert_eq!(grid(area, 3, &row_major(4, 3)).len(), 4); // second row only half full
        assert!(grid(area, 3, &[]).is_empty());
        assert!(grid(area, 0, &row_major(6, 3)).is_empty());
    }

    #[test]
    fn grid_fills_rows_left_to_right() {
        let cells = grid(Rect::new(0, 0, 90, 30), 3, &row_major(6, 3));
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
        for cell in grid(area, 3, &row_major(5, 3)) {
            assert_eq!(area.intersection(cell), cell, "{cell:?} leaks out of {area:?}");
        }
    }

    #[test]
    fn spanning_card_covers_its_cells() {
        //   w w a
        //   b t .
        //   . t .
        let at = |col, row, cols, rows| Placement { col, row, cols, rows };
        let placements = [at(0, 0, 2, 1), at(2, 0, 1, 1), at(0, 1, 1, 1), at(1, 1, 1, 2)];
        let cells = grid(Rect::new(0, 0, 90, 30), 3, &placements);

        assert_eq!(cells[0], Rect::new(0, 0, 60, 10)); // wide: two columns
        assert_eq!(cells[1], Rect::new(60, 0, 30, 10));
        assert_eq!(cells[3], Rect::new(30, 10, 30, 20)); // tall: two rows
    }

    #[test]
    fn renders_spanning_cards() {
        let titles = ["wide", "a", "b", "tall"].map(String::from);
        let at = |col, row, cols, rows| Placement { col, row, cols, rows };
        let placements = [at(0, 0, 2, 1), at(2, 0, 1, 1), at(0, 1, 1, 1), at(1, 1, 1, 2)];
        let app = App::new(3, titles.into_iter().zip(placements));

        let mut terminal = Terminal::new(TestBackend::new(60, 14)).unwrap();
        terminal.draw(|f| render(&app, f)).unwrap();
        insta::assert_snapshot!(terminal.backend());
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
    fn card_shows_its_items() {
        let mut app = App::demo();
        let view = CardView::new(State::Active)
            .line(card::Line::strong("on"))
            .item(card::Item::new("1", "AirPods Pro").detail("not connected"))
            .item(card::Item::new("2", "Magic Keyboard").detail("connected").state(State::Active))
            .item(card::Item::new("3", "A device with a really long name").detail("connected"))
            .item(card::Item::new("4", "Cut off: no room left"));
        app.apply(Update { slot: 0, view });
        let mut terminal = Terminal::new(TestBackend::new(90, 14)).unwrap();
        terminal.draw(|f| render(&app, f)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn inside_a_card_the_selected_row_is_marked() {
        let mut app = App::demo();
        let mut view = CardView::new(State::Active).line(card::Line::strong("on"));
        for (id, name) in [("1", "AirPods Pro"), ("2", "Magic Keyboard"), ("3", "MX Master 3S"), ("4", "Speaker")] {
            view = view.item(card::Item::new(id, name).detail("connected").action("toggle"));
        }
        app.apply(Update { slot: 0, view });
        app.update(Action::Enter);
        for _ in 0..3 {
            app.update(Action::SelectNext); // onto row 4: one more than fits, so the list scrolls by one
        }

        let mut terminal = Terminal::new(TestBackend::new(90, 14)).unwrap();
        terminal.draw(|f| render(&app, f)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn busy_and_failed_cards_say_so() {
        let mut app = App::demo();
        let run = |slot| Action::Run(Run { slot, action: "toggle".to_string(), arg: None });
        app.update(run(0)); // still running
        app.update(run(1));
        app.finish(1, Some("Failed to connect \"aa-bb-cc-dd-ee-02\"".to_string()), Instant::now());
        app.update(run(2)); // fine: nothing to show
        app.finish(2, None, Instant::now());

        let mut terminal = Terminal::new(TestBackend::new(90, 14)).unwrap();
        terminal.draw(|f| render(&app, f)).unwrap();
        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn scroll_keeps_the_selection_visible() {
        assert_eq!(scroll(None, 3), 0);
        assert_eq!(scroll(Some(0), 3), 0);
        assert_eq!(scroll(Some(2), 3), 0); // last visible row
        assert_eq!(scroll(Some(3), 3), 1);
        assert_eq!(scroll(Some(9), 3), 7);
        assert_eq!(scroll(Some(5), 0), 0); // no room at all: nothing to scroll
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
