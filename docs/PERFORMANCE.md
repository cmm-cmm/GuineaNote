# Performance measurements

## Current expanded formats: below 100 ms, 2026-11-09

Final release after the ENV/config/source expansion and replacement of the YAML
parser: **138 samples per run, two runs, 276 total; max 57.719 ms; no sample
>=100 ms** on the same local 10 MiB corpus/Windows host. Raw samples:
[expanded run 1](responsiveness-expanded-run1.json),
[expanded run 2](responsiveness-expanded-run2.json).

The original workloads below now additionally test YAML, JSON5/JSONC, ENV,
INI, Properties, XML/YAML config inference, notebook cells, generic code,
extensionless Python scripts and directory collapse. Source review supports
all UTF-8 files; naming/coloring support is not a full grammar for every
programming language. Added parsers remain off the input thread.

| Workload | Samples | p50 ms | p95 ms | Max ms |
| --- | ---: | ---: | ---: | ---: |
| ascii | 60 | 19.754 | 21.409 | 22.294 |
| unicode | 60 | 3.492 | 6.331 | 44.337 |
| note.md | 6 | 5.036 | 7.489 | 7.489 |
| data.json | 6 | 4.295 | 9.721 | 9.721 |
| data.xml | 6 | 6.530 | 7.592 | 7.592 |
| data.csv | 6 | 6.842 | 15.592 | 15.592 |
| data.tsv | 6 | 8.281 | 12.495 | 12.495 |
| data.toml | 6 | 4.877 | 5.812 | 5.812 |
| broken.json | 6 | 5.601 | 5.898 | 5.898 |
| config.yaml | 6 | 7.222 | 8.230 | 8.230 |
| config.json5 | 6 | 5.992 | 7.991 | 7.991 |
| .env.local | 6 | 8.072 | 11.393 | 11.393 |
| settings.ini | 6 | 6.070 | 7.736 | 7.736 |
| app.properties | 6 | 7.014 | 8.116 | 8.116 |
| application.config | 6 | 6.343 | 7.235 | 7.235 |
| .prettierrc | 6 | 4.978 | 5.920 | 5.920 |
| example.ipynb | 6 | 6.232 | 7.252 | 7.252 |
| algorithm.futurelang | 6 | 6.664 | 7.719 | 7.719 |
| script | 6 | 5.959 | 7.571 | 7.571 |
| large-json | 14 | 5.757 | 10.558 | 10.558 |
| workflow | 40 | 9.278 | 49.324 | 57.719 |

Full 10 MiB JSON review completion, including repeated locale switches, took
777.682 / 779.545 ms in the final runs; interactive feedback is separately
bounded by the sample table. The generic source painter now walks ordered
highlight spans once, instead of rescanning every span per grapheme. Repeated
unsupported typing in read-only review gives one notice rather than redrawing
for every ignored key. Prefix/shebang detection copies at most 160 scalars.

An earlier widened run before these changes recorded multiple failures,
including a 3,325.952 ms typed-path burst; it is preserved as
[expanded earlier run](responsiveness-expanded-before.json). Host/PTY scheduling
and unrelated load vary; the final passing measurements do not establish a
hard real-time guarantee on arbitrary machines or inputs. No timing threshold
was raised or disabled. The script still fails on any measured sample >=100 ms.

Final validation: 69 Rust tests passed; required fmt/strict Clippy/release build,
Linux/macOS cross-Clippy and workspace dependency audit passed. Runtime/latency
on Linux/macOS remain unverified. CI remains disabled during development.

## Initial interaction budget: below 100 ms, 2026-11-09

The user's final requirement supersedes the initial 150 ms target. Scope:
UTF-8 inputs up to 10 MiB on this Windows/MSVC development host (hardware below),
release profile. Native ConPTY + real editor input/output + pyte parsing;
physical monitor/font latency is not included. Build/test processes were allowed
to finish before the final two runs. OS scheduling, antivirus and unrelated host
load are not controlled; these measurements are not universal certification.

Reproduce locally (no GitHub CI):

```powershell
cargo build --release
python -X utf8 scripts/pty_responsiveness.py 10485760
```

The script fails if **any** measured interaction is >=100 ms. Two final runs:
107 fresh screen transitions each, **214 total, maximum 49.018 ms**, no failures.
Raw samples: [run 1](responsiveness-10m-run1.json),
[run 2](responsiveness-10m-run2.json). Sorted sample-rank percentiles:

| Workload | Samples | p50 ms | p95 ms | Max ms |
| --- | ---: | ---: | ---: | ---: |
| ascii | 60 | 20.621 | 28.984 | 39.009 |
| unicode | 60 | 3.463 | 7.320 | 42.170 |
| note.md | 6 | 6.953 | 7.355 | 7.355 |
| data.json | 6 | 7.959 | 10.802 | 10.802 |
| data.xml | 6 | 6.286 | 12.703 | 12.703 |
| data.csv | 6 | 9.518 | 11.672 | 11.672 |
| data.tsv | 6 | 9.531 | 10.781 | 10.781 |
| data.toml | 6 | 8.615 | 10.109 | 10.109 |
| broken.json | 6 | 8.961 | 10.209 | 10.209 |
| large-json | 14 | 5.472 | 7.031 | 7.031 |
| workflow | 38 | 8.879 | 35.601 | 49.018 |

ASCII is one 10 MiB line. Unicode repeats Vietnamese, NFD accents, CJK and ZWJ
emoji in one line up to 10 MiB; the fixture has 11 graphemes per unit. Operations
cover typing at start, End, Left/Right at end, insert/undo/redo/backspace at end,
Home, help/dismissal and locale switches. Review cases include Markdown,
JSON, XML, CSV, TSV, TOML and invalid JSON. Large JSON runs background review
while switching locales. Workflow cases cover tabs, clipboard, history, dirty
close/cancellation, prompts, find feedback, sidebar/focus, opening a folder,
expanding a directory and dirty quit. Saves check exact bytes and clean terminal
exit. These samples do not cover every possible command/content/terminal.

Full 10 MiB JSON review completion, including repeated locale changes, took
**790.227 / 778.185 ms** in the two runs. Interactive review feedback and locale
switches remained within the table's budget. Parse completion, disk sync,
network filesystems, cold filesystem cache and arbitrary Unicode clusters do
not have a universal 100 ms completion guarantee. Review display is capped at
100,000 rows; full source remains editable.

The probe found real regressions before the final implementation:

- Before parallel indexing, cold End on the Unicode line took 153.8 ms:
  [raw earlier run](responsiveness-before-parallel.json).
- A pasted/rapidly typed path caused one redraw per character, giving a
  222.728 ms folder response: [raw earlier run](responsiveness-before-input-batch.json).
- The four-worker cold index showed a 133.078 ms sample during variable host
  load: [raw variance run](responsiveness-cold-index-variance.json).

Fixes: borrowed chunk grapheme navigation, constant-time history savepoints,
bounded visible strings, sparse display checkpoints with incremental edit/undo/
redo invalidation, cold indices split at verified grapheme boundaries across up
to eight available workers (tab stops recombined exactly), and bounded input
batches (128 events or 4 ms) before drawing. File/review worker delivery polls
at 8 ms while active; idle waits return to 250 ms and do not redraw. Background
save completion preserves typing made after the committed snapshot.

64 Rust tests, fmt and strict Clippy pass. Linux x64 and macOS ARM64 Clippy
cross-checks pass; native runtime/latency on those OSes is still unverified.
Normal edit/workspace PTY and panic terminal restoration pass on Windows.

The measurements below are historical checkpoints and retain their original
limitations and implementation descriptions.

## Earlier baseline

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
