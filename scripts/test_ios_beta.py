from datetime import UTC, datetime, timedelta
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from ios_beta import BUNDLE_ID, BONJOUR_TYPES, TEAM_ID, export_signing_certificate, validate_info, validate_profile


class IOSBetaTests(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "darwin", "Requires macOS codesign")
    def test_extracts_a_real_embedded_signing_certificate(self):
        with tempfile.TemporaryDirectory() as temporary:
            certificate = export_signing_certificate(
                Path("/System/Applications/Calculator.app"), Path(temporary)
            )
        result = subprocess.run(
            ["openssl", "x509", "-inform", "DER", "-noout", "-subject"],
            input=certificate, capture_output=True, check=True,
        )
        self.assertIn(b"Apple", result.stdout)

    def info(self):
        return {
            "CFBundleIdentifier": BUNDLE_ID,
            "CFBundleShortVersionString": "0.1.7",
            "CFBundleVersion": "12.2",
            "NSLocalNetworkUsageDescription": "Connect to your TV.",
            "NSBonjourServices": list(BONJOUR_TYPES),
            "DTPlatformName": "iphoneos",
            "DTSDKName": "iphoneos26.2",
        }

    def profile(self):
        return {
            "TeamIdentifier": [TEAM_ID],
            "ExpirationDate": datetime.now(UTC) + timedelta(days=1),
            "Entitlements": {
                "application-identifier": f"{TEAM_ID}.{BUNDLE_ID}",
                "get-task-allow": False,
                "beta-reports-active": True,
            },
        }

    def test_current_device_metadata_is_accepted(self):
        validate_info(self.info(), "0.1.7", "12.2")
        validate_profile(self.profile())

    def test_old_bundle_simulator_or_wrong_build_cannot_be_uploaded(self):
        for patch in [
            {"CFBundleIdentifier": "nl.neerdael.adbstudio.poc"},
            {"DTPlatformName": "iphonesimulator"},
            {"CFBundleVersion": "12.1"},
            {"CFBundleShortVersionString": "0.1.6"},
            {"DTSDKName": "iphoneos18.5"},
        ]:
            with self.subTest(patch=patch), self.assertRaises(ValueError):
                validate_info(self.info() | patch, "0.1.7", "12.2")

    def test_missing_local_network_declarations_fail(self):
        for patch in [
            {"NSLocalNetworkUsageDescription": ""},
            {"NSBonjourServices": ["_adb._tcp"]},
        ]:
            with self.subTest(patch=patch), self.assertRaises(ValueError):
                validate_info(self.info() | patch, "0.1.7", "12.2")

    def test_development_adhoc_expired_and_other_app_profiles_fail(self):
        profiles = []
        p = self.profile()
        p["Entitlements"]["get-task-allow"] = True
        profiles.append(p)
        p = self.profile()
        p["ProvisionedDevices"] = ["test-device"]
        profiles.append(p)
        p = self.profile()
        p["ExpirationDate"] = datetime.now(UTC) - timedelta(seconds=1)
        profiles.append(p)
        p = self.profile()
        p["Entitlements"]["application-identifier"] = f"{TEAM_ID}.other.app"
        profiles.append(p)
        p = self.profile()
        p["TeamIdentifier"] = ["OTHERTEAM"]
        profiles.append(p)
        p = self.profile()
        p["Entitlements"]["beta-reports-active"] = False
        profiles.append(p)
        for index, profile in enumerate(profiles):
            with self.subTest(profile=index), self.assertRaises(ValueError):
                validate_profile(profile)


if __name__ == "__main__":
    unittest.main()
