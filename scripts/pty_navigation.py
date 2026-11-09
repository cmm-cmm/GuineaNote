"""Cold long-Unicode-line End navigation, followed by cached movement.
Release binaries, owned fixtures, strict UI response budget below 100 ms.
Use --record-only to retain exploratory measurements before optimization.
"""
import json
import os
import platform
from pathlib import Path
import sys
import tempfile
import time
from pty_responsiveness import action, RESULTS
from pty_smoke import ROOT, Session

BINARY = ROOT / "target/release" / ("guineanote.exe" if os.name == "nt" else "guineanote")


def main():
    completions=[]
    with tempfile.TemporaryDirectory(prefix="guineanote-navigation-") as directory:
        root = Path(directory)
        unit = "Việt e\u0301 界 👩‍💻 "
        repeats = 10 * 1024 * 1024 // len(unit.encode())
        fixtures=[("unicode",unit*repeats,repeats*11,6),
                  ("ascii-prefix-bare-cr","a"*(10*1024*1024-4)+"\r界",10*1024*1024-2,2)]
        for label, content, graphemes, runs in fixtures:
            path = root / f"{label}.rs"
            path.write_bytes(content.encode())
            for run in range(runs):
                opened=time.perf_counter()
                ui = Session([str(BINARY), str(path)])
                try:
                    ui.wait("Opened")
                    completions.append({"fixture":label,"process_start_to_opened_ms":round((time.perf_counter()-opened)*1000,3)})
                    action(ui, f"{label}:type-start", "x"*10, "Ln 1, Col 11 ")
                    action(ui, f"{label}:first-end", "\x1b[F", f"Ln 1, Col {graphemes+11} ")
                    action(ui, f"{label}:cached-left", "\x1b[D", f"Ln 1, Col {graphemes+10} ")
                    action(ui, f"{label}:cached-right", "\x1b[C", f"Ln 1, Col {graphemes+11} ")
                    ui.send("\x11")
                    ui.wait("Discard ALL")
                    ui.send("y")
                    ui.wait("TERM_STATE_OK")
                    ui.wait("CHILD_EXIT=0")
                finally:
                    (ROOT / f"target/pty-navigation-{label}-{run}.log").write_bytes(ui.output)
                    ui.close()
        report = {"platform": platform.platform(), "input_bytes_max": 10*1024*1024,
                  "cold_sessions": 8, "samples": RESULTS, "sample_count": len(RESULTS),
                  "background_open_completion":completions,
                  "max_ms": max(r["ms"] for r in RESULTS),
                  "failures": [r for r in RESULTS if r["ms"] >= 100]}
        (ROOT / "target/navigation-responsiveness.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report, indent=2))
        if "--record-only" not in sys.argv:
            assert not report["failures"], "Navigation budget exceeded"


if __name__ == "__main__":
    main()
