# Source formats and read-only review

F7 cycles Source → Split → Review. Split uses two panes from 76 available
columns; narrower areas show review full-width. Esc returns from full review.
F2 accepts `source`, `split`, `review`, and `preview` (cycle). F8 switches the
interface between Vietnamese and English; startup selection is `--language vi`
or `--language en`, overriding the configuration's `language` field.

| Format | Detection | Review |
| --- | --- | --- |
| Markdown | md, markdown, mdown | Styled headings, emphasis, quotes, lists/tasks, code fences, tables, links, footnotes |
| JSON | json, jsonc | Strict JSON formatted view; invalid JSON reports an error (JSONC comments are highlighted in source but are not accepted by strict review) |
| JSON Lines | jsonl, ndjson | Each nonempty physical line is parsed as a separate JSON record; record number on errors |
| XML | xml, svg, xsd, xsl | Indented element/text structure; checks matching end tags, attributes and one complete root |
| CSV / TSV | csv / tsv | Numbered table, first record as header, row/column counts, quoted fields including newlines, unequal-width rows report errors |
| TOML | toml | Parsed, formatted text with syntax colors |
| YAML / INI | yaml, yml / ini, cfg, conf, env, properties, .env and .env.* | Highlighted source review, without structural validation |

Review never rewrites document bytes, changes the source selection, or resets
undo history. Fix invalid data in source/split to refresh the review. JSON
preserves object insertion order and arbitrary-precision number representations;
duplicate object keys follow the parser's last-value behavior. XML text is
shown as structure and can normalize display indentation; source stays exact.
DTD/external entities are never loaded. Markdown HTML stays literal; links,
images and code are not opened, downloaded or executed.

CSV/TSV table cells clip with an ellipsis (48 cells maximum per field). Full
review Left/Right shifts the first displayed column, exposing columns beyond
the viewport; source holds complete untruncated field contents. Output is
limited to 100,000 display rows and reports truncation. Parsing validates all
CSV/TSV records even after reaching that display limit. Markdown preview tables
are a flowing text representation, not an editable spreadsheet.

Source detection and lightweight line-local syntax also support Rust, Python,
JavaScript, TypeScript, Go, C, C++, C#, Java, Kotlin, Swift, Ruby, PHP, Lua,
Shell, PowerShell, SQL, HTML, CSS/SCSS/SASS/LESS, Dockerfile, Makefile, Diff and
GraphQL. Names/extensions are case-insensitive. Fenced Markdown uses the same
alias registry (for example `rust`, `rs`, `typescript`, `bash`, `json`).
Unknown files open as plain UTF-8 text. Language support here means detection
and lightweight coloring, not a full grammar, compiler, LSP or formatter.

Only UTF-8 textual formats are editable. Binary XLSX, PDF, databases and legacy
encodings do not become editable just because their extension is recognized.
Existing strict UTF-8/BOM, newline, conflict and save protections still apply.

Parsing uses [pulldown-cmark](https://docs.rs/pulldown-cmark/0.13.6/pulldown_cmark/),
[serde_json](https://docs.rs/serde_json/1.0.154/serde_json/),
[quick-xml](https://docs.rs/quick-xml/0.42.0/quick_xml/) and
[csv](https://docs.rs/csv/1.4.0/csv/). Parsing/wrapping runs in a worker; results
are keyed by document, revision, viewport width, language and table position.
Stale results never replace source or a newer review.
