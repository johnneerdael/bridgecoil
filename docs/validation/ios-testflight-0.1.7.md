# iOS TestFlight delivery — 0.1.7 (3.1)

31 October 2026. [Signed workflow run 41218436962](https://github.com/johnneerdael/bridgecoil/actions/runs/41218436962), source `60e8808d67b0322a6634ad93841c7322a6535f66`.

## Application identity

- Product: Bridgecoil; bundle ID `nl.neerdael.bridgecoil`; team `6DF93UDRBJ`.
- App Store Connect record `6829245214`, SKU `bridgecoil`, English (U.S.), with iOS and macOS entries.
- This delivery is an ARM64 iOS IPA, version 0.1.7, bundle build number 3.1. The macOS entry is reserved for future delivery.

## Observed checks

| Boundary | Result |
| --- | --- |
| Frontend type checks and tests | Passed in the signed workflow |
| Rust core tests | Passed; live/device tests remain separately gated |
| iOS metadata/profile checks | Five tests passed, including real certificate extraction from a signed macOS system app |
| Native build | Xcode 26.6 / iOS 26.5 SDK; deployment target iOS 15 |
| Built bundle | Exact bundle ID, marketing/build versions, LAN purpose and all three ADB Bonjour declarations verified |
| Exported IPA | Code signature verified; signing leaf matched configured Apple Distribution certificate `31592EFB272C88A2700DE5BD97EC63198DC92A41` |
| Embedded provisioning | Correct app/team, unexpired App Store beta profile, development/ad-hoc/enterprise provisioning rejected |
| Apple validation | `VERIFY SUCCEEDED with no errors` |
| Apple upload | `UPLOAD SUCCEEDED with no errors`; delivery `3d682772-5b56-428d-93a8-4ec2f8e54ea2` |
| Apple processing | `VALID`, confirmed using altool delivery status and the App Store Connect app/builds relationship |
| Initial TestFlight availability | `MISSING_EXPORT_COMPLIANCE` for internal and external testing; not yet install-ready |
| Current TestFlight status | `Waiting for Review`, observed in App Store Connect on 31 October 2026 after the owner submitted the build; associated with `Bridgecoil External Beta` |
| Beta information | Description, feedback email, marketing/privacy URLs, review notes and private review contact saved and checked after reload; sign-in not required |
| What to Test | Version/build-specific notes published and read back exactly by [run 41224939045](https://github.com/johnneerdael/bridgecoil/actions/runs/41224939045), without rebuilding or uploading another IPA |

## Remaining acceptance

The owner selected invitation-only external testing outside France. The app bundles `libadb` TLS/pairing and `ureq` with Rustls, so its implementation is not limited to Apple's operating-system crypto APIs. No exemption flag was inserted simply to bypass the questionnaire. The initial compliance block is no longer shown; the build progressed through `Ready to Submit` to `Waiting for Review`. This observation does not establish eligibility for a different distribution scope.

**Bridgecoil Internal** and **Bridgecoil External Beta** groups were created. Internal automatic distribution was disabled at creation. The external review wizard was prepared for 3.1 with the saved notes loaded, sign-in not required and automatic tester notification disabled. The owner then submitted the build; App Store Connect confirms `Waiting for Review` and associates it with **Bridgecoil External Beta**. No invitations or public link were enabled by the preparation automation. Review contacts remain private in App Store Connect. Apple beta-review approval and tester access have not been verified.

No physical iPhone/iPad installation, LAN prompt, pairing, APK/file picker, capture/export or background/reconnect acceptance was performed. Follow [the physical-device checklist](../ios-beta.md#physical-device-acceptance).

The Tauri archive uses a temporary signing identity during export. Validation applies to the final IPA; the archive is not evidence of the final distribution signer. An initial certificate-extraction command parsing fault was reproduced on a real signed macOS app and repaired before the successful run. Existing signing/profile requirements were retained.

## Credentials

The five signing/API credentials are encrypted repository secrets. Runner signing files/keychains were cleaned after the run. Private keys and passwords were not committed or included in artifacts. The local Apple credential-metadata file is ignored by Git.
