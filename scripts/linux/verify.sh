#!/usr/bin/env bash
# Runs inside the container (see run.sh): refresh /work/src from the host
# archive, then the local gate and every PTY probe, sequentially. Results go to
# /out. The container's Xvfb clipboard is private, so the clipboard probe never
# touches the developer's clipboard.
set -u
OUT=/out
mkdir -p "$OUT" /work/src
: > "$OUT/summary.txt"
cd /work/src
find . -mindepth 1 -maxdepth 1 ! -name target -exec rm -rf {} +
tar -xf /host/linux-src.tar -C /work/src
rm -rf /tmp/history
git clone -q --no-checkout /host/linux-src.bundle /tmp/history
mv /tmp/history/.git /work/src/.git
git config --global --add safe.directory /work/src
git reset -q
git rev-parse HEAD > "$OUT/head.txt"
git status --short > "$OUT/git-status.txt"
step() {
  local name="$1"; shift
  local start=$(date +%s)
  "$@" > "$OUT/$name.txt" 2>&1
  local code=$?
  echo "$name exit=$code seconds=$(( $(date +%s) - start ))" | tee -a "$OUT/summary.txt"
  return $code
}
{ uname -a; rustc --version; nproc; head -1 /etc/os-release; } > "$OUT/platform.txt"
step fmt cargo fmt --all -- --check
step clippy cargo clippy --workspace --all-targets -- -D warnings
step test cargo test --workspace
step build-release cargo build --workspace --release || exit 1
step build-panic-example cargo build -p guinea-tui --example panic_cleanup
sha256sum target/release/guinea target/release/guineanote-agent target/release/guineanote-mcp > "$OUT/binary.sha256"
step pip python3 -m pip install --break-system-packages --target target/pty-tools psutil==7.2.2 pyte==0.8.2 Pillow
P="python3 -X utf8"
step unittest-observer $P -m unittest discover -s scripts -p test_pty_observer.py
step smoke $P scripts/pty_smoke.py
step panic-cleanup $P scripts/pty_smoke.py --panic target/debug/examples/panic_cleanup
step keyboard $P scripts/pty_keyboard.py
step dialogs-reader $P scripts/pty_dialogs_reader.py
step search $P scripts/pty_search.py --trace
step tabs $P scripts/pty_tabs.py
step dialog-mouse $P scripts/pty_dialog_mouse.py
step agent $P scripts/pty_agent.py
step recovery $P scripts/pty_recovery.py
step session $P scripts/pty_session.py
step terminal-matrix $P scripts/terminal_matrix.py
Xvfb :99 -nolisten tcp > "$OUT/xvfb.txt" 2>&1 &
XVFB=$!
sleep 1
DISPLAY=:99 step clipboard $P scripts/pty_clipboard.py --use-system-clipboard
kill "$XVFB" 2>/dev/null
report() { cp "$(cat "target/$1-latest-path.txt")/report.json" "$OUT/$2-report.json" 2>/dev/null; }
step latency-trace $P scripts/pty_interaction_latency.py; report interaction latency-trace
step latency-no-trace $P scripts/pty_interaction_latency.py --no-trace; report interaction latency-no-trace
step navigation $P scripts/pty_navigation.py --trace --record-only; report navigation navigation-all
F=unprepared-12mib,unprepared-line-101,unprepared-after-newline
step navigation-unprepared-2cpu $P scripts/pty_navigation.py --trace --record-only --fixtures $F --affinity 0,1
report navigation navigation-unprepared-2cpu
step navigation-unprepared-1cpu $P scripts/pty_navigation.py --trace --record-only --fixtures $F --affinity 0
report navigation navigation-unprepared-1cpu
echo done >> "$OUT/summary.txt"
