//! Workspace surfaces and interaction geometry. All text edits stay in core.
use guinea_core::{
    TextBuffer,
    coordinates::{CharIndex, cell_column},
};
use guinea_syntax::{BasicSyntax, Language, SyntaxProvider};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    app::{App, Focus, HELP, HitRegions, InputKind, Modal},
    render::{LinePaint, prefix_through_cell, safe, styled_line},
    theme::Theme,
};

fn text_style(color: ratatui::style::Color) -> Style {
    Style::default().fg(color)
}
fn bold(color: ratatui::style::Color) -> Style {
    text_style(color).add_modifier(Modifier::BOLD)
}

fn clipped(text: &str, width: usize) -> String {
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

fn input_tail(text: &str, width: usize) -> String {
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

fn panel(title: &str, theme: Theme, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(text_style(if focused { theme.mint } else { theme.line }))
        .style(Style::default().bg(theme.panel).fg(theme.text))
        .title(Line::styled(
            format!(" {title} "),
            bold(if focused { theme.mint } else { theme.muted }),
        ))
}

fn header(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let brand = Line::from(vec![
        Span::styled(" G/ ", bold(theme.mint)),
        Span::styled("GUINEA", bold(theme.text)),
        Span::styled("NOTE", bold(theme.mint)),
        Span::styled("    A little more focused.", text_style(theme.muted)),
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
            Paragraph::new("WORKSPACE  /  v0.1.0 ")
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
    ] {
        let width = label.len() as u16;
        if x + width > area.right() {
            break;
        }
        let rect = Rect::new(x, area.y + 1, width, 1);
        frame.render_widget(
            Paragraph::new(label).style(Style::default().bg(theme.raised).fg(theme.muted)),
            rect,
        );
        app.hits.actions.push((rect, command));
        x += width + 1;
    }
    if area.width > 92 {
        frame.render_widget(
            Paragraph::new("F1 Help   F2 Commands ")
                .style(text_style(theme.blue))
                .alignment(Alignment::Right),
            Rect::new(area.right() - 25, area.y + 1, 25, 1),
        );
    }
}

fn explorer(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let focused = app.focus == Focus::Explorer;
    let block = panel("EXPLORER", theme, focused);
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
                text_style(if focused { theme.mint } else { theme.blue }),
            ),
            Span::styled(guides, text_style(theme.faint)),
            Span::styled(
                format!("{icon} "),
                text_style(if row.entry.directory {
                    theme.mint
                } else {
                    theme.blue
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
            spans.push(Span::styled(" ●", text_style(theme.mint)));
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
        frame.render_widget(Paragraph::new(message).style(text_style(theme.muted)), body);
    }
    let hint = if tree.status.contains("limit reached") {
        " Entry cap reached · open subfolder".into()
    } else if tree.busy() {
        " Loading…".into()
    } else {
        format!(" {} items  ·  F5 refresh", tree.rows.len())
    };
    frame.render_widget(
        Paragraph::new(clipped(&hint, inner.width as usize)).style(text_style(theme.faint)),
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

fn tab_label(tab: &crate::app::Tab, welcome: bool) -> String {
    let name = tab
        .document
        .path
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            if welcome {
                "Welcome".into()
            } else {
                "Untitled".into()
            }
        });
    format!(
        "  {} {}  ",
        clipped(&name, 21),
        if tab.document.is_dirty() { "●" } else { " " }
    )
}

fn tabs(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.panel)),
        area,
    );
    let labels = app
        .tabs
        .iter()
        .map(|tab| tab_label(tab, app.welcome()))
        .collect::<Vec<_>>();
    let mut first = app.active;
    let mut used = Line::from(labels[first].as_str()).width();
    while first > 0 && used + Line::from(labels[first - 1].as_str()).width() <= area.width as usize
    {
        first -= 1;
        used += Line::from(labels[first].as_str()).width();
    }
    let mut x = area.x;
    for (index, label) in labels.into_iter().enumerate().skip(first) {
        let width = (Line::from(label.as_str()).width() as u16).min(area.right() - x);
        if width < 5 {
            break;
        }
        let selected = index == app.active;
        let rect = Rect::new(x, area.y, width, 1);
        frame.render_widget(
            Paragraph::new(label).style(
                Style::default()
                    .bg(if selected { theme.raised } else { theme.panel })
                    .fg(if selected { theme.text } else { theme.muted }),
            ),
            rect,
        );
        app.hits.tabs.push((rect, index));
        if area.height > 1 {
            frame.render_widget(
                Paragraph::new("─".repeat(width as usize)).style(text_style(if selected {
                    theme.mint
                } else {
                    theme.line
                })),
                Rect::new(x, area.y + 1, width, 1),
            );
        }
        x += width;
        if x >= area.right() {
            break;
        }
    }
}

fn welcome(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    if area.height < 10 || area.width < 34 {
        frame.render_widget(
            Paragraph::new(
                "GUINEANOTE\n\nCtrl+O  Open file\nCtrl+K  Open folder\nCtrl+N  New document",
            )
            .style(text_style(theme.mint))
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
            Line::styled(r"  ____ _   _ ___ _   _ _____    _    ", bold(theme.mint)),
            Line::styled(r" / ___| | | |_ _| \ | | ____|  / \   ", bold(theme.mint)),
            Line::styled(r"| |  _| | | || ||  \| |  _|   / _ \  ", bold(theme.mint)),
            Line::styled(r"| |_| | |_| || || |\  | |___ / ___ \ ", bold(theme.mint)),
            Line::styled(r" \____|\___/|___|_| \_|_____/_/   \_\", bold(theme.mint)),
            Line::styled("N O T E", bold(theme.text)),
        ]
    } else {
        vec![
            Line::styled("G U I N E A N O T E", bold(theme.mint)),
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
        Paragraph::new("Your files. Your flow. Your terminal.")
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
            Span::styled(format!("  {label:<25}"), text_style(theme.text)),
            Span::styled(format!("{shortcut:>9}  ›"), text_style(theme.mint)),
        ]);
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(theme.raised)),
            row,
        );
        app.hits.actions.push((row, action));
    }
    if top + logo_height + 10 < area.bottom() {
        frame.render_widget(
            Paragraph::new("F2 Commands     F1 Keyboard guide")
                .alignment(Alignment::Center)
                .style(text_style(theme.faint)),
            Rect::new(x, top + logo_height + 10, width, 1),
        );
    }
}

fn editor(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let compact = area.height < 9;
    let regions = Layout::vertical([
        Constraint::Length(if compact { 1 } else { 2 }),
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
        .unwrap_or_else(|| "Welcome / your next idea starts here".into());
    frame.render_widget(
        Paragraph::new(format!(
            "  {}",
            clipped(&path, regions[1].width.saturating_sub(3) as usize)
        ))
        .style(text_style(theme.muted)),
        regions[1],
    );
    let block = panel(
        if app.welcome() {
            "START HERE"
        } else {
            "EDITOR"
        },
        theme,
        app.focus == Focus::Editor,
    );
    let body = block.inner(regions[2]);
    frame.render_widget(block, regions[2]);
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
    let language = Language::detect(document.path.as_deref());
    let (cursor_line, _) = document.position();
    let start = document
        .buffer()
        .line_to_char(cursor_line)
        .expect("cursor line");
    let raw = document.buffer().line(cursor_line).expect("cursor line");
    let cursor_cell = cell_column(
        &raw,
        CharIndex(document.cursor() - start),
        app.config.tab_width,
    )
    .expect("grapheme cursor")
    .0;
    let gutter = if body.width > 14 {
        document.buffer().len_lines().to_string().len().min(6) + 3
    } else {
        0
    };
    let width = body.width as usize - gutter;
    app.hits.editor_text = Rect::new(body.x + gutter as u16, body.y, width as u16, body.height);
    if cursor_line < tab.top {
        tab.top = cursor_line;
    }
    if cursor_line >= tab.top + body.height as usize {
        tab.top = cursor_line + 1 - body.height as usize;
    }
    if cursor_cell < tab.left {
        tab.left = cursor_cell;
    }
    if cursor_cell >= tab.left + width {
        tab.left = cursor_cell + 1 - width;
    }
    for offset in 0..body.height {
        let line_idx = tab.top + offset as usize;
        let Some(raw) = document.buffer().line(line_idx) else {
            break;
        };
        let text = raw
            .strip_suffix("\r\n")
            .or_else(|| raw.strip_suffix('\n'))
            .unwrap_or(&raw);
        let start = document
            .buffer()
            .line_to_char(line_idx)
            .expect("visible line");
        let active_line = line_idx == cursor_line;
        if gutter > 0 {
            let label = format!("{:>digits$} │ ", line_idx + 1, digits = gutter - 3);
            frame.render_widget(
                Paragraph::new(label).style(text_style(if active_line {
                    theme.mint
                } else {
                    theme.faint
                })),
                Rect::new(body.x, body.y + offset, gutter as u16, 1),
            );
        }
        let highlights = if app.config.syntax && language != Language::Plain {
            Some(BasicSyntax.highlight_line(
                language,
                prefix_through_cell(text, tab.left + width, app.config.tab_width),
                document.revision(),
            ))
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
    if app.focus == Focus::Editor && app.modal.is_none() && !busy {
        frame.set_cursor_position((
            body.x + gutter as u16 + (cursor_cell - tab.left) as u16,
            body.y + (cursor_line - tab.top) as u16,
        ));
    }
}

fn status(frame: &mut Frame<'_>, app: &mut App, areas: &[Rect], theme: Theme) {
    let message = if app.focus == Focus::Explorer
        && (app.explorer.status.starts_with("Cannot")
            || app.explorer.status.contains("limit reached"))
    {
        app.explorer.status.as_str()
    } else {
        app.message.as_str()
    };
    let error = message.contains("Cannot")
        || message.contains("denied")
        || message.contains("outside")
        || message.contains("unsupported");
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                if app.busy() { " ◌ " } else { " ● " },
                text_style(if error { theme.red } else { theme.mint }),
            ),
            Span::styled(safe(message), text_style(theme.muted)),
        ])),
        areas[0],
    );
    let document = app.document();
    let (line, column) = document.position();
    let mode = if document.readonly {
        " READ ONLY "
    } else if app.focus == Focus::Explorer {
        " FILES "
    } else {
        " EDIT "
    };
    let language = Language::detect(document.path.as_deref());
    let badge = Span::styled(
        mode,
        Style::default()
            .bg(if document.readonly {
                theme.amber
            } else {
                theme.mint
            })
            .fg(theme.canvas)
            .add_modifier(Modifier::BOLD),
    );
    let line = Line::from(vec![
        badge,
        Span::styled(
            format!("  Ln {}, Col {} ", line + 1, column + 1),
            text_style(theme.text),
        ),
        Span::styled(
            format!(
                " │ {}  UTF-8{}  {}",
                language.label(),
                if document.bom { "+BOM" } else { "" },
                document.line_ending().label()
            ),
            text_style(theme.muted),
        ),
        Span::styled(
            if document.is_dirty() {
                "  ● MODIFIED"
            } else if document.path.is_none() {
                "  New buffer"
            } else {
                "  ● Saved"
            },
            text_style(if document.is_dirty() {
                theme.amber
            } else {
                theme.mint
            }),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme.raised)),
        areas[1],
    );
    let hints = if app.focus == Focus::Explorer {
        " ↑↓ Browse   → Expand   ← Collapse   Enter Open   Tab Editor   ^B Hide files"
    } else {
        " ^S Save   ^O Open   ^K Folder   ^B Files   F4 Pane   F6 Tabs   F2 Commands   ^Q Quit"
    };
    frame.render_widget(
        Paragraph::new(hints).style(text_style(theme.muted)),
        areas[2],
    );
}

fn modal(frame: &mut Frame<'_>, modal: &Modal, area: Rect, theme: Theme) {
    let (title, height, border) = match modal {
        Modal::Help => ("KEYBOARD GUIDE", 31, theme.blue),
        Modal::Confirm { .. } => ("UNSAVED CHANGES / CONFIRM", 10, theme.amber),
        Modal::Input {
            kind: InputKind::Command,
            ..
        } => ("COMMANDS", 15, theme.mint),
        Modal::Input { .. } => ("OPEN / SEARCH / SAVE", 10, theme.mint),
    };
    let width = area.width.saturating_sub(4).min(84);
    let height = area.height.saturating_sub(2).min(height);
    let rect = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let shadow = Rect::new(
        (rect.x + 1).min(area.right()),
        (rect.y + 1).min(area.bottom()),
        rect.width.min(area.right().saturating_sub(rect.x + 1)),
        rect.height.min(area.bottom().saturating_sub(rect.y + 1)),
    );
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.canvas)),
        shadow,
    );
    frame.render_widget(Clear, rect);
    let block = panel(title, theme, true).border_style(text_style(border));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let body = Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(2),
        inner.height,
    );
    let content = match modal {
        Modal::Help => HELP.into(),
        Modal::Confirm { message, .. } => format!(
            "\n{}\n\n[Y] Confirm     [N / Esc] Keep working",
            safe(message)
        ),
        Modal::Input { kind, value } => {
            let suggestions = if matches!(kind, InputKind::Command) {
                "\n\nnew  open  open-folder  save  save-as  close\nexplorer  focus  refresh  find  replace  quit\n\nType a command · Enter runs · Esc closes"
            } else {
                "\n\nEnter accepts     Esc cancels"
            };
            format!(
                "{}\n\n  › {}│{suggestions}",
                kind.label(),
                input_tail(value, body.width.saturating_sub(5) as usize)
            )
        }
    };
    frame.render_widget(
        Paragraph::new(content)
            .style(text_style(theme.text))
            .wrap(Wrap { trim: false }),
        body,
    );
}

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    let theme = Theme::new(app.config.basic_colors);
    app.hits = HitRegions::default();
    frame.render_widget(Block::default().style(theme.base()), area);
    if area.width < 30 || area.height < 9 {
        app.focus = Focus::Editor;
        frame.render_widget(
            Paragraph::new("GUINEANOTE\nResize to at least 30 × 9\nCtrl+Q to quit")
                .style(text_style(theme.mint)),
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
    if let Some(value) = &app.modal {
        modal(frame, value, area, theme);
    }
}
