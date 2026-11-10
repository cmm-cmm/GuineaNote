"""Native find/replace UI, Unicode, undo, read-only and 10 MiB feedback.
All fixtures/preferences are owned temporary data; no user file changes.
"""
import json
import csv
import sys
import hashlib
import os
import tempfile
import time
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
    trace_dir=ROOT/'target'/('search-traces-'+str(time.time_ns()));trace_dir.mkdir()
    counter=0
    def start_session(arguments,**kwargs):
        nonlocal counter
        counter+=1
        if '--trace' in sys.argv:kwargs['env']={'GUINEANOTE_TRACE':str(trace_dir/(str(counter)+'.csv'))}
        return Session(arguments,**kwargs)
    with tempfile.TemporaryDirectory(prefix="guineanote-search-") as directory:
        root=Path(directory)
        path=root/"note.txt"
        original="Việt 👩‍💻\r\nViệt e\u0301\r\n"
        path.write_bytes(b"\xef\xbb\xbf"+original.encode())
        ui=start_session([str(BINARY),str(root),str(path)],colors=True)
        try:
            ui.resize(42,148);ui.wait("Opened")
            action(ui,"search:open-replace",click(ui,"Replace ^H"),"FIND / REPLACE")
            action(ui,"search:query-unicode","Việt","Việt│")
            ui.wait("Match 1 / 2")
            ui.send("\tVietnam")
            ui.wait("Vietnam│");ui.read(.04)
            snapshot(ui,"find-replace-en")
            action(ui,"search:replace-one",click(ui," Replace ","Replace all"),lambda s:"Replacing" in s or "Replaced 1 matches" in s)
            ui.wait("Match 1 / 1")
            started=time.perf_counter()
            action(ui,"search:replace-all",click(ui," Replace all "),lambda s:"Replacing" in s or "No matches" in s)
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
            ui.send("\x1b[19~");ui.wait("SETTINGS");ui.send("v\r");ui.wait("SOẠN THẢO")
            ui.send(click(ui,"Thay ^H"));ui.send("Việt");ui.wait("Kết quả 1 / 2");ui.read(.04)
            snapshot(ui,"find-replace-vi")
            ui.send("\x1b");ui.wait_until(lambda lines:"TÌM / THAY THẾ" not in "\n".join(lines))
            ui.send("\x11");ui.wait("TERM_STATE_OK");ui.wait("CHILD_EXIT=0")
        finally:
            (ROOT/"target/pty-search.log").write_bytes(ui.output);ui.close()
            (trace_dir/'small.log').write_bytes(ui.output)

        readonly=start_session([str(BINARY),"--readonly",str(path)])
        try:
            readonly.wait("Opened");readonly.send(click(readonly,"Replace ^H"));readonly.send("Việt");readonly.wait("Match 1 / 2")
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
            action(ui,"search:10MiB-open",click(ui,"Replace ^H"),"FIND / REPLACE")
            started=time.perf_counter()
            action(ui,"search:10MiB-query-feedback","marker","marker│")
            ui.wait("Match 1 /",timeout=40)
            COMPLETIONS.append({"operation":"10MiB search complete","ms":round((time.perf_counter()-started)*1000,3)})
            started=time.perf_counter()
            action(ui,"search:10MiB-last-page-feedback",click(ui," < Prev "),lambda s:"Searching" in s)
            ui.wait_until(lambda lines:"Searching" not in "\n".join(lines),timeout=40)
            COMPLETIONS.append({"operation":"10MiB previous-page complete","ms":round((time.perf_counter()-started)*1000,3)})
            ui.send("\titem");ui.wait("item│")
            started=time.perf_counter()
            action(ui,"search:10MiB-replace-feedback",click(ui," Replace all "),lambda s:"Replacing" in s or "No matches" in s)
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
        for trace in trace_dir.glob('*.csv'):
            for row in csv.DictReader(trace.open(encoding='utf-8')):
                if row['stage']=='dropped_samples':dropped+=int(row['microseconds'])
                if row['stage']=='input_to_flush':
                    groups[(trace.stem,row['operation'],row['phase'])].append(int(row['microseconds'])/1000)
                    if int(row['microseconds'])>50000:investigations.append({'session':trace.stem,**row})
        report.update({'application_groups':[{'session':k[0],'operation':k[1],'phase':k[2],**stats(v)} for k,v in sorted(groups.items())],
                       'application_investigations':investigations,'dropped_samples':dropped,
                       'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'source_sha256':source_hash()})
        (trace_dir/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        (ROOT/'target/search-latest-path.txt').write_text(str(trace_dir),encoding='utf-8')
        (ROOT/"target/search-responsiveness.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
        print(json.dumps({k:v for k,v in report.items() if k!='samples'},indent=2))
        assert not report['failures'],"Search UI response budget exceeded"


if __name__=="__main__":
    main()
