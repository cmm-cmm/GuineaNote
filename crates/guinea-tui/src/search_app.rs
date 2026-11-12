use super::*;
use crate::search::{Command, Search};

impl App {
    pub(super) fn open_search(&mut self) {
        self.tabs[self.active].review.mode = ViewMode::Source;
        let range = self.document().selection().range();
        if !range.is_empty() && range.len() <= 256 {
            let value = self.document().selected_text();
            if !value.contains(['\r', '\n']) {
                self.search = value;
            }
        }
        self.modal = Some(Modal::Search(
            Search::new(
                self.document(),
                self.search.clone(),
                self.replacement.clone(),
            )
            .with_options(self.search_options),
        ));
    }
    pub(super) fn search_key(&mut self, mut search: Search, key: KeyEvent) {
        let command = search.key(key);
        self.search = search.query.clone();
        self.replacement = search.replacement.clone();
        self.search_options = search.options;
        self.search_command(search, command);
    }
    fn search_command(&mut self, mut search: Search, command: Command) {
        match command {
            Command::Close => {
                self.message = "Search closed".into();
                return;
            }
            Command::Next => search.navigate(false),
            Command::Previous => search.navigate(true),
            Command::Replace | Command::ReplaceAll => {
                search.replace(self.document(), matches!(command, Command::ReplaceAll))
            }
            _ => {}
        }
        if let Some(range) = search.current() {
            let _ = self.document_mut().select(range.start, range.end);
            self.tabs[self.active].follow_cursor = true;
        }
        if matches!(command, Command::Go) && search.ready() {
            self.focus = Focus::Editor;
            return;
        }
        self.modal = Some(Modal::Search(search));
        self.redraw = true;
    }
    pub(super) fn search_action(&mut self, action: &str) {
        if !matches!(&self.modal, Some(Modal::Search(_))) {
            return;
        }
        let Some(Modal::Search(mut search)) = self.modal.take() else {
            return;
        };
        let command = match action {
            "search-query" => {
                search.focus = 0;
                Command::None
            }
            "search-replacement" => {
                search.focus = 1;
                Command::None
            }
            "search-next" => Command::Next,
            "search-previous" => Command::Previous,
            "search-go" => Command::Go,
            "search-replace" => Command::Replace,
            "search-all" => Command::ReplaceAll,
            "search-case" | "search-word" => {
                search.toggle(action == "search-word");
                self.search_options = search.options;
                Command::None
            }
            _ => Command::None,
        };
        self.search_command(search, command);
    }
    pub(super) fn poll_search(&mut self) {
        if !matches!(&self.modal, Some(Modal::Search(_))) {
            return;
        }
        let Some(Modal::Search(mut search)) = self.modal.take() else {
            return;
        };
        let (changed, result) = search.poll(self.document());
        self.redraw |= changed;
        if let Some(result) = result {
            if self.tabs[self.active].ai_action.is_some() || self.document().readonly {
                search.error = Some("Document is read-only".into());
            } else if result.document.id != self.document().id
                || result.revision != self.document().revision()
            {
                search.error = Some("Document changed; search again before replacing".into());
            } else if result.document.revision() != result.revision {
                match self
                    .document_mut()
                    .accept_prepared(&result.document, result.revision)
                {
                    Ok(()) => {
                        search.applied(self.document(), result.count);
                        self.tabs[self.active].follow_cursor = true;
                    }
                    Err(e) => search.error = Some(e.to_string()),
                }
            } else {
                search.applied(self.document(), 0);
            }
        }
        if changed && let Some(range) = search.current() {
            let _ = self.document_mut().select(range.start, range.end);
            self.tabs[self.active].follow_cursor = true;
        }
        self.modal = Some(Modal::Search(search));
    }
}
