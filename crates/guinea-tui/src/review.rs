//! Cached, read-only review. Parsing and wrapping never run in the input loop.
use crate::{
    i18n::Locale,
    render::{LinePaint, safe, styled_line},
    theme::Theme,
};
use guinea_core::{Document, TextBuffer};
use guinea_syntax::{BasicSyntax, Language, SyntaxProvider};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

mod data;
mod value;

const MAX_ROWS: usize = 100_000;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    Source,
    Split,
    Review,
}
impl ViewMode {
    pub fn next(self) -> Self {
        match self {
            Self::Source => Self::Split,
            Self::Split => Self::Review,
            Self::Review => Self::Source,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    id: u64,
    revision: u64,
    width: usize,
    language: Language,
    locale: Locale,
    column: usize,
    json_lines: bool,
}
struct Rendered {
    key: Key,
    lines: Vec<Line<'static>>,
    error: bool,
}

#[derive(Default)]
pub struct Review {
    pub mode: ViewMode,
    pub top: usize,
    pub width: usize,
    pub column: usize,
    desired: Option<(Key, Instant)>,
    pending: Option<Receiver<Rendered>>,
    cached: Option<Rendered>,
}

fn json_lines(document: &Document) -> bool {
    document
        .path
        .as_ref()
        .and_then(|p| p.extension())
        .and_then(|s| s.to_str())
        .is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "jsonl" | "ndjson"))
}

impl Review {
    pub fn needs_poll(&self, document: &Document, locale: Locale) -> bool {
        self.busy()
            || (self.mode != ViewMode::Source
                && self.width > 0
                && self.cached.as_ref().is_none_or(|cache| {
                    cache.key
                        != Key {
                            id: document.id,
                            revision: document.revision(),
                            width: self.width,
                            language: crate::app::document_language(document),
                            locale,
                            column: self.column,
                            json_lines: json_lines(document),
                        }
                }))
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.desired.is_some()
    }
    pub fn ready(&self) -> bool {
        self.cached.is_some()
    }
    pub fn error(&self) -> bool {
        self.cached.as_ref().is_some_and(|cache| cache.error)
    }
    pub fn lines(&self) -> &[Line<'static>] {
        self.cached
            .as_ref()
            .map_or(&[], |cache| cache.lines.as_slice())
    }
    pub fn scroll(&mut self, delta: isize, height: usize) {
        self.top = self
            .top
            .saturating_add_signed(delta)
            .min(self.lines().len().saturating_sub(height));
    }
    pub fn poll(&mut self, document: &Document, basic: bool, locale: Locale) -> bool {
        let mut redraw = false;
        let key = Key {
            id: document.id,
            revision: document.revision(),
            width: self.width.max(1),
            language: crate::app::document_language(document),
            locale,
            column: self.column,
            json_lines: json_lines(document),
        };
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(rendered) => {
                    self.pending = None;
                    if rendered.key == key {
                        self.cached = Some(rendered);
                        redraw = true;
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.cached = Some(Rendered {
                        key,
                        lines: vec![Line::from("Review worker failed; source retained")],
                        error: true,
                    });
                    redraw = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.mode == ViewMode::Source || self.width == 0 {
            self.desired = None;
            return redraw;
        }
        if self.cached.as_ref().is_some_and(|cache| cache.key == key) {
            self.desired = None;
            return redraw;
        }
        if self.desired.is_none_or(|(old, _)| old != key) {
            self.desired = Some((key, Instant::now()));
        }
        let initial = self.cached.is_none();
        if self.pending.is_none()
            && (initial
                || self
                    .desired
                    .is_some_and(|(_, since)| since.elapsed() >= Duration::from_millis(40)))
        {
            let buffer = document.snapshot_buffer();
            let path = document.path.clone();
            let (sender, receiver) = mpsc::channel();
            self.pending = Some(receiver);
            thread::spawn(move || {
                let text = buffer.text();
                let result = render(
                    &text,
                    key.language,
                    path.as_deref(),
                    key.width,
                    Theme::new(basic),
                    locale,
                    key.column,
                );
                let (lines, error) = match result {
                    Ok(lines) => (lines, false),
                    Err(error) => (
                        vec![Line::styled(
                            safe(&error),
                            Style::default().fg(Theme::new(basic).red),
                        )],
                        true,
                    ),
                };
                let _ = sender.send(Rendered { key, lines, error });
            });
        }
        redraw
    }
}

/// Coalesces spans and bounds output; long paragraphs cannot allocate one span
/// per character. Terminal cells, rather than bytes, determine wrapping.
struct Writer {
    lines: Vec<Line<'static>>,
    spans: Vec<Span<'static>>,
    cell: usize,
    width: usize,
    locale: Locale,
}
impl Writer {
    fn new(width: usize, locale: Locale) -> Self {
        Self {
            locale,
            lines: vec![],
            spans: vec![],
            cell: 0,
            width: width.max(1),
        }
    }
    fn full(&self) -> bool {
        self.lines.len() >= MAX_ROWS
    }
    fn newline(&mut self) {
        if self.full() {
            return;
        }
        self.lines.push(Line::from(std::mem::take(&mut self.spans)));
        self.cell = 0;
    }
    fn blank(&mut self) {
        if !self.spans.is_empty() {
            self.newline();
        }
        if self.lines.last().is_some_and(|l| !l.spans.is_empty()) {
            self.newline();
        }
    }
    fn push(&mut self, text: &str, style: Style) {
        for g in text.graphemes(true) {
            if self.full() {
                return;
            }
            if g == "\n" || g == "\r\n" {
                self.newline();
                continue;
            }
            let (display, cells) = guinea_core::coordinates::display_grapheme(g, self.cell, 4);
            if self.cell + cells > self.width && self.cell > 0 {
                self.newline();
            }
            if cells > self.width {
                continue;
            }
            if let Some(last) = self.spans.last_mut().filter(|span| span.style == style) {
                last.content.to_mut().push_str(&display);
            } else {
                self.spans.push(Span::styled(display, style));
            }
            self.cell += cells;
        }
    }
    fn finish(mut self) -> Vec<Line<'static>> {
        if !self.spans.is_empty() {
            self.newline();
        }
        if self.full() {
            self.lines.push(Line::from(
                self.locale
                    .tr("… Review limited to 100,000 display rows; source is complete.")
                    .into_owned(),
            ));
        }
        self.lines
    }
}

fn code(writer: &mut Writer, text: &str, language: Language, theme: Theme) {
    for raw in text.lines() {
        if writer.full() {
            break;
        }
        // Bound lexical work for enormous individual lines, then wrap the tail.
        let end = raw
            .char_indices()
            .nth(8192)
            .map_or(raw.len(), |(byte, _)| byte);
        let prefix = &raw[..end];
        let highlights = BasicSyntax.highlight_line(language, prefix, 0);
        let line = styled_line(
            prefix,
            0,
            0..0,
            0,
            usize::MAX / 2,
            4,
            LinePaint {
                cell_start: 0,
                theme,
                highlights: &highlights.spans,
            },
        );
        for span in line.spans {
            writer.push(&span.content, span.style);
        }
        writer.push(&raw[end..], Style::default().fg(theme.text));
        writer.newline();
    }
}

pub(crate) fn render(
    text: &str,
    language: Language,
    path: Option<&std::path::Path>,
    width: usize,
    theme: Theme,
    locale: Locale,
    column: usize,
) -> Result<Vec<Line<'static>>, String> {
    let mut writer = Writer::new(width, locale);
    let title = Style::default().fg(theme.mint).add_modifier(Modifier::BOLD);
    match language {
        Language::Markdown => markdown(&mut writer, text, theme),
        Language::Json => {
            let extension = path
                .and_then(|p| p.extension())
                .and_then(|p| p.to_str())
                .unwrap_or("");
            if matches!(extension.to_ascii_lowercase().as_str(), "jsonl" | "ndjson") {
                for (index, line) in text
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| !line.trim().is_empty())
                {
                    let value: serde_json::Value = serde_json::from_str(line)
                        .map_err(|e| format!("JSON record {}: {e}", index + 1))?;
                    if !writer.full() {
                        writer.push(
                            &format!(
                                "{} {}\n",
                                if locale == Locale::Vi {
                                    "Bản ghi"
                                } else {
                                    "Record"
                                },
                                index + 1
                            ),
                            title,
                        );
                        code(
                            &mut writer,
                            &serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?,
                            Language::Json,
                            theme,
                        );
                    }
                }
            } else {
                let value: serde_json::Value =
                    serde_json::from_str(text).map_err(|e| format!("JSON: {e}"))?;
                writer.push(&writer.locale.tr("JSON · formatted view\n\n"), title);
                code(
                    &mut writer,
                    &serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?,
                    Language::Json,
                    theme,
                );
            }
        }
        Language::Xml => xml(&mut writer, text, theme)?,
        Language::Csv | Language::Tsv => {
            table(&mut writer, text, language == Language::Tsv, theme, column)?
        }
        Language::Toml => {
            let value: toml::Value = toml::from_str(text).map_err(|e| format!("TOML: {e}"))?;
            writer.push(&writer.locale.tr("TOML · formatted view\n\n"), title);
            code(
                &mut writer,
                &toml::to_string_pretty(&value).map_err(|e| e.to_string())?,
                Language::Toml,
                theme,
            );
        }
        Language::Yaml => data::yaml(&mut writer, text, theme)?,
        Language::Json5 => data::json5(&mut writer, text, theme)?,
        Language::Notebook => data::notebook(&mut writer, text, theme)?,
        Language::Ini | Language::Env | Language::Config | Language::Properties => {
            data::config(&mut writer, text, language, theme)?
        }
        _ => {
            writer.push(
                &format!(
                    "{} · {}\n\n",
                    language.label(),
                    if locale == Locale::Vi {
                        "bản xem mã nguồn"
                    } else {
                        "source review"
                    }
                ),
                title,
            );
            code(&mut writer, text, language, theme);
        }
    }
    Ok(writer.finish())
}

fn markdown(writer: &mut Writer, text: &str, theme: Theme) {
    let normal = Style::default().fg(theme.text);
    let mut style = normal;
    let mut stack = Vec::new();
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut code_language = None;
    let mut link = None;
    for event in Parser::new_ext(
        text,
        Options::ENABLE_TABLES
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_FOOTNOTES,
    ) {
        if writer.full() {
            break;
        }
        match event {
            Event::Start(tag) => {
                stack.push(style);
                match tag {
                    Tag::Heading { level, .. } => {
                        writer.blank();
                        style = Style::default().fg(theme.mint).add_modifier(Modifier::BOLD);
                        writer.push(
                            &format!("{} ", "▌".repeat(if level as usize <= 2 { 2 } else { 1 })),
                            style,
                        );
                    }
                    Tag::Paragraph => {}
                    Tag::Strong => style = style.add_modifier(Modifier::BOLD),
                    Tag::Emphasis => style = style.add_modifier(Modifier::ITALIC),
                    Tag::Strikethrough => style = style.add_modifier(Modifier::CROSSED_OUT),
                    Tag::BlockQuote(_) => {
                        writer.blank();
                        writer.push("│ ", Style::default().fg(theme.purple));
                        style = style.fg(theme.muted);
                    }
                    Tag::List(start) => {
                        writer.blank();
                        lists.push(start);
                    }
                    Tag::Item => {
                        if !writer.spans.is_empty() {
                            writer.newline();
                        }
                        let indent = "  ".repeat(lists.len().saturating_sub(1).min(20));
                        let bullet = if let Some(Some(number)) = lists.last_mut() {
                            let label = format!("{number}. ");
                            *number += 1;
                            label
                        } else {
                            "• ".into()
                        };
                        writer.push(
                            &format!("{indent}{bullet}"),
                            Style::default().fg(theme.blue),
                        );
                    }
                    Tag::CodeBlock(kind) => {
                        writer.blank();
                        let alias = match kind {
                            CodeBlockKind::Fenced(ref info) => {
                                info.split_whitespace().next().unwrap_or("")
                            }
                            CodeBlockKind::Indented => "",
                        };
                        code_language = Some(Language::from_alias(alias));
                        writer.push(
                            &format!("┌─ {}\n", if alias.is_empty() { "code" } else { alias }),
                            Style::default().fg(theme.blue),
                        );
                    }
                    Tag::Link { dest_url, .. } => {
                        link = Some(dest_url.to_string());
                        style = style.fg(theme.blue).add_modifier(Modifier::UNDERLINED);
                    }
                    Tag::Image { .. } => writer.push("[image] ", Style::default().fg(theme.muted)),
                    Tag::Table(_) => {
                        writer.blank();
                    }
                    Tag::TableHead | Tag::TableRow => {
                        if !writer.spans.is_empty() {
                            writer.newline();
                        }
                    }
                    Tag::TableCell => writer.push("│ ", Style::default().fg(theme.faint)),
                    _ => {}
                }
            }
            Event::End(tag) => {
                match tag {
                    TagEnd::Paragraph if lists.is_empty() => writer.blank(),
                    TagEnd::Heading(_) | TagEnd::Item | TagEnd::TableHead | TagEnd::TableRow => {
                        writer.newline()
                    }
                    TagEnd::List(_) => {
                        lists.pop();
                        writer.blank();
                    }
                    TagEnd::CodeBlock => {
                        code_language = None;
                        writer.push("└─\n", Style::default().fg(theme.blue));
                        writer.blank();
                    }
                    TagEnd::Link => {
                        if let Some(url) = link.take() {
                            writer.push(&format!(" ({url})"), Style::default().fg(theme.muted));
                        }
                    }
                    TagEnd::TableCell => writer.push("  ", normal),
                    _ => {}
                }
                style = stack.pop().unwrap_or(normal);
            }
            Event::Text(text) => {
                if let Some(language) = code_language {
                    code(writer, &text, language, theme);
                } else {
                    writer.push(&text, style);
                }
            }
            Event::Code(text) => {
                writer.push(&text, Style::default().fg(theme.amber).bg(theme.raised))
            }
            Event::SoftBreak => writer.push(" ", style),
            Event::HardBreak => writer.newline(),
            Event::Rule => {
                writer.blank();
                writer.push(&"─".repeat(writer.width), Style::default().fg(theme.line));
                writer.newline();
            }
            Event::TaskListMarker(checked) => writer.push(
                if checked { "[x] " } else { "[ ] " },
                Style::default().fg(theme.mint),
            ),
            Event::Html(html) | Event::InlineHtml(html) => {
                writer.push(&html, Style::default().fg(theme.muted))
            }
            Event::FootnoteReference(label) => {
                writer.push(&format!("[{label}]"), Style::default().fg(theme.purple))
            }
            _ => {}
        }
    }
}

fn xml(writer: &mut Writer, text: &str, theme: Theme) -> Result<(), String> {
    use quick_xml::events::Event as Xml;
    let mut reader = quick_xml::Reader::from_str(text);
    reader.config_mut().check_end_names = true;
    let mut depth: usize = 0;
    let mut roots = 0;
    writer.push(
        &writer.locale.tr("XML · structure view\n\n"),
        Style::default().fg(theme.mint).add_modifier(Modifier::BOLD),
    );
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("XML at byte {}: {e}", reader.error_position()))?;
        let mut output = String::new();
        let mut color = theme.text;
        match event {
            Xml::Eof => break,
            Xml::Start(ref e) | Xml::Empty(ref e) => {
                if depth == 0 {
                    roots += 1;
                }
                // Attribute parsing validates duplicate names and quoting.
                for attribute in e.attributes() {
                    attribute.map_err(|e| format!("XML attribute: {e}"))?;
                }
                output = format!(
                    "{}<{}{}>",
                    "  ".repeat(depth.min(40)),
                    e.as_ref(),
                    if matches!(event, Xml::Empty(_)) {
                        "/"
                    } else {
                        ""
                    }
                );
                if matches!(event, Xml::Start(_)) {
                    depth += 1;
                }
                color = theme.blue;
            }
            Xml::End(e) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or("XML: unexpected closing element")?;
                output = format!("{}</{}>", "  ".repeat(depth.min(40)), e.as_ref());
                color = theme.blue;
            }
            Xml::Text(e) => {
                let value = e.as_ref();
                if !value.trim().is_empty() {
                    if depth == 0 {
                        return Err("XML: text outside the root element".into());
                    }
                    output = format!("{}{}", "  ".repeat(depth.min(40)), value.trim());
                }
            }
            Xml::CData(e) => {
                output = format!("{}<![CDATA[{}]]>", "  ".repeat(depth.min(40)), e.as_ref())
            }
            Xml::Comment(e) => {
                output = format!("{}<!--{}-->", "  ".repeat(depth.min(40)), e.as_ref());
                color = theme.muted;
            }
            Xml::DocType(_) => {
                output = writer
                    .locale
                    .tr("[DOCTYPE retained in source; external entities are not loaded]")
                    .into_owned()
            }
            Xml::GeneralRef(e) => {
                output = format!("{}&{};", "  ".repeat(depth.min(40)), e.as_ref())
            }
            _ => {}
        }
        if !output.is_empty() && !writer.full() {
            writer.push(&output, Style::default().fg(color));
            writer.newline();
        }
    }
    if depth != 0 || roots != 1 {
        return Err("XML: expected one complete root element".into());
    }
    Ok(())
}

fn cell(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for g in text.graphemes(true) {
        let printable = if g.chars().any(char::is_control) {
            "↵".into()
        } else {
            safe(g)
        };
        let cells = Line::from(printable.as_str()).width();
        if used + cells > width {
            while used >= width && !out.is_empty() {
                let (byte, last) = out.grapheme_indices(true).next_back().unwrap();
                used -= Line::from(last).width();
                out.truncate(byte);
            }
            out.push('…');
            used += 1;
            break;
        }
        out.push_str(&printable);
        used += cells;
    }
    out.push_str(&" ".repeat(width.saturating_sub(used)));
    out
}

fn table(
    writer: &mut Writer,
    text: &str,
    tsv: bool,
    theme: Theme,
    column: usize,
) -> Result<(), String> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(if tsv { b'\t' } else { b',' })
        .from_reader(text.as_bytes());
    let headers = reader.headers().map_err(|e| format!("Table: {e}"))?.clone();
    let count = headers.len();
    let column = column.min(count.saturating_sub(1));
    let columns = count
        .saturating_sub(column)
        .min(12)
        .min((writer.width.saturating_sub(8) / 7).max(1));
    let field_width = (writer.width.saturating_sub(9 + columns * 3) / columns.max(1)).clamp(3, 48);
    let style = Style::default().fg(theme.mint).add_modifier(Modifier::BOLD);
    writer.push(
        &format!(
            "{} · {} · {count} {}\n",
            if tsv { "TSV" } else { "CSV" },
            if writer.locale == Locale::Vi {
                "bản ghi đầu là tiêu đề"
            } else {
                "first record is the header"
            },
            if writer.locale == Locale::Vi {
                "cột"
            } else {
                "columns"
            }
        ),
        style,
    );
    if columns < count || column > 0 {
        writer.push(
            &if writer.locale == Locale::Vi {
                format!(
                    "Cột {}–{}/{count} · ←→ duyệt; nguồn vẫn giữ đầy đủ\n",
                    column + 1,
                    column + columns
                )
            } else {
                format!(
                    "Columns {}–{}/{count} · ←→ browse; source retains all fields\n",
                    column + 1,
                    column + columns
                )
            },
            Style::default().fg(theme.amber),
        );
    }
    let table_row = |fields: &csv::StringRecord, label: String| -> String {
        format!(
            "{label:>6} │ {} │",
            fields
                .iter()
                .skip(column)
                .take(columns)
                .map(|s| cell(s, field_width))
                .collect::<Vec<_>>()
                .join(" │ ")
        )
    };
    writer.push(&table_row(&headers, "#".into()), style);
    writer.newline();
    writer.push(&"─".repeat(writer.width), Style::default().fg(theme.line));
    writer.newline();
    let mut rows = 0;
    for record in reader.records() {
        let record = record.map_err(|e| format!("Table: {e}"))?;
        rows += 1;
        if !writer.full() {
            writer.push(
                &table_row(&record, rows.to_string()),
                Style::default().fg(if rows % 2 == 0 {
                    theme.muted
                } else {
                    theme.text
                }),
            );
            writer.newline();
        }
    }
    // Keep row count visible at the top, even when output is capped.
    writer.lines.insert(
        0,
        Line::styled(
            if writer.locale == Locale::Vi {
                format!("{rows} dòng dữ liệu · {count} cột")
            } else {
                format!("{rows} data rows · {count} columns")
            },
            style,
        ),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_results_locale_and_resize_never_replace_source() {
        let mut d = Document::from_text("{\"old\": 1}");
        d.path = Some("data.json".into());
        let mut review = Review {
            mode: ViewMode::Split,
            width: 60,
            ..Review::default()
        };
        review.poll(&d, false, Locale::En);
        d.replace_range(0..d.buffer().len_chars(), "{\"latest\": 42}")
            .unwrap();
        review.width = 33;
        let deadline = Instant::now() + Duration::from_secs(5);
        while review.needs_poll(&d, Locale::Vi) {
            review.poll(&d, false, Locale::Vi);
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(2));
        }
        let key = review.cached.as_ref().unwrap().key;
        assert_eq!(
            (key.revision, key.width, key.locale),
            (d.revision(), 33, Locale::Vi)
        );
        assert!(!review.error());
        let rendered = review
            .lines()
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect::<String>();
        assert!(rendered.contains("latest"));
        assert!(!rendered.contains("old"));
        assert!(rendered.contains("bản xem"));
        assert_eq!(d.text(), "{\"latest\": 42}");
    }
    #[test]
    fn table_cells_remain_aligned_and_later_columns_are_reachable() {
        assert_eq!(Line::from(cell("abcdef", 3)).width(), 3);
        assert_eq!(Line::from(cell("界界界", 3)).width(), 3);
        let lines = render(
            "first,second,third\na,b,c\n",
            Language::Csv,
            None,
            50,
            Theme::new(false),
            Locale::En,
            2,
        )
        .unwrap();
        let text = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect::<String>();
        assert!(text.contains("third"));
        assert!(!text.contains("second"));
    }
    fn plain(text: &str, language: Language, path: &str) -> Result<String, String> {
        render(
            text,
            language,
            Some(std::path::Path::new(path)),
            100,
            Theme::new(false),
            Locale::En,
            0,
        )
        .map(|lines| {
            lines
                .iter()
                .map(|l| {
                    l.spans
                        .iter()
                        .map(|s| s.content.as_ref())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
    }
    #[test]
    fn markdown_formats_blocks_and_never_emits_terminal_escape() {
        let rendered = plain("# Title\n\n**bold** and `code`\n\n- [x] done\n\n```rust\nlet x = 1;\n```\n\n[link](https://example.com)\n\n\u{1b}[31m", Language::Markdown, "x.md").unwrap();
        for expected in ["Title", "bold", "[x]", "let x = 1;", "https://example.com"] {
            assert!(rendered.contains(expected));
        }
        assert!(!rendered.contains("**"));
        assert!(!rendered.contains('\u{1b}'));
    }
    #[test]
    fn structured_formats_report_errors_and_preserve_source_semantics() {
        let text = r#"{"z":[1,true,null],"a":"Việt"}"#;
        let rendered = plain(text, Language::Json, "a.json").unwrap();
        assert!(rendered.contains("  \"z\": ["));
        assert!(rendered.find("\"z\"") < rendered.find("\"a\""));
        assert!(plain("{bad}", Language::Json, "x.json").is_err());
        assert!(
            plain("{}\n{bad}", Language::Json, "x.jsonl")
                .unwrap_err()
                .contains("record 2")
        );
        assert!(
            plain("<a><b/></a>", Language::Xml, "x.xml")
                .unwrap()
                .contains("  <b/>")
        );
        for invalid in ["<a><b></a>", "<a>", "<a/><b/>"] {
            assert!(plain(invalid, Language::Xml, "x.xml").is_err(), "{invalid}");
        }
        assert!(
            plain(
                "name,value\n\"Việt, Nam\",\"a\nb\"\n",
                Language::Csv,
                "x.csv"
            )
            .unwrap()
            .contains("1 data rows")
        );
        assert!(plain("a,b\n1,2,3\n", Language::Csv, "x.csv").is_err());
        assert!(
            plain("a\tb\n界\t2", Language::Tsv, "x.tsv")
                .unwrap()
                .contains("界")
        );
    }
}
