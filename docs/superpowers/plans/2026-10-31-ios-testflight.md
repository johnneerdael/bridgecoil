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

The selected identity could not be exported unattended: macOS returned Keychain authentication status -25293. No access-control policy was changed and no GitHub signing secrets were written. User-side export authorization or a password-protected P12 is required. The user requested API-key creation guidance and received the App Store Connect steps.
