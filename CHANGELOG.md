# Changelog

All notable changes. Versions follow [Semantic Versioning](https://semver.org/).
Release targets: Windows x64 and Linux x64 (static musl). macOS is not
released.

## [Unreleased]

### Fixed
- F2 action list: Enter runs the highlighted row. Typing a name that was
  also an action name (`find`, then arrows to `find-next`) ran the typed
  action instead of the highlighted one. A typed name is now the first row,
  ahead of names that start with or contain it and of description matches;
  pasting into the filter restarts the highlight at the top.
- Split view below 76 editor columns shows the review alone, as before, but
  is now read-only like a full review and Esc returns to the source. Typing
  used to edit the source that was not on screen while the status bar said
  EDIT.
- The caret stays visible while a save runs, or while a job works on another
  tab; it hid although typing was still accepted.
- Find/Replace result rows show context from the match's own line only; text
  of the neighbouring lines no longer appears beside its line number.

## [0.1.1] — 2026-11-13

### Install
- `cargo install --git https://github.com/cmm-cmm/GuineaNote --tag v0.1.1 --locked guineanote`
  (and `guinea-agent` for the Agent binaries) builds a tagged release from
  GitHub; documented in the README.

### Changed
- Dialogs no longer draw a mouse-only close mark in the top-right corner;
  Esc closes every dialog, and dialogs that need it keep their own Cancel /
  Keep working button.

### Fixed
- Windows: the editor border, file details, the large-file prompt and the
  Opening/Saving status showed paths with the internal `\\?\` prefix; paths
  are now shown as typed (`D:\notes\a.md`, `\\server\share\a.md`).

### Documentation
- README screenshots rendered from the real editor
  (`scripts/readme_screenshots.py`).

## [0.1.0] — 2026-11-13 (preview)

First public release, published as a GitHub pre-release.

### Editor
- Tabs with a one-row tab bar and tab manager (F12 / Ctrl+P), Ctrl+O file
  picker, Settings (F10, Vietnamese/English), file details (F9), F1 guide and
  F2 searchable action list showing each action's key or why it is unavailable.
- Find/Replace panel (Ctrl+F / Ctrl+H) with Match case (Alt+C), Whole word
  (Alt+W) and regular expressions (Alt+R, linear-time `regex`, `$1`/`${name}`
  replacements); Replace all is one Undo step. Mark All and bookmarks of
  marked lines.
- Editing: typing Undo groups, word motion/deletion, auto-indent, block
  indent/outdent, duplicate, move lines, whole-line cut/copy, bracketed paste;
  sort, remove duplicate/empty lines, trim trailing whitespace, UPPER/lower/
  Title case and per-language line comments from F2.
- Bookmarks (Ctrl+F2, Alt+F2 / Shift+F2, gutter click), smart highlighting of
  the selected word, bracket pair highlight, go to line (Ctrl+L / Ctrl+G).
- Session restore of open files, carets and the active tab
  (`session = false` disables it).
- Read-only formatted previews (Markdown, JSON, CSV, XML, TOML, YAML, …) as a
  split or a linked tab; multi-line syntax highlighting.
- System clipboard (Windows API; xclip/xsel/wl-clipboard on Linux), opt-in
  OSC 52, internal fallback. Optional mouse with keyboard alternatives.
- Opt-in Agent CLI (`guineanote-agent`) and MCP server (`guineanote-mcp`)
  editing live documents at an expected revision under a read-only lease.

### Data safety
- Strict UTF-8 with BOM, LF/CRLF/mixed newlines and untouched bytes
  preserved; atomic saves with external-change conflict checks; checksummed
  crash-recovery drafts (offered once even after a crash during handoff).

### Responsiveness
- Long lines indexed on a worker; large line/case/comment tools run on a
  worker and Esc cancels them with text and history unchanged.
- Keys and pastes typed while a file opens or a tool runs are queued and
  applied in order (bounded, refused loudly when full) instead of dropped;
  a job pauses only its own tab. Quit/Close pressed during a save wait for
  it (Esc withdraws; a failed save closes nothing) instead of being refused.
- Undo charged by the memory each step adds: six whole-file steps of a
  10 MiB file fit the 64 MiB history budget.
- Latency gate: every measured interaction above 50 ms is investigated;
  verified natively on Windows/ConPTY and Linux x86_64 (container).

### Fixed during verification
- Session restore on Windows lost carets and the active tab (verbatim
  `\\?\` paths never matched).
- Whole-document line tools selected the whole result (the next key replaced
  the file) and sorted the empty line after a final newline to the top.

### Packaging
- Windows binaries link the C runtime statically; Linux release binaries
  are static musl executables; `scripts/release/notices.py` generates
  `THIRD_PARTY_NOTICES.txt`.
- Reproducible archives with `SHA256SUMS` and `release-manifest.json`
  (`scripts/release/package.py`); prepared, not yet submitted, winget
  manifest (`cmm-cmm.GuineaNote`) and npm packages (`guinea-note`).

### Community
- Public repository with a Code of Conduct (Contributor Covenant 2.1),
  contributing guide, security policy (private vulnerability reports),
  support page, issue forms and a pull request template.
- README rewritten for the public repository, with the Guinea Note banner;
  mascot icons (PNG, ICO, SVG) in `icon/`.

[0.1.1]: https://github.com/cmm-cmm/GuineaNote/releases/tag/v0.1.1
[0.1.0]: https://github.com/cmm-cmm/GuineaNote/releases/tag/v0.1.0
