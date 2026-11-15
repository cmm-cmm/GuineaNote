"""Check the website in site/ (standard library only; CI runs it).

    python -X utf8 scripts/site/check.py

- the version on the pages and in llms*.txt equals [workspace.package] version
  in Cargo.toml, so a version bump cannot leave the site behind;
- every local link and image (/..., #...) points at an existing file or id;
- every JSON-LD block parses, and the FAQPage questions equal the visible
  <summary> questions of the same page;
- text files use LF line endings.
"""
import json
import re
import sys
import tomllib
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SITE = ROOT / "site"
PAGES = {"/": SITE / "index.html", "/vi/": SITE / "vi" / "index.html", "/404.html": SITE / "404.html"}
errors = []


class Page(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.ids, self.links, self.jsonld, self.summaries = set(), [], [], []
        self._jsonld = self._summary = None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        for name in ("href", "src"):
            if name in attrs:
                self.links.append(attrs[name])
        if tag == "script" and attrs.get("type") == "application/ld+json":
            self._jsonld = []
        if tag == "summary":
            self._summary = []

    def handle_endtag(self, tag):
        if tag == "script" and self._jsonld is not None:
            self.jsonld.append("".join(self._jsonld))
            self._jsonld = None
        if tag == "summary" and self._summary is not None:
            self.summaries.append("".join(self._summary).strip())
            self._summary = None

    def handle_data(self, data):
        if self._jsonld is not None:
            self._jsonld.append(data)
        if self._summary is not None:
            self._summary.append(data)


def target(path):
    """Local URL path -> file in site/."""
    path = path.split("#")[0].split("?")[0]
    if path.endswith("/"):
        path += "index.html"
    return SITE / path.lstrip("/")


def main():
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    parsed = {}
    for url, file in PAGES.items():
        page = Page()
        page.feed(file.read_text(encoding="utf-8"))
        parsed[url] = page
    for url, page in parsed.items():
        for link in page.links:
            if link.startswith(("http://", "https://", "mailto:")):
                continue
            if link.startswith("#"):
                if link[1:] and link[1:] not in page.ids:
                    errors.append(f"{url}: no id for {link}")
                continue
            if not target(link).is_file():
                errors.append(f"{url}: missing file for {link}")
            elif "#" in link:
                fragment = link.split("#", 1)[1]
                other = parsed.get(link.split("#")[0] or url)
                if other is not None and fragment and fragment not in other.ids:
                    errors.append(f"{url}: no id for {link}")
        questions = []
        for block in page.jsonld:
            try:
                data = json.loads(block)
            except json.JSONDecodeError as error:
                errors.append(f"{url}: JSON-LD does not parse: {error}")
                continue
            for node in data.get("@graph", [data]):
                if node.get("@type") == "SoftwareApplication" and node.get("softwareVersion") != version:
                    errors.append(f"{url}: softwareVersion {node.get('softwareVersion')} != Cargo.toml {version}")
                if node.get("@type") == "FAQPage":
                    questions = [q["name"] for q in node["mainEntity"]]
        if questions and questions != page.summaries:
            errors.append(f"{url}: FAQPage questions differ from the visible <summary> questions")
        if url != "/404.html" and f"v{version}" not in file_text(PAGES[url]):
            errors.append(f"{url}: does not mention v{version}")
    for name in ("llms.txt", "llms-full.txt"):
        if f"version: {version}" not in file_text(SITE / name).lower().replace("current version", "version"):
            errors.append(f"{name}: does not state the current version {version}")
    for path in sorted(SITE.rglob("*")):
        if path.is_file() and path.suffix in {".html", ".css", ".js", ".txt", ".xml", ".webmanifest", ""}:
            if b"\r" in path.read_bytes():
                errors.append(f"{path.relative_to(ROOT).as_posix()}: CRLF line endings")
    if errors:
        print("\n".join(errors))
        raise SystemExit(f"site check failed: {len(errors)} problem(s)")
    print(f"site check passed: version {version}, {sum(len(p.links) for p in parsed.values())} links")


def file_text(path):
    return path.read_text(encoding="utf-8")


if __name__ == "__main__":
    main()
