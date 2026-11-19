use std::ops::Range;

use string_offset::CharOffset;
use warpui::text::TextBuffer;

use crate::{find_next_paragraph_end, find_previous_paragraph_start};

/// Vim's "inner paragraph" text object, e.g. `dip`. This includes lines surrounding the cursor
/// until blank lines are encountered in either direction, not including either blank line.
/// See https://vimdoc.sourceforge.net/htmldoc/motion.html#ip
/// or enter ":help ip" in Vim.
///
/// Returns None when the buffer is empty or the offset is out of bounds.
pub fn vim_inner_paragraph<T, C>(buffer: &T, offset: C) -> Option<Range<CharOffset>>
where
    T: TextBuffer + ?Sized,
    C: Into<CharOffset>,
{
    let offset = offset.into();
    let mut chars = buffer.chars_at(offset).ok()?.peekable();
    if chars.peek().is_some_and(|c| *c == '\n') {
        let paragraph_end = offset + chars.take_while(|c| *c == '\n').count() - 1;
        let paragraph_start = blank_lines_start(buffer, offset)?;
        return Some(paragraph_start..paragraph_end);
    }
    let paragraph_start = find_previous_paragraph_start(buffer, offset)
        .map(|i| i + 1)
        .unwrap_or_default();
    let paragraph_end = find_next_paragraph_end(buffer, offset)
        .map(|i| i - 1)
        .unwrap_or_else(|| offset + chars.count());

    Some(paragraph_start..paragraph_end)
}

/// Vim's "a paragraph" text object, e.g. `dap`. This includes lines surrounding the cursor until
/// blank lines are encountered in either direction, including one blank line. Usually the blank
/// line to be included is the one following, unless the paragraph is at the end of the buffer in
/// which case we include the one preceding.
/// See https://vimdoc.sourceforge.net/htmldoc/motion.html#ap
/// or enter ":help ap" in Vim.
///
/// Returns None when the buffer is empty or the offset is out of bounds.
pub fn vim_a_paragraph<T, C>(buffer: &T, offset: C) -> Option<Range<CharOffset>>
where
    T: TextBuffer + ?Sized,
    C: Into<CharOffset>,
{
    let offset = offset.into();
    let mut chars = buffer.chars_at(offset).ok()?.peekable();
    if chars.peek().is_some_and(|c| *c == '\n') {
        let paragraph_end = find_next_paragraph_end(buffer, offset)
            .map(|i| i - 1)
            .unwrap_or_else(|| offset + chars.count());
        let paragraph_start = blank_lines_start(buffer, offset)?;
        return Some(paragraph_start..paragraph_end);
    }
    let end_offset = find_next_paragraph_end(buffer, offset);
    // We either need to include all the consecutive blank lines above or below the paragraph
    // content.
    let paragraph_start = find_previous_paragraph_start(buffer, offset)
        .map(|i| {
            // If end_offset is Some, we'll include the consecutive blank lines below.
            if end_offset.is_some() {
                i + 1
            } else {
                // Calculate the number of blank lines above the start of the paragraph content.
                i + 1
                    - buffer
                        .chars_rev_at(i)
                        .map(|chars| chars.take_while(|c| *c == '\n').count())
                        .unwrap_or_default()
            }
        })
        .unwrap_or_default();
    let paragraph_end = end_offset
        .map(|i| {
            // Calculate the number of blank lines below the end of the paragraph content.
            i - 1
                + buffer
                    .chars_at(i)
                    .map(|chars| chars.take_while(|c| *c == '\n').count())
                    .unwrap_or_default()
        })
        .unwrap_or_else(|| offset + chars.count());

    Some(paragraph_start..paragraph_end)
}

/// Returns where the blank lines around `offset` start, for an `offset` that is at a newline.
fn blank_lines_start<T>(buffer: &T, offset: CharOffset) -> Option<CharOffset>
where
    T: TextBuffer + ?Sized,
{
    let newlines_before = buffer
        .chars_rev_at(offset)
        .ok()?
        .take_while(|c| *c == '\n')
        .count();
    if newlines_before == offset.as_usize() {
        // Only newlines precede `offset`, so the buffer starts with a blank line.
        Some(CharOffset::zero())
    } else {
        // The first newline of the run ends the line above the blank lines.
        Some(offset + 1 - newlines_before)
    }
}

#[cfg(test)]
#[path = "paragraph_tests.rs"]
mod tests;
