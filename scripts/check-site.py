#!/usr/bin/env python3
"""Check static site references before uploading the Pages artifact."""
from html.parser import HTMLParser
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parent.parent / "site"


class Page(HTMLParser):
    def __init__(self, path):
        super().__init__(convert_charrefs=True)
        self.path = path
        self.ids = set()
        self.references = []
        self.errors = []
        self.h1_count = 0
        self.feed(path.read_text())

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            if attrs["id"] in self.ids:
                self.errors.append(f"duplicate id: {attrs['id']}")
            self.ids.add(attrs["id"])
        if tag == "h1":
            self.h1_count += 1
        if tag == "img" and "alt" not in attrs:
            self.errors.append("image without alt text")
        for attr in ["src", "href"]:
            if attrs.get(attr):
                self.references.append(attrs[attr])
        if attrs.get("srcset"):
            self.references.extend(part.strip().split()[0] for part in attrs["srcset"].split(","))


def check():
    pages = {p: Page(p) for p in ROOT.rglob("*.html")}
    errors = []

    def reference(source, value):
        url = urlsplit(value)
        if url.scheme or url.netloc:
            return
        if url.path.startswith("/"):
            errors.append(f"{source.relative_to(ROOT)}: project-site link starts at domain root: {value}")
            return
        target = (source.parent / unquote(url.path)).resolve() if url.path else source
        if not target.is_relative_to(ROOT):
            errors.append(f"{source.relative_to(ROOT)}: reference leaves site directory: {value}")
            return
        if target.is_dir():
            target /= "index.html"
        if not target.is_file():
            errors.append(f"{source.relative_to(ROOT)}: missing target: {value}")
        elif url.fragment and target in pages and unquote(url.fragment) not in pages[target].ids:
            errors.append(f"{source.relative_to(ROOT)}: missing anchor: {value}")

    for path, page in pages.items():
        errors.extend(f"{path.relative_to(ROOT)}: {error}" for error in page.errors)
        if page.h1_count != 1:
            errors.append(f"{path.relative_to(ROOT)}: expected one h1, found {page.h1_count}")
        for value in page.references:
            reference(path, value)
    for path in ROOT.rglob("*.css"):
        for value in re.findall(r"url\([\"']?([^\"')]+)[\"']?\)", path.read_text()):
            reference(path, value)
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"Checked {len(pages)} pages: local links, anchors, images, styles, scripts and font references resolve.")


if __name__ == "__main__":
    check()
