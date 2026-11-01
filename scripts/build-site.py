#!/usr/bin/env python3
"""Stage the website with the canonical screenshots and the user guide."""
from pathlib import Path
import shutil
import subprocess
import sys

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
    subprocess.run(
        [sys.executable, "-m", "mkdocs", "build", "--strict", "--site-dir", str(OUTPUT / "guide")],
        cwd=ROOT,
        check=True,
    )
    print(f"Staged website, the user guide and {len(images)} screenshots in {OUTPUT}")


if __name__ == "__main__":
    build()
