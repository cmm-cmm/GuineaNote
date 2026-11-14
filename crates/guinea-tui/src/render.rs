use guinea_core::coordinates::display_grapheme;
use guinea_syntax::{HighlightSpan, TokenKind};
use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{app::App, theme::Theme};

pub(crate) fn safe(text: &str) -> String {
    text.chars()
        .map(|ch| if ch.is_control() { '�' } else { ch })
        .collect()
}

/// Lex from a known line origin, then clip spans to the visible scalar range.
/// Unknown context beyond the budget renders plain text rather than false colors.
pub(crate) fn source_highlights(
    document: &guinea_core::Document,
    line: usize,
    window: &guinea_core::LineWindow,
    language: guinea_syntax::Language,
    state: guinea_syntax::LineState,
) -> Option<guinea_syntax::Highlights> {
    use guinea_core::TextBuffer;
    let origin = document.buffer().line_to_char(line).ok()?;
    let offset = window.char_start.checked_sub(origin)?;
    let visible = window.text.chars().count();
    if offset.saturating_add(visible) > 16_384 {
        return None;
    }
    let prefix = document
        .buffer()
        .slice(origin..window.char_start + visible)
        .ok()?;
    let (mut highlights, _) = guinea_syntax::BasicSyntax.highlight_line_from(
        language,
        &prefix,
        document.revision(),
        state,
    );
    highlights.spans.retain_mut(|span| {
        let start = span.chars.start.max(offset);
        let end = span.chars.end.min(offset + visible);
        span.chars = start.saturating_sub(offset)..end.saturating_sub(offset);
        start < end
    });
    Some(highlights)
}

// Lex only the prefix needed for this viewport. Starting at byte zero keeps
// the line-local string/comment context correct while avoiding hidden suffixes.
#[cfg(test)]
pub(crate) fn prefix_through_cell(text: &str, right: usize, tab_width: usize) -> &str {
    let mut cell = 0;
    let mut end = 0;
    for (byte, grapheme) in text.grapheme_indices(true) {
        if cell >= right {
            break;
        }
        cell += display_grapheme(grapheme, cell, tab_width).1;
        end = byte + grapheme.len();
    }
    &text[..end]
}

/// Clip by terminal cells, padding any partially clipped wide grapheme. Never
/// emit half of an emoji/CJK cluster, nor a raw terminal control sequence.
#[cfg(test)]
fn visible_line(
    text: &str,
    start_char: usize,
    selection: std::ops::Range<usize>,
    left: usize,
    width: usize,
    tab_width: usize,
) -> Line<'static> {
    styled_line(
        text,
        start_char,
        selection,
        left,
        width,
        tab_width,
        LinePaint {
            cell_start: 0,
            theme: Theme::new(false),
            highlights: &[],
            marks: &[],
        },
    )
}

pub(crate) struct LinePaint<'a> {
    pub cell_start: usize,
    pub theme: Theme,
    pub highlights: &'a [HighlightSpan],
    /// Absolute scalar ranges painted over syntax colors (selection wins).
    pub marks: &'a [(std::ops::Range<usize>, Mark)],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    /// Another occurrence of the selected word (smart highlighting).
    Occurrence,
    /// The bracket at the caret or its match.
    Bracket,
    /// A Mark All match (Notepad++ "Find Mark Style").
    Found,
}

pub(crate) fn styled_line(
    text: &str,
    start_char: usize,
    selection: std::ops::Range<usize>,
    left: usize,
    width: usize,
    tab_width: usize,
    paint: LinePaint<'_>,
) -> Line<'static> {
    let mut spans = Vec::new();
    let mut cell = paint.cell_start;
    let mut char_idx = start_char;
    let mut token_idx = 0;
    for g in text.graphemes(true) {
        let count = g.chars().count();
        let (display, cells) = display_grapheme(g, cell, tab_width);
        let next = cell + cells;
        if cell >= left + width {
            break;
        }
        if next > left {
            let visible_start = cell.max(left);
            let visible_end = next.min(left + width);
            let output = if cell < left || next > left + width {
                " ".repeat(visible_end - visible_start)
            } else {
                display
            };
            let relative = char_idx - start_char;
            while paint
                .highlights
                .get(token_idx)
                .is_some_and(|span| span.chars.end <= relative)
            {
                token_idx += 1;
            }
            let token = paint
                .highlights
                .get(token_idx)
                .filter(|span| span.chars.contains(&relative));
            let mut style = token.map_or(Style::default().fg(paint.theme.text), |span| {
                let base = Style::default();
                match span.kind {
                    TokenKind::Keyword => base.fg(paint.theme.keyword).add_modifier(Modifier::BOLD),
                    TokenKind::String => base.fg(paint.theme.accent),
                    TokenKind::Number | TokenKind::Constant => base.fg(paint.theme.warm),
                    TokenKind::Comment => base.fg(paint.theme.muted).add_modifier(Modifier::ITALIC),
                    TokenKind::Heading => base.fg(paint.theme.accent).add_modifier(Modifier::BOLD),
                    TokenKind::Key | TokenKind::Property => base.fg(paint.theme.secondary),
                    TokenKind::Function => {
                        base.fg(paint.theme.secondary).add_modifier(Modifier::BOLD)
                    }
                    TokenKind::Type => base.fg(paint.theme.warm).add_modifier(Modifier::ITALIC),
                    TokenKind::Operator => base.fg(paint.theme.warm),
                    TokenKind::Punctuation => base.fg(paint.theme.muted),
                    TokenKind::Attribute => {
                        base.fg(paint.theme.keyword).add_modifier(Modifier::ITALIC)
                    }
                    TokenKind::Escape => base.fg(paint.theme.warm).add_modifier(Modifier::BOLD),
                }
            });
            match paint
                .marks
                .iter()
                .find(|(range, _)| range.contains(&char_idx))
                .map(|(_, mark)| *mark)
            {
                Some(Mark::Occurrence) => {
                    style = style.add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
                }
                Some(Mark::Bracket) => {
                    style = style
                        .fg(paint.theme.canvas)
                        .bg(paint.theme.accent)
                        .add_modifier(Modifier::BOLD);
                }
                // Reversed warm-on-canvas: a warm background in colour, and
                // reverse video once NO_COLOR resets every colour.
                Some(Mark::Found) => {
                    style = style
                        .fg(paint.theme.warm)
                        .bg(paint.theme.canvas)
                        .add_modifier(Modifier::REVERSED);
                }
                None => {}
            }
            if char_idx < selection.end && char_idx + count > selection.start {
                style = style.bg(paint.theme.line).fg(paint.theme.text);
            }
            spans.push(Span::styled(output, style));
        }
        cell = next;
        char_idx += count;
    }
    Line::from(spans)
}

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    crate::workspace_render::draw(frame, app);
    if app.config.no_color {
        for cell in &mut frame.buffer_mut().content {
            cell.set_fg(ratatui::style::Color::Reset)
                .set_bg(ratatui::style::Color::Reset);
        }
    } else if app.config.ansi_colors {
        let theme = Theme::new(app.config.basic_colors);
        for cell in &mut frame.buffer_mut().content {
            cell.set_fg(ansi_color(cell.fg, theme, false));
            cell.set_bg(ansi_color(cell.bg, theme, true));
        }
    }
}
fn ansi_color(
    color: ratatui::style::Color,
    theme: Theme,
    background: bool,
) -> ratatui::style::Color {
    use ratatui::style::Color;
    if color == theme.canvas || color == theme.panel {
        Color::Black
    } else if color == theme.raised {
        Color::DarkGray
    } else if color == theme.accent || color == theme.secondary || color == theme.keyword {
        if background {
            Color::Green
        } else {
            Color::LightGreen
        }
    } else if color == theme.warm || color == theme.line {
        Color::Yellow
    } else if color == theme.danger {
        Color::Red
    } else if color == theme.faint {
        Color::DarkGray
    } else if color == theme.muted {
        Color::Gray
    } else if matches!(color, Color::Rgb(..) | Color::Indexed(_)) {
        if background {
            Color::Black
        } else {
            Color::White
        }
    } else {
        color
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unified_search_keeps_both_fields_and_focused_actions_visible_without_mouse() {
        for locale in [crate::i18n::Locale::En, crate::i18n::Locale::Vi] {
            for size in [(30, 9), (40, 28), (80, 24), (148, 42)] {
                let mut app = App::new(
                    vec![],
                    false,
                    Config {
                        language: locale,
                        mouse: false,
                        ..Config::default()
                    },
                );
                let mut terminal = Terminal::new(TestBackend::new(size.0, size.1)).unwrap();
                for command in ["find", "replace"] {
                    app.command(command);
                    if let Some(crate::app::Modal::Search(search)) = app.modal.as_mut() {
                        search.query = "query".into();
                        search.replacement = "replacement".into();
                        search.focus = 7;
                    }
                    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
                    let content = terminal
                        .backend()
                        .buffer()
                        .content()
                        .iter()
                        .map(|cell| cell.symbol())
                        .collect::<String>();
                    assert!(content.contains("query"), "{size:?}");
                    assert!(content.contains("replacement"), "{size:?}");
                    assert!(
                        content.contains(if locale == crate::i18n::Locale::En {
                            "Searching"
                        } else {
                            "Đang tìm"
                        }),
                        "Search status hidden at {size:?}"
                    );
                    assert!(
                        content.contains(if locale == crate::i18n::Locale::En {
                            "Replace all"
                        } else {
                            "Thay tất cả"
                        }),
                        "{size:?}"
                    );
                    for (rect, _) in &app.hits.modal_actions {
                        assert!(rect.right() <= size.0 && rect.bottom() <= size.1);
                    }
                }
            }
        }
    }

    #[test]
    fn keyboard_guide_scrolls_to_the_last_line_in_a_small_terminal() {
        use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
        for locale in [crate::i18n::Locale::En, crate::i18n::Locale::Vi] {
            let mut app = App::new(
                vec![],
                false,
                Config {
                    language: locale,
                    ..Config::default()
                },
            );
            let mut terminal = Terminal::new(TestBackend::new(30, 9)).unwrap();
            app.command("help");
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            assert!(app.help_rows > app.help_height);
            app.handle(Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)));
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            let content = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(content.contains(if locale == crate::i18n::Locale::En {
                "recovery"
            } else {
                "nháp"
            }));
            assert!(app.help_scroll > 0);
            app.handle(Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)));
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            assert_eq!(app.help_scroll, 0);
        }
    }
    #[test]
    fn caret_stays_visible_while_a_save_runs_and_typing_is_allowed() {
        use crossterm::event::Event;
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::new(vec![], false, Config::default());
        app.tabs[0].document.insert("draft").unwrap();
        app.tabs[0].document.path = Some(dir.path().join("note.txt"));
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let before = terminal.get_cursor_position().unwrap();
        // The save is announced but its worker has not started: edits continue.
        app.command("save");
        assert!(app.busy());
        app.handle(Event::Paste("xy".into()));
        assert_eq!(app.document().text(), "draftxy");
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let after = terminal.get_cursor_position().unwrap();
        assert_eq!(
            (after.x, after.y),
            (before.x + 2, before.y),
            "the caret must follow typing during a save"
        );
    }

    #[test]
    fn split_without_room_for_two_panes_is_read_only_until_it_fits_again() {
        use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
        let key = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
        let screen = |terminal: &Terminal<TestBackend>| {
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
        };
        let mut app = App::new(vec![], false, Config::default());
        app.tabs[0].document.insert("# Title").unwrap();
        app.tabs[0].document.path = Some("note.md".into());
        app.command("split");
        // Before the first frame the layout is unknown: split stays editable.
        app.handle(Event::Paste("!".into()));
        assert_eq!(app.document().text(), "# Title!");
        let mut terminal = Terminal::new(TestBackend::new(70, 24)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(app.hits.editor_text.is_empty() && !app.hits.review.is_empty());
        app.handle(Event::Paste("blind".into()));
        app.handle(key(KeyCode::Char('x')));
        app.handle(key(KeyCode::Backspace));
        app.handle(key(KeyCode::Enter));
        app.command("undo");
        assert_eq!(
            app.document().text(),
            "# Title!",
            "typing must not edit a source that is not on screen"
        );
        assert!(app.message.contains("read-only"), "{}", app.message);
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(screen(&terminal).contains(" REVIEW "));
        assert!(!screen(&terminal).contains(" EDIT "));
        // The hint on screen says Esc returns to the source; it must.
        app.handle(key(KeyCode::Esc));
        assert_eq!(app.tabs[0].review.mode, crate::review::ViewMode::Source);
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        app.handle(Event::Paste("?".into()));
        assert_eq!(app.document().text(), "# Title!?");
        // With room for both panes, split is an editor again.
        app.command("split");
        terminal.backend_mut().resize(120, 30);
        terminal.autoresize().unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(!app.hits.editor_text.is_empty() && !app.hits.review.is_empty());
        app.handle(Event::Paste(" ok".into()));
        assert_eq!(app.document().text(), "# Title!? ok");
        assert!(screen(&terminal).contains(" EDIT "));
    }

    #[test]
    fn ansi_palette_project_and_command_dialogs_fit_small_viewports() {
        for size in [(80, 24), (40, 12), (30, 9)] {
            let mut app = App::new(
                vec![],
                false,
                Config {
                    ansi_colors: true,
                    no_color: false,
                    ..Config::default()
                },
            );
            app.tabs[0].document.insert("text").unwrap();
            let mut terminal = Terminal::new(TestBackend::new(size.0, size.1)).unwrap();
            for command in ["open-path", "save-as", "goto", "help"] {
                app.command(command);
                terminal.draw(|f| draw(f, &mut app)).unwrap();
                for cell in terminal.backend().buffer().content() {
                    assert!(!matches!(
                        cell.fg,
                        ratatui::style::Color::Rgb(..) | ratatui::style::Color::Indexed(_)
                    ));
                    assert!(!matches!(
                        cell.bg,
                        ratatui::style::Color::Rgb(..) | ratatui::style::Color::Indexed(_)
                    ));
                }
            }
            app.modal = None;
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            let normal = app.hits.editor_text.height;
            app.command("focus-mode");
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            assert!(app.hits.editor_text.height >= normal);
        }
    }
    #[test]
    fn clipped_syntax_uses_real_string_context_and_falls_back_when_unknown() {
        let document = guinea_core::Document::from_text("let value = \"true + 42\";");
        let window = document.line_window(0, 13, 8, 4).unwrap();
        let result = source_highlights(
            &document,
            0,
            &window,
            guinea_syntax::Language::Rust,
            Default::default(),
        )
        .unwrap();
        assert!(!result.spans.is_empty());
        assert!(result.spans.iter().all(|s| s.kind == TokenKind::String));
        let document = guinea_core::Document::from_text(&format!("{}true", " ".repeat(17000)));
        let window = document.line_window(0, 17000, 4, 4).unwrap();
        assert!(
            source_highlights(
                &document,
                0,
                &window,
                guinea_syntax::Language::Rust,
                Default::default()
            )
            .is_none()
        );
    }

    #[test]
    fn monochrome_resets_colors_in_source_and_dialog_cells() {
        let mut app = App::new(
            vec![],
            false,
            Config {
                no_color: true,
                ..Config::default()
            },
        );
        app.tabs[0].document.insert("let value = 42;").unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 28)).unwrap();
        for settings in [false, true] {
            if settings {
                app.command("settings");
            }
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            for cell in terminal.backend().buffer().content() {
                assert_eq!(cell.fg, ratatui::style::Color::Reset);
                assert_eq!(cell.bg, ratatui::style::Color::Reset);
            }
        }
    }

    fn mouse_event(
        kind: crossterm::event::MouseEventKind,
        x: u16,
        y: u16,
    ) -> crossterm::event::Event {
        crossterm::event::Event::Mouse(crossterm::event::MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: crossterm::event::KeyModifiers::NONE,
        })
    }
    #[test]
    fn confirmation_buttons_cancel_by_default_and_close_the_clicked_inactive_tab() {
        use crossterm::event::{
            Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind,
        };
        let mut app = App::new(vec![], false, Config::default());
        app.tabs[0].document.path = Some("alpha.txt".into());
        app.tabs[0].document.insert("draft").unwrap();
        app.command("new");
        app.tabs[1].document.path = Some("beta.json".into());
        let active = app.document().id;
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        for discard in [false, true] {
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            let rect = app.hits.tabs.iter().find(|(_, i)| *i == 0).unwrap().0;
            app.handle(mouse_event(
                MouseEventKind::Down(MouseButton::Middle),
                rect.x + 2,
                rect.y,
            ));
            assert!(matches!(
                app.modal,
                Some(crate::app::Modal::Confirm {
                    selected: false,
                    ..
                })
            ));
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            let accept = app
                .hits
                .modal_actions
                .iter()
                .find(|(_, a)| *a == "confirm-accept")
                .unwrap()
                .0;
            let outside = app
                .hits
                .actions
                .iter()
                .find(|(_, a)| *a == "new")
                .unwrap()
                .0;
            app.handle(mouse_event(
                MouseEventKind::Down(MouseButton::Left),
                outside.x,
                outside.y,
            ));
            assert_eq!(
                app.tabs.len(),
                2,
                "Dialog must block clicks on the workspace"
            );
            if discard {
                app.handle(mouse_event(
                    MouseEventKind::Down(MouseButton::Left),
                    accept.x + 1,
                    accept.y + 1,
                ));
            } else {
                app.handle(Event::Key(KeyEvent::new(
                    KeyCode::Enter,
                    KeyModifiers::NONE,
                )));
            }
            assert!(app.modal.is_none());
            assert_eq!(app.document().id, active);
            assert_eq!(app.tabs.len(), if discard { 1 } else { 2 });
        }
    }
    #[test]
    fn mouse_drag_selects_whole_unicode_clusters_and_wheel_preserves_caret() {
        use crossterm::event::{
            Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind,
        };
        let mut app = App::new(vec![], false, Config::default());
        app.tabs[0]
            .document
            .insert(&format!(
                "a界e\u{301}👩‍💻z\n{}",
                (1..80).map(|n| format!("line{n}\n")).collect::<String>()
            ))
            .unwrap();
        app.tabs[0].document.select(0, 0).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let rect = app.hits.editor_text;
        app.handle(mouse_event(
            MouseEventKind::Down(MouseButton::Left),
            rect.x + 1,
            rect.y,
        ));
        app.handle(mouse_event(
            MouseEventKind::Drag(MouseButton::Left),
            rect.x + 7,
            rect.y,
        ));
        app.handle(mouse_event(
            MouseEventKind::Up(MouseButton::Left),
            rect.x + 7,
            rect.y,
        ));
        assert_eq!(app.document().selected_text(), "界e\u{301}👩‍💻z");
        let cursor = app.document().cursor();
        app.handle(mouse_event(MouseEventKind::ScrollDown, rect.x + 3, rect.y));
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        assert_eq!(app.tabs[0].top, 3);
        assert_eq!(app.document().cursor(), cursor);
        app.handle(Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)));
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        assert_eq!(app.tabs[0].top, 0);
        let nav = app
            .hits
            .actions
            .iter()
            .find(|(_, a)| *a == "tabs")
            .unwrap()
            .0;
        // One tab bar row: tabs on the left, controls on the right.
        assert!(
            app.hits
                .tabs
                .iter()
                .all(|(r, _)| r.y == nav.y && r.right() < nav.x)
        );
        let status = terminal.backend().buffer();
        let row = (0..status.area.height)
            .find(|&y| {
                (0..status.area.width)
                    .map(|x| status[(x, y)].symbol())
                    .collect::<String>()
                    .contains("Ln 1, Col")
            })
            .unwrap();
        let utf8 = (0..status.area.width)
            .find(|&x| status[(x, row)].symbol() == "U")
            .unwrap();
        assert!(
            utf8 > status.area.width / 2,
            "Encoding belongs to the right status group"
        );
    }
    #[test]
    fn small_dialog_buttons_stay_inside_and_tabs_require_a_real_double_click() {
        use crossterm::event::{MouseButton, MouseEventKind};
        for command in [
            "settings", "preview", "open", "tabs", "info", "save-as", "help", "close",
        ] {
            let mut app = App::new(vec![], false, Config::default());
            app.tabs[0].document.insert("draft").unwrap();
            app.command(command);
            let mut terminal = Terminal::new(TestBackend::new(30, 9)).unwrap();
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            for (r, _) in &app.hits.modal_actions {
                assert!(
                    r.x >= app.hits.modal.x
                        && r.right() <= app.hits.modal.right()
                        && r.y >= app.hits.modal.y
                        && r.bottom() <= app.hits.modal.bottom(),
                    "{command}: button {r:?} outside {:?}",
                    app.hits.modal
                );
            }
        }
        let mut app = App::new(vec![], false, Config::default());
        app.command("new");
        app.command("tabs");
        let mut terminal = Terminal::new(TestBackend::new(110, 32)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let list = match &app.modal {
            Some(crate::app::Modal::Tabs(m)) => m.list,
            _ => unreachable!(),
        };
        app.handle(mouse_event(MouseEventKind::ScrollDown, 0, 0));
        app.handle(mouse_event(
            MouseEventKind::Down(MouseButton::Left),
            list.x + 2,
            list.y + 4,
        ));
        assert!(matches!(&app.modal,Some(crate::app::Modal::Tabs(m)) if m.selected==1));
        app.handle(mouse_event(
            MouseEventKind::Down(MouseButton::Left),
            list.x + 2,
            list.y,
        ));
        assert!(app.modal.is_some());
        app.handle(mouse_event(
            MouseEventKind::Down(MouseButton::Left),
            list.x + 2,
            list.y,
        ));
        assert!(app.modal.is_none());
        assert_eq!(app.active, 0);
    }
    #[test]
    fn ai_context_and_manager_details_follow_linked_review_and_locale() {
        let mut app = App::new(vec![], false, Config::default());
        app.tabs[0].document.insert("{\"name\":\"Việt\"}").unwrap();
        app.tabs[0].document.path = Some("data.json".into());
        app.tabs[0].ai_action = Some("analyze".into());
        app.tabs[0].document.readonly = true;
        app.command("review-tab");
        let mut terminal = Terminal::new(TestBackend::new(148, 42)).unwrap();
        for locale in [crate::i18n::Locale::En, crate::i18n::Locale::Vi] {
            app.config.language = locale;
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            let screen = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(screen.contains(if locale == crate::i18n::Locale::En {
                "AI · Analyzing"
            } else {
                "AI · Đang phân tích"
            }));
            assert!(screen.contains(if locale == crate::i18n::Locale::En {
                "AI · READ ONLY"
            } else {
                "AI · CHỈ ĐỌC"
            }));
            assert!(
                app.hits.tab_closes.is_empty(),
                "Leased tabs must not expose an enabled close target"
            );
            app.command("tabs");
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            let screen = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(screen.contains("JSON · 17 B"));
            assert!(screen.contains(if locale == crate::i18n::Locale::En {
                "Other tabs remain editable"
            } else {
                "Các thẻ khác vẫn chỉnh sửa được"
            }));
            app.handle(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Esc,
                    crossterm::event::KeyModifiers::NONE,
                ),
            ));
        }
    }
    #[test]
    fn new_dialogs_are_visible_and_safe_when_resized() {
        let mut app = App::new(vec![], false, Config::default());
        for command in ["settings", "preview", "info", "open", "tabs"] {
            app.command(command);
            let mut terminal = Terminal::new(TestBackend::new(110, 32)).unwrap();
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            for (width, height) in [(50, 14), (30, 9), (110, 32)] {
                terminal.backend_mut().resize(width, height);
                terminal.autoresize().unwrap();
                terminal.draw(|f| draw(f, &mut app)).unwrap();
            }
            assert!(app.modal.is_some());
        }
    }
    use crate::config::Config;
    use guinea_syntax::{BasicSyntax, Language, SyntaxProvider};
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn overflowing_tabs_keep_count_active_and_close_hit_regions_correct() {
        use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let mut app = App::new(vec![], false, Config::default());
        for index in 0..25 {
            app.tabs[app.active].document.path =
                Some(format!("directory-{index}/same-long-name.md").into());
            app.command("new");
        }
        let mut terminal = Terminal::new(TestBackend::new(110, 30)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>()
                .contains("Tabs 26/26")
        );
        assert!(app.hits.tabs.iter().any(|(_, index)| *index == app.active));
        assert!(app.hits.tabs.len() < app.tabs.len());
        let active_id = app.document().id;
        let (rect, index) = app
            .hits
            .tab_closes
            .iter()
            .find(|(_, i)| *i != app.active)
            .copied()
            .unwrap();
        let closed_id = app.tabs[index].document.id;
        app.handle(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        }));
        assert_eq!(app.document().id, active_id);
        assert!(!app.tabs.iter().any(|t| t.document.id == closed_id));
        for locale in [crate::i18n::Locale::Vi, crate::i18n::Locale::En] {
            app.config.language = locale;
            for (width, height) in [(30, 9), (50, 14), (110, 30)] {
                terminal.backend_mut().resize(width, height);
                terminal.autoresize().unwrap();
                terminal.draw(|f| draw(f, &mut app)).unwrap();
                assert!(app.hits.actions.iter().any(|(_, action)| *action == "tabs"));
            }
        }
    }

    #[test]
    fn main_menu_is_separated_from_tab_management_and_click_regions() {
        use crate::i18n::Locale;
        for locale in [Locale::En, Locale::Vi] {
            for welcome in [true, false] {
                for (width, height) in [(30, 9), (40, 12), (80, 24), (100, 28), (148, 42)] {
                    let mut app = App::new(vec![], false, Config::default());
                    app.config.language = locale;
                    if !welcome {
                        app.tabs[0].document.path = Some("note.rs".into());
                    }
                    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                    terminal.draw(|f| draw(f, &mut app)).unwrap();
                    let menu = app
                        .hits
                        .actions
                        .iter()
                        .find(|(_, a)| *a == "open")
                        .unwrap()
                        .0;
                    let controls = app
                        .hits
                        .actions
                        .iter()
                        .find(|(_, a)| *a == "new")
                        .unwrap()
                        .0;
                    assert!(controls.y > menu.bottom(), "{width}x{height}");
                    for x in 0..width {
                        assert_eq!(
                            terminal.backend().buffer()[(x, menu.bottom())].symbol(),
                            "─"
                        );
                    }
                    // Tabs share the tab bar row, left of every control.
                    for (r, _) in &app.hits.tabs {
                        assert_eq!(r.y, controls.y);
                        assert!(r.right() <= controls.x, "{width}x{height}");
                    }
                    assert!(!menu.intersects(controls));
                }
            }
        }
    }

    #[test]
    fn tabs_keep_neighbors_and_active_file_visible() {
        let mut app = App::new(vec![], false, Config::default());
        app.tabs[0].document.path = Some("README.md".into());
        app.command("new");
        app.tabs[1].document.path = Some("main.rs".into());
        let mut terminal = Terminal::new(TestBackend::new(100, 28)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert_eq!(
            app.hits
                .tabs
                .iter()
                .map(|(_, index)| *index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        terminal.backend_mut().resize(50, 10);
        terminal.autoresize().unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(app.hits.editor.width > 20);
    }

    #[test]
    fn status_keeps_global_cancellation_feedback_visible() {
        let mut app = App::new(vec![], false, Config::default());
        app.message = "Cancelled".into();
        let mut terminal = Terminal::new(TestBackend::new(100, 28)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let screen = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(screen.contains("Cancelled"));
    }

    #[test]
    fn syntax_prefix_stops_at_viewport_without_splitting_graphemes() {
        let text = format!("let 界\t👩‍💻{}", "x".repeat(1024 * 1024));
        assert_eq!(prefix_through_cell(&text, 5, 4), "let 界");
        assert_eq!(prefix_through_cell(&text, 9, 4), "let 界\t👩‍💻");
        assert!(prefix_through_cell(&text, 80, 4).len() < 100);
        let prefix = prefix_through_cell("let s = \"// remains a string\"; // comment", 18, 4);
        let spans = BasicSyntax.highlight_line(Language::Rust, prefix, 0).spans;
        assert!(spans.iter().any(|span| span.kind == TokenKind::String));
        assert!(!spans.iter().any(|span| span.kind == TokenKind::Comment));
    }

    #[test]
    fn cell_clipping_never_splits_wide_cluster_or_emits_controls() {
        let line = visible_line("界👩‍💻\t\u{1b}", 0, 0..0, 1, 7, 4);
        let text = line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>();
        assert_eq!(text, " 👩‍💻    ");
        let line = visible_line("\u{1b}", 0, 0..0, 0, 10, 4);
        assert_eq!(line.spans[0].content, "�");
    }

    #[test]
    fn renders_unicode_viewport_and_handles_tiny_resize() {
        let mut app = App::new(vec![], false, Config::default());
        app.handle(crossterm::event::Event::Paste(
            "Việt 👩‍💻\t界\nsecond line".into(),
        ));
        app.tabs[0].document.path = Some(std::path::PathBuf::from(
            "long directory name/longer directory name/visible-position.txt",
        ));
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let screen = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(screen.contains("UTF-8"));
        assert!(screen.contains("Ln 2, Col"));
        assert!(screen.contains("second line"));
        terminal.backend_mut().resize(2, 1);
        terminal.autoresize().unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
    }

    #[test]
    fn up_down_wait_for_the_long_line_index_and_keep_input_order() {
        use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
        let unit = "Việt e\u{301} 界 👩‍💻 ";
        let long = unit.repeat(2 * 1024 * 1024 / unit.len());
        let text = format!("{long}\nshort\nlast");
        let key = |code| Event::Key(KeyEvent::new(code, KeyModifiers::NONE));
        let mut app = App::new(
            vec![],
            false,
            Config {
                mouse: false,
                ..Config::default()
            },
        );
        app.tabs[0].document = guinea_core::Document::from_text(&text);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        app.handle(key(KeyCode::End));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(app.caret_layout_pending() && app.layout.busy());
        let end = app.document().cursor();
        // Down waits for the index instead of counting the line now.
        app.handle(key(KeyCode::Down));
        app.handle(key(KeyCode::Down));
        assert_eq!(app.document().cursor(), end, "queued, not applied yet");
        assert_eq!(app.message, "Indexing the long line… the caret follows");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while app.document().cursor() == end {
            assert!(
                std::time::Instant::now() < deadline,
                "queued moves never applied"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
            app.poll_jobs();
        }
        assert_eq!(app.document().position(), (2, 4), "two lines down, clamped");
        // Another key before the index lands applies queued moves first.
        app.handle(key(KeyCode::Up));
        app.handle(key(KeyCode::Up));
        assert_eq!(app.document().cursor_line(), 0);
        let mut app2 = App::new(vec![], false, Config::default());
        app2.tabs[0].document = guinea_core::Document::from_text(&text);
        terminal.draw(|frame| draw(frame, &mut app2)).unwrap();
        app2.handle(key(KeyCode::End));
        terminal.draw(|frame| draw(frame, &mut app2)).unwrap();
        app2.handle(key(KeyCode::Down));
        app2.handle(Event::Paste("X".into()));
        assert_eq!(
            app2.document().cursor_line(),
            1,
            "Down applied before the paste"
        );
        assert!(app2.document().text().ends_with("\nshortX\nlast"));
    }

    #[test]
    fn first_end_on_unindexed_long_line_renders_without_blocking_then_settles() {
        use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
        let unit = "Việt e\u{301} 界 👩‍💻 ";
        let text = unit.repeat(2 * 1024 * 1024 / unit.len());
        for locale in [crate::i18n::Locale::En, crate::i18n::Locale::Vi] {
            let mut app = App::new(
                vec![],
                false,
                Config {
                    language: locale,
                    mouse: false,
                    ..Config::default()
                },
            );
            app.tabs[0].document = guinea_core::Document::from_text(&text);
            let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            app.handle(Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)));
            assert!(app.caret_layout_pending());
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            assert!(app.layout.busy(), "End must index on the layout worker");
            let screen = |terminal: &Terminal<TestBackend>| {
                terminal
                    .backend()
                    .buffer()
                    .content()
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>()
            };
            let pending = screen(&terminal);
            assert!(pending.contains(if locale == crate::i18n::Locale::En {
                "Col …"
            } else {
                "Cột …"
            }));
            assert!(pending.contains("界"), "caret window shows nearby text");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            while app.layout.busy() {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(5));
                app.poll_jobs();
            }
            assert!(!app.caret_layout_pending());
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            let expected = app.document().position().1 + 1;
            assert_eq!(expected, text.graphemes(true).count() + 1);
            let settled = screen(&terminal);
            assert!(
                settled.contains(&format!("{expected} ")),
                "exact column after indexing"
            );
        }
    }

    /// Every clickable region must run something real: main-window actions
    /// are commands the dispatcher knows, dialog actions are handled by the
    /// dialog mouse code, and F2 rows are keymap commands. Checked at the
    /// supported sizes, in both languages, for welcome/editor/focus states
    /// and with every dialog open.
    #[test]
    fn every_clickable_region_maps_to_a_handled_action() {
        use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
        let mouse_code = concat!(include_str!("mouse_app.rs"), include_str!("search_app.rs"));
        let mut main_actions = std::collections::BTreeSet::new();
        let mut modal_actions = std::collections::BTreeSet::new();
        for locale in [crate::i18n::Locale::En, crate::i18n::Locale::Vi] {
            for size in [(30, 9), (56, 20), (80, 24), (100, 28), (148, 42)] {
                for state in 0..3 {
                    let mut app = App::new(
                        vec![],
                        false,
                        Config {
                            language: locale,
                            ..Config::default()
                        },
                    );
                    if state > 0 {
                        app.handle(Event::Paste("Việt e\u{301} 👩‍💻\nline two".into()));
                        app.command("new");
                    }
                    if state == 2 {
                        app.command("focus-mode");
                    }
                    let mut terminal = Terminal::new(TestBackend::new(size.0, size.1)).unwrap();
                    terminal.draw(|f| draw(f, &mut app)).unwrap();
                    main_actions.extend(app.hits.actions.iter().map(|(_, id)| *id));
                    for open in [
                        "find",
                        "tabs",
                        "settings",
                        "preview",
                        "help",
                        "open",
                        "goto",
                        "file-info",
                        "commands",
                        "quit",
                    ] {
                        app.modal = None;
                        app.command(open);
                        terminal.draw(|f| draw(f, &mut app)).unwrap();
                        modal_actions.extend(app.hits.modal_actions.iter().map(|(_, id)| *id));
                        for (_, id) in &app.hits.commands {
                            assert!(crate::commands::find(id).is_some(), "F2 row {id}");
                        }
                    }
                    app.handle(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
                }
            }
        }
        assert!(main_actions.contains("open") && main_actions.contains("tabs"));
        for id in &main_actions {
            let mut app = App::new(vec![], false, Config::default());
            app.command(id);
            assert!(
                !app.message.contains("Unknown command"),
                "main-window button {id} runs no command"
            );
        }
        assert!(
            modal_actions.contains("cancel") && modal_actions.contains("search-query"),
            "{modal_actions:?} {main_actions:?}"
        );
        for id in &modal_actions {
            assert!(
                mouse_code.contains(&format!("\"{id}\"")),
                "dialog button {id} has no mouse handler"
            );
        }
    }

    #[test]
    fn mark_all_matches_are_painted_in_visible_lines() {
        use crossterm::event::Event;
        use ratatui::style::Modifier;
        let config = Config {
            ansi_colors: false,
            no_color: false,
            ..Config::default()
        };
        let mut app = App::new(vec![], false, config);
        app.handle(Event::Paste("x Việt y\nviệt Việt\n".into()));
        let active = app.active;
        let _ = app.tabs[active].document.select(2, 6);
        app.command("mark-all");
        // Move the caret away so the selection paint does not hide the mark.
        let _ = app.tabs[active].document.select(0, 0);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let editor = app.hits.editor_text;
        let reversed = |x: u16, y: u16| {
            terminal.backend().buffer()[(editor.x + x, editor.y + y)]
                .modifier
                .contains(Modifier::REVERSED)
        };
        assert!(reversed(2, 0) && reversed(5, 0), "first “Việt”");
        assert!(!reversed(0, 0) && !reversed(6, 0));
        assert!(!reversed(0, 1), "case-sensitive: “việt” is not marked");
        assert!(reversed(5, 1) && reversed(8, 1));
        app.command("mark-clear");
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let reversed = |x: u16, y: u16| {
            terminal.backend().buffer()[(editor.x + x, editor.y + y)]
                .modifier
                .contains(Modifier::REVERSED)
        };
        assert!(!reversed(2, 0));
    }

    #[test]
    fn bookmarks_smart_highlight_and_brackets_are_painted() {
        use crossterm::event::{
            Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
        };
        use ratatui::style::Modifier;
        // The palette assertions below need the colour theme, whatever TERM
        // or NO_COLOR the test runs under (TERM=linux selects ANSI-16).
        let config = Config {
            ansi_colors: false,
            no_color: false,
            ..Config::default()
        };
        let mut app = App::new(vec![], false, config);
        app.handle(Event::Paste(
            "let total = f(total);\ntotal += 1;\nnext".into(),
        ));
        app.handle(Event::Key(KeyEvent::new(
            KeyCode::F(2),
            KeyModifiers::CONTROL,
        )));
        assert!(app.document().is_bookmarked(2));
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let editor = app.hits.editor_text;
        let row = |terminal: &Terminal<TestBackend>, y: u16| {
            (0..80)
                .map(|x| terminal.backend().buffer()[(x, y)].symbol().to_string())
                .collect::<String>()
        };
        assert!(
            row(&terminal, editor.y + 2).contains('◆'),
            "bookmark gutter marker"
        );
        // Select the first "total": the other two visible ones are underlined.
        let active = app.active;
        let _ = app.tabs[active].document.select(4, 9);
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let underlined = |terminal: &Terminal<TestBackend>, x: u16, y: u16| {
            terminal.backend().buffer()[(x, y)]
                .modifier
                .contains(Modifier::UNDERLINED)
        };
        assert!(
            underlined(&terminal, editor.x + 14, editor.y),
            "second total"
        );
        assert!(underlined(&terminal, editor.x, editor.y + 1), "third total");
        assert!(
            !underlined(&terminal, editor.x + 4, editor.y),
            "selection itself"
        );
        // The caret before "(" highlights the bracket pair.
        let _ = app.tabs[active].document.select(13, 13);
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let theme = crate::theme::Theme::new(app.config.basic_colors);
        assert_eq!(
            terminal.backend().buffer()[(editor.x + 13, editor.y)].bg,
            theme.accent
        );
        assert_eq!(
            terminal.backend().buffer()[(editor.x + 19, editor.y)].bg,
            theme.accent
        );
        // Clicking the line number toggles the bookmark of that line.
        app.handle(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: editor.x - 2,
            row: editor.y,
            modifiers: KeyModifiers::NONE,
        }));
        assert!(app.document().is_bookmarked(0));
        app.handle(Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::ALT)));
        assert_eq!(app.document().cursor_line(), 2);
        app.handle(Event::Key(KeyEvent::new(
            KeyCode::F(2),
            KeyModifiers::SHIFT,
        )));
        assert_eq!(app.document().cursor_line(), 0);
    }
}
