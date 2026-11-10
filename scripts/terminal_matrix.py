"""Local terminal checks, optionally through an explicitly supplied SSH host/tmux.

Uses owned disposable fixtures, a prebuilt binary and no GitHub Actions. Remote
mode requires python3 and the binary already installed; never installs services
or changes SSH configuration. Examples:
  python scripts/terminal_matrix.py
  python scripts/terminal_matrix.py --tmux
  python scripts/terminal_matrix.py --ssh user@host --remote-binary /opt/guineanote
"""
import argparse
import base64
import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import subprocess
import tempfile
import time
from pty_smoke import ROOT, Session


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--binary',type=Path,default=ROOT/'target/release'/('guineanote.exe' if os.name=='nt' else 'guineanote'))
    parser.add_argument('--ssh');parser.add_argument('--remote-binary');parser.add_argument('--tmux',action='store_true')
    args=parser.parse_args()
    if args.ssh and (args.ssh.startswith('-') or any(c.isspace() for c in args.ssh) or not args.remote_binary):
        parser.error('SSH requires a host without whitespace/options and --remote-binary')

    def remote(code):
        r=subprocess.run(['ssh',args.ssh,shlex.join(['python3','-c',code])],capture_output=True,text=True,encoding='utf-8',timeout=30)
        if r.returncode:raise RuntimeError(r.stderr)
        return json.loads(r.stdout)

    local=tempfile.TemporaryDirectory(prefix='guineanote-terminal-matrix-')
    base=remote("import tempfile,json;print(json.dumps(tempfile.mkdtemp(prefix='guineanote-terminal-matrix-')))") if args.ssh else local.name
    results=[];report_path=ROOT/'target'/('terminal-matrix-'+str(time.time_ns())+'.json')
    report={'observer_platform':platform.platform(),'transport':'ssh' if args.ssh else 'local PTY',
            'tmux_requested':args.tmux,'cases':results,'github_actions':False,
            'coverage_note':'Only the selected runtime/transport is exercised; no cross-platform certification.'}
    try:
        cases=[('truecolor',100,28,''),('80x24',80,24,''),('40x12',40,12,''),
               ('monochrome',80,24,'no_color = true\n'),('ansi16',80,24,'ansi_colors = true\n'),
               ('keyboard-only',80,24,'mouse = false\n')]
        for label,cols,rows,options in cases:
            config='language = "en"\n'+options
            original=b'\xef\xbb\xbf'+b'first marker\n'
            if args.ssh:
                data=base64.b64encode(json.dumps({'base':base,'label':label,'config':config,'original':base64.b64encode(original).decode()}).encode()).decode()
                paths=remote("import json,base64,pathlib;d=json.loads(base64.b64decode("+repr(data)+"));p=pathlib.Path(d['base'])/d['label'];p.mkdir();(p/'file.txt').write_bytes(base64.b64decode(d['original']));(p/'config.toml').write_text(d['config']);print(json.dumps([str(p/'file.txt'),str(p/'config.toml'),str(p/'preferences.toml')]))")
                file,config_path,preferences=paths
                command=['env','TERM='+('linux' if label=='ansi16' else 'xterm-256color'),
                         'GUINEANOTE_PREFERENCES='+preferences,args.remote_binary,'--config',config_path,file]
                if args.tmux:
                    command=['tmux','-L','guineanote-matrix-'+str(time.time_ns()),'new-session',shlex.join(command)]
                arguments=['ssh','-tt',args.ssh,shlex.join(command)]
            else:
                directory=Path(base)/label;directory.mkdir()
                file=directory/'file.txt';file.write_bytes(original)
                config_path=directory/'config.toml';config_path.write_text(config,encoding='utf-8')
                preferences=directory/'preferences.toml'
                arguments=[str(args.binary.resolve()),'--config',str(config_path),str(file)]
                if args.tmux:
                    if not shutil.which('tmux'):
                        results.append({'case':label,'status':'SKIP','reason':'tmux is not installed on this runtime'});continue
                    arguments=['tmux','-L','guineanote-matrix-'+str(time.time_ns()),'new-session',shlex.join(arguments)]
            ui=Session(arguments,preferences_path=Path(preferences) if not args.ssh else None,
                       env={'TERM':'linux' if label=='ansi16' else 'xterm-256color'})
            started=time.perf_counter()
            try:
                ui.resize(rows,cols);ui.wait('Opened');ui.wait('first marker')
                ui.send('\x1b[200~Việt e\u0301 👩‍💻\nsecond\r\n\x1b[201~')
                ui.wait('second');ui.send('\x13');ui.wait('● Saved')
                expected=b'\xef\xbb\xbf'+'Việt e\u0301 👩‍💻\nsecond\r\nfirst marker\n'.encode()
                actual=base64.b64decode(remote('import pathlib,base64,json;print(json.dumps(base64.b64encode(pathlib.Path('+repr(file)+').read_bytes()).decode()))')) if args.ssh else file.read_bytes()
                assert actual==expected, 'Paste/save changed BOM, Unicode or newline bytes'
                ui.send('\x1b[19~');ui.wait('SETTINGS · F8');ui.send('\x1b');ui.read(.02)
                before=len(ui.output);ui.send('\x1b[C');ui.wait('Ln 3, Col 2 ')
                emitted=len(ui.output)-before
                ui.send('\x1b[23~');ui.wait('F11 Workspace');ui.send('\x1b[23~')
                ui.wait_until(lambda lines:'F11 Workspace' not in '\n'.join(lines) and '1/1' in '\n'.join(lines))
                ui.send('\x11');ui.wait('TERM_STATE_OK');ui.wait('CHILD_EXIT=0')
                results.append({'case':label,'status':'PASS','viewport':[cols,rows],
                                'exact_paste_save':True,'cursor_output_bytes_observed':emitted,
                                'elapsed_ms':round((time.perf_counter()-started)*1000,3)})
            finally:
                (ROOT/'target'/('terminal-matrix-'+label+'.log')).write_bytes(ui.output);ui.close()
        report['status']='PASS' if results and all(r['status']=='PASS' for r in results) else 'INCOMPLETE'
    except Exception as error:
        report['status']='FAIL';report['error']=str(error)
        raise
    finally:
        report_path.write_text(json.dumps(report,indent=2),encoding='utf-8')
        (ROOT/'target/terminal-matrix-latest-path.txt').write_text(str(report_path),encoding='utf-8')
        print(json.dumps(report,indent=2))
        if args.ssh:
            # Delete only the exact owned temp root, after verifying its parent/name.
            remote("import pathlib,tempfile,shutil,json;p=pathlib.Path("+repr(base)+").resolve();assert p.parent==pathlib.Path(tempfile.gettempdir()).resolve() and p.name.startswith('guineanote-terminal-matrix-');shutil.rmtree(p);print(json.dumps(True))")
        local.cleanup()


if __name__=='__main__':main()
