mod buffer;
pub mod coordinates;
mod document;

pub use buffer::{EditError, RopeyBuffer, TextBuffer};
pub use document::{Document, Fingerprint, LineEnding, Movement, Selection};
