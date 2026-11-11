"""Native find/replace UI, Unicode, undo, read-only and 10 MiB feedback.
All fixtures/preferences are owned temporary data; no user file changes.
"""
import json
import argparse
import csv
import hashlib
import os
import tempfile
import time
import platform
import subprocess
from pathlib import Path
from pty_smoke import ROOT,Session
from pty_responsiveness import action,RESULTS
from pty_dialog_mouse import click
from tui_preview import snapshot
from pty_interaction_latency import stats,source_hash
from collections import defaultdict

BINARY=ROOT/"target/release"/("guinea.exe" if os.name=="nt" else "guinea")
COMPLETIONS=[]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--trace',action='store_true')
    parser.add_argument('--profile-observer',action='store_true')
    parser.add_argument('--legacy-replacement-feedback',action='store_true',help='reproduce the old predicate that skips Searching feedback')
    parser.add_argument('--record-only',action='store_true',help='retain outliers without enforcing the 50 ms threshold')
    args=parser.parse_args()
    def replacement_feedback(screen):
        # A small replacement can finish before the next frame. Searching then
        # acknowledges it while refreshed results are still being computed.
        states=('Replacing','No matches') if args.legacy_replacement_feedback else ('Replacing','Searching','No matches')
        return any(state in screen for state in states)
    session_type=Session
    if args.profile_observer:
        from pty_observer import ProfiledSession
        session_type=ProfiledSession
    trace_dir=ROOT/'target'/('search-traces-'+str(time.time_ns()));trace_dir.mkdir()
    counter=0
    def start_session(arguments,**kwargs):
        nonlocal counter
        counter+=1
        if args.trace:kwargs['env']={'GUINEANOTE_TRACE':str(trace_dir/(str(counter)+'.csv'))}
        return session_type(arguments,**kwargs)
    with tempfile.TemporaryDirectory(prefix="guineanote-search-") as directory:
        root=Path(directory)
        path=root/"note.txt"
        original="Việt 👩‍💻\r\nViệt e\u0301\r\n"
        path.write_bytes(b"\xef\xbb\xbf"+original.encode())
        ui=start_session([str(BINARY),str(root),str(path)],colors=True)
        try:
            ui.resize(42,148);ui.wait("Opened")
            action(ui,"search:open-replace",click(ui,"Find/Replace ^F"),"FIND / REPLACE")
            action(ui,"search:query-unicode","Việt","Việt│")
            ui.wait("Match 1 / 2")
            ui.send("\tVietnam")
            ui.wait("Vietnam│");ui.read(.04)
            snapshot(ui,"find-replace-en")
            action(ui,"search:replace-one",click(ui," Replace ","Replace all"),lambda s:"Replacing" in s or "Replaced 1 matches" in s)
            ui.wait("Match 1 / 1")
            assert not replacement_feedback("\n".join(ui.screen.display)),"Feedback must be fresh after the click"
            started=time.perf_counter()
            action(ui,"search:replace-all",click(ui," Replace all "),replacement_feedback)
            ui.wait('No matches')
            COMPLETIONS.append({'operation':'small replace-all and result refresh complete','ms':round((time.perf_counter()-started)*1000,3)})
            assert path.read_bytes()==b"\xef\xbb\xbf"+original.encode(), (path.read_bytes(),original.encode())
            action(ui,"search:close","\x1b",lambda s:"FIND / REPLACE" not in s)
            ui.wait("Vietnam")
            ui.send("\x13");ui.wait_until(lambda lines:"● Saved" in "\n".join(lines) and "● MODIFIED" not in "\n".join(lines))
            assert path.read_bytes()==b"\xef\xbb\xbf"+original.replace("Việt","Vietnam").encode()
            ui.read(.01)
            action(ui,"search:undo-all","\x1a","Ln 2, Col 5 ")
            action(ui,"search:undo-one","\x1a","Ln 1, Col 5 ")
            action(ui,"search:open-find","\x06","FIND / REPLACE")
            action(ui,"search:replace-selected-query","\x01absent","absent│")
            ui.wait("No matches")
            ui.send("\x1b");ui.wait_until(lambda lines:"FIND / REPLACE" not in "\n".join(lines))
            # Do not repeatedly open the Windows destination during ReplaceFile:
            # Python CRT readers can deny DELETE sharing and cause a legitimate
            # safe-save failure. Await application completion before byte checks.
            ui.send("\x13");ui.wait_until(lambda lines:"● Saved" in "\n".join(lines) and "● MODIFIED" not in "\n".join(lines))
            assert path.read_bytes()==b"\xef\xbb\xbf"+original.encode(), (path.read_bytes(),original.encode())
            ui.send("\x1b[21~");ui.wait("SETTINGS");ui.send("v\r");ui.wait("SOẠN THẢO")
            ui.send(click(ui,"Tìm/Thay ^F"));ui.send("Việt");ui.wait("Kết quả 1 / 2");ui.read(.04)
            snapshot(ui,"find-replace-vi")
            ui.send("\x1b");ui.wait_until(lambda lines:"TÌM / THAY THẾ" not in "\n".join(lines))
            ui.send("\x11");ui.wait("TERM_STATE_OK");ui.wait("CHILD_EXIT=0")
        finally:
            (ROOT/"target/pty-search.log").write_bytes(ui.output);ui.close()
            (trace_dir/'small.log').write_bytes(ui.output)

        readonly=start_session([str(BINARY),"--readonly",str(path)])
        try:
            readonly.wait("Opened");readonly.send(click(readonly,"Find/Replace ^F"));readonly.send("Việt");readonly.wait("Match 1 / 2")
            readonly.send("\tX\x12");readonly.wait("Document is read-only")
            readonly.send("\x1b");readonly.wait_until(lambda lines:"FIND / REPLACE" not in "\n".join(lines))
            readonly.send("\x11");readonly.wait("TERM_STATE_OK");readonly.wait("CHILD_EXIT=0")
            assert path.read_bytes()==b"\xef\xbb\xbf"+original.encode(), (path.read_bytes(),original.encode())
        finally:
            (ROOT/"target/pty-search-readonly.log").write_bytes(readonly.output);readonly.close()
            (trace_dir/'readonly.log').write_bytes(readonly.output)

        size=10*1024*1024
        text=("marker row\n"*((size+10)//11))[:size]
        large=root/"large.txt";large.write_bytes(text.encode())
        ui=start_session([str(BINARY),str(large)])
        try:
            ui.wait("Opened")
            action(ui,"search:10MiB-open",click(ui,"Find/Replace ^F"),"FIND / REPLACE")
            started=time.perf_counter()
            action(ui,"search:10MiB-query-feedback","marker","marker│")
            ui.wait("Match 1 /",timeout=40)
            COMPLETIONS.append({"operation":"10MiB search complete","ms":round((time.perf_counter()-started)*1000,3)})
            started=time.perf_counter()
            action(ui,"search:10MiB-last-page-feedback",click(ui," < Prev "),lambda s:"Searching" in s)
            ui.wait_until(lambda lines:"Searching" not in "\n".join(lines),timeout=40)
            COMPLETIONS.append({"operation":"10MiB previous-page complete","ms":round((time.perf_counter()-started)*1000,3)})
            ui.send("\titem");ui.wait("item│")
            assert not replacement_feedback("\n".join(ui.screen.display)),"Feedback must be fresh after the click"
            started=time.perf_counter()
            action(ui,"search:10MiB-replace-feedback",click(ui," Replace all "),replacement_feedback)
            ui.wait("No matches",timeout=40)
            COMPLETIONS.append({"operation":"10MiB replace-all complete","ms":round((time.perf_counter()-started)*1000,3)})
            ui.send("\x1b");ui.wait_until(lambda lines:"FIND / REPLACE" not in "\n".join(lines))
            ui.send("\x1a");ui.wait("● Saved")
            ui.send("\x11");ui.wait("TERM_STATE_OK");ui.wait("CHILD_EXIT=0")
            assert large.read_bytes()==text.encode()
        finally:
            (ROOT/"target/pty-search-large.log").write_bytes(ui.output);ui.close()
            (trace_dir/'large.log').write_bytes(ui.output)
        report={"max_input_bytes":size,"samples":RESULTS,"sample_count":len(RESULTS),"max_ms":max(r["ms"] for r in RESULTS),"background_completion":COMPLETIONS,"failures":[r for r in RESULTS if r["ms"]>50]}
        groups=defaultdict(list);investigations=[];dropped=0
        incomplete=[]
        traces=list(trace_dir.glob('*.csv'))
        for trace in traces:
            with trace.open(encoding='utf-8') as stream:
                rows=list(csv.DictReader(stream))
            if not any(row['stage']=='dropped_samples' for row in rows):incomplete.append(trace.name)
            for row in rows:
                if row['stage']=='dropped_samples':dropped+=int(row['microseconds'])
                if row['stage']=='input_to_flush':
                    groups[(trace.stem,row['operation'],row['phase'])].append(int(row['microseconds'])/1000)
                    if int(row['microseconds'])>50000:investigations.append({'session':trace.stem,**row})
        report.update({'application_groups':[{'session':k[0],'operation':k[1],'phase':k[2],**stats(v)} for k,v in sorted(groups.items())],
                       'application_investigations':investigations,'dropped_samples':dropped,
                       'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'source_sha256':source_hash(),
                       'git_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
                       'profile':'release','platform':platform.platform(),'python':platform.python_version(),
                       'harness_sha256':{str(p.relative_to(ROOT)).replace('\\','/'):hashlib.sha256(p.read_bytes()).hexdigest()
                                         for p in sorted((ROOT/'scripts').glob('*.py'))},
                       'trace_enabled':args.trace,'observer_profile_enabled':args.profile_observer,
                       'replacement_feedback_definition':'legacy_skips_searching' if args.legacy_replacement_feedback else 'initial_feedback_includes_searching',
                       'record_only':args.record_only,'incomplete_traces':incomplete,
                       'status':('invalid_trace' if incomplete or dropped or (args.trace and len(traces)!=counter) else
                                 'needs_investigation' if report['failures'] or investigations else 'within_investigation_threshold'),
                       'limits':['Input-to-flush begins after application event decode; excludes adapter queue and physical paint',
                                 'Observer select wait includes application, transport and scheduling; it is not isolated queue time',
                                 'Observer stage timers add overhead; compare unprofiled runs on the same binary',
                                 'Display time covers checks inside read; outer predicates and scheduling remain unattributed',
                                 'Small per-session groups do not certify percentile targets or explain historical outliers']})
        (trace_dir/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        (ROOT/'target/search-latest-path.txt').write_text(str(trace_dir),encoding='utf-8')
        (ROOT/"target/search-responsiveness.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
        print(json.dumps({k:v for k,v in report.items() if k!='samples'},indent=2))
        assert not args.trace or len(traces)==counter,"Missing application trace"
        assert not incomplete and dropped==0,"Incomplete/dropped trace invalidates acceptance"
        if not args.record_only:
            assert not report['failures'] and not investigations,"Search response requires >50 ms investigation"


if __name__=="__main__":
    main()
