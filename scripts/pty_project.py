"""Native project files/search, F2 discovery, goto, focus and stale-buffer safety."""
import json
import os
from pathlib import Path
import tempfile
from pty_smoke import ROOT, Session
from pty_responsiveness import action, RESULTS
from tui_preview import snapshot

BINARY=ROOT/'target/release'/('guinea.exe' if os.name=='nt' else 'guinea')


def main():
    with tempfile.TemporaryDirectory(prefix='guineanote-project-') as d:
        root=Path(d)
        (root/'src').mkdir();(root/'other').mkdir();(root/'target').mkdir()
        main=root/'src/main.rs'; other=root/'other/main.rs'
        main.write_bytes('first marker\nneedle source\nViệt e\u0301 👩‍💻\n'.encode())
        other.write_bytes(b'other marker\nsecond\nneedle other\n')
        (root/'.gitignore').write_bytes(b'ignored.txt\n')
        (root/'ignored.txt').write_bytes(b'needle forbidden-marker')
        (root/'target/build.txt').write_bytes(b'needle forbidden-marker')
        ui=Session([str(BINARY),str(root)],colors=True)
        try:
            ui.wait('START HERE')
            action(ui,'project:open-files','\x05','PROJECT · Ctrl+E / Ctrl+G')
            action(ui,'project:path-query','src main','› src main│')
            ui.wait('1 matches')
            assert 'forbidden-marker' not in '\n'.join(ui.screen.display)
            snapshot(ui,'project-files')
            ui.send('\r');ui.wait('first marker')
            ui.wait('Opened');ui.read(.05)
            snapshot(ui,'workspace-menu-tabs-separated')
            action(ui,'project:find-across-files','\x07','PROJECT · Ctrl+E / Ctrl+G')
            action(ui,'project:content-query','needle','› needle│')
            ui.wait('2 matches')
            snapshot(ui,'project-text-search')
            # Lexical path order: other/main.rs, then src/main.rs.
            ui.send('\r');ui.wait('other marker');ui.wait('Ln 3, Col 7 ')
            ui.command('goto');ui.wait('Go to line[:column]')
            ui.send('2:2\r');ui.wait('Ln 2, Col 2 ')
            action(ui,'project:focus-on','\x1b[23~','F11 Workspace')
            assert 'EXPLORER' not in '\n'.join(ui.screen.display)
            snapshot(ui,'focus-editor')
            action(ui,'project:focus-off','\x1b[23~','EXPLORER')
            ui.command('files');ui.send('src main');ui.wait('1 matches');ui.send('\r');ui.wait('first marker')
            ui.send('unsaved ');ui.wait('unsaved first marker')
            ui.send('\x07needle');ui.wait('2 matches');ui.send('\x1b[B\r')
            ui.wait('Search result changed')
            ui.wait('unsaved first marker')
            assert main.read_bytes().startswith(b'first marker')
            ui.send('\x11');ui.wait('Discard ALL');ui.send('y');ui.wait('TERM_STATE_OK');ui.wait('CHILD_EXIT=0')
            report={'samples':RESULTS,'sample_count':len(RESULTS),'max_ms':max(r['ms'] for r in RESULTS),
                    'disk_unchanged':True,'stale_dirty_buffer_retained':True}
            (ROOT/'target/project-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
            print(json.dumps(report,indent=2))
        finally:
            (ROOT/'target/pty-project.log').write_bytes(ui.output);ui.close()


if __name__=='__main__':main()
