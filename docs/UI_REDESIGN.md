# Workspace TUI redesign

User-requested scope change: replace the initial minimal interface with a more
visual terminal workspace and support both individual files and directory trees.
This explicitly brings the previously post-MVP file explorer into current scope;
LSP, plugins, Git runtime and recovery remain out of scope.

Design: the original navy surfaces, green focus/brand accents, walnut-brown
borders, warm-white text, sage/olive and light-brown syntax, restrained
rounded borders, an explorer sidebar, file tabs, breadcrumbs, a welcome screen,
visible keyboard actions, and distinct editor/explorer focus. Standard Unicode
and terminal colors; no Nerd Font or external image dependency. A 256-color
palette is available for terminals without true color.

The user clarified that the original main backgrounds must stay unchanged:
navy canvas #0c111c, panels #111827 and raised surfaces #192436.
Green, brown and white apply to content and details: brown borders #705a43,
warm-white text #f8f5ec and primary green #86d38a.
Sage/olive distinguishes syntax and secondary labels; light brown marks values,
types and operators. Clay is reserved for errors and warnings. The indexed
fallback keeps the original indexed backgrounds and uses green/earth/cream accents.

Palette validation uses existing Rust tests, strict Clippy, a release build,
native dialogs/reader captures and the 10 MiB response-budget probe. Results
and the separately measured background parse completion are in PERFORMANCE.md.

Behavior: `guineanote .` opens a workspace; a file argument opens the standalone
editor; no arguments shows the current directory and welcome screen. Ctrl+K
opens a folder, Ctrl+B toggles the explorer, F4 changes pane focus, arrows and
Enter browse/open, F5 refreshes. Tab returns from tree to editor and still inserts
a tab in the editor. All primary operations remain keyboard-accessible. Optional
mouse interaction covers the tree, tabs, visible actions and editor caret.
The welcome wordmark uses ASCII, avoiding font-dependent heavy box glyphs.
Neighboring file tabs remain visible when space permits. Terminal layouts below
66 columns hide the sidebar; low terminals use compact chrome rather than
removing the document area.

Directory loading is shallow, lazy and asynchronous. Children are only read on
expansion, sorted directories first, and symlinks are shown without following
them. Each directory listing has a visible 10,000-entry safety cap. Changing the
workspace invalidates old results. Directory reads never mutate document text,
history, savepoints or files. Existing strict UTF-8 and safe-save logic is reused.

Validation: tree ordering/collapse/refresh/stale jobs and errors, focus routing,
no accidental edits from the sidebar, opening files while retaining dirty tabs,
small-terminal layout, Unicode clipping, mouse hit regions, native PTY folder
browsing and existing editing/save/cleanup flows, plus required Rust gates and
cross-target clippy. Capture real terminal previews for visual inspection.
