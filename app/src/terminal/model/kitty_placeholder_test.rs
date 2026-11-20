// SPDX-FileCopyrightText: 2026 Tianyu (Sky) Lu
// SPDX-License-Identifier: AGPL-3.0-only

use zap_ng_ui::color::ColorU;

use super::*;
use crate::terminal::model::ansi::NamedColor;

/// The diacritics for 0, 1 and 2.
const D0: char = '\u{0305}';
const D1: char = '\u{030D}';
const D2: char = '\u{030E}';

fn placeholder_cell(foreground: Color, diacritics: &[char]) -> Cell {
    let mut cell = Cell::default();
    cell.c = PLACEHOLDER;
    cell.fg = foreground;
    for &diacritic in diacritics {
        assert!(cell.push_zerowidth(diacritic, false));
    }
    cell
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Spec(ColorU::new(r, g, b, 255))
}

#[test]
fn diacritics_are_numbered_in_code_point_order() {
    assert!(ROW_COLUMN_DIACRITICS
        .windows(2)
        .all(|pair| pair[0] < pair[1]));
    assert_eq!(diacritic_value('\u{0305}'), Some(0));
    assert_eq!(diacritic_value('\u{030D}'), Some(1));
    assert_eq!(diacritic_value('\u{0483}'), Some(30));
    assert_eq!(diacritic_value('\u{1D244}'), Some(296));
    // A combining character below the text is not one of them.
    assert_eq!(diacritic_value('\u{0316}'), None);
}

#[test]
fn cell_names_its_image_by_foreground_colour() {
    let cell = placeholder_cell(rgb(1, 2, 3), &[D1, D2]);
    assert_eq!(
        PlaceholderCell::from_cell(&cell, None),
        Some(PlaceholderCell {
            image_id: 0x010203,
            row: 1,
            column: 2,
        })
    );

    let cell = placeholder_cell(Color::Indexed(42), &[D0, D0]);
    assert_eq!(
        PlaceholderCell::from_cell(&cell, None).map(|cell| cell.image_id),
        Some(42)
    );

    // The third diacritic is the id's high byte.
    let cell = placeholder_cell(rgb(0, 0, 3), &[D0, D0, D2]);
    assert_eq!(
        PlaceholderCell::from_cell(&cell, None).map(|cell| cell.image_id),
        Some(0x02000003)
    );
}

#[test]
fn cell_without_an_image_id_is_no_placeholder() {
    let cell = placeholder_cell(Color::Named(NamedColor::Foreground), &[D0, D0]);
    assert_eq!(PlaceholderCell::from_cell(&cell, None), None);

    let cell = placeholder_cell(rgb(0, 0, 0), &[D0, D0]);
    assert_eq!(PlaceholderCell::from_cell(&cell, None), None);

    let mut cell = placeholder_cell(rgb(0, 0, 7), &[]);
    cell.c = 'x';
    assert_eq!(PlaceholderCell::from_cell(&cell, None), None);

    // 296 is no byte, so it can't be the id's high byte.
    let cell = placeholder_cell(rgb(0, 0, 7), &[D0, D0, '\u{1D244}']);
    assert_eq!(PlaceholderCell::from_cell(&cell, None), None);
}

#[test]
fn cell_takes_what_it_leaves_out_from_the_cell_to_its_left() {
    let left = PlaceholderCell {
        image_id: 0x05000007,
        row: 3,
        column: 4,
    };

    // No diacritics: the next column of the same row.
    let cell = placeholder_cell(rgb(0, 0, 7), &[]);
    assert_eq!(
        PlaceholderCell::from_cell(&cell, Some(left)),
        Some(PlaceholderCell {
            image_id: 0x05000007,
            row: 3,
            column: 5,
        })
    );

    // Only the row, and it is another one: the row starts over.
    let cell = placeholder_cell(rgb(0, 0, 7), &[D1]);
    assert_eq!(
        PlaceholderCell::from_cell(&cell, Some(left)),
        Some(PlaceholderCell {
            image_id: 7,
            row: 1,
            column: 0,
        })
    );

    // Another image to the left: nothing is taken from it.
    let cell = placeholder_cell(rgb(0, 0, 8), &[]);
    assert_eq!(
        PlaceholderCell::from_cell(&cell, Some(left)),
        Some(PlaceholderCell {
            image_id: 8,
            row: 0,
            column: 0,
        })
    );

    // Nothing to the left.
    let cell = placeholder_cell(rgb(0, 0, 7), &[]);
    assert_eq!(
        PlaceholderCell::from_cell(&cell, None),
        Some(PlaceholderCell {
            image_id: 7,
            row: 0,
            column: 0,
        })
    );
}

#[test]
fn virtual_placement_keeps_the_images_proportions() {
    let image_size = Vector2F::new(100., 50.);
    let cell_size = Vector2F::new(10., 20.);
    let size = |cols, rows| VirtualPlacement { cols, rows }.size_in_cells(image_size, cell_size);

    assert_eq!(size(Some(4), Some(7)), (4, 7));
    // 10 columns are 100 pixels wide, so the image is 50 high: 3 rows of 20.
    assert_eq!(size(Some(10), None), (10, 3));
    // 2 rows are 40 pixels high, so the image is 80 wide: 8 columns.
    assert_eq!(size(None, Some(2)), (8, 2));
    assert_eq!(size(None, None), (10, 3));
    assert_eq!(size(Some(100_000), Some(1)), (297, 1));
}

#[test]
fn image_is_fitted_into_its_box_and_centered() {
    // A 2:1 image in a square box is as wide as the box and half as high.
    let rect = image_rect_in_box(Vector2F::new(200., 100.), Vector2F::new(40., 40.));
    assert_eq!(rect.origin(), Vector2F::new(0., 10.));
    assert_eq!(rect.size(), Vector2F::new(40., 20.));
}
