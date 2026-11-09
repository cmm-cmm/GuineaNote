//! Small, line-local lexer, not a full language grammar. No code is executed.
use std::{ops::Range, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Plain,
    Rust,
    Python,
    JavaScript,
    Json,
    Toml,
    Markdown,
}

impl Language {
    pub fn detect(path: Option<&Path>) -> Self {
        match path
            .and_then(Path::extension)
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "rs" => Self::Rust,
            "py" => Self::Python,
            "js" | "jsx" | "ts" | "tsx" => Self::JavaScript,
            "json" => Self::Json,
            "toml" => Self::Toml,
            "md" | "markdown" => Self::Markdown,
            _ => Self::Plain,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Plain => "Text",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::JavaScript => "JS/TS",
            Self::Json => "JSON",
            Self::Toml => "TOML",
            Self::Markdown => "Markdown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    String,
    Number,
    Comment,
    Heading,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Unicode scalar offsets relative to the supplied line.
    pub chars: Range<usize>,
    pub kind: TokenKind,
}

#[derive(Debug, Clone)]
pub struct Highlights {
    pub revision: u64,
    pub spans: Vec<HighlightSpan>,
}

impl Highlights {
    pub fn for_revision(&self, revision: u64) -> Option<&[HighlightSpan]> {
        (self.revision == revision).then_some(&self.spans)
    }
}

pub trait SyntaxProvider {
    fn highlight_line(&self, language: Language, line: &str, revision: u64) -> Highlights;
}

pub struct BasicSyntax;

impl SyntaxProvider for BasicSyntax {
    fn highlight_line(&self, language: Language, line: &str, revision: u64) -> Highlights {
        let mut result = Highlights {
            revision,
            spans: Vec::new(),
        };
        if language == Language::Plain {
            return result;
        }
        let chars: Vec<char> = line.chars().collect();
        if language == Language::Markdown {
            if line.trim_start().starts_with('#') {
                result.spans.push(HighlightSpan {
                    chars: 0..chars.len(),
                    kind: TokenKind::Heading,
                });
            }
            return result;
        }
        let keywords = match language {
            Language::Rust => {
                "fn let mut pub use mod impl trait struct enum match if else for while loop return self Self const static async await move true false Some None Ok Err"
            }
            Language::Python => {
                "def class import from as if elif else for while return with try except finally pass in is not and or lambda True False None async await yield"
            }
            Language::JavaScript => {
                "const let var function class import export from if else for while return async await new true false null undefined throw try catch"
            }
            Language::Json | Language::Toml => "true false null",
            _ => "",
        };
        let mut idx = 0;
        while idx < chars.len() {
            let start = idx;
            let hash_comment =
                matches!(language, Language::Python | Language::Toml) && chars[idx] == '#';
            let slash_comment = matches!(language, Language::Rust | Language::JavaScript)
                && chars[idx] == '/'
                && chars.get(idx + 1) == Some(&'/');
            if hash_comment || slash_comment {
                result.spans.push(HighlightSpan {
                    chars: start..chars.len(),
                    kind: TokenKind::Comment,
                });
                break;
            }
            let kind = if chars[idx] == '"' || chars[idx] == '\'' {
                let quote = chars[idx];
                idx += 1;
                while idx < chars.len() {
                    if chars[idx] == '\\' {
                        idx = (idx + 2).min(chars.len());
                    } else if chars[idx] == quote {
                        idx += 1;
                        break;
                    } else {
                        idx += 1;
                    }
                }
                Some(TokenKind::String)
            } else if chars[idx].is_ascii_digit() {
                idx += 1;
                while idx < chars.len()
                    && (chars[idx].is_ascii_alphanumeric() || matches!(chars[idx], '.' | '_'))
                {
                    idx += 1;
                }
                Some(TokenKind::Number)
            } else if chars[idx].is_alphabetic() || chars[idx] == '_' {
                idx += 1;
                while idx < chars.len() && (chars[idx].is_alphanumeric() || chars[idx] == '_') {
                    idx += 1;
                }
                let word: String = chars[start..idx].iter().collect();
                keywords
                    .split_whitespace()
                    .any(|keyword| keyword == word)
                    .then_some(TokenKind::Keyword)
            } else {
                idx += 1;
                None
            };
            if let Some(kind) = kind {
                result.spans.push(HighlightSpan {
                    chars: start..idx,
                    kind,
                });
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_uses_scalar_offsets_and_ignores_comments_inside_strings() {
        let result =
            BasicSyntax.highlight_line(Language::Rust, "let tên = \"// Việt\"; // comment", 7);
        assert_eq!(
            result.spans,
            vec![
                HighlightSpan {
                    chars: 0..3,
                    kind: TokenKind::Keyword
                },
                HighlightSpan {
                    chars: 10..19,
                    kind: TokenKind::String
                },
                HighlightSpan {
                    chars: 21..31,
                    kind: TokenKind::Comment
                },
            ]
        );
        assert!(result.for_revision(8).is_none());
        assert!(result.for_revision(7).is_some());
    }

    #[test]
    fn detection_plain_text_and_markdown() {
        assert_eq!(Language::detect(Some(Path::new("file.RS"))), Language::Rust);
        assert!(
            BasicSyntax
                .highlight_line(Language::Plain, "fn 123", 0)
                .spans
                .is_empty()
        );
        assert_eq!(
            BasicSyntax
                .highlight_line(Language::Markdown, "# Heading", 0)
                .spans[0]
                .kind,
            TokenKind::Heading
        );
        assert_eq!(
            BasicSyntax
                .highlight_line(Language::Json, "true 42", 0)
                .spans
                .len(),
            2
        );
    }
}
