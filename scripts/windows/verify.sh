#!/usr/bin/env bash
# Native Windows/ConPTY verification (Git Bash), the counterpart of
# scripts/linux/run.sh. Run from the repository root with nothing else
# building or probing:  bash scripts/windows/verify.sh [OUT]   (default target/windows-out)
# Release build, then every PTY probe and the latency probes sequentially.
# The clipboard probe uses the real clipboard and restores its previous text;
# set SKIP_SYSTEM_CLIPBOARD=1 to leave the clipboard alone.
set -u
OUT="${1:-target/windows-out}"
mkdir -p "$OUT"
: > "$OUT/summary.txt"
cargo build --workspace --release > "$OUT/build.txt" 2>&1 || { echo "build failed"; exit 1; }
sha256sum target/release/guinea.exe target/release/guineanote-agent.exe target/release/guineanote-mcp.exe > "$OUT/binary.sha256"
git rev-parse HEAD > "$OUT/head.txt"
git status --short > "$OUT/git-status.txt"
run() {
  local name="$1"; shift
  local start=$(date +%s)
  python -X utf8 "$@" > "$OUT/$name.txt" 2>&1
  local code=$?
  echo "$name exit=$code seconds=$(( $(date +%s) - start ))" | tee -a "$OUT/summary.txt"
}
report() { cp "$(cat "target/$1-latest-path.txt")/report.json" "$OUT/$2-report.json" 2>/dev/null; }
run unittest-observer -m unittest discover -s scripts -p test_pty_observer.py
run smoke scripts/pty_smoke.py
cargo build -p guinea-tui --example panic_cleanup > "$OUT/build-panic.txt" 2>&1
run panic-cleanup scripts/pty_smoke.py --panic target/debug/examples/panic_cleanup.exe
run keyboard scripts/pty_keyboard.py
run dialogs-reader scripts/pty_dialogs_reader.py
run search scripts/pty_search.py --trace
run tabs scripts/pty_tabs.py
run dialog-mouse scripts/pty_dialog_mouse.py
run agent scripts/pty_agent.py
run recovery scripts/pty_recovery.py
run session scripts/pty_session.py
run mark scripts/pty_mark.py
run regex scripts/pty_regex.py
run typeahead scripts/pty_typeahead.py
run terminal-matrix scripts/terminal_matrix.py
run latency-trace scripts/pty_interaction_latency.py; report interaction latency-trace
run latency-no-trace scripts/pty_interaction_latency.py --no-trace; report interaction latency-no-trace
run navigation scripts/pty_navigation.py --trace --record-only; report navigation navigation-all
F=unprepared-12mib,unprepared-line-101,unprepared-after-newline
run navigation-unprepared-2cpu scripts/pty_navigation.py --trace --record-only --fixtures $F --affinity 0,1
report navigation navigation-unprepared-2cpu
run navigation-unprepared-1cpu scripts/pty_navigation.py --trace --record-only --fixtures $F --affinity 0
report navigation navigation-unprepared-1cpu
if [ -z "${SKIP_SYSTEM_CLIPBOARD:-}" ]; then
  run clipboard scripts/pty_clipboard.py --use-system-clipboard
else
  echo "clipboard skipped" | tee -a "$OUT/summary.txt"
fi
cat "$OUT/summary.txt"
