# Performance measurements

Measured locally 2026-11-08 on Windows 11 Home Single Language 10.0.26300,
Intel Core i5-1235U (10 cores / 12 logical processors), approximately 15.7 GiB
visible RAM, x86_64-pc-windows-msvc, rustc 1.98.1. Release/bench profile, default
allocator. Other application/build activity was present; these are exploratory
microbenchmarks, not controlled performance certification.

Reproduce:

```sh
cargo bench --bench editor
```

The default datasets are 1 KiB, 1 MiB and 10 MiB. To include 100 MiB:

```powershell
$env:GUINEANOTE_BENCH_SIZES='104857600'
cargo bench --bench editor
```

Each dataset repeats the fixed row in `benches/editor.rs` containing Rust-like
ASCII, Vietnamese, ZWJ emoji, tab and CRLF, truncating only at a UTF-8 boundary.
Each operation runs 21 times; reported p50/p95 are sorted sample ranks, with
no statistical confidence interval. Random edits use a fixed LCG seed and
choose line starts, insert one scalar, compute dirty, then undo. Scrolling
measures 100 core Down+position commands; it does not include terminal rendering.
Open uses the OS warm cache and includes metadata/read/UTF-8/rope/fingerprint.
Save includes hashing checks, sync and replacement, not TUI worker scheduling.
Search looks for a missing literal, including construction of a text snapshot.

| Dataset | Operation | p50 ms | p95 ms |
| --- | --- | ---: | ---: |
| 1 KiB | open, warm cache | 0.289 | 0.415 |
| 1 KiB | save | 8.905 | 10.630 |
| 1 KiB | random line edit + dirty + undo | 0.036 | 0.104 |
| 1 KiB | scroll 100 lines | 0.736 | 1.717 |
| 1 KiB | search absent | 0.002 | 0.002 |
| 1 MiB | open, warm cache | 8.146 | 9.791 |
| 1 MiB | save | 34.883 | 38.513 |
| 1 MiB | random line edit + dirty + undo | 0.044 | 0.293 |
| 1 MiB | scroll 100 lines | 1.277 | 2.080 |
| 1 MiB | search absent | 1.534 | 1.984 |
| 10 MiB | open, warm cache | 77.000 | 100.422 |
| 10 MiB | save | 157.183 | 163.835 |
| 10 MiB | random line edit + dirty + undo | 0.061 | 0.109 |
| 10 MiB | scroll 100 lines | 2.193 | 2.317 |
| 10 MiB | search absent | 27.210 | 31.965 |
| 100 MiB | open, warm cache | 1381.959 | 1405.826 |
| 100 MiB | save | 1319.720 | 1377.143 |
| 100 MiB | random line edit + dirty + undo | 0.066 | 0.119 |
| 100 MiB | scroll 100 lines | 1.962 | 2.263 |
| 100 MiB | search absent | 236.647 | 252.379 |

The raw 100 MiB CSV is `benchmark-results-100m.csv`. Boundary validation was
changed from a full-document boundary vector to candidate-local GraphemeCursor
checks after the first trial exposed high search cost. The table uses the
second, improved run; the subsequent bare-CR navigation correction and save
metadata precheck were validated by tests, without rerunning unchanged workload
timings. No claim is made that every user interaction meets p95 <20 ms.

A native ConPTY release smoke test measured 2.004 s idle, 0.000 CPU seconds at
OS sampling precision, 0 new output bytes, and approximately 5.1 MiB resident
memory for the tiny open buffer (RSS 5,320,704 bytes). This is a short idle
sample, not a peak-memory measurement or proof of zero CPU usage.

Cold-cache open, peak RSS across file sizes, throughput, long-line rendering,
random mid-grapheme-heavy-line edits, end-to-end keyboard-to-paint latency,
and sustained CPU profiling: NOT TESTED. Full-document jobs run in workers but
completion delivery has a 250 ms polling bound. Long-line grapheme scans,
visible-line syntax and dirty comparisons can still affect responsiveness.
Resident Ropey storage, history and snapshots grow with input/workload.
100 MiB is an evaluated workload, not a support guarantee. No 1 GB or
bounded-memory support is claimed.

## M4 continuation: long lines, 2026-11-09

A new repeatable workload uses a single ASCII line of exactly 1 MiB. Each
sample moves Start→Right→Left and queries position, or converts CharIndex(0) to
a terminal cell. Same host and release profile; 21 samples per operation, with
concurrent development/tool compilation, so comparisons are exploratory.

```powershell
$env:GUINEANOTE_BENCH_SIZES='1048576'
cargo bench --bench editor
```

| Single-line 1 MiB operation | Before p50/p95 ms | After p50/p95 ms |
| --- | ---: | ---: |
| Start→Right→Left + position | 666.273 / 841.542 | 3.536 / 5.365 |
| Cell column at start | 117.028 / 125.393 | <0.001 / <0.001 at output precision |

Raw outputs: `benchmark-hardening-before.csv`,
`benchmark-hardening-after.csv`. The fix removes full-line boundary vectors,
uses contextual GraphemeCursor checks, avoids unnecessary full-line scalar
counts, and scans position/cell prefixes only as far as the cursor. Existing
Unicode properties plus a new full-string forward/backward segmentation
property pass. The renderer also limits syntax lexing to the prefix needed
through the viewport's right edge, preserving line-local lexical context.

The native ConPTY latency probe includes actual key input, rendering, PTY
transport and pyte screen parsing. It types 31 single characters at the start
of a single-line Rust file, waits for each fresh column value, saves and checks
exact bytes, then verifies normal terminal cleanup. Reproduce:

```powershell
python scripts/pty_latency.py
```

| Rust single-line bytes | Input→output p50/p95 ms | Max ms | Max sampled RSS | OS peak working set |
| --- | ---: | ---: | ---: | ---: |
| 1,024 | 14.753 / 20.325 | 20.649 | 5,660,672 B | 5,742,592 B |
| 1,048,576 | 25.182 / 36.740 | 37.814 | 8,957,952 B | 11,943,936 B |

Raw evidence is `benchmark-ui-hardening.json` (Windows 11, Python 3.14.10).
These measurements include client/PTY overhead, not physical display paint or
font shaping. They do not establish the <20 ms p95 target: even the small file
slightly exceeds it in this run. They measure the **start** of long lines;
far-right cursor operations still scan prefixes and need further profiling.
Peak memory here covers these two UI workloads, not the earlier 100 MiB corpus.
The earlier NOT TESTED list describes the initial checkpoint; these new narrow
latency/working-set measurements only close the corresponding cases above.
