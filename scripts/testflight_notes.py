#!/usr/bin/env python3
"""Publish versioned What to Test text to one exact processed TestFlight build.

Uses macOS altool for ES256 signing and system curl for trusted HTTPS. Tokens,
private keys and raw authentication/HTTP responses must never reach the logs.
"""
import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
from urllib.parse import urlencode, urlsplit

APP_ID = "6829245214"
BUNDLE_ID = "nl.neerdael.bridgecoil"
LOCALE = "en-US"
ORIGIN = "https://api.appstoreconnect.apple.com"
NOTES_FILE = Path(__file__).resolve().parent.parent / "docs/testflight/what-to-test.en-US.txt"


def render_notes(version: str, build_number: str, testing_notes: str, focus: str = "") -> str:
    for name, value in [("version", version), ("build number", build_number)]:
        if re.fullmatch(r"[0-9]+(?:\.[0-9]+){0,2}", value) is None:
            raise ValueError(f"Supply an explicit numeric {name} with at most three components")
    if not testing_notes.strip():
        raise ValueError("What to Test source text is empty")
    sections = [f"Bridgecoil {version} ({build_number})"]
    if focus.strip():
        sections.append(f"Release focus:\n{focus.strip()}")
    sections.append(f"What to test:\n{testing_notes.strip()}")
    rendered = "\n\n".join(sections)
    if len(rendered) > 4000:
        raise ValueError("Rendered What to Test exceeds Apple's 4000-character limit")
    return rendered


@contextmanager
def api_key_file():
    existing = os.environ.get("APPLE_API_KEY_PATH")
    if existing:
        if not Path(existing).is_file():
            raise ValueError("APPLE_API_KEY_PATH must identify a private key file")
        yield existing
        return
    private_key = os.environ.get("APPLE_API_PRIVATE_KEY", "")
    if not private_key.strip():
        raise ValueError("Set APPLE_API_KEY_PATH or APPLE_API_PRIVATE_KEY")
    with tempfile.TemporaryDirectory(prefix="bridgecoil-testflight-", dir=os.environ.get("RUNNER_TEMP")) as directory:
        key_path = Path(directory) / "AuthKey.p8"
        descriptor = os.open(key_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "w") as key_file:
            key_file.write(private_key)
        yield str(key_path)


def generate_token(key_path: str) -> str:
    key_id = os.environ.get("APPLE_API_KEY", "")
    issuer = os.environ.get("APPLE_API_ISSUER", "")
    if not key_id or not issuer:
        raise ValueError("Set APPLE_API_KEY and APPLE_API_ISSUER")
    try:
        result = subprocess.run(
            ["xcrun", "altool", "--generate-jwt", "--apiKey", key_id,
             "--apiIssuer", issuer, "--p8-file-path", key_path, "--output-format", "json"],
            capture_output=True, text=True, timeout=60, check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        raise ValueError("Apple JWT generation failed; check Xcode and API credentials") from None
    # altool 26 emits the token on stderr, even with --output-format json.
    tokens = re.findall(r"(?<![\w.-])[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+(?![\w.-])", result.stdout + "\n" + result.stderr)
    if result.returncode or len(tokens) != 1:
        raise ValueError("Apple JWT generation failed; check Xcode and API credentials")
    return tokens[0]


def config_string(value: str) -> str:
    """Quote one curl config value, including text supplied as plain workflow data."""
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"').replace("\r", "\\r").replace("\n", "\\n").replace("\t", "\\t") + '"'


class APIError(ValueError):
    def __init__(self, status: int):
        self.status = status
        super().__init__(f"App Store Connect HTTP {status}; verify API access and retry notes mode")


class AppleAPI:
    def __init__(self, token_provider):
        self.token_provider = token_provider

    def request(self, method: str, path: str, payload: dict | None = None) -> dict:
        url = ORIGIN + path if path.startswith("/") else path
        parsed = urlsplit(url)
        if parsed.scheme != "https" or parsed.netloc != "api.appstoreconnect.apple.com" or parsed.fragment:
            raise ValueError("Refusing an API URL outside Apple's HTTPS origin")
        if method not in {"GET", "POST", "PATCH"}:
            raise ValueError("Unsupported API method")
        config = [f"url = {config_string(url)}", f"request = {config_string(method)}",
                  f"header = {config_string('Authorization: Bearer ' + self.token_provider())}",
                  'header = "Content-Type: application/json"']
        if payload is not None:
            config.append(f"data = {config_string(json.dumps(payload))}")
        try:
            # No redirects, verbose output, credentials in argv, or TLS exceptions.
            result = subprocess.run(
                ["curl", "--disable", "--silent", "--show-error", "--globoff",
                 "--connect-timeout", "15", "--max-time", "60", "--proto", "=https",
                 "--write-out", "\n%{http_code}", "--config", "-"],
                input="\n".join(config) + "\n", capture_output=True, text=True, timeout=65, check=False,
            )
        except (OSError, subprocess.TimeoutExpired):
            raise ValueError("App Store Connect transport failed; retry notes mode") from None
        if result.returncode:
            raise ValueError("App Store Connect transport failed; retry notes mode")
        body, _, status_text = result.stdout.rpartition("\n")
        if not status_text.isdigit():
            raise ValueError("App Store Connect returned an invalid HTTP response")
        status = int(status_text)
        if not 200 <= status < 300:
            raise APIError(status)
        try:
            return json.loads(body)
        except json.JSONDecodeError:
            raise ValueError("App Store Connect returned invalid JSON") from None


def collection(api: AppleAPI, path: str, deadline: float | None = None) -> dict:
    data, included, seen = [], [], set()
    while path:
        if path in seen or len(seen) >= 100:
            raise ValueError("App Store Connect pagination exceeded its safety bound")
        if deadline is not None and time.monotonic() >= deadline:
            raise ValueError("Timed out waiting for the exact TestFlight build; retry notes mode")
        seen.add(path)
        page = api.request("GET", path)
        data.extend(page["data"])
        included.extend(page.get("included", []))
        path = page.get("links", {}).get("next")
    return {"data": data, "included": included}


def select_build(page: dict, version: str, build_number: str) -> dict | None:
    releases = {item["id"]: item for item in page.get("included", []) if item["type"] == "preReleaseVersions"}
    matches = []
    for build in page["data"]:
        relations = build.get("relationships", {})
        app = relations.get("app", {}).get("data") or {}
        release_id = (relations.get("preReleaseVersion", {}).get("data") or {}).get("id")
        release = releases.get(release_id, {}).get("attributes", {})
        if (build["attributes"].get("version") == build_number and app.get("id") == APP_ID
                and release.get("version") == version and release.get("platform") == "IOS"):
            matches.append(build)
    if len(matches) > 1:
        raise ValueError("Multiple builds match the exact version, build number, app and platform")
    return matches[0] if matches else None


def wait_for_build(api: AppleAPI, version: str, build_number: str, timeout: int = 1200) -> dict:
    deadline = time.monotonic() + timeout
    app = api.request("GET", f"/v1/apps/{APP_ID}")["data"]
    if app.get("id") != APP_ID or app.get("attributes", {}).get("bundleId") != BUNDLE_ID:
        raise ValueError("App Store Connect app ID/bundle ID does not match Bridgecoil")
    # The app relationship endpoint cannot include release/app linkage. Query the
    # build collection instead, with supported filters and included relationships.
    query = urlencode({"filter[app]": APP_ID, "filter[version]": build_number,
                       "include": "preReleaseVersion,app", "limit": "200"})
    path = f"/v1/builds?{query}"
    while time.monotonic() < deadline:
        try:
            page = collection(api, path, deadline)
        except APIError as error:
            if error.status != 429 and error.status < 500:
                raise
            page = {"data": []}
        build = select_build(page, version, build_number)
        if build:
            state = build["attributes"].get("processingState")
            if state in {"FAILED", "INVALID"} or build["attributes"].get("expired"):
                raise ValueError(f"Exact TestFlight build is unusable ({state} or expired)")
            if state == "VALID":
                return build
        print("Waiting for the requested TestFlight build to finish processing…", flush=True)
        time.sleep(min(30, max(0, deadline - time.monotonic())))
    raise ValueError("Timed out waiting for the exact TestFlight build; retry notes mode")


def publish_notes(api: AppleAPI, build_id: str, text: str) -> str:
    localizations = collection(api, f"/v1/builds/{build_id}/betaBuildLocalizations")["data"]
    matches = [item for item in localizations if item["attributes"].get("locale") == LOCALE]
    if len(matches) > 1:
        raise ValueError("Multiple en-US beta build localizations; inspect App Store Connect")
    if matches:
        localization_id = matches[0]["id"]
        action = "unchanged"
        if matches[0]["attributes"].get("whatsNew") != text:
            api.request("PATCH", f"/v1/betaBuildLocalizations/{localization_id}", {"data": {
                "type": "betaBuildLocalizations", "id": localization_id,
                "attributes": {"whatsNew": text},
            }})
            action = "updated"
    else:
        created = api.request("POST", "/v1/betaBuildLocalizations", {"data": {
            "type": "betaBuildLocalizations", "attributes": {"locale": LOCALE, "whatsNew": text},
            "relationships": {"build": {"data": {"type": "builds", "id": build_id}}},
        }})
        localization_id = created["data"]["id"]
        action = "created"
    saved = api.request("GET", f"/v1/betaBuildLocalizations/{localization_id}")["data"]["attributes"]
    if saved.get("locale") != LOCALE or saved.get("whatsNew") != text:
        raise ValueError("What to Test readback differs from the requested notes; retry notes mode")
    return action


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["render", "publish"])
    parser.add_argument("--version", required=True)
    parser.add_argument("--build-number", required=True)
    parser.add_argument("--notes-file", type=Path, default=NOTES_FILE)
    parser.add_argument("--timeout", type=int, default=1200, help="Build-processing poll limit in seconds (1–1800)")
    args = parser.parse_args()
    try:
        text = render_notes(args.version, args.build_number, args.notes_file.read_text(), os.environ.get("TESTFLIGHT_RELEASE_FOCUS", ""))
        if args.command == "render":
            print(text)
            return 0
        if os.environ.get("GITHUB_REF") != "refs/heads/main":
            raise ValueError("App Store Connect API writes are restricted to refs/heads/main")
        if not 1 <= args.timeout <= 1800:
            raise ValueError("Poll timeout must be between 1 and 1800 seconds")
        with api_key_file() as key_path:
            api = AppleAPI(lambda: generate_token(key_path))
            build = wait_for_build(api, args.version, args.build_number, args.timeout)
            action = publish_notes(api, build["id"], text)
        message = f"What to Test {action} and verified for Bridgecoil {args.version} ({args.build_number}), {LOCALE}."
        print(message)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
                summary.write(message + "\n")
        return 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        # Report only our controlled ValueError messages. Raw subprocess output and
        # JSON/OS exceptions can contain credential material or request data.
        if isinstance(error, ValueError) and not isinstance(error, json.JSONDecodeError):
            print(f"TestFlight notes failed: {error}", file=sys.stderr)
        else:
            print("TestFlight notes failed: check source files and the API response schema", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
