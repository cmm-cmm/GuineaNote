use crate::{
    app::{Action, App, HitRegions, Modal},
    i18n::Locale,
    render::safe,
    theme::Theme,
    workspace_render::{clipped, input_tail, panel},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

fn tr<'a>(locale: Locale, en: &'a str, vi: &'a str) -> &'a str {
    if locale == Locale::Vi { vi } else { en }
}
fn fg(color: ratatui::style::Color) -> Style {
    Style::default().fg(color)
}
fn bold(color: ratatui::style::Color) -> Style {
    fg(color).add_modifier(Modifier::BOLD)
}
fn surface(
    frame: &mut Frame<'_>,
    area: Rect,
    width: u16,
    height: u16,
    title: &str,
    theme: Theme,
    hits: &mut HitRegions,
) -> Rect {
    let width = area.width.saturating_sub(4).min(width);
    let height = area.height.saturating_sub(2).min(height);
    let rect = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let shadow = Rect::new(
        rect.x + 1,
        rect.y + 1,
        width.min(area.right().saturating_sub(rect.x + 1)),
        height.min(area.bottom().saturating_sub(rect.y + 1)),
    );
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.canvas)),
        shadow,
    );
    frame.render_widget(Clear, rect);
    let warning = title.contains("CONFIRM") || title.contains("XÁC NHẬN");
    let block = panel("", theme, true)
        .border_style(fg(if warning { theme.warm } else { theme.secondary }))
        .title(Line::styled(
            format!(
                " {} ",
                clipped(title, rect.width.saturating_sub(8) as usize)
            ),
            bold(if warning { theme.warm } else { theme.text }).bg(theme.raised),
        ));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    hits.modal = rect;
    if rect.width >= 8 && rect.height >= 3 {
        let close = Rect::new(rect.right() - 5, rect.y, 3, 1);
        text(
            frame,
            close,
            Line::from(" x "),
            fg(theme.text).bg(theme.raised),
        );
        hits.modal_actions.push((close, "cancel"));
    }
    inner
}
fn text(frame: &mut Frame<'_>, area: Rect, value: impl Into<Line<'static>>, style: Style) {
    frame.render_widget(Paragraph::new(value.into()).style(style), area);
}
fn row(area: Rect, offset: u16, height: u16) -> Rect {
    Rect::new(
        area.x + 2,
        area.y + offset,
        area.width.saturating_sub(4),
        height.min(area.height.saturating_sub(offset)),
    )
}
fn button(frame: &mut Frame<'_>, rect: Rect, label: &str, theme: Theme, primary: bool) {
    text(
        frame,
        rect,
        Line::from(label.to_owned()),
        Style::default()
            .bg(if primary { theme.accent } else { theme.raised })
            .fg(if primary { theme.canvas } else { theme.text })
            .add_modifier(Modifier::BOLD),
    );
}

fn decision(
    frame: &mut Frame<'_>,
    rect: Rect,
    label: &str,
    selected: bool,
    destructive: bool,
    theme: Theme,
) {
    let tone = if destructive {
        theme.warm
    } else {
        theme.accent
    };
    let block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(fg(if selected { tone } else { theme.line }))
        .style(
            fg(if selected { theme.canvas } else { tone }).bg(if selected {
                tone
            } else {
                theme.raised
            }),
        );
    if rect.height >= 3 {
        let inner = block.inner(rect);
        frame.render_widget(block, rect);
        text(
            frame,
            inner,
            Line::from(clipped(label, inner.width as usize))
                .alignment(ratatui::layout::Alignment::Center),
            bold(if selected { theme.canvas } else { tone }),
        );
    } else {
        button(
            frame,
            rect,
            &clipped(label, rect.width as usize),
            theme,
            selected,
        );
    }
}

pub fn draw(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) -> bool {
    let locale = app.config.language;
    let active = app.active;
    // F2 shows why an action is unavailable; computed before borrowing hits.
    let unavailable: Vec<(&'static str, &'static str)> = crate::commands::COMMANDS
        .iter()
        .filter_map(|c| app.unavailable(c.id).map(|reason| (c.id, reason)))
        .collect();
    let hits = &mut app.hits;
    match app.modal.as_mut() {
        Some(Modal::Search(search)) => {
            let body = surface(
                frame,
                area,
                100,
                26,
                tr(
                    locale,
                    if area.width < 72 {
                        "FIND / REPLACE"
                    } else {
                        "FIND / REPLACE · Ctrl+F / Ctrl+H"
                    },
                    if area.width < 72 {
                        "TÌM / THAY THẾ"
                    } else {
                        "TÌM / THAY THẾ · Ctrl+F / Ctrl+H"
                    },
                ),
                theme,
                hits,
            );
            let compact = body.height < 18 || body.width < 72;
            let tight = body.height < 7;
            let query_y = if compact { 0 } else { 3 };
            if !compact {
                // Clickable option toggles; Alt+C / Alt+W / Alt+R are the
                // keyboard path.
                let toggles = [
                    (
                        !search.options.ignore_case,
                        tr(locale, "Match case · Alt+C", "Phân biệt hoa thường · Alt+C"),
                        "search-case",
                    ),
                    (
                        search.options.whole_word,
                        tr(locale, "Whole word · Alt+W", "Khớp cả từ · Alt+W"),
                        "search-word",
                    ),
                    (
                        search.options.regex,
                        tr(locale, "Regex · Alt+R", "Regex · Alt+R"),
                        "search-regex",
                    ),
                ];
                // Same left margin as the other panel rows.
                let line = row(body, 0, 1);
                let mut x = line.x;
                for (on, label, action) in toggles {
                    let label = format!("{} {label}", if on { "[x]" } else { "[ ]" });
                    let width = (Line::from(label.as_str()).width() as u16)
                        .min(line.right().saturating_sub(x));
                    let rect = Rect::new(x, line.y, width, 1);
                    text(
                        frame,
                        rect,
                        Line::from(label),
                        if on {
                            fg(theme.accent)
                        } else {
                            fg(theme.muted)
                        },
                    );
                    hits.modal_actions.push((rect, action));
                    x = x.saturating_add(width + 3).min(line.right());
                }
                text(
                    frame,
                    row(body, 2, 1),
                    Line::from(tr(locale, "Find", "Tìm kiếm")),
                    bold(theme.accent),
                );
            }
            for (index, offset, action) in [
                (0, query_y, "search-query"),
                (
                    1,
                    if tight {
                        1
                    } else if compact {
                        2
                    } else {
                        6
                    },
                    "search-replacement",
                ),
            ] {
                if index == 1 && !compact {
                    text(
                        frame,
                        row(body, 5, 1),
                        Line::from(tr(
                            locale,
                            "Replace with · empty text deletes the match",
                            "Thay bằng · để trống để xóa kết quả khớp",
                        )),
                        bold(theme.warm),
                    );
                }
                let rect = row(body, offset, 1);
                let prefix = if compact {
                    if index == 0 { "F › " } else { "R › " }
                } else {
                    "› "
                };
                let prefix_width = Line::from(prefix).width() as u16;
                let (value, caret) =
                    search.field(index, rect.width.saturating_sub(prefix_width + 2) as usize);
                text(
                    frame,
                    rect,
                    Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(
                            value,
                            fg(theme.text).bg(
                                if search.focus == index && search.field_selected(index) {
                                    theme.line
                                } else {
                                    theme.raised
                                },
                            ),
                        ),
                    ]),
                    fg(if search.focus == index {
                        theme.text
                    } else {
                        theme.muted
                    })
                    .bg(theme.raised),
                );
                if !search.replacing() {
                    hits.modal_actions.push((rect, action));
                }
                if search.focus == index && !search.replacing() && rect.height > 0 && rect.width > 3
                {
                    frame.set_cursor_position((
                        rect.x + (caret as u16 + prefix_width).min(rect.width - 1),
                        rect.y,
                    ));
                }
            }
            let status_y = if tight {
                2
            } else if compact {
                3
            } else {
                8
            };
            let status = if let Some(error) = &search.error {
                locale.message(error)
            } else if search.replacing() {
                tr(
                    locale,
                    "Replacing… Esc cancels before apply",
                    "Đang thay thế… Esc hủy trước khi áp dụng",
                )
                .into()
            } else if search.busy() {
                tr(locale, "Searching…", "Đang tìm kiếm…").into()
            } else if search.query.is_empty() {
                tr(locale, "Type the text to find", "Nhập nội dung cần tìm").into()
            } else if search.total == 0 {
                tr(locale, "No matches", "Không có kết quả").into()
            } else if locale == Locale::Vi {
                format!(
                    "Kết quả {} / {}",
                    search.offset + search.selected + 1,
                    search.total
                )
            } else {
                format!(
                    "Match {} / {}",
                    search.offset + search.selected + 1,
                    search.total
                )
            };
            text(
                frame,
                row(body, status_y, 1),
                Line::from(clipped(&status, body.width.saturating_sub(4) as usize)),
                bold(if search.error.is_some() {
                    theme.warm
                } else {
                    theme.secondary
                }),
            );
            if !compact {
                let note = if app.tabs[active].document.readonly {
                    tr(
                        locale,
                        "Read-only document · replacement is disabled",
                        "Tài liệu chỉ đọc · không thể thay thế",
                    )
                    .into()
                } else if let Some(count) = search.replaced {
                    if locale == Locale::Vi {
                        format!("Đã thay {count} kết quả · Ctrl+Z hoàn tác sau khi đóng")
                    } else {
                        format!("Replaced {count} matches · Ctrl+Z undoes after closing")
                    }
                } else {
                    format!(
                        "{} → {}",
                        clipped(&search.query, 30),
                        if search.replacement.is_empty() {
                            tr(locale, "(empty: delete)", "(trống: xóa)").to_owned()
                        } else {
                            clipped(&search.replacement, 42)
                        }
                    )
                };
                text(
                    frame,
                    row(body, status_y + 1, 1),
                    Line::from(clipped(&note, body.width.saturating_sub(4) as usize)),
                    fg(theme.warm),
                );
            }
            let list_y = status_y + if compact { 1 } else { 3 };
            let list = row(
                body,
                list_y,
                body.height
                    .saturating_sub(list_y + if compact { 3 } else { 4 }),
            );
            search.list = list;
            search.height = list.height.max(1) as usize;
            for (row_index, found) in search
                .entries
                .iter()
                .skip(search.top())
                .take(list.height as usize)
                .enumerate()
            {
                let chosen = search.top() + row_index == search.selected;
                let line = format!(
                    "{} {:>5}  {}",
                    if chosen { "›" } else { " " },
                    found.line + 1,
                    found.preview
                );
                text(
                    frame,
                    Rect::new(list.x, list.y + row_index as u16, list.width, 1),
                    Line::from(clipped(&line, list.width as usize)),
                    fg(if chosen && search.focus == 2 {
                        theme.canvas
                    } else if chosen {
                        theme.text
                    } else {
                        theme.muted
                    })
                    .bg(if chosen && search.focus == 2 {
                        theme.accent
                    } else if chosen {
                        theme.raised
                    } else {
                        theme.panel
                    }),
                );
            }
            let mut x = body.x + 2;
            let actions = [
                (" < Prev ", " < Trước ", "search-previous", search.ready()),
                (" Next > ", " Sau > ", "search-next", search.ready()),
                (" Go ", " Về kết quả ", "search-go", search.ready()),
                (
                    " Replace ",
                    " Thay ",
                    "search-replace",
                    search.ready() && !app.tabs[active].document.readonly,
                ),
                (
                    " Replace all ",
                    " Thay tất cả ",
                    "search-all",
                    search.ready() && !app.tabs[active].document.readonly,
                ),
            ];
            // Keep the focused button visible even when the row cannot fit
            // all actions. Tab/Shift+Tab can reach every action at 30x9.
            let first = if compact && search.focus >= 3 {
                search.focus - 3
            } else {
                0
            };
            for (index, (en, vi, action, enabled)) in actions.into_iter().enumerate().skip(first) {
                let label = tr(locale, en, vi);
                let width = Line::from(label).width() as u16;
                if x + width > body.right() {
                    break;
                }
                let rect = Rect::new(
                    x,
                    body.bottom().saturating_sub(if compact { 2 } else { 3 }),
                    width,
                    1,
                );
                button(
                    frame,
                    rect,
                    label,
                    theme,
                    enabled && search.focus == index + 3,
                );
                if !enabled {
                    text(
                        frame,
                        rect,
                        Line::from(label),
                        fg(theme.faint).bg(theme.panel).add_modifier(
                            if search.focus == index + 3 {
                                Modifier::UNDERLINED
                            } else {
                                Modifier::empty()
                            },
                        ),
                    );
                }
                if enabled {
                    hits.modal_actions.push((rect, action));
                }
                x += width + 1;
            }
            if !compact {
                text(
                    frame,
                    row(body, body.height - 1, 1),
                    Line::from(tr(
                        locale,
                        "Tab / Shift+Tab Focus · Enter Act · F3 Next · ^R Replace · Alt+A All · Esc Close",
                        "Tab / Shift+Tab Chọn · Enter Chạy · F3 Sau · ^R Thay · Alt+A Tất cả · Esc Đóng",
                    )),
                    fg(theme.muted),
                );
            } else {
                text(
                    frame,
                    row(body, body.height.saturating_sub(1), 1),
                    Line::from("Tab Enter Alt+A Esc"),
                    fg(theme.muted),
                );
            }
        }
        Some(Modal::Confirm {
            action,
            message,
            selected,
        }) => {
            let body = surface(
                frame,
                area,
                88,
                18,
                tr(
                    locale,
                    "UNSAVED CHANGES / CONFIRM",
                    "THAY ĐỔI CHƯA LƯU / XÁC NHẬN",
                ),
                theme,
                hits,
            );
            let (heading, description, accept) = match action {
                Action::Close | Action::CloseTab(_) => (
                    tr(locale, "Close this tab?", "Đóng thẻ này?"),
                    tr(
                        locale,
                        "Discard unsaved changes in this tab? The saved file stays unchanged.",
                        "Bỏ thay đổi chưa lưu trong thẻ này? File đã lưu được giữ nguyên.",
                    ),
                    tr(locale, "Discard & close", "Bỏ và đóng"),
                ),
                Action::Quit => (
                    tr(
                        locale,
                        "Leave this workspace?",
                        "Thoát không gian làm việc?",
                    ),
                    tr(
                        locale,
                        "Discard ALL unsaved tabs and quit? Saved files stay unchanged.",
                        "Bỏ TẤT CẢ thay đổi chưa lưu và thoát? File đã lưu được giữ nguyên.",
                    ),
                    tr(locale, "Discard & quit", "Bỏ và thoát"),
                ),
                Action::Reload => (
                    tr(locale, "Reload this file?", "Tải lại file này?"),
                    tr(
                        locale,
                        "Reload from disk and discard this tab's edits/history?",
                        "Tải lại từ đĩa và bỏ thay đổi cùng lịch sử của thẻ này?",
                    ),
                    tr(locale, "Reload file", "Tải lại file"),
                ),
                Action::OpenLarge { .. } => (
                    tr(locale, "Open a large file?", "Mở file dung lượng lớn?"),
                    tr(
                        locale,
                        "The file will be loaded into memory. Opening may take longer.",
                        "File sẽ được nạp vào bộ nhớ. Thời gian mở có thể lâu hơn.",
                    ),
                    tr(locale, "Open anyway", "Vẫn mở"),
                ),
            };
            text(
                frame,
                row(body, 0, 2),
                Line::from(format!("!  {heading}")),
                bold(theme.warm),
            );
            if body.height >= 10 {
                let target = match action {
                    Action::CloseTab(index) => *index,
                    _ => active,
                };
                let detail = if matches!(action, Action::Quit) {
                    let dirty = app
                        .tabs
                        .iter()
                        .filter(|t| t.review_source.is_none() && t.document.is_dirty());
                    let count = dirty.clone().count();
                    format!(
                        "{} {} · {}",
                        count,
                        tr(locale, "unsaved tabs", "thẻ chưa lưu"),
                        dirty
                            .take(3)
                            .map(|t| t.title(locale))
                            .collect::<Vec<_>>()
                            .join(" · ")
                    )
                } else if let Action::OpenLarge { path, .. } = action {
                    path.to_string_lossy().into_owned()
                } else {
                    app.tabs
                        .get(target)
                        .map(|t| t.title(locale))
                        .unwrap_or_default()
                };
                text(
                    frame,
                    row(body, 3, 2),
                    Line::from(clipped(&detail, body.width.saturating_sub(4) as usize)),
                    bold(theme.text),
                );
                frame.render_widget(
                    Paragraph::new(description)
                        .style(fg(theme.muted))
                        .wrap(Wrap { trim: false }),
                    row(body, 6, 2),
                );
                let hint = if matches!(action, Action::OpenLarge { .. }) {
                    message.split(" Y ").next().unwrap_or(message)
                } else {
                    tr(
                        locale,
                        "Keep working to save your changes first.",
                        "Chọn Tiếp tục để lưu các thay đổi trước.",
                    )
                };
                text(
                    frame,
                    row(body, 8, 1),
                    Line::from(clipped(hint, body.width.saturating_sub(4) as usize)),
                    fg(theme.faint),
                );
            }
            let bh = if body.height >= 5 { 3 } else { 1 };
            let by = body
                .bottom()
                .saturating_sub(bh + u16::from(body.height >= 10));
            let available = body.width.saturating_sub(4);
            let width = available.saturating_sub(2) / 2;
            let cancel = Rect::new(body.x + 2, by, width, bh);
            let confirm = Rect::new(
                cancel.right() + 2,
                by,
                available.saturating_sub(width + 2),
                bh,
            );
            decision(
                frame,
                cancel,
                tr(locale, "Keep working", "Tiếp tục"),
                !*selected,
                false,
                theme,
            );
            decision(
                frame,
                confirm,
                accept,
                *selected,
                !matches!(action, Action::OpenLarge { .. }),
                theme,
            );
            hits.modal_actions.push((cancel, "cancel"));
            hits.modal_actions.push((confirm, "confirm-accept"));
            if body.height >= 10 {
                text(
                    frame,
                    row(body, body.height - 1, 1),
                    Line::from(tr(
                        locale,
                        "Tab / ← → Select · Enter Choose · Y Confirm · Esc Keep working",
                        "Tab / ← → Chọn · Enter Thực hiện · Y Xác nhận · Esc Tiếp tục",
                    )),
                    fg(theme.muted),
                );
            }
        }
        Some(Modal::Input { kind, value }) => {
            let command = matches!(kind, crate::app::InputKind::Command);
            let body = surface(
                frame,
                area,
                88,
                if command { 24 } else { 16 },
                tr(locale, "OPEN / SEARCH / SAVE", "MỞ / TÌM / LƯU"),
                theme,
                hits,
            );
            text(
                frame,
                row(body, 0, 1),
                Line::from(locale.tr(kind.label()).into_owned()),
                bold(theme.text),
            );
            let field = row(body, if body.height >= 8 { 2 } else { 1 }, 1);
            let tail = input_tail(value, field.width.saturating_sub(3) as usize);
            text(
                frame,
                field,
                Line::from(format!("› {tail}│")),
                fg(theme.text).bg(theme.raised),
            );
            if field.width > 3 {
                frame.set_cursor_position((
                    field.x + (Line::from(tail.as_str()).width() as u16 + 2).min(field.width - 1),
                    field.y,
                ));
            }
            if command && body.height >= 8 {
                let suggestions = crate::commands::suggestions(value, locale);
                let height = body.height.saturating_sub(8) as usize;
                let top = app.palette_index.saturating_sub(height.saturating_sub(1));
                for (index, command) in suggestions.iter().enumerate().skip(top).take(height) {
                    let rect = row(body, 5 + (index - top) as u16, 1);
                    let reason = unavailable
                        .iter()
                        .find(|(id, _)| *id == command.id)
                        .map(|(_, reason)| *reason);
                    let label = format!(
                        "{:<16} {}{}",
                        command.id,
                        crate::commands::description(command, locale),
                        reason.map_or(String::new(), |r| format!(" — {}", locale.tr(r)))
                    );
                    text(
                        frame,
                        rect,
                        Line::from(clipped(&label, rect.width as usize)),
                        fg(if index == app.palette_index {
                            theme.canvas
                        } else {
                            theme.text
                        })
                        .bg(if index == app.palette_index {
                            theme.accent
                        } else {
                            theme.panel
                        }),
                    );
                    hits.commands.push((rect, command.id));
                }
            } else if body.height >= 10 {
                text(
                    frame,
                    row(body, 5, 2),
                    Line::from(tr(
                        locale,
                        "Enter to submit · Esc to keep working",
                        "Enter để thực hiện · Esc để tiếp tục làm việc",
                    )),
                    fg(theme.muted),
                );
            }
            let y = body.bottom().saturating_sub(2);
            let cancel = Rect::new(body.x + 2, y, 16.min(body.width / 2), 1);
            let submit = Rect::new(
                cancel.right() + 2,
                y,
                body.width.saturating_sub(cancel.width + 6).min(20),
                1,
            );
            button(
                frame,
                cancel,
                tr(locale, " Esc Cancel ", " Esc Hủy "),
                theme,
                false,
            );
            button(
                frame,
                submit,
                tr(locale, " Enter Submit ", " Enter Thực hiện "),
                theme,
                true,
            );
            hits.modal_actions.push((cancel, "cancel"));
            hits.modal_actions.push((submit, "input-submit"));
        }
        Some(Modal::Help) => {
            let body = surface(
                frame,
                area,
                92,
                34,
                tr(locale, "KEYBOARD GUIDE", "HƯỚNG DẪN PHÍM TẮT"),
                theme,
                hits,
            );
            let content = if locale == Locale::Vi {
                crate::i18n::HELP_VI
            } else {
                crate::app::HELP
            };
            let view = row(body, 0, body.height.saturating_sub(1));
            let lines = crate::keyboard::help_lines(content, view.width as usize);
            app.help_rows = lines.len();
            app.help_height = view.height.max(1) as usize;
            app.help_scroll = app
                .help_scroll
                .min(app.help_rows.saturating_sub(app.help_height));
            frame.render_widget(
                Paragraph::new(
                    lines
                        .into_iter()
                        .skip(app.help_scroll)
                        .take(app.help_height)
                        .map(Line::from)
                        .collect::<Vec<_>>(),
                )
                .style(fg(theme.text)),
                view,
            );
            text(
                frame,
                row(body, body.height.saturating_sub(1), 1),
                Line::from(tr(
                    locale,
                    "↑↓ PgUp/PgDn · Esc Close",
                    "↑↓ PgUp/PgDn · Esc Đóng",
                )),
                fg(theme.muted),
            );
        }
        Some(Modal::Tabs(manager)) => {
            let body = surface(
                frame,
                area,
                100,
                29,
                tr(
                    locale,
                    "OPEN TABS · Ctrl+P / F12",
                    "QUẢN LÝ THẺ · Ctrl+P / F12",
                ),
                theme,
                hits,
            );
            let compact = body.height < 17;
            let dirty = manager
                .entries
                .iter()
                .filter(|e| !e.review && e.dirty)
                .count();
            let summary = format!(
                "{} {} · {} {} · {} {}",
                manager.entries.len(),
                tr(locale, "open", "thẻ mở"),
                dirty,
                tr(locale, "unsaved", "chưa lưu"),
                manager.filtered.len(),
                tr(locale, "matching", "kết quả")
            );
            text(
                frame,
                row(body, 0, 1),
                Line::from(summary),
                bold(theme.accent),
            );
            if !compact {
                text(
                    frame,
                    row(body, 2, 1),
                    Line::from(tr(
                        locale,
                        "Filter by name or path · Ctrl+U clear",
                        "Lọc theo tên hoặc đường dẫn · Ctrl+U xóa lọc",
                    )),
                    fg(theme.muted),
                );
            }
            text(
                frame,
                row(body, if compact { 1 } else { 3 }, 1),
                Line::from(format!(
                    "› {}│",
                    input_tail(&manager.query, body.width.saturating_sub(7) as usize)
                )),
                fg(theme.text),
            );
            let list = row(
                body,
                if compact { 2 } else { 5 },
                body.height.saturating_sub(if compact { 4 } else { 14 }),
            );
            manager.list = list;
            manager.height = list.height.max(1) as usize;
            if manager.selected < manager.top {
                manager.top = manager.selected;
            }
            if manager.selected >= manager.top + manager.height {
                manager.top = manager.selected + 1 - manager.height;
            }
            manager.top = manager
                .top
                .min(manager.filtered.len().saturating_sub(manager.height));
            for (offset, &index) in manager
                .filtered
                .iter()
                .skip(manager.top)
                .take(list.height as usize)
                .enumerate()
            {
                let entry = &manager.entries[index];
                let chosen = manager.top + offset == manager.selected;
                let state = if entry.ai {
                    tr(locale, "AI · READ ONLY", "AI · CHỈ ĐỌC")
                } else if entry.review {
                    "REVIEW"
                } else if entry.readonly {
                    tr(locale, "READ ONLY", "CHỈ ĐỌC")
                } else if entry.dirty {
                    tr(locale, "UNSAVED", "CHƯA LƯU")
                } else if entry.path.is_empty() {
                    tr(locale, "NEW BUFFER", "THẺ TRỐNG")
                } else {
                    tr(locale, "SAVED", "ĐÃ LƯU")
                };
                let title = format!(
                    "{} {:>3} {} {}",
                    if entry.index == active { "●" } else { " " },
                    entry.index + 1,
                    if entry.dirty { "*" } else { " " },
                    entry.title
                );
                let state_width = (Line::from(state).width() as u16 + 2).min(list.width / 3);
                let title_width = list.width.saturating_sub(state_width);
                let style = Style::default()
                    .bg(if chosen { theme.raised } else { theme.panel })
                    .fg(if chosen { theme.text } else { theme.muted });
                let y = list.y + offset as u16;
                text(
                    frame,
                    Rect::new(list.x, y, title_width, 1),
                    Line::from(clipped(&title, title_width as usize)),
                    style,
                );
                text(
                    frame,
                    Rect::new(list.x + title_width, y, state_width, 1),
                    Line::from(clipped(state, state_width as usize)),
                    style.fg(if entry.dirty {
                        theme.warm
                    } else {
                        theme.secondary
                    }),
                );
            }
            if manager.filtered.is_empty() {
                text(
                    frame,
                    list,
                    Line::from(tr(locale, "No matching tabs", "Không có thẻ phù hợp")),
                    fg(theme.muted),
                );
            }
            if !compact && let Some(index) = manager.filtered.get(manager.selected) {
                let entry = &manager.entries[*index];
                text(
                    frame,
                    row(body, body.height.saturating_sub(8), 1),
                    Line::from(format!(
                        "{} · {} · {} {}",
                        entry.format,
                        crate::file_info::bytes(entry.bytes),
                        entry.lines,
                        tr(
                            locale,
                            if entry.lines == 1 { "line" } else { "lines" },
                            "dòng"
                        )
                    ))
                    .alignment(ratatui::layout::Alignment::Right),
                    bold(theme.warm),
                );
                if let Some(action) = &entry.ai_action {
                    text(
                        frame,
                        row(body, body.height.saturating_sub(7), 1),
                        Line::from(format!(
                            "AI · {} · {}",
                            locale.ai_action(action),
                            tr(
                                locale,
                                "Other tabs remain editable",
                                "Các thẻ khác vẫn chỉnh sửa được"
                            )
                        )),
                        fg(theme.accent),
                    );
                }
                let path = if entry.path.is_empty() {
                    tr(
                        locale,
                        "Blank document · choose Save to create a file",
                        "Thẻ trống · chọn Lưu để tạo file",
                    )
                } else {
                    &entry.path
                };
                frame.render_widget(
                    Paragraph::new(safe(path))
                        .style(fg(theme.secondary))
                        .wrap(Wrap { trim: false }),
                    row(body, body.height.saturating_sub(6), 3),
                );
            }
            let mut x = body.x + 2;
            for (en, vi, action) in [
                (" Switch ", " Chuyển ", "tabs-switch"),
                (" Save ", " Lưu ", "tabs-save"),
                (" Close ", " Đóng ", "tabs-close"),
                (" + New ", " + Mới ", "tabs-new"),
                (" Cancel ", " Hủy ", "cancel"),
            ] {
                let label = tr(locale, en, vi);
                let width = Line::from(label).width() as u16;
                if x + width > body.right() {
                    break;
                }
                let rect = Rect::new(x, body.bottom().saturating_sub(2), width, 1);
                button(frame, rect, label, theme, action == "tabs-switch");
                hits.modal_actions.push((rect, action));
                x += width + 2;
            }
            text(
                frame,
                row(body, body.height.saturating_sub(1), 1),
                Line::from(tr(
                    locale,
                    "↑↓ / Tab Select · Enter Switch · ^S Save · ^W Close · ^N New · Esc Cancel",
                    "↑↓ / Tab Chọn · Enter Chuyển · ^S Lưu · ^W Đóng · ^N Mới · Esc Hủy",
                )),
                fg(theme.muted),
            );
        }
        Some(Modal::Settings { selected }) => {
            let body = surface(
                frame,
                area,
                88,
                20,
                tr(locale, "SETTINGS · F10", "THIẾT LẬP · F10"),
                theme,
                hits,
            );
            if body.height < 16 {
                text(
                    frame,
                    row(body, 0, 1),
                    Line::from(tr(locale, "Language", "Ngôn ngữ")),
                    bold(theme.text),
                );
                for (offset, lang, label, action) in [
                    (1, Locale::Vi, "Tiếng Việt", "settings-vi"),
                    (2, Locale::En, "English", "settings-en"),
                ] {
                    let rect = row(body, offset, 1);
                    button(frame, rect, label, theme, *selected == lang);
                    hits.modal_actions.push((rect, action));
                }
                let y = body.bottom().saturating_sub(1);
                let apply = Rect::new(body.x + 2, y, body.width.saturating_sub(6) / 2, 1);
                let cancel = Rect::new(apply.right() + 2, y, apply.width, 1);
                button(frame, apply, tr(locale, "Apply", "Áp dụng"), theme, true);
                button(frame, cancel, tr(locale, "Cancel", "Hủy"), theme, false);
                hits.modal_actions.push((apply, "settings-apply"));
                hits.modal_actions.push((cancel, "cancel"));
                return true;
            }
            text(
                frame,
                row(body, 1, 1),
                Line::from(tr(locale, "GENERAL  /  LANGUAGE", "CHUNG  /  NGÔN NGỮ")),
                bold(theme.secondary),
            );
            text(
                frame,
                row(body, 3, 1),
                Line::from(tr(
                    locale,
                    "Make this workspace feel like yours.",
                    "Ngôn ngữ phù hợp với cách bạn làm việc.",
                )),
                bold(theme.text),
            );
            text(
                frame,
                row(body, 5, 2),
                Line::from(tr(
                    locale,
                    "Choose the language for menus, dialogs and messages.",
                    "Chọn ngôn ngữ cho menu, hộp thoại và thông báo.",
                )),
                fg(theme.muted),
            );
            let cards = row(body, 8, 5);
            let width = cards.width.saturating_sub(2) / 2;
            for (index, language, title, subtitle, action) in [
                (0, Locale::Vi, "Tiếng Việt", "Vietnamese", "settings-vi"),
                (1, Locale::En, "English", "Tiếng Anh", "settings-en"),
            ] {
                let rect = Rect::new(cards.x + index * (width + 2), cards.y, width, cards.height);
                let chosen = *selected == language;
                let block = panel(if chosen { " ● " } else { " ○ " }, theme, chosen);
                let inner = block.inner(rect);
                frame.render_widget(block, rect);
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::styled(
                            format!("  {title}"),
                            bold(if chosen { theme.accent } else { theme.text }),
                        ),
                        Line::styled(format!("  {subtitle}"), fg(theme.muted)),
                    ]),
                    inner,
                );
                hits.modal_actions.push((rect, action));
            }
            let footer = body.bottom().saturating_sub(2);
            let apply = Rect::new(body.x + 2, footer, 22.min(body.width.saturating_sub(4)), 1);
            button(
                frame,
                apply,
                tr(locale, " Enter  Apply ", " Enter  Áp dụng "),
                theme,
                true,
            );
            hits.modal_actions.push((apply, "settings-apply"));
            let cancel = Rect::new(
                body.x + 27,
                footer,
                19.min(body.width.saturating_sub(28)),
                1,
            );
            button(
                frame,
                cancel,
                tr(locale, " Esc  Cancel ", " Esc  Hủy "),
                theme,
                false,
            );
            hits.modal_actions.push((cancel, "cancel"));
            text(
                frame,
                row(body, body.height.saturating_sub(1), 1),
                Line::from(tr(
                    locale,
                    "← → Select · saved for the next launch after Apply",
                    "← → Chọn · lưu cho lần mở sau khi Áp dụng",
                )),
                fg(theme.muted),
            );
        }
        Some(Modal::ReviewOptions { selected }) => {
            let body = surface(
                frame,
                area,
                90,
                18,
                tr(locale, "REVIEW OPTIONS · F8", "CHẾ ĐỘ REVIEW · F8"),
                theme,
                hits,
            );
            if body.height < 14 {
                text(
                    frame,
                    row(body, 0, 1),
                    Line::from(tr(locale, "Choose a view", "Chọn chế độ xem")),
                    bold(theme.text),
                );
                for (index, en, vi, action) in [
                    (0, "1 Split view", "1 Xem song song", "review-split"),
                    (1, "2 Review tab", "2 Tab review", "review-tab"),
                ] {
                    let rect = row(body, index as u16 + 2, 1);
                    button(frame, rect, tr(locale, en, vi), theme, *selected == index);
                    hits.modal_actions.push((rect, action));
                }
                return true;
            }
            text(
                frame,
                row(body, 1, 1),
                Line::from(tr(
                    locale,
                    "Choose your reading space.",
                    "Chọn không gian đọc của bạn.",
                )),
                bold(theme.text),
            );
            text(
                frame,
                row(body, 3, 1),
                Line::from(tr(
                    locale,
                    "Both views follow changes in the source document.",
                    "Cả hai chế độ đều cập nhật theo nội dung nguồn.",
                )),
                fg(theme.muted),
            );
            for (index, title, description, action) in [
                (
                    0,
                    tr(locale, "1  Split view", "1  Xem song song"),
                    tr(
                        locale,
                        "Source + review, side by side.",
                        "Nguồn + bản xem trong hai khung.",
                    ),
                    "review-split",
                ),
                (
                    1,
                    tr(locale, "2  Review tab", "2  Tab review riêng"),
                    tr(
                        locale,
                        "A dedicated read-only reading tab.",
                        "Tab đọc riêng, liên kết với nguồn.",
                    ),
                    "review-tab",
                ),
            ] {
                let rect = row(body, 5 + index as u16 * 4, 4);
                let block = panel(title, theme, *selected == index);
                let inner = block.inner(rect);
                frame.render_widget(block, rect);
                text(frame, inner, Line::from(description), fg(theme.muted));
                hits.modal_actions.push((rect, action));
            }
            text(
                frame,
                row(body, body.height.saturating_sub(1), 1),
                Line::from(tr(
                    locale,
                    "↑↓ Choose · Enter Open · 1 / 2 Quick select · Esc Cancel",
                    "↑↓ Chọn · Enter Mở · 1 / 2 Chọn nhanh · Esc Hủy",
                )),
                fg(theme.secondary),
            );
        }
        Some(Modal::FilePicker(picker)) => {
            let body = surface(
                frame,
                area,
                106,
                28,
                tr(locale, "OPEN FILE", "MỞ FILE"),
                theme,
                hits,
            );
            picker.hits = crate::file_picker::Hits::default();
            let compact = body.height < 12;
            let parent = Rect::new(
                body.x + 2,
                body.y + u16::from(!compact),
                8.min(body.width),
                1,
            );
            button(frame, parent, " ↑ .. ", theme, false);
            picker.hits.parent = parent;
            text(
                frame,
                Rect::new(body.x + 12, parent.y, body.width.saturating_sub(14), 1),
                Line::from(clipped(
                    &picker.root.to_string_lossy(),
                    body.width.saturating_sub(14) as usize,
                )),
                fg(theme.muted),
            );
            if !compact {
                text(
                    frame,
                    row(body, 3, 1),
                    Line::from(tr(locale, "Choose a file to open", "Chọn file cần mở")),
                    bold(theme.text),
                );
            }
            let query = format!(
                " › {}│",
                input_tail(&picker.query, body.width.saturating_sub(9) as usize)
            );
            text(
                frame,
                row(body, if compact { 1 } else { 4 }, 1),
                Line::from(query),
                Style::default().bg(theme.raised).fg(theme.accent),
            );
            let list = Rect::new(
                body.x + 2,
                body.y + if compact { 2 } else { 6 },
                body.width.saturating_sub(4),
                body.height.saturating_sub(if compact { 4 } else { 10 }),
            );
            picker.hits.list = list;
            picker.height = list.height.max(1) as usize;
            if picker.selected < picker.top {
                picker.top = picker.selected;
            }
            if picker.selected >= picker.top + picker.height {
                picker.top = picker.selected + 1 - picker.height;
            }
            for (offset, &entry_index) in picker
                .filtered
                .iter()
                .skip(picker.top)
                .take(picker.height)
                .enumerate()
            {
                let entry = &picker.entries[entry_index];
                let selected = picker.top + offset == picker.selected;
                let icon = if entry.directory {
                    "▸"
                } else if entry.symlink {
                    "›"
                } else {
                    "·"
                };
                let label = format!("{} {icon} {}", if selected { "▌" } else { " " }, entry.name);
                let line = Line::from(vec![Span::styled(
                    clipped(
                        &label,
                        list.width
                            .saturating_sub(if list.width >= 38 { 17 } else { 0 })
                            as usize,
                    ),
                    if selected {
                        bold(theme.text)
                    } else {
                        fg(if entry.directory {
                            theme.secondary
                        } else {
                            theme.muted
                        })
                    },
                )]);
                frame.render_widget(
                    Paragraph::new(line).style(Style::default().bg(if selected {
                        theme.raised
                    } else {
                        theme.panel
                    })),
                    Rect::new(list.x, list.y + offset as u16, list.width, 1),
                );
                let kind = if entry.directory {
                    tr(locale, "Folder", "Thư mục")
                } else {
                    guinea_syntax::Language::detect(Some(&entry.path)).label()
                };
                if list.width >= 38 {
                    text(
                        frame,
                        Rect::new(
                            list.right().saturating_sub(15),
                            list.y + offset as u16,
                            15,
                            1,
                        ),
                        Line::from(clipped(kind, 14)),
                        fg(if entry.directory {
                            theme.accent
                        } else {
                            theme.faint
                        }),
                    );
                }
            }
            if picker.filtered.is_empty() {
                text(
                    frame,
                    list,
                    Line::from(tr(
                        locale,
                        "No matching files · enter a path to open or create a buffer",
                        "Không có file khớp · nhập đường dẫn để mở hoặc tạo bộ đệm",
                    )),
                    fg(theme.muted),
                );
            }
            let status = if picker.status.is_empty() {
                format!(
                    "{} {}{}",
                    picker.filtered.len(),
                    tr(locale, "entries", "mục"),
                    if picker.capped {
                        tr(locale, " · 10,000-entry cap", " · giới hạn 10.000 mục")
                    } else {
                        ""
                    }
                )
            } else {
                locale.message(&picker.status)
            };
            if !compact {
                text(
                    frame,
                    row(body, body.height - 4, 1),
                    Line::from(clipped(&status, body.width.saturating_sub(4) as usize)),
                    fg(theme.secondary),
                );
            }
            let button_width = (body.width.saturating_sub(6) / 2).min(20);
            let open = Rect::new(body.x + 2, body.bottom() - 2, button_width, 1);
            let cancel = Rect::new(open.right() + 2, body.bottom() - 2, button_width, 1);
            button(
                frame,
                open,
                tr(locale, " Enter  Open ", " Enter  Mở "),
                theme,
                true,
            );
            button(
                frame,
                cancel,
                tr(locale, " Esc  Cancel ", " Esc  Hủy "),
                theme,
                false,
            );
            picker.hits.open = open;
            picker.hits.cancel = cancel;
            text(
                frame,
                row(body, body.height - 1, 1),
                Line::from(tr(
                    locale,
                    "Double-click / Enter Open · Wheel Browse · Tab Complete · Alt+Up Parent",
                    "Nhấp đúp / Enter Mở · Cuộn Duyệt · Tab Hoàn tất · Alt+Up Thư mục cha",
                )),
                fg(theme.muted),
            );
        }
        Some(Modal::FileInfo(info)) => {
            let body = surface(
                frame,
                area,
                100,
                30,
                tr(locale, "FILE DETAILS · F9", "THÔNG TIN FILE · F9"),
                theme,
                hits,
            );
            if body.height < 8 {
                return true;
            }
            if let Some(details) = &info.details {
                text(
                    frame,
                    row(body, 1, 1),
                    Line::from(clipped(
                        &details.name,
                        body.width.saturating_sub(4) as usize,
                    )),
                    bold(theme.text),
                );
                text(
                    frame,
                    row(body, 2, 1),
                    Line::from(format!("{} · UTF-8", details.language)),
                    fg(theme.secondary),
                );
                let summary = format!(
                    " {} {}    │    {} {}    │    {} ",
                    tr(locale, "SIZE", "DUNG LƯỢNG"),
                    crate::file_info::bytes(details.size),
                    details.lines,
                    tr(locale, "lines", "dòng"),
                    if details.dirty {
                        tr(locale, "MODIFIED", "ĐÃ SỬA")
                    } else {
                        tr(locale, "SAVED / UNCHANGED", "ĐÃ LƯU / CHƯA SỬA")
                    }
                );
                text(
                    frame,
                    row(body, 4, 2),
                    Line::from(summary),
                    Style::default()
                        .bg(theme.raised)
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                );
                let mut y = body.y + 7;
                let end = body.bottom().saturating_sub(2);
                info.height = end.saturating_sub(y).max(1) as usize;
                for field in details.fields.iter().skip(info.top) {
                    if y >= end {
                        break;
                    }
                    let label = tr(locale, field.en, field.vi);
                    text(
                        frame,
                        Rect::new(body.x + 2, y, 23.min(body.width), 1),
                        Line::from(label),
                        fg(theme.muted),
                    );
                    let width = body.width.saturating_sub(28).max(1);
                    let height = ((Line::from(safe(&field.value))
                        .width()
                        .div_ceil(width as usize))
                    .max(1) as u16)
                        .min(4)
                        .min(end - y);
                    frame.render_widget(
                        Paragraph::new(safe(&field.value))
                            .style(fg(theme.text))
                            .wrap(Wrap { trim: false }),
                        Rect::new(body.x + 26, y, width, height),
                    );
                    y += height + 1;
                }
            } else {
                text(
                    frame,
                    row(body, 2, 3),
                    Line::from(info.error.as_ref().map(|e| safe(e)).unwrap_or_else(|| {
                        tr(
                            locale,
                            "Reading metadata and counting text…",
                            "Đang đọc metadata và thống kê nội dung…",
                        )
                        .into()
                    })),
                    fg(theme.muted),
                );
            }
            text(
                frame,
                row(body, body.height - 1, 1),
                Line::from(tr(
                    locale,
                    "↑↓ / PgUp PgDn Scroll · Snapshot at open · Esc Close",
                    "↑↓ / PgUp PgDn Cuộn · Thống kê lúc mở dialog · Esc Đóng",
                )),
                fg(theme.secondary),
            );
        }
        _ => return false,
    }
    true
}
