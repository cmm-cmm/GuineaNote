//! Action dispatch (keymap v1 ids), availability reasons, clipboard and
//! typing helpers shared by key handling and the F2 action list.
use super::*;

impl App {
    /// Why an action cannot run now, shared by the dispatcher and F2.
    pub fn unavailable(&self, command: &str) -> Option<&'static str> {
        const CHANGES: &[&str] = &[
            "save",
            "save-as",
            "close",
            "reload",
            "undo",
            "redo",
            "cut",
            "paste",
            "replace",
            "find",
            "find-next",
            "duplicate",
            "move-line-up",
            "move-line-down",
            "indent",
            "outdent",
            "delete-word-left",
            "delete-word-right",
        ];
        const REVIEW_BLOCKED: &[&str] = &[
            "save",
            "save-as",
            "reload",
            "find",
            "find-next",
            "replace",
            "undo",
            "redo",
            "cut",
            "paste",
            "duplicate",
            "move-line-up",
            "move-line-down",
            "indent",
            "outdent",
            "delete-word-left",
            "delete-word-right",
        ];
        const BUSY_BLOCKED: &[&str] = &[
            "save",
            "save-as",
            "open",
            "reload",
            "close",
            "quit",
            "find",
            "find-next",
            "replace",
        ];
        const EDITS: &[&str] = &[
            "undo",
            "redo",
            "cut",
            "paste",
            "duplicate",
            "move-line-up",
            "move-line-down",
            "indent",
            "outdent",
            "delete-word-left",
            "delete-word-right",
        ];
        let tab = &self.tabs[self.active];
        if (command == "quit" && !self.agent_leases.is_empty())
            || (tab.ai_action.is_some() && CHANGES.contains(&command))
        {
            return Some("AI is working in this tab; read-only until it finishes");
        }
        if tab.review_source.is_some() && REVIEW_BLOCKED.contains(&command) {
            return Some("Review tab is read-only; return to source to edit or save");
        }
        if self.busy() && BUSY_BLOCKED.contains(&command) {
            return Some("Background operation in progress; retry when complete");
        }
        if tab.document.readonly && EDITS.contains(&command) {
            return Some("Document is read-only");
        }
        None
    }

    pub fn command(&mut self, command: &str) {
        let command = crate::commands::canonical(command);
        if let Some(reason) = self.unavailable(command) {
            self.message = reason.into();
            self.redraw = true;
            return;
        }
        match command {
            "language" | "language-vi" | "language-en" => {
                self.config.language = match command { "language-vi" => crate::i18n::Locale::Vi, "language-en" => crate::i18n::Locale::En, _ => self.config.language.toggle() };
            }
            "files" => self.open_project(crate::project::Mode::Files),
            "project-search" => self.open_project(crate::project::Mode::Contents),
            "goto" => { self.source_tab(); self.tabs[self.active].review.mode = ViewMode::Source; self.prompt(InputKind::GoToLine); }
            "focus-mode" => { self.focus_mode = !self.focus_mode; self.focus = Focus::Editor; }
            "settings" => self.modal = Some(Modal::Settings { selected: self.config.language }),
            "tabs" => self.modal = Some(Modal::Tabs(TabManager::new(&self.tabs, self.active, self.config.language))),
            "info" | "file-info" => self.modal = Some(Modal::FileInfo(FileInfo::new(self.document(), self.config.language))),
            "preview" => self.modal = Some(Modal::ReviewOptions { selected: 0 }),
            "review" | "review-tab" => self.open_review_tab(),
            "source" => { self.source_tab(); self.tabs[self.active].review.mode = ViewMode::Source; },
            "split" => { self.source_tab(); self.tabs[self.active].review.mode = ViewMode::Split; self.focus = Focus::Editor; },
            "new" => self.new_blank(),
            "open-folder" => self.prompt(InputKind::Folder),
            "explorer" => { if self.focus_mode { self.focus_mode = false; self.explorer_visible = true; } else { self.explorer_visible = !self.explorer_visible; } self.focus = if self.explorer_visible { Focus::Explorer } else { Focus::Editor }; }
            "focus" => { self.focus_mode = false; self.explorer_visible = true; self.focus = if self.focus == Focus::Editor { Focus::Explorer } else { Focus::Editor }; }
            "refresh" => self.explorer.refresh(),
            "open" => {
                let root = self.document().path.as_ref().and_then(|p| p.parent()).filter(|p| !p.as_os_str().is_empty()).map(PathBuf::from).unwrap_or_else(|| self.explorer.root.clone());
                self.modal = Some(Modal::FilePicker(Box::new(FilePicker::new(root))));
            }
            "open-path" => self.prompt(InputKind::Open),
            "save-as" => self.prompt(InputKind::SaveAs),
            "save" => {
                if let Some(path) = self.document().path.clone() { self.start_save(path); } else { self.prompt(InputKind::SaveAs); }
            }
            "close" if self.tabs[self.active].review_source.is_none() && self.document().is_dirty() => self.modal = Some(Modal::Confirm { selected: false, action: Action::Close, message: "Discard unsaved changes in this tab? Y discard / N cancel (Ctrl+S saves after cancel)".into() }),
            "close" => self.close(),
            "quit" if self.tabs.iter().any(|t| t.review_source.is_none() && t.document.is_dirty()) => self.modal = Some(Modal::Confirm { selected: false, action: Action::Quit, message: "Discard ALL unsaved tabs and quit? Y discard / N cancel (save tabs after cancel)".into() }),
            "quit" => self.request_quit(),
            "reload" if self.document().path.is_some() => self.modal = Some(Modal::Confirm { selected: false, action: Action::Reload, message: "Reload from disk and discard this tab's edits/history? Y reload / N cancel".into() }),
            "reload" => self.message = "Untitled buffer has no file to reload".into(),
            "next" => { self.active = (self.active + 1) % self.tabs.len(); self.focus = Focus::Editor; }
            "prev" => { self.active = (self.active + self.tabs.len() - 1) % self.tabs.len(); self.focus = Focus::Editor; }
            "help" => {
                self.help_scroll = 0;
                self.modal = Some(Modal::Help);
            }
            "find" => self.open_search(),
            "find-next" => self.find_next(),
            "replace" => self.open_search(),
            "undo" => self.edit(|d| d.undo().map(|_| ())),
            "redo" => self.edit(|d| d.redo().map(|_| ())),
            "select-all" => self.document_mut().select_all(),
            "copy" => self.copy_selection(),
            "cut" if self.document().selection().is_empty() => {
                let mut text = None;
                self.edit(|d| d.cut_line().map(|cut| text = Some(cut)));
                if let Some(text) = text { self.copy_text(text); }
            }
            "cut" => { self.copy_selection(); self.edit(|d| d.insert("")); }
            "paste" => self.paste(),
            "commands" => self.prompt(InputKind::Command),
            "outline" => {
                let review = &mut self.tabs[self.active].review;
                review.outline_visible = !review.outline_visible;
                review.outline_focus = false;
            }
            "duplicate" => self.edit(Document::duplicate),
            "move-line-up" => self.edit(|d| d.move_lines(false)),
            "move-line-down" => self.edit(|d| d.move_lines(true)),
            "indent" | "outdent" => {
                let tab = self.config.tab_width;
                let unit = self.document().indent_unit(tab);
                if command == "indent" && !self.document().selection_spans_lines() {
                    self.edit(|d| d.type_text(&unit));
                } else {
                    self.edit(|d| d.indent_lines(&unit, tab, command == "outdent"));
                }
            }
            "word-left" | "word-right" | "select-word-left" | "select-word-right" => {
                let movement = if command.ends_with("left") { Movement::WordLeft } else { Movement::WordRight };
                self.document_mut().move_cursor(movement, command.starts_with("select"));
                self.tabs[self.active].follow_cursor = true;
            }
            "delete-word-left" => self.edit(Document::delete_word_left),
            "delete-word-right" => self.edit(Document::delete_word_right),
            _ => self.message = self.config.language.render(Msg::UnknownCommand(command)),
        }
        self.redraw = true;
    }

    /// Enter keeps the caret line's indentation. A newline followed by more
    /// input in the same burst is a paste delivered as key events (ConPTY);
    /// its own indentation arrives with it, so auto-indent is cancelled then.
    pub(super) fn newline(&mut self) {
        let indent = self.document().caret_indentation();
        let newline = self.document().line_ending().newline();
        let revision = self.document().revision();
        self.edit(|d| d.type_text(newline));
        if indent.is_empty() || self.document().revision() == revision {
            return;
        }
        let pending = (self.document().id, self.document().revision(), indent);
        if self.input_batch {
            self.pending_indent = Some(pending);
        } else {
            self.pending_indent = Some(pending);
            self.end_input(true);
        }
    }

    /// Called by the event loop before handling a burst of input events.
    pub fn begin_input(&mut self) {
        self.input_batch = true;
    }

    /// Called after a burst. `drained` is true when no further input was queued.
    pub fn end_input(&mut self, drained: bool) {
        self.input_batch = false;
        if !drained {
            return;
        }
        if let Some((id, revision, indent)) = self.pending_indent.take()
            && self.document().id == id
            && self.document().revision() == revision
        {
            self.edit(|d| d.type_text(&indent));
            self.redraw = true;
        }
    }

    /// Copy the selection, or the whole caret line when nothing is selected.
    pub(super) fn copy_selection(&mut self) {
        let text = if self.document().selection().is_empty() {
            self.document().current_line_text()
        } else {
            self.document().selected_text()
        };
        self.copy_text(text);
    }

    pub(super) fn copy_text(&mut self, text: String) {
        use crate::clipboard::Copied;
        self.message = match self.clipboard.copy(text) {
            Copied::System => "Copied to clipboard",
            Copied::Osc52 => "Copied through the terminal (OSC 52)",
            Copied::Internal => "Copied to internal clipboard",
            Copied::TooLargeForOsc52 => {
                "Selection too large for OSC 52; copied to internal clipboard"
            }
        }
        .into();
    }

    pub(super) fn paste(&mut self) {
        match self.clipboard.paste() {
            crate::clipboard::Paste::Ready(text) => self.edit(|d| d.insert(&text)),
            crate::clipboard::Paste::Pending => {
                self.paste_target = Some(self.document().id);
                self.message = "Pasting from system clipboard…".into();
            }
        }
    }

    pub(super) fn poll_clipboard(&mut self) {
        for event in self.clipboard.poll() {
            self.redraw = true;
            let (text, fallback) = match event {
                crate::clipboard::Event::CopyFailed(error) => {
                    self.message = self.config.language.render(Msg::CopyFailed(&error));
                    continue;
                }
                crate::clipboard::Event::Pasted(text) => (text, None),
                crate::clipboard::Event::PasteFallback(text, error) => (text, Some(error)),
            };
            let target = self.paste_target.take();
            if target != Some(self.document().id) || self.modal.is_some() {
                self.message = "Paste cancelled: the active tab changed".into();
            } else if text.is_empty() {
                self.message = "Clipboard is empty".into();
            } else {
                self.edit(|d| d.insert(&text));
                self.tabs[self.active].follow_cursor = true;
                if let Some(error) = fallback {
                    self.message = self.config.language.render(Msg::PasteFallback(&error));
                }
            }
        }
    }
}
