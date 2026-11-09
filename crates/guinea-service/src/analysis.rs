//! Inert structured review shared by Agent transports and the TUI JSON parser.
use crate::{Error, PAGE_ITEMS, Result};
use guinea_syntax::Language;
use serde_json::{Value, json};
pub fn parse_json(text: &str) -> std::result::Result<Value, serde_json::Error> {
    serde_json::from_str(text)
}
pub fn analyze(text: &str, language: Language, offset: usize, limit: usize) -> Result<Value> {
    if limit == 0 || limit > PAGE_ITEMS {
        return Err(Error::new(
            "LIMIT_EXCEEDED",
            "Analysis page limit is 1..200",
        ));
    }
    let mut entries = Vec::new();
    let mut count = 0;
    // Construct excerpts only for the requested page. A wide root array can
    // contain millions of entries; allocating discarded row objects dominates
    // analysis time even though the actual response contains at most 200.
    macro_rules! add {
        ($entry:expr $(,)?) => {{
            if count >= offset && entries.len() < limit {
                entries.push($entry);
            }
            count += 1;
        }};
    }
    match language {
        Language::Markdown => {
            use pulldown_cmark::{Event, Parser, Tag, TagEnd};
            let mut heading = None;
            let mut heading_byte = 0;
            let mut heading_scalar = 0;
            for (event, range) in Parser::new(text).into_offset_iter() {
                match event {
                    Event::Start(Tag::Heading { level, .. }) => {
                        heading_scalar += text[heading_byte..range.start].chars().count();
                        heading_byte = range.start;
                        heading = Some((level as usize, heading_scalar, String::new()))
                    }
                    Event::Text(t) | Event::Code(t) => {
                        if let Some((_, _, title)) = &mut heading
                            && title.len() < 2048
                        {
                            title.extend(t.chars().take(512));
                        }
                    }
                    Event::End(TagEnd::Heading(_)) => {
                        if let Some((level, byte, title)) = heading.take() {
                            add!(json!({"level":level,"start":byte,"title":title}));
                        }
                    }
                    _ => {}
                }
            }
        }
        Language::Json | Language::Notebook => {
            let value = parse_json(text).map_err(|_| Error::new("INVALID_DATA", "Invalid JSON"))?;
            match value {
                Value::Object(map) => {
                    for (key, value) in map {
                        add!(
                            json!({"key":key.chars().take(512).collect::<String>(),"type":kind(&value),"value":bounded(value)}),
                        );
                    }
                }
                Value::Array(list) => {
                    for (index, value) in list.into_iter().enumerate() {
                        add!(json!({"index":index,"type":kind(&value),"value":bounded(value)}));
                    }
                }
                value => add!(json!({"type":kind(&value),"value":bounded(value)})),
            }
        }
        Language::Csv | Language::Tsv => {
            let mut reader = csv::ReaderBuilder::new()
                .delimiter(if language == Language::Tsv {
                    b'\t'
                } else {
                    b','
                })
                .from_reader(text.as_bytes());
            let headers = reader
                .headers()
                .map_err(|_| Error::new("INVALID_DATA", "Invalid table headers"))?
                .clone();
            for (index, record) in reader.records().enumerate() {
                let record = record.map_err(|_| {
                    Error::new("INVALID_DATA", "Invalid or unequal-width table row")
                })?;
                let fields = record
                    .iter()
                    .take(16)
                    .map(|v| v.chars().take(64).collect::<String>())
                    .collect::<Vec<_>>();
                add!(json!({"row":index+1,"values":fields}));
            }
            return Ok(
                json!({"format":language.label(),"headers":headers.iter().take(16).map(|s|s.chars().take(64).collect::<String>()).collect::<Vec<_>>(),"column_count":headers.len(),"values_are_excerpts":true,"displayed_columns_max":16,"cell_excerpt_scalars":64,"entries":entries,"total":count,"truncated":offset.saturating_add(limit)<count,"next_offset":(offset.saturating_add(limit)<count).then_some(offset.saturating_add(limit))}),
            );
        }
        Language::Xml => {
            use quick_xml::{Reader, events::Event};
            let mut reader = Reader::from_str(text);
            let mut depth = 0usize;
            let mut roots = 0;
            loop {
                match reader
                    .read_event()
                    .map_err(|_| Error::new("INVALID_DATA", "Invalid XML"))?
                {
                    Event::Start(e) => {
                        for attribute in e.attributes() {
                            attribute.map_err(|_| {
                                Error::new("INVALID_DATA", "Invalid XML attributes")
                            })?;
                        }
                        if depth == 0 {
                            roots += 1;
                        }
                        add!(
                            json!({"depth":depth,"element":e.name().as_ref().chars().take(512).collect::<String>()}),
                        );
                        depth += 1;
                        if depth > 128 {
                            return Err(Error::new("LIMIT_EXCEEDED", "XML nesting exceeds 128"));
                        }
                    }
                    Event::Empty(e) => {
                        for attribute in e.attributes() {
                            attribute.map_err(|_| {
                                Error::new("INVALID_DATA", "Invalid XML attributes")
                            })?;
                        }
                        if depth == 0 {
                            roots += 1;
                        }
                        add!(
                            json!({"depth":depth,"element":e.name().as_ref().chars().take(512).collect::<String>()}),
                        );
                    }
                    Event::End(_) => {
                        depth = depth.checked_sub(1).ok_or_else(|| {
                            Error::new("INVALID_DATA", "Unmatched XML closing tag")
                        })?
                    }
                    Event::Text(e) if depth == 0 && !e.as_ref().trim().is_empty() => {
                        return Err(Error::new("INVALID_DATA", "XML text outside root"));
                    }
                    Event::Eof => {
                        if roots != 1 || depth != 0 {
                            return Err(Error::new(
                                "INVALID_DATA",
                                "XML requires one complete root",
                            ));
                        }
                        break;
                    }
                    _ => {}
                }
            }
        }
        _ => {
            for (index, line) in text.lines().enumerate() {
                add!(json!({"line":index+1,"text":line.chars().take(512).collect::<String>()}));
            }
        }
    }
    Ok(
        json!({"format":language.label(),"entry_values_are_excerpts":true,"entries":entries,"total":count,"truncated":offset.saturating_add(limit)<count,"next_offset":(offset.saturating_add(limit)<count).then_some(offset.saturating_add(limit))}),
    )
}
fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
fn bounded(value: Value) -> Value {
    match value {
        Value::String(s) => Value::String(s.chars().take(1024).collect()),
        Value::Array(a) => json!({"length":a.len()}),
        Value::Object(o) => json!({"keys":o.len()}),
        Value::Number(n) if n.to_string().len() > 1024 => {
            json!({"excerpt":n.to_string().chars().take(1024).collect::<String>(),"truncated":true})
        }
        v => v,
    }
}
