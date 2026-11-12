# Unprepared long lines — first End off the input thread (12/11/2026)

## Finding

The open worker pre-indexes only the first 64 lines of documents up to 10 MiB.
A first End (or any far caret move) on a long line **without** a complete
display index ran `indexed_line` → `parallel_index` on the UI thread. The
existing harness never hit this: its corpus was exactly 10 MiB, line 0, already
prepared. New `pty_navigation.py` fixtures cover a 12 MiB line (above the
prepare limit, opened through the large-file prompt), a 5 MiB line at line 101,
and End after a newline edit invalidated the index. `--affinity` pins the editor
to one or two CPUs; Rust's `available_parallelism` still reports 12 on Windows,
so up to 8 index threads compete for the pinned CPUs, like a loaded machine.

This is a plausible mechanism for the historical cold End **916.326 ms** sample,
but that sample has no trace and is **not** closed by this work.

## Change (commit 6942b8b)

- `guinea-core`: `layout_pending` (cheap check), `layout_job` (O(1) rope
  snapshot), `LayoutJob::run` (copies and indexes the line on a worker, leaving
  one CPU free), revision-checked `accept_layout`, bounded `caret_window`.
- `guinea-tui`: when the caret line is pending, the frame shows a caret-anchored
  window and `Col …`; the job starts after the frame is flushed (spawning
  mid-frame delayed one terminal write to 147 ms on two CPUs); one job at a time;
  mouse placement on a pending line is refused with a message.

## Results (Windows x64 release, ConPTY, PTY feedback in ms)

`first-end` is the first frame after End. Completion (exact column) is reported
separately and is background work.

| Fixture | CPUs | Before median / max | After median / max (HEAD 1b533dc) | After completion median |
| --- | --- | --- | --- | --- |
| 12 MiB line | 12 | 38.5 / 39.0 | 11.1 / 13.3 | 46.7 |
| after newline edit | 12 | 30.8 / 31.4 | 16.4 / 16.7 | 50.0 |
| 5 MiB line 101 | 12 | 19.0 / 20.3 | 11.3 / 20.2 | 29.1 |
| 12 MiB line | 2 | 158.8 / 197.4 | 10.7 / 12.0 | 155.5 |
| after newline edit | 2 | 141.9 / 142.1 | 16.4 / 17.8 | 128.7 |
| 5 MiB line 101 | 2 | 75.5 / 104.8 | 10.4 / 10.7 | 74.8 |
| 12 MiB line | 1 | 522.4 / 539.7 | 17.9 / 18.5 | 178.0 |
| after newline edit | 1 | 191.7 / 351.6 | 16.4 / 16.5 | 144.3 |
| 5 MiB line 101 | 1 | 108.1 / 109.0 | 9.7 / 9.9 | 74.3 |

Interleaved A/B in one session (two CPUs, 12 MiB, 3 rounds × 2 sessions):
baseline 128.9–154.4 ms vs fixed 9.2–10.4 ms; the first typing sample was the
same for both (7.0–11.3 ms), ruling out a typing regression seen in one earlier,
non-interleaved run.

Final HEAD runs: no PTY or application sample above 50 ms in any of the three
navigation runs (max 41.4 ms), zero dropped trace rows; general interaction
corpus 2,050 input-to-flush samples, application max 5.2 ms, PTY max 28.1 /
25.7 ms with trace on / off; all 13 native probes passed
([final-probes.txt](../diagnostics/unprepared-lines/final-probes.txt)).

## Remaining

- `goto` to line 101 took 34–41 ms because the worker segmented the whole 5 MiB
  line to find the column. Since 12/11 (later commit) it reads a bounded,
  doubling prefix on the input thread and start-of-line queries on unindexed
  long lines no longer copy the line: 20.5–25.1 ms PTY on 12/2/1 CPUs, all
  navigation samples ≤25.1 ms.
- Investigated: two startup `resize` frames on a one-CPU affinity took 127 and
  148 ms (pre-existing; the baseline had 179 ms). `frame_build` was 0.3 ms; the
  time is the terminal write/flush while the open worker indexes the 10 MiB line
  with up to 8 threads that Windows' `available_parallelism` (12) cannot see are
  pinned to one CPU. Not changed: a single-threaded prepare would delay opening
  (edits are paused until open completes) on normal machines.
- Up/Down from a pending long line still computes the preferred column
  synchronously.
- Completion on one CPU is 74–195 ms; the editor stays responsive meanwhile.
- Linux x86_64 (Debian 12 container, Python PTY, 12/11): navigation including
  the unprepared fixtures max 7.6 / 6.8 / 6.6 ms on 12/2/1 CPUs, no
  investigation ([evidence](../diagnostics/linux-container/manifest.json)).
  macOS remains unverified.

Evidence: [docs/diagnostics/unprepared-lines](../diagnostics/unprepared-lines/manifest.json)
(reports with binary hashes and SHA-256 manifest).
