//! Textual data/config review. All evaluation, interpolation and execution is
//! deliberately absent: displayed values come only from the document snapshot.
use super::value::Value;
use super::{Writer, code, markdown};
use crate::{i18n::Locale, theme::Theme};
use guinea_syntax::Language;
use ratatui::style::{Modifier, Style};

fn heading(writer: &mut Writer, en: &str, vi: &str, theme: Theme) {
    writer.push(
        if writer.locale == Locale::Vi { vi } else { en },
        Style::default().fg(theme.mint).add_modifier(Modifier::BOLD),
    );
    writer.newline();
    writer.blank();
}

pub(super) fn yaml(writer: &mut Writer, text: &str, theme: Theme) -> Result<(), String> {
    let options = serde_saphyr::options! {
        reject_unsupported_tags: true,
        non_finite_float_policy: serde_saphyr::NonFiniteFloatPolicy::PassThrough,
    };
    // Keep default depth/node/alias budgets and the absent include resolver.
    let documents: Vec<Value> = serde_saphyr::from_multiple_with_options(text, options)
        .map_err(|e| format!("YAML: {e}"))?;
    for (index, value) in documents.into_iter().enumerate() {
        if writer.full() {
            continue;
        }
        heading(
            writer,
            &format!("YAML · document {}", index + 1),
            &format!("YAML · tài liệu {}", index + 1),
            theme,
        );
        let formatted = serde_saphyr::to_string(&value).map_err(|e| format!("YAML: {e}"))?;
        code(writer, &formatted, Language::Yaml, theme);
        writer.blank();
    }
    Ok(())
}

pub(super) fn json5(writer: &mut Writer, text: &str, theme: Theme) -> Result<(), String> {
    json5_depth(text)?;
    // Ordered generic values retain JSON5 NaN/Infinity and mapping order.
    let value: Value = json5::from_str(text).map_err(|e| format!("JSON5/JSONC: {e}"))?;
    let formatted = json5::to_string(&value).map_err(|e| format!("JSON5: {e}"))?;
    heading(
        writer,
        "JSON5/JSONC · formatted view",
        "JSON5/JSONC · bản xem có định dạng",
        theme,
    );
    code(writer, &formatted, Language::Json5, theme);
    Ok(())
}

fn json5_depth(text: &str) -> Result<(), String> {
    let mut chars = text.chars().peekable();
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut line_comment = false;
    let mut block_comment = false;
    while let Some(ch) = chars.next() {
        if line_comment {
            if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                line_comment = false;
            }
            continue;
        }
        if block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_comment = false;
            }
            continue;
        }
        if let Some(end) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == end {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                line_comment = true;
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                block_comment = true;
            }
            '[' | '{' => {
                depth += 1;
                if depth > 128 {
                    return Err("JSON5: nesting exceeds review limit (128); source retained".into());
                }
            }
            ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

fn notebook_text(value: Option<&serde_json::Value>) -> Result<String, String> {
    match value {
        None => Ok(String::new()),
        Some(serde_json::Value::String(text)) => Ok(text.clone()),
        Some(serde_json::Value::Array(lines)) => lines
            .iter()
            .map(|line| {
                line.as_str()
                    .ok_or("notebook source/output must contain strings")
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|lines| lines.concat())
            .map_err(String::from),
        _ => Err("notebook source/output must be a string or an array of strings".into()),
    }
}

pub(super) fn notebook(writer: &mut Writer, text: &str, theme: Theme) -> Result<(), String> {
    let notebook: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("Notebook JSON: {e}"))?;
    let cells = notebook
        .get("cells")
        .and_then(|v| v.as_array())
        .ok_or("Notebook: expected a cells array (nbformat 4)")?;
    if notebook.get("nbformat").and_then(|v| v.as_u64()) != Some(4) {
        return Err("Notebook: only nbformat 4 is supported; source is retained".into());
    }
    let alias = notebook
        .pointer("/metadata/language_info/name")
        .or_else(|| notebook.pointer("/metadata/kernelspec/language"))
        .and_then(|v| v.as_str())
        .unwrap_or("python");
    let language = Language::from_alias(alias);
    heading(
        writer,
        &format!("Notebook · {} cells · {}", cells.len(), language.label()),
        &format!("Notebook · {} ô · {}", cells.len(), language.label()),
        theme,
    );
    for (index, cell) in cells.iter().enumerate() {
        let kind = cell
            .get("cell_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Notebook cell {}: missing cell_type", index + 1))?;
        let source = notebook_text(cell.get("source"))
            .map_err(|e| format!("Notebook cell {}: {e}", index + 1))?;
        if writer.full() {
            continue;
        }
        heading(
            writer,
            &format!("CELL {} · {}", index + 1, kind.to_ascii_uppercase()),
            &format!("Ô {} · {}", index + 1, kind.to_ascii_uppercase()),
            theme,
        );
        if kind == "markdown" {
            markdown(writer, &source, theme);
        } else {
            code(
                writer,
                &source,
                if kind == "code" {
                    language
                } else {
                    Language::Plain
                },
                theme,
            );
        }
        if let Some(outputs) = cell.get("outputs").and_then(|v| v.as_array()) {
            for output in outputs {
                if writer.full() {
                    break;
                }
                let value = output
                    .get("text")
                    .or_else(|| output.pointer("/data/text/plain"));
                if value.is_some() {
                    let text = notebook_text(value)?;
                    writer.push(
                        if writer.locale == Locale::Vi {
                            "Kết quả đã lưu:\n"
                        } else {
                            "Saved output:\n"
                        },
                        Style::default().fg(theme.muted),
                    );
                    code(writer, &text, Language::Plain, theme);
                } else if output.get("output_type").and_then(|v| v.as_str()) == Some("error") {
                    writer.push(
                        &format!(
                            "{}: {}\n",
                            output
                                .get("ename")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Error"),
                            output.get("evalue").and_then(|v| v.as_str()).unwrap_or("")
                        ),
                        Style::default().fg(theme.red),
                    );
                } else if output.get("data").is_some() {
                    writer.push(
                        if writer.locale == Locale::Vi {
                            "[Kết quả ảnh/HTML được giữ trong nguồn]\n"
                        } else {
                            "[Image/HTML output retained in source]\n"
                        },
                        Style::default().fg(theme.muted),
                    );
                }
            }
        }
        writer.blank();
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Field {
    section: String,
    key: String,
    value: String,
    line: usize,
}

fn fields(text: &str, language: Language) -> Result<Vec<Field>, String> {
    let env = language == Language::Env;
    let properties = language == Language::Properties;
    let mut result = Vec::new();
    let mut section = String::new();
    let mut lines = text.lines().enumerate().peekable();
    while let Some((index, raw)) = lines.next() {
        let mut text = if properties {
            raw.trim_start_matches([' ', '\t', '\u{c}'])
        } else {
            raw.trim()
        }
        .to_string();
        if text.is_empty() || text.starts_with(['#', ';']) || (properties && text.starts_with('!'))
        {
            continue;
        }
        if !env && !properties && text.starts_with('[') {
            let end = text
                .find(']')
                .ok_or_else(|| format!("INI line {}: unclosed section", index + 1))?;
            if !text[end + 1..].trim().is_empty() && !text[end + 1..].trim().starts_with(['#', ';'])
            {
                return Err(format!(
                    "INI line {}: unexpected text after section",
                    index + 1
                ));
            }
            section = text[1..end].trim().to_string();
            continue;
        }
        if env {
            text = text
                .strip_prefix("export ")
                .unwrap_or(&text)
                .trim_start()
                .to_string();
        }
        if properties {
            while text.ends_with('\\')
                && text.bytes().rev().take_while(|b| *b == b'\\').count() % 2 == 1
            {
                text.pop();
                let Some((_, next)) = lines.next() else {
                    break;
                };
                text.push_str(next.trim_start());
            }
        }
        let mut escaped_separator = false;
        let separator = if env {
            text.find('=').map(|at| (at, 1))
        } else {
            text.char_indices()
                .find(|(_, c)| {
                    if properties && escaped_separator {
                        escaped_separator = false;
                        return false;
                    }
                    if properties && *c == '\\' {
                        escaped_separator = true;
                        return false;
                    }
                    matches!(c, '=' | ':') || (properties && c.is_ascii_whitespace())
                })
                .map(|(byte, ch)| (byte, ch.len_utf8()))
        };
        let (key, mut value) = if let Some((at, size)) = separator {
            let value = text[at + size..].trim_start();
            (
                if properties {
                    text[..at].to_string()
                } else {
                    text[..at].trim().to_string()
                },
                if properties {
                    value
                        .trim_start_matches(['=', ':'])
                        .trim_start()
                        .to_string()
                } else {
                    value.to_string()
                },
            )
        } else if properties {
            (text.clone(), String::new())
        } else {
            return Err(format!(
                "{} line {}: expected KEY=VALUE",
                language.label(),
                index + 1
            ));
        };
        if key.is_empty()
            || (env
                && (!key
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')))
        {
            return Err(format!(
                "{} line {}: invalid key",
                language.label(),
                index + 1
            ));
        }
        if env {
            if let Some(quote) = value.chars().next().filter(|c| matches!(c, '\'' | '"')) {
                let mut closed = None;
                loop {
                    let mut escaped = false;
                    for (at, ch) in value.char_indices().skip(1) {
                        if ch == quote && !escaped {
                            closed = Some(at);
                            break;
                        }
                        escaped = ch == '\\' && !escaped;
                    }
                    if closed.is_some() {
                        break;
                    }
                    let Some((_, next)) = lines.next() else {
                        return Err(format!("ENV line {}: unclosed quoted value", index + 1));
                    };
                    value.push('\n');
                    value.push_str(next);
                }
                let end = closed.unwrap();
                let rest = value[end + quote.len_utf8()..].trim();
                if !rest.is_empty() && !rest.starts_with('#') {
                    return Err(format!(
                        "ENV line {}: unexpected text after quote",
                        index + 1
                    ));
                }
                value = value[1..end].to_string();
            } else if let Some(comment) = value
                .char_indices()
                .find(|(at, c)| {
                    *c == '#' && (*at == 0 || value[..*at].ends_with(char::is_whitespace))
                })
                .map(|(at, _)| at)
            {
                value.truncate(comment);
                value = value.trim_end().to_string();
            }
        }
        result.push(Field {
            section: section.clone(),
            key,
            value,
            line: index + 1,
        });
    }
    Ok(result)
}

pub(super) fn config(
    writer: &mut Writer,
    text: &str,
    language: Language,
    theme: Theme,
) -> Result<(), String> {
    if language == Language::Config {
        let trimmed = text.trim_start();
        if trimmed.starts_with('<') {
            return super::xml(writer, text, theme);
        }
        if trimmed.starts_with('{') {
            return json5(writer, text, theme);
        }
        if trimmed.starts_with('[') && serde_json::from_str::<serde_json::Value>(text).is_ok() {
            return json5(writer, text, theme);
        }
        // Config has many incompatible dialects. Only infer INI when it has
        // explicit sections or assignments; keep unfamiliar dialects readable.
        if !trimmed.starts_with('[') && !text.lines().any(|line| line.contains('=')) {
            if text
                .lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .any(|line| {
                    line.split_once(':').is_some_and(|(key, value)| {
                        !key.trim().is_empty() && value.starts_with(char::is_whitespace)
                    })
                })
            {
                return yaml(writer, text, theme);
            }
            heading(
                writer,
                "Config · source review",
                "Config · bản xem nguồn",
                theme,
            );
            code(writer, text, Language::Generic, theme);
            return Ok(());
        }
    }
    let rows = match fields(text, language) {
        Ok(rows) => rows,
        Err(_) if language == Language::Config => {
            heading(
                writer,
                "Config · source review",
                "Config · bản xem nguồn",
                theme,
            );
            code(writer, text, Language::Generic, theme);
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    heading(
        writer,
        &format!(
            "{} · {} fields · literal values",
            language.label(),
            rows.len()
        ),
        &format!(
            "{} · {} trường · giá trị nguyên văn",
            language.label(),
            rows.len()
        ),
        theme,
    );
    let mut section = None;
    for field in rows {
        if writer.full() {
            break;
        }
        if section != Some(field.section.clone()) {
            if !field.section.is_empty() {
                writer.blank();
                writer.push(
                    &format!("▾ [{}]\n", field.section),
                    Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
                );
            }
            section = Some(field.section);
        }
        writer.push(
            &format!("{:>4}  ", field.line),
            Style::default().fg(theme.muted),
        );
        writer.push(
            &format!("{} = ", field.key),
            Style::default().fg(theme.purple),
        );
        writer.push(&field.value, Style::default().fg(theme.mint));
        writer.newline();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn output(language: Language, text: &str) -> Result<String, String> {
        let mut writer = Writer::new(100, Locale::En);
        match language {
            Language::Yaml => yaml(&mut writer, text, Theme::new(false))?,
            Language::Json5 => json5(&mut writer, text, Theme::new(false))?,
            Language::Notebook => notebook(&mut writer, text, Theme::new(false))?,
            _ => config(&mut writer, text, language, Theme::new(false))?,
        }
        Ok(writer
            .finish()
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect::<String>())
    }
    #[test]
    fn structured_yaml_json5_and_config_dialects() {
        let yaml = output(
            Language::Yaml,
            "---\nname: Việt\nlist: [1, 2]\n---\na: &x {key: value}\nb: *x\n",
        )
        .unwrap();
        assert!(yaml.contains("document 2"));
        assert!(yaml.contains("key: value"));
        assert!(output(Language::Yaml, "a: [broken").is_err());
        assert!(output(Language::Yaml, "a: &x [*x]").is_err());
        assert!(output(Language::Yaml, "key: !include ./missing.yaml").is_err());
        assert!(
            output(Language::Yaml, "key: ${HOME}")
                .unwrap()
                .contains("${HOME}")
        );
        assert!(
            output(Language::Yaml, "? [a, b]\n: value\n")
                .unwrap()
                .contains("value")
        );
        let json = output(
            Language::Json5,
            "{ // comment\nname: 'Việt', hex: 0x10, values: [NaN, Infinity, -Infinity,],}",
        )
        .unwrap();
        assert!(json.contains("Việt"));
        assert!(json.contains("NaN"));
        assert!(json.contains("Infinity"));
        assert!(output(Language::Json5, "{a: ").is_err());
        let deep = format!(
            "// ignored [\u{2028}{}0{}",
            "[".repeat(129),
            "]".repeat(129)
        );
        assert!(json5_depth(&deep).is_err());
        assert!(json5_depth("{a:'[[[[', /* {{{ */ b: true}").is_ok());
        assert!(
            output(Language::Config, "<app><add key=\"a\" value=\"b\"/></app>")
                .unwrap()
                .contains("structure view")
        );
        assert!(
            output(Language::Config, "{key:'value',}")
                .unwrap()
                .contains("JSON5")
        );
        assert!(
            output(Language::Config, "key: value\nlist: [1,2]\n")
                .unwrap()
                .contains("YAML")
        );
        assert!(
            output(Language::Config, "server { listen 80; }")
                .unwrap()
                .contains("source review")
        );
    }
    #[test]
    fn notebook_renders_saved_cells_and_text_without_running_or_embedding_outputs() {
        let text = r##"{"nbformat":4,"metadata":{"language_info":{"name":"python"}},"cells":[{"cell_type":"markdown","source":["# Notebook\n","**hello**"]},{"cell_type":"code","source":["print('Việt')\n"],"outputs":[{"output_type":"stream","text":["Việt\n"]},{"output_type":"display_data","data":{"image/png":"DO_NOT_EMBED"}}]}]}"##;
        let result = output(Language::Notebook, text).unwrap();
        for expected in [
            "2 cells",
            "Python",
            "Notebook",
            "hello",
            "print('Việt')",
            "Saved output",
            "retained in source",
        ] {
            assert!(result.contains(expected), "{expected}: {result}");
        }
        assert!(!result.contains("DO_NOT_EMBED"));
        assert!(!result.contains("**"));
        assert!(output(Language::Notebook, "{\"nbformat\":3,\"cells\":[]}").is_err());
        assert!(
            output(
                Language::Notebook,
                "{\"nbformat\":4,\"cells\":[{\"cell_type\":\"code\",\"source\":[1]}]}"
            )
            .is_err()
        );
    }
    #[test]
    fn env_is_literal_and_preserves_duplicates_multiline_quotes_and_hashes() {
        let rows = fields("export HOST=localhost # comment\nURL=https://a/#path\nHOST='${HOME}'\nTEXT=\"line1\nline2\"\nEMPTY=\n", Language::Env).unwrap();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].value, "localhost");
        assert_eq!(rows[1].value, "https://a/#path");
        assert_eq!(rows[2].value, "${HOME}");
        assert_eq!(rows[3].value, "line1\nline2");
        for bad in ["bad key=x", "1KEY=x", "A='unclosed", "A=\"x\"junk"] {
            assert!(fields(bad, Language::Env).is_err(), "{bad}");
        }
    }
    #[test]
    fn ini_properties_sections_and_continuations() {
        let rows = fields(
            "[server]\nport=80\nport:81\n[db] # comment\nname=Việt\n",
            Language::Ini,
        )
        .unwrap();
        assert_eq!(rows[0].section, "server");
        assert_eq!(rows[1].key, "port");
        assert_eq!(rows[2].section, "db");
        let rows = fields(
            "! comment\nname: Guinea\\\n Note\nempty\n",
            Language::Properties,
        )
        .unwrap();
        assert_eq!(rows[0].value, "GuineaNote");
        assert_eq!(rows[1].value, "");
        assert!(fields("[broken", Language::Ini).is_err());
        let rows = fields(
            "a\\=b=c\nspace\\ key=literal\\u0041\n",
            Language::Properties,
        )
        .unwrap();
        assert_eq!(rows[0].key, "a\\=b");
        assert_eq!(rows[0].value, "c");
        assert_eq!(rows[1].key, "space\\ key");
        assert_eq!(rows[1].value, "literal\\u0041");
    }
}
