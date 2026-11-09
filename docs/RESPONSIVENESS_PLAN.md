# Responsiveness, formats, review and localization

Requested target: every interactive action should respond below 100 ms. Confirmed
measurement corpus: UTF-8 text up to 10 MiB, normal source lines and minified
single lines; record larger workloads separately. This is a measured UI-response
budget, not a claim that arbitrary disk reads/fsync/remote filesystems complete
within 100 ms. Long work must show feedback promptly and execute off the UI loop.
No GitHub CI is introduced; use local unit/property/PTY/benchmark checks.

Implementation sequence:

1. Chunk-aware grapheme navigation, O(1) history savepoint dirty queries, fast
   ASCII coordinate paths, bounded visible line slices, faster worker delivery.
2. Extend extension/name/code-fence language detection and lightweight syntax
   for common programming and textual data formats. Do not execute source or
   silently parse binary formats as UTF-8.
3. Markdown and structured data review in TUI: toggle source/split/preview, headings, emphasis,
   quotes, tasks/lists, links, fenced code, tables. Parse off-thread, cache by
   document id/revision/width/locale/table position, discard stale results, never rewrite source bytes.
   Add Vietnamese/English via startup configuration, CLI and F8.
4. Expand native PTY latency probes to include navigation, edits, tabs, search,
   preview and worker response; report max as well as percentiles. Record exact
   corpus/hardware/build and any failures, including completion times above the
   target. Do not label unmeasured cases as a universal guarantee.

History savepoints identify exact transaction states. Recreating identical
bytes through a new edit branch can conservatively remain dirty until saved;
undo/redo to the savepoint clears dirty. This avoids scanning whole buffers on
every redraw while retaining all existing save-failure protection.

Final evidence: PERFORMANCE.md; two 107-sample native release runs passed
the final <100 ms threshold, with max 49.018 ms. Earlier failing runs are kept.
