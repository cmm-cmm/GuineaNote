"""Capture the real ConPTY/PTY screen as text and a terminal-cell PNG preview.

Uses pty_smoke's dev tools plus Pillow; never changes an existing project file.
    python -m pip install --target target/pty-tools Pillow
    python scripts/tui_preview.py
"""
import os
from pathlib import Path

from pty_smoke import ROOT, Session
from PIL import Image, ImageDraw, ImageFont

PALETTE = {
    "default": "#f8f5ec", "black": "#0c111c", "red": "#efaa91", "green": "#86d38a",
    "brown": "#dab58a", "yellow": "#dab58a", "blue": "#bdd9b3", "magenta": "#bad68a",
    "cyan": "#86d38a", "white": "#f8f5ec", "brightblack": "#91816b",
    "brightred": "#efaa91", "brightgreen": "#86d38a", "brightyellow": "#dab58a",
    "brightblue": "#bdd9b3", "brightmagenta": "#bad68a", "brightcyan": "#86d38a", "brightwhite": "#ffffff",
}


def color(value, default):
    if value == "default":
        return default
    if value in PALETTE:
        return PALETTE[value]
    if len(value) == 6:
        return "#" + value
    return default


def snapshot(session, name):
    folder = ROOT / "target" / "previews"
    folder.mkdir(exist_ok=True)
    (folder / f"{name}.txt").write_text("\n".join(session.screen.display), encoding="utf-8")
    font_path = Path("C:/Windows/Fonts/consola.ttf") if os.name == "nt" else Path("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf")
    font = ImageFont.truetype(str(font_path), 20)
    bold_path = Path("C:/Windows/Fonts/consolab.ttf") if os.name == "nt" else font_path
    strong = ImageFont.truetype(str(bold_path), 20)
    cell_width = round(font.getlength("M"))
    cell_height = sum(font.getmetrics())
    pad = 20
    width, height = session.screen.columns * cell_width + pad * 2, session.screen.lines * cell_height + pad * 2
    image = Image.new("RGB", (width, height), "#0c111c")
    draw = ImageDraw.Draw(image)
    for row in range(session.screen.lines):
        for column in range(session.screen.columns):
            char = session.screen.buffer[row][column]
            x, y = pad + column * cell_width, pad + row * cell_height
            fg, bg = color(char.fg, "#f8f5ec"), color(char.bg, "#0c111c")
            if char.reverse:
                fg, bg = bg, fg
            draw.rectangle((x, y, x + cell_width - 1, y + cell_height - 1), fill=bg)
    for row in range(session.screen.lines):
        for column in range(session.screen.columns):
            char = session.screen.buffer[row][column]
            fg = color(char.bg if char.reverse else char.fg, "#f8f5ec")
            if char.data and char.data != " ":
                draw.text((pad + column * cell_width, pad + row * cell_height), char.data, font=strong if char.bold else font, fill=fg)
    image.save(folder / f"{name}.png")
    print(str(folder / f"{name}.png"))


def main():
    binary = ROOT / "target" / "release" / ("guineanote.exe" if os.name == "nt" else "guineanote")
    session = Session([str(binary), str(ROOT)], colors=True)
    try:
        session.resize(36, 124)
        session.wait("EXPLORER")
        session.read(0.8)
        snapshot(session, "guineanote-welcome")
        session.command("open")
        session.wait("Open file or folder")
        session.write(str(ROOT / "README.md") + "\r")
        session.wait("Opened")
        session.command("open")
        session.wait("Open file or folder")
        session.write(str(ROOT / "src" / "main.rs") + "\r")
        session.wait("Opened")
        session.write("\x1bOS")  # F4 focuses explorer.
        session.write("\x1b[H" + "\x1b[B" * 5 + "\x1b[C")
        session.read(0.5)
        snapshot(session, "guineanote-workspace")
        session.write("\x11")
        session.wait("TERM_STATE_OK")
        session.wait("CHILD_EXIT=0")
    finally:
        session.close()


if __name__ == "__main__":
    main()
