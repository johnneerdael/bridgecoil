# iOS builds and TestFlight

Bridgecoil targets iOS 15 and later. A fresh release device archive of version 0.1.7 built with Xcode 26.3 / iOS SDK 26.2 on 31 October 2026. Its bundle ID, version, local-network purpose and three Bonjour declarations passed the artifact check. A build-number override of `1.1` was also built and verified without changing the marketing version.

This establishes current source-build support. Physical-iPhone permission handling, discovery/pairing, Files integration and lifecycle behavior remain unvalidated. A signed export, accepted upload, Apple processing and an installed working beta are separate checks.

## Run the workflow

Open [Actions → iOS beta](https://github.com/johnneerdael/bridgecoil/actions/workflows/ios-beta.yml), choose **Run workflow**, and select a mode:

| Mode | Result | Credentials |
| --- | --- | --- |
| `validate` | Unsigned ARM64 device archive, metadata checks and an archive artifact | None |
| `testflight` | App Store Connect IPA, signature/profile checks and upload to Apple | Apple signing secrets; `main` only |

The workflow runs only when dispatched. A push does not publish a beta or a GitHub release. Run numbers and retry attempts produce a distinct numeric bundle version, such as `12.2`, while `package.json` and Tauri retain the marketing version. Never reuse a bundle version after Apple accepts it. The current format accepts run numbers 1–9999 and attempts 1–99.

The macOS runner uses an iOS 26+ SDK, runs frontend checks/tests and Rust core tests, then builds the app. Build artifacts are retained for seven days, including after an artifact-check failure so the exported package can be inspected. Only a verified exported IPA is eligible for upload to Apple; the `.xcarchive` is build/debug material and may be unsigned or use Tauri's temporary export signature.

## Apple account setup

Use team **6DF93UDRBJ** and bundle ID **nl.neerdael.bridgecoil**. The existing App Store Connect record is **Bridgecoil**, app ID **6829245214**, SKU **bridgecoil**, with iOS and macOS platforms. The current workflow builds iOS. Future macOS delivery can use the same app record.

1. Register the app identifier in Apple Developer, or let Xcode automatic provisioning register it when authorized.
2. Reuse the existing **Bridgecoil app record in App Store Connect**, with primary language English (U.S.) and SKU `bridgecoil`. Xcode provisioning does not create app records. Retain the shared record for future macOS delivery.
3. Create a dedicated Team API key under **Users and Access → Integrations → App Store Connect API → Team Keys**. Tauri's automatic-provisioning guide uses Admin access. Copy the Key ID and Issuer ID, and download the `.p8` file once. Keep it outside Git.
4. Configure the repository secrets below. Automatic signing authenticates with the API key and obtains the app-specific App Store Connect provisioning profile. Existing development/wildcard profiles are not TestFlight profiles.
5. Run `testflight` on `main`. Verify processing in App Store Connect before adding the build to testers. Complete any requested beta information and encryption questionnaire there. Do not declare encryption exempt merely to skip a question.

See [Tauri iOS signing](https://v2.tauri.app/distribute/sign/ios/), [Apple API keys](https://developer.apple.com/help/app-store-connect/get-started/app-store-connect-api/), [uploading builds](https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds/) and [TestFlight](https://developer.apple.com/help/app-store-connect/test-a-beta-version/testflight-overview/).

## Repository secrets

| Secret | Value |
| --- | --- |
| `IOS_CERTIFICATE` | Base64 of the selected Apple Distribution identity exported as password-protected PKCS#12, including its private key |
| `IOS_CERTIFICATE_PASSWORD` | Password chosen for that export |
| `APPLE_API_KEY` | App Store Connect Key ID |
| `APPLE_API_ISSUER` | App Store Connect Issuer ID |
| `APPLE_API_PRIVATE_KEY` | Contents of the downloaded `.p8` file |

The configured certificate fingerprint is `31592EFB272C88A2700DE5BD97EC63198DC92A41`. Review `CERTIFICATE_SHA1` in `scripts/ios_beta.py` when rotating it. The Developer ID Application certificate used for macOS is a different identity.

To export manually, open Keychain Access → My Certificates, select **Apple Distribution: John Neerdael (6DF93UDRBJ)** with its private key, and export only that identity as a `.p12`. Approve macOS's authorization prompt and choose an export password. Keep the file outside the repository. Do not export every identity from the login keychain.

Use GitHub's repository Secrets UI or `gh secret set` with standard input; never put secret values in command arguments, commits, issue text or chat. For example, with the correct repository account active:

```sh
base64 -i /private/path/distribution.p12 | gh secret set IOS_CERTIFICATE --repo johnneerdael/bridgecoil
gh secret set IOS_CERTIFICATE_PASSWORD --repo johnneerdael/bridgecoil
gh secret set APPLE_API_KEY --repo johnneerdael/bridgecoil
gh secret set APPLE_API_ISSUER --repo johnneerdael/bridgecoil
gh secret set APPLE_API_PRIVATE_KEY --repo johnneerdael/bridgecoil < /private/path/AuthKey_KEYID.p8
```

The commands without redirected input prompt for their value. Signing material is imported into an isolated runner keychain; the keychain and temporary certificate/API-key files are removed after every run. Signing secrets are never read by `validate` mode or pull-request workflows.

## Local checks

```sh
python3 -m unittest discover -s scripts -p 'test_ios_beta.py'
actionlint .github/workflows/ios-beta.yml
python3 scripts/ios_beta.py config --build-number 1.1 --output /tmp/bridgecoil-ios-beta.json
npm run ios -- build --target aarch64 --no-sign --archive-only --ci --config /tmp/bridgecoil-ios-beta.json
python3 scripts/ios_beta.py check-archive src-tauri/gen/apple/build/adb-studio-app_iOS.xcarchive --version 0.1.7 --build-number 1.1
```

Use the current marketing version in the artifact check. `check-ipa` additionally verifies the Apple Distribution certificate, bundle/team profile, expiration, App Store beta entitlement and absence of development/ad-hoc/enterprise provisioning.

## Physical-device acceptance

Record the iPhone/iPad model, iOS version, installed build and target TV firmware for each run:

- [ ] Install from TestFlight and cold-launch without a crash.
- [ ] Grant LAN access, discover the TV, pair with its code, connect and confirm the selected target/profile.
- [ ] Deny LAN access in a separate run; verify actionable recovery and retry after enabling permission in Settings.
- [ ] Inspect apps, perform an explicitly reviewed reversible change, verify readback and recover it.
- [ ] Pick an APK and an upload from local Files and iCloud Drive; test cancellation, provider access and connection/session changes.
- [ ] Save a downloaded TV file, screenshot and recording to Files; reopen the saved result and check its contents.
- [ ] Background the controller during normal use and a picker flow; return, reconnect if necessary and check uncertain operation state before retrying.
- [ ] Exercise remote controls, mapping setup and per-operation History against the TV. Test TV restart persistence independently.
- [ ] Check iPhone/iPad layout, rotation, large text, keyboard focus and light/dark native chrome.

Until these are observed, describe iOS as build-supported with physical acceptance pending. TestFlight does not remove iOS background-execution limits. Ordinary distribution uses Apple signing/provisioning and App Store Connect; Apple's separate iOS notarization route serves alternative distribution.
