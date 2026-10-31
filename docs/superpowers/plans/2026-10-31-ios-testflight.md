# Bridgecoil iOS TestFlight delivery

User selected TestFlight beta builds. Use the existing Apple Distribution identity for team `6DF93UDRBJ`; retain bundle ID `nl.neerdael.bridgecoil` and current marketing version. Do not treat an unsigned archive, signed IPA or upload as proof of physical-iPhone compatibility.

## Implementation

1. Add a manual macOS/Xcode 26+ workflow with unsigned device-archive validation and signed TestFlight modes. Do not expose signing material to PR builds or automatically publish on push.
2. Use an isolated runner keychain and App Store Connect API authentication. Register/provision the bundle through Xcode automatic signing when the API key has the required access. Store the selected distribution identity and Apple API key only as encrypted GitHub secrets; keep temporary material out of artifacts and remove it after every run.
3. Generate a distinct numeric bundle build number without changing the app version. Check the built bundle ID/version, LAN purpose and three Bonjour types; for distribution verify the signature, team and non-development App Store profile before upload.
4. Test the validation rules and run a fresh device-archive build. Verify the Actions run and publish the workflow/docs. Attempt signed build/upload when required Apple credentials are available.
5. Document physical-iPhone acceptance: permission grant/denial, discovery and pairing, app changes, Files/iCloud pickers, capture/export, foreground/background reconnect and iPad layout. Leave App Store Connect processing and installation status explicit.

## Current evidence

The cached v0.1.7 archive is unsigned and still uses `nl.neerdael.adbstudio.poc`; a fresh current release archive has now built successfully with `nl.neerdael.bridgecoil`, version 0.1.7, and the expected LAN/Bonjour metadata. A second build verified the independent `1.1` bundle build number. Source contains iOS Bonjour discovery and file-picker integration. Physical iPhone tests remain pending. A valid distribution identity exists locally. No Bridgecoil distribution profile, Apple API key in the usual local folders, or Apple GitHub secrets were found. The user has been asked for the API key path and IDs; no private key was requested in chat.

The initial unattended certificate export was rejected by Keychain. A subsequent owner-authorized, single-identity export succeeded without changing its access policy or writing a private signing file to disk. All five signing/API secrets are now configured in GitHub. The owner supplied the Apple API key path and IDs through a local metadata file; that file is excluded from Git.


## Verified CI and remaining delivery gate

[Unsigned iOS workflow run 41215611335](https://github.com/johnneerdael/bridgecoil/actions/runs/41215611335) completed successfully, including app checks/tests, Rust core tests, the release archive, artifact metadata verification and upload. Independent workflow review found no important code defects. Local frontend validation passed 201 tests and type checking; four iOS metadata/profile tests passed.

The Account Holder accepted the updated Developer Program License Agreement. Authenticated provisioning access was rechecked against the expected distribution certificate. The user created App Store Connect record `6829245214`, SKU `bridgecoil`, with iOS and macOS platforms, using `nl.neerdael.bridgecoil`.

The first signed run, `41217036131`, completed Xcode export and then exposed a certificate-extraction argument bug in the verifier. A real macOS signed-app regression reproduced it; attaching the optional prefix with `--extract-certificates=<prefix>` fixed it. All five verifier tests pass. Exported packages are now retained when signing verification fails, while Apple upload remains success-gated. The retry, [run 41218436962](https://github.com/johnneerdael/bridgecoil/actions/runs/41218436962), passed on commit `60e8808d67b0322a6634ad93841c7322a6535f66`. It verified the expected signing certificate and App Store profile for version 0.1.7, build 3.1. Apple validation and upload both succeeded with no errors. Apple processing subsequently returned `VALID` through the delivery tracker and the app/builds API relationship. TestFlight then reported `MISSING_EXPORT_COMPLIANCE` for both internal and external testing. The standard-encryption category is staged but not saved; the owner has been asked whether France is in the intended distribution scope. Tester assignment and physical-device acceptance remain pending.
