# Connection diagnostics POC 0.1.3

The user reported a TLS error during direct connection to Google TV and requested a newer APK with better error messaging. Whether pairing completed in the previous build is unknown. The working local reference is tv-remote; its broader software was not imported.

This build distinguishes an explicitly rejected TLS client key from an ambiguous close before ADB CNXN and other TLS failures. It exposes the original error under Details. The ADB TLS profile already accepted self-signed device certificates and skipped CA/hostname checks; CertificateVerify proof remains enabled. This build does not claim to fix the reported TV failure.

Successful pairing is recorded locally by host across separate pairing/connection ports. That record describes a past completed pairing exchange, not current TV trust. Pairing and connection have progress and cancellation. Legacy key publication requires the explicit legacy option; its TV-approval wait can extend beyond the initial handshake timeout.

Validation: frontend type check and 26 tests passed; native workspace compile passed after using Channel directly as a Tauri command argument; auth-error classification and the existing direct-wire install/interruption tests passed. The production ARM64 APK built, manifest ID nl.neerdael.adbstudio.poc and versionCode1003/version0.1.3 were checked, and the production frontend was checked for exclusion of the fixture banner. It was installed over USB on CPH2747; package readback and the rendered connection screen were verified, with no startup permission error visible. No TV connection or APK installation on a TV was performed by the agent.

Artifact: artifacts/Bridgecoil-0.1.3-arm64.apk, 45255552 bytes, SHA256 4c96e6888166201648c6696b5c6ab915be59fec7fb02a9a1629e6a8bc7c0f4af. The manifest is artifacts/connection-diagnostics-0.1.3.json. Phone screenshots/UI dumps remain outside the repository.

The source snapshot also contains earlier workspace refinements, a separate UI-only test harness and an APK signature-inspection prototype. The signature prototype is not wired into APK installation. The complete workspace is unfinished. The user's latest instruction stops broader feature work and emulator testing while they try this diagnostic build; the next connection change must use the new error evidence.
