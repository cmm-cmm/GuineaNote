"""Render the README screenshots from the real editor running in a PTY.

Each scene drives `guinea` with sample files in a temporary folder (private
preferences, session and internal clipboard) and draws the terminal cells to
a PNG with the same colors the terminal received. Output: docs/images/*.png.
Run after `cargo build --release`; Pillow goes into target/pty-tools:

    python -m pip install --target target/pty-tools Pillow
    python -X utf8 scripts/readme_screenshots.py [--root D:/demo]

Paths appear on screen, so `--root` puts the sample project in a short,
neutral folder (created fresh and removed afterwards); the default is a
temporary folder.
"""
import argparse
import os
import shutil
from pathlib import Path
import tempfile

from pty_smoke import ROOT, Session
from PIL import Image, ImageDraw, ImageFont

BINARY = ROOT / "target" / "release" / ("guinea.exe" if os.name == "nt" else "guinea")
OUT = ROOT / "docs" / "images"
ROWS, COLS = 30, 112
BACKGROUND, FOREGROUND = "#090e17", "#f8f5ec"
# ANSI names pyte reports for 16-color output (truecolor arrives as hex).
PALETTE = {
    "black": "#090e17", "red": "#efaa91", "green": "#86d38a", "brown": "#dbad5b",
    "yellow": "#dbad5b", "blue": "#bdd9b3", "magenta": "#bad68a", "cyan": "#86d38a",
    "white": "#f8f5ec", "brightblack": "#91816b", "brightred": "#efaa91",
    "brightgreen": "#86d38a", "brightyellow": "#dbad5b", "brightblue": "#bdd9b3",
    "brightmagenta": "#bad68a", "brightcyan": "#86d38a", "brightwhite": "#ffffff",
}
F2, F8, F12 = "\x1bOQ", "\x1b[19~", "\x1b[24~"
CTRL_F, CTRL_Q, ALT_R = "\x06", "\x11", "\x1br"

MAIN_RS = """use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Counts words in a UTF-8 file, keeping the original bytes untouched.
pub fn word_counts(path: &Path) -> std::io::Result<HashMap<String, usize>> {
    let text = fs::read_to_string(path)?;
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric());
        if !word.is_empty() {
            *counts.entry(word.to_lowercase()).or_insert(0) += 1;
        }
    }
    Ok(counts)
}

fn top(counts: &HashMap<String, usize>, limit: usize) -> Vec<(&str, usize)> {
    let mut pairs: Vec<_> = counts.iter().map(|(w, n)| (w.as_str(), *n)).collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    pairs.truncate(limit);
    pairs
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "notes.md".into());
    match word_counts(Path::new(&path)) {
        Ok(counts) => {
            for (word, count) in top(&counts, 10) {
                println!("{count:>6}  {word}");
            }
        }
        Err(error) => eprintln!("cannot read {path}: {error}"),
    }
}
"""

NOTES_MD = """# Release checklist

Guinea Note keeps **every byte** you did not edit: BOM, CRLF and Unicode
such as *tiếng Việt* stay exactly as they were.

## Before tagging

- [x] Run `cargo test --workspace`
- [x] PTY probes on Windows and Linux
- [ ] Update the changelog

> [!NOTE]
> Every interaction above 50 ms is investigated before a release.

## Targets

| Platform | Archive | Status |
| --- | --- | --- |
| Windows x64 | zip | ready |
| Linux x64 | tar.gz | ready |

## Commands

```sh
cargo build --workspace --release --locked
python -X utf8 scripts/pty_smoke.py
```
"""

CONFIG_TOML = """# Guinea Note configuration
language = "en"
tab_width = 4
mouse = true
clipboard = "auto"
recovery = true
session = true
"""


def font(bold=False):
    if os.name == "nt":
        candidates = ["C:/Windows/Fonts/CascadiaMono.ttf", "C:/Windows/Fonts/consola.ttf"]
    else:
        candidates = ["/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"]
    path = next(p for p in candidates if Path(p).exists())
    face = ImageFont.truetype(path, 20)
    if bold:
        try:
            face.set_variation_by_name("Bold")
        except (OSError, ValueError):
            pass
    return face


def color(value, default):
    if value == "default":
        return default
    if value in PALETTE:
        return PALETTE[value]
    if len(value) == 6:
        return "#" + value
    return default


def render(session, name):
    regular, strong = font(), font(bold=True)
    cell_w = round(regular.getlength("M"))
    ascent, descent = regular.getmetrics()
    cell_h = ascent + descent + 2
    pad = 18
    screen = session.screen
    image = Image.new("RGB", (screen.columns * cell_w + 2 * pad, screen.lines * cell_h + 2 * pad), BACKGROUND)
    draw = ImageDraw.Draw(image)
    for row in range(screen.lines):
        line = screen.buffer[row]
        for column in range(screen.columns):
            char = line[column]
            fg, bg = color(char.fg, FOREGROUND), color(char.bg, BACKGROUND)
            if char.reverse:
                fg, bg = bg, fg
            x, y = pad + column * cell_w, pad + row * cell_h
            draw.rectangle((x, y, x + cell_w - 1, y + cell_h - 1), fill=bg)
    for row in range(screen.lines):
        line = screen.buffer[row]
        for column in range(screen.columns):
            char = line[column]
            if not char.data or char.data == " ":
                continue
            fg = color(char.bg if char.reverse else char.fg, FOREGROUND)
            x, y = pad + column * cell_w, pad + row * cell_h + 1
            draw.text((x, y), char.data, font=strong if char.bold else regular, fill=fg)
            if char.underscore:
                draw.line((x, y + ascent + 1, x + cell_w - 1, y + ascent + 1), fill=fg)
    OUT.mkdir(parents=True, exist_ok=True)
    image.save(OUT / f"{name}.png", optimize=True)
    print(OUT / f"{name}.png")


def launch(root, files, language="en"):
    env = {"GUINEANOTE_SESSION": str(root / "session.toml"),
           "GUINEANOTE_RECOVERY_DIR": str(root / "recovery")}
    session = Session([str(BINARY), "--language", language, *map(str, files)], colors=True,
                      preferences_path=root / "preferences.toml", env=env)
    session.resize(ROWS, COLS)
    return session


def settle(session):
    session.read(0.6)


def quit(session):
    session.send(CTRL_Q)
    session.wait("TERM_STATE_OK", timeout=20)
    session.wait("CHILD_EXIT=0")
    session.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    args = parser.parse_args()
    if args.root:
        if args.root.exists():
            raise SystemExit(f"{args.root} exists; pass a new folder")
        args.root.mkdir(parents=True)
        try:
            scenes(args.root)
        finally:
            shutil.rmtree(args.root)
    else:
        with tempfile.TemporaryDirectory(prefix="guineanote-screens-") as directory:
            scenes(Path(directory))


def scenes(root):
    if True:
        project = root / "wordcount"
        (project / "src").mkdir(parents=True)
        main_rs, notes, config = project / "src" / "main.rs", project / "NOTES.md", project / "guinea.toml"
        main_rs.write_text(MAIN_RS, encoding="utf-8", newline="\n")
        notes.write_text(NOTES_MD, encoding="utf-8", newline="\r\n")
        config.write_text(CONFIG_TOML, encoding="utf-8", newline="\n")

        # 1. Editor: tabs, syntax colors, bookmarks, the caret line.
        ui = launch(root, [notes, config, main_rs])
        ui.wait("word_counts")
        ui.command("goto")
        ui.wait("Go to line")
        ui.write("6\r")
        ui.command("bookmark-toggle")
        ui.command("goto")
        ui.wait("Go to line")
        ui.write("19:13\r")
        ui.command("bookmark-toggle")
        ui.command("select-word-right")  # "pairs": smart highlighting marks the others
        settle(ui)
        render(ui, "editor")

        # 2. Find/Replace with a regular expression.
        ui.send(CTRL_F)
        ui.wait("FIND / REPLACE")
        ui.send(r"fn (\w+)")
        ui.send(ALT_R)
        ui.wait("[x] Regex")
        ui.wait("Match 1 /")
        ui.send("\t" + "fn renamed_$1")
        settle(ui)
        render(ui, "find-replace")
        ui.send("\x1b")
        ui.wait_until(lambda rows: "FIND / REPLACE" not in "\n".join(rows))

        # 3. F2 action list filtered to line tools.
        ui.send(F2)
        ui.wait("Command (")
        ui.send("lines")
        settle(ui)
        render(ui, "actions")
        ui.send("\x1b")
        settle(ui)

        # 4. Markdown preview beside its source.
        ui.command("prev")
        ui.command("prev")
        ui.wait("Release checklist")
        ui.command("split")
        settle(ui)
        render(ui, "markdown-preview")
        quit(ui)

        # 5. Vietnamese interface with the tab manager.
        ui = launch(root, [main_rs, notes, config], language="vi")
        ui.wait("word_counts")
        # Files open one after another; wait for the last tab.
        ui.wait("guinea.toml")
        ui.wait("3/3")
        ui.send(F12)
        settle(ui)
        render(ui, "tabs-vietnamese")
        ui.send("\x1b")
        settle(ui)
        quit(ui)


if __name__ == "__main__":
    main()
