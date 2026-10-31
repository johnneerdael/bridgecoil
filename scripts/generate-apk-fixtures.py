#!/usr/bin/env python3
"""Generate small, owned APK fixtures with the independent Android SDK signer.

Signing keys are generated in a temporary directory and never retained. No APK is installed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import zipfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--sdk", default=os.environ.get("ANDROID_HOME"))
parser.add_argument("--jdk", default=os.environ.get("JAVA_HOME"))
parser.add_argument("--output", type=Path, default=root / "tests/fixtures/apks")
args = parser.parse_args()
if not args.sdk or not args.jdk:
    parser.error("Set ANDROID_HOME and JAVA_HOME or supply --sdk and --jdk")
sdk, jdk = Path(args.sdk), Path(args.jdk)
tools = sdk / "build-tools/36.1.0"
env = {**os.environ, "JAVA_HOME": str(jdk), "PATH": str(jdk / "bin") + os.pathsep + os.environ.get("PATH", "")}
args.output.mkdir(parents=True, exist_ok=True)

def run(command, expected=0):
    result = subprocess.run([str(c) for c in command], capture_output=True, text=True, env=env)
    if result.returncode != expected:
        raise RuntimeError(f"Tool failed ({result.returncode}): {command[0]}\n{result.stdout}\n{result.stderr}")
    return result.stdout + result.stderr

with tempfile.TemporaryDirectory(prefix="bridgecoil-apk-fixtures-") as temp:
    work = Path(temp)
    password = "public-fixture-only"
    keys = {}
    for name, algorithm in [("rsa", "RSA"), ("ec", "EC"), ("other", "RSA")]:
        key = work / f"{name}.p12"
        command = [jdk / "bin/keytool", "-genkeypair", "-alias", "fixture", "-keystore", key,
                   "-storetype", "PKCS12", "-storepass", password, "-keypass", password,
                   "-dname", "CN=Bridgecoil disposable fixture,O=Tests,C=NL", "-validity", "3650", "-keyalg", algorithm]
        command += ["-groupname", "secp256r1"] if algorithm == "EC" else ["-keysize", "2048"]
        run(command)
        keys[name] = key

    def unsigned(name, version=1, target=34, split=None):
        directory = work / name
        directory.mkdir()
        manifest = directory / "AndroidManifest.xml"
        split_attribute = f' split="{split}"' if split else ""
        manifest.write_text(f'''<manifest xmlns:android="http://schemas.android.com/apk/res/android"
package="org.bridgecoil.fixture.install" android:versionCode="{version}" android:versionName="{version}.0"{split_attribute}>
<uses-sdk android:minSdkVersion="24" android:targetSdkVersion="{target}" />
<application android:hasCode="false" android:label="Bridgecoil install fixture" />
</manifest>''')
        apk = directory / "unsigned.apk"
        run([tools / "aapt2", "link", "-I", sdk / "platforms/android-36/android.jar", "--manifest", manifest, "-o", apk])
        with zipfile.ZipFile(apk, "a") as archive:
            archive.writestr("assets/fixture.txt", b"Owned Bridgecoil fixture. No user data.\n", compress_type=zipfile.ZIP_STORED)
        aligned = directory / "aligned.apk"
        run([tools / "zipalign", "-f", "4", apk, aligned])
        return aligned

    cases = [
        ("v2-rsa.apk", "rsa", False, True, False, 1, 34, None),
        ("v2-ec.apk", "ec", False, True, False, 1, 34, None),
        ("v2-v3.apk", "rsa", False, True, True, 1, 34, None),
        ("v1-only.apk", "rsa", True, False, False, 1, 28, None),
        ("update-v2.apk", "rsa", False, True, False, 2, 34, None),
        ("config-test.apk", "rsa", False, True, False, 1, 34, "config.test"),
        ("other-signer.apk", "other", False, True, False, 1, 34, None),
    ]
    metadata = {}
    for name, key, v1, v2, v3, version, target, split in cases:
        original = unsigned(name.removesuffix(".apk"), version, target, split)
        destination = args.output / name
        run([tools / "apksigner", "sign", "--ks", keys[key], "--ks-key-alias", "fixture",
             "--ks-pass", "pass:" + password, "--key-pass", "pass:" + password,
             "--min-sdk-version", "24", "--v1-signing-enabled", str(v1).lower(),
             "--v2-signing-enabled", str(v2).lower(), "--v3-signing-enabled", str(v3).lower(),
             "--v4-signing-enabled", "false", "--out", destination, original])
        evidence = run([tools / "apksigner", "verify", "--verbose", "--print-certs", "--min-sdk-version", "24", destination])
        match = re.search(r"Signer #1 certificate SHA-256 digest: ([0-9a-fA-F]+)", evidence)
        if not match:
            raise RuntimeError("The SDK verifier did not report a signer fingerprint")
        metadata[name] = {"sha256": hashlib.sha256(destination.read_bytes()).hexdigest(),
                          "signerSha256": match.group(1).lower(), "schemes": [s for s, enabled in [("v1", v1), ("v2", v2), ("v3", v3)] if enabled],
                          "versionCode": version, "split": split, "sdkVerified": True}
    plain = unsigned("unsigned-only")
    shutil.copy2(plain, args.output / "unsigned.apk")
    metadata["unsigned.apk"] = {"sha256": hashlib.sha256(plain.read_bytes()).hexdigest(), "sdkVerified": False}
    tampered = bytearray((args.output / "v2-rsa.apk").read_bytes())
    with zipfile.ZipFile(args.output / "v2-rsa.apk") as archive:
        entry = archive.getinfo("assets/fixture.txt")
        offset = entry.header_offset
        name_length = int.from_bytes(tampered[offset + 26:offset + 28], "little")
        extra_length = int.from_bytes(tampered[offset + 28:offset + 30], "little")
        tampered[offset + 30 + name_length + extra_length] ^= 1
    bad = args.output / "tampered.apk"
    bad.write_bytes(tampered)
    result = subprocess.run([str(tools / "apksigner"), "verify", "--min-sdk-version", "24", str(bad)], capture_output=True, env=env)
    if result.returncode == 0:
        raise RuntimeError("Independent SDK signer unexpectedly accepted tampered APK")
    metadata[bad.name] = {"sha256": hashlib.sha256(tampered).hexdigest(), "sdkVerified": False, "reason": "Asset bytes changed after signing"}
    (args.output / "fixtures.json").write_text(json.dumps({"generator": "Android SDK aapt2/zipalign/apksigner 36.1.0, JDK17", "package": "org.bridgecoil.fixture.install", "files": metadata}, indent=2) + "\n")
    print(f"Generated {len(metadata)} owned fixtures; SDK independently rejects tampered contents. No APK installed.")
