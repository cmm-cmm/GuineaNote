# Settings, file navigation and reader redesign

## Dialog, mouse and search redesign

All dialogs now share raised title bars, clear borders and a shadow. (The
clickable top-right close mark was removed on 13/11/2026: Esc closes every
dialog, and dialogs that need it have their own Cancel / Keep working button.) Confirmations identify affected files and explain the
consequence above two large decision buttons. Keep working is initially selected;
Tab/Left/Right changes the selection, Enter/Space applies it, and Y/N remain
available. Input dialogs have a framed field and clickable Submit/Cancel.
Settings, review options, file picker and tab manager adapt to compact windows.
Modal clicks cannot activate controls behind the dialog.

The user requested a slightly darker overall background: canvas `#090E17`,
panel `#0D1422`, raised surfaces `#162031`. Green, honey amber `#DBAD5B` and
warm white remain the foreground palette. Confirmations use amber emphasis;
normal dialogs use green emphasis. The tab toolbar and labels have a spacer;
format/encoding/newline metadata is right-aligned, including tab-manager details.

Mouse selection supports drag and Shift-click. Vertical scrolling preserves
the caret; horizontal wheel and Shift+wheel scroll sideways. Keyboard movement
resumes following the caret. Middle-click closes a tab through existing safety
checks. File-picker and tab-manager rows support select, real double-click and
wheel navigation; blank rows do not activate stale targets. Windows input now
decodes button releases, drag, middle/right buttons and wheel modifiers.

Find/Replace is a full dialog with editable fields, line-numbered context,
replacement preview, total count, paged results and clickable actions. Literal,
case-sensitive scans and prepared replacements run on bounded background
workers. Revision/access checks precede application, each replace operation is
one Undo step, and saving is explicit. See [SEARCH_REPLACE.md](SEARCH_REPLACE.md).

## Previous workspace polish checkpoint

At this checkpoint, navy canvas/panel/raised surfaces were preserved.
The user clarified the warm accent as **honey amber, not orange**: true-color
`#DBAD5B`, with indexed color 179 as the fallback. Numbers, modified states,
file metadata and read-only badges use this accent.

The breadcrumb retains the filename at the end of long paths and adds a
compact source/split/review, language and buffer-size badge when space allows.
AI ownership replaces that badge with its current localized operation.
Leased source/review tabs prioritize AI read-only status, mute Save/Close and
remove the close mouse target; navigation/copy/new/other-tab hints remain.
Releasing the lease clears stale "AI is working" feedback and restores access.

Ctrl+P adds a cached format/size/line summary and AI operation beside the
selected file's path. Windows verbatim prefixes are hidden for presentation;
filesystem paths are unchanged. The status bar adds scalar selection count
and caret percentage only when there is enough room, preserving existing
position, encoding and save information. These surfaces read buffer metadata,
not the filesystem or complete document text.

Long first-line display indexes are prepared in the existing open worker for
buffers <=10 MiB. Parallel index workers borrow their line segments. Small
printable-ASCII edits between ASCII neighbors preserve a complete tab-free
index; Unicode/control/tab edits retain conservative invalidation. This moves
index preparation out of key handling without changing document revisions,
selection semantics or undo. Later lines remain lazy. See PERFORMANCE.md for
measurements, preparation completion time and the finite verification scope.

## Previous Settings and reader checkpoint

Requested: F8 Settings (Vietnamese/English initially), file details, a file-list
open dialog, richer syntax styling and a polished Markdown reader. Review must
offer split view and a separate synchronized read-only tab. User confirmed
syntax/presentation improvements; source formatting is not rewritten.

Implementation: asynchronous file picker/metadata/preferences; language cards
with Apply/Cancel and saved preferences; F9 file details; F7 view chooser;
linked review tabs with no editable history; richer semantic token colors;
Markdown heading hierarchy, outline navigation, reading/task statistics, aligned
tables, callout cards, numbered code blocks and link references. All file and
Markdown content remains inert. No GitHub CI. Preserve the local <100 ms UI
response budget on the confirmed release corpus up to 10 MiB, with I/O/parse
completion reported separately.

Validation: unit/behavior tests for settings, picker, details, review tab/history
isolation, Unicode and styling; native PTY interaction/latency and screenshots;
format, strict Clippy, workspace tests, release build and dependency audit.

Verified locally on Windows/MSVC: all 80 Rust tests pass, as do strict Clippy,
release build, native dialogs/reader, Unicode edit, workspace/mouse and panic
cleanup probes. Linux/macOS cross-Clippy passes (native execution pending).
The dependency audit passes the recorded RustSec snapshot with the existing
duplicate-Syn warning. No GitHub CI was created.

Two release latency runs: 374 fresh screen transitions, p50 9.507 ms,
p95 30.127 ms, max 58.551 ms, zero >=100 ms. The corpus reaches 10 MiB;
background JSON preview completion is 419.093 / 406.744 ms separately.
Raw samples: responsiveness-ux-run1.json and responsiveness-ux-run2.json.
This is measured host/workload evidence, not a timing guarantee for every file.

Controls: Ctrl+O filters/selects files and traverses folders; F8 selects language
with arrows or v/e, Enter applies and Esc cancels. F9 opens active-file details.
F7 then 1 selects split; F7 then 2 opens/reuses a linked review tab. F6 switches
tabs, Esc returns to source, Ctrl+W closes review without discarding source.
F10 toggles Markdown contents; Tab focuses the visible outline and Enter jumps.
All dialogs also have applicable clickable actions; details supports scrolling.
Use `cargo run -- .` for development, or the release profile for latency checks.

Settings precedence: built-in EN default < saved preferences < explicit config
language < CLI --language. Apply persists only language atomically in the user
config directory; no source/config file is rewritten. F2 language-vi/en remains
session-only. GUINEANOTE_PREFERENCES overrides the preference path for isolated
testing. File picker lists at most 10,000 shallow entries; full paths remain
available. Details is an immutable snapshot of the active buffer and disk
metadata; its fingerprint is the last verified open/save hash, not a fresh hash.
