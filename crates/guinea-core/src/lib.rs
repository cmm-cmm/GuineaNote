mod buffer;
pub mod coordinates;
mod document;
mod editing;
pub use editing::{CaseConversion, LineOperation, MARK_LINE_LIMIT};
mod graphemes;
pub mod search;
mod view;
mod view_state;
pub use search::SearchOptions;
pub use view_state::ViewState;

pub use buffer::{ContentSnapshot, EditError, RopeyBuffer, TextBuffer, TextChunk};
pub use document::{
    Document, Fingerprint, HistoryLimits, HistoryStatus, LineEnding, Movement, Selection, TextEdit,
};
pub use view::{CaretWindow, LayoutJob, LayoutReady, LineWindow};
