//! Mouse input: tree/tab/toolbar hits, caret placement and dialog clicks.
use super::*;

impl App {
    pub(super) fn mouse(&mut self, mouse: MouseEvent) {
        let position = (mouse.column, mouse.row).into();
        if mouse.kind == MouseEventKind::Up(MouseButton::Left) {
            self.mouse_selecting = None;
            return;
        }
        if mouse.kind == MouseEventKind::Drag(MouseButton::Left) {
            if self.mouse_selecting == Some(self.document().id)
                && self.hits.editor.contains(position)
            {
                self.place_cursor(mouse.column, mouse.row, true);
                self.redraw = true;
            }
            return;
        }
        if matches!(mouse.kind, MouseEventKind::Down(_)) {
            self.mouse_selecting = None;
            self.last_modal_click = None;
        }
        if self.hits.tab_strip.contains(position) {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Middle) => {
                    if let Some((_, index)) =
                        self.hits.tabs.iter().find(|(r, _)| r.contains(position))
                    {
                        self.request_close_tab(*index);
                        self.redraw = true;
                    }
                    return;
                }
                MouseEventKind::ScrollDown => {
                    self.command("next");
                    return;
                }
                MouseEventKind::ScrollUp => {
                    self.command("prev");
                    return;
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some((_, index)) = self
                        .hits
                        .tab_closes
                        .iter()
                        .find(|(rect, _)| rect.contains(position))
                    {
                        self.request_close_tab(*index);
                        self.redraw = true;
                        return;
                    }
                }
                _ => {}
            }
        }
        if self.hits.outline.contains(position) {
            let rect = self.hits.outline;
            let review = &mut self.tabs[self.active].review;
            if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                let top = review
                    .outline_selected
                    .saturating_sub(rect.height as usize / 2)
                    .min(review.outline().len().saturating_sub(rect.height as usize));
                review.outline_selected = (top + mouse.row.saturating_sub(rect.y) as usize)
                    .min(review.outline().len().saturating_sub(1));
                review.jump_heading();
                self.redraw = true;
            }
            return;
        }
        if self.hits.review.contains(position) {
            let height = self.page_height;
            match mouse.kind {
                MouseEventKind::ScrollDown => self.tabs[self.active].review.scroll(3, height),
                MouseEventKind::ScrollUp => self.tabs[self.active].review.scroll(-3, height),
                _ => {}
            }
            self.redraw = true;
        } else if self.hits.editor.contains(position)
            && matches!(
                mouse.kind,
                MouseEventKind::ScrollUp
                    | MouseEventKind::ScrollDown
                    | MouseEventKind::ScrollLeft
                    | MouseEventKind::ScrollRight
            )
        {
            let tab = &mut self.tabs[self.active];
            tab.follow_cursor = false;
            match mouse.kind {
                MouseEventKind::ScrollUp if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                    tab.left = tab.left.saturating_sub(12)
                }
                MouseEventKind::ScrollDown if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                    tab.left = tab.left.saturating_add(12)
                }
                MouseEventKind::ScrollLeft => tab.left = tab.left.saturating_sub(12),
                MouseEventKind::ScrollRight => tab.left = tab.left.saturating_add(12),
                MouseEventKind::ScrollUp => tab.top = tab.top.saturating_sub(3),
                MouseEventKind::ScrollDown => {
                    tab.top = tab
                        .top
                        .saturating_add(3)
                        .min(tab.document.buffer().len_lines().saturating_sub(1))
                }
                _ => {}
            }
            self.redraw = true;
        } else if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            if let Some((_, index)) = self
                .hits
                .tabs
                .iter()
                .find(|(rect, _)| rect.contains(position))
            {
                self.active = *index;
                self.redraw = true;
            } else if let Some((_, action)) = self
                .hits
                .actions
                .iter()
                .find(|(rect, _)| rect.contains(position))
            {
                self.command(action);
            } else if self.hits.editor.contains(position) {
                self.place_cursor(
                    mouse.column,
                    mouse.row,
                    mouse.modifiers.contains(KeyModifiers::SHIFT),
                );
                self.mouse_selecting = Some(self.document().id);
                self.redraw = true;
            }
        }
    }

    pub(super) fn modal_mouse(&mut self, mouse: MouseEvent) {
        self.redraw = true;
        let point = (mouse.column, mouse.row).into();
        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && self
                .hits
                .modal_actions
                .iter()
                .any(|(r, a)| *a == "cancel" && r.contains(point))
        {
            let search = matches!(&self.modal, Some(Modal::Search(_)));
            self.modal = None;
            self.last_modal_click = None;
            self.message = if search { "Search closed" } else { "Cancelled" }.into();
            return;
        }
        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && matches!(
                &self.modal,
                Some(Modal::Input {
                    kind: InputKind::Command,
                    ..
                })
            )
            && let Some(command) = self
                .hits
                .commands
                .iter()
                .find(|(r, _)| r.contains(point))
                .map(|(_, c)| *c)
        {
            self.modal = None;
            self.command(command);
            return;
        }
        if matches!(&self.modal, Some(Modal::Search(_))) {
            let Some(Modal::Search(mut search)) = self.modal.take() else {
                unreachable!()
            };
            let mut go = false;
            if search.list.contains(point) && search.ready() {
                match mouse.kind {
                    MouseEventKind::ScrollUp => search.navigate(true),
                    MouseEventKind::ScrollDown => search.navigate(false),
                    MouseEventKind::Down(MouseButton::Left) => {
                        let at = search.top() + mouse.row.saturating_sub(search.list.y) as usize;
                        if let Some(found) = search.entries.get(at) {
                            go = self.double_click(ModalClick::Search(
                                self.document().id,
                                found.range.start,
                            ));
                            search.selected = at;
                            search.focus = 2;
                        }
                    }
                    _ => {}
                }
            }
            self.modal = Some(Modal::Search(search));
            self.search_action(if go { "search-go" } else { "search-select" });
            if go {
                return;
            }
        }
        if matches!(&self.modal, Some(Modal::Tabs(_))) {
            let Some(Modal::Tabs(mut manager)) = self.modal.take() else {
                unreachable!()
            };
            match mouse.kind {
                MouseEventKind::ScrollUp if manager.list.contains(point) => manager.move_by(-3),
                MouseEventKind::ScrollDown if manager.list.contains(point) => manager.move_by(3),
                MouseEventKind::Down(MouseButton::Left) if manager.list.contains(point) => {
                    let row = manager.top + (mouse.row - manager.list.y) as usize;
                    if let Some(&entry) = manager.filtered.get(row) {
                        manager.selected = row;
                        let index = manager.entries[entry].index;
                        if self.double_click(ModalClick::Tab(self.tabs[index].id)) {
                            self.active = index;
                            return;
                        }
                    }
                }
                _ => {}
            }
            self.modal = Some(Modal::Tabs(manager));
        }
        if matches!(&self.modal, Some(Modal::FilePicker(_))) {
            let Some(Modal::FilePicker(mut picker)) = self.modal.take() else {
                unreachable!()
            };
            let point = (mouse.column, mouse.row).into();
            let mut action = PickerAction::Keep;
            match mouse.kind {
                MouseEventKind::ScrollUp if picker.hits.list.contains(point) => picker.move_by(-3),
                MouseEventKind::ScrollDown if picker.hits.list.contains(point) => picker.move_by(3),
                MouseEventKind::Down(MouseButton::Left) if picker.hits.list.contains(point) => {
                    let row = picker.top + mouse.row.saturating_sub(picker.hits.list.y) as usize;
                    if let Some(&entry) = picker.filtered.get(row) {
                        picker.selected = row;
                        if self.double_click(ModalClick::File(picker.entries[entry].path.clone())) {
                            action = picker.activate();
                        }
                    }
                }
                MouseEventKind::Down(MouseButton::Left) if picker.hits.parent.contains(point) => {
                    picker.parent()
                }
                MouseEventKind::Down(MouseButton::Left) if picker.hits.open.contains(point) => {
                    action = picker.activate()
                }
                MouseEventKind::Down(MouseButton::Left) if picker.hits.cancel.contains(point) => {
                    action = PickerAction::Cancel
                }
                _ => {}
            }
            self.picker_action(action, picker);
            return;
        }
        if let Some(Modal::FileInfo(info)) = &mut self.modal {
            match mouse.kind {
                MouseEventKind::ScrollDown if self.hits.modal.contains(point) => info.scroll(3),
                MouseEventKind::ScrollUp if self.hits.modal.contains(point) => info.scroll(-3),
                _ => {}
            }
        }
        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && let Some(action) = self
                .hits
                .modal_actions
                .iter()
                .find(|(r, _)| r.contains((mouse.column, mouse.row).into()))
                .map(|(_, a)| *a)
        {
            match action {
                action if action.starts_with("search-") => self.search_action(action),
                "confirm-accept" => {
                    if let Some(Modal::Confirm { action, .. }) = self.modal.take() {
                        self.confirm(action);
                    }
                }
                "input-submit" => {
                    if let Some(Modal::Input { kind, value }) = self.modal.take() {
                        self.submit(kind, value);
                    }
                }
                "tabs-new" => {
                    self.modal = None;
                    self.command("new");
                }
                "tabs-switch" | "tabs-save" | "tabs-close" => {
                    if let Some(Modal::Tabs(manager)) = self.modal.take() {
                        if let Some(index) = manager.current() {
                            if action == "tabs-close" {
                                self.request_close_tab(index);
                            } else {
                                self.active = index;
                                if action == "tabs-save" {
                                    self.command("save");
                                }
                            }
                        } else {
                            self.modal = Some(Modal::Tabs(manager));
                        }
                    }
                }
                "cancel" => {
                    self.modal = None;
                    self.message = "Cancelled".into();
                }
                "settings-vi" => {
                    self.modal = Some(Modal::Settings {
                        selected: Locale::Vi,
                    })
                }
                "settings-en" => {
                    self.modal = Some(Modal::Settings {
                        selected: Locale::En,
                    })
                }
                "settings-apply" => {
                    if let Some(Modal::Settings { selected }) = self.modal.take() {
                        self.apply_settings(selected);
                    }
                }
                "review-split" => {
                    self.modal = None;
                    self.command("split");
                }
                "review-tab" => {
                    self.modal = None;
                    self.command("review-tab");
                }
                _ => {}
            }
        }
    }

    pub(super) fn double_click(&mut self, target: ModalClick) -> bool {
        let now = std::time::Instant::now();
        let double = self
            .last_modal_click
            .take()
            .is_some_and(|(time, previous)| {
                previous == target
                    && now.duration_since(time) < std::time::Duration::from_millis(450)
            });
        if !double {
            self.last_modal_click = Some((now, target));
        }
        double
    }

    pub(super) fn place_cursor(&mut self, column: u16, row: u16, extend: bool) {
        use guinea_core::coordinates::display_grapheme;
        let rect = self.hits.editor_text;
        if rect.height == 0 || row < rect.y || row >= rect.bottom() {
            return;
        }
        if self.caret_layout_pending() {
            // The visible caret line is an approximation until indexed.
            self.message = "Indexing long line… click again when it finishes".into();
            return;
        }
        let tab = &self.tabs[self.active];
        let line = tab.top + (row - rect.y) as usize;
        if tab.document.layout_pending(
            line,
            self.config.tab_width,
            0,
            tab.left.saturating_add(rect.width as usize),
        ) {
            self.message = "Indexing long line… click again when it finishes".into();
            return;
        }
        let Ok(window) =
            tab.document
                .line_window(line, tab.left, rect.width as usize, self.config.tab_width)
        else {
            return;
        };
        let text = window.text.as_str();
        let start = window.char_start;
        let target = tab.left + column.saturating_sub(rect.x) as usize;
        let mut cell = window.cell_start;
        let mut offset = 0;
        for grapheme in text.graphemes(true) {
            let width = display_grapheme(grapheme, cell, self.config.tab_width).1;
            if target < cell + width {
                if target - cell > width / 2 {
                    offset += grapheme.chars().count();
                }
                break;
            }
            cell += width;
            offset += grapheme.chars().count();
        }
        let anchor = if extend {
            self.document().selection().anchor
        } else {
            start + offset
        };
        let _ = self.document_mut().select(anchor, start + offset);
        self.tabs[self.active].follow_cursor = true;
    }
}
