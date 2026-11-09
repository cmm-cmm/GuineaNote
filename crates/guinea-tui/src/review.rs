//! Cached, read-only review. Parsing and wrapping never run in the input loop.
use crate::{
    i18n::Locale,
    render::{LinePaint, safe, styled_line},
    theme::Theme,
};
use guinea_core::{Document, TextBuffer};
use guinea_syntax::{BasicSyntax, Language, SyntaxProvider};
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
mod markdown;
mod value;

const MAX_ROWS: usize = 100_000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    Source,
    Split,
    Review,
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
#[derive(Default)]
pub struct OutlineEntry {
    pub level: usize,
    pub title: String,
    pub row: usize,
}
#[derive(Default)]
pub struct MarkdownStats {
    pub words: usize,
    pub headings: usize,
    pub tasks: usize,
    pub completed_tasks: usize,
    pub code_blocks: usize,
    pub tables: usize,
    pub links: usize,
}
struct Content {
    lines: Vec<Line<'static>>,
    outline: Vec<OutlineEntry>,
    stats: Option<MarkdownStats>,
}
struct Rendered {
    key: Key,
    lines: Vec<Line<'static>>,
    error: bool,
    outline: Vec<OutlineEntry>,
    stats: Option<MarkdownStats>,
}

#[derive(Default)]
pub struct Review {
    pub mode: ViewMode,
    pub top: usize,
    pub width: usize,
    pub column: usize,
    pub outline_visible: bool,
    pub outline_focus: bool,
    pub outline_selected: usize,
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
    pub fn outline(&self) -> &[OutlineEntry] {
        self.cached.as_ref().map_or(&[], |c| c.outline.as_slice())
    }
    pub fn stats(&self) -> Option<&MarkdownStats> {
        self.cached.as_ref().and_then(|c| c.stats.as_ref())
    }
    pub fn jump_heading(&mut self) {
        if let Some(entry) = self.outline().get(self.outline_selected) {
            self.top = entry.row;
        }
        self.outline_focus = false;
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
                        outline: Vec::new(),
                        stats: None,
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
                let result = render_content(
                    &text,
                    key.language,
                    path.as_deref(),
                    key.width,
                    Theme::new(basic),
                    locale,
                    key.column,
                );
                let (content, error) = match result {
                    Ok(content) => (content, false),
                    Err(error) => (
                        Content {
                            lines: vec![Line::styled(
                                safe(&error),
                                Style::default().fg(Theme::new(basic).red),
                            )],
                            outline: vec![],
                            stats: None,
                        },
                        true,
                    ),
                };
                let _ = sender.send(Rendered {
                    key,
                    lines: content.lines,
                    error,
                    outline: content.outline,
                    stats: content.stats,
                });
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
    outline: Vec<OutlineEntry>,
    stats: Option<MarkdownStats>,
}
impl Writer {
    fn new(width: usize, locale: Locale) -> Self {
        Self {
            locale,
            outline: vec![],
            stats: None,
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
    fn row(&mut self, line: Line<'static>) {
        if !self.spans.is_empty() {
            self.newline();
        }
        if !self.full() {
            self.lines.push(line);
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

fn render_content(
    text: &str,
    language: Language,
    path: Option<&std::path::Path>,
    width: usize,
    theme: Theme,
    locale: Locale,
    column: usize,
) -> Result<Content, String> {
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
    let outline = if language == Language::Markdown {
        std::mem::take(&mut writer.outline)
    } else {
        vec![]
    };
    let stats = if language == Language::Markdown {
        writer.stats.take()
    } else {
        None
    };
    Ok(Content {
        lines: writer.finish(),
        outline,
        stats,
    })
}

#[cfg(test)]
fn render(
    text: &str,
    language: Language,
    path: Option<&std::path::Path>,
    width: usize,
    theme: Theme,
    locale: Locale,
    column: usize,
) -> Result<Vec<Line<'static>>, String> {
    render_content(text, language, path, width, theme, locale, column).map(|content| content.lines)
}
fn markdown(writer: &mut Writer, text: &str, theme: Theme) {
    markdown::render(writer, text, theme);
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
    fn rich_markdown_has_navigable_outline_cards_stats_and_bounded_unicode_tables() {
        let text = "# Title\n\n## Details\n\n- [x] done\n- [ ] todo\n\n> [!NOTE]\n> A useful note.\n\n```rust\nfn main() { let tên = \"Việt\"; }\n```\n\n| Name | Value |\n|---|---|\n| 界 | **bold** |\n\n[Read](https://example.com)\n";
        for width in [24, 50, 100] {
            let result = render_content(
                text,
                Language::Markdown,
                None,
                width,
                Theme::new(false),
                Locale::En,
                0,
            )
            .unwrap();
            let stats = result.stats.unwrap();
            assert_eq!(
                (
                    stats.headings,
                    stats.tasks,
                    stats.completed_tasks,
                    stats.code_blocks,
                    stats.tables,
                    stats.links
                ),
                (2, 2, 1, 1, 1, 1)
            );
            assert_eq!(result.outline[1].title, "Details");
            assert!(result.outline[1].row < result.lines.len());
            assert!(
                result.lines.iter().all(|line| line.width() <= width),
                "width {width}"
            );
            let output = result
                .lines
                .iter()
                .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
                .collect::<String>();
            for expected in ["DOCUMENT", "NOTE", "LINK REFERENCES", "Name", "Việt"] {
                assert!(output.contains(expected), "{expected}");
            }
        }
    }
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
