# Editor behavior

All document edits go through Document commands. One insertion/paste/deletion
or replace-all is one undo transaction. There is no timed typing coalescing.
Undo and redo restore both content and selection. Saving preserves history;
undoing to the saved content clears dirty, and branching clears redo. A no-op
edit does not add history. Readonly documents reject edit/undo/redo/save.

Cursor Left/Right and Backspace/Delete operate on extended grapheme clusters:
Vietnamese combining marks, regional flags, ZWJ emoji and CRLF are not split.
After an edit joins adjacent clusters, the cursor snaps forward to the next
valid boundary. Shift extends selection, Ctrl+A selects all, and typing replaces
the selection. Up/Down preserve the preferred grapheme column, not a cell or
UTF-16 column. Status line/column numbers are 1-based line/grapheme positions.

The tab strip shows neighboring tabs when they fit and scrolls enough to keep
the active tab visible. Position/encoding
come before the path in the status so long paths cannot hide the coordinates.
Viewport scrolling uses terminal cells. Tabs advance to configurable tab stops;
wide clusters are clipped as a unit with blank padding at the edges. Standalone
zero-width clusters receive a dotted-circle fallback. Controls are displayed as
replacement glyphs, never written as terminal commands. LF/CRLF line endings
are omitted from visible text but retained in storage. Terminal/font Unicode
rendering can still differ from the width library, especially for emoji.

The original newline insertion policy is captured on load: exclusively CRLF
uses CRLF; LF, mixed, bare-CR and empty input use LF. Existing endings are never
normalized. Ropey's Unicode/standalone-CR line splitting is disabled: only LF
and CRLF define editor lines; a standalone CR is preserved as a control. The
status describes the load/insertion policy, not a continuously rescanned count.

Literal search is case sensitive, wraps, and only selects matches whose two
endpoints are grapheme boundaries. Empty queries have no matches. Replace-all
uses nonoverlapping literal matches, accepts an empty replacement, and is one
undo transaction. The internal clipboard is shared across tabs; it is not the
OS clipboard. Bracketed paste is supported through Crossterm on Unix; Windows
native pasted characters are processed as Unicode key input.

Open/save and full-document find/replace are background jobs. Input edits,
clipboard commands and tab switches are refused until completion with a status
message; typed input during that interval is not queued. Resize still works.
Closing or quitting dirty tabs and every reload requires explicit Y; Enter
does not authorize discarding. N/Esc cancels. Save-as only creates a new path;
an existing unrelated destination is never overwritten. External modifications
require reload (with confirmation), a different save path, or cancel.

`F2` opens the command prompt. Its command list is available through `F1` and
the README, providing keyboard fallbacks when terminals intercept Ctrl keys.
Directory browsing is now supported under the user's explicit scope expansion:
`guineanote .` opens a workspace, no arguments show the current directory and
welcome, and file arguments preserve standalone editing. Ctrl+K opens a folder,
Ctrl+B toggles the tree, F4 changes pane, F5 refreshes, and tree arrows/Enter
expand/collapse/open. Tab/Esc returns from tree to editor; Tab in the editor
still inserts a tab. Opening another workspace keeps all dirty tabs and history.
Tree entries are read-only and loaded asynchronously on demand. Symlinks are
not followed. Clicks can activate tree entries, tabs and visible actions; clicking
text positions the caret on a grapheme boundary. Mouse is optional via config.
The sidebar auto-hides below 66 columns and the minimum compact layout is 30×9.

`Ctrl+P` command palette remains post-MVP. Configuration
is read only when explicitly supplied with `--config`; unknown TOML fields,
invalid tab widths and zero warning thresholds are errors before terminal setup.
