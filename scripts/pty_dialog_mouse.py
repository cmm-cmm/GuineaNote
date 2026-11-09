"""Native mouse dialogs, drag/scroll, double-click and separated tabs.
Owned fixtures only; fails any measured UI response >=100 ms.
"""
import json
import os
from pathlib import Path
import tempfile
from pty_responsiveness import action, RESULTS
from pty_smoke import ROOT, Session
from tui_preview import snapshot

BINARY=ROOT/"target/release"/("guineanote.exe" if os.name=="nt" else "guineanote")


def pointer(x,y,button=0,release=True):
    return f"\x1b[<{button};{x+1};{y+1}M"+(f"\x1b[<{button};{x+1};{y+1}m" if release else "")


def locate(ui,label,condition=None):
    ui.wait_until(lambda lines:any(label in line and (condition is None or condition in line) for line in lines))
    row,line=next((i,line) for i,line in enumerate(ui.screen.display)
                  if label in line and (condition is None or condition in line))
    return line.index(label),row


def click(ui,label,condition=None,button=0):
    x,y=locate(ui,label,condition)
    return pointer(x+1,y,button)


def close_x(ui,title):
    ui.wait_until(lambda lines:any(title in line and " x " in line for line in lines))
    row,line=next((i,line) for i,line in enumerate(ui.screen.display) if title in line)
    return pointer(line.rindex(" x ")+1,row)


def main():
    with tempfile.TemporaryDirectory(prefix="guineanote-dialog-mouse-") as directory:
        root=Path(directory)
        alpha=root/"alpha.txt"
        original="alpha first line\n"+"".join(f"line{n:03}\n" for n in range(1,81))
        alpha.write_bytes(original.encode())
        beta=root/"beta.json"
        beta.write_bytes(b'{"name":"GuineaNote"}')
        ui=Session([str(BINARY),str(root),str(alpha),str(beta)],colors=True)
        try:
            ui.resize(42,148)
            ui.wait("Tabs 2/2")
            action(ui,"mouse:activate-alpha",click(ui,"alpha.txt"," x "),"Tabs 1/2")
            ui.wait("alpha first line")
            nav=next(i for i,line in enumerate(ui.screen.display) if "Tabs 1/2" in line)
            tab=locate(ui,"alpha.txt"," x ")[1]
            assert tab>=nav+2,"No gap between navigation and tab labels"
            footer=next(line for line in ui.screen.display if "Ln 1, Col" in line)
            assert footer.index("UTF-8")>ui.screen.columns//2
            x,y=locate(ui,"alpha first line")
            action(ui,"mouse:drag-selection",pointer(x,y,release=False)+pointer(x+5,y,32,False)+f"\x1b[<0;{x+6};{y+1}m","SEL 5")
            ui.send("\x03\x14\x16")
            ui.wait("Tabs 3/3")
            ui.wait("Ln 1, Col 6 ")
            action(ui,"dialog:close-prompt","\x17","Keep working")
            ui.read(.04)
            snapshot(ui,"confirmation-en-redesign")
            action(ui,"dialog:keep-working",click(ui,"Keep working","Discard & close"),lambda s:"UNSAVED CHANGES / CONFIRM" not in s)
            assert "Tabs 3/3" in "\n".join(ui.screen.display)
            ui.send("\x17")
            ui.wait("UNSAVED CHANGES / CONFIRM")
            action(ui,"dialog:header-close",close_x(ui,"UNSAVED CHANGES / CONFIRM"),lambda s:"UNSAVED CHANGES / CONFIRM" not in s)
            ui.send("\x17")
            ui.wait("UNSAVED CHANGES / CONFIRM")
            action(ui,"dialog:discard-close",click(ui,"Discard & close"),"Tabs 2/2")
            action(ui,"mouse:return-alpha",click(ui,"alpha.txt"," x "),"Tabs 1/2")
            x,y=locate(ui,"alpha first line")
            action(ui,"mouse:wheel-editor",pointer(x+5,y,65,False),lambda s:"line003" in s and "alpha first line" not in s)
            assert "Ln 1, Col 6 " in "\n".join(ui.screen.display),"Wheel moved caret"
            action(ui,"mouse:keyboard-resume","\x1b[H","alpha first line")
            ui.send("dirty ")
            ui.wait("MODIFIED")
            ui.send(click(ui,"beta.json"," x "))
            ui.wait("Tabs 2/2")
            action(ui,"mouse:middle-close-inactive",click(ui,"alpha.txt"," x ",1),"UNSAVED CHANGES / CONFIRM")
            action(ui,"dialog:discard-inactive",click(ui,"Discard & close"),"Tabs 1/1")
            assert alpha.read_bytes()==original.encode()
            ui.send("\x0f")
            ui.wait("OPEN FILE")
            ui.send("alpha")
            ui.wait("1 entries")
            p=click(ui,"alpha.txt","Text")
            action(ui,"mouse:double-open-file",p+p,"Tabs 2/2")
            ui.wait("alpha first line")
            ui.send("\x10")
            ui.wait("OPEN TABS")
            ui.send("beta")
            ui.wait("1 matching")
            p=click(ui,"beta.json","SAVED")
            action(ui,"mouse:double-switch-tab",p+p,"Tabs 1/2")
            assert "OPEN TABS" not in "\n".join(ui.screen.display)
            ui.send("\x14draft\x13")
            ui.wait("Save to NEW path")
            saved=root/"saved.txt"
            ui.send(str(saved))
            ui.wait("saved.txt")
            snapshot(ui,"input-dialog-redesign")
            action(ui,"dialog:mouse-submit",click(ui,"Enter Submit"),lambda s:"Saving " in s or "● Saved" in s)
            ui.wait_file(saved,"draft")
            ui.wait("● Saved")
            ui.send("\x1b[19~")
            ui.wait("SETTINGS")
            ui.send(click(ui,"Tiếng Việt"))
            ui.send(click(ui,"Apply","Cancel"))
            ui.wait("SOẠN THẢO")
            ui.send("x\x11")
            ui.wait("THAY ĐỔI CHƯA LƯU / XÁC NHẬN")
            ui.wait("Bỏ và thoát")
            ui.read(.04)
            snapshot(ui,"confirmation-vi-redesign")
            action(ui,"dialog:quit-vietnamese",click(ui,"Bỏ và thoát"),"TERM_STATE_OK")
            ui.wait("CHILD_EXIT=0")
            assert beta.read_bytes()==b'{"name":"GuineaNote"}'
            report={"sample_count":len(RESULTS),"max_ms":max(r['ms'] for r in RESULTS),"samples":RESULTS,"failures":[r for r in RESULTS if r['ms']>=100]}
            (ROOT/"target/dialog-mouse-responsiveness.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
            print(json.dumps({k:v for k,v in report.items() if k!='samples'},indent=2))
            assert not report['failures'],"Dialog/mouse UI response budget exceeded"
        finally:
            (ROOT/"target/pty-dialog-mouse.log").write_bytes(ui.output)
            ui.close()


if __name__=="__main__":
    main()
