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
the active tab visible. A spacer separates its toolbar from the labels at normal
window sizes. Position/save state is on the left of the status; format, encoding
and newline policy are right-aligned. Paths appear in the breadcrumb instead.
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

Ctrl+F/H opens a dedicated search/replace panel; F3 finds the next previous-query
match. Results include a total count, line-numbered context and 200-match pages.
Enter/Shift+Enter moves next/previous; Tab cycles fields/results, Enter in results
returns to the editor. Ctrl+R replaces one, Ctrl+Shift+R replaces all. Replacement
is prepared off-thread and applied only to the original document revision with
current write permission. Each operation is one Undo step; saving is separate.
Esc or the close button discards pending work. See SEARCH_REPLACE.md for limits.

Open/save and full-document find/replace are background jobs. Opening and legacy
find-next temporarily block edits while allowing status feedback and resize;
saving allows continued editing and preserves later edits as dirty. The search
panel allows query changes during scans, cancelling/coalescing stale work, and
can close during replacement preparation before changes are applied.
Closing or quitting dirty tabs and every reload shows explicit decision buttons.
Keep working is initially selected; Enter activates the current choice,
Tab/Left/Right switches choices, Y confirms and N/Esc cancels. Save-as only creates a new path;
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
text positions the caret on a grapheme boundary. Drag/Shift-click extends a
selection; the wheel scrolls without moving the caret, Shift+wheel/horizontal
wheel scrolls sideways. Keyboard navigation follows the caret again. Middle-click
closes tabs with normal safeguards. File-picker/tab-manager rows support
double-click to open/switch; one click selects. Mouse is optional via config.
The sidebar auto-hides below 66 columns and the minimum compact layout is 30×9.

`Ctrl+P` opens the tab manager, with name/path filtering and switch/save/close.
Configuration is read only when explicitly supplied with `--config`; F8 language
preferences are stored separately. Unknown TOML fields,
invalid tab widths and zero warning thresholds are errors before terminal setup.
