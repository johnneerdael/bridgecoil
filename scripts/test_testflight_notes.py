"""Behavior checks for build-specific TestFlight notes; no Apple credentials needed."""
import copy
import io
from pathlib import Path
from contextlib import redirect_stderr
import subprocess
import unittest
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit

import testflight_notes as notes


def build_page():
    return {
        "data": [{"type": "builds", "id": "target", "attributes": {
            "version": "3.1", "processingState": "VALID", "expired": False,
        }, "relationships": {
            "app": {"data": {"type": "apps", "id": notes.APP_ID}},
            "preReleaseVersion": {"data": {"type": "preReleaseVersions", "id": "release"}},
        }}],
        "included": [{"type": "preReleaseVersions", "id": "release", "attributes": {
            "version": "0.1.7", "platform": "IOS",
        }}],
    }


class MemoryAPI:
    def __init__(self, saved=None):
        self.saved = saved
        self.writes = []
        self.gets = []

    def request(self, method, path, payload=None):
        if method == "GET":
            self.gets.append(path)
            if path.startswith("/v1/apps/") and not "/builds" in path:
                return {"data": {"id": notes.APP_ID, "attributes": {"bundleId": notes.BUNDLE_ID}}}
            if urlsplit(path).path == "/v1/builds":
                query = parse_qs(urlsplit(path).query)
                expected = {"filter[app]": [notes.APP_ID], "filter[version]": ["3.1"],
                            "include": ["preReleaseVersion,app"], "limit": ["200"]}
                if query != expected:
                    raise AssertionError(f"Build query does not match Apple's supported contract: {query}")
                return build_page()
            if path.endswith("/betaBuildLocalizations"):
                return {"data": [] if self.saved is None else [self.saved]}
            if path == "/v1/betaBuildLocalizations/localization":
                return {"data": self.saved}
            raise AssertionError(path)
        self.writes.append((method, path, payload))
        if method == "PATCH":
            self.saved["attributes"].update(payload["data"]["attributes"])
        else:
            self.saved = dict(payload["data"], id="localization")
        return {"data": self.saved}


class NotesTests(unittest.TestCase):
    def test_render_has_exact_build_header_and_optional_focus(self):
        text = notes.render_notes("0.1.7", "3.1", "Test pairing.", "Check reconnection.")
        self.assertEqual(text, "Bridgecoil 0.1.7 (3.1)\n\nRelease focus:\nCheck reconnection.\n\nWhat to test:\nTest pairing.")
        self.assertNotIn("Release focus", notes.render_notes("0.1.7", "3.1", "Test pairing."))

    def test_render_rejects_empty_oversize_and_malformed_versions(self):
        for body in ["", " \n", "x" * 4000]:
            with self.subTest(body_length=len(body)), self.assertRaises(ValueError):
                notes.render_notes("0.1.7", "3.1", body)
        for version, build in [("", "3.1"), ("0.1.7\n", "3.1"), ("0.1.7", "latest")]:
            with self.subTest(version=version, build=build), self.assertRaises(ValueError):
                notes.render_notes(version, build, "Test pairing.")
        prefix = len(notes.render_notes("0.1.7", "3.1", "x")) - 1
        self.assertEqual(len(notes.render_notes("0.1.7", "3.1", "x" * (4000 - prefix))), 4000)

    def test_select_requires_build_marketing_version_platform_and_app(self):
        page = build_page()
        self.assertEqual(notes.select_build(page, "0.1.7", "3.1")["id"], "target")
        variants = []
        for target, field, value in [("build", "version", "3.2"), ("release", "version", "0.1.6"), ("release", "platform", "MAC_OS")]:
            variant = copy.deepcopy(page)
            resource = variant["data"][0] if target == "build" else variant["included"][0]
            resource["attributes"][field] = value
            variants.append(variant)
        variant = copy.deepcopy(page)
        variant["data"][0]["relationships"]["app"]["data"]["id"] = "another-app"
        variants.append(variant)
        variant = copy.deepcopy(page)
        variant["included"] = []
        variants.append(variant)
        for variant in variants:
            with self.subTest(variant=variant):
                self.assertIsNone(notes.select_build(variant, "0.1.7", "3.1"))
        page["data"].append(copy.deepcopy(page["data"][0]) | {"id": "duplicate"})
        with self.assertRaises(ValueError):
            notes.select_build(page, "0.1.7", "3.1")

    def test_create_update_idempotence_and_readback(self):
        api = MemoryAPI()
        self.assertEqual(notes.publish_notes(api, "target", "First notes"), "created")
        method, path, payload = api.writes[-1]
        self.assertEqual((method, path), ("POST", "/v1/betaBuildLocalizations"))
        self.assertEqual(payload["data"]["attributes"], {"locale": "en-US", "whatsNew": "First notes"})
        self.assertEqual(payload["data"]["relationships"]["build"]["data"]["id"], "target")
        self.assertEqual(api.gets[-1], "/v1/betaBuildLocalizations/localization")
        self.assertEqual(notes.publish_notes(api, "target", "Updated notes"), "updated")
        self.assertEqual(api.writes[-1][0:2], ("PATCH", "/v1/betaBuildLocalizations/localization"))
        self.assertEqual(notes.publish_notes(api, "target", "Updated notes"), "unchanged")
        self.assertEqual(len(api.writes), 2)
        self.assertEqual(api.gets[-1], "/v1/betaBuildLocalizations/localization")

    def test_readback_mismatch_fails(self):
        api = MemoryAPI()
        request = api.request
        def wrong_readback(method, path, payload=None):
            result = request(method, path, payload)
            if path == "/v1/betaBuildLocalizations/localization":
                result = copy.deepcopy(result)
                result["data"]["attributes"]["whatsNew"] = "stale"
            return result
        api.request = wrong_readback
        with self.assertRaisesRegex(ValueError, "readback"):
            notes.publish_notes(api, "target", "Requested notes")

    def test_wait_checks_bundle_id_and_exact_processing_state(self):
        api = MemoryAPI()
        self.assertEqual(notes.wait_for_build(api, "0.1.7", "3.1", timeout=5)["id"], "target")
        build_queries = [urlsplit(path) for path in api.gets if urlsplit(path).path == "/v1/builds"]
        self.assertEqual(len(build_queries), 1)
        self.assertEqual(parse_qs(build_queries[0].query), {
            "filter[app]": [notes.APP_ID], "filter[version]": ["3.1"],
            "include": ["preReleaseVersion,app"], "limit": ["200"],
        })
        with self.assertRaises(AssertionError):
            api.request("GET", f"/v1/apps/{notes.APP_ID}/builds?include=preReleaseVersion,app")
        request = api.request
        def wrong_app(method, path, payload=None):
            result = request(method, path, payload)
            if path == f"/v1/apps/{notes.APP_ID}":
                result["data"]["attributes"]["bundleId"] = "wrong.bundle"
            return result
        api.request = wrong_app
        with self.assertRaisesRegex(ValueError, "bundle"):
            notes.wait_for_build(api, "0.1.7", "3.1", timeout=5)

    def test_polling_times_out_and_rejects_failed_build(self):
        api = MemoryAPI()
        request = api.request
        def processing(method, path, payload=None):
            result = request(method, path, payload)
            if "/builds?" in path:
                result["data"][0]["attributes"]["processingState"] = "PROCESSING"
            return result
        api.request = processing
        with patch.object(notes.time, "monotonic", side_effect=[0, 0, 0, 6, 6]), patch.object(notes.time, "sleep"):
            with self.assertRaisesRegex(ValueError, "Timed out"):
                notes.wait_for_build(api, "0.1.7", "3.1", timeout=5)
        def failed(method, path, payload=None):
            result = processing(method, path, payload)
            if "/builds?" in path:
                result["data"][0]["attributes"]["processingState"] = "INVALID"
            return result
        api.request = failed
        with self.assertRaisesRegex(ValueError, "INVALID"):
            notes.wait_for_build(api, "0.1.7", "3.1", timeout=5)

    def test_paginated_builds_do_not_choose_an_unrelated_latest_build(self):
        api = MemoryAPI()
        request = api.request
        def paginated(method, path, payload=None):
            if path == notes.ORIGIN + "/v1/builds?cursor=page2":
                return build_page()
            result = request(method, path, payload)
            if urlsplit(path).path == "/v1/builds":
                result["data"][0]["attributes"]["version"] = "99.1"
                result["links"] = {"next": notes.ORIGIN + "/v1/builds?cursor=page2"}
            return result
        api.request = paginated
        self.assertEqual(notes.wait_for_build(api, "0.1.7", "3.1", timeout=5)["id"], "target")

    def test_publish_command_refuses_feature_branches_before_loading_credentials(self):
        stderr = io.StringIO()
        args = ["testflight_notes.py", "publish", "--version", "0.1.7", "--build-number", "3.1"]
        with patch.object(notes.sys, "argv", args), patch.dict(notes.os.environ, {"GITHUB_REF": "refs/heads/feature"}, clear=True):
            with patch.object(notes, "api_key_file") as key_file, redirect_stderr(stderr):
                self.assertEqual(notes.main(), 1)
                key_file.assert_not_called()
        self.assertIn("restricted to refs/heads/main", stderr.getvalue())

    def test_curl_secrets_are_stdin_only_and_errors_are_sanitized(self):
        api = notes.AppleAPI(lambda: "header.payload.signature")
        with patch.object(notes.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, '{"data": []}\n200', "")) as run:
            self.assertEqual(api.request("GET", "/v1/apps"), {"data": []})
            args, kwargs = run.call_args
            self.assertNotIn("header.payload.signature", str(args))
            self.assertIn("Authorization: Bearer header.payload.signature", kwargs["input"])
            self.assertNotIn("--insecure", args[0])
            self.assertEqual(args[0][1], "--disable")
        with self.assertRaises(ValueError):
            api.request("GET", "https://attacker.invalid/v1/apps")
        with patch.object(notes.subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "header.payload.signature", "PRIVATE KEY")):
            with self.assertRaisesRegex(ValueError, "transport failed") as caught:
                api.request("GET", "/v1/apps")
            self.assertNotIn("signature", str(caught.exception))
            self.assertNotIn("PRIVATE KEY", str(caught.exception))

    def test_altool_token_capture_and_private_key_cleanup(self):
        env = {"APPLE_API_KEY": "KEYID", "APPLE_API_ISSUER": "ISSUER", "APPLE_API_PRIVATE_KEY": "private-key-content"}
        result = subprocess.CompletedProcess([], 0, "", "Running altool...\nheader.payload.signature\n")
        with patch.dict(notes.os.environ, env, clear=True), patch.object(notes.subprocess, "run", return_value=result) as run:
            with notes.api_key_file() as key_path:
                self.assertEqual(Path(key_path).read_text(), "private-key-content")
                self.assertEqual(Path(key_path).stat().st_mode & 0o777, 0o600)
                self.assertEqual(notes.generate_token(key_path), "header.payload.signature")
                self.assertNotIn("private-key-content", str(run.call_args))
            self.assertFalse(Path(key_path).exists())
        with patch.dict(notes.os.environ, env, clear=True), patch.object(notes.subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "private-key-content", "header.payload.signature")):
            with self.assertRaisesRegex(ValueError, "JWT generation failed") as caught:
                notes.generate_token("/unused")
            self.assertNotIn("private-key-content", str(caught.exception))
            self.assertNotIn("header.payload.signature", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
