#!/usr/bin/env python3
"""Build the small agent that runs in Termux, for every CPU a TV can have."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

root = Path(__file__).resolve().parents[1]
targets = {
    "armeabi-v7a": ("armv7-linux-androideabi", "armv7a-linux-androideabi24-clang"),
    "arm64-v8a": ("aarch64-linux-android", "aarch64-linux-android24-clang"),
    "x86_64": ("x86_64-linux-android", "x86_64-linux-android24-clang"),
}
parser = argparse.ArgumentParser()
parser.add_argument("--ndk", default=os.environ.get("NDK_HOME"))
args = parser.parse_args()
if not args.ndk:
    parser.error("Set NDK_HOME or pass --ndk")
host = "darwin-x86_64" if platform.system() == "Darwin" else "linux-x86_64"
tools = Path(args.ndk) / "toolchains/llvm/prebuilt" / host / "bin"
out = root / "crates/adb-core/assets/agent"
sources = sorted((root / "crates/tv-agent").rglob("*"))
source = hashlib.sha256()
for path in [p for p in sources if p.is_file()] + [root / "Cargo.lock"]:
    source.update(path.read_bytes())
digests = {}
for abi, (target, clang) in targets.items():
    key = target.upper().replace("-", "_")
    env = {
        **os.environ,
        f"CC_{target.replace('-', '_')}": str(tools / clang),
        f"AR_{target.replace('-', '_')}": str(tools / "llvm-ar"),
        f"CARGO_TARGET_{key}_LINKER": str(tools / clang),
        "CARGO_PROFILE_RELEASE_OPT_LEVEL": "z",
        "CARGO_PROFILE_RELEASE_LTO": "fat",
        "CARGO_PROFILE_RELEASE_STRIP": "symbols",
        "CARGO_PROFILE_RELEASE_PANIC": "abort",
        "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "1",
    }
    command = ["node", "scripts/cargo.mjs", "build", "--locked", "-p", "tv-agent", "--release", "--target", target]
    subprocess.run(command, check=True, cwd=root, env=env)
    (out / abi).mkdir(parents=True, exist_ok=True)
    built = root / "target" / target / "release/bridgecoil-agent"
    shutil.copyfile(built, out / abi / "bridgecoil-agent")
    digests[abi] = hashlib.sha256(built.read_bytes()).hexdigest()
manifest = {"sourceSha256": source.hexdigest(), "binaries": digests}
(out / "agent-build.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps(manifest, indent=2))
