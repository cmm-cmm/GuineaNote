//! Contextual grapheme queries over borrowed UTF-8 chunks, without copying a line.
use crate::{
    EditError, TextBuffer,
    coordinates::{ByteOffset, CharIndex},
};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

enum Query {
    Boundary,
    Next,
    Previous,
}

fn query(buffer: &impl TextBuffer, idx: usize, kind: Query) -> Result<(bool, usize), EditError> {
    let byte = buffer.char_to_byte(CharIndex(idx))?.0;
    let mut cursor = GraphemeCursor::new(byte, buffer.len_bytes(), true);
    let mut chunk = buffer.chunk_at_byte(ByteOffset(byte))?;
    loop {
        let result = match kind {
            Query::Boundary => cursor
                .is_boundary(chunk.text, chunk.byte_start)
                .map(|yes| (yes, byte)),
            Query::Next => cursor
                .next_boundary(chunk.text, chunk.byte_start)
                .map(|next| (true, next.unwrap_or(byte))),
            Query::Previous => cursor
                .prev_boundary(chunk.text, chunk.byte_start)
                .map(|previous| (true, previous.unwrap_or(byte))),
        };
        match result {
            Ok((yes, position)) => return Ok((yes, buffer.byte_to_char(ByteOffset(position))?.0)),
            Err(GraphemeIncomplete::PreContext(end)) => {
                let context = buffer.chunk_at_byte(ByteOffset(end.saturating_sub(1)))?;
                cursor.provide_context(
                    &context.text[..end - context.byte_start],
                    context.byte_start,
                );
            }
            Err(GraphemeIncomplete::NextChunk) => {
                chunk = buffer.chunk_at_byte(ByteOffset(chunk.byte_start + chunk.text.len()))?
            }
            Err(GraphemeIncomplete::PrevChunk) => {
                chunk = buffer.chunk_at_byte(ByteOffset(
                    chunk
                        .byte_start
                        .checked_sub(1)
                        .ok_or(EditError::OutOfBounds)?,
                ))?
            }
            Err(GraphemeIncomplete::InvalidOffset) => return Err(EditError::OutOfBounds),
        }
    }
}

pub fn is_boundary(buffer: &impl TextBuffer, idx: usize) -> bool {
    query(buffer, idx, Query::Boundary).is_ok_and(|(yes, _)| yes)
}
pub fn next(buffer: &impl TextBuffer, idx: usize) -> Result<usize, EditError> {
    query(buffer, idx, Query::Next).map(|(_, position)| position)
}
pub fn previous(buffer: &impl TextBuffer, idx: usize) -> Result<usize, EditError> {
    query(buffer, idx, Query::Previous).map(|(_, position)| position)
}
