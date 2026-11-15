"""Build the website's images from the repository's own assets (site/, README).

Dev-only; needs Pillow (install it into target/, not the system):

    python -m pip install --target target/site-tools pillow==12.0.0
    python -X utf8 scripts/site/build_assets.py

Writes into site/:
- images/*.webp      README screenshots (docs/images/*.png), re-encoded as WebP
- assets/*.png       icon sizes cut from icon/png (mascot), apple-touch-icon
- assets/og-*.png    1200x630 social preview cards (English, Vietnamese)
- favicon.ico        copy of icon/ico/favicon.ico
The fonts are Windows system fonts (Segoe UI, Cascadia Mono), so run it on
Windows. Outputs are committed; rerun only when a source image changes.
"""
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "target" / "site-tools"))
from PIL import Image, ImageDraw, ImageFont  # noqa: E402

SITE = ROOT / "site"
FONTS = Path("C:/Windows/Fonts")
NAVY, PANEL, GREEN, MINT, CREAM = "#0B1320", "#101B2C", "#22C55E", "#86EFAC", "#FFF7E6"
SCREENSHOTS = ["editor", "find-replace", "markdown-preview", "actions", "tabs-vietnamese"]


def font(name, size):
    return ImageFont.truetype(str(FONTS / name), size)


def screenshots():
    out = SITE / "images"
    out.mkdir(parents=True, exist_ok=True)
    for name in SCREENSHOTS:
        image = Image.open(ROOT / "docs" / "images" / f"{name}.png").convert("RGB")
        image.save(out / f"{name}.webp", "WEBP", quality=88, method=6)


def icons():
    assets = SITE / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    master = Image.open(ROOT / "icon" / "png" / "guineanote-512.png").convert("RGBA")
    for size in (192, 512):
        master.resize((size, size), Image.LANCZOS).save(assets / f"icon-{size}.png", optimize=True)
    for size in (32, 96):
        master.resize((size, size), Image.LANCZOS).save(assets / f"mascot-{size}.png", optimize=True)
    touch = Image.new("RGBA", (180, 180), NAVY)
    touch.alpha_composite(master.resize((156, 156), Image.LANCZOS), (12, 12))
    touch.convert("RGB").save(SITE / "apple-touch-icon.png", optimize=True)
    shutil.copyfile(ROOT / "icon" / "ico" / "favicon.ico", SITE / "favicon.ico")


def og_card(path, subtitle, footer):
    width, height = 1200, 630
    card = Image.new("RGB", (width, height), NAVY)
    draw = ImageDraw.Draw(card)
    draw.rounded_rectangle((18, 18, width - 19, height - 19), radius=28, outline=GREEN, width=4)
    mascot = Image.open(ROOT / "icon" / "png" / "guineanote-512.png").convert("RGBA")
    mascot = mascot.resize((330, 330), Image.LANCZOS)
    card.paste(mascot, (64, 120), mascot)
    x = 440
    title = font("segoeuib.ttf", 104)
    draw.text((x, 118), "Guinea", font=title, fill=CREAM)
    draw.text((x + draw.textlength("Guinea", font=title), 118), "Note", font=title, fill=GREEN)
    size = 38
    while draw.textlength(subtitle, font=font("seguisb.ttf", size)) > width - x - 60:
        size -= 1
    draw.text((x + 4, 262 + (38 - size) // 2), subtitle, font=font("seguisb.ttf", size), fill=MINT)
    mono = font("CascadiaMono.ttf", 34)
    command = "npx guinea-note notes.md"
    box_width = draw.textlength("> " + command, font=mono) + 56
    draw.rounded_rectangle((x, 352, x + box_width, 430), radius=14, fill=PANEL, outline="#1E3A2F", width=2)
    draw.text((x + 28, 370), ">", font=mono, fill=GREEN)
    draw.text((x + 28 + draw.textlength("> ", font=mono), 370), command, font=mono, fill=CREAM)
    draw.text((x + 4, 470), footer, font=font("segoeui.ttf", 30), fill="#9FB3A8")
    draw.text((x + 4, 540), "guineanote.konexforge.com", font=font("seguisb.ttf", 28), fill=GREEN)
    card.save(path, optimize=True)


def main():
    screenshots()
    icons()
    og_card(SITE / "assets" / "og-image.png", "Notepad++-style text editor for the terminal",
            "Windows x64 · Linux x64 · Free & MIT · Rust")
    og_card(SITE / "assets" / "og-image-vi.png", "Trình soạn thảo kiểu Notepad++ cho terminal",
            "Windows x64 · Linux x64 · Miễn phí, MIT · Rust")
    for path in sorted(SITE.rglob("*")):
        if path.suffix in {".png", ".webp", ".ico"}:
            print(f"{path.stat().st_size:>8}  {path.relative_to(ROOT).as_posix()}")


if __name__ == "__main__":
    main()
