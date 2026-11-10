//! Workspace surfaces and interaction geometry. All text edits stay in core.
use guinea_core::TextBuffer;
use guinea_syntax::Language;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    app::{App, Focus, HitRegions},
    i18n::Locale,
    render::{LinePaint, safe, styled_line},
    review::ViewMode,
    theme::Theme,
};

fn text_style(color: ratatui::style::Color) -> Style {
    Style::default().fg(color)
}
fn bold(color: ratatui::style::Color) -> Style {
    text_style(color).add_modifier(Modifier::BOLD)
}

pub(crate) fn clipped(text: &str, width: usize) -> String {
    let safe = safe(text);
    if Line::from(safe.as_str()).width() <= width {
        return safe;
    }
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut cells = 0;
    for grapheme in safe.graphemes(true) {
        let size = Line::from(grapheme).width();
        if cells + size >= width {
            break;
        }
        out.push_str(grapheme);
        cells += size;
    }
    out.push('…');
    out
}

pub(crate) fn input_tail(text: &str, width: usize) -> String {
    let safe = safe(text);
    if Line::from(safe.as_str()).width() <= width {
        return safe;
    }
    let mut cells = 1;
    let mut start = safe.len();
    for (byte, grapheme) in safe.grapheme_indices(true).rev() {
        cells += Line::from(grapheme).width();
        if cells > width {
            break;
        }
        start = byte;
    }
    format!("…{}", &safe[start..])
}

pub(crate) fn panel(title: &str, theme: Theme, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(text_style(if focused { theme.accent } else { theme.line }))
        .style(Style::default().bg(theme.panel).fg(theme.text))
        .title(Line::styled(
            format!(" {title} "),
            bold(if focused { theme.accent } else { theme.muted }),
        ))
}

fn header(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let locale = app.config.language;
    let brand = Line::from(vec![
        Span::styled(" G/ ", bold(theme.accent)),
        Span::styled("GUINEA", bold(theme.text)),
        Span::styled("NOTE", bold(theme.accent)),
        Span::styled(
            locale.tr("    A little more focused.").into_owned(),
            text_style(theme.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(brand),
        Rect::new(area.x, area.y, area.width, 1),
    );
    if area.height < 2 {
        return;
    }
    if area.width > 85 {
        frame.render_widget(
            Paragraph::new(locale.tr("WORKSPACE  /  v0.1.0 ").into_owned())
                .style(text_style(theme.faint))
                .alignment(Alignment::Right),
            Rect::new(area.x + area.width - 27, area.y, 27, 1),
        );
    }
    let mut x = area.x + 1;
    for (label, command) in [
        (" Open file  ^O ", "open"),
        (" Open folder  ^K ", "open-folder"),
        (" New  ^N ", "new"),
        (" Save  ^S ", "save"),
        (" Find ^F ", "find"),
        (" Replace ^H ", "replace"),
        (" Review  F7 ", "preview"),
        (" Settings F8 ", "settings"),
        (" Info F9 ", "info"),
    ] {
        let label = if area.width < 110 {
            match command {
                "open" => " Open ^O ",
                "open-folder" => " Folder ^K ",
                "new" => " New ^N ",
                "save" => " Save ^S ",
                "preview" => " View F7 ",
                "settings" => " F8 ",
                "info" => " F9 ",
                _ => label,
            }
        } else {
            label
        };
        let label = locale.tr(label);
        let width = Line::from(label.as_ref()).width() as u16;
        if x + width > area.right() {
            break;
        }
        let rect = Rect::new(x, area.y + 1, width, 1);
        frame.render_widget(
            Paragraph::new(label.into_owned()).style(Style::default().bg(theme.raised).fg(
                if app.tabs[app.active].ai_action.is_some() && command == "save" {
                    theme.faint
                } else {
                    theme.muted
                },
            )),
            rect,
        );
        app.hits.actions.push((rect, command));
        x += width + 1;
    }
    if area.right().saturating_sub(x) >= 25 {
        frame.render_widget(
            Paragraph::new(locale.tr("F1 Help   F2 Commands ").into_owned())
                .style(text_style(theme.secondary))
                .alignment(Alignment::Right),
            Rect::new(area.right() - 25, area.y + 1, 25, 1),
        );
    }
}

fn explorer(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let locale = app.config.language;
    let focused = app.focus == Focus::Explorer;
    let block = panel(&locale.tr("EXPLORER"), theme, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height < 3 {
        return;
    }
    let name = format!(" ▾ {}", app.explorer.name());
    frame.render_widget(
        Paragraph::new(clipped(&name, inner.width as usize)).style(bold(theme.text)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new("─".repeat(inner.width as usize)).style(text_style(theme.line)),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let body = Rect::new(inner.x, inner.y + 2, inner.width, inner.height - 3);
    app.hits.tree = body;
    app.explorer.viewport_height = body.height.max(1) as usize;
    let tree = &mut app.explorer;
    if tree.selected < tree.top {
        tree.top = tree.selected;
    }
    if tree.selected >= tree.top + body.height.max(1) as usize {
        tree.top = tree.selected + 1 - body.height.max(1) as usize;
    }
    let active_path = app.tabs[app.active].document.path.as_deref();
    for (offset, row) in tree
        .rows
        .iter()
        .skip(tree.top)
        .take(body.height as usize)
        .enumerate()
    {
        let index = tree.top + offset;
        let selected = index == tree.selected;
        let mut guides = String::new();
        for &continuation in &row.guides {
            guides.push_str(if continuation { "│ " } else { "  " });
        }
        guides.push_str(if row.last { "└ " } else { "├ " });
        let icon = if row.entry.symlink {
            "›"
        } else if row.entry.directory {
            if tree.is_expanded(&row.entry.path) {
                "▾"
            } else {
                "▸"
            }
        } else {
            file_icon(&row.entry.path)
        };
        let mut spans = vec![
            Span::styled(
                if selected { "▌" } else { " " },
                text_style(if focused {
                    theme.accent
                } else {
                    theme.secondary
                }),
            ),
            Span::styled(guides, text_style(theme.faint)),
            Span::styled(
                format!("{icon} "),
                text_style(if row.entry.directory {
                    theme.accent
                } else {
                    theme.secondary
                }),
            ),
            Span::styled(
                safe(&row.entry.name),
                if selected {
                    bold(theme.text)
                } else {
                    text_style(theme.muted)
                },
            ),
        ];
        if active_path == Some(row.entry.path.as_path()) {
            spans.push(Span::styled(" ●", text_style(theme.accent)));
        }
        let style = Style::default().bg(if selected { theme.raised } else { theme.panel });
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(style),
            Rect::new(body.x, body.y + offset as u16, body.width, 1),
        );
    }
    if tree.rows.is_empty() {
        let message = if tree.busy() {
            "  Loading files…"
        } else if tree.status.starts_with("Cannot") {
            "  Cannot read folder"
        } else {
            "  This folder is empty"
        };
        frame.render_widget(
            Paragraph::new(locale.tr(message).into_owned()).style(text_style(theme.muted)),
            body,
        );
    }
    let hint = if tree.status.contains("limit reached") {
        " Entry cap reached · open subfolder".into()
    } else if tree.busy() {
        " Loading…".into()
    } else {
        if locale == Locale::Vi {
            format!(" {} mục  ·  F5 đọc lại", tree.rows.len())
        } else {
            format!(" {} items  ·  F5 refresh", tree.rows.len())
        }
    };
    frame.render_widget(
        Paragraph::new(clipped(&locale.tr(&hint), inner.width as usize))
            .style(text_style(theme.faint)),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}

fn file_icon(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
    {
        "rs" => "rs",
        "ts" | "tsx" => "ts",
        "js" | "jsx" => "js",
        "py" => "py",
        "json" => "{}",
        "toml" | "yml" | "yaml" => "≡",
        "md" => "#",
        _ => "·",
    }
}

fn tab_label(tab: &crate::app::Tab, welcome: bool, locale: Locale) -> String {
    let name = if welcome {
        locale.tr("Welcome").into_owned()
    } else {
        let title = tab.title(locale);
        if tab.review_source.is_some() {
            format!(
                "Review · {}",
                title.strip_suffix(" [Review]").unwrap_or(&title)
            )
        } else {
            title
        }
    };
    format!(
        " {}{} {} ",
        if tab.ai_action.is_some() {
            "AI · "
        } else {
            ""
        },
        clipped(&name, 25),
        if tab.document.is_dirty() { "●" } else { " " },
    )
}

fn tabs(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.panel)),
        area,
    );
    let label_row = if area.height >= 4 { 2 } else { 1 };
    app.hits.tab_strip = Rect::new(
        area.x,
        area.y + label_row,
        area.width,
        u16::from(area.height > label_row),
    );
    let locale = app.config.language;
    let count = if area.width < 40 {
        format!("{}/{}", app.active + 1, app.tabs.len())
    } else {
        format!(
            "{} {}/{}",
            locale.tr("Tabs"),
            app.active + 1,
            app.tabs.len()
        )
    };
    let count_width = Line::from(count.as_str()).width().min(area.width as usize) as u16;
    frame.render_widget(
        Paragraph::new(count).style(bold(theme.accent)),
        Rect::new(area.x, area.y, count_width, 1),
    );
    let mut x = area.x + count_width + 1;
    let controls: Vec<(&str, &str)> = if area.width >= 56 {
        vec![
            (" + New ", "new"),
            (" Save ", "save"),
            (" Close ", "close"),
            (" < ", "prev"),
            (" > ", "next"),
            (" All ^P ", "tabs"),
        ]
    } else {
        vec![
            (" + ", "new"),
            (" < ", "prev"),
            (" > ", "next"),
            (" =^P ", "tabs"),
        ]
    };
    for (label, action) in controls {
        let label = locale.tr(label);
        let width = Line::from(label.as_ref()).width() as u16;
        if x + width > area.right() {
            break;
        }
        let rect = Rect::new(x, area.y, width, 1);
        frame.render_widget(
            Paragraph::new(label.into_owned()).style(Style::default().bg(theme.raised).fg(
                if action == "new" {
                    theme.accent
                } else if app.tabs[app.active].ai_action.is_some()
                    && matches!(action, "save" | "close")
                {
                    theme.faint
                } else {
                    theme.text
                },
            )),
            rect,
        );
        app.hits.actions.push((rect, action));
        x += width + 1;
    }
    let (dirty, ai) = app.tabs.iter().filter(|t| t.review_source.is_none()).fold(
        (0usize, 0usize),
        |(dirty, ai), tab| {
            (
                dirty + usize::from(tab.document.is_dirty()),
                ai + usize::from(tab.ai_action.is_some()),
            )
        },
    );
    if (dirty > 0 || ai > 0) && area.right().saturating_sub(x) > 8 {
        let hint = if ai > 0 {
            format!("AI {ai} · ● {dirty} {}", locale.tr("unsaved"))
        } else {
            format!("● {dirty} {}", locale.tr("unsaved"))
        };
        frame.render_widget(
            Paragraph::new(clipped(&hint, (area.right() - x) as usize))
                .style(text_style(theme.warm)),
            Rect::new(x, area.y, area.right() - x, 1),
        );
    }
    if area.height < 2 {
        return;
    }
    if label_row == 2 {
        frame.render_widget(
            Block::default().style(Style::default().bg(theme.canvas)),
            Rect::new(area.x, area.y + 1, area.width, 1),
        );
    }
    let welcome = app.welcome();
    let label = |index: usize| {
        format!(
            "{}{}",
            index + 1,
            tab_label(&app.tabs[index], welcome, locale)
        )
    };
    let mut first = app.active;
    let mut used = Line::from(label(first)).width() + 3;
    while first > 0 && used + Line::from(label(first - 1)).width() + 3 <= area.width as usize {
        first -= 1;
        used += Line::from(label(first)).width() + 3;
    }
    let mut x = area.x;
    for index in first..app.tabs.len() {
        let label = label(index);
        let width = ((Line::from(label.as_str()).width() + 3) as u16).min(area.right() - x);
        if width < 5 {
            break;
        }
        let selected = index == app.active;
        let rect = Rect::new(x, area.y + label_row, width, 1);
        frame.render_widget(
            Paragraph::new(clipped(&label, width.saturating_sub(3) as usize)).style(
                Style::default()
                    .bg(if selected { theme.raised } else { theme.panel })
                    .fg(if app.tabs[index].ai_action.is_some() {
                        theme.accent
                    } else if selected {
                        theme.text
                    } else {
                        theme.muted
                    }),
            ),
            rect,
        );
        app.hits.tabs.push((rect, index));
        let locked = app.tabs[index].ai_action.is_some();
        let close = Rect::new(rect.right() - 3, rect.y, 3, 1);
        frame.render_widget(
            Paragraph::new(if locked { " · " } else { " x " }).style(
                Style::default()
                    .bg(if selected { theme.raised } else { theme.panel })
                    .fg(if locked { theme.faint } else { theme.warm }),
            ),
            close,
        );
        if !locked {
            app.hits.tab_closes.push((close, index));
        }
        if area.height > label_row + 1 {
            frame.render_widget(
                Paragraph::new("─".repeat(width as usize)).style(text_style(if selected {
                    theme.accent
                } else {
                    theme.line
                })),
                Rect::new(x, area.y + label_row + 1, width, 1),
            );
        }
        x += width;
        if x >= area.right() {
            break;
        }
    }
}

fn welcome(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let locale = app.config.language;
    if area.height < 10 || area.width < 34 {
        frame.render_widget(
            Paragraph::new(
                locale.tr("GUINEANOTE\n\nCtrl+O  Open file\nCtrl+K  Open folder\nCtrl+N  New document").into_owned(),
            )
            .style(text_style(theme.accent))
            .alignment(Alignment::Center),
            area,
        );
        return;
    }
    let width = area.width.min(57);
    let x = area.x + (area.width - width) / 2;
    let large = area.height >= 17 && area.width >= 42;
    let logo = if large {
        vec![
            Line::styled(r"  ____ _   _ ___ _   _ _____    _    ", bold(theme.accent)),
            Line::styled(r" / ___| | | |_ _| \ | | ____|  / \   ", bold(theme.accent)),
            Line::styled(r"| |  _| | | || ||  \| |  _|   / _ \  ", bold(theme.accent)),
            Line::styled(r"| |_| | |_| || || |\  | |___ / ___ \ ", bold(theme.accent)),
            Line::styled(r" \____|\___/|___|_| \_|_____/_/   \_\", bold(theme.accent)),
            Line::styled("N O T E", bold(theme.text)),
        ]
    } else {
        vec![
            Line::styled("G U I N E A N O T E", bold(theme.accent)),
            Line::styled(
                "T E R M I N A L   W O R K S P A C E",
                text_style(theme.muted),
            ),
        ]
    };
    let logo_height = logo.len() as u16;
    let top = area.y + area.height.saturating_sub(logo_height + 11) / 2;
    frame.render_widget(
        Paragraph::new(logo).alignment(Alignment::Center),
        Rect::new(x, top, width, logo_height),
    );
    frame.render_widget(
        Paragraph::new(
            locale
                .tr("Your files. Your flow. Your terminal.")
                .into_owned(),
        )
        .alignment(Alignment::Center)
        .style(text_style(theme.muted)),
        Rect::new(x, top + logo_height + 1, width, 1),
    );
    for (index, (label, shortcut, action)) in [
        ("Open a file", "Ctrl+O", "open"),
        ("Open a folder", "Ctrl+K", "open-folder"),
        ("Start a new document", "Ctrl+N", "new"),
    ]
    .into_iter()
    .enumerate()
    {
        let row = Rect::new(
            x + 2,
            top + logo_height + 3 + index as u16 * 2,
            width.saturating_sub(4),
            1,
        );
        if row.y >= area.bottom() {
            break;
        }
        let line = Line::from(vec![
            Span::styled(
                format!("  {:<25}", locale.tr(label)),
                text_style(theme.text),
            ),
            Span::styled(format!("{shortcut:>9}  ›"), text_style(theme.accent)),
        ]);
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(theme.raised)),
            row,
        );
        app.hits.actions.push((row, action));
    }
    if top + logo_height + 10 < area.bottom() {
        frame.render_widget(
            Paragraph::new(locale.tr("F2 Commands     F1 Keyboard guide").into_owned())
                .alignment(Alignment::Center)
                .style(text_style(theme.faint)),
            Rect::new(x, top + logo_height + 10, width, 1),
        );
    }
}

fn editor(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let locale = app.config.language;
    let compact = area.height < 10;
    let regions = Layout::vertical([
        Constraint::Length(if compact { 1 } else { 4 }),
        Constraint::Length(if compact { 0 } else { 1 }),
        Constraint::Min(1),
    ])
    .split(area);
    tabs(frame, app, regions[0], theme);
    let path = app
        .document()
        .path
        .as_ref()
        .map(|path| {
            path.strip_prefix(&app.explorer.root)
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| {
            if app.welcome() {
                locale
                    .tr("Welcome / your next idea starts here")
                    .into_owned()
            } else {
                app.tabs[app.active].title(locale)
            }
        });
    let tab = &app.tabs[app.active];
    let context = if let Some(action) = &tab.ai_action {
        format!(" AI · {} ", locale.ai_action(action))
    } else {
        let mode = match tab.review.mode {
            ViewMode::Source => "SOURCE",
            ViewMode::Split => "SPLIT",
            ViewMode::Review => "REVIEW",
        };
        format!(
            " {} · {} · {} ",
            locale.tr(mode),
            crate::app::document_language(&tab.document).label(),
            crate::file_info::bytes(tab.document.buffer().len_bytes())
        )
    };
    let context_width = if regions[1].width >= 60 && !app.welcome() {
        (Line::from(context.as_str()).width() as u16).min(regions[1].width / 2)
    } else {
        0
    };
    frame.render_widget(
        Paragraph::new(format!(
            "  {}",
            input_tail(
                &path,
                regions[1].width.saturating_sub(context_width + 3) as usize
            )
        ))
        .style(text_style(theme.muted)),
        regions[1],
    );
    if context_width > 0 {
        frame.render_widget(
            Paragraph::new(clipped(&context, context_width as usize)).style(
                Style::default()
                    .bg(theme.raised)
                    .fg(if tab.ai_action.is_some() {
                        theme.warm
                    } else {
                        theme.secondary
                    }),
            ),
            Rect::new(
                regions[1].right() - context_width,
                regions[1].y,
                context_width,
                regions[1].height,
            ),
        );
    }
    let mut editor_area = regions[2];
    let mode = app.tabs[app.active].review.mode;
    if mode != ViewMode::Source && !app.welcome() {
        if mode == ViewMode::Split && editor_area.width >= 76 {
            let halves =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(editor_area);
            editor_area = halves[0];
            review_panel(frame, app, halves[1], theme);
        } else {
            review_panel(frame, app, editor_area, theme);
            return;
        }
    }
    let block = panel(
        &locale.tr(if app.welcome() {
            "START HERE"
        } else {
            "EDITOR"
        }),
        theme,
        app.focus == Focus::Editor,
    );
    let body = block.inner(editor_area);
    frame.render_widget(block, editor_area);
    app.hits.editor = body;
    app.page_height = body.height.max(1) as usize;
    if body.width < 2 || body.height == 0 {
        return;
    }
    if app.welcome() {
        welcome(frame, app, body, theme);
        return;
    }
    let busy = app.busy();
    let tab = &mut app.tabs[app.active];
    let document = &tab.document;
    let language = crate::app::document_language(document);
    let (cursor_line, cursor_cell) = {
        let _span = crate::trace::span("cursor_layout");
        (
            document.position().0,
            document.cursor_cell(app.config.tab_width),
        )
    };
    let gutter = if body.width > 14 {
        document.buffer().len_lines().to_string().len().min(6) + 3
    } else {
        0
    };
    let width = body.width as usize - gutter;
    app.hits.editor_text = Rect::new(body.x + gutter as u16, body.y, width as u16, body.height);
    if tab.follow_cursor && cursor_line < tab.top {
        tab.top = cursor_line;
    }
    if tab.follow_cursor && cursor_line >= tab.top + body.height as usize {
        tab.top = cursor_line + 1 - body.height as usize;
    }
    if tab.follow_cursor && cursor_cell < tab.left {
        tab.left = cursor_cell;
    }
    if tab.follow_cursor && cursor_cell >= tab.left + width {
        tab.left = cursor_cell + 1 - width;
    }
    for offset in 0..body.height {
        let line_idx = tab.top + offset as usize;
        let _line_span = crate::trace::span("line_layout_paint");
        let Ok(window) = document.line_window(line_idx, tab.left, width, app.config.tab_width)
        else {
            break;
        };
        let text = window.text.as_str();
        let start = window.char_start;
        let active_line = line_idx == cursor_line;
        if gutter > 0 {
            let label = format!("{:>digits$} │ ", line_idx + 1, digits = gutter - 3);
            frame.render_widget(
                Paragraph::new(label).style(text_style(if active_line {
                    theme.accent
                } else {
                    theme.faint
                })),
                Rect::new(body.x, body.y + offset, gutter as u16, 1),
            );
        }
        let highlights = if app.config.syntax && language != Language::Plain {
            crate::render::source_highlights(document, line_idx, &window, language)
        } else {
            None
        };
        let line = styled_line(
            text,
            start,
            document.selection().range(),
            tab.left,
            width,
            app.config.tab_width,
            LinePaint {
                cell_start: window.cell_start,
                theme,
                highlights: highlights
                    .as_ref()
                    .and_then(|h| h.for_revision(document.revision()))
                    .unwrap_or(&[]),
            },
        );
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(if active_line {
                theme.raised
            } else {
                theme.panel
            })),
            Rect::new(body.x + gutter as u16, body.y + offset, width as u16, 1),
        );
    }
    if app.focus == Focus::Editor
        && app.modal.is_none()
        && !busy
        && (tab.top..tab.top + body.height as usize).contains(&cursor_line)
        && (tab.left..tab.left + width).contains(&cursor_cell)
    {
        frame.set_cursor_position((
            body.x + gutter as u16 + (cursor_cell - tab.left) as u16,
            body.y + (cursor_line - tab.top) as u16,
        ));
    }
}

fn review_panel(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let locale = app.config.language;
    let is_markdown = crate::app::document_language(app.document()) == Language::Markdown;
    let linked = app.tabs[app.active].review_source.is_some();
    let block = panel(&locale.tr("REVIEW · F7"), theme, true);
    let body = block.inner(area);
    frame.render_widget(block, area);
    if body.width < 4 || body.height < 3 {
        return;
    }
    let review = &mut app.tabs[app.active].review;
    let show_outline = is_markdown && review.outline_visible && body.width >= 92;
    let outline_width = if show_outline { 25 } else { 0 };
    let content = Rect::new(
        body.x + 1,
        body.y + if is_markdown { 4 } else { 2 },
        body.width.saturating_sub(2 + outline_width),
        body.height.saturating_sub(if is_markdown { 4 } else { 2 }),
    );
    app.hits.review = content;
    app.page_height = content.height.max(1) as usize;
    let width = content.width.max(1) as usize;
    if review.width != width {
        review.width = width;
        app.redraw = true;
    }
    let hint = if !review.ready() {
        "Rendering review… source remains editable in split view"
    } else if review.error() {
        "Invalid data · fix the source to refresh review"
    } else if review.busy() {
        "Updating review…"
    } else {
        "↑↓ Scroll · F7 View · Esc Source"
    };
    if is_markdown {
        let reader_title = if locale == Locale::Vi {
            "ĐỌC MARKDOWN"
        } else {
            "MARKDOWN READER"
        };
        let live = if linked {
            "  LIVE · READ ONLY"
        } else {
            "  SPLIT"
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(reader_title, bold(theme.accent)),
                Span::styled(live, text_style(theme.muted)),
            ])),
            Rect::new(body.x + 1, body.y, body.width.saturating_sub(2), 1),
        );
        if let Some(stats) = review.stats() {
            let text = if locale == Locale::Vi {
                format!(
                    "{} từ · ~{} phút · {} mục · Code {} · Bảng {} · Việc {}/{}",
                    stats.words,
                    stats.words.div_ceil(200).max(1),
                    stats.headings,
                    stats.code_blocks,
                    stats.tables,
                    stats.completed_tasks,
                    stats.tasks
                )
            } else {
                format!(
                    "{} words · ~{} min · {} sections · Code {} · Tables {} · Tasks {}/{}",
                    stats.words,
                    stats.words.div_ceil(200).max(1),
                    stats.headings,
                    stats.code_blocks,
                    stats.tables,
                    stats.completed_tasks,
                    stats.tasks
                )
            };
            frame.render_widget(
                Paragraph::new(clipped(&text, body.width.saturating_sub(2) as usize))
                    .style(text_style(theme.secondary)),
                Rect::new(body.x + 1, body.y + 1, body.width.saturating_sub(2), 1),
            );
        }
        let total = review.lines().len().max(1);
        let percent = (review.top + content.height as usize).min(total) * 100 / total;
        let navigation = if locale == Locale::Vi {
            format!("{percent}% · F10 Mục lục · Tab Chuyển vùng · Esc Về nguồn")
        } else {
            format!("{percent}% · F10 Contents · Tab Focus outline · Esc Source")
        };
        frame.render_widget(
            Paragraph::new(if review.busy() || review.error() {
                locale.tr(hint).into_owned()
            } else {
                navigation
            })
            .style(text_style(theme.muted)),
            Rect::new(body.x + 1, body.y + 2, body.width.saturating_sub(2), 1),
        );
    } else {
        frame.render_widget(
            Paragraph::new(locale.tr(hint).into_owned()).style(text_style(if review.error() {
                theme.danger
            } else {
                theme.muted
            })),
            Rect::new(body.x + 1, body.y, body.width.saturating_sub(2), 1),
        );
    }
    let height = content.height as usize;
    review.top = review.top.min(review.lines().len().saturating_sub(height));
    for (offset, line) in review
        .lines()
        .iter()
        .skip(review.top)
        .take(height)
        .enumerate()
    {
        frame.render_widget(
            Paragraph::new(line.clone()),
            Rect::new(content.x, content.y + offset as u16, content.width, 1),
        );
    }
    if show_outline {
        let rect = Rect::new(body.right() - 25, content.y, 25, content.height);
        let block = panel(
            if locale == Locale::Vi {
                "MỤC LỤC"
            } else {
                "CONTENTS"
            },
            theme,
            review.outline_focus,
        );
        let inner = block.inner(rect);
        frame.render_widget(block, rect);
        app.hits.outline = inner;
        let top = review
            .outline_selected
            .saturating_sub(inner.height as usize / 2)
            .min(review.outline().len().saturating_sub(inner.height as usize));
        for (offset, entry) in review
            .outline()
            .iter()
            .skip(top)
            .take(inner.height as usize)
            .enumerate()
        {
            let selected = top + offset == review.outline_selected;
            let label = format!(
                "{}{}{}",
                if selected { "▌" } else { " " },
                " ".repeat(entry.level.saturating_sub(1).min(5)),
                entry.title
            );
            frame.render_widget(
                Paragraph::new(clipped(&label, inner.width as usize)).style(
                    Style::default()
                        .fg(if selected { theme.accent } else { theme.muted })
                        .bg(if selected { theme.raised } else { theme.panel }),
                ),
                Rect::new(inner.x, inner.y + offset as u16, inner.width, 1),
            );
        }
    }
}

fn status(frame: &mut Frame<'_>, app: &mut App, areas: &[Rect], theme: Theme) {
    let locale = app.config.language;
    let message = if app.focus == Focus::Explorer
        && (app.explorer.status.starts_with("Cannot")
            || app.explorer.status.contains("limit reached"))
    {
        app.explorer.status.as_str()
    } else {
        app.message.as_str()
    };
    let error = app.recovery.failed
        || message.contains("Cannot")
        || message.contains("denied")
        || message.contains("outside")
        || message.contains("unsupported");
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                if app.busy() { " ◌ " } else { " ● " },
                text_style(if error { theme.danger } else { theme.accent }),
            ),
            Span::styled(
                if app.recovery.failed {
                    format!("{} · ", locale.tr("Recovery warning"))
                } else {
                    String::new()
                },
                text_style(theme.danger),
            ),
            Span::styled(safe(&locale.message(message)), text_style(theme.muted)),
        ])),
        areas[0],
    );
    let document = app.document();
    let (line, column) = document.position();
    let mode = if app.tabs[app.active].ai_action.is_some() {
        " AI · READ ONLY "
    } else if app.tabs[app.active].review_source.is_some() {
        " REVIEW "
    } else if document.readonly {
        " READ ONLY "
    } else if app.tabs[app.active].review.mode == ViewMode::Review {
        " REVIEW "
    } else if app.focus == Focus::Explorer {
        " FILES "
    } else {
        " EDIT "
    };
    let language = crate::app::document_language(document);
    let badge = Span::styled(
        locale.tr(mode).into_owned(),
        Style::default()
            .bg(if document.readonly {
                theme.warm
            } else {
                theme.accent
            })
            .fg(theme.canvas)
            .add_modifier(Modifier::BOLD),
    );
    let line = Line::from(vec![
        badge,
        Span::styled(
            if locale == Locale::Vi {
                format!("  Dòng {}, Cột {} ", line + 1, column + 1)
            } else {
                format!("  Ln {}, Col {} ", line + 1, column + 1)
            },
            text_style(theme.text),
        ),
        Span::styled(
            locale
                .tr(if document.is_dirty() {
                    "  ● MODIFIED"
                } else if document.path.is_none() {
                    "  New buffer"
                } else {
                    "  ● Saved"
                })
                .into_owned(),
            text_style(if document.is_dirty() {
                theme.warm
            } else {
                theme.accent
            }),
        ),
    ]);
    let full_format = format!(
        " {} · UTF-8{} · {} ",
        language.label(),
        if document.bom { "+BOM" } else { "" },
        document.line_ending().label()
    );
    let left_width = line.width();
    let available = (areas[1].width as usize).saturating_sub(left_width + 1);
    let format = if available >= Line::from(full_format.as_str()).width() {
        full_format
    } else {
        format!(" {} ", language.label())
    };
    let mut right = Line::from(vec![Span::styled(format, text_style(theme.secondary))]);
    if areas[1].width >= 100 {
        let selected = document.selection().range().len();
        let progress = document.cursor().saturating_mul(100) / document.buffer().len_chars().max(1);
        let detail = if selected > 0 {
            format!(
                "{} {selected} · {progress}% │",
                if locale == Locale::Vi {
                    "CHỌN"
                } else {
                    "SEL"
                }
            )
        } else {
            format!("{progress}% │")
        };
        if left_width + right.width() + Line::from(detail.as_str()).width() + 2
            <= areas[1].width as usize
        {
            right.spans.insert(
                0,
                Span::styled(
                    detail,
                    text_style(if selected > 0 {
                        theme.warm
                    } else {
                        theme.faint
                    }),
                ),
            );
        }
    }
    let right_width = (right.width() as u16).min(areas[1].width / 2);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme.raised)),
        Rect::new(areas[1].x, areas[1].y, areas[1].width - right_width, 1),
    );
    frame.render_widget(
        Paragraph::new(right)
            .alignment(Alignment::Right)
            .style(Style::default().bg(theme.raised)),
        Rect::new(areas[1].right() - right_width, areas[1].y, right_width, 1),
    );
    let hints = if app.tabs[app.active].ai_action.is_some() {
        " ↑↓ Navigate   ^C Copy   F6 Other tabs   ^T New tab   F8 Settings"
    } else if app.focus == Focus::Explorer {
        " ↑↓ Browse   → Expand   ← Collapse   Enter Open   Tab Editor   ^B Hide files"
    } else {
        " ^S Save   ^O Open   ^K Folder   ^B Files   F4 Pane   F6 Tabs   F7 Review   F2 Commands   ^Q Quit"
    };
    frame.render_widget(
        Paragraph::new(locale.tr(hints).into_owned()).style(text_style(theme.muted)),
        areas[2],
    );
}

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    let locale = app.config.language;
    let theme = Theme::new(app.config.basic_colors);
    app.hits = HitRegions::default();
    frame.render_widget(Block::default().style(theme.base()), area);
    if area.width < 30 || area.height < 9 {
        app.focus = Focus::Editor;
        frame.render_widget(
            Paragraph::new(
                locale
                    .tr("GUINEANOTE\nResize to at least 30 × 9\nCtrl+Q to quit")
                    .into_owned(),
            )
            .style(text_style(theme.accent)),
            area,
        );
        return;
    }
    let compact = area.height < 14;
    let areas = Layout::vertical([
        Constraint::Length(if compact { 1 } else { 2 }),
        Constraint::Length(if compact { 0 } else { 1 }),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);
    header(frame, app, areas[0], theme);
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(text_style(theme.line)),
        areas[1],
    );
    let workspace = Rect::new(
        areas[2].x + 1,
        areas[2].y,
        areas[2].width.saturating_sub(2),
        areas[2].height,
    );
    if app.explorer_visible && workspace.width >= 64 {
        let sidebar = app.config.sidebar_width.min(workspace.width / 3).max(22);
        let split = Layout::horizontal([
            Constraint::Length(sidebar),
            Constraint::Length(1),
            Constraint::Min(20),
        ])
        .split(workspace);
        explorer(frame, app, split[0], theme);
        editor(frame, app, split[2], theme);
    } else {
        if app.focus == Focus::Explorer {
            app.focus = Focus::Editor;
            app.message = "Explorer needs a wider terminal; Ctrl+O still opens files".into();
        }
        editor(frame, app, workspace, theme);
    }
    status(frame, app, &areas[3..], theme);
    crate::dialogs::draw(frame, app, area, theme);
}
