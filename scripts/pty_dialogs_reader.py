"""Native settings/picker/details/linked Markdown reader behavior and captures.
Only owned temporary files and an explicitly isolated preferences path change.
"""
import os
from pathlib import Path
import tempfile
from pty_smoke import ROOT, Session
from tui_preview import snapshot as capture

BINARY = ROOT / "target/release" / ("guinea.exe" if os.name == "nt" else "guinea")
F7, F8, F9 = "\x1b[18~", "\x1b[19~", "\x1b[20~"


def snapshot(session,name):
    session.read(0.12)
    capture(session,name)


def main():
    with tempfile.TemporaryDirectory(prefix="guineanote-ux-") as directory:
        root=Path(directory); prefs=root / "preferences.toml"; source=root / "Guide.md"
        original="# Build something thoughtful\n\nA **focused reader** for your ideas, code and data. Use `F7` to choose your space.\n\n## Today\n\n- [x] Settings and file navigation\n- [x] Live review tabs\n- [ ] The next idea\n\n> [!NOTE]\n> Read with context. Keep your source intact.\n\n## A little code\n\n```rust\n#[derive(Debug)]\nstruct Workspace { name: String }\nfn main() {\n    let project = Workspace { name: \"GuineaNote\".into() };\n    println!(\"Hello, {}\\n\", project.name);\n}\n```\n\n## Data at a glance\n\n| Format | Reader | State |\n|:---|:---|:---|\n| Markdown | Rich document | Ready |\n| JSON / YAML | Structured | Ready |\n| CSV / TSV | Table | Ready |\n\n## Explore\n\n[Project notes](https://example.com/notes)\n"
        source.write_bytes(original.encode()); (root/"main.rs").write_text('fn main() { println!("hello"); }\n',encoding="utf-8")
        first=Session([str(BINARY),str(root),str(source)],colors=True,preferences_path=prefs)
        try:
            first.wait("Opened"); first.resize(42,148); first.read(.1)
            first.send(F8); first.wait("SETTINGS · F8"); snapshot(first,"settings-en")
            first.send("v\r"); first.wait("SOẠN THẢO"); first.send("\x11"); first.wait("TERM_STATE_OK"); first.wait("CHILD_EXIT=0")
        finally: first.close()
        assert '"vi"' in prefs.read_text(encoding="utf-8")
        config=root/"ui.toml"; config.write_text("tab_width = 4\n",encoding="utf-8")
        session=Session([str(BINARY),"--config",str(config),str(root),str(source)],colors=True,preferences_path=prefs)
        try:
            session.resize(42,148); session.wait("Đã mở")
            session.send(F8); session.wait("THIẾT LẬP · F8"); session.send("e\x1b")
            session.wait_until(lambda lines:"THIẾT LẬP · F8" not in "\n".join(lines)); assert "SOẠN THẢO" in "\n".join(session.screen.display)
            session.send(F8); session.wait("THIẾT LẬP · F8"); session.send("e\r"); session.wait("EDITOR")
            session.send("\x0f"); session.wait("OPEN FILE"); session.wait("main.rs"); snapshot(session,"open-file-picker")
            session.send("main\r"); session.wait('println!("hello");'); session.send("\x17"); session.wait("Build something thoughtful")
            session.send(F9); session.wait("FILE DETAILS · F9"); session.wait("Characters"); snapshot(session,"file-details")
            session.send("\x1b"); session.wait_until(lambda lines:"FILE DETAILS · F9" not in "\n".join(lines))
            session.send(F7); session.wait("REVIEW OPTIONS · F7"); snapshot(session,"review-options")
            session.send("1"); session.wait("MARKDOWN READER"); session.wait("DOCUMENT"); snapshot(session,"markdown-split")
            session.send(F7); session.wait("REVIEW OPTIONS · F7"); session.send("2"); session.wait(" REVIEW   Ln"); session.wait("CONTENTS"); session.wait("DOCUMENT"); snapshot(session,"markdown-review-tab")
            session.send("\t\x1b[B\x1b[B\r"); session.wait("Hello,"); snapshot(session,"markdown-code-card")
            session.send("\x13"); session.wait("Review tab is read-only"); assert source.read_bytes()==original.encode()
            session.send("\x1b"); session.wait(" EDIT   Ln"); session.send("Live-marker\r\r")
            session.send("\x1b[17~"); session.wait(" REVIEW   Ln"); session.send("\x1b[H"); session.wait("Live-marker")
            assert source.read_bytes()==original.encode()
            session.send("\x17"); session.wait(" EDIT   Ln"); session.send("\x13"); session.wait_file(source,"Live-marker\n\n"+original)
            session.send("\x11"); session.wait("TERM_STATE_OK"); session.wait("CHILD_EXIT=0")
            print("PASS: settings persist/cancel, picker, details, split/tab review, outline, read-only history, live updates, exact save")
        finally:
            (ROOT/"target/pty-dialogs-reader.log").write_bytes(session.output); session.close()


if __name__=="__main__": main()
