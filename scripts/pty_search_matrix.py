"""Sequential old/corrected feedback comparison, trace and observer on/off.

Each child runs the same Unicode/readonly/10 MiB safety workflow. Legacy
outliers are expected investigative data; corrected outliers fail unless
--record-only is specified. Build and install PTY dev tools before running.
"""
import argparse
from collections import defaultdict
import csv
import hashlib
import itertools
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

from pty_smoke import ROOT
from pty_interaction_latency import stats


def distribution(values):
    result = stats(values)
    # App percentile budgets do not apply to background completion or PTY.
    result.pop("status")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=20)
    parser.add_argument("--record-only", action="store_true")
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")
    out = ROOT / "target" / ("search-matrix-" + str(time.time_ns()))
    out.mkdir()
    (ROOT / "target/search-matrix-latest-path.txt").write_text(str(out), encoding="utf-8")
    modes = list(itertools.product((True, False), (True, False), (True, False)))
    runs, samples, completion, application = [], defaultdict(list), defaultdict(list), defaultdict(list)
    binaries, sources, harnesses = set(), set(), set()
    corrected_outliers = []
    for iteration in range(args.runs):
        # Rotate condition order to spread drift across conditions. Never run
        # builds, audits or another probe concurrently with this process.
        offset = iteration % len(modes)
        for legacy, trace, profiled in modes[offset:] + modes[:offset]:
            mode = ("legacy" if legacy else "corrected") + ("-trace" if trace else "-off") + ("-profiled" if profiled else "-plain")
            destination = out / mode / str(iteration + 1)
            destination.mkdir(parents=True)
            command = [sys.executable, "-X", "utf8", str(ROOT / "scripts/pty_search.py"), "--record-only"]
            if legacy:
                command.append("--legacy-replacement-feedback")
            if trace:
                command.append("--trace")
            if profiled:
                command.append("--profile-observer")
            latest = ROOT / "target/search-latest-path.txt"
            previous = latest.read_text() if latest.exists() else None
            with (destination / "console.txt").open("wb") as console:
                result = subprocess.run(command, cwd=ROOT, stdout=console, stderr=subprocess.STDOUT)
            entry = {"mode": mode, "run": iteration + 1, "exit": result.returncode}
            runs.append(entry)
            (out / "runs.json").write_text(json.dumps(runs, indent=2), encoding="utf-8")
            current = latest.read_text() if latest.exists() else None
            if result.returncode or current == previous:
                raise RuntimeError(f"Incomplete workflow: {mode}, run {iteration + 1}; see {destination}")
            captured = Path(current.strip())
            shutil.copytree(captured, destination, dirs_exist_ok=True)
            report = json.loads((destination / "report.json").read_text())
            binaries.add(report["binary_sha256"])
            sources.add(report["source_sha256"])
            harnesses.add(json.dumps(report["harness_sha256"], sort_keys=True))
            for sample in report["samples"]:
                samples[mode, sample["action"]].append(sample["ms"])
                if not legacy and sample["ms"] > 50:
                    corrected_outliers.append({**entry, **sample})
            for sample in report["background_completion"]:
                completion[mode, sample["operation"]].append(sample["ms"])
            for path in destination.glob("*.csv"):
                with path.open(encoding="utf-8") as stream:
                    for row in csv.DictReader(stream):
                        if row["stage"] == "input_to_flush":
                            application[mode, path.stem, row["operation"], row["phase"]].append(int(row["microseconds"]) / 1000)
                            if not legacy and int(row["microseconds"]) > 50000:
                                corrected_outliers.append({**entry, "application": row})
        print(f"Completed {iteration + 1}/{args.runs} rounds (eight conditions each)", flush=True)
    summary = {
        "runs_per_condition": args.runs, "runs": runs,
        "binary_sha256": sorted(binaries), "source_sha256": sorted(sources),
        "harness_variants": len(harnesses),
        "pty_groups": [{"mode": key[0], "action": key[1], **distribution(values)} for key, values in sorted(samples.items())],
        "completion_groups": [{"mode": key[0], "operation": key[1], **distribution(values)} for key, values in sorted(completion.items())],
        "application_groups": [{"mode": key[0], "session": key[1], "operation": key[2], "phase": key[3], **stats(values)} for key, values in sorted(application.items())],
        "corrected_outliers": corrected_outliers,
        "status": "needs_investigation" if corrected_outliers else "within_investigation_threshold",
        "limits": ["Percentiles recomputed from raw samples, separately by condition and category",
                   "Application percentile targets apply only to input_to_flush; background completion is reported separately",
                   "Twenty first-use samples are not a reliable long-tail certification; no physical-paint or OS-wide SLA",
                   "Feedback definition changed; this comparison is a harness correction, not editor acceleration"],
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    artifacts = {str(path.relative_to(out)).replace("\\", "/"): hashlib.sha256(path.read_bytes()).hexdigest()
                 for path in sorted(out.rglob("*")) if path.is_file()}
    (out / "checksums.json").write_text(json.dumps(artifacts, indent=2), encoding="utf-8")
    print(out, flush=True)
    assert len(binaries) == len(sources) == len(harnesses) == 1, "Comparison inputs changed during collection"
    assert args.record_only or not corrected_outliers, "Corrected feedback still requires >50 ms investigation"


if __name__ == "__main__":
    main()
