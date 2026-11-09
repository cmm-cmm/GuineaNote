# Architecture

Current view optimization: the TUI file-open worker prepares the first-line
display index for buffers <=10 MiB. Its parallel workers borrow UTF-8 segments;
sparse checkpoints stay shared with snapshots. A complete tab-free index can
shift byte/scalar/cell/grapheme checkpoints after small printable-ASCII edits
between ASCII neighbors. Unicode joins, controls, tabs and incomplete indexes
follow conservative invalidation. Other lines remain lazy; preparation and
file-open completion have separate timing from UI key response.

Current Agent extension: `guinea-service` holds typed operations, policy,
bounded structured analysis and a live UI command channel; `guinea-agent`
builds the separate JSON CLI and official rmcp stdio adapter. The TUI remains
the sole document state owner. Service workers prepare shared snapshots and
perform filesystem/parse/serialization work; UI commits swap shared buffers
with revision checks and preserve existing histories. AI leases lock only the
owned source and linked reviews and release through RAII. Local IPC is
authenticated owner-only named pipes on Windows or Unix sockets, enabled only
by --agent. See AGENT_IMPLEMENTATION.md for limits, cancellation and security
scope. Earlier MVP worker/latency notes below describe their original stage;
the current active-job loop polls at 8 ms and idle waits up to 250 ms.

`guineanote` parses file/workspace-directory arguments using Clap, loads
explicit TOML configuration and enters the TUI. Workspace crates:

| Crate | Responsibility |
| --- | --- |
| guinea-core | TextBuffer, private Ropey storage, coordinates, Document, selection/history/search |
| guinea-io | Strict UTF-8/BOM reads, file metadata, fingerprints and replacement writes |
| guinea-tui | Application commands, tabs, viewport, rendering, dialogs and terminal/input lifecycle |
| guinea-syntax | Extension detection and revision-tagged basic highlight spans |

The redesigned TUI adds an Explorer model and a separate workspace renderer.
Explorer caches shallow directory listings and expanded paths; one read-only
background scan runs at a time. A generation counter rejects results from old
workspace roots. Symlink entries are displayed without traversal and each
listing is capped at 10,000 entries with visible feedback. No recursive startup
scan or filesystem mutation is introduced.

UI focus is explicit (editor/explorer). The renderer publishes rectangles for
tabs, toolbar/welcome actions, tree rows and text cells. Mouse caret placement
converts terminal cells to checked grapheme boundaries through Document.select.
Windows mouse records are translated using the existing safe native wrapper;
Unix uses Crossterm. Mouse capture is configurable and keyboard paths remain.
Cleanup attempts mouse restoration separately so a mouse-state error cannot
skip alternate-screen/cursor/raw-mode restoration.

The navy theme with green/walnut/warm-white content has true-color and 256-color
variants, honors NO_COLOR and needs no patched font. At narrow sizes the sidebar
hides and the editor switches to a compact layout. The welcome wordmark is ASCII
for font portability.

UI code cannot access a `ropey::Rope`. Document exposes an immutable buffer
interface and editing commands. TextBuffer uses scalar indices and checked
ranges, allocating owned strings for returned lines/slices. Rope editing and
indexing are logarithmic plus text length. Rendering and navigation still copy
owned lines, but GraphemeCursor checks/traverses candidate boundaries without
allocating a grapheme index for the entire line. Position/cell calculations
scan only the prefix through the cursor. A cursor far along a very long line
is therefore still an expensive workload. Search uses a string snapshot and checks candidate match
boundaries with GraphemeCursor; it does not allocate an index for every grapheme.

The coordinate module has distinct ByteOffset, CharIndex, GraphemeColumn,
CellColumn and Utf16Offset types. Document selection indices are scalar indices
whose endpoints must be grapheme boundaries. Rendering accounts separately for
tab stops, terminal width, clipping and safe control-character presentation.
No LSP runtime or UTF-16 protocol integration is present.

Each Document has a unique id, optional absolute path, Ropey buffer, UTF-8 BOM
flag, readonly flag, fingerprint, revision, newline insertion policy, selection,
saved buffer snapshot, and undo/redo transactions. UTF-8 is the only editable
encoding, so there is no misleading configurable encoding enum. Dirty state
compares content and BOM to the saved snapshot; it remains correct after undo,
redo and branching. Comparisons can be linear in content length. Rope snapshots
share storage but retaining history is not bounded memory.

The event loop owns application state. One worker at a time receives a Document
clone or path and returns a completed open/save/search/replace transaction.
Edits and tab changes pause while a job runs. The UI continues handling resize
and rendering status. Worker panic payloads propagate to the main thread;
the terminal panic hook restores state and invokes the previous hook.

On Unix, input uses Crossterm poll/read. On Windows, a bounded channel carries
events from a blocking `crossterm_winapi` safe wrapper reader. This avoids the
Crossterm 0.29 surrogate-pair/key-up issue observed in native ConPTY testing.
The decoder ignores releases before pairing UTF-16 units, respects repeat
counts and distinguishes AltGr text from Ctrl shortcuts. Application code has
`unsafe_code = forbid`; the OS wrappers belong to the dependency.
The Windows input thread is process-lifetime and blocked at idle; the binary
creates one editor session per process.

Ratatui renders only after changes; it diffs terminal output. The loop waits
up to 250 ms for events (four idle wakeups/sec). File job completion latency can
therefore be up to 250 ms even when filesystem work completes sooner. Syntax
is synchronous for the prefix needed through the viewport's right edge, uses
revision-tagged results, and rejects revision mismatches. Lexing starts at the
line's beginning to retain string/comment context, and stops before invisible
suffixes. Multiline parsing, incremental caches and cancellation of
in-flight jobs remain future work.
