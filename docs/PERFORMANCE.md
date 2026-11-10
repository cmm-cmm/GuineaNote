# Performance measurements

Current development state: **paused by user**. Latest ASCII/tabbed long-line
results, failed samples, RAM trade-off and restart notes are in the
[pause checkpoint](performance/ASCII_LAYOUT_PAUSE.md). Existing measurements below
remain historical; no universal SLA is inferred.

## Current interaction targets — 2026-11-10

The adopted goals are p50 <=10 ms, p95 <=20 ms and p99 <=50 ms per operation,
corpus and phase. Every interaction >50 ms requires investigation, even when
group percentiles pass. This supersedes the previous 100 ms warning. See
[targets](performance/INTERACTION_LATENCY_TARGETS.md),
[method](performance/BENCHMARK_METHODOLOGY.md) and
[new baseline](performance/LATENCY_BASELINE.md). Historical mixed PTY results
below are retained; they are not interchangeable with application input-to-flush.
Cold End 916.326 ms remains unexplained. No global SLA or native Unix/SSH
certification is claimed; no GitHub Actions is activated.

Latest cache/menu work and raw before/after evidence:
[50 ms investigation report](performance/LATENCY50_IMPROVEMENTS.md).

## Risk mitigation, 2026-11-10

Final Windows x64/MSVC release, sequential probes after builds. Corpus <=10 MiB.

| Workload | Samples | Max UI ms |
| --- | ---: | ---: |
| General editor/review/dialog/tree | 196 | 36.155 |
| Tabs/overflow manager | 26 | 44.172 |
| Dialog/mouse | 15 | 33.523 |
| Find/replace feedback | 13 | 70.618 |
| Human UI during MCP analysis | 28 | 14.348 |
| Navigation with trace / without trace | 52 / 52 | 22.873 / 18.905 |
| After 2401 history transactions | 5 | 12.242 |

387/387 latest responses are <100 ms. Mixed-corpus nearest-rank p95/p99 are
31.525/34.637 ms, descriptive only. Twenty-four fresh navigation sessions include
a second long Unicode line after editing its short predecessor. Traced runs
record zero dropped samples. ConPTY transport and pyte screen parsing are
included; physical paint is not. The historical 916.326 ms cold End remains
unexplained; new passing samples do not remove that failure.

[Raw reports, platform and binary hash](diagnostics/risk-after/validation.json).
[Risk analysis and limits](RISK_MITIGATION.md). An initial ~500 ms exit failure
is retained: tracing found unanswered ConPTY cursor-status requests in the PTY
emulator. Answering pyte write_process_input fixed this separate protocol issue;
raw-mode restore changed from ~512 ms to 0.159 ms in the diagnostic run. This is
not evidence of the cause of the old cold End outlier.

History stress observed 53219328-byte RSS peak over 41 samples (observer requested
5 ms intervals), not an allocator bound. Bulk 2400-key completion was 524.933 ms,
reported as throughput. The following five individual feedback samples passed.
Small-buffer idle: 2.005 s, 0 CPU seconds and 0 output bytes; RSS 7524352 bytes.

Background completion remains separate. In this run 10 MiB replace-all completed
in 193.014 ms and MCP analysis in 1418.957 ms. The existing 21-sample core/I/O
microbenchmark records 10 MiB warm open p50/p95 107.817/126.219 ms, safe save
100.931/137.572 ms, random edit+Undo 0.012/0.018 ms and absent search
15.575/18.331 ms. See [CSV](diagnostics/risk-after/editor-benchmark.csv).
Opening here excludes TUI prewarm, startup, cold disk and physical paint.
No Rust-versus-TypeScript speed factor is inferred.

Earlier sections below are retained historical checkpoints.


## Dialog, mouse and find/replace, 2026-11-09

Native Windows 11/MSVC release; sequential probes after builds completed.
Input corpus reaches 10 MiB. Measurements include ConPTY transport and Python
screen parsing, not physical display paint. Background completion is separate.

| Workload | Samples | Max UI ms |
| --- | ---: | ---: |
| General editor/review/dialog/tree corpus | 196 | 54.066 |
| Tab controls and overflow manager | 26 | 37.034 |
| Human UI during live MCP JSON analysis | 28 | 8.599 |
| Redesigned dialogs and native mouse actions | 15 | 26.757 |
| Find/replace including 10 MiB search and replace-all feedback | 13 | 64.349 |

These **278 transitions passed** the strict <100 ms threshold. Raw samples:
[general](responsiveness-dialog-search-general.json),
[tabs](responsiveness-dialog-search-tabs.json),
[Agent](responsiveness-dialog-search-agent.json),
[dialog/mouse](responsiveness-dialog-search-dialogs-mouse.json),
[search](responsiveness-dialog-search-search.json).

The initial fresh-session navigation probe had **one 916.326 ms first-End
outlier among 32 samples**. Its cause is not established; it must not be
discarded or counted as a passing budget check. Four subsequent independent
32-sample runs did not reproduce it: **128 responses, max 14.848 ms**. All
**438 transitions in the primary run include 437 below budget and this one failure**;
therefore a universal <100 ms claim is not supported. Evidence is retained in
[initial navigation](responsiveness-navigation-dialog-outlier.json) and
repeat runs [2](responsiveness-navigation-dialog-run2.json),
[3](responsiveness-navigation-dialog-run3.json),
[4](responsiveness-navigation-dialog-run4.json),
[5](responsiveness-navigation-dialog-run5.json).

After the final help-text/Vietnamese error-copy update and release rebuild,
the search probe passed again: **13 responses, max 67.142 ms**. Search/page/
replace-all completion was 118.380/71.676/151.381 ms. The native Unicode smoke
probe also passed again. [Final copy recheck](responsiveness-dialog-search-final-copy.json)
retains this additional run. Across all runs, **451 measured transitions have
450 below budget and one unexplained failure**; repeats do not erase it.

10 MiB search completion was **122.987 ms**, previous-page completion
**77.676 ms**, and replace-all completion **147.707 ms**. The query has nearly
a million matches; scanning counts all but retains only 200 per page. Worker
concurrency is capped at two and query changes cancel/coalesce stale work.
General JSON review completed in **396.049 ms**; concurrent Agent analysis in
**877.566 ms**. These single-run completion observations are not a statistical
throughput comparison. Search/replacement preparation never scans the full
buffer in the key/mouse handler, and applying a prepared edit swaps shared
buffer state after revision/access checks.

Reproduction and functional scope are in [TEST_PLAN.md](TEST_PLAN.md).
[Validation manifest](dialog-search-validation.json) records checks and limits.
The darker theme/dialog layout does not introduce animation or periodic redraw.
The native smoke probe observed 0.0 CPU seconds and no output over 2.012 idle
seconds for a small buffer (RSS 6,500,352 bytes); this is not a peak-memory claim.

## Previous prepared navigation and amber workspace, 2026-11-09

Native Windows 11/MSVC release, sequential local probes after all builds.
Navy surfaces remain unchanged; the warm foreground is now honey amber.

| Workload | Samples | Max UI ms |
| --- | ---: | ---: |
| General editor/review/dialog/tree corpus, up to 10 MiB | 196 | 71.884 |
| Tab controls and overflow manager | 26 | 56.564 |
| Human UI while MCP analyzes a 10 MiB live JSON array | 28 | 22.443 |
| Eight fresh sessions, typing and first End/cached movement | 32 | 43.349 |

All **282 UI responses are strictly <100 ms** on this finite native corpus.
Raw data: [general](responsiveness-workspace-amber.json),
[tabs](responsiveness-tabs-amber.json), [concurrent Agent](responsiveness-agent-amber.json),
[prepared navigation](responsiveness-navigation-prepared.json).
General JSON review completion is **871.494 ms**; Agent JSON analysis completion
is **2,270.936 ms**, both separately from initial UI feedback. These values do
not establish a universal latency guarantee or a parse-time speedup.

Six fresh Unicode sessions previously had first-End responses of 35.945–68.894
ms. An initial lazy-index change still reached 98.888 ms in the expanded probe,
so it was followed by preparing the first-line index in the file-open worker
and preserving it across safe small ASCII edits. The final six first-End
responses are **3.146–10.768 ms**. The index work happens during opening;
process startup through Opened, including PTY/Python-wrapper startup, loading
and preparation, takes **219.765–898.510 ms** in this probe. That interval is
reported separately and is not a filesystem-only measurement.

The final 32 samples also include two 10 MiB ASCII-prefix files ending with
bare CR/CJK, initial typing and cached movement. The scanner now inspects only
the needed ASCII prefix plus lookahead and stops at CR, avoiding repeated
full-tail scans. Parallel workers borrow their segments instead of allocating
another full line across workers. Unicode joins, controls/tabs and incomplete
indexes retain conservative invalidation; only verified printable-ASCII cases
reuse a complete tab-free index. Preparation covers the first line of TUI
file opens <=10 MiB; other lines and Agent-created buffers retain lazy indexing.

Exploratory data is retained: [original navigation](responsiveness-navigation-before.json),
[lazy iteration](responsiveness-navigation-lazy.json). Reproduce with
`scripts/pty_navigation.py`, `scripts/pty_responsiveness.py 10485760`,
`scripts/pty_tabs.py` and `scripts/pty_agent_latency.py` after
`cargo build --workspace --release`. No concurrent build/audit/performance probe.
Measurements include PTY transport and Python screen parsing, exclude physical
display paint, and do not verify native Linux/macOS performance.

## Agent/MCP and AI read-only tabs, 2026-11-09

Native Windows 11/MSVC release binaries, owned temporary fixtures, 10 MiB
maximum input. No concurrent compilation, audit or other performance probe.
Each UI sample waits for fresh screen output and includes PTY transport and
Python screen parsing; physical display/font paint is outside this measurement.

| Workload | Samples | Max ms |
| --- | ---: | ---: |
| Existing ASCII/Unicode/editor/review/dialog/tree corpus | 196 | 98.483 |
| Mouse/tab controls, overflow and dirty-close protection | 26 | 25.432 |
| Live 10 MiB JSON analysis with concurrent human UI actions | 28 | 27.395 |

All **250 samples are strictly <100 ms**. The slowest existing-corpus sample
is Unicode End navigation; its 98.483 ms has little headroom, so this is finite
host/workload evidence, not a guarantee for arbitrary hardware or operations.
The concurrent probe checks that all 28 inputs occur while analysis is active:
locked-source navigation/edit feedback, Settings, tab manager, new tab and
typing in another tab. The locked document's contents remain unchanged.

Agent analysis completion: **1,657.802 ms**, including MCP/local IPC and hash,
not part of the UI response budget. Before constructing excerpts only for the
requested page, the same root-array workload took **7,678.205 ms** (28 UI
samples, max 9.842 ms). This single before/after pair observes about 4.6× lower
completion time; it is not a statistical throughput benchmark. The JSON parser
still materializes its input tree; no constant-memory claim is made. Existing
10 MiB JSON TUI review completes in **775.823 ms**, measured separately.

Raw evidence: [general regression](responsiveness-agent-regression.json),
[tab controls](responsiveness-agent-tabs.json), [Agent before](responsiveness-agent-before.json),
[Agent final](responsiveness-agent.json). Reproduce after
`cargo build --workspace --release` with `pty_responsiveness.py 10485760`,
`pty_tabs.py` and `pty_agent_latency.py` in scripts. Run sequentially.
Linux/macOS native latency remains unverified. General/tab probes ran before
the final worker-only lazy-excerpt optimization; Agent and live functional
probes were rerun afterward.

## Visible tab management, 2026-11-09

The extended native release corpus adds All tabs opening, filtering and
cancellation on 10 MiB ASCII/Unicode buffers and the multi-tab workflow.
**196 samples, max 53.054 ms, no sample >=100 ms**; full JSON review completion
is **448.020 ms** separately. The dedicated mouse/tab probe adds **26 samples,
max 36.585 ms**, including New, Save feedback, Previous/Next, individual close,
dirty confirmation/cancel and switching hidden tabs through the filtered list.
Raw evidence: [corpus](responsiveness-tabs.json),
[tab controls](responsiveness-tab-controls.json). Reproduce after release build
with `python -X utf8 scripts/pty_responsiveness.py 10485760` and
`python -X utf8 scripts/pty_tabs.py`. Same host/PTY/physical-display limitations
below apply; these 222 passing samples do not certify every input or platform.

Tab labels are generated only for the visible neighborhood, and All tabs caches
metadata for filtering without reading file contents or the filesystem. Dirty
counts exclude linked reviews to avoid counting a source twice. Existing TUI
performance protections remain in place. Agent/MCP was still a proposal at
that previous checkpoint; its current evidence is recorded above.

## Green/brown/white details on the original navy, 2026-11-09

Updated foregrounds, syntax and border accents while preserving the original
canvas, panel and raised backgrounds in true-color and indexed modes. The
unchanged 10 MiB release probe passes **187 samples, max 53.321 ms, zero
>=100 ms**. Background JSON review completion was 738.473 ms separately.
Raw evidence: [palette run](responsiveness-palette.json). The same host and
measurement limits below apply. Existing workspace tests (80) passed during
the palette change, then all 40 TUI tests, strict Clippy, release build and the
native dialogs/reader probe passed after restoring the original backgrounds.

## Current Settings and linked-reader UI: below 100 ms, 2026-11-09

Two native release runs after the dialog/reader redesign: **187 samples per
run, 374 total, max 58.551 ms, no sample >=100 ms**. Combined p50 is 9.507 ms
and p95 is 30.127 ms (sorted sample-rank percentiles). Same Windows/MSVC host,
10 MiB maximum corpus, no concurrent compilation. Raw evidence:
[UX run 1](responsiveness-ux-run1.json), [UX run 2](responsiveness-ux-run2.json).

| Workload | Samples | p50 ms | p95 ms | Max ms |
| --- | ---: | ---: | ---: | ---: |
| ASCII single line, edits/navigation/dialogs | 68 | 9.007 | 24.403 | 32.110 |
| Unicode single line, edits/navigation/dialogs | 68 | 3.633 | 11.010 | 58.231 |
| 17 format reviews, split/tab chooser | 170 | 11.177 | 31.161 | 38.799 |
| Large JSON, Settings during parsing | 28 | 15.904 | 22.190 | 24.085 |
| Tabs/clipboard/prompts/search/tree/quit | 40 | 9.517 | 32.805 | 58.551 |

F8 now opens Settings; language changes use explicit Apply. The probe measures
opening/dismissing Settings, applying VI/EN, opening/dismissing F9 details, and
choosing split or a dedicated review tab with F7. It retains the previous file
formats and fresh-screen checks. Full 10 MiB JSON review completion including
locale changes took **419.093 / 406.744 ms**, reported separately from feedback.
File-list loading, metadata/statistics, preference writes and preview parsing
run in workers. These timings measure input through PTY/emulated screen state,
not physical display latency or a universal bound for every file and host.

`pty_dialogs_reader.py` separately verifies saved preferences on restart,
Apply/Cancel, file filtering/opening, details, both review layouts, outline
navigation, linked live updates, read-only protection and byte-exact source
saves. Its screenshots are under target/previews. Format/strict Clippy/tests/
release build pass (80 Rust tests), as do native edit/workspace/panic probes,
Linux/macOS cross-Clippy and the recorded-snapshot dependency audit. Native
runtime/latency on Linux/macOS remains unverified. No GitHub CI was added.

## Previous expanded formats: below 100 ms, 2026-11-09

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
