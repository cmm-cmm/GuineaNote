# Agent CLI / live bridge / MCP implementation

Branch: `dev`. P0–P4 are implemented and validated on native Windows.
Linux/macOS cross-target strict Clippy passes; native runtime validation on
those platforms remains pending. No GitHub CI. Earlier engineering-day estimates in
AGENT_MCP_PLAN.md describe the proposal, not a runtime guarantee.

Build all binaries with `cargo build --workspace` (or `--release`). The editor
keeps its existing file/folder arguments; Agent tools are separate native Rust
binaries, with no Node runtime, shell execution or HTTP listener.

```powershell
# Explicitly grant local access to this workspace and its unsaved buffers.
cargo run -- --agent --agent-write .
# Read-only Agent access: omit --agent-write.
target/debug/guineanote-agent.exe --root . sessions list --json
target/debug/guineanote-agent.exe --session SESSION_ID tabs list --json
target/debug/guineanote-mcp.exe --session SESSION_ID --access write
```

Agent access is off by default. `--agent` grants reads of live buffers;
`--agent-write` additionally permits planning/applying patches, saving, opening,
creating and closing clean tabs. `--agent-secrets` explicitly permits sensitive
paths. Client `--access write` and the editor grant are both required for writes.
Disk-only CLI/MCP mode is read-only. All tool file paths are workspace-relative.
The session credential directory is always excluded, including with secret access.
`--root` must match the selected editor's workspace, including when connecting
from another directory. Session discovery never silently attaches a client.

The editor exposes a session-qualified document handle and a distinct tab
handle. A source and its linked review share document identity but have
different tab handles. Metadata and page results use schema_version 1.

While a content operation owns a document, its source/review tabs display
**AI** and become read-only. A global AI count remains visible when space
permits. Navigation, copying and work in other tabs continue. A release guard
restores each tab's original read-only state, including on cancellation/error.
Short operations paint their AI state before processing release. Uncancelable
filesystem work finishes its safe commit before releasing ownership; consult
the write ledger after an ambiguous disconnect.

CLI groups: `capabilities`, `files list/info`, `documents read/search/analyze/
open/new/save`, `tabs list/activate/close`, `sessions list`, `patch plan/apply`,
`review`, `transaction`. `request --file -` accepts one typed JSON request on
stdin; `--request-id` supplies a stable write idempotency key. Stdout contains
JSON; --help/argument parsing use standard CLI output. MCP tools use the same
typed contract, require request_id on writes and distinguish execution errors
from JSON-RPC parameter/protocol errors. `get_transaction` takes
**transaction_id**, separate from the current request's request_id.

Content offsets are zero-based Unicode scalar ranges with grapheme endpoints.
Use returned hash with expected_hash between read/search/analysis pages.
Patches have 1–100 non-overlapping edits, <=1 MiB inserted text and a base
revision. Planning publishes a read-only **AI diff excerpt** tab and leaves
source unchanged; apply commits a prepared shared buffer with one undo step.
Saving is separate, revision-checked and reuses strict UTF-8/BOM/newline,
fingerprint/conflict and atomic-save behavior. Dirty tab close is rejected
without silently discarding content. No multi-file atomic transaction is claimed.

Limits: 10 MiB Agent buffers, 64 KiB content pages, 200 entries/page,
512 KiB serialized result data (request a smaller page if exceeded),
2 MiB IPC/MCP frames, 8 local connections, 64 queued jobs, one service worker,
32 pending plans and a bounded 2,048-write idempotency ledger. Request digests
are stored rather than patch bodies. Restarting the editor invalidates handles
and its in-memory ledger. Structured analysis covers Markdown outline,
JSON/notebook root structure, XML elements and CSV/TSV rows; other UTF-8 formats
have a bounded source view. Excerpt fields are explicitly marked.

IPC uses an owner-only Windows named pipe (remote connections disabled) or
Unix socket, owner-only credential files and random session/token identifiers.
Discovery verifies permissions without changing them. GUINEANOTE_SESSION_DIR
can select a dedicated private runtime directory. Credentials never appear in
session lists or tool results. Path policy rejects parent/device/ADS paths,
symlinks/reparse points, out-of-scope documents and sensitive paths without a
grant. Existing filesystem TOCTOU limitations remain; same-user administrator
processes are outside the credential-isolation threat model.

Queued UI commits can be cancelled until the UI owner claims them. Once a
buffer commit starts, the service waits for its actual result; once atomic
filesystem saving starts, it finishes and merges saved metadata even with a
dialog open. Cancellation never claims a started commit was rolled back.
Parsing checks cancellation at stage boundaries; a running parser or filesystem
call is not forcibly interrupted. The lease remains visible until safe cleanup.

The MCP adapter uses locked official rmcp 3.9.0. Native wire probes pass legacy
2025-11-25 initialize and modern 2026-07-28 discovery/per-request metadata,
tools/resources/schema, Unicode, scoped reads and EOF shutdown. This is not
certification for every host application; no named external client app has
been tested yet. Resources return a bounded JSON page; use read_document for
continuation. No prompts, sampling, remote HTTP or subscription capability is
advertised.

Evidence: 96 Rust tests pass (core 19, I/O 12, service 8, syntax 5, TUI 47,
CLI 4, workflow 1). Service tests were rerun after the analysis optimization.
Fmt, native and Linux/macOS cross-target strict Clippy, debug/release workspace
builds and locked dependency audit pass. Native release wire and live probes
pass; see TEST_PLAN.md for reproducible commands. Fixtures/runtime credentials
are owned temporary data.

| Phase | Verified result |
| --- | --- |
| P0 | Typed schema, JSON CLI, paged Unicode read/search/analysis, limits/errors, existing CLI tests |
| P1 | Owner-only native IPC, two editors with identical filenames, explicit root/session selection, isolated handles, unsaved snapshots |
| P2 | MCP 2025-11-25 and 2026-07-28 tools/resources, invalid/oversized requests, EOF, cancellation and typed overload |
| P3 | Plan/diff/apply/save, stale/invalid edits, no-op revisions, one undo, replay/conflict, byte-exact BOM/CRLF/Unicode save, external disk conflict |
| P4 | Junction/device/secret scope tests, queue/timeout/commit races, worker/client death cleanup, dialog-safe metadata, audit, integration guide and concurrent latency |

Latest release corpus: 196 general UI samples, max **98.483 ms**; 26 tab
control samples, max **25.432 ms**; 28 samples while MCP analyzes a 10 MiB live
JSON array, max **27.395 ms**. All 250 are strictly below 100 ms on the recorded
Windows host. Agent analysis completion is **1,657.802 ms**, separately.
See [raw evidence and limits](PERFORMANCE.md) and [Vietnamese guide](AGENT_USAGE.md).
These finite samples do not certify all operations, workloads or platforms.
