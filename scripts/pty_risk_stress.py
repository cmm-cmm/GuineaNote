"""10 MiB native UI feedback after more edits than the Undo transaction cap.

RSS observations include a sampling observer and are not allocator bounds.
All fixtures are private disposable files. No disk source is saved.
"""
import json
import os
import platform
from pathlib import Path
import tempfile
import threading
import time
from pty_smoke import ROOT, Session
from pty_responsiveness import action, RESULTS

BINARY = ROOT / "target/release" / ("guinea.exe" if os.name == "nt" else "guinea")


def main():
    import psutil
    with tempfile.TemporaryDirectory(prefix="guineanote-history-stress-") as directory:
        root = Path(directory)
        source = root / "history.rs"
        unit = "Việt e\u0301 界 👩‍💻 "
        content = "header\n" + unit * ((10*1024*1024-7)//len(unit.encode()))
        original = content.encode()
        source.write_bytes(original)
        ui = Session([str(BINARY), str(source)])
        stop = threading.Event()
        observer = None
        rss = []
        try:
            ui.wait("Opened")
            editor = next(p for p in psutil.Process(ui.pid).children(recursive=True)
                          if p.name().casefold() == BINARY.name.casefold())
            def observe():
                while not stop.wait(.005):
                    try:
                        rss.append(editor.memory_info().rss)
                    except psutil.Error:
                        break
            observer = threading.Thread(target=observe)
            observer.start()
            before = editor.memory_info().rss
            start = time.perf_counter()
            ui.send("x" * 2400)
            ui.wait("Ln 1, Col 2401 ", timeout=30)
            bulk_ms = round((time.perf_counter()-start)*1000,3)
            # Bulk typing is throughput, not one interactive sample.
            action(ui, "history:next-key", "y", "Ln 1, Col 2402 ")
            action(ui, "history:undo", "\x1a", "Ln 1, Col 2401 ")
            action(ui, "history:redo", "\x19", "Ln 1, Col 2402 ")
            action(ui, "history:details-open", "\x1b[20~", "FILE DETAILS · F9")
            action(ui, "history:details-close", "\x1b", lambda s: "FILE DETAILS · F9" not in s)
            after = editor.memory_info().rss
            ui.send("\x11")
            ui.wait("Discard ALL")
            ui.send("y")
            ui.wait("TERM_STATE_OK")
            ui.wait("CHILD_EXIT=0")
            assert source.read_bytes() == original
            report = {"platform": platform.platform(), "file_bytes": len(original),
                      "typed_transactions": 2401, "bulk_2400_keys_ms": bulk_ms,
                      "rss_before_bytes": before, "rss_after_bytes": after,
                      "peak_observed_rss_bytes": max(rss or [before, after]),
                      "rss_samples": len(rss), "sample_interval_ms": 5,
                      "samples": RESULTS, "max_ms": max(r["ms"] for r in RESULTS),
                      "failures": [r for r in RESULTS if r["ms"] >= 100],
                      "source_unchanged": True}
            (ROOT / "target/risk-history-stress.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
            print(json.dumps(report,indent=2))
            assert not report["failures"]
        finally:
            stop.set()
            if observer:
                observer.join()
            (ROOT / "target/pty-risk-stress.log").write_bytes(ui.output)
            ui.close()


if __name__ == "__main__":
    main()
