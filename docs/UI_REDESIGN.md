# Workspace TUI redesign

User-requested scope change: replace the initial minimal interface with a more
visual terminal workspace and support both individual files and directory trees.
This explicitly brings the previously post-MVP file explorer into current scope;
LSP, plugins, Git runtime and recovery remain out of scope.

Design: deep navy surfaces, mint focus/brand accent, blue/purple syntax, restrained
rounded borders, an explorer sidebar, file tabs, breadcrumbs, a welcome screen,
visible keyboard actions, and distinct editor/explorer focus. Standard Unicode
and terminal colors; no Nerd Font or external image dependency. A 256-color
palette is available for terminals without true color.

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
