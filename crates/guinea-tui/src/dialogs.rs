use crate::{
    app::{App, Modal},
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
    let block = panel(title, theme, true);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
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
            .bg(if primary { theme.mint } else { theme.raised })
            .fg(if primary { theme.canvas } else { theme.text })
            .add_modifier(Modifier::BOLD),
    );
}

pub fn draw(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) -> bool {
    let locale = app.config.language;
    let hits = &mut app.hits;
    match app.modal.as_mut() {
        Some(Modal::Settings { selected }) => {
            let body = surface(
                frame,
                area,
                88,
                20,
                tr(locale, "SETTINGS · F8", "THIẾT LẬP · F8"),
                theme,
            );
            text(
                frame,
                row(body, 1, 1),
                Line::from(tr(locale, "GENERAL  /  LANGUAGE", "CHUNG  /  NGÔN NGỮ")),
                bold(theme.blue),
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
                            bold(if chosen { theme.mint } else { theme.text }),
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
                tr(locale, "REVIEW OPTIONS · F7", "CHẾ ĐỘ REVIEW · F7"),
                theme,
            );
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
                fg(theme.blue),
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
            );
            if body.height < 9 || body.width < 15 {
                return true;
            }
            let parent = Rect::new(body.x + 2, body.y + 1, 8, 1);
            button(frame, parent, " ↑ .. ", theme, false);
            picker.hits.parent = parent;
            text(
                frame,
                Rect::new(body.x + 12, body.y + 1, body.width.saturating_sub(14), 1),
                Line::from(clipped(
                    &picker.root.to_string_lossy(),
                    body.width.saturating_sub(14) as usize,
                )),
                fg(theme.muted),
            );
            text(
                frame,
                row(body, 3, 1),
                Line::from(tr(locale, "Open file or folder", "Mở file hoặc thư mục")),
                bold(theme.text),
            );
            let query = format!(
                " › {}│",
                input_tail(&picker.query, body.width.saturating_sub(9) as usize)
            );
            text(
                frame,
                row(body, 4, 1),
                Line::from(query),
                Style::default().bg(theme.raised).fg(theme.mint),
            );
            let list = Rect::new(
                body.x + 2,
                body.y + 6,
                body.width.saturating_sub(4),
                body.height.saturating_sub(10),
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
                    clipped(&label, list.width.saturating_sub(17) as usize),
                    if selected {
                        bold(theme.text)
                    } else {
                        fg(if entry.directory {
                            theme.blue
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
                        theme.mint
                    } else {
                        theme.faint
                    }),
                );
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
            text(
                frame,
                row(body, body.height - 4, 1),
                Line::from(clipped(&status, body.width.saturating_sub(4) as usize)),
                fg(theme.blue),
            );
            let open = Rect::new(body.x + 2, body.bottom() - 2, 20, 1);
            let cancel = Rect::new(body.x + 25, body.bottom() - 2, 18, 1);
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
                    "↑↓ Select · Type to filter/path · Tab Complete · Alt+↑ Parent",
                    "↑↓ Chọn · Gõ tên/đường dẫn · Tab Hoàn tất · Alt+↑ Thư mục cha",
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
                    fg(theme.blue),
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
                        .fg(theme.mint)
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
                fg(theme.blue),
            );
        }
        _ => return false,
    }
    true
}
