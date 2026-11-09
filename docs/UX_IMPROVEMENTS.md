# Settings, file navigation and reader redesign

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
