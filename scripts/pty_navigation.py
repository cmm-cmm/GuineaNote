"""Cold long-Unicode-line End navigation, followed by cached movement.
Release binaries, owned fixtures, strict UI response budget at most 50 ms.
Use --record-only to retain exploratory measurements before optimization.

Unprepared fixtures cover lines the open worker does not pre-index: a 12 MiB
line (above the 10 MiB prepare limit, opened through the large-file prompt), a
long line after line 64, and End after a newline edit invalidated the index.
--affinity 0 or 0,1 pins the editor to one/two CPUs to model low-core or
contended machines. --fixtures selects a comma-separated subset by label.
"""
import csv
import argparse
import json
import os
import platform
from pathlib import Path
import sys
import tempfile
import time
from pty_responsiveness import action, RESULTS
from pty_smoke import ROOT, Session
from pty_interaction_latency import source_hash, stats
from collections import defaultdict
import hashlib
import subprocess

# GUINEANOTE_BINARY selects another build for interleaved A/B comparisons.
BINARY = Path(os.environ.get("GUINEANOTE_BINARY") or ROOT / "target/release" / ("guinea.exe" if os.name == "nt" else "guinea"))


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--trace',action='store_true')
    parser.add_argument('--record-only',action='store_true')
    parser.add_argument('--cold-runs',type=int,default=6)
    parser.add_argument('--unprepared-runs',type=int,default=4)
    parser.add_argument('--affinity',help='comma-separated CPU indices for the editor process')
    parser.add_argument('--fixtures',help='comma-separated fixture labels to run')
    args=parser.parse_args()
    assert args.cold_runs>0
    completions=[]
    traces=[]
    groups=defaultdict(list);investigations=[]
    trace_dir = ROOT / "target" / ("navigation-traces-" + str(time.time_ns()))
    trace_dir.mkdir(parents=True)
    with tempfile.TemporaryDirectory(prefix="guineanote-navigation-") as directory:
        root = Path(directory)
        unit = "Việt e\u0301 界 👩‍💻 "
        repeats = 10 * 1024 * 1024 // len(unit.encode())
        two_repeats = (10 * 1024 * 1024 - len("prefix\n")) // len(unit.encode())
        large_repeats = 12 * 1024 * 1024 // len(unit.encode())
        late_repeats = 5 * 1024 * 1024 // len(unit.encode())
        fixtures=[("unicode",unit*repeats,repeats*11,args.cold_runs,1),
                  ("ascii-prefix-bare-cr","a"*(10*1024*1024-4)+"\r界",10*1024*1024-2,2,1),
                  ("second-long-line","prefix\n"+unit*two_repeats,two_repeats*11,4,2),
                  ("unprepared-12mib",unit*large_repeats,large_repeats*11,args.unprepared_runs,1),
                  ("unprepared-line-101","x\n"*100+unit*late_repeats,late_repeats*11,args.unprepared_runs,101),
                  ("unprepared-after-newline",unit*repeats,repeats*11,args.unprepared_runs,1)]
        if args.fixtures:
            wanted=set(args.fixtures.split(','))
            fixtures=[f for f in fixtures if f[0] in wanted]
            assert fixtures,'No fixture matched --fixtures'
        env_extra={"GUINEANOTE_PROBE_AFFINITY":args.affinity} if args.affinity else {}
        for label, content, graphemes, runs, line in fixtures:
            path = root / f"{label}.rs"
            path.write_bytes(content.encode())
            for run in range(runs):
                opened=time.perf_counter()
                trace_path = trace_dir / f"{label}-{run}.csv"
                env=dict(env_extra)
                if args.trace:
                    env["GUINEANOTE_TRACE"]=str(trace_path)
                ui = Session([str(BINARY), str(path)], env=env or None)
                try:
                    if label == "unprepared-12mib":
                        ui.wait("exceeds warning")
                        ui.send("y")
                    ui.wait("Opened")
                    completions.append({"fixture":label,"process_start_to_opened_ms":round((time.perf_counter()-opened)*1000,3)})
                    action(ui, f"{label}:type-start", "x"*10, "Ln 1, Col 11 ")
                    if line == 2:
                        action(ui, f"{label}:down", "\x1b[B", "Ln 2, Col 11 ")
                    if line == 101:
                        action(ui, f"{label}:goto-prompt", "\x0c", "Go to line")
                        action(ui, f"{label}:goto", "101\r", "Ln 101, Col 1 ")
                    if label == "unprepared-after-newline":
                        action(ui, f"{label}:newline", "\r", "Ln 2, Col 1 ")
                        action(ui, f"{label}:join", "\x7f", "Ln 1, Col 11 ")
                    end = graphemes + (11 if line == 1 else 1)
                    # Feedback: the first frame after End, exact or the
                    # pending caret window ("Col …") while the line indexes.
                    # Completion (exact column) is background work, reported
                    # separately and never mixed into interaction samples.
                    end_started=time.perf_counter()
                    action(ui, f"{label}:first-end", "\x1b[F",
                           lambda screen: f"Ln {line}, Col {end} " in screen or f"Ln {line}, Col … " in screen)
                    ui.wait(f"Ln {line}, Col {end} ", timeout=60)
                    completions.append({"fixture":label,"first_end_completion_ms":round((time.perf_counter()-end_started)*1000,3)})
                    action(ui, f"{label}:cached-left", "\x1b[D", f"Ln {line}, Col {end-1} ")
                    action(ui, f"{label}:cached-right", "\x1b[C", f"Ln {line}, Col {end} ")
                    ui.send("\x11")
                    ui.wait("Discard ALL")
                    ui.send("y")
                    # Exit is not an interaction metric; a pinned CPU can
                    # spend seconds finishing a recovery checkpoint first.
                    ui.wait("TERM_STATE_OK", timeout=120)
                    ui.wait("CHILD_EXIT=0", timeout=120)
                finally:
                    (trace_dir / f"pty-navigation-{label}-{run}.log").write_bytes(ui.output)
                    ui.close()
                if trace_path.exists():
                    rows = list(csv.DictReader(trace_path.open(encoding="utf-8")))
                    stages = {}
                    for row in rows:
                        if row["stage"] == "dropped_samples":
                            continue
                        stages.setdefault(row["stage"], []).append(int(row["microseconds"])/1000)
                        if row['stage']=='input_to_flush':
                            groups[(label,row['operation'],row['phase'])].append(int(row['microseconds'])/1000)
                            if int(row['microseconds'])>50000:investigations.append({'fixture':label,'run':run,**row})
                    traces.append({"fixture":label,"session":run,"file":str(trace_path.relative_to(ROOT)),
                                   "dropped_samples":sum(int(r["microseconds"]) for r in rows if r["stage"]=="dropped_samples"),
                                   "stage_max_ms":{k:round(max(v),3) for k,v in stages.items()}})
        report = {"platform": platform.platform(), "input_bytes_max": max(len(f[1].encode()) for f in fixtures),
                  "cold_sessions": sum(f[3] for f in fixtures), "traces": traces, "samples": RESULTS, "sample_count": len(RESULTS),
                  "background_open_completion":completions,
                  "max_ms": max(r["ms"] for r in RESULTS),
                  "failures": [r for r in RESULTS if r["ms"] > 50],
                  'investigation_threshold_ms':50,'application_investigations':investigations,
                  'application_groups':[{'fixture':k[0],'operation':k[1],'phase':k[2],**stats(v)} for k,v in sorted(groups.items())],
                  'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'source_sha256':source_hash(),
                  'git_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
                  'profile':'release','trace_enabled':args.trace,'affinity':args.affinity,
                  'fixtures':[f[0] for f in fixtures],
                  'limits':['First process/category is not cold OS disk cache','Main-loop receipt after decode, not adapter queue or pixels','Finite samples; no universal SLA']}
        (trace_dir/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        (ROOT/'target/navigation-latest-path.txt').write_text(str(trace_dir),encoding='utf-8')
        (ROOT / "target/navigation-responsiveness.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps(report, indent=2))
        assert not any(t['dropped_samples'] for t in traces),'Dropped trace rows'
        if not args.record_only:
            assert not report["failures"] and not investigations, "Navigation response >50 ms requires investigation"


if __name__ == "__main__":
    main()
