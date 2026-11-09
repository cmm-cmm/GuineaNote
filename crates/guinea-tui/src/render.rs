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
                    TokenKind::Keyword => base.fg(paint.theme.purple).add_modifier(Modifier::BOLD),
                    TokenKind::String => base.fg(paint.theme.mint),
                    TokenKind::Number | TokenKind::Constant => base.fg(paint.theme.amber),
                    TokenKind::Comment => base.fg(paint.theme.muted).add_modifier(Modifier::ITALIC),
                    TokenKind::Heading => base.fg(paint.theme.mint).add_modifier(Modifier::BOLD),
                    TokenKind::Key | TokenKind::Property => base.fg(paint.theme.blue),
                    TokenKind::Function => base.fg(paint.theme.blue).add_modifier(Modifier::BOLD),
                    TokenKind::Type => base.fg(paint.theme.amber).add_modifier(Modifier::ITALIC),
                    TokenKind::Operator => base.fg(paint.theme.red),
                    TokenKind::Punctuation => base.fg(paint.theme.muted),
                    TokenKind::Attribute => {
                        base.fg(paint.theme.purple).add_modifier(Modifier::ITALIC)
                    }
                    TokenKind::Escape => base.fg(paint.theme.amber).add_modifier(Modifier::BOLD),
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
    #[test]
    fn new_dialogs_are_visible_and_safe_when_resized() {
        let mut app = App::new(vec![], false, Config::default());
        for command in ["settings", "preview", "info", "open"] {
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
