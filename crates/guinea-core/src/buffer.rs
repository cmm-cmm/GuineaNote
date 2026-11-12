use std::ops::Range;

use crate::coordinates::{ByteOffset, CharIndex};
use ropey::Rope;
use thiserror::Error;

pub struct TextChunk<'a> {
    pub text: &'a str,
    pub byte_start: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EditError {
    #[error("text coordinate is out of bounds or not a character boundary")]
    OutOfBounds,
    #[error("document is read-only")]
    ReadOnly,
    #[error("edit would split a grapheme cluster")]
    GraphemeBoundary,
    #[error("document revision changed")]
    RevisionConflict,
    /// A replacement would grow the document beyond [`crate::MAX_REPLACED_BYTES`].
    #[error("replacement result exceeds 128 MiB")]
    TooLarge,
}

/// All indices are Unicode scalar indices, never byte offsets or screen cells.
/// Returned strings are owned. Rope edits/indexing are logarithmic plus copied
/// text; line/slice cost also includes the returned text length.
pub trait TextBuffer {
    fn len_bytes(&self) -> usize;
    fn len_chars(&self) -> usize;
    fn len_lines(&self) -> usize;
    fn line(&self, line_idx: usize) -> Option<String>;
    fn insert(&mut self, char_idx: usize, text: &str) -> Result<(), EditError>;
    fn delete(&mut self, range: Range<usize>) -> Result<String, EditError>;
    /// Discard a range without materializing its removed bytes.
    fn remove(&mut self, range: Range<usize>) -> Result<(), EditError> {
        self.delete(range).map(|_| ())
    }
    fn range_equals(&self, range: Range<usize>, expected: &str) -> Result<bool, EditError> {
        let mut offset = 0usize;
        let mut equal = true;
        self.visit_chunks(range, &mut |chunk| {
            let end = offset.saturating_add(chunk.len());
            equal = expected.as_bytes().get(offset..end) == Some(chunk.as_bytes());
            offset = end;
            equal
        })?;
        Ok(equal && offset == expected.len())
    }
    fn slice(&self, range: Range<usize>) -> Result<String, EditError>;
    fn char_to_line(&self, char_idx: usize) -> Result<usize, EditError>;
    fn line_to_char(&self, line_idx: usize) -> Result<usize, EditError>;
    fn char_to_byte(&self, idx: CharIndex) -> Result<ByteOffset, EditError>;
    fn byte_to_char(&self, idx: ByteOffset) -> Result<CharIndex, EditError>;
    fn chunk_at_byte(&self, idx: ByteOffset) -> Result<TextChunk<'_>, EditError>;
    fn visit_chunks(
        &self,
        range: Range<usize>,
        visitor: &mut dyn FnMut(&str) -> bool,
    ) -> Result<(), EditError>;
    fn text(&self) -> String {
        self.slice(0..self.len_chars()).expect("valid full range")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RopeyBuffer(Rope);

impl From<&str> for RopeyBuffer {
    fn from(text: &str) -> Self {
        Self(Rope::from_str(text))
    }
}

impl RopeyBuffer {
    pub(crate) fn from_reader(reader: impl std::io::Read) -> std::io::Result<Self> {
        Rope::from_reader(reader).map(Self)
    }
    pub(crate) fn content_snapshot(&self) -> ContentSnapshot {
        ContentSnapshot(self.clone())
    }

    fn check(&self, range: &Range<usize>) -> Result<(), EditError> {
        if range.start > range.end || range.end > self.len_chars() {
            Err(EditError::OutOfBounds)
        } else {
            Ok(())
        }
    }
}

impl TextBuffer for RopeyBuffer {
    fn len_bytes(&self) -> usize {
        self.0.len_bytes()
    }
    fn len_chars(&self) -> usize {
        self.0.len_chars()
    }
    fn len_lines(&self) -> usize {
        self.0.len_lines()
    }
    fn line(&self, idx: usize) -> Option<String> {
        self.0.get_line(idx).map(|s| s.to_string())
    }
    fn insert(&mut self, idx: usize, text: &str) -> Result<(), EditError> {
        self.check(&(idx..idx))?;
        self.0.insert(idx, text);
        Ok(())
    }
    fn delete(&mut self, range: Range<usize>) -> Result<String, EditError> {
        let old = self.slice(range.clone())?;
        self.0.remove(range);
        Ok(old)
    }
    fn remove(&mut self, range: Range<usize>) -> Result<(), EditError> {
        self.check(&range)?;
        self.0.remove(range);
        Ok(())
    }
    fn slice(&self, range: Range<usize>) -> Result<String, EditError> {
        self.check(&range)?;
        Ok(self.0.slice(range).to_string())
    }
    fn char_to_line(&self, idx: usize) -> Result<usize, EditError> {
        self.check(&(idx..idx))?;
        Ok(self.0.char_to_line(idx))
    }
    fn line_to_char(&self, idx: usize) -> Result<usize, EditError> {
        if idx >= self.len_lines() {
            return Err(EditError::OutOfBounds);
        }
        Ok(self.0.line_to_char(idx))
    }
    fn char_to_byte(&self, idx: CharIndex) -> Result<ByteOffset, EditError> {
        self.check(&(idx.0..idx.0))?;
        Ok(ByteOffset(self.0.char_to_byte(idx.0)))
    }
    fn byte_to_char(&self, idx: ByteOffset) -> Result<CharIndex, EditError> {
        let chunk = self.chunk_at_byte(idx)?;
        if !chunk.text.is_char_boundary(idx.0 - chunk.byte_start) {
            return Err(EditError::OutOfBounds);
        }
        Ok(CharIndex(self.0.byte_to_char(idx.0)))
    }
    fn chunk_at_byte(&self, idx: ByteOffset) -> Result<TextChunk<'_>, EditError> {
        let (text, byte_start, _, _) = self
            .0
            .get_chunk_at_byte(idx.0)
            .ok_or(EditError::OutOfBounds)?;
        Ok(TextChunk { text, byte_start })
    }
    fn visit_chunks(
        &self,
        range: Range<usize>,
        visitor: &mut dyn FnMut(&str) -> bool,
    ) -> Result<(), EditError> {
        self.check(&range)?;
        for chunk in self.0.slice(range).chunks() {
            if !visitor(chunk) {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn checked_ranges_never_panic() {
        let mut b = RopeyBuffer::from("Việt 🐹\r\n");
        assert_eq!(b.len_lines(), 2);
        assert!(b.insert(100, "x").is_err());
        assert!(b.delete(8..100).is_err());
        assert!(b.slice(usize::MAX..usize::MAX).is_err());
        assert!(b.line(10).is_none());
        assert_eq!(b.line(0).unwrap(), "Việt 🐹\r\n");
    }

    proptest! {
        #[test]
        fn edit_matches_string(text in ".{0,100}", insert in ".{0,30}", pos in any::<usize>()) {
            let mut b = RopeyBuffer::from(text.as_str());
            let idx = pos % (text.chars().count() + 1);
            let byte = text.char_indices().nth(idx).map_or(text.len(), |(i,_)| i);
            let mut expected = text.clone();
            expected.insert_str(byte, &insert);
            b.insert(idx, &insert).unwrap();
            prop_assert_eq!(b.text(), expected);
            prop_assert_eq!(b.delete(idx..idx + insert.chars().count()).unwrap(), insert);
            prop_assert_eq!(b.text(), text);
        }
    }
}

/// Immutable, backend-opaque shared content. No caret, layout or Undo history.
#[derive(Debug, Clone)]
pub struct ContentSnapshot(RopeyBuffer);
impl ContentSnapshot {
    pub fn len_bytes(&self) -> usize {
        self.0.len_bytes()
    }
    pub fn len_chars(&self) -> usize {
        self.0.len_chars()
    }
    pub fn len_lines(&self) -> usize {
        self.0.len_lines()
    }
    pub fn text(&self) -> String {
        self.0.text()
    }
    pub fn visit_chunks(&self, visitor: &mut dyn FnMut(&str) -> bool) {
        self.0
            .visit_chunks(0..self.0.len_chars(), visitor)
            .expect("full snapshot range");
    }
}
