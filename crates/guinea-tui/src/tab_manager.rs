//! A metadata-only snapshot; filtering never reads file contents or the disk.
use crate::{app::Tab, i18n::Locale};
use guinea_core::TextBuffer;
use ratatui::layout::Rect;
use unicode_segmentation::UnicodeSegmentation;

pub struct Entry {
    pub index: usize,
    pub title: String,
    pub path: String,
    pub dirty: bool,
    pub review: bool,
    pub readonly: bool,
    pub ai: bool,
    pub ai_action: Option<String>,
    pub format: &'static str,
    pub bytes: usize,
    pub lines: usize,
    folded: String,
}
pub struct TabManager {
    pub query: String,
    pub entries: Vec<Entry>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub top: usize,
    pub height: usize,
    pub list: Rect,
}
impl TabManager {
    pub fn new(tabs: &[Tab], active: usize, locale: Locale) -> Self {
        let entries = tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let title = tab.title(locale);
                let path = tab
                    .document
                    .path
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                // Hide Windows' internal verbatim prefix in display metadata.
                // The document and all filesystem operations keep their path.
                #[cfg(windows)]
                let path = if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
                    format!(r"\\{unc}")
                } else {
                    path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned()
                };
                Entry {
                    index,
                    folded: format!("{title}\n{path}").to_lowercase(),
                    title,
                    path,
                    dirty: tab.document.is_dirty(),
                    review: tab.review_source.is_some(),
                    readonly: tab.document.readonly,
                    ai: tab.ai_action.is_some(),
                    ai_action: tab.ai_action.clone(),
                    format: crate::app::document_language(&tab.document).label(),
                    bytes: tab.document.buffer().len_bytes(),
                    lines: tab.document.buffer().len_lines(),
                }
            })
            .collect::<Vec<_>>();
        Self {
            filtered: (0..entries.len()).collect(),
            entries,
            selected: active,
            query: String::new(),
            top: 0,
            height: 10,
            list: Rect::default(),
        }
    }
    pub fn current(&self) -> Option<usize> {
        self.filtered
            .get(self.selected)
            .map(|&i| self.entries[i].index)
    }
    pub fn refresh(&mut self, tabs: &[Tab], active: usize, locale: Locale) {
        let selected = self.current();
        let query = std::mem::take(&mut self.query);
        *self = Self::new(tabs, active, locale);
        self.query = query;
        self.filter();
        if let Some(index) = selected.and_then(|index| {
            self.filtered
                .iter()
                .position(|&i| self.entries[i].index == index)
        }) {
            self.selected = index;
        }
    }
    pub fn move_by(&mut self, delta: isize) {
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(self.filtered.len().saturating_sub(1));
    }
    pub fn filter(&mut self) {
        let query = self.query.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.folded.contains(&query))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.top = 0;
    }
    pub fn paste(&mut self, text: &str) {
        self.query.push_str(&text.replace(['\r', '\n'], ""));
        self.filter();
    }
    pub fn backspace(&mut self) {
        if let Some((byte, _)) = self.query.grapheme_indices(true).next_back() {
            self.query.truncate(byte);
            self.filter();
        }
    }
}
