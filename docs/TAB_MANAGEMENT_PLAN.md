# Tab management improvement plan

Goal: make creating, saving, closing and switching content tabs discoverable
without memorizing shortcuts. Show the active position, total open tabs and
unsaved state. Preserve the existing navy surfaces and green/brown/white details.
Closing a tab does not delete its file from disk.

## Delivery steps

1. **Visible controls**: a local tab toolbar with New, Save, Close, Previous,
   Next and All tabs; active/total counter; dirty count; numbered tabs with
   individual close buttons. Distinguish blank documents and linked reviews.
2. **Overflow navigation**: a searchable All tabs dialog (Ctrl+P / F2 `tabs`)
   with paths, source/review/read-only/unsaved states and switch/save/close/new
   actions. Keep the active tab visible when labels overflow. Retain F6,
   Shift+F6 and Ctrl+Tab; add Ctrl+PageUp/PageDown and Ctrl+T for a new tab.
3. **Safety and verification**: closing dirty sources requires confirmation;
   cancel keeps text/history, closing a review preserves its source, closing
   the final tab leaves a blank document. Saves use existing conflict checks.
   Verify keyboard and mouse, VI/EN, narrow resize, duplicate basenames, hidden
   tabs, linked review, exact saved bytes and the local <100 ms response budget
   on the confirmed corpus up to 10 MiB. No GitHub CI.

## Later improvements

Optional tab reordering/pinning, session restore and save-all/close-others need
separate implementation and failure-policy design. They are not required for
the initial discoverability fix. Keep disk deletion separate from closing tabs.

## Acceptance evidence

All three delivery steps are implemented. The toolbar has active/total count,
New/Save/Close, Previous/Next and All tabs. Each visible tab has a close hit
region; inactive closes keep the active document. Blank names are stable and
numbered, and review labels remain recognizable when clipped. The searchable
manager shows full paths, states and keyboard/mouse actions; a save completion
refreshes metadata while preserving its filter/selection.

Verified locally: 84 workspace Rust tests, strict Clippy and release build.
Four new behavior tests cover inactive dirty close/cancel, review/source safety,
duplicate-path filtering, empty results, history preservation, pending-save
metadata and overflow/resize/hit regions. `scripts/pty_tabs.py` verifies mouse
new/save/previous/next/individual close, dirty cancel, hidden-tab filtering,
review close, last-tab blank, VI/EN and exact disk bytes. Screenshots are actual
terminal captures under target/previews/tab-*.png.

The new native controls probe passes 26 responses, max 36.585 ms; the extended
10 MiB release corpus passes 196 responses, max 53.054 ms. No sample reaches
100 ms. JSON parse completion is 448.020 ms separately. Raw evidence:
[controls](responsiveness-tab-controls.json), [10 MiB corpus](responsiveness-tabs.json).
These are measured Windows workloads, not a universal timing guarantee.
No GitHub CI. Agent/MCP is separately assessed in AGENT_MCP_PLAN.md and remains
a proposal; no MCP endpoint or Agent command runtime was enabled.
