//! Unicode placeholders of the Kitty graphics protocol.
//!
//! A program that can't address the terminal's screen directly (one running inside tmux, say)
//! shows an image by creating a virtual placement (`U=1`) for it and printing placeholder
//! characters where the image should be. A placeholder cell names the image by its foreground
//! colour and the part of the image it shows by combining diacritics, so the image moves, scrolls
//! and is erased with the text.

use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::Vector2F;

use super::ansi::Color;
use super::cell::Cell;
use super::char_or_str::CharOrStr;
use super::kitty::VirtualPlacement;

/// The character a placeholder cell holds.
pub const PLACEHOLDER: char = '\u{10EEEE}';

/// The diacritics that encode a number in a placeholder cell: the number is the diacritic's index.
/// They are the combining characters of class 230 without a decomposition in Unicode 6.0.0, in
/// code point order, as the protocol defines them.
const ROW_COLUMN_DIACRITICS: [char; 297] = [
    '\u{0305}',
    '\u{030D}',
    '\u{030E}',
    '\u{0310}',
    '\u{0312}',
    '\u{033D}',
    '\u{033E}',
    '\u{033F}',
    '\u{0346}',
    '\u{034A}',
    '\u{034B}',
    '\u{034C}',
    '\u{0350}',
    '\u{0351}',
    '\u{0352}',
    '\u{0357}',
    '\u{035B}',
    '\u{0363}',
    '\u{0364}',
    '\u{0365}',
    '\u{0366}',
    '\u{0367}',
    '\u{0368}',
    '\u{0369}',
    '\u{036A}',
    '\u{036B}',
    '\u{036C}',
    '\u{036D}',
    '\u{036E}',
    '\u{036F}',
    '\u{0483}',
    '\u{0484}',
    '\u{0485}',
    '\u{0486}',
    '\u{0487}',
    '\u{0592}',
    '\u{0593}',
    '\u{0594}',
    '\u{0595}',
    '\u{0597}',
    '\u{0598}',
    '\u{0599}',
    '\u{059C}',
    '\u{059D}',
    '\u{059E}',
    '\u{059F}',
    '\u{05A0}',
    '\u{05A1}',
    '\u{05A8}',
    '\u{05A9}',
    '\u{05AB}',
    '\u{05AC}',
    '\u{05AF}',
    '\u{05C4}',
    '\u{0610}',
    '\u{0611}',
    '\u{0612}',
    '\u{0613}',
    '\u{0614}',
    '\u{0615}',
    '\u{0616}',
    '\u{0617}',
    '\u{0657}',
    '\u{0658}',
    '\u{0659}',
    '\u{065A}',
    '\u{065B}',
    '\u{065D}',
    '\u{065E}',
    '\u{06D6}',
    '\u{06D7}',
    '\u{06D8}',
    '\u{06D9}',
    '\u{06DA}',
    '\u{06DB}',
    '\u{06DC}',
    '\u{06DF}',
    '\u{06E0}',
    '\u{06E1}',
    '\u{06E2}',
    '\u{06E4}',
    '\u{06E7}',
    '\u{06E8}',
    '\u{06EB}',
    '\u{06EC}',
    '\u{0730}',
    '\u{0732}',
    '\u{0733}',
    '\u{0735}',
    '\u{0736}',
    '\u{073A}',
    '\u{073D}',
    '\u{073F}',
    '\u{0740}',
    '\u{0741}',
    '\u{0743}',
    '\u{0745}',
    '\u{0747}',
    '\u{0749}',
    '\u{074A}',
    '\u{07EB}',
    '\u{07EC}',
    '\u{07ED}',
    '\u{07EE}',
    '\u{07EF}',
    '\u{07F0}',
    '\u{07F1}',
    '\u{07F3}',
    '\u{0816}',
    '\u{0817}',
    '\u{0818}',
    '\u{0819}',
    '\u{081B}',
    '\u{081C}',
    '\u{081D}',
    '\u{081E}',
    '\u{081F}',
    '\u{0820}',
    '\u{0821}',
    '\u{0822}',
    '\u{0823}',
    '\u{0825}',
    '\u{0826}',
    '\u{0827}',
    '\u{0829}',
    '\u{082A}',
    '\u{082B}',
    '\u{082C}',
    '\u{082D}',
    '\u{0951}',
    '\u{0953}',
    '\u{0954}',
    '\u{0F82}',
    '\u{0F83}',
    '\u{0F86}',
    '\u{0F87}',
    '\u{135D}',
    '\u{135E}',
    '\u{135F}',
    '\u{17DD}',
    '\u{193A}',
    '\u{1A17}',
    '\u{1A75}',
    '\u{1A76}',
    '\u{1A77}',
    '\u{1A78}',
    '\u{1A79}',
    '\u{1A7A}',
    '\u{1A7B}',
    '\u{1A7C}',
    '\u{1B6B}',
    '\u{1B6D}',
    '\u{1B6E}',
    '\u{1B6F}',
    '\u{1B70}',
    '\u{1B71}',
    '\u{1B72}',
    '\u{1B73}',
    '\u{1CD0}',
    '\u{1CD1}',
    '\u{1CD2}',
    '\u{1CDA}',
    '\u{1CDB}',
    '\u{1CE0}',
    '\u{1DC0}',
    '\u{1DC1}',
    '\u{1DC3}',
    '\u{1DC4}',
    '\u{1DC5}',
    '\u{1DC6}',
    '\u{1DC7}',
    '\u{1DC8}',
    '\u{1DC9}',
    '\u{1DCB}',
    '\u{1DCC}',
    '\u{1DD1}',
    '\u{1DD2}',
    '\u{1DD3}',
    '\u{1DD4}',
    '\u{1DD5}',
    '\u{1DD6}',
    '\u{1DD7}',
    '\u{1DD8}',
    '\u{1DD9}',
    '\u{1DDA}',
    '\u{1DDB}',
    '\u{1DDC}',
    '\u{1DDD}',
    '\u{1DDE}',
    '\u{1DDF}',
    '\u{1DE0}',
    '\u{1DE1}',
    '\u{1DE2}',
    '\u{1DE3}',
    '\u{1DE4}',
    '\u{1DE5}',
    '\u{1DE6}',
    '\u{1DFE}',
    '\u{20D0}',
    '\u{20D1}',
    '\u{20D4}',
    '\u{20D5}',
    '\u{20D6}',
    '\u{20D7}',
    '\u{20DB}',
    '\u{20DC}',
    '\u{20E1}',
    '\u{20E7}',
    '\u{20E9}',
    '\u{20F0}',
    '\u{2CEF}',
    '\u{2CF0}',
    '\u{2CF1}',
    '\u{2DE0}',
    '\u{2DE1}',
    '\u{2DE2}',
    '\u{2DE3}',
    '\u{2DE4}',
    '\u{2DE5}',
    '\u{2DE6}',
    '\u{2DE7}',
    '\u{2DE8}',
    '\u{2DE9}',
    '\u{2DEA}',
    '\u{2DEB}',
    '\u{2DEC}',
    '\u{2DED}',
    '\u{2DEE}',
    '\u{2DEF}',
    '\u{2DF0}',
    '\u{2DF1}',
    '\u{2DF2}',
    '\u{2DF3}',
    '\u{2DF4}',
    '\u{2DF5}',
    '\u{2DF6}',
    '\u{2DF7}',
    '\u{2DF8}',
    '\u{2DF9}',
    '\u{2DFA}',
    '\u{2DFB}',
    '\u{2DFC}',
    '\u{2DFD}',
    '\u{2DFE}',
    '\u{2DFF}',
    '\u{A66F}',
    '\u{A67C}',
    '\u{A67D}',
    '\u{A6F0}',
    '\u{A6F1}',
    '\u{A8E0}',
    '\u{A8E1}',
    '\u{A8E2}',
    '\u{A8E3}',
    '\u{A8E4}',
    '\u{A8E5}',
    '\u{A8E6}',
    '\u{A8E7}',
    '\u{A8E8}',
    '\u{A8E9}',
    '\u{A8EA}',
    '\u{A8EB}',
    '\u{A8EC}',
    '\u{A8ED}',
    '\u{A8EE}',
    '\u{A8EF}',
    '\u{A8F0}',
    '\u{A8F1}',
    '\u{AAB0}',
    '\u{AAB2}',
    '\u{AAB3}',
    '\u{AAB7}',
    '\u{AAB8}',
    '\u{AABE}',
    '\u{AABF}',
    '\u{AAC1}',
    '\u{FE20}',
    '\u{FE21}',
    '\u{FE22}',
    '\u{FE23}',
    '\u{FE24}',
    '\u{FE25}',
    '\u{FE26}',
    '\u{10A0F}',
    '\u{10A38}',
    '\u{1D185}',
    '\u{1D186}',
    '\u{1D187}',
    '\u{1D188}',
    '\u{1D189}',
    '\u{1D1AA}',
    '\u{1D1AB}',
    '\u{1D1AC}',
    '\u{1D1AD}',
    '\u{1D242}',
    '\u{1D243}',
    '\u{1D244}',
];

/// The highest number of rows or columns a virtual placement spans: placeholder cells can't name
/// more than there are diacritics.
const MAX_CELLS: u32 = ROW_COLUMN_DIACRITICS.len() as u32;

fn diacritic_value(diacritic: char) -> Option<u32> {
    ROW_COLUMN_DIACRITICS
        .binary_search(&diacritic)
        .ok()
        .map(|index| index as u32)
}

/// The low three bytes of an image id, which a placeholder cell carries in its foreground colour.
fn image_id_low_bytes(foreground: &Color) -> Option<u32> {
    match foreground {
        Color::Spec(color) => {
            Some(u32::from(color.r) << 16 | u32::from(color.g) << 8 | u32::from(color.b))
        }
        Color::Indexed(index) => Some(u32::from(*index)),
        Color::Named(_) => None,
    }
}

/// What a placeholder cell shows: the cell of an image's virtual placement at `row` and `column`,
/// both counted from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceholderCell {
    pub image_id: u32,
    pub row: u32,
    pub column: u32,
}

impl PlaceholderCell {
    /// Reads a placeholder cell. `left` is the placeholder cell directly to its left, if there is
    /// one: a cell may leave out its diacritics from the end, and then continues that cell.
    ///
    /// Returns `None` for a cell that is no placeholder or names no image.
    pub fn from_cell(cell: &Cell, left: Option<PlaceholderCell>) -> Option<Self> {
        if cell.c != PLACEHOLDER {
            return None;
        }

        match cell.raw_content() {
            CharOrStr::Char(_) => Self::resolve(&cell.fg, std::iter::empty(), left),
            CharOrStr::Str(content) => Self::resolve(&cell.fg, content.chars().skip(1), left),
        }
    }

    fn resolve(
        foreground: &Color,
        diacritics: impl Iterator<Item = char>,
        left: Option<PlaceholderCell>,
    ) -> Option<Self> {
        let id_low_bytes = image_id_low_bytes(foreground)?;

        // The diacritics give, in this order, the row, the column and the high byte of the id.
        let mut numbers = diacritics.map_while(diacritic_value);
        let row = numbers.next();
        let column = numbers.next();
        let id_high_byte = match numbers.next() {
            Some(id_high_byte) => Some(u8::try_from(id_high_byte).ok()?),
            None => None,
        };

        // What a cell leaves out it takes from the cell to its left, as long as the two belong
        // together: the same image and, going from left to right, the same row and the next
        // column.
        let left = left.filter(|left| left.image_id & 0x00FF_FFFF == id_low_bytes);
        let row = row.or(left.map(|left| left.row)).unwrap_or(0);
        let left = left.filter(|left| left.row == row);
        let column = column.or(left.map(|left| left.column + 1)).unwrap_or(0);
        let left = left.filter(|left| left.column + 1 == column);
        let id_high_byte = id_high_byte
            .or(left.map(|left| (left.image_id >> 24) as u8))
            .unwrap_or(0);

        let image_id = u32::from(id_high_byte) << 24 | id_low_bytes;
        (image_id != 0).then_some(PlaceholderCell {
            image_id,
            row,
            column,
        })
    }
}

impl VirtualPlacement {
    /// The number of columns and rows the placement spans. A size the placement leaves out is the
    /// one that keeps the image's proportions, or the image's own size if it gives neither.
    pub fn size_in_cells(&self, image_size: Vector2F, cell_size: Vector2F) -> (u32, u32) {
        let cells = |pixels: f32, cell_pixels: f32| (pixels / cell_pixels).ceil() as u32;
        let aspect_ratio = image_size.x() / image_size.y();

        let (columns, rows) = match (
            self.cols.filter(|&cols| cols != 0),
            self.rows.filter(|&rows| rows != 0),
        ) {
            (Some(columns), Some(rows)) => (columns, rows),
            (Some(columns), None) => {
                let height = columns as f32 * cell_size.x() / aspect_ratio;
                (columns, cells(height, cell_size.y()))
            }
            (None, Some(rows)) => {
                let width = rows as f32 * cell_size.y() * aspect_ratio;
                (cells(width, cell_size.x()), rows)
            }
            (None, None) => (
                cells(image_size.x(), cell_size.x()),
                cells(image_size.y(), cell_size.y()),
            ),
        };

        (columns.clamp(1, MAX_CELLS), rows.clamp(1, MAX_CELLS))
    }
}

/// Where an image goes in the box of cells its virtual placement spans, relative to the box's
/// top left corner: scaled to fit the box with its proportions kept, and centered.
pub fn image_rect_in_box(image_size: Vector2F, box_size: Vector2F) -> RectF {
    let scale = (box_size.x() / image_size.x()).min(box_size.y() / image_size.y());
    let size = image_size * scale;
    RectF::new((box_size - size) * 0.5, size)
}

#[cfg(test)]
#[path = "kitty_placeholder_test.rs"]
mod tests;
