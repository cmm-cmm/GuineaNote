use crate::{Selection, view::LineLayout};
use std::{cell::RefCell, collections::VecDeque, sync::Arc};

/// One view's caret and display cache. No text, history, path or terminal types.
/// A detached view can only be restored to the exact document revision.
#[derive(Debug, Clone, Default)]
pub struct ViewState {
    pub(crate) selection: Selection,
    pub(crate) preferred_column: Option<usize>,
    pub(crate) layout_cache: RefCell<Option<Arc<LineLayout<'static>>>>,
    pub(crate) layout_others: RefCell<VecDeque<Arc<LineLayout<'static>>>>,
    pub(crate) binding: Option<(u64, u64)>,
}
impl ViewState {
    pub fn selection(&self) -> Selection {
        self.selection
    }
}
