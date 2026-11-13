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
    app::{App, HitRegions},
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

/// Keyboard hints as keycaps: the key bold in `key`, its label in `label`,
/// with the exact text of `commands::hints` (tests and probes read it).
fn hint_line(
    items: &[(&str, &str)],
    locale: Locale,
    key: ratatui::style::Color,
    label: ratatui::style::Color,
) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (k, l)) in crate::commands::hint_parts(items, locale)
        .into_iter()
        .enumerate()
    {
        spans.push(Span::raw(if index == 0 { " " } else { "   " }));
        spans.push(Span::styled(k, bold(key)));
        spans.push(Span::styled(format!(" {l}"), text_style(label)));
    }
    Line::from(spans)
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
    let actions_y = area.y + u16::from(area.height > 1);
    if area.height > 1 {
        let brand = Line::from(vec![
            Span::styled(" G/ ", bold(theme.accent)),
            Span::styled("GUINEA ", bold(theme.text)),
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
        if area.width > 85 {
            frame.render_widget(
                Paragraph::new(locale.tr("WORKSPACE  /  v0.1.0 ").into_owned())
                    .style(text_style(theme.faint))
                    .alignment(Alignment::Right),
                Rect::new(area.x + area.width - 27, area.y, 27, 1),
            );
        }
    }
    // Labelled buttons whenever they fit next to the help hints; keys come
    // from the keymap and are drawn as the accent part of each button.
    const ACTIONS: [(&str, &str, &str, &str); 5] = [
        ("open", "Open file", "Open", "Open"),
        ("find", "Find/Replace", "Find/Replace", "Find/Replace"),
        ("preview", "Review", "View", "View"),
        ("settings", "Settings", "Settings", ""),
        ("file-info", "Info", "Info", ""),
    ];
    let help = hint_line(
        &[("help", "Help"), ("commands", "Commands")],
        locale,
        theme.secondary,
        theme.muted,
    );
    // One cell of margin on the right, matching the left edge.
    let help_width = help.width() as u16 + 2;
    let button = |label: &str, command: &str| {
        let key = crate::commands::short_key(command);
        if label.is_empty() {
            (String::new(), key)
        } else {
            (format!("{} ", locale.tr(label)), key)
        }
    };
    let total = |variant: usize| {
        ACTIONS
            .iter()
            .map(|action| {
                let label = [action.1, action.2, action.3][variant];
                let (text, key) = button(label, action.0);
                Line::from(format!(" {text}{key} ").as_str()).width() as u16 + 1
            })
            .sum::<u16>()
            + 1
    };
    let variant = (0..3)
        .find(|&v| total(v) + help_width <= area.width)
        .unwrap_or(2);
    let mut x = area.x + 1;
    for action in ACTIONS {
        let command = action.0;
        let (text, key) = button([action.1, action.2, action.3][variant], command);
        let line = Line::from(vec![
            Span::raw(" "),
            Span::styled(text, text_style(theme.text)),
            Span::styled(key, bold(theme.accent)),
            Span::raw(" "),
        ]);
        let width = line.width() as u16;
        if x + width > area.right() {
            break;
        }
        let rect = Rect::new(x, actions_y, width, 1);
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(theme.raised)),
            rect,
        );
        app.hits.actions.push((rect, command));
        x += width + 1;
    }
    if area.right().saturating_sub(x) >= help_width {
        frame.render_widget(
            Paragraph::new(help).alignment(Alignment::Right),
            Rect::new(area.right() - help_width, actions_y, help_width - 1, 1),
        );
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

/// One tab bar row: the tabs on the left (each with its own close `x`), the
/// position count and tab controls on the right. The control labels shrink
/// before the tabs lose their minimum space; hidden tabs stay reachable with
/// F12/Ctrl+P, F6/F7 and the mouse wheel over the tabs.
fn tabs(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    if area.is_empty() {
        return;
    }
    let row = Rect::new(area.x, area.y, area.width, 1);
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.panel)),
        row,
    );
    let locale = app.config.language;
    let key = crate::commands::short_key;
    let count = format!("{}/{}", app.active + 1, app.tabs.len());
    let wide: Vec<(String, &str)> = vec![
        (format!(" + {} {} ", locale.tr("New"), key("new")), "new"),
        (format!(" {} {} ", locale.tr("Save"), key("save")), "save"),
        (
            format!(" {} {} ", locale.tr("Close"), key("close")),
            "close",
        ),
        (format!(" < {} ", key("prev")), "prev"),
        (format!(" {} > ", key("next")), "next"),
        (format!(" {} {} ", locale.tr("All"), key("tabs")), "tabs"),
    ];
    let medium: Vec<(String, &str)> = vec![
        (format!(" + {} ", locale.tr("New")), "new"),
        (format!(" {} ", locale.tr("Save")), "save"),
        (format!(" {} ", locale.tr("Close")), "close"),
        (" < ".into(), "prev"),
        (" > ".into(), "next"),
        (format!(" {} {} ", locale.tr("All"), key("tabs")), "tabs"),
    ];
    let narrow: Vec<(String, &str)> = vec![
        (" + ".into(), "new"),
        (" < ".into(), "prev"),
        (" > ".into(), "next"),
        (format!(" {} ", key("tabs")), "tabs"),
    ];
    let labelled = format!(" {} {count}", locale.tr("Tabs"));
    let bare = format!(" {count}");
    let width_of = |count: &str, controls: &[(String, &str)]| {
        Line::from(count).width()
            + controls
                .iter()
                .map(|(label, _)| Line::from(label.as_str()).width() + 1)
                .sum::<usize>()
    };
    let welcome = app.welcome();
    // " 3 main.rs ●" (" 3 AI · main.rs" while AI works), then the close button.
    let label = |index: usize| {
        let label = tab_label(&app.tabs[index], welcome, locale);
        let label = label.trim_end();
        let (ai, name) = label
            .trim_start()
            .strip_prefix("AI · ")
            .map_or(("", label.trim_start()), |name| ("AI · ", name));
        format!(" {} {ai}{name}", index + 1)
    };
    // Each tab is its label, a 3-cell close button and a 1-cell divider. One
    // cell on each side is reserved for the ‹ / › markers that show tabs
    // scrolled out of the strip.
    let span = |index: usize| Line::from(label(index)).width() + 4;
    // Like Notepad++, tabs come first: use the longest control labels that
    // still show every tab; when none does, keep half the row (up to 56 cells).
    let all_tabs = (0..app.tabs.len()).map(span).sum::<usize>() + 3;
    let room = (area.width as usize).saturating_sub((area.width as usize / 2).min(56));
    // The "Tabs n/m" label stays unless even the shortest buttons need its room.
    let variants = [
        (&labelled, wide),
        (&labelled, medium),
        (&labelled, narrow.clone()),
        (&bare, narrow),
    ];
    let choice = variants[..3]
        .iter()
        .position(|(count, controls)| width_of(count, controls) + all_tabs <= area.width as usize)
        .or_else(|| {
            variants
                .iter()
                .position(|(count, controls)| width_of(count, controls) <= room)
        })
        .unwrap_or(3);
    let (count, controls) = variants.into_iter().nth(choice).expect("four variants");
    let cluster = (width_of(count, &controls) as u16).min(area.width);
    let mut x = area.right() - cluster;
    let count_width = Line::from(count.as_str()).width() as u16;
    frame.render_widget(
        Paragraph::new(count.clone()).style(bold(theme.accent)),
        Rect::new(x, area.y, count_width.min(area.right() - x), 1),
    );
    x += count_width;
    for (label, action) in controls {
        let width = Line::from(label.as_str()).width() as u16;
        if x + 1 + width > area.right() {
            break;
        }
        x += 1;
        let rect = Rect::new(x, area.y, width, 1);
        let locked = app.tabs[app.active].ai_action.is_some() && matches!(action, "save" | "close");
        let base =
            Style::default()
                .bg(theme.raised)
                .fg(if locked { theme.faint } else { theme.text });
        // The key inside the label is the accent part, as in the menu.
        let shortcut = key(action);
        let line = match label.rfind(shortcut.as_str()).filter(|_| !locked) {
            Some(at) if action != "tabs" || label.trim() != shortcut => Line::from(vec![
                Span::styled(label[..at].to_string(), base),
                Span::styled(
                    shortcut.clone(),
                    base.fg(theme.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(label[at + shortcut.len()..].to_string(), base),
            ]),
            _ => Line::from(Span::styled(
                label.clone(),
                if action == "new" && !locked {
                    base.fg(theme.accent)
                } else {
                    base
                },
            )),
        };
        frame.render_widget(Paragraph::new(line).style(base), rect);
        app.hits.actions.push((rect, action));
        x += width;
    }
    let strip_width = area.width.saturating_sub(cluster + 1);
    app.hits.tab_strip = Rect::new(area.x, area.y, strip_width, 1);
    if strip_width < 8 {
        return;
    }
    let fits = strip_width.saturating_sub(2) as usize;
    let mut first = app.active;
    let mut used = span(first);
    while first > 0 && used + span(first - 1) <= fits {
        first -= 1;
        used += span(first);
    }
    let right = area.x + strip_width - 1;
    let mut x = area.x + 1;
    if first > 0 {
        frame.render_widget(
            Paragraph::new("‹").style(Style::default().bg(theme.panel).fg(theme.warm)),
            Rect::new(area.x, area.y, 1, 1),
        );
    }
    for index in first..app.tabs.len() {
        let label = label(index);
        let width = ((Line::from(label.as_str()).width() + 3) as u16).min(right - x);
        if width < 6 {
            break;
        }
        let selected = index == app.active;
        let background = if selected { theme.raised } else { theme.panel };
        let rect = Rect::new(x, area.y, width, 1);
        let name_style =
            Style::default()
                .bg(background)
                .fg(if app.tabs[index].ai_action.is_some() {
                    theme.accent
                } else if selected {
                    theme.text
                } else {
                    theme.muted
                });
        frame.render_widget(
            Paragraph::new(clipped(&label, width.saturating_sub(3) as usize)).style(if selected {
                name_style.add_modifier(Modifier::BOLD)
            } else {
                name_style
            }),
            rect,
        );
        if selected && app.tabs[index].ai_action.is_none() {
            // The active tab's number is the accent marker (no extra row).
            let digits = (index + 1).to_string().len() as u16;
            frame.render_widget(
                Paragraph::new((index + 1).to_string()).style(
                    Style::default()
                        .bg(background)
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Rect::new(x + 1, area.y, digits.min(width.saturating_sub(4)), 1),
            );
        }
        app.hits.tabs.push((rect, index));
        let locked = app.tabs[index].ai_action.is_some();
        let close = Rect::new(rect.right() - 3, rect.y, 3, 1);
        frame.render_widget(
            Paragraph::new(if locked { " · " } else { " x " }).style(
                Style::default().bg(background).fg(if selected && !locked {
                    theme.warm
                } else {
                    theme.faint
                }),
            ),
            close,
        );
        if !locked {
            app.hits.tab_closes.push((close, index));
        }
        x += width;
        let last = index + 1 == app.tabs.len();
        if !last && x < right {
            frame.render_widget(
                Paragraph::new("│").style(Style::default().bg(theme.panel).fg(theme.line)),
                Rect::new(x, area.y, 1, 1),
            );
            x += 1;
        }
        if !last && (x >= right || x + 6 > right || x + span(index + 1) as u16 > right) {
            frame.render_widget(
                Paragraph::new("›").style(Style::default().bg(theme.panel).fg(theme.warm)),
                Rect::new(right, area.y, 1, 1),
            );
            break;
        }
    }
}

/// File path and context drawn into the top border of the editor or review
/// panel (after its title), instead of a separate row above it.
fn border_info(frame: &mut Frame<'_>, app: &App, area: Rect, title: &str, theme: Theme) {
    if area.width < 20 || app.welcome() {
        return;
    }
    let locale = app.config.language;
    let tab = &app.tabs[app.active];
    let path = tab
        .document
        .path
        .as_ref()
        .map(|path| {
            let path = crate::paths::display(path);
            let root = crate::paths::display(&app.root);
            path.strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| tab.title(locale));
    let (dirty, ai) = app.tabs.iter().filter(|t| t.review_source.is_none()).fold(
        (0usize, 0usize),
        |(dirty, ai), tab| {
            (
                dirty + usize::from(tab.document.is_dirty()),
                ai + usize::from(tab.ai_action.is_some()),
            )
        },
    );
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
    let unsaved = match (dirty, ai) {
        (0, 0) => String::new(),
        (dirty, 0) => format!(" ● {dirty} {} ", locale.tr("unsaved")),
        (dirty, ai) => format!(" AI {ai} · ● {dirty} {} ", locale.tr("unsaved")),
    };
    // Corner + " TITLE " + "─" on the left, "─" + corner on the right.
    let start = area.x + 2 + Line::from(format!(" {title} ").as_str()).width() as u16;
    let end = area.right().saturating_sub(2);
    let available = end.saturating_sub(start) as usize;
    let context_width = Line::from(context.as_str()).width();
    let unsaved_width = Line::from(unsaved.as_str()).width();
    let (show_context, show_unsaved) = if available >= context_width + unsaved_width + 12 {
        (true, unsaved_width > 0)
    } else if available >= context_width + 12 {
        (true, false)
    } else {
        (false, false)
    };
    // One border cell stays visible between the unsaved count and the chip.
    let right_width = if show_context { context_width } else { 0 }
        + if show_unsaved { unsaved_width } else { 0 }
        + usize::from(show_context && show_unsaved);
    let path_width = available.saturating_sub(right_width + 2);
    if path_width >= 4 {
        let shown = format!(" {} ", input_tail(&path, path_width - 2));
        let width = Line::from(shown.as_str()).width() as u16;
        frame.render_widget(
            Paragraph::new(shown).style(Style::default().bg(theme.panel).fg(theme.muted)),
            Rect::new(start, area.y, width, 1),
        );
    }
    let mut x = end - right_width as u16;
    if show_unsaved {
        frame.render_widget(
            Paragraph::new(unsaved).style(Style::default().bg(theme.panel).fg(theme.warm)),
            Rect::new(x, area.y, unsaved_width as u16, 1),
        );
        x += unsaved_width as u16 + u16::from(show_context);
    }
    if show_context {
        frame.render_widget(
            Paragraph::new(context).style(Style::default().bg(theme.raised).fg(
                if tab.ai_action.is_some() {
                    theme.warm
                } else {
                    theme.secondary
                },
            )),
            Rect::new(x, area.y, context_width as u16, 1),
        );
    }
}

fn welcome(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let locale = app.config.language;
    if area.height < 10 || area.width < 34 {
        frame.render_widget(
            Paragraph::new(
                locale
                    .tr("GUINEA NOTE\n\nCtrl+O  Open file\nCtrl+N  New document")
                    .into_owned(),
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
    for (index, (label, action)) in [
        ("Open a file", "open"),
        ("Type a file path", "open-path"),
        ("Start a new document", "new"),
    ]
    .into_iter()
    .enumerate()
    {
        let shortcut = crate::commands::find(action)
            .and_then(|c| c.keys.first())
            .map_or_else(|| format!("F2 {action}"), |b| b.label.to_string());
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
            Paragraph::new(crate::commands::hints(
                &[("commands", "Commands"), ("help", "Keyboard guide")],
                locale,
            ))
            .alignment(Alignment::Center)
            .style(text_style(theme.faint)),
            Rect::new(x, top + logo_height + 10, width, 1),
        );
    }
}

fn editor(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let locale = app.config.language;
    // One tab bar row; path and file context live in the panel border.
    let regions = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    if app.focus_mode {
        let label = format!(
            " {} · {}/{} · F11 {}",
            app.tabs[app.active].title(locale),
            app.active + 1,
            app.tabs.len(),
            if locale == Locale::Vi {
                "Không gian làm việc"
            } else {
                "Workspace"
            }
        );
        frame.render_widget(
            Paragraph::new(clipped(&label, regions[0].width as usize)).style(bold(theme.text)),
            regions[0],
        );
    } else {
        tabs(frame, app, regions[0], theme);
    }
    let mut editor_area = regions[1];
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
    let title = locale.tr(if app.welcome() {
        "START HERE"
    } else {
        "EDITOR"
    });
    let block = panel(&title, theme, true);
    let body = block.inner(editor_area);
    frame.render_widget(block, editor_area);
    border_info(frame, app, editor_area, &title, theme);
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
    let tab_width = app.config.tab_width;
    let caret_pending = app.caret_layout_pending();
    let tab = &mut app.tabs[app.active];
    let document = &tab.document;
    let language = crate::app::document_language(document);
    let cursor_line = document.cursor_line();
    if caret_pending {
        // Never scan a multi-MiB line on the input thread: index it on the
        // layout worker and show a caret-anchored window meanwhile.
        app.layout.request(document, cursor_line, tab_width);
    }
    let cursor_cell = if caret_pending {
        None
    } else {
        let _span = crate::trace::span("cursor_layout");
        Some(document.cursor_cell(tab_width))
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
    if let Some(cursor_cell) = cursor_cell {
        if tab.follow_cursor && cursor_cell < tab.left {
            tab.left = cursor_cell;
        }
        if tab.follow_cursor && cursor_cell >= tab.left + width {
            tab.left = cursor_cell + 1 - width;
        }
    }
    let highlight = app.config.syntax && language != Language::Plain;
    app.syntax_pending = highlight
        && !tab
            .syntax
            .prepare(document, language, tab.top + body.height as usize);
    // Smart highlighting of the selected word and the caret's bracket pair,
    // both limited to what is visible (no document-wide scan per frame).
    let selected_word = document.selected_word();
    let bracket = document.matching_bracket();
    let caret_window = if caret_pending {
        document.caret_window(width, tab_width).ok()
    } else {
        None
    };
    for offset in 0..body.height {
        let line_idx = tab.top + offset as usize;
        let _line_span = crate::trace::span("line_layout_paint");
        if line_idx >= document.buffer().len_lines() {
            break;
        }
        let anchored = caret_window.as_ref().filter(|_| line_idx == cursor_line);
        let left = if anchored.is_some() { 0 } else { tab.left };
        let window = if let Some(caret) = anchored {
            guinea_core::LineWindow {
                text: caret.window.text.clone(),
                char_start: caret.window.char_start,
                cell_start: 0,
            }
        } else if document.layout_pending(line_idx, tab_width, 0, tab.left.saturating_add(width)) {
            // Another long unindexed line: keep the frame bounded.
            guinea_core::LineWindow {
                text: "…".into(),
                char_start: document.buffer().line_to_char(line_idx).unwrap_or(0),
                cell_start: tab.left,
            }
        } else {
            let Ok(window) = document.line_window(line_idx, tab.left, width, tab_width) else {
                break;
            };
            window
        };
        let text = window.text.as_str();
        let start = window.char_start;
        let active_line = line_idx == cursor_line;
        if gutter > 0 {
            let marker = if document.is_bookmarked(line_idx) {
                "◆"
            } else {
                "│"
            };
            let label = format!("{:>digits$} {marker} ", line_idx + 1, digits = gutter - 3);
            frame.render_widget(
                Paragraph::new(label).style(text_style(if active_line {
                    theme.accent
                } else {
                    theme.faint
                })),
                Rect::new(body.x, body.y + offset, gutter as u16, 1),
            );
        }
        let highlights = if highlight {
            tab.syntax
                .state(document, language, line_idx)
                .and_then(|state| {
                    crate::render::source_highlights(document, line_idx, &window, language, state)
                })
        } else {
            None
        };
        let mut marks = Vec::new();
        // Mark All matches inside the visible text of this line only.
        if let Some((needle, options)) = &tab.mark {
            for range in guinea_core::search::find_matches(text, needle, *options) {
                let from = start + text[..range.start].chars().count();
                let to = from + text[range].chars().count();
                marks.push((from..to, crate::render::Mark::Found));
            }
        }
        if let Some(word) = &selected_word {
            let options = guinea_core::SearchOptions {
                ignore_case: false,
                whole_word: true,
                regex: false,
            };
            for range in guinea_core::search::find_matches(text, word, options) {
                let from = start + text[..range.start].chars().count();
                let to = from + text[range].chars().count();
                if document.selection().range().start != from {
                    marks.push((from..to, crate::render::Mark::Occurrence));
                }
            }
        }
        if let Some((a, b)) = bracket {
            for at in [a, b] {
                marks.push((at..at + 1, crate::render::Mark::Bracket));
            }
        }
        let line = styled_line(
            text,
            start,
            document.selection().range(),
            left,
            width,
            tab_width,
            LinePaint {
                cell_start: window.cell_start,
                theme,
                highlights: highlights
                    .as_ref()
                    .and_then(|h| h.for_revision(document.revision()))
                    .unwrap_or(&[]),
                marks: &marks,
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
    if app.modal.is_none()
        && !busy
        && (tab.top..tab.top + body.height as usize).contains(&cursor_line)
    {
        let column = match (cursor_cell, &caret_window) {
            (Some(cell), _) => (tab.left..tab.left + width)
                .contains(&cell)
                .then(|| cell - tab.left),
            (None, Some(caret)) => Some(caret.caret_cell.min(width.saturating_sub(1))),
            (None, None) => None,
        };
        if let Some(column) = column {
            frame.set_cursor_position((
                body.x + gutter as u16 + column as u16,
                body.y + (cursor_line - tab.top) as u16,
            ));
        }
    }
}

fn review_panel(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let locale = app.config.language;
    let is_markdown = crate::app::document_language(app.document()) == Language::Markdown;
    let linked = app.tabs[app.active].review_source.is_some();
    let title = locale.tr("REVIEW · F8");
    let block = panel(&title, theme, true);
    let body = block.inner(area);
    frame.render_widget(block, area);
    // Full review has no editor panel: its border carries the file context.
    if app.tabs[app.active].review.mode == ViewMode::Review {
        border_info(frame, app, area, &title, theme);
    }
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
        "↑↓ Scroll · F8 View · Esc Source"
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
            format!("{percent}% · Alt+M Mục lục · Tab Chuyển vùng · Esc Về nguồn")
        } else {
            format!("{percent}% · Alt+M Contents · Tab Focus outline · Esc Source")
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
    let message = if app.layout.busy() && app.caret_layout_pending() {
        "Indexing long line… keep typing or moving"
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
    let caret_pending = app.caret_layout_pending();
    let document = app.document();
    let (line, column) = if caret_pending {
        (document.cursor_line(), None)
    } else {
        let (line, column) = document.position();
        (line, Some(column + 1))
    };
    let column = column.map_or_else(|| "…".to_string(), |c| c.to_string());
    let mode = if app.tabs[app.active].ai_action.is_some() {
        " AI · READ ONLY "
    } else if app.tabs[app.active].review_source.is_some() {
        " REVIEW "
    } else if document.readonly {
        " READ ONLY "
    } else if app.tabs[app.active].review.mode == ViewMode::Review {
        " REVIEW "
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
                format!("  Dòng {}, Cột {column} ", line + 1)
            } else {
                format!("  Ln {}, Col {column} ", line + 1)
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
    // Footer hints are generated from the keymap; see commands::hints.
    let (key, label) = (theme.accent, theme.muted);
    let hints = if app.focus_mode {
        hint_line(
            &[
                ("focus-mode", "Exit focus"),
                ("save", "Save"),
                ("open", "Open"),
                ("find", "Find"),
                ("tabs", "Tabs"),
                ("commands", "Commands"),
            ],
            locale,
            key,
            label,
        )
    } else if app.tabs[app.active].ai_action.is_some() {
        let mut line = Line::from(vec![
            Span::styled(" ↑↓", bold(key)),
            Span::styled(format!(" {} ", locale.tr("Navigate")), text_style(label)),
        ]);
        line.spans.extend(
            hint_line(
                &[
                    ("copy", "Copy"),
                    ("prev", "Prev"),
                    ("next", "Next"),
                    ("new", "New tab"),
                    ("settings", "Settings"),
                ],
                locale,
                key,
                label,
            )
            .spans,
        );
        line
    } else {
        hint_line(
            &[
                ("save", "Save"),
                ("find", "Find/Replace"),
                ("prev", "Prev"),
                ("next", "Next"),
                ("tabs", "Tabs"),
                ("help", "Help"),
                ("preview", "Review"),
                ("quit", "Quit"),
            ],
            locale,
            key,
            label,
        )
    };
    frame.render_widget(Paragraph::new(hints), areas[2]);
}

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    let locale = app.config.language;
    let theme = Theme::new(app.config.basic_colors);
    app.hits = HitRegions::default();
    frame.render_widget(Block::default().style(theme.base()), area);
    if area.width < 30 || area.height < 9 {
        frame.render_widget(
            Paragraph::new(
                locale
                    .tr("GUINEA NOTE\nResize to at least 30 × 9\nCtrl+Q to quit")
                    .into_owned(),
            )
            .style(text_style(theme.accent)),
            area,
        );
        return;
    }
    let compact = area.height < 14;
    let header_height = if app.focus_mode {
        0
    } else if compact || !app.welcome() {
        1
    } else {
        2
    };
    let areas = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Length(u16::from(header_height > 0)),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);
    if header_height > 0 {
        header(frame, app, areas[0], theme);
    }
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
    editor(frame, app, workspace, theme);
    status(frame, app, &areas[3..], theme);
    crate::dialogs::draw(frame, app, area, theme);
}
