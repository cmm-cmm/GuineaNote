"""Grouped application input-to-flush metrics; PTY feedback reported separately.

No CI integration. Retains every sample/outlier. Run sequentially after release
builds. --no-trace repeats the same workload to observe PTY tracing overhead.
"""
import argparse
import csv
import hashlib
import json
import math
import os
import platform
from pathlib import Path
import subprocess
import tempfile
import time
from collections import defaultdict
from pty_smoke import ROOT, Session
from pty_responsiveness import action, RESULTS

BINARY=ROOT/'target/release'/('guinea.exe' if os.name=='nt' else 'guinea')


def stats(values):
    values=sorted(values)
    rank=lambda p:values[max(0,math.ceil(p*len(values))-1)]
    p50,p95,p99=rank(.5),rank(.95),rank(.99)
    return {'n':len(values),'p50_ms':p50,'p95_ms':p95,'p99_ms':p99,'max_ms':max(values),
            'over_50ms':sum(v>50 for v in values),
            'over_100ms':sum(v>100 for v in values),
            'status':('needs_investigation' if max(values)>50 else
                      'insufficient_samples' if len(values)<20 else
                      'pass' if p50<=10 and p95<=20 and p99<=50 else 'needs_investigation')}


def source_hash():
    h=hashlib.sha256()
    paths=[ROOT/'Cargo.toml',ROOT/'Cargo.lock']
    paths+=list((ROOT/'crates').rglob('*.rs'))+list((ROOT/'crates').rglob('Cargo.toml'))
    paths+=list((ROOT/'src').rglob('*.rs'))
    for p in sorted(paths):
        h.update(str(p.relative_to(ROOT)).replace('\\','/').encode());h.update(b'\0');h.update(p.read_bytes())
    return h.hexdigest()


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--no-trace',action='store_true');parser.add_argument('--iterations',type=int,default=100)
    args=parser.parse_args()
    assert args.iterations>=20
    out=ROOT/'target'/('interaction-'+str(time.time_ns()));out.mkdir()
    groups=defaultdict(list);dropped=0;sessions=[];investigations=[]
    with tempfile.TemporaryDirectory(prefix='guineanote-interaction-') as d:
        root=Path(d)
        for label,size in [('small',1024),('unicode-long-line',10*1024*1024)]:
            unit='a e\u0301 界 👩‍💻 '
            content='marker '+unit*((size-7)//len(unit.encode()))
            path=root/(label+'.rs');path.write_bytes(content.encode())
            trace=out/(label+'.csv')
            ui=Session([str(BINARY),str(path)],env={} if args.no_trace else {'GUINEANOTE_TRACE':str(trace)})
            try:
                opened=time.perf_counter();ui.wait('Opened');ui.read(.03)
                sessions.append({'corpus':label,'bytes':path.stat().st_size,'open_completion_ms':round((time.perf_counter()-opened)*1000,3)})
                for _ in range(args.iterations):
                    action(ui,'typing:'+label,'x','Ln 1, Col 2 ')
                    action(ui,'delete:'+label,'\x7f','Ln 1, Col 1 ')
                    action(ui,'caret:'+label,'\x1b[C','Ln 1, Col 2 ')
                    action(ui,'caret:'+label,'\x1b[D','Ln 1, Col 1 ')
                for _ in range(30):
                    action(ui,'dialogs:'+label,'\x1b[21~','SETTINGS · F10')
                    action(ui,'dialog-close:'+label,'\x1b',lambda s:'SETTINGS · F10' not in s)
                end=8+((size-7)//len(unit.encode()))*8
                for _ in range(25):
                    action(ui,'long-end:'+label,'\x1b[F',f'Ln 1, Col {end} ')
                    action(ui,'long-home:'+label,'\x1b[H','Ln 1, Col 1 ')
                for _ in range(25):
                    action(ui,'history-type:'+label,'z','Ln 1, Col 2 ')
                    action(ui,'undo:'+label,'\x1a','Ln 1, Col 1 ')
                    action(ui,'redo:'+label,'\x19','Ln 1, Col 2 ')
                    ui.send('\x7f');ui.wait('Ln 1, Col 1 ')
                for _ in range(25):
                    action(ui,'project-open:'+label,'\x05','PROJECT · Ctrl+E / Ctrl+G')
                    action(ui,'project-query:'+label,'m','› m│')
                    action(ui,'project-close:'+label,'\x1b',lambda s:'PROJECT · Ctrl+E / Ctrl+G' not in s)
                for _ in range(25):
                    for rows,cols in [(24,80),(28,100)]:
                        # Drain the previous ConPTY frame before changing the
                        # observer geometry; late old-size output otherwise
                        # wraps into the new screen and corrupts its status.
                        ui.read(.03)
                        before=len(ui.output)
                        ui.resize(rows,cols)
                        # Resizing the observer preserves old rows: a status at
                        # row 26 can move into the predicate before any redraw.
                        # Require fresh output before issuing the next resize,
                        # otherwise ConPTY can coalesce the two size changes.
                        ui.wait_until(lambda lines:len(ui.output)>before and any(
                            'Ln 1, Col 1 ' in line for line in lines[-3:]))
                        ui.read(.03)
                row,line=next((i,line) for i,line in enumerate(ui.screen.display) if 'marker ' in line)
                x=line.index('marker ')
                for _ in range(25):
                    for offset,column in [(1,2),(2,3)]:
                        keys=f'\x1b[<0;{x+offset+1};{row+1}M\x1b[<0;{x+offset+1};{row+1}m'
                        action(ui,'mouse:'+label,keys,f'Ln 1, Col {column} ')
                ui.send('\x1b[H');ui.wait('Ln 1, Col 1 ')
                ui.send('\x14');ui.wait('Tabs 2/2')
                for _ in range(30):
                    action(ui,'tabs:'+label,'\x1b[18~','Tabs 1/2')
                    action(ui,'tabs:'+label,'\x1b[18~','Tabs 2/2')
                ui.send('\x1b[18~');ui.wait('Tabs 1/2')
                for _ in range(25):
                    action(ui,'search-open:'+label,'\x06','FIND / REPLACE')
                    action(ui,'search-query:'+label,'m','› m│')
                    action(ui,'search-close:'+label,'\x1b',lambda s:'FIND / REPLACE' not in s)
                    # Query persists; clear it next time through the panel's Ctrl+U.
                    ui.send('\x06\x15');ui.wait('› │');ui.send('\x1b');ui.read(.01)
                ui.send('\x11');ui.wait('Discard ALL');ui.send('y');ui.wait('TERM_STATE_OK');ui.wait('CHILD_EXIT=0')
            finally:
                (out/(label+'.log')).write_bytes(ui.output);ui.close()
            if trace.exists():
                for r in csv.DictReader(trace.open(encoding='utf-8')):
                    if r['stage']=='dropped_samples':dropped+=int(r['microseconds'])
                    if r['stage']=='input_to_flush':
                        groups[(label,r['operation'],r['phase'])].append(int(r['microseconds'])/1000)
                        if int(r['microseconds'])>50000:investigations.append({'corpus':label,**r})
    summary=[{'corpus':k[0],'operation':k[1],'phase':k[2],**stats(v)} for k,v in sorted(groups.items())]
    observed=defaultdict(list)
    for r in RESULTS:observed[r['action']].append(r['ms'])
    report={'profile':'release','platform':platform.platform(),'terminal':'ConPTY' if os.name=='nt' else 'native PTY',
            'term':os.getenv('TERM'),'colorterm':os.getenv('COLORTERM'),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
            'source_sha256':source_hash(),'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),
            'source_hash_scope':'Cargo.toml, Cargo.lock, crates/**/*.rs, crates/**/Cargo.toml, src/**/*.rs; includes relative paths and original working-tree bytes',
            'trace_enabled':not args.no_trace,'dropped_samples':dropped,'sessions':sessions,'application_groups':summary,
            'pty_samples':RESULTS,'pty_groups':[{'operation':k,**stats(v)} for k,v in sorted(observed.items())],
            'warnings':[g for g in summary if g['status']=='needs_investigation'],
            'investigation_threshold_ms':50,'application_investigations':investigations,
            'pty_investigations':[r for r in RESULTS if r['ms']>50],
            'limits':['Start is main-loop receipt after decode, excludes adapter queue','Cold is first category per document in process, not cold disk',
                      'No physical-paint measurement','Finite samples and trace overhead; p99 is descriptive','No universal latency SLA claimed']}
    (out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    (ROOT/'target/interaction-latest-path.txt').write_text(str(out),encoding='utf-8')
    print(json.dumps({'report':str(out/'report.json'),'groups':summary,'dropped':dropped,'warnings':len(report['warnings'])},indent=2))
    assert dropped==0,'Trace dropped rows; run is not valid for acceptance'


if __name__=='__main__':main()
