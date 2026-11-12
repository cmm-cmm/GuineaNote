use super::{MarkdownStats, OutlineEntry, Writer, code};
use crate::{i18n::Locale, render::safe, theme::Theme, workspace_render::clipped};
use guinea_syntax::{BasicSyntax, Language, SyntaxProvider};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use std::collections::BTreeMap;
use unicode_segmentation::UnicodeSegmentation;

fn fg(color: ratatui::style::Color) -> Style {
    Style::default().fg(color)
}
fn bold(color: ratatui::style::Color) -> Style {
    fg(color).add_modifier(Modifier::BOLD)
}
fn label<'a>(locale: Locale, en: &'a str, vi: &'a str) -> &'a str {
    if locale == Locale::Vi { vi } else { en }
}
fn prose(writer: &mut Writer, text: &str, style: Style) {
    for word in text.split_word_bounds() {
        if writer.full() {
            break;
        }
        let whitespace = word.trim().is_empty();
        if whitespace && writer.cell == 0 {
            continue;
        }
        let cells = Line::from(word).width();
        if !whitespace
            && cells <= writer.width
            && writer.cell > 0
            && writer.cell + cells > writer.width
        {
            writer.newline();
        }
        writer.push(word, style);
    }
}

#[derive(Default)]
struct Metadata {
    stats: MarkdownStats,
    links: Vec<String>,
    indices: BTreeMap<String, usize>,
}
impl Metadata {
    fn observe(&mut self, event: &Event<'_>) {
        match event {
            Event::Start(Tag::Heading { .. }) => self.stats.headings += 1,
            Event::Start(Tag::CodeBlock(_)) => self.stats.code_blocks += 1,
            Event::Start(Tag::Table(_)) => self.stats.tables += 1,
            Event::TaskListMarker(done) => {
                self.stats.tasks += 1;
                self.stats.completed_tasks += usize::from(*done);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                self.stats.links += 1;
                if self.links.len() < 500 && !self.indices.contains_key(dest_url.as_ref()) {
                    let url = clipped(dest_url, 512);
                    self.links.push(url);
                    self.indices.insert(dest_url.to_string(), self.links.len());
                }
            }
            _ => {}
        }
    }
}
fn collect<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    end: TagEnd,
    meta: &mut Metadata,
) -> Vec<Event<'a>> {
    let mut collected = Vec::new();
    let mut bytes = 0;
    let mut depth = 0;
    for event in events.by_ref() {
        meta.observe(&event);
        if matches!(&event, Event::Start(tag) if tag.to_end()==end) {
            depth += 1;
        }
        if matches!(&event, Event::End(tag) if *tag==end) {
            if depth == 0 {
                break;
            }
            depth -= 1;
        }
        if collected.len() < 100_000 && bytes < 2 * 1024 * 1024 {
            bytes += match &event {
                Event::Text(s) | Event::Code(s) => s.len(),
                _ => 0,
            };
            collected.push(event);
        }
    }
    collected
}
fn inline(events: &[Event<'_>], theme: Theme, meta: &Metadata) -> Line<'static> {
    let mut spans = Vec::new();
    let mut style = fg(theme.text);
    let mut stack = Vec::new();
    let mut link_ids = Vec::new();
    for event in events {
        match event {
            Event::Start(tag) => {
                stack.push(style);
                match tag {
                    Tag::Strong => style = style.add_modifier(Modifier::BOLD),
                    Tag::Emphasis => style = style.add_modifier(Modifier::ITALIC),
                    Tag::Strikethrough => style = style.add_modifier(Modifier::CROSSED_OUT),
                    Tag::Link { dest_url, .. } => {
                        style = style.fg(theme.secondary).add_modifier(Modifier::UNDERLINED);
                        link_ids.push(meta.indices.get(dest_url.as_ref()).copied());
                    }
                    _ => {}
                }
            }
            Event::End(tag) => {
                if *tag == TagEnd::Link
                    && let Some(Some(id)) = link_ids.pop()
                {
                    spans.push(Span::styled(format!(" [{id}]"), fg(theme.muted)));
                }
                style = stack.pop().unwrap_or(fg(theme.text));
            }
            Event::Text(text) => spans.push(Span::styled(safe(text), style)),
            Event::Code(text) => {
                spans.push(Span::styled(safe(text), fg(theme.warm).bg(theme.raised)))
            }
            Event::SoftBreak => spans.push(Span::raw(" ")),
            Event::HardBreak => spans.push(Span::raw("\n")),
            Event::TaskListMarker(done) => spans.push(Span::styled(
                if *done { "[x] " } else { "[ ] " },
                fg(theme.accent),
            )),
            Event::Html(text) | Event::InlineHtml(text) => {
                spans.push(Span::styled(safe(text), fg(theme.muted)))
            }
            Event::FootnoteReference(name) => {
                spans.push(Span::styled(format!("[{name}]"), fg(theme.keyword)))
            }
            _ => {}
        }
    }
    Line::from(spans)
}
fn wrapped(line: Line<'static>, width: usize, locale: Locale) -> Vec<Line<'static>> {
    let mut writer = Writer::new(width.max(1), locale);
    for span in line.spans {
        prose(&mut writer, &span.content, span.style);
    }
    writer.finish()
}
fn box_line(writer: &mut Writer, line: Line<'static>, color: ratatui::style::Color, theme: Theme) {
    let width = writer.width.saturating_sub(4).max(1);
    let used = line.width().min(width);
    let mut spans = vec![Span::styled("│ ", fg(color))];
    spans.extend(line.spans);
    spans.push(Span::raw(" ".repeat(width - used)));
    spans.push(Span::styled(" │", fg(color)));
    writer.row(Line::from(spans).style(Style::default().bg(theme.raised)));
}
fn border(writer: &mut Writer, top: bool, title: &str, color: ratatui::style::Color) {
    let width = writer.width;
    if width < 6 {
        return;
    }
    let title = clipped(title, width.saturating_sub(6));
    let length = Line::from(title.as_str()).width();
    let text = if top {
        format!(
            "╭─ {title} {}╮",
            "─".repeat(width.saturating_sub(length + 5))
        )
    } else {
        format!("╰{}╯", "─".repeat(width.saturating_sub(2)))
    };
    writer.row(Line::styled(text, fg(color)));
}

fn code_card(writer: &mut Writer, text: &str, alias: &str, theme: Theme) {
    writer.blank();
    let language = Language::from_alias(alias);
    let count = text.lines().count();
    let title = format!(
        "{}  ·  {count} {}",
        if alias.is_empty() { "CODE" } else { alias },
        label(writer.locale, "lines", "dòng")
    );
    if writer.width < 16 {
        code(writer, text, language, theme);
        return;
    }
    border(writer, true, &title, theme.secondary);
    let digits = count.max(1).to_string().len();
    let inner_width = writer.width.saturating_sub(4 + digits + 3).max(1);
    for (index, raw) in text.lines().enumerate() {
        if writer.full() {
            break;
        }
        let prefix_end = raw.char_indices().nth(8192).map_or(raw.len(), |(b, _)| b);
        let prefix = &raw[..prefix_end];
        let spans = BasicSyntax.highlight_line(language, prefix, 0);
        let painted = crate::render::styled_line(
            prefix,
            0,
            0..0,
            0,
            usize::MAX / 2,
            4,
            crate::render::LinePaint {
                cell_start: 0,
                theme,
                highlights: &spans.spans,
                marks: &[],
            },
        );
        let mut inner = Writer::new(inner_width, writer.locale);
        for span in painted.spans {
            inner.push(&span.content, span.style);
        }
        inner.push(&raw[prefix_end..], fg(theme.text));
        let mut lines = inner.finish();
        if lines.is_empty() {
            lines.push(Line::default());
        }
        for (part, line) in lines.into_iter().enumerate() {
            if writer.full() {
                break;
            }
            let number = if part == 0 {
                format!("{:>digits$} │ ", index + 1)
            } else {
                format!("{:>digits$} │ ", "·")
            };
            let mut spans = vec![Span::styled(number, fg(theme.muted))];
            spans.extend(line.spans);
            box_line(writer, Line::from(spans), theme.line, theme);
        }
    }
    border(writer, false, "", theme.secondary);
    writer.blank();
}
fn table(writer: &mut Writer, events: &[Event<'_>], theme: Theme, meta: &Metadata) {
    let mut rows: Vec<Vec<Line<'static>>> = Vec::new();
    let mut row = Vec::new();
    let mut cell = Vec::new();
    let mut in_cell = false;
    for event in events {
        match event {
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_cell = false;
                row.push(inline(&cell, theme, meta));
            }
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                if !row.is_empty() {
                    rows.push(std::mem::take(&mut row));
                }
                if rows.len() >= 10_000 {
                    break;
                }
            }
            _ if in_cell => cell.push(event.clone()),
            _ => {}
        }
    }
    let count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if count == 0 {
        return;
    }
    if writer.width < 12 {
        for row in &rows {
            for cell in row {
                for span in &cell.spans {
                    writer.push(&span.content, span.style);
                }
                writer.push(" | ", fg(theme.muted));
            }
            writer.newline();
        }
        return;
    }
    let columns = count
        .min(12)
        .min((writer.width.saturating_sub(1) / 6).max(1));
    let available = writer.width.saturating_sub(columns * 3 + 1);
    let mut widths = vec![3; columns];
    for row in rows.iter().take(128) {
        for (index, cell) in row.iter().take(columns).enumerate() {
            widths[index] = widths[index].max(cell.width().min(32));
        }
    }
    while widths.iter().sum::<usize>() > available && widths.iter().any(|w| *w > 3) {
        let index = widths.iter().enumerate().max_by_key(|(_, w)| *w).unwrap().0;
        widths[index] -= 1;
    }
    let rule = |left: &str, middle: &str, right: &str| {
        format!(
            "{left}{}{right}",
            widths
                .iter()
                .map(|w| "─".repeat(w + 2))
                .collect::<Vec<_>>()
                .join(middle)
        )
    };
    writer.blank();
    writer.row(Line::styled(rule("╭", "┬", "╮"), fg(theme.line)));
    for (row_index, row) in rows.iter().enumerate() {
        if writer.full() {
            break;
        }
        let cells = widths
            .iter()
            .enumerate()
            .map(|(index, width)| {
                wrapped(
                    row.get(index).cloned().unwrap_or_default(),
                    *width,
                    writer.locale,
                )
            })
            .collect::<Vec<_>>();
        let height = cells.iter().map(Vec::len).max().unwrap_or(1).clamp(1, 8);
        for offset in 0..height {
            let mut spans = vec![Span::styled("│", fg(theme.line))];
            for (index, width) in widths.iter().enumerate() {
                let line = cells[index].get(offset).cloned().unwrap_or_default();
                let used = line.width().min(*width);
                spans.push(Span::raw(" "));
                spans.extend(line.spans.into_iter().map(|span| {
                    if row_index == 0 {
                        span.style(bold(theme.accent))
                    } else {
                        span
                    }
                }));
                spans.push(Span::raw(" ".repeat(width - used + 1)));
                spans.push(Span::styled("│", fg(theme.line)));
            }
            writer.row(
                Line::from(spans).style(Style::default().bg(if row_index % 2 == 0 {
                    theme.raised
                } else {
                    theme.panel
                })),
            );
        }
        if row_index == 0 {
            writer.row(Line::styled(rule("├", "┼", "┤"), fg(theme.line)));
        }
    }
    writer.row(Line::styled(rule("╰", "┴", "╯"), fg(theme.line)));
    if columns < count {
        writer.row(Line::styled(
            label(
                writer.locale,
                "… Additional columns remain in source.",
                "… Các cột còn lại được giữ trong nguồn.",
            ),
            fg(theme.muted),
        ));
    }
    writer.blank();
}

pub(super) fn render(writer: &mut Writer, text: &str, theme: Theme) {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM;
    let mut events = Parser::new_ext(text, options).peekable();
    let mut meta = Metadata::default();
    meta.stats.words = text.unicode_words().count();
    let mut style = fg(theme.text);
    let mut stack = Vec::new();
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut link_ids = Vec::new();
    while let Some(event) = events.next() {
        if writer.full() {
            break;
        }
        meta.observe(&event);
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let contents = collect(&mut events, TagEnd::Heading(level), &mut meta);
                let line = inline(&contents, theme, &meta);
                let title = line
                    .spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>();
                writer.blank();
                if writer.outline.len() < 2000 {
                    writer.outline.push(OutlineEntry {
                        level: level as usize,
                        title: clipped(&title, 180),
                        row: writer.lines.len(),
                    });
                }
                if level as usize == 1 && writer.width >= 20 {
                    border(
                        writer,
                        true,
                        label(writer.locale, "DOCUMENT", "TÀI LIỆU"),
                        theme.accent,
                    );
                    for line in wrapped(
                        Line::styled(title.clone(), bold(theme.accent)),
                        writer.width.saturating_sub(4),
                        writer.locale,
                    ) {
                        box_line(writer, line, theme.accent, theme);
                    }
                    border(writer, false, "", theme.accent);
                } else {
                    let prefix = format!("H{}  ", level as usize);
                    writer.push(&prefix, bold(theme.secondary));
                    writer.push(
                        &title,
                        bold(if level as usize == 2 {
                            theme.accent
                        } else {
                            theme.text
                        }),
                    );
                    writer.newline();
                    if level as usize == 2 {
                        writer.row(Line::styled("─".repeat(writer.width), fg(theme.line)));
                    }
                }
                writer.blank();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let alias = match kind {
                    CodeBlockKind::Fenced(ref info) => info.split_whitespace().next().unwrap_or(""),
                    CodeBlockKind::Indented => "",
                }
                .to_string();
                let contents = collect(&mut events, TagEnd::CodeBlock, &mut meta);
                let raw = contents
                    .iter()
                    .filter_map(|event| {
                        if let Event::Text(s) = event {
                            Some(s.as_ref())
                        } else {
                            None
                        }
                    })
                    .collect::<String>();
                code_card(writer, &raw, &alias, theme);
            }
            Event::Start(Tag::Table(_)) => {
                let contents = collect(&mut events, TagEnd::Table, &mut meta);
                table(writer, &contents, theme, &meta);
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                let contents = collect(&mut events, TagEnd::BlockQuote(kind), &mut meta);
                let title = kind
                    .map(|k| format!("{k:?}").to_uppercase())
                    .unwrap_or_else(|| label(writer.locale, "QUOTE", "TRÍCH DẪN").into());
                let color = if title == "WARNING" || title == "CAUTION" {
                    theme.warm
                } else {
                    theme.keyword
                };
                writer.blank();
                border(writer, true, &title, color);
                for line in wrapped(
                    inline(&contents, theme, &meta),
                    writer.width.saturating_sub(4),
                    writer.locale,
                ) {
                    box_line(writer, line, color, theme);
                }
                border(writer, false, "", color);
                writer.blank();
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                let contents = collect(&mut events, TagEnd::Image, &mut meta);
                let alt = inline(&contents, theme, &meta);
                writer.blank();
                border(
                    writer,
                    true,
                    label(writer.locale, "IMAGE REFERENCE", "THAM CHIẾU ẢNH"),
                    theme.secondary,
                );
                for line in wrapped(alt, writer.width.saturating_sub(4), writer.locale) {
                    box_line(writer, line, theme.line, theme);
                }
                for line in wrapped(
                    Line::styled(clipped(&dest_url, 512), fg(theme.muted)),
                    writer.width.saturating_sub(4),
                    writer.locale,
                ) {
                    box_line(writer, line, theme.line, theme);
                }
                border(writer, false, "", theme.secondary);
                writer.blank();
            }
            Event::Start(tag) => {
                stack.push(style);
                match tag {
                    Tag::Strong => style = style.add_modifier(Modifier::BOLD),
                    Tag::Emphasis => style = style.add_modifier(Modifier::ITALIC),
                    Tag::Strikethrough => style = style.add_modifier(Modifier::CROSSED_OUT),
                    Tag::List(start) => {
                        writer.blank();
                        lists.push(start);
                    }
                    Tag::Item => {
                        if !writer.spans.is_empty() {
                            writer.newline();
                        }
                        let indent = "  ".repeat(lists.len().saturating_sub(1).min(20));
                        let bullet = if let Some(Some(n)) = lists.last_mut() {
                            let s = format!("{n}. ");
                            *n += 1;
                            s
                        } else {
                            "• ".into()
                        };
                        writer.push(&format!("{indent}{bullet}"), fg(theme.secondary));
                    }
                    Tag::Link { dest_url, .. } => {
                        style = style.fg(theme.secondary).add_modifier(Modifier::UNDERLINED);
                        link_ids.push(meta.indices.get(dest_url.as_ref()).copied());
                    }
                    Tag::FootnoteDefinition(ref name) => {
                        writer.blank();
                        writer.push(&format!("[{name}] "), bold(theme.keyword));
                    }
                    _ => {}
                }
            }
            Event::End(tag) => {
                match tag {
                    TagEnd::Paragraph if lists.is_empty() => writer.blank(),
                    TagEnd::Item => writer.newline(),
                    TagEnd::List(_) => {
                        lists.pop();
                        writer.blank();
                    }
                    TagEnd::Link => {
                        if let Some(Some(index)) = link_ids.pop() {
                            writer.push(&format!(" [{index}]"), fg(theme.muted));
                        }
                    }
                    _ => {}
                }
                style = stack.pop().unwrap_or(fg(theme.text));
            }
            Event::Text(text) => prose(writer, &text, style),
            Event::Code(text) => writer.push(&format!(" {text} "), fg(theme.warm).bg(theme.raised)),
            Event::SoftBreak => prose(writer, " ", style),
            Event::HardBreak => writer.newline(),
            Event::Rule => {
                writer.blank();
                writer.row(Line::styled("─".repeat(writer.width), fg(theme.line)));
                writer.blank();
            }
            Event::TaskListMarker(done) => writer.push(
                if done { "[x] " } else { "[ ] " },
                fg(if done { theme.accent } else { theme.warm }),
            ),
            Event::Html(html) | Event::InlineHtml(html) => writer.push(&html, fg(theme.muted)),
            Event::FootnoteReference(name) => writer.push(&format!("[{name}]"), fg(theme.keyword)),
            _ => {}
        }
    }
    if !meta.links.is_empty() && !writer.full() {
        writer.blank();
        writer.row(Line::styled(
            label(writer.locale, "LINK REFERENCES", "THAM CHIẾU LIÊN KẾT"),
            bold(theme.secondary),
        ));
        for (index, url) in meta.links.iter().enumerate() {
            writer.push(&format!("[{}] {url}\n", index + 1), fg(theme.muted));
        }
    }
    writer.stats = Some(meta.stats);
}
