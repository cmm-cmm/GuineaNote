# Architecture

Current checkpoint: risk mitigation, 2026-11-10. Research describes the previous
source baseline and proposed evolution; this file describes implemented behavior.
See [risk register](RISK_MITIGATION.md) for evidence and remaining limits.

| Component | Responsibility |
| --- | --- |
| guinea-core | Private Ropey storage, checked coordinates, Document transactions/history, opaque ContentSnapshot, detachable ViewState |
| guinea-io | Streaming strict UTF-8/BOM reads/writes, stable identity and conflict checks, platform metadata, private recovery store |
| guinea-tui | Sole application state owner; commands, tabs, viewport, dialogs, input and rendering |
| guinea-syntax | Filename/content detection and revision-tagged basic line-local lexer |
| guinea-service | Typed Agent operations, workspace/secret policy, bounded analysis, live revision/lease protocol |
| guinea-agent | Separate JSON CLI and official rmcp stdio adapter; authenticated local IPC |

Rust stable/native, Ropey, Ratatui and Crossterm remain. No Node runtime, storage
rewrite, LSP runtime, new CI or package publication is introduced.

## Content and views

UI code cannot access a ropey::Rope. TextBuffer exposes checked scalar ranges
and chunk visitation. ContentSnapshot shares immutable content without exposing
its concrete backend, caret, history or caches. I/O and recovery consume chunks;
search/review may still allocate a full string on a worker with input limits.
worker_snapshot() preserves document metadata and access but omits history/cache.
Save completion merges metadata only, preserving edits made while saving.

Document remains the compatibility owner of one active ViewState. A detached
ViewState contains selection, preferred column and display caches, with no file
path/history/content. swap_view checks document id/revision and grapheme endpoints;
stale views are rejected. This is an incremental boundary: a shared DocumentStore,
editable splits and backend replacement are future work. Parser output still uses
Ratatui in the TUI; shared neutral parser models are not claimed as implemented.

ByteOffset, CharIndex, GraphemeColumn, CellColumn and Utf16Offset remain distinct.
Selections must end on extended-grapheme boundaries. Newline bytes stay in the
rope; BOM is separate. Dirty state uses content/save tokens and BOM in O(1),
including Undo/Redo and branches. Saving does not clear history.

History defaults: 2048 total Undo+Redo transactions and 64 MiB charged edit payload
plus 4096 bytes per transaction. The nearest transaction is retained even if it
alone exceeds the budget. This accounting is conservative policy, not an RSS
bound. Oldest history is trimmed with feedback; F9 reports counts/charge/trimming.

## Viewport and scheduling

An initial open worker prepares eligible long Unicode lines among the first 64
lines for documents <=10 MiB. Small/ASCII lines retain their direct fast paths.
The per-view LRU holds up to 64 indexes and a 16 MiB text budget, with an exception
for one oversized active line. Checkpoints preserve exact grapheme/tab widths.
Single-line edits remap unaffected line origins; newline edits invalidate safely.
Inactive source-layout text and review caches each have a 32 MiB aggregate target;
active working sets, object overhead and pending work are additional memory.

At most two TUI search/review CPU jobs hold a shared permit. Inactive reviews
cancel delivery; rendezvous channels avoid accumulating completed invisible
results. Search cancels/coalesces stale queries. Review input is limited to
16 MiB, output to 100000 rows and 16 MiB conservatively charged cells/spans;
source content stays complete. Cache eviction uses a bounded background drop
consumer; overload keeps the cache for retry. Existing I/O, recovery and Agent
workers have separate admission limits; two is not a global process-thread cap.

The event loop polls active jobs at 8 ms, otherwise waits for events up to 250 ms.
Idle does not redraw. Input bursts coalesce for <=4 ms and <=128 events before a
frame. Ratatui still builds the visible frame and diffs terminal cells; no claim
of custom dirty-region rendering is made.

Source lexing starts at the actual line origin and clips resulting spans. Beyond
16384 scalars of required context it renders plain text. This avoids false string
colors when horizontally scrolled; multiline grammar state remains future work.
NO_COLOR resets foreground/background cells; basic_colors selects the existing
256-color theme. The dark navy design and green/amber/white details remain.

GUINEANOTE_TRACE enables a create-new CSV sink. UI try_send uses a 4096-entry
queue; a worker writes stage durations and elapsed timestamps. Shutdown records
dropped samples. No keys, source, paths or clipboard are included. Tracing is
opt-in and its overhead is separate from an untraced performance run.

## I/O, recovery and Agent ownership

See [file safety](FILE_SAFETY.md) for commit boundaries, metadata and races.
Recovery runs on one worker with a one-request queue; UI captures shared roots
about once per second when state changes. A versioned checksummed private store
reopens committed drafts as dirty independent tabs, requiring deliberate Save As.
Normal acknowledged quit/discard removes checkpoints; crash/disconnect preserves
them. Unseen or corrupt drafts are retained. The checkpoint is not a backup of
every keypress and does not autosave source files.

The TUI owns all commits. Agent workers prepare changes and UI accepts only the
expected document revision with access/lease checks. AI locks source and linked
review views while other tabs stay editable. Recovery provenance is retained
separately from save destinations, including across repeated crashes; Agent tab
listing and snapshots enforce workspace/secret policy on that provenance.

Explorer remains shallow/lazy: one background scan, generation checks, no final
symlink traversal, 10000-entry visible cap. Mouse actions publish hit rectangles;
keyboard alternatives remain. Unix uses Crossterm input; the existing safe Windows
adapter handles UTF-16/repeats/AltGr and native mouse records. Terminal cleanup
restores raw mode, alternate screen, mouse and cursor on normal exit/panic.
Project Rust forbids unsafe_code; OS wrappers remain dependencies.
