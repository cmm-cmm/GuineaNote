# 09 — Responsiveness and editing: findings and proposals (2026-11-12)

Scope: the owner asked to prioritise UI responsiveness and edit handling.
This report records what was measured on branch `ccr-d3e9f3cf-1d3qa2`
(PR #1, head 82d46f5), what was already fixed, and what is proposed next.
Measurements: Linux x86_64 cloud container (kernel 6.18), Rust 1.99.1
release builds, Python `pty` + pyte probes. Windows was not re-measured.

## 1. Already fixed in this round (82d46f5)

Core timings on a 10 MiB / 307,384-line CRLF corpus
(`cargo run --release -p guinea-core --example edit_profile`); details in
[EDIT_RESPONSIVENESS](../performance/EDIT_RESPONSIVENESS.md).

| Command | Before | After |
| --- | ---: | ---: |
| toggle-comment, whole file | 433 ms | 30 ms (worker above 1 Mi scalars) |
| indent, whole file | 328 ms | 17 ms |
| sort-lines | 111 ms | 54 ms (worker above 1 Mi scalars) |
| remove-duplicate-lines | 115 ms | 33 ms (worker above 1 Mi scalars) |
| Up into a 10 MiB Unicode line | 133 ms | 0.01 ms |
| Down from an unindexed long line | 73 ms | queued until the line is indexed |

End to end after the fix: 1,540 PTY samples of the interaction corpus,
max 9.4 ms; the unindexed long-line navigation corpus (12 MiB line, line
101, after newline) max 6.1 ms; no sample above 50 ms; 12 functional
probes pass.

## 2. New findings

Measured with `cargo run --release -p guinea-io --example io_profile`
(10,485,818-byte file) and by reading `App::handle`.

### F1 — Input is dropped, not delayed, while a background job runs (high)

`App::handle` returns early for every event while `busy()` (open, reload,
compute jobs such as F3, large line tools): keys, pasted text and mouse
events are discarded with "Background operation in progress; edits paused".
Saving is the only exception. Windows where this happens on the 10 MiB
corpus: opening a file ≈ 50 ms, sort/case on the worker ≈ 55–60 ms, F3 ≈
5 ms; session restore opens files one after another. A fast typist or a
paste that lands in that window loses text silently except for the status
message. Notepad++ never drops typed input.

### F2 — One job pauses every tab (medium)

`busy()` is application-wide: opening a large file in a new tab, or sorting
one document, pauses edits in all other tabs although they are not touched.

### F3 — Undo history is charged twice for whole-file edits (medium)

A transaction is charged `removed + inserted` bytes (`before + after` for
prepared edits). Consecutive transactions share their snapshot ropes
(`T(n).after` is `T(n+1).before`), so a whole-file edit is counted twice.
Result on the 10 MiB file: each whole-file edit is charged 20.7 MiB and the
64 MiB budget keeps only **3** whole-file Undo steps (older ones trimmed),
while unique memory per step is ≈ 10 MiB.

### F4 — Whole-file edits grow memory quickly (low/medium)

RSS with one 10 MiB document: 24.7 MiB; after three whole-file indents:
83.7 MiB; it plateaus near 102 MiB once history trimming starts (the
allocator keeps freed pages). Each whole-block rewrite builds a complete new
rope even when only some lines change.

### F5 — Open and save are 45–50 ms for 10 MiB (informational)

`guinea_io::open` ≈ 50 ms and `guinea_io::save` (temp file, sync, rename)
≈ 45–49 ms, both on the file worker. Acceptable, but they are the windows in
which F1/F2 apply.

## 3. Proposals (prioritised)

| # | Proposal | Priority / cost | Acceptance criteria |
| --- | --- | --- | --- |
| P1 | **Queue input during background jobs instead of dropping it.** Keep a bounded FIFO (e.g. 1,024 events, pasted text ≤ 4 MiB) while `busy()`; replay it in order when the job ends; Esc and Ctrl+Q keep their current meaning. If the queue overflows, refuse further input loudly instead of losing it silently. | P0 / S–M | PTY probe types and pastes while a 10 MiB file opens and while a large sort runs; every character lands in order, Undo groups unchanged, nothing above 50 ms |
| P2 | **Per-document jobs.** Track the pending job's document id; only that tab (and its review tabs) pause, other tabs stay editable; opening into a new tab pauses nothing. | P1 / M | Typing in tab A while tab B opens or sorts a 10 MiB file applies immediately; save/close of B still waits for its job |
| P3 | **Charge Undo by unique memory.** Charge a transaction its inserted bytes (plus a constant), and count the oldest retained `before` once. | P1 / S | `io_profile` keeps ≥ 6 whole-file Undo steps on 10 MiB at the same 64 MiB budget; measured RSS stays within budget + one document |
| P4 | **Splice only the changed middle.** In `replace_block`/prepared edits, trim the common prefix and suffix of old and new text before splicing, so ropes share unchanged chunks. | P2 / S | Commenting 1,000 lines of a 10 MiB file allocates O(changed) memory; history charge reflects the changed span |
| P5 | **Cancel worker edits with Esc.** Large sort/case/comment jobs get a cancel flag like search; Esc drops the prepared result. | P2 / S | Esc during a 100 MiB sort returns control within one frame and leaves text and history unchanged |
| P6 | **Re-verify PR #1 on Windows/ConPTY** (all 13 probes, latency and navigation corpora) before merge. | P0 for merge / S | Same probe set passes natively; numbers recorded beside the Linux ones |
| P7 | **Next Notepad++ features (M9)**, by value/cost: change-history margin (lines changed since save, from the saved snapshot), word completion from open tabs, code outline from the lexer, macro record/replay, keymap file. | P2 / M each | Each with Unicode/CRLF tests, F2 entry, VI/EN strings, latency gate |

Recommended order: P1 → P3 → P2 → P6 (before merging PR #1) → P4/P5 → P7.
P1 and P3 are small, fix silent data loss and lost Undo, and need no new
dependency.

## 4. Open decisions for the owner

- Drop-down menu (File/Edit/Search/View) or keep F2 as the entry point.
- Session restore on by default (current) or opt-in.
- Whether Linux/SSH is a release gate for v0.2.

## 5. Reproduce

```sh
cargo run --release -p guinea-core --example edit_profile
cargo run --release -p guinea-io --example io_profile
python -X utf8 scripts/pty_interaction_latency.py
python -X utf8 scripts/pty_navigation.py --trace --record-only \
  --fixtures unprepared-12mib,unprepared-line-101,unprepared-after-newline
```
