use warpui::color::ColorU;

use super::*;
use crate::terminal::model::ansi::Color;
use crate::terminal::model::kitty_placeholder::PLACEHOLDER;

/// The diacritics for 0 and 1.
const D0: char = '\u{0305}';
const D1: char = '\u{030D}';

fn placeholder_cell(image_id: u8, diacritics: &[char]) -> Cell {
    let mut cell = Cell::default();
    cell.c = PLACEHOLDER;
    cell.fg = Color::Spec(ColorU::new(0, 0, image_id, 255));
    for &diacritic in diacritics {
        assert!(cell.push_zerowidth(diacritic, false));
    }
    cell
}

fn run(
    image_id: u32,
    row: u32,
    column: u32,
    grid_row: usize,
    grid_column: usize,
    len: usize,
) -> PlaceholderRun {
    PlaceholderRun {
        first: PlaceholderCell {
            image_id,
            row,
            column,
        },
        grid_row,
        grid_column,
        len,
    }
}

#[test]
fn neighbouring_cells_of_an_image_row_form_one_run() {
    let mut runs = PlaceholderRuns::default();
    // Two rows of three cells each, the way a client prints them: the first cell of a row says
    // where it is, the others follow it.
    for (grid_row, row_diacritic) in [(5, D0), (6, D1)] {
        runs.push(&placeholder_cell(7, &[row_diacritic, D0]), grid_row, 2);
        runs.push(&placeholder_cell(7, &[]), grid_row, 3);
        runs.push(&placeholder_cell(7, &[]), grid_row, 4);
    }

    assert_eq!(
        runs.runs,
        vec![run(7, 0, 0, 5, 2, 3), run(7, 1, 0, 6, 2, 3)]
    );
}

#[test]
fn run_ends_where_the_image_or_its_columns_change() {
    let mut runs = PlaceholderRuns::default();
    runs.push(&placeholder_cell(7, &[D0, D0]), 0, 0);
    // Another image.
    runs.push(&placeholder_cell(8, &[D0, D1]), 0, 1);
    // The same image, but not its next column.
    runs.push(&placeholder_cell(8, &[D0, D0]), 0, 2);
    // Not the next cell of the grid, so it doesn't continue the cell before it.
    runs.push(&placeholder_cell(8, &[]), 0, 4);

    assert_eq!(
        runs.runs,
        vec![
            run(7, 0, 0, 0, 0, 1),
            run(8, 0, 1, 0, 1, 1),
            run(8, 0, 0, 0, 2, 1),
            run(8, 0, 0, 0, 4, 1),
        ]
    );
}
