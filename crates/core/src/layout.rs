//! Where cards sit: `pos` and `span` from the config → a cell rectangle per card.
//!
//! Pure grid math in cell units. Turning cells into screen areas is `ui`'s job.

use std::{collections::HashMap, ops::Range};

use anyhow::{bail, ensure};

use crate::{action::Direction, config::CardConfig};

/// More than this makes no sense in a terminal, and keeps a typo like `pos = [0, 9999]` harmless.
pub const MAX_COLUMNS: usize = 12;
pub const MAX_ROWS: usize = 12;

/// A card's rectangle on the grid, in cells. `col`/`row` are 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub col: usize,
    pub row: usize,
    pub cols: usize,
    pub rows: usize,
}

impl Placement {
    pub fn col_range(self) -> Range<usize> {
        self.col..self.col + self.cols
    }

    pub fn row_range(self) -> Range<usize> {
        self.row..self.row + self.rows
    }

    /// Every `(col, row)` the card covers.
    fn cells(self) -> impl Iterator<Item = (usize, usize)> {
        self.row_range().flat_map(move |row| self.col_range().map(move |col| (col, row)))
    }

    /// How far `to` is when moving in `dir`: (steps along `dir`, steps sideways).
    /// `None` if `to` doesn't lie in that direction. Smaller is closer.
    pub(crate) fn distance(self, to: Placement, dir: Direction) -> Option<(usize, usize)> {
        match dir {
            Direction::Right => {
                Some((ahead(self.col_range(), to.col_range())?, apart(self.row_range(), to.row_range())))
            }
            Direction::Left => {
                Some((ahead(to.col_range(), self.col_range())?, apart(self.row_range(), to.row_range())))
            }
            Direction::Down => {
                Some((ahead(self.row_range(), to.row_range())?, apart(self.col_range(), to.col_range())))
            }
            Direction::Up => Some((ahead(to.row_range(), self.row_range())?, apart(self.col_range(), to.col_range()))),
        }
    }
}

/// Steps from `a` forward to `b`. `None` unless `b` lies entirely after `a`.
fn ahead(a: Range<usize>, b: Range<usize>) -> Option<usize> {
    (b.start >= a.end).then(|| b.start - a.end + 1)
}

/// Steps between two ranges on the other axis. 0 if they overlap.
fn apart(a: Range<usize>, b: Range<usize>) -> usize {
    ahead(a.clone(), b.clone()).or_else(|| ahead(b, a)).unwrap_or(0)
}

/// One placement per card, in config order.
///
/// Cards with `pos` claim their cells first. The rest flow into free cells
/// left to right, top to bottom, keeping their config order (like CSS grid).
pub fn place(cards: &[CardConfig], columns: usize) -> anyhow::Result<Vec<Placement>> {
    ensure!((1..=MAX_COLUMNS).contains(&columns), "ui.columns must be between 1 and {MAX_COLUMNS}");

    let mut owner: HashMap<(usize, usize), usize> = HashMap::new(); // cell → card index
    let mut placed: Vec<Option<Placement>> = vec![None; cards.len()];

    for (i, card) in cards.iter().enumerate() {
        let Some([col, row]) = card.pos else { continue };
        let p = Placement { col, row, cols: card.span[0], rows: card.span[1] };
        check_bounds(card, p, columns)?;
        if let Some(&other) = p.cells().find_map(|cell| owner.get(&cell)) {
            bail!("card `{}` overlaps card `{}`", card.id, cards[other].id);
        }
        owner.extend(p.cells().map(|cell| (cell, i)));
        placed[i] = Some(p);
    }

    let mut cursor = 0; // next cell to try, counted row by row: row * columns + col
    for (i, card) in cards.iter().enumerate().filter(|(_, card)| card.pos.is_none()) {
        let [cols, rows] = card.span;
        check_bounds(card, Placement { col: 0, row: 0, cols, rows }, columns)?;

        let p = (cursor..)
            .map(|k| Placement { col: k % columns, row: k / columns, cols, rows })
            .find(|p| p.col + p.cols <= columns && p.cells().all(|cell| !owner.contains_key(&cell)))
            .expect("rows below every card are free");
        check_bounds(card, p, columns)?;

        owner.extend(p.cells().map(|cell| (cell, i)));
        placed[i] = Some(p);
        cursor = p.row * columns + p.col + p.cols;
    }

    Ok(placed.into_iter().map(|p| p.expect("every card was placed")).collect())
}

/// Checked before touching any cells, so a huge span can't make us loop for ages.
fn check_bounds(card: &CardConfig, p: Placement, columns: usize) -> anyhow::Result<()> {
    ensure!(p.cols > 0 && p.rows > 0, "card `{}`: span must be at least [1, 1]", card.id);
    let (right, bottom) = (p.col + p.cols, p.row + p.rows);
    ensure!(right <= columns, "card `{}` needs {right} columns, the grid has {columns}", card.id);
    ensure!(bottom <= MAX_ROWS, "card `{}` needs {bottom} rows, the grid has at most {MAX_ROWS}", card.id);
    Ok(())
}

/// `n` single cells, filled row by row. The grid every test used before `pos`/`span` existed.
#[cfg(test)]
pub(crate) fn row_major(n: usize, columns: usize) -> Vec<Placement> {
    (0..n).map(|i| Placement { col: i % columns, row: i / columns, cols: 1, rows: 1 }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, pos: Option<[usize; 2]>, span: [usize; 2]) -> CardConfig {
        CardConfig { pos, span, ..CardConfig::new(id, "test") }
    }

    fn auto(id: &str) -> CardConfig {
        card(id, None, [1, 1])
    }

    /// `[col, row, cols, rows]` per card: short to read in asserts.
    fn rects(cards: &[CardConfig], columns: usize) -> Vec<[usize; 4]> {
        place(cards, columns).unwrap().iter().map(|p| [p.col, p.row, p.cols, p.rows]).collect()
    }

    fn err(cards: &[CardConfig], columns: usize) -> String {
        place(cards, columns).unwrap_err().to_string()
    }

    #[test]
    fn auto_fills_row_by_row() {
        let cards: Vec<_> = ["a", "b", "c", "d"].map(auto).into();
        assert_eq!(rects(&cards, 3), [[0, 0, 1, 1], [1, 0, 1, 1], [2, 0, 1, 1], [0, 1, 1, 1]]);
        assert_eq!(place(&cards, 3).unwrap(), row_major(4, 3));
    }

    #[test]
    fn wide_card_wraps_to_the_next_row() {
        // a a .     the gap stays: auto-placement never goes back
        // b b c
        let cards = [card("a", None, [2, 1]), card("b", None, [2, 1]), auto("c")];
        assert_eq!(rects(&cards, 3), [[0, 0, 2, 1], [0, 1, 2, 1], [2, 1, 1, 1]]);
    }

    #[test]
    fn pinned_cards_go_first_and_auto_flows_around_them() {
        // a p p
        // b c d
        let cards = [auto("a"), auto("b"), card("p", Some([1, 0]), [2, 1]), auto("c"), auto("d")];
        assert_eq!(rects(&cards, 3), [[0, 0, 1, 1], [0, 1, 1, 1], [1, 0, 2, 1], [1, 1, 1, 1], [2, 1, 1, 1]]);
    }

    #[test]
    fn tall_card() {
        // t a
        // t b
        let cards = [card("t", None, [1, 2]), auto("a"), auto("b")];
        assert_eq!(rects(&cards, 2), [[0, 0, 1, 2], [1, 0, 1, 1], [1, 1, 1, 1]]);
    }

    #[test]
    fn overlap_is_an_error() {
        let cards = [card("a", Some([0, 0]), [2, 1]), card("b", Some([1, 0]), [1, 1])];
        assert_eq!(err(&cards, 3), "card `b` overlaps card `a`");
    }

    #[test]
    fn out_of_bounds_is_an_error() {
        assert_eq!(err(&[card("a", Some([2, 0]), [2, 1])], 3), "card `a` needs 4 columns, the grid has 3");
        assert_eq!(err(&[card("a", None, [4, 1])], 3), "card `a` needs 4 columns, the grid has 3");
        assert_eq!(err(&[card("a", Some([0, 12]), [1, 1])], 3), "card `a` needs 13 rows, the grid has at most 12");
    }

    #[test]
    fn grid_full_is_an_error() {
        let cards: Vec<_> = (0..13).map(|i| auto(&format!("c{i}"))).collect();
        assert_eq!(err(&cards, 1), "card `c12` needs 13 rows, the grid has at most 12");
    }

    #[test]
    fn bad_span_and_columns() {
        assert_eq!(err(&[card("a", None, [0, 1])], 3), "card `a`: span must be at least [1, 1]");
        assert_eq!(err(&[], 0), "ui.columns must be between 1 and 12");
        assert_eq!(err(&[], 13), "ui.columns must be between 1 and 12");
    }

    #[test]
    fn distance_looks_only_in_one_direction() {
        let at = |col, row| Placement { col, row, cols: 1, rows: 1 };
        let from = at(1, 1);
        assert_eq!(from.distance(at(2, 1), Direction::Right), Some((1, 0)));
        assert_eq!(from.distance(at(2, 3), Direction::Right), Some((1, 2))); // further sideways
        assert_eq!(from.distance(at(0, 1), Direction::Right), None); // wrong way
        assert_eq!(from.distance(at(1, 0), Direction::Up), Some((1, 0)));
        assert_eq!(from.distance(from, Direction::Down), None); // never itself
    }

    #[test]
    fn distance_from_a_wide_card() {
        // w w
        // . b
        let wide = Placement { col: 0, row: 0, cols: 2, rows: 1 };
        let below = Placement { col: 1, row: 1, cols: 1, rows: 1 };
        assert_eq!(wide.distance(below, Direction::Down), Some((1, 0))); // overlapping columns: straight down
        assert_eq!(below.distance(wide, Direction::Up), Some((1, 0)));
    }
}
