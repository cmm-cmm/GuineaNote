//! Keyboard input inside dialogs and prompt submission.
use super::*;

impl App {
    pub(super) fn modal_key(&mut self, key: KeyEvent) {
        let modal = self.modal.take().expect("modal exists");
        match modal {
            Modal::Search(search) => self.search_key(search, key),
            Modal::Tabs(mut manager) => {
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                match key.code {
                    KeyCode::Esc => return,
                    KeyCode::Char('p') if ctrl => return,
                    KeyCode::F(12) => return,
                    KeyCode::Char('n' | 't') if ctrl => {
                        self.command("new");
                        return;
                    }
                    KeyCode::Char('s') if ctrl => {
                        if let Some(index) = manager.current() {
                            self.active = index;
                            self.command("save");
                            return;
                        }
                    }
                    KeyCode::Char('w') if ctrl => {
                        if let Some(index) = manager.current() {
                            self.request_close_tab(index);
                            return;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(index) = manager.current() {
                            self.active = index;
                            return;
                        }
                    }
                    KeyCode::Up => manager.move_by(-1),
                    KeyCode::Down => manager.move_by(1),
                    KeyCode::Tab | KeyCode::BackTab => manager.move_by(
                        if key.code == KeyCode::BackTab
                            || key.modifiers.contains(KeyModifiers::SHIFT)
                        {
                            -1
                        } else {
                            1
                        },
                    ),
                    _ if crate::keyboard::tab_command(key) == Some("next") => manager.move_by(1),
                    _ if crate::keyboard::tab_command(key) == Some("prev") => manager.move_by(-1),
                    KeyCode::PageUp => manager.move_by(-(manager.height as isize)),
                    KeyCode::PageDown => manager.move_by(manager.height as isize),
                    KeyCode::Home => manager.selected = 0,
                    KeyCode::End => manager.selected = manager.filtered.len().saturating_sub(1),
                    KeyCode::Backspace => manager.backspace(),
                    KeyCode::Char('u') if ctrl => {
                        manager.query.clear();
                        manager.filter();
                    }
                    KeyCode::Char(ch)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        manager.paste(&ch.to_string())
                    }
                    _ => {}
                }
                self.modal = Some(Modal::Tabs(manager));
            }
            Modal::Settings { mut selected } => match key.code {
                KeyCode::Esc | KeyCode::F(10) => self.message = "Cancelled".into(),
                KeyCode::Left
                | KeyCode::Up
                | KeyCode::Right
                | KeyCode::Down
                | KeyCode::Tab
                | KeyCode::BackTab => {
                    selected = selected.toggle();
                    self.modal = Some(Modal::Settings { selected });
                }
                KeyCode::Char('v' | 'V') => {
                    self.modal = Some(Modal::Settings {
                        selected: Locale::Vi,
                    })
                }
                KeyCode::Char('e' | 'E') => {
                    self.modal = Some(Modal::Settings {
                        selected: Locale::En,
                    })
                }
                KeyCode::Enter | KeyCode::Char(' ') => self.apply_settings(selected),
                _ => self.modal = Some(Modal::Settings { selected }),
            },
            Modal::ReviewOptions { mut selected } => match key.code {
                KeyCode::Esc | KeyCode::F(8) => self.message = "Cancelled".into(),
                KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Tab
                | KeyCode::BackTab => {
                    selected = 1 - selected;
                    self.modal = Some(Modal::ReviewOptions { selected });
                }
                KeyCode::Char('1') => self.command("split"),
                KeyCode::Char('2') => self.command("review-tab"),
                KeyCode::Enter => self.command(if selected == 0 { "split" } else { "review-tab" }),
                _ => self.modal = Some(Modal::ReviewOptions { selected }),
            },
            Modal::FilePicker(mut picker) => {
                let action = picker.key(key);
                self.picker_action(action, picker);
            }
            Modal::FileInfo(mut info) => {
                match key.code {
                    KeyCode::Esc | KeyCode::F(9) | KeyCode::Enter => return,
                    KeyCode::Up => info.scroll(-1),
                    KeyCode::Down => info.scroll(1),
                    KeyCode::PageUp => info.scroll(-(info.height as isize)),
                    KeyCode::PageDown => info.scroll(info.height as isize),
                    KeyCode::Home => info.top = 0,
                    KeyCode::End => info.scroll(isize::MAX),
                    _ => {}
                }
                self.modal = Some(Modal::FileInfo(info));
            }
            Modal::Help => {
                match key.code {
                    KeyCode::Esc | KeyCode::F(1) | KeyCode::Enter => return,
                    KeyCode::Up => self.help_scroll = self.help_scroll.saturating_sub(1),
                    KeyCode::Down => self.help_scroll = self.help_scroll.saturating_add(1),
                    KeyCode::PageUp => {
                        self.help_scroll = self.help_scroll.saturating_sub(self.help_height)
                    }
                    KeyCode::PageDown => {
                        self.help_scroll = self.help_scroll.saturating_add(self.help_height)
                    }
                    KeyCode::Home => self.help_scroll = 0,
                    KeyCode::End => self.help_scroll = self.help_rows,
                    _ => {}
                }
                self.help_scroll = self
                    .help_scroll
                    .min(self.help_rows.saturating_sub(self.help_height));
                self.modal = Some(Modal::Help);
            }
            Modal::Confirm {
                action,
                message,
                mut selected,
            } => match key.code {
                KeyCode::Char('y' | 'Y') => self.confirm(action),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if selected {
                        self.confirm(action);
                    } else {
                        self.message = "Cancelled".into();
                    }
                }
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    selected = !selected;
                    self.modal = Some(Modal::Confirm {
                        action,
                        message,
                        selected,
                    });
                }
                KeyCode::Esc | KeyCode::Char('n' | 'N') => {
                    self.message = "Cancelled".into();
                }
                _ => {
                    self.modal = Some(Modal::Confirm {
                        action,
                        message,
                        selected,
                    });
                }
            },
            Modal::Input { kind, mut value } => {
                if matches!(kind, InputKind::Command) {
                    let commands = crate::commands::suggestions(&value, self.config.language);
                    match key.code {
                        KeyCode::Up => self.palette_index = self.palette_index.saturating_sub(1),
                        KeyCode::Down => {
                            self.palette_index =
                                (self.palette_index + 1).min(commands.len().saturating_sub(1))
                        }
                        KeyCode::Tab => {
                            if let Some(command) = commands.get(self.palette_index) {
                                value = command.id.into();
                            }
                            self.palette_index = 0;
                        }
                        KeyCode::Enter if !commands.iter().any(|c| c.id == value.trim()) => {
                            if let Some(command) = commands.get(self.palette_index) {
                                value = command.id.into();
                            }
                        }
                        KeyCode::Char(_) | KeyCode::Backspace => self.palette_index = 0,
                        _ => {}
                    }
                }
                match key.code {
                    KeyCode::Esc => {
                        self.message = "Cancelled".into();
                        return;
                    }
                    KeyCode::Enter => {
                        self.submit(kind, value);
                        return;
                    }
                    KeyCode::Backspace => {
                        if let Some((byte, _)) = value.grapheme_indices(true).next_back() {
                            value.truncate(byte);
                        }
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        value.clear()
                    }
                    KeyCode::Char(ch)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        if value.len() + ch.len_utf8() <= 4096 {
                            value.push(ch);
                        } else {
                            self.message = "Input is limited to 4096 UTF-8 bytes".into();
                        }
                    }
                    _ => {}
                }
                self.modal = Some(Modal::Input { kind, value });
            }
        }
    }

    pub(super) fn submit(&mut self, kind: InputKind, value: String) {
        match kind {
            // A folder path opens the file picker there to choose a file.
            InputKind::Open if !value.is_empty() && PathBuf::from(&value).is_dir() => {
                self.modal = Some(Modal::FilePicker(Box::new(FilePicker::new(PathBuf::from(
                    value,
                )))));
            }
            InputKind::Open if !value.is_empty() => self.start_open(
                PathBuf::from(value),
                None,
                self.config.large_file_warning_bytes,
            ),
            InputKind::SaveAs if !value.is_empty() => self.start_save(PathBuf::from(value)),
            InputKind::Command => self.command(value.trim()),
            InputKind::GoToLine => {
                let coordinates = value
                    .split(':')
                    .map(str::trim)
                    .map(str::parse::<usize>)
                    .collect::<Result<Vec<_>, _>>();
                let Ok(coordinates) = coordinates else {
                    self.message = "Invalid line or column".into();
                    return;
                };
                if coordinates.is_empty() || coordinates.len() > 2 || coordinates.contains(&0) {
                    self.message = "Invalid line or column".into();
                    return;
                }
                let line = coordinates[0] - 1;
                let column = coordinates.get(1).copied().unwrap_or(1) - 1;
                match grapheme_column_to_char(self.document(), line, column) {
                    Some(at) if self.document_mut().select(at, at).is_ok() => {
                        self.tabs[self.active].follow_cursor = true;
                        self.message = "Moved to line".into();
                    }
                    _ => self.message = "Invalid line or column".into(),
                }
            }
            _ => {
                self.message = "Empty input; cancelled".into();
            }
        }
    }
}

/// Scalar index of 0-based grapheme `column` on `line`, reading only a bounded
/// prefix of the line (doubled until the column is reached), so going to a
/// multi-MiB line never copies or segments the whole line. Grapheme starts
/// before the prefix's last cluster do not depend on text beyond the prefix;
/// the prefix end itself only counts at the true line end.
pub(super) fn grapheme_column_to_char(
    document: &Document,
    line: usize,
    column: usize,
) -> Option<usize> {
    use guinea_core::TextBuffer;
    let bounds = document.line_bounds(line).ok()?;
    let mut length = column.saturating_mul(4).saturating_add(64);
    loop {
        let end = bounds.start.saturating_add(length).min(bounds.end);
        let text = document.buffer().slice(bounds.start..end).ok()?;
        let complete = end == bounds.end;
        let mut starts = text
            .grapheme_indices(true)
            .map(|(byte, _)| byte)
            .collect::<Vec<_>>();
        if complete {
            starts.push(text.len());
        } else {
            // The last cluster may continue past the prefix.
            starts.pop();
        }
        if let Some(&byte) = starts.get(column) {
            return Some(bounds.start + text[..byte].chars().count());
        }
        if complete {
            return None;
        }
        length = length.saturating_mul(2);
    }
}

#[cfg(test)]
mod tests {
    use super::grapheme_column_to_char;
    use guinea_core::Document;
    use unicode_segmentation::UnicodeSegmentation;

    #[test]
    fn bounded_goto_matches_full_segmentation_across_prefix_growth() {
        let line = "e\u{301}👩\u{200d}💻x".repeat(500);
        let text = format!("first\r\n{line}\r\nlast");
        let document = Document::from_text(&text);
        let starts: Vec<usize> = line
            .grapheme_indices(true)
            .map(|(byte, _)| line[..byte].chars().count())
            .chain(std::iter::once(line.chars().count()))
            .collect();
        let origin = "first\r\n".chars().count();
        for column in [0, 1, 2, 63, 64, 65, 200, 1499, 1500] {
            assert_eq!(
                grapheme_column_to_char(&document, 1, column),
                starts.get(column).map(|chars| origin + chars),
                "column {column}"
            );
        }
        assert_eq!(grapheme_column_to_char(&document, 1, 1501), None);
        assert_eq!(grapheme_column_to_char(&document, 9, 0), None);
    }
}
