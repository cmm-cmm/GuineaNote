"""Cold long-Unicode-line End navigation, followed by cached movement.
Release binaries, owned fixtures, strict UI response budget below 100 ms.
Use --record-only to retain exploratory measurements before optimization.
"""
import csv
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
    traces=[]
    trace_dir = ROOT / "target" / ("navigation-traces-" + str(time.time_ns()))
    if "--trace" in sys.argv:
        trace_dir.mkdir(parents=True)
    with tempfile.TemporaryDirectory(prefix="guineanote-navigation-") as directory:
        root = Path(directory)
        unit = "Việt e\u0301 界 👩‍💻 "
        repeats = 10 * 1024 * 1024 // len(unit.encode())
        two_repeats = (10 * 1024 * 1024 - len("prefix\n")) // len(unit.encode())
        fixtures=[("unicode",unit*repeats,repeats*11,6,1),
                  ("ascii-prefix-bare-cr","a"*(10*1024*1024-4)+"\r界",10*1024*1024-2,2,1),
                  ("second-long-line","prefix\n"+unit*two_repeats,two_repeats*11,4,2)]
        for label, content, graphemes, runs, line in fixtures:
            path = root / f"{label}.rs"
            path.write_bytes(content.encode())
            for run in range(runs):
                opened=time.perf_counter()
                trace_path = trace_dir / f"{label}-{run}.csv"
                ui = Session([str(BINARY), str(path)], env={"GUINEANOTE_TRACE": str(trace_path)} if "--trace" in sys.argv else None)
                try:
                    ui.wait("Opened")
                    completions.append({"fixture":label,"process_start_to_opened_ms":round((time.perf_counter()-opened)*1000,3)})
                    action(ui, f"{label}:type-start", "x"*10, "Ln 1, Col 11 ")
                    if line == 2:
                        action(ui, f"{label}:down", "\x1b[B", "Ln 2, Col 11 ")
                    end = graphemes + (11 if line == 1 else 1)
                    action(ui, f"{label}:first-end", "\x1b[F", f"Ln {line}, Col {end} ")
                    action(ui, f"{label}:cached-left", "\x1b[D", f"Ln {line}, Col {end-1} ")
                    action(ui, f"{label}:cached-right", "\x1b[C", f"Ln {line}, Col {end} ")
                    ui.send("\x11")
                    ui.wait("Discard ALL")
                    ui.send("y")
                    ui.wait("TERM_STATE_OK")
                    ui.wait("CHILD_EXIT=0")
                finally:
                    (ROOT / f"target/pty-navigation-{label}-{run}.log").write_bytes(ui.output)
                    ui.close()
                if trace_path.exists():
                    rows = list(csv.DictReader(trace_path.open(encoding="utf-8")))
                    stages = {}
                    for row in rows:
                        if row["stage"] == "dropped_samples":
                            continue
                        stages.setdefault(row["stage"], []).append(int(row["microseconds"])/1000)
                    traces.append({"fixture":label,"session":run,"file":str(trace_path.relative_to(ROOT)),
                                   "dropped_samples":sum(int(r["microseconds"]) for r in rows if r["stage"]=="dropped_samples"),
                                   "stage_max_ms":{k:round(max(v),3) for k,v in stages.items()}})
        report = {"platform": platform.platform(), "input_bytes_max": 10*1024*1024,
                  "cold_sessions": sum(f[3] for f in fixtures), "traces": traces, "samples": RESULTS, "sample_count": len(RESULTS),
                  "background_open_completion":completions,
                  "max_ms": max(r["ms"] for r in RESULTS),
                  "failures": [r for r in RESULTS if r["ms"] >= 100]}
        (ROOT / "target/navigation-responsiveness.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report, indent=2))
        if "--record-only" not in sys.argv:
            assert not report["failures"], "Navigation budget exceeded"


if __name__ == "__main__":
    main()
