//! Explicit coordinate units. Conversions reject positions inside UTF-8/UTF-16
//! characters. Grapheme and cell functions operate on a supplied line/snapshot.
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

use crate::EditError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteOffset(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CharIndex(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphemeColumn(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellColumn(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utf16Offset(pub usize);

pub fn char_to_byte(text: &str, idx: CharIndex) -> Result<ByteOffset, EditError> {
    text.char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(text.len()))
        .nth(idx.0)
        .map(ByteOffset)
        .ok_or(EditError::OutOfBounds)
}

pub fn byte_to_char(text: &str, idx: ByteOffset) -> Result<CharIndex, EditError> {
    text.get(..idx.0)
        .map(|s| CharIndex(s.chars().count()))
        .ok_or(EditError::OutOfBounds)
}

pub fn char_to_utf16(text: &str, idx: CharIndex) -> Result<Utf16Offset, EditError> {
    let byte = char_to_byte(text, idx)?;
    Ok(Utf16Offset(text[..byte.0].encode_utf16().count()))
}

pub fn utf16_to_char(text: &str, idx: Utf16Offset) -> Result<CharIndex, EditError> {
    let mut units = 0;
    for (i, ch) in text.chars().enumerate() {
        if units == idx.0 {
            return Ok(CharIndex(i));
        }
        units += ch.len_utf16();
    }
    if units == idx.0 {
        Ok(CharIndex(text.chars().count()))
    } else {
        Err(EditError::OutOfBounds)
    }
}

pub fn grapheme_boundaries(text: &str) -> Vec<usize> {
    let mut result = Vec::new();
    let mut idx = 0;
    for g in text.graphemes(true) {
        result.push(idx);
        idx += g.chars().count();
    }
    result.push(idx);
    result
}

pub fn grapheme_column(text: &str, idx: CharIndex) -> Result<GraphemeColumn, EditError> {
    let byte = checked_grapheme_byte(text, idx)?;
    Ok(GraphemeColumn(text[..byte].graphemes(true).count()))
}

fn checked_grapheme_byte(text: &str, idx: CharIndex) -> Result<usize, EditError> {
    let byte = char_to_byte(text, idx)?.0;
    if GraphemeCursor::new(byte, text.len(), true)
        .is_boundary(text, 0)
        .expect("complete text supplies context")
    {
        Ok(byte)
    } else {
        Err(EditError::GraphemeBoundary)
    }
}

/// Safe printable representation; control characters cannot inject terminal
/// escape sequences. Tab width depends on the current absolute cell column.
pub fn display_grapheme(g: &str, cell: usize, tab_width: usize) -> (String, usize) {
    if g == "\t" {
        let width = tab_width.max(1) - cell % tab_width.max(1);
        return (" ".repeat(width), width);
    }
    if g.chars().any(char::is_control) {
        return ("�".into(), 1);
    }
    let width = UnicodeWidthStr::width(g);
    if width == 0 {
        return (format!("◌{g}"), 1);
    }
    (g.into(), width)
}

pub fn cell_column(text: &str, idx: CharIndex, tab_width: usize) -> Result<CellColumn, EditError> {
    let end = checked_grapheme_byte(text, idx)?;
    let mut cell = 0;
    for g in text[..end].graphemes(true) {
        cell += display_grapheme(g, cell, tab_width).1;
    }
    Ok(CellColumn(cell))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_are_distinct_and_checked() {
        let s = "ế👩‍💻\t界";
        assert_eq!(char_to_byte(s, CharIndex(1)).unwrap(), ByteOffset(3));
        assert!(byte_to_char(s, ByteOffset(1)).is_err());
        assert_eq!(char_to_utf16(s, CharIndex(2)).unwrap(), Utf16Offset(3));
        assert!(utf16_to_char(s, Utf16Offset(2)).is_err());
        assert_eq!(grapheme_boundaries(s), vec![0, 1, 4, 5, 6]);
        assert!(grapheme_column(s, CharIndex(2)).is_err());
        assert_eq!(cell_column(s, CharIndex(6), 4).unwrap(), CellColumn(6));
        assert_eq!(display_grapheme("\u{1b}", 0, 4), ("�".into(), 1));
    }
}
