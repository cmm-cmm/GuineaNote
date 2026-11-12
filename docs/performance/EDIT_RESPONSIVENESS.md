# Edit command responsiveness — 2026-11-12

Goal: no edit command should hold the input thread long enough to freeze the
UI. Measured with `cargo run --release -p guinea-core --example edit_profile`
(core only, median/max of three runs, document construction not timed) on a
10,485,792-byte, 307,384-line CRLF corpus with Vietnamese, NFD and emoji
text, and a 10 MiB single-line Unicode corpus for Up/Down. Linux x86_64 cloud
container, Rust 1.99.1 release build.

## Before / after (median, ms)

| Command (whole document) | Before | After | Where it runs now |
| --- | ---: | ---: | --- |
| toggle-comment | 433 | 30 | worker above 1 Mi scalars |
| indent (Tab on the selection) | 328 | 17 | input thread |
| sort-lines | 111 | 54 | worker above 1 Mi scalars |
| remove-duplicate-lines | 115 | 33 | worker above 1 Mi scalars |
| remove-empty-lines | 29 | 23 | worker above 1 Mi scalars |
| trim-trailing | 29 | 20 | worker above 1 Mi scalars |
| upper-case (select all) | 55 | 60 | worker above 1 Mi scalars |
| Up/Down into a 10 MiB Unicode line, column 3 | 6.6 | 0.01 | input thread |
| Up into a 10 MiB Unicode line from its end | 133 | 0.01 | input thread |
| Down from the end of an unindexed 10 MiB line | 73 | 48 (core) | queued in the TUI until the layout worker indexes the line |

Up/Down "before" values are single runs of the same moves on the previous
code; all other values are medians of three.

Typing, Backspace, paste of 1 MiB, select-all + type, duplicate line, Undo
of a whole-document edit and copying 10 MiB were already below 3 ms and are
unchanged.

## Causes and fixes

- **Per-line rope queries.** Indent and comment asked the rope for every
  line's bounds and text (≈0.43 µs + a small allocation per line) and then
  applied one rope edit per line after a grapheme-boundary check per edit.
  They now read the selected block once, rewrite it as a string and splice it
  with two rope edits (`Document::replace_block`), keeping bookmarks on their
  lines and one Undo step. `apply_edits` with more than 32 edits (remove
  bookmarked lines, Agent patches) also composes the span in one pass.
- **Per-line allocations.** Sort, dedupe, trim and remove-empty used one
  `String` per line; they now work on borrowed slices (`sort_unstable`,
  `HashSet<&str>`).
- **Unavoidable whole-text work.** Sorting and Unicode case mapping of 10 MiB
  still take ~55–60 ms. Block commands (line tools, case, comment) whose
  scope exceeds 1,048,576 scalars therefore run on the compute worker: the UI
  keeps drawing ("Sorting lines… edits paused"), the result commits as one
  Undo step, and smaller scopes still apply immediately.
- **Whole-line copies for Up/Down.** Moving onto a long line copied and
  segmented the whole target line to find the goal column.
  `Document::grapheme_column_to_char` now returns at once when the column
  exceeds the line's scalars, uses a cached display index when one covers
  the column, and otherwise segments a prefix that doubles until it reaches
  the column.
- **Goal column on an unindexed line.** Leaving a caret deep in an unindexed
  multi-MiB line still needs the caret's grapheme column. The TUI now queues
  Up/Down/PageUp/PageDown while the layout worker builds that index ("Indexing
  the long line… the caret follows") and applies them when it lands; any other
  input applies queued moves first so nothing is reordered. Queues are capped
  at 4,096 moves.

## Limits

Core timings exclude rendering and terminal I/O; end-to-end PTY results are
in `IMPLEMENTATION_STATUS.md`. The worker path pauses edits (not drawing)
until it finishes. Windows was not re-measured for this change.
