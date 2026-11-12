# Editor behavior

All document edits go through Document commands. Each command (paste, cut,
replace-all, duplicate, move line, block indent, word delete) is one undo
transaction. Typing coalesces: contiguous typed text (including newlines, so a
paste delivered as key events is one step), contiguous Backspaces and contiguous
Deletes each extend the previous step while keystrokes are less than 1 s apart.
A caret move, selection change, other edit, Undo/Redo or save starts a new step.
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

Literal search is case sensitive by default (Alt+C ignores case, Alt+W matches
whole words), wraps, and only selects matches whose two endpoints are grapheme
boundaries. Empty queries have no matches. Replace-all uses nonoverlapping
matches, accepts an empty replacement, and is one undo transaction. Copy/cut/
paste use the system clipboard (see `clipboard` in the example config) with the
internal clipboard as fallback. Without a selection Ctrl+C copies and Ctrl+X cuts
the whole caret line including its newline. Bracketed paste is enabled through
Crossterm; Windows native pasted characters arrive as Unicode key input and are
merged into one Undo step by typing coalescing.

## Editing commands (keymap v1)

- **Word motion** Ctrl+Left/Right (Alt+B/Alt+F), with Shift to select, uses
  Unicode word segmentation, skips whitespace first, crosses line ends and keeps
  grapheme boundaries. It inspects at most 4,096 scalars around the caret, so it
  stays bounded on multi-MiB lines. Ctrl+Backspace/Alt+Backspace and
  Ctrl+Delete/Alt+D delete to the previous/next word boundary.
- **Enter** keeps the caret line's leading spaces/tabs (up to the caret). When
  more input follows Enter in the same input burst it is treated as a paste and
  no indentation is added, so pasted code is not indented twice.
- **Tab** inserts one indentation unit, or indents every selected line when the
  selection spans lines; **Shift+Tab** removes one level (a tab or up to
  `tab_width` spaces). The unit is `tab_width` spaces when space-indented lines
  outnumber tab-indented lines in the first 2,000 lines, otherwise a tab.
  Empty lines are not indented in a block.
- **Ctrl+D** duplicates the selection after itself, or the caret line below it
  (caret moves to the copy). **Alt+Up/Alt+Down** move the selected lines.
  Newline sequences are preserved byte-for-byte; inserted newlines use the
  document's newline policy (CRLF only for exclusively CRLF files).
- Commands that change text are refused with a reason (also shown in F2) for
  read-only documents, AI-owned tabs and linked review tabs.
- **Line and text tools (F2):** sort ascending/descending, remove duplicate
  lines, remove empty (blank) lines and trim trailing spaces/tabs apply to the
  selected lines or the whole document; existing newline sequences are reused
  in order. UPPER/lower/Title case use Unicode case mapping on the selection
  or caret line. `toggle-comment` adds or removes the language's line comment
  (`//`, `#`, `--`, `;`) after each non-empty line's indentation. Each is one
  Undo step; "Nothing to change" is reported when the text is already so.

Ctrl+F/H opens a dedicated search/replace panel; F3 finds the next previous-query
match. Results include a total count, line-numbered context and 200-match pages.
Enter/Shift+Enter moves next/previous; Tab cycles fields/results, Enter in results
returns to the editor. Ctrl+R replaces one, Alt+A replaces all. Replacement
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

`F2` opens the action list generated from the keymap (`commands.rs`), with
each action's keys and, when unavailable, the reason. It is the fallback for
every key a terminal intercepts; see KEYBOARD_SHORTCUTS.md.
Folder features were removed on 12/11/2026: there is no directory tree, open
folder, project search or folder argument (`guinea <folder>` exits with a
message before touching the terminal). No arguments show the welcome screen;
Ctrl+O opens the file picker, and typing a folder path in the F2 `open-path`
prompt opens the picker in that folder. Clicks can activate tabs and visible
actions; clicking
text positions the caret on a grapheme boundary. Drag/Shift-click extends a
selection; the wheel scrolls without moving the caret, Shift+wheel/horizontal
wheel scrolls sideways. Keyboard navigation follows the caret again. Middle-click
closes tabs with normal safeguards. File-picker/tab-manager rows support
double-click to open/switch; one click selects. Mouse is optional via config.
The sidebar auto-hides below 66 columns and the minimum compact layout is 30×9.

`Ctrl+P` opens the tab manager, with name/path filtering and switch/save/close.
Configuration is read only when explicitly supplied with `--config`; F10 language
preferences are stored separately. Unknown TOML fields,
invalid tab widths and zero warning thresholds are errors before terminal setup.
