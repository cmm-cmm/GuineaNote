"""Long ASCII viewport/navigation and memory observations; preserves outliers."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import time
from collections import defaultdict
from pty_smoke import ROOT, Session
from pty_responsiveness import action, RESULTS
from pty_interaction_latency import BINARY, source_hash, stats
import psutil


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--iterations',type=int,default=30)
    parser.add_argument('--no-trace',action='store_true')
    args=parser.parse_args();assert args.iterations>=20
    out=ROOT/'target'/('ascii-navigation-'+str(time.time_ns()));out.mkdir()
    groups=defaultdict(list);investigations=[];sessions=[];dropped=0
    with tempfile.TemporaryDirectory(prefix='guinea-ascii-navigation-') as d:
        root=Path(d)
        for name,unit in [('ascii','a'),('ascii-tabs','ab\t')]:
            original=unit*(10*1024*1024//len(unit))
            path=root/(name+'.txt');path.write_bytes(original.encode())
            trace=out/(name+'.csv');started=time.perf_counter()
            ui=Session([str(BINARY),str(path)],env={} if args.no_trace else {'GUINEANOTE_TRACE':str(trace)})
            try:
                ui.wait('Opened');ui.read(.03)
                opened=(time.perf_counter()-started)*1000
                editor=next(p for p in psutil.Process(ui.pid).children(recursive=True) if p.name().casefold()==BINARY.name.casefold())
                rss=[editor.memory_info().rss]
                end=len(original)+1
                for _ in range(args.iterations):
                    action(ui,name+':end','\x1b[F',f'Ln 1, Col {end} ')
                    action(ui,name+':left','\x1b[D',f'Ln 1, Col {end-1} ')
                    action(ui,name+':right','\x1b[C',f'Ln 1, Col {end} ')
                    action(ui,name+':home','\x1b[H','Ln 1, Col 1 ')
                    action(ui,name+':type','x','Ln 1, Col 2 ')
                    action(ui,name+':delete','\x7f','Ln 1, Col 1 ')
                    rss.append(editor.memory_info().rss)
                memory=editor.memory_info()
                sessions.append({'fixture':name,'bytes':path.stat().st_size,'open_feedback_ms':opened,
                                 'rss_at_open':rss[0],'max_sampled_rss':max(rss),'os_peak_working_set':getattr(memory,'peak_wset',None)})
                ui.send('\x11');ui.wait('Discard ALL');ui.send('y');ui.wait('TERM_STATE_OK');ui.wait('CHILD_EXIT=0')
                assert path.read_bytes()==original.encode()
            finally:
                (out/(name+'.log')).write_bytes(ui.output);ui.close()
            if trace.exists():
                rows=list(csv.DictReader(trace.open(encoding='utf-8')))
                assert any(r['stage']=='dropped_samples' for r in rows),'Incomplete trace'
                for row in rows:
                    if row['stage']=='dropped_samples':dropped+=int(row['microseconds'])
                    if row['stage']=='input_to_flush':
                        groups[(name,row['operation'],row['phase'])].append(int(row['microseconds'])/1000)
                        if int(row['microseconds'])>50000:investigations.append({'fixture':name,**row})
    report={'git_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
            'source_sha256':source_hash(),'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),
            'profile':'release','platform':platform.platform(),'trace_enabled':not args.no_trace,
            'iterations':args.iterations,'investigation_threshold_ms':50,'sessions':sessions,
            'application_groups':[{'fixture':k[0],'operation':k[1],'phase':k[2],**stats(v)} for k,v in sorted(groups.items())],
            'application_investigations':investigations,'pty_samples':RESULTS,'pty_investigations':[r for r in RESULTS if r['ms']>50],
            'dropped_samples':dropped,'limits':['Main-loop receipt after decode; excludes adapter queue and physical paint',
                'Same terminal 100x28; Windows/ConPTY only when run on Windows','RSS observations are not an allocator bound; open completion is separate']}
    (out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    (ROOT/'target/ascii-navigation-latest-path.txt').write_text(str(out),encoding='utf-8')
    print(json.dumps({'report':str(out/'report.json'),'investigations':len(investigations),'dropped':dropped,'sessions':sessions},indent=2))
    assert dropped==0,'Trace loss invalidates acceptance'


if __name__=='__main__':main()
