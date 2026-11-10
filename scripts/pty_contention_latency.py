"""Repeated opt-in Agent contention; application/PTY timings stay separate."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time
from collections import defaultdict
from pty_interaction_latency import BINARY, source_hash, stats
from pty_smoke import ROOT


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--runs',type=int,default=10)
    args=parser.parse_args();assert args.runs>0
    out=ROOT/'target'/('contention-'+str(time.time_ns()));out.mkdir()
    groups=defaultdict(list);investigations=[];pty=[];runs=[];dropped=0
    for run in range(args.runs):
        folder=out/str(run);folder.mkdir();trace=folder/'trace.csv'
        env=dict(os.environ,GUINEANOTE_TRACE=str(trace))
        with (folder/'console.txt').open('w',encoding='utf-8') as log:
            result=subprocess.run([sys.executable,'-X','utf8',str(ROOT/'scripts/pty_agent_latency.py'),
                                   '--report-dir',str(folder),'--record-only'],stdout=log,stderr=subprocess.STDOUT,env=env)
        assert result.returncode==0, f'Functional failure; raw log: {folder}'
        report=json.loads((folder/'responsiveness-agent.json').read_text())
        runs.append({'run':run,'concurrent_pty_samples':report['concurrent_samples'],
                     'agent_analysis_completion_ms':report['agent_analysis_completion_ms']})
        pty.extend({'run':run,**r} for r in report['results'])
        rows=list(csv.DictReader(trace.open(encoding='utf-8')))
        assert any(r['stage']=='dropped_samples' for r in rows), 'Trace was not finalized'
        for row in rows:
            if row['stage']=='dropped_samples':dropped+=int(row['microseconds'])
            if row['stage']=='input_to_flush':
                context='agent_active' if row['agent_active']=='true' else 'agent_idle'
                groups[(row['operation'],row['phase'],context)].append(int(row['microseconds'])/1000)
                if int(row['microseconds'])>50000:investigations.append({'run':run,**row})
    summary=[{'operation':k[0],'phase':k[1],'context':k[2],**stats(v)} for k,v in sorted(groups.items())]
    report={'profile':'release','platform':platform.platform(),'git_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
            'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'source_sha256':source_hash(),
            'investigation_threshold_ms':50,'runs':runs,'application_groups':summary,'dropped_samples':dropped,
            'application_investigations':investigations,'pty_samples':pty,'pty_investigations':[r for r in pty if r['ms']>50],
            'limits':['Agent active means owner state at event receipt, not proof of simultaneous CPU execution',
                      'Main-loop receipt after decode; PTY observations separate','Insufficient per-category samples remain explicit']}
    (out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    (ROOT/'target/contention-latest-path.txt').write_text(str(out),encoding='utf-8')
    print(json.dumps({'report':str(out/'report.json'),'groups':summary,'investigations':len(investigations),'dropped':dropped},indent=2))
    assert dropped==0,'Trace loss invalidates acceptance'


if __name__=='__main__':main()
