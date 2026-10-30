#!/usr/bin/env python3
"""Build the auditable one-shot TV inspector; no downloads or TV execution."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--sdk", default=os.environ.get("ANDROID_HOME"))
parser.add_argument("--jdk", default=os.environ.get("JAVA_HOME"))
args = parser.parse_args()
if not args.sdk or not args.jdk:
    parser.error("Set ANDROID_HOME and JAVA_HOME or pass --sdk and --jdk")
sdk, jdk = Path(args.sdk), Path(args.jdk)
android = sdk / "platforms/android-36/android.jar"
d8 = sdk / "build-tools/36.1.0/d8"
source = root / "assets/inspector/BridgecoilInspector.java"
output = root / "crates/adb-core/assets/bridgecoil-inspector.jar"
output.parent.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="bridgecoil-inspector-") as directory:
    build = Path(directory)
    classes, dex = build / "classes", build / "dex"
    classes.mkdir(); dex.mkdir()
    subprocess.run([str(jdk / "bin/javac"), "-source", "8", "-target", "8", "-classpath", str(android), "-d", str(classes), str(source)], check=True)
    subprocess.run([str(d8), "--release", "--min-api", "24", "--lib", str(android), "--output", str(dex), *[str(p) for p in sorted(classes.rglob("*.class"))]], check=True, env={**os.environ, "JAVA_HOME": str(jdk)})
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        entry = zipfile.ZipInfo("classes.dex", date_time=(1980, 1, 1, 0, 0, 0))
        entry.compress_type = zipfile.ZIP_DEFLATED
        archive.writestr(entry, (dex / "classes.dex").read_bytes())
manifest = {"sourceSha256": hashlib.sha256(source.read_bytes()).hexdigest(), "jarSha256": hashlib.sha256(output.read_bytes()).hexdigest(), "compileSdk": 36, "buildTools": "36.1.0", "minSdk": 24}
(output.parent / "inspector-build.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps(manifest))
