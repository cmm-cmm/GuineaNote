"""Type-ahead and cancel during background jobs (research 09, P1/P5): keys typed while a
~9 MiB file opens, and while a whole-file sort runs on the worker, must all
land in order instead of being dropped. Saves and checks exact bytes.
Only owned temporary files/preferences change. Run after cargo build --release.
"""
import json
import os
from pathlib import Path
import tempfile
import time
from pty_smoke import ROOT, Session

BINARY = ROOT / "target/release" / ("guinea.exe" if os.name == "nt" else "guinea")
F2 = "\x1bOQ"
CTRL_S, CTRL_Q, END = "\x13", "\x11", "\x1b[F"
QUEUED = b"typing is queued"


def saved(ui, path, expected):
    ui.send(CTRL_S)
    deadline = time.monotonic() + 20
    while path.read_bytes() != expected:
        if time.monotonic() > deadline:
            data = path.read_bytes()
            raise AssertionError(f"saved {len(data)} bytes, head {data[:80]!r}")
        ui.read(.05)


def quit(ui):
    ui.send(CTRL_Q)
    ui.wait("TERM_STATE_OK", timeout=20)
    ui.wait("CHILD_EXIT=0")
    ui.close()


def during_open(root, round_):
    path = root / f"open-{round_}.txt"
    body = "dòng é 👩‍💻\r\n" * 380_000  # ~9 MiB, below the prompt
    path.write_bytes(body.encode())
    typed = f"Việt{round_}+ok "
    ui = Session([str(BINARY), str(path)], preferences_path=root / "preferences.toml")
    try:
        # Sent before the editor has even finished opening the file.
        ui.send(typed)
        ui.wait("MODIFIED", timeout=30)
        saved(ui, path, (typed + body).encode())
        queued = QUEUED in ui.output
        quit(ui)
        return queued
    except BaseException:
        ui.close()
        raise


def during_sort(root):
    path = root / "sort.txt"
    lines = [f"line {n:06} ✓" for n in range(150_000, 0, -1)]
    path.write_bytes("\r\n".join(lines).encode())
    ui = Session([str(BINARY), str(path)], preferences_path=root / "preferences.toml")
    try:
        ui.wait("line 150000")
        ui.send(F2)
        ui.wait("Command (")
        # The sort runs on the worker; the keys after it arrive mid-job. A
        # whole-document sort leaves the caret (here at the start), then End
        # and the text follow in order.
        ui.send("sort-lines\r" + "head " + END + "+tail Việt")
        ui.wait("MODIFIED", timeout=30)
        rows = sorted(lines)
        rows[0] = "head " + rows[0] + "+tail Việt"
        expected = "\r\n".join(rows)
        saved(ui, path, expected.encode())
        queued = QUEUED in ui.output
        quit(ui)
        return queued
    except BaseException:
        ui.close()
        raise


def cancel_sort(root, round_):
    """Esc during a large sort frees the UI and leaves the file unchanged.
    Returns False when the sort finished before Esc arrived (then it must be
    a normal, complete sort)."""
    path = root / f"cancel-{round_}.txt"
    lines = [f"dòng {n:06} ✓" for n in range(500_000, 0, -1)]
    original = "\r\n".join(lines).encode()
    path.write_bytes(original)
    ui = Session([str(BINARY), str(path)], preferences_path=root / "preferences.toml")
    try:
        ui.wait("dòng 500000")
        ui.send(F2)
        ui.wait("Command (")
        ui.send("sort-lines\r")
        ui.wait("Esc cancels")
        ui.send("\x1b")
        ui.wait_until(lambda rows: any("Cancelled" in r or "Done" in r for r in rows), timeout=30)
        cancelled = any("Cancelled; the document is unchanged" in r for r in ui.screen.display)
        if cancelled:
            ui.read(1.0)  # the discarded result arrives meanwhile
            assert not any("MODIFIED" in r for r in ui.screen.display)
            assert path.read_bytes() == original
        if cancelled:
            quit(ui)
        else:
            ui.send(CTRL_S)
            ui.wait("● Saved")
            assert path.read_bytes() == "\r\n".join(sorted(lines)).encode()
            quit(ui)
        return cancelled
    except BaseException:
        ui.close()
        raise


def main():
    with tempfile.TemporaryDirectory(prefix="guineanote-typeahead-") as directory:
        root = Path(directory)
        opens = [during_open(root, n) for n in range(3)]
        sort = during_sort(root)
        cancels = [cancel_sort(root, n) for n in range(3)]
    assert any(cancels), "Esc never arrived while the sort ran"
    # Exact bytes prove nothing was lost; the status line proves the input
    # really arrived while a job ran (in at least one round per scenario).
    assert any(opens), "no round typed while the open job ran"
    assert sort, "the typing did not arrive while the sort ran"
    print(json.dumps({"typeahead": {"open_rounds": len(opens), "open_queued": sum(opens),
                                    "sort_queued": sort, "esc_cancelled": sum(cancels),
                                    "exact_bytes": True}}))


if __name__ == "__main__":
    main()
