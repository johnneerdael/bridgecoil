#!/usr/bin/env python3
"""Stage the website with the canonical user-guide screenshots."""
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "dist-site"


def build():
    if OUTPUT.is_symlink():
        raise SystemExit("Refusing to replace a symlink at dist-site")
    if OUTPUT.exists():
        shutil.rmtree(OUTPUT)
    shutil.copytree(ROOT / "site", OUTPUT, ignore=shutil.ignore_patterns("*.md"))
    screenshots = ROOT / "docs/guide/screenshots"
    images = sorted(screenshots.glob("*.png"))
    if not images:
        raise SystemExit("No user-guide screenshots found")
    (OUTPUT / "screenshots").mkdir(exist_ok=True)
    for source in images:
        shutil.copy2(source, OUTPUT / "screenshots" / source.name)
    print(f"Staged website and {len(images)} screenshots in {OUTPUT}")


if __name__ == "__main__":
    build()
