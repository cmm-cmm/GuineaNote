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
        },
    )
}

pub(crate) struct LinePaint<'a> {
    pub cell_start: usize,
    pub theme: Theme,
    pub highlights: &'a [HighlightSpan],
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(app.hits.tabs.iter().all(|(r, _)| r.y > nav.bottom()));
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
        assert!(app.hits.tree.width > 0);
        terminal.backend_mut().resize(50, 10);
        terminal.autoresize().unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert_eq!(app.hits.tree.width, 0);
        assert!(app.hits.editor.width > 20);
    }

    #[test]
    fn explorer_focus_keeps_global_cancellation_feedback_visible() {
        let mut app = App::new(vec![], false, Config::default());
        app.focus = crate::app::Focus::Explorer;
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
}
