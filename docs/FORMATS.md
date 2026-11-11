# Source formats and read-only review

F8 offers Split or a linked, read-only Review tab. Split uses two panes from
76 available columns; narrower areas show review full-width. F6/F7 switch tabs;
Esc returns from review to its source. F2 accepts `source`, `split`,
`review` / `review-tab`, and `preview` (chooser). F10 opens language Settings
with Apply/Cancel and persisted preferences; an explicit config language takes
precedence, and `--language vi` / `--language en` overrides both.

| Format | Detection | Review |
| --- | --- | --- |
| Markdown | md, markdown, mdown | Contents navigation, heading cards, reading/task statistics, callouts, numbered code cards, aligned tables, links and footnotes |
| JSON | json | Strict JSON formatted view; invalid JSON reports an error |
| JSON5 / JSONC | json5, jsonc | Formatted JSON5, comments/trailing commas/unquoted keys/hex/NaN/Infinity accepted; JSONC uses the JSON5 superset |
| JSON Lines | jsonl, ndjson | Each nonempty physical line is parsed as a separate JSON record; record number on errors |
| XML | xml, svg, xsd, xsl | Indented element/text structure; checks matching end tags, attributes and one complete root |
| CSV / TSV | csv / tsv | Numbered table, first record as header, row/column counts, quoted fields including newlines, unequal-width rows report errors |
| TOML | toml | Parsed, formatted text with syntax colors |
| YAML | yaml, yml | Parsed/formatted YAML, multiple documents, anchors/aliases, parser depth/repetition limits retained |
| ENV | .env, .env.*, *.env | Literal key/value review, export, quotes, multiline quoted values, line numbers, comments and duplicate keys |
| INI | ini | Sections, key=value/key:value, line numbers, duplicate keys retained |
| Properties | properties | Fields, escaped key separators and continuation lines; escapes displayed literally |
| Config | config, cfg, conf, rc, .editorconfig, .gitconfig, .npmrc, .yarnrc, .eslintrc, .babelrc, .prettierrc, config/settings names | Infer XML, JSON5/JSONC, JSON arrays, YAML or INI; unfamiliar dialects retain a highlighted source review |
| Notebook | ipynb | nbformat 4: Markdown/code/raw cells, saved text outputs and errors; image/HTML payloads stay in source |
| All other UTF-8 source/text | Any extension or no extension | Editable text and read-only source review; generic lexical colors for unknown code extensions, shebang detection for extensionless scripts |

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
CSV/TSV records even after reaching that display limit. Markdown tables use
borders, aligned columns and alternating row backgrounds. They display up to
12 fitting columns, 10,000 rows and 8 wrapped lines per cell; the full content
remains in source. Markdown block collection is bounded to 100,000 events / an
approximately 2 MiB text budget, and the outline to 2,000 headings. These are
display limits; review never edits the document.

Source detection and lightweight line-local syntax also support Rust, Python,
JavaScript, TypeScript, Go, C, C++, C#, Java, Kotlin, Swift, Ruby, PHP, Lua,
Shell, PowerShell, SQL, HTML, CSS/SCSS/SASS/LESS, Dockerfile, Makefile, Diff and
GraphQL. Names/extensions are case-insensitive. Fenced Markdown uses the same
alias registry (for example `rust`, `rs`, `typescript`, `bash`, `json`).
Additional named source aliases include Dart, Zig, Nim, R, Julia, Scala,
Haskell, Clojure/EDN, Elixir, Erlang, F#, OCaml, Perl/Raku, Lisp/Scheme,
Objective-C/MATLAB, Fortran, Assembly, Pascal, Verilog/VHDL, Batch, CMake,
Nix, HCL/Terraform, Protocol Buffers, Solidity, Vue, Svelte, Astro, LaTeX,
Typst and templates. Common special filenames and Python/Node/Shell/Ruby/Perl/
PowerShell shebangs are recognized without requiring a filename extension.

All UTF-8 programming files can be opened, edited and reviewed. Unknown
extensions use generic strings/numbers/keywords/comments; plain text stays
plain. Language support here means detection
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

YAML uses serde-saphyr 2.1.1 with normalized review serialization; comments/
anchor presentation can differ from source. Its depth/node/alias budgets stay
enabled. Unsupported custom tags are rejected; include/property resolvers are
not installed. Ordered generic values retain mapping order, complex keys and
non-finite numbers without a libyaml dependency.
JSON5 uses json5 1.3.7 with a nesting preflight capped at 128 that respects
strings and comments. These views normalize data presentation and never replace
the source. Notebook cells and stored outputs are never executed; image/HTML
payloads are not embedded. ENV variables are never loaded, interpolated or
exported into the process, and commands/config directives are never executed.
INI/Properties values are literal text, not an evaluation of a tool's runtime
configuration. Generic config fallback is not structural validation of every
possible config dialect.

Parser references: [serde-saphyr](https://docs.rs/serde-saphyr/2.1.1/serde_saphyr/)
and [json5](https://docs.rs/json5/1.3.7/json5/).
