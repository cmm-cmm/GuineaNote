"""Capture real review panes in both locales, using temporary example data."""
import os
from pathlib import Path
import tempfile
from pty_smoke import ROOT, Session
from tui_preview import snapshot


def main():
    binary = ROOT / "target" / "release" / ("guinea.exe" if os.name == "nt" else "guinea")
    with tempfile.TemporaryDirectory(prefix="guineanote-preview-") as directory:
        root = Path(directory)
        fixtures = [
            ("README.md", "# Không gian của ý tưởng\n\nGuineaNote — **tập trung** trong terminal.\n\n## Việc hôm nay\n\n- [x] Giao diện Việt / Anh\n- [x] Xem Markdown và dữ liệu\n- [ ] Viết thêm một ý tưởng\n\n> Mọi thay đổi đều bắt đầu từ một dòng.\n\n```rust\nfn main() {\n    let message = \"Xin chào, GuineaNote!\";\n}\n```\n\n| Định dạng | Chế độ xem |\n|---|---|\n| Markdown | Tài liệu |\n| JSON / XML | Cấu trúc |\n| CSV / TSV | Bảng dữ liệu |\n"),
            (".env.local", "# Local configuration\nAPP_NAME=GuineaNote\nAPP_LOCALE=vi\nAPP_PORT=8080\nAPI_URL=https://example.com/api\nMESSAGE='Xin chào, GuineaNote!'\nLITERAL='${APP_NAME}'\n"),
            ("settings.yaml", "app:\n  name: GuineaNote\n  languages: [vi, en]\nperformance:\n  interaction_budget_ms: 100\nformats:\n  - Markdown\n  - JSON5\n  - YAML\n"),
            ("customers.csv", "name,city,plan,active\nMinh,Hồ Chí Minh,Pro,true\nAn,Hà Nội,Team,true\nLinh,Đà Nẵng,Free,false\nAlex,London,Team,true\n"),
            ("settings.json", '{"project":"GuineaNote","languages":["vi","en"],"performance":{"interaction_budget_ms":100,"background_review":true},"formats":["Markdown","JSON","XML","CSV","TSV"]}'),
        ]
        for filename, content in fixtures:
            (root / filename).write_bytes(content.encode())
        for filename, _ in fixtures:
            session = Session([str(binary), "--language", "vi", str(root / filename)], colors=True)
            try:
                session.resize(38, 148)
                session.wait("Đã mở")
                session.send("\x1b[19~")
                session.wait("CHẾ ĐỘ REVIEW · F8")
                session.send("1")
                session.wait("XEM TRƯỚC · F8")
                session.read(0.35)
                snapshot(session, "review-vi-" + Path(filename).suffix[1:])
                session.send("\x1b[21~")
                session.wait("THIẾT LẬP · F10")
                session.send("e\r")
                session.wait("EDITOR")
                session.read(0.2)
                snapshot(session, "review-en-" + Path(filename).suffix[1:])
                session.send("\x11")
                session.wait("TERM_STATE_OK")
                session.wait("CHILD_EXIT=0")
            finally:
                session.close()


if __name__ == "__main__":
    main()
