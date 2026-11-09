use std::ops::Range;

use ropey::Rope;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EditError {
    #[error("text coordinate is out of bounds or not a character boundary")]
    OutOfBounds,
    #[error("document is read-only")]
    ReadOnly,
    #[error("edit would split a grapheme cluster")]
    GraphemeBoundary,
}

/// All indices are Unicode scalar indices, never byte offsets or screen cells.
/// Returned strings are owned. Rope edits/indexing are logarithmic plus copied
/// text; line/slice cost also includes the returned text length.
pub trait TextBuffer {
    fn len_chars(&self) -> usize;
    fn len_lines(&self) -> usize;
    fn line(&self, line_idx: usize) -> Option<String>;
    fn insert(&mut self, char_idx: usize, text: &str) -> Result<(), EditError>;
    fn delete(&mut self, range: Range<usize>) -> Result<String, EditError>;
    fn slice(&self, range: Range<usize>) -> Result<String, EditError>;
    fn char_to_line(&self, char_idx: usize) -> Result<usize, EditError>;
    fn line_to_char(&self, line_idx: usize) -> Result<usize, EditError>;
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
    fn check(&self, range: &Range<usize>) -> Result<(), EditError> {
        if range.start > range.end || range.end > self.len_chars() {
            Err(EditError::OutOfBounds)
        } else {
            Ok(())
        }
    }
}

impl TextBuffer for RopeyBuffer {
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
