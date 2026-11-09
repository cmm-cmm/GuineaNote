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
    TypeScript,
    Go,
    C,
    Cpp,
    CSharp,
    Java,
    Kotlin,
    Swift,
    Ruby,
    Php,
    Lua,
    Shell,
    PowerShell,
    Sql,
    Yaml,
    Xml,
    Html,
    Css,
    Csv,
    Tsv,
    Ini,
    Dockerfile,
    Makefile,
    Diff,
    Graphql,
}

impl Language {
    pub fn detect(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return Self::Plain;
        };
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name == "dockerfile" || name.starts_with("dockerfile.") {
            return Self::Dockerfile;
        }
        if matches!(name.as_str(), "makefile" | "gnumakefile") {
            return Self::Makefile;
        }
        if name == ".env" || name.starts_with(".env.") {
            return Self::Ini;
        }
        Self::from_alias(path.extension().and_then(|s| s.to_str()).unwrap_or(""))
    }
    /// Shared by filename detection and Markdown fenced code blocks.
    pub fn from_alias(alias: &str) -> Self {
        match alias.to_ascii_lowercase().as_str() {
            "rs" | "rust" => Self::Rust,
            "py" | "pyw" | "python" => Self::Python,
            "js" | "jsx" | "mjs" | "cjs" | "javascript" => Self::JavaScript,
            "ts" | "tsx" | "typescript" => Self::TypeScript,
            "go" => Self::Go,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Self::Cpp,
            "cs" | "csharp" => Self::CSharp,
            "java" => Self::Java,
            "kt" | "kts" | "kotlin" => Self::Kotlin,
            "swift" => Self::Swift,
            "rb" | "ruby" => Self::Ruby,
            "php" => Self::Php,
            "lua" => Self::Lua,
            "sh" | "bash" | "zsh" | "fish" | "shell" => Self::Shell,
            "ps1" | "psm1" | "psd1" | "powershell" => Self::PowerShell,
            "sql" => Self::Sql,
            "json" | "jsonc" | "jsonl" | "ndjson" => Self::Json,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "xml" | "svg" | "xsd" | "xsl" => Self::Xml,
            "html" | "htm" => Self::Html,
            "css" | "scss" | "sass" | "less" => Self::Css,
            "csv" => Self::Csv,
            "tsv" => Self::Tsv,
            "ini" | "cfg" | "conf" | "env" | "properties" => Self::Ini,
            "dockerfile" => Self::Dockerfile,
            "mk" | "makefile" => Self::Makefile,
            "diff" | "patch" => Self::Diff,
            "graphql" | "gql" => Self::Graphql,
            "md" | "markdown" | "mdown" => Self::Markdown,
            _ => Self::Plain,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Plain => "Text",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Go => "Go",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::CSharp => "C#",
            Self::Java => "Java",
            Self::Kotlin => "Kotlin",
            Self::Swift => "Swift",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::Lua => "Lua",
            Self::Shell => "Shell",
            Self::PowerShell => "PowerShell",
            Self::Sql => "SQL",
            Self::Yaml => "YAML",
            Self::Xml => "XML",
            Self::Html => "HTML",
            Self::Css => "CSS",
            Self::Csv => "CSV",
            Self::Tsv => "TSV",
            Self::Ini => "INI/ENV",
            Self::Dockerfile => "Dockerfile",
            Self::Makefile => "Makefile",
            Self::Diff => "Diff",
            Self::Graphql => "GraphQL",
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
    Key,
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
            Language::JavaScript | Language::TypeScript => {
                "const let var function class import export from if else for while return async await new true false null undefined throw try catch"
            }
            Language::Json | Language::Toml | Language::Yaml => "true false null yes no on off",
            Language::Go => {
                "package import func var const type struct interface map chan go defer select range if else for return nil true false"
            }
            Language::C
            | Language::Cpp
            | Language::CSharp
            | Language::Java
            | Language::Kotlin
            | Language::Swift => {
                "int char void bool double float long short unsigned signed auto const static extern struct enum class public private protected virtual override interface namespace using import package include define if else switch case default for while do break continue return throw try catch finally new delete this self super null nullptr true false val var fun func let async await"
            }
            Language::Ruby => {
                "def class module require end do if elsif else unless while for return yield true false nil self"
            }
            Language::Php => {
                "php function class public private protected namespace use if else foreach while return echo new null true false"
            }
            Language::Lua => {
                "local function end if then elseif else for while do repeat until return nil true false and or not"
            }
            Language::Shell => {
                "if then elif else fi for while do done case esac function in export local readonly echo return exit"
            }
            Language::PowerShell => {
                "function param if elseif else foreach for while do switch try catch finally return begin process end true false null"
            }
            Language::Sql => {
                "select from where join inner left right outer on as insert into values update set delete create table index view drop alter order by group having limit offset union all distinct and or not null true false primary key references default"
            }
            Language::Dockerfile => {
                "from run cmd label expose env add copy entrypoint volume user workdir arg onbuild stopsignal healthcheck shell"
            }
            Language::Makefile => {
                "include ifdef ifndef ifeq ifneq else endif export override define endef"
            }
            Language::Graphql => {
                "query mutation subscription fragment on type input interface enum scalar union schema extend implements true false null"
            }

            _ => "",
        };
        let mut idx = 0;
        while idx < chars.len() {
            let start = idx;
            let hash_comment = matches!(
                language,
                Language::Python
                    | Language::Toml
                    | Language::Yaml
                    | Language::Ruby
                    | Language::Shell
                    | Language::PowerShell
                    | Language::Ini
                    | Language::Dockerfile
                    | Language::Makefile
                    | Language::Graphql
            ) && chars[idx] == '#';
            let slash_comment = matches!(
                language,
                Language::Rust
                    | Language::JavaScript
                    | Language::TypeScript
                    | Language::Go
                    | Language::C
                    | Language::Cpp
                    | Language::CSharp
                    | Language::Java
                    | Language::Kotlin
                    | Language::Swift
                    | Language::Php
                    | Language::Css
                    | Language::Json
            ) && chars[idx] == '/'
                && chars.get(idx + 1) == Some(&'/');
            let dash_comment = matches!(language, Language::Sql | Language::Lua)
                && chars[idx] == '-'
                && chars.get(idx + 1) == Some(&'-');
            let ini_comment = language == Language::Ini && chars[idx] == ';';
            if hash_comment || slash_comment || dash_comment || ini_comment {
                result.spans.push(HighlightSpan {
                    chars: start..chars.len(),
                    kind: TokenKind::Comment,
                });
                break;
            }
            let kind = if chars[idx] == '"' || chars[idx] == '\'' || chars[idx] == '`' {
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
                let key = matches!(language, Language::Json | Language::Yaml | Language::Toml)
                    && chars[idx..]
                        .iter()
                        .find(|c| !c.is_whitespace())
                        .is_some_and(|c| matches!(c, ':' | '='));
                Some(if key {
                    TokenKind::Key
                } else {
                    TokenKind::String
                })
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
                    .any(|keyword| keyword.eq_ignore_ascii_case(&word))
                    .then_some(TokenKind::Keyword)
                    .or_else(|| {
                        (matches!(
                            language,
                            Language::Yaml
                                | Language::Toml
                                | Language::Ini
                                | Language::Xml
                                | Language::Html
                                | Language::Css
                        ) && chars[idx..]
                            .iter()
                            .find(|c| !c.is_whitespace())
                            .is_some_and(|c| matches!(c, ':' | '=' | '>')))
                        .then_some(TokenKind::Key)
                    })
            } else {
                idx += 1;
                (matches!(
                    language,
                    Language::Csv | Language::Tsv | Language::Xml | Language::Html | Language::Css
                ) && matches!(chars[start], ',' | '\t' | '<' | '>' | '{' | '}' | ':' | ';'))
                .then_some(TokenKind::Key)
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
    fn programming_and_data_aliases_and_special_names() {
        for (filename, expected) in [
            ("test.TSX", Language::TypeScript),
            ("app.go", Language::Go),
            ("main.cpp", Language::Cpp),
            ("app.cs", Language::CSharp),
            ("main.kt", Language::Kotlin),
            ("test.sql", Language::Sql),
            ("config.YAML", Language::Yaml),
            ("image.svg", Language::Xml),
            ("data.ndjson", Language::Json),
            ("data.tsv", Language::Tsv),
            ("Dockerfile.dev", Language::Dockerfile),
            ("Makefile", Language::Makefile),
            (".env.local", Language::Ini),
            ("module.ps1", Language::PowerShell),
        ] {
            assert_eq!(Language::detect(Some(Path::new(filename))), expected);
        }
        for (language, source, kind) in [
            (Language::Sql, "SELECT", TokenKind::Keyword),
            (Language::Yaml, "name:", TokenKind::Key),
            (Language::Json, "\"name\": 42", TokenKind::Key),
            (Language::Lua, "-- comment", TokenKind::Comment),
            (Language::Csv, "a,b", TokenKind::Key),
        ] {
            assert_eq!(
                BasicSyntax.highlight_line(language, source, 0).spans[0].kind,
                kind
            );
        }
    }

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
