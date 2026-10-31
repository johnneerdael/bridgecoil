#!/usr/bin/env python3
"""Prepare and verify Bridgecoil iOS artifacts without printing signing secrets."""
import argparse
import base64
from datetime import UTC, datetime
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
BUNDLE_ID = "nl.neerdael.bridgecoil"
TEAM_ID = "6DF93UDRBJ"
CERTIFICATE_SHA1 = "31592EFB272C88A2700DE5BD97EC63198DC92A41"
BONJOUR_TYPES = {"_adb-tls-connect._tcp", "_adb-tls-pairing._tcp", "_adb._tcp"}


def validate_info(info, version, build_number):
    expected = {
        "CFBundleIdentifier": BUNDLE_ID,
        "CFBundleShortVersionString": version,
        "CFBundleVersion": build_number,
        "DTPlatformName": "iphoneos",
    }
    for key, value in expected.items():
        if info.get(key) != value:
            raise ValueError(f"{key} must be {value!r}; got {info.get(key)!r}")
    if not str(info.get("NSLocalNetworkUsageDescription", "")).strip():
        raise ValueError("The built app needs its local-network usage description")
    if not BONJOUR_TYPES.issubset(info.get("NSBonjourServices", [])):
        raise ValueError("The built app is missing required ADB Bonjour types")
    sdk = re.fullmatch(r"iphoneos(\d+)(?:\.\d+)*", info.get("DTSDKName", ""))
    if not sdk or int(sdk[1]) < 26:
        raise ValueError("App Store Connect requires an iOS 26 or newer SDK")


def validate_profile(profile):
    entitlements = profile.get("Entitlements", {})
    if TEAM_ID not in profile.get("TeamIdentifier", []):
        raise ValueError("Provisioning profile belongs to a different team")
    if entitlements.get("application-identifier") != f"{TEAM_ID}.{BUNDLE_ID}":
        raise ValueError("Provisioning profile belongs to a different app")
    if entitlements.get("get-task-allow") is not False:
        raise ValueError("Development provisioning is not a TestFlight distribution")
    if entitlements.get("beta-reports-active") is not True:
        raise ValueError("An App Store Connect provisioning profile is required")
    if "ProvisionedDevices" in profile or profile.get("ProvisionsAllDevices"):
        raise ValueError("Ad hoc and enterprise profiles are not TestFlight profiles")
    expiration = profile.get("ExpirationDate")
    if not isinstance(expiration, datetime):
        raise ValueError("Provisioning profile has no expiration date")
    if expiration.replace(tzinfo=UTC) <= datetime.now(UTC):
        raise ValueError("Provisioning profile has expired")


def one_app(directory):
    apps = list(directory.glob("*.app"))
    if len(apps) != 1:
        raise ValueError(f"Expected exactly one application in {directory}")
    return apps[0]


def check_app(app, version, build_number):
    validate_info(plistlib.loads((app / "Info.plist").read_bytes()), version, build_number)


def prepare_config(build_number, output):
    if not re.fullmatch(r"[1-9]\d{0,3}\.[1-9]\d?", build_number):
        raise ValueError("Use workflow run number (1–9999) and attempt (1–99), e.g. 12.2")
    config = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text())
    version = config["version"]
    package = json.loads((ROOT / "package.json").read_text())
    rust = tomllib.loads((ROOT / "src-tauri/Cargo.toml").read_text())
    if config["identifier"] != BUNDLE_ID:
        raise ValueError("Tauri identifier changed; review signing configuration")
    if package["version"] != version or rust["package"]["version"] != version:
        raise ValueError("Application version files do not agree")
    output.write_text(json.dumps({"bundle": {"iOS": {
        "developmentTeam": TEAM_ID, "bundleVersion": build_number,
    }}}))
    print(f"Prepared {BUNDLE_ID} version {version}, build {build_number}")


def stage_signing_files():
    names = ["IOS_CERTIFICATE", "IOS_CERTIFICATE_PASSWORD", "APPLE_API_PRIVATE_KEY", "APPLE_API_KEY", "APPLE_API_ISSUER"]
    missing = [name for name in names if not os.environ.get(name)]
    if missing:
        raise ValueError("Missing GitHub secrets: " + ", ".join(missing))
    key_id = os.environ["APPLE_API_KEY"]
    if not re.fullmatch(r"[A-Z0-9]{10}", key_id):
        raise ValueError("APPLE_API_KEY must be the ten-character Key ID")
    directory = Path(os.environ["RUNNER_TEMP"]) / "bridgecoil-signing"
    directory.mkdir(mode=0o700, exist_ok=True)
    certificate = directory / "distribution.p12"
    certificate.write_bytes(base64.b64decode(os.environ["IOS_CERTIFICATE"], validate=True))
    key = directory / f"AuthKey_{key_id}.p8"
    key.write_text(os.environ["APPLE_API_PRIVATE_KEY"])
    certificate.chmod(0o600)
    key.chmod(0o600)
    with Path(os.environ["GITHUB_ENV"]).open("a") as env:
        env.write(f"APPLE_API_KEY_PATH={key}\nAPI_PRIVATE_KEYS_DIR={directory}\n")
    print("Prepared temporary signing files; secret values are not logged")


def verify_ipa(ipa, version, build_number):
    with tempfile.TemporaryDirectory(prefix="bridgecoil-ipa-") as temporary:
        directory = Path(temporary)
        subprocess.run(["ditto", "-x", "-k", str(ipa), str(directory)], check=True)
        app = one_app(directory / "Payload")
        check_app(app, version, build_number)
        subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
        prefix = directory / "signer"
        subprocess.run(["codesign", "-d", "--extract-certificates", str(prefix), str(app)], check=True)
        signer = hashlib.sha1((directory / "signer0").read_bytes()).hexdigest().upper()
        if signer != CERTIFICATE_SHA1:
            raise ValueError("IPA was not signed by the configured Apple Distribution certificate")
        result = subprocess.run(["security", "cms", "-D", "-i", str(app / "embedded.mobileprovision")], capture_output=True, check=True)
        validate_profile(plistlib.loads(result.stdout))
    print(f"Verified distribution IPA: {BUNDLE_ID}, version {version}, build {build_number}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    config = commands.add_parser("config")
    config.add_argument("--build-number", required=True)
    config.add_argument("--output", type=Path, required=True)
    commands.add_parser("signing-files")
    for command in ["check-archive", "check-ipa"]:
        check = commands.add_parser(command)
        check.add_argument("path", type=Path)
        check.add_argument("--version", required=True)
        check.add_argument("--build-number", required=True)
    args = parser.parse_args()
    try:
        if args.command == "config":
            prepare_config(args.build_number, args.output)
        elif args.command == "signing-files":
            stage_signing_files()
        elif args.command == "check-archive":
            check_app(one_app(args.path / "Products/Applications"), args.version, args.build_number)
            print("Verified device archive metadata; this does not establish physical-iPhone compatibility")
        elif args.command == "check-ipa":
            verify_ipa(args.path, args.version, args.build_number)
    except (ValueError, KeyError) as error:
        raise SystemExit(str(error)) from error


if __name__ == "__main__":
    main()
