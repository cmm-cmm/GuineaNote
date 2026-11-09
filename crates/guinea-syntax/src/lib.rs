//! Small, line-local lexer, not a full language grammar. No code is executed.
use std::{ops::Range, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Plain,
    Generic,
    Other(&'static str),
    Json5,
    Notebook,
    Env,
    Config,
    Properties,
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
            return Self::Env;
        }
        if matches!(
            name.as_str(),
            ".editorconfig"
                | ".gitconfig"
                | ".npmrc"
                | ".yarnrc"
                | ".eslintrc"
                | ".babelrc"
                | ".prettierrc"
                | "config"
                | "configure"
                | "settings"
        ) {
            return Self::Config;
        }
        if matches!(name.as_str(), "cmakelists.txt" | "cmakecache.txt") {
            return Self::Other("CMake");
        }
        if name == "justfile" {
            return Self::Other("Just");
        }
        if name == "rakefile" || name == "gemfile" {
            return Self::Ruby;
        }
        Self::from_alias(path.extension().and_then(|s| s.to_str()).unwrap_or(""))
    }
    pub fn detect_content(path: Option<&Path>, prefix: &str) -> Self {
        let detected = Self::detect(path);
        if let Some(line) = prefix
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("#!"))
        {
            for token in line.split_whitespace() {
                let name = token
                    .rsplit('/')
                    .next()
                    .unwrap_or(token)
                    .to_ascii_lowercase();
                if name.starts_with("python") {
                    return Self::Python;
                }
                if matches!(name.as_str(), "node" | "nodejs" | "deno" | "bun") {
                    return Self::JavaScript;
                }
                if matches!(name.as_str(), "sh" | "bash" | "zsh" | "fish") {
                    return Self::Shell;
                }
                if name == "ruby" {
                    return Self::Ruby;
                }
                if name == "perl" {
                    return Self::Other("Perl");
                }
                if name == "pwsh" {
                    return Self::PowerShell;
                }
            }
            return Self::Generic;
        }
        detected
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
            "json" | "jsonl" | "ndjson" => Self::Json,
            "json5" | "jsonc" => Self::Json5,
            "ipynb" | "jupyter" => Self::Notebook,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "xml" | "svg" | "xsd" | "xsl" => Self::Xml,
            "html" | "htm" => Self::Html,
            "css" | "scss" | "sass" | "less" => Self::Css,
            "csv" => Self::Csv,
            "tsv" => Self::Tsv,
            "ini" => Self::Ini,
            "env" => Self::Env,
            "config" | "cfg" | "conf" | "rc" => Self::Config,
            "properties" => Self::Properties,
            "dart" => Self::Other("Dart"),
            "zig" => Self::Other("Zig"),
            "nim" | "nims" => Self::Other("Nim"),
            "r" | "rmd" => Self::Other("R"),
            "jl" | "julia" => Self::Other("Julia"),
            "scala" | "sc" => Self::Other("Scala"),
            "hs" | "lhs" | "haskell" => Self::Other("Haskell"),
            "clj" | "cljs" | "cljc" | "edn" | "clojure" => Self::Other("Clojure"),
            "ex" | "exs" | "elixir" => Self::Other("Elixir"),
            "erl" | "hrl" | "erlang" => Self::Other("Erlang"),
            "fs" | "fsx" | "fsi" | "fsharp" => Self::Other("F#"),
            "ml" | "mli" | "ocaml" => Self::Other("OCaml"),
            "pl" | "pm" | "perl" => Self::Other("Perl"),
            "raku" | "rakumod" => Self::Other("Raku"),
            "lisp" | "lsp" | "el" | "scm" | "ss" | "rkt" => Self::Other("Lisp/Scheme"),
            "m" | "mm" => Self::Other("Objective-C/MATLAB"),
            "matlab" => Self::Other("MATLAB"),
            "f" | "for" | "f90" | "f95" | "f03" | "fortran" => Self::Other("Fortran"),
            "asm" | "s" | "inc" => Self::Other("Assembly"),
            "pas" | "pp" | "dpr" => Self::Other("Pascal"),
            "v" | "sv" | "svh" | "vh" => Self::Other("Verilog"),
            "vhd" | "vhdl" => Self::Other("VHDL"),
            "bat" | "cmd" => Self::Other("Batch"),
            "cmake" => Self::Other("CMake"),
            "nix" => Self::Other("Nix"),
            "tf" | "tfvars" | "hcl" => Self::Other("HCL/Terraform"),
            "proto" => Self::Other("Protocol Buffers"),
            "sol" => Self::Other("Solidity"),
            "vue" => Self::Other("Vue"),
            "svelte" => Self::Other("Svelte"),
            "astro" => Self::Other("Astro"),
            "tex" | "sty" | "cls" => Self::Other("LaTeX"),
            "typ" => Self::Other("Typst"),
            "pug" | "jade" | "ejs" | "hbs" | "mustache" => Self::Other("Template"),
            "txt" | "text" | "" => Self::Plain,
            "log" => Self::Other("Log"),
            "dockerfile" => Self::Dockerfile,
            "mk" | "makefile" => Self::Makefile,
            "diff" | "patch" => Self::Diff,
            "graphql" | "gql" => Self::Graphql,
            "md" | "markdown" | "mdown" => Self::Markdown,
            _ => Self::Generic,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Plain => "Text",
            Self::Generic => "Generic code",
            Self::Other(label) => label,
            Self::Json5 => "JSON5/JSONC",
            Self::Notebook => "Notebook",
            Self::Env => "ENV",
            Self::Config => "Config",
            Self::Properties => "Properties",
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
            Self::Ini => "INI",
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
    Function,
    Type,
    Constant,
    Property,
    Operator,
    Punctuation,
    Attribute,
    Escape,
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
    /// Ordered by scalar start, without overlapping spans.
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

fn slash_comment_language(language: Language) -> bool {
    matches!(
        language,
        Language::Rust
            | Language::C
            | Language::Cpp
            | Language::CSharp
            | Language::Java
            | Language::Kotlin
            | Language::Swift
            | Language::JavaScript
            | Language::TypeScript
            | Language::Go
            | Language::Php
            | Language::Css
            | Language::Json5
            | Language::Generic
            | Language::Other(_)
    )
}

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
            Language::Json
            | Language::Json5
            | Language::Notebook
            | Language::Toml
            | Language::Yaml => "true false null yes no on off",
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

            Language::Generic | Language::Other(_) => {
                "fn def function func fun let var val const static pub public private class struct type trait interface enum module namespace package import export from use include define extends implements return yield if then else elif elseif end for foreach while do loop match case switch break continue try catch except finally throw throws new async await true false null nil none self this super and or not in is lambda local begin select where void int string bool float double record sealed override with when unless until print println echo"
            }
            _ => "",
        };
        let mut idx = 0;
        while idx < chars.len() {
            let start = idx;
            let hash_comment = matches!(
                language,
                Language::Python
                    | Language::Generic
                    | Language::Other(_)
                    | Language::Env
                    | Language::Config
                    | Language::Properties
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
                    | Language::Generic
                    | Language::Other(_)
                    | Language::Json5
                    | Language::Notebook
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
            let ini_comment = matches!(
                language,
                Language::Ini | Language::Config | Language::Properties
            ) && chars[idx] == ';';
            if hash_comment || slash_comment || dash_comment || ini_comment {
                result.spans.push(HighlightSpan {
                    chars: start..chars.len(),
                    kind: TokenKind::Comment,
                });
                break;
            }
            if slash_comment_language(language)
                && chars[idx] == '/'
                && chars.get(idx + 1) == Some(&'*')
            {
                idx += 2;
                while idx < chars.len() && !(chars[idx - 1] == '*' && chars[idx] == '/') {
                    idx += 1;
                }
                if idx < chars.len() {
                    idx += 1;
                }
                result.spans.push(HighlightSpan {
                    chars: start..idx,
                    kind: TokenKind::Comment,
                });
                continue;
            }
            if chars[idx] == '@'
                || (matches!(language, Language::Rust)
                    && chars[idx] == '#'
                    && chars.get(idx + 1) == Some(&'['))
            {
                idx += 1;
                while idx < chars.len()
                    && (chars[idx].is_alphanumeric() || matches!(chars[idx], '_' | '.' | '[' | ']'))
                {
                    idx += 1;
                }
                result.spans.push(HighlightSpan {
                    chars: start..idx,
                    kind: TokenKind::Attribute,
                });
                continue;
            }
            if language == Language::Rust
                && chars[idx] == '\''
                && chars
                    .get(idx + 1)
                    .is_some_and(|c| c.is_alphabetic() || *c == '_')
                && chars.get(idx + 2) != Some(&'\'')
            {
                idx += 2;
                while idx < chars.len() && (chars[idx].is_alphanumeric() || chars[idx] == '_') {
                    idx += 1;
                }
                result.spans.push(HighlightSpan {
                    chars: start..idx,
                    kind: TokenKind::Attribute,
                });
                continue;
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
                let key = matches!(
                    language,
                    Language::Json
                        | Language::Json5
                        | Language::Notebook
                        | Language::Yaml
                        | Language::Toml
                        | Language::Env
                        | Language::Config
                        | Language::Properties
                        | Language::Ini
                ) && chars[idx..]
                    .iter()
                    .find(|c| !c.is_whitespace())
                    .is_some_and(|c| matches!(c, ':' | '='));
                if key {
                    Some(TokenKind::Key)
                } else {
                    let mut part = start;
                    let mut scan = start + 1;
                    while scan < idx {
                        if chars[scan] == '\\' {
                            if part < scan {
                                result.spans.push(HighlightSpan {
                                    chars: part..scan,
                                    kind: TokenKind::String,
                                });
                            }
                            let mut end = (scan + 2).min(idx);
                            if chars.get(scan + 1) == Some(&'u')
                                && chars.get(scan + 2) == Some(&'{')
                            {
                                while end < idx && chars[end] != '}' {
                                    end += 1;
                                }
                                end = (end + 1).min(idx);
                            }
                            result.spans.push(HighlightSpan {
                                chars: scan..end,
                                kind: TokenKind::Escape,
                            });
                            part = end;
                            scan = end;
                        } else {
                            scan += 1;
                        }
                    }
                    if part < idx {
                        result.spans.push(HighlightSpan {
                            chars: part..idx,
                            kind: TokenKind::String,
                        });
                    }
                    continue;
                }
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
                let keyword = keywords
                    .split_whitespace()
                    .any(|keyword| {
                        if matches!(
                            language,
                            Language::Sql
                                | Language::Dockerfile
                                | Language::Generic
                                | Language::Other(_)
                        ) {
                            keyword.eq_ignore_ascii_case(&word)
                        } else {
                            keyword == word
                        }
                    })
                    .then_some(TokenKind::Keyword)
                    .or_else(|| {
                        (matches!(
                            language,
                            Language::Yaml
                                | Language::Json5
                                | Language::Env
                                | Language::Config
                                | Language::Properties
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
                    });
                let next = chars[idx..].iter().copied().find(|c| !c.is_whitespace());
                keyword.or_else(|| {
                    if next == Some('(') || (language == Language::Rust && next == Some('!')) {
                        Some(TokenKind::Function)
                    } else if word.chars().any(|c| c.is_alphabetic())
                        && word
                            .chars()
                            .all(|c| c.is_uppercase() || c.is_ascii_digit() || c == '_')
                    {
                        Some(TokenKind::Constant)
                    } else if word.chars().next().is_some_and(char::is_uppercase) {
                        Some(TokenKind::Type)
                    } else if start > 0 && matches!(chars[start - 1], '.' | '$') {
                        Some(TokenKind::Property)
                    } else {
                        None
                    }
                })
            } else {
                idx += 1;
                (matches!(
                    language,
                    Language::Csv | Language::Tsv | Language::Xml | Language::Html | Language::Css
                ) && matches!(chars[start], ',' | '\t' | '<' | '>' | '{' | '}' | ':' | ';'))
                .then_some(TokenKind::Key)
                .or_else(|| {
                    if "(){}[],;:.".contains(chars[start]) {
                        Some(TokenKind::Punctuation)
                    } else if "+-*/%=!<>|&^~?:".contains(chars[start]) {
                        Some(TokenKind::Operator)
                    } else {
                        None
                    }
                })
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
    fn semantic_tokens_escape_lifetimes_macros_and_block_comments_are_ordered() {
        let text = "#[derive(Debug)] fn main<'a>() { let thing: Widget = build(); println!(\"hi\\n\"); /* block */ MAX_VALUE + thing.field; }";
        let result = BasicSyntax.highlight_line(Language::Rust, text, 3);
        for kind in [
            TokenKind::Attribute,
            TokenKind::Function,
            TokenKind::Type,
            TokenKind::Escape,
            TokenKind::Constant,
            TokenKind::Property,
            TokenKind::Operator,
            TokenKind::Punctuation,
            TokenKind::Comment,
        ] {
            assert!(result.spans.iter().any(|s| s.kind == kind), "{kind:?}");
        }
        assert!(
            result
                .spans
                .windows(2)
                .all(|s| s[0].chars.end <= s[1].chars.start)
        );
        assert!(
            result
                .spans
                .iter()
                .filter(|s| s.kind == TokenKind::String)
                .all(|s| !text
                    .chars()
                    .skip(s.chars.start)
                    .take(s.chars.len())
                    .collect::<String>()
                    .contains("block"))
        );
    }

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
            (".env.local", Language::Env),
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
    fn extended_sources_config_names_and_shebangs_have_safe_fallbacks() {
        for (name, label) in [
            ("App.vue", "Vue"),
            ("main.dart", "Dart"),
            ("build.zig", "Zig"),
            ("main.exs", "Elixir"),
            ("analysis.R", "R"),
            ("CMakeLists.txt", "CMake"),
            (".env.production.local", "ENV"),
            ("app.config", "Config"),
            (".editorconfig", "Config"),
            ("config.jsonc", "JSON5/JSONC"),
            ("analysis.ipynb", "Notebook"),
            ("app.properties", "Properties"),
        ] {
            assert_eq!(Language::detect(Some(Path::new(name))).label(), label);
        }
        assert_eq!(
            Language::detect(Some(Path::new("future.unknownlang"))),
            Language::Generic
        );
        assert_eq!(
            Language::detect_content(
                Some(Path::new("script")),
                "#!/usr/bin/env python3\nprint(1)"
            ),
            Language::Python
        );
        assert_eq!(
            Language::detect_content(None, "#!/bin/bash\necho hello"),
            Language::Shell
        );
        let spans = BasicSyntax
            .highlight_line(
                Language::Generic,
                "function foo() { return 42; } // comment",
                0,
            )
            .spans;
        assert!(spans.iter().any(|span| span.kind == TokenKind::Keyword));
        assert!(spans.iter().any(|span| span.kind == TokenKind::Comment));
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
                    chars: 8..9,
                    kind: TokenKind::Operator
                },
                HighlightSpan {
                    chars: 10..19,
                    kind: TokenKind::String
                },
                HighlightSpan {
                    chars: 19..20,
                    kind: TokenKind::Punctuation
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
