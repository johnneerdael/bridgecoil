# Bridgecoil

A Tauri 2 app with a Rust backend and a mobile-first Svelte interface. This first version prioritizes direct ADB pairing, connection and installation of a standalone APK selected through a native file picker. The broader approved workspace design remains in `docs/superpowers/specs/`.

The current implementation adds the five workspace areas through a hamburger menu, Bridgecoil's supplied typography/palette/assets, System/Light/Dark appearance and bounded page layouts. The broader implementation is in progress: app labels/icons, split sets/signature review, discovery, physical key mapping/startup, artifact transfers and full platform acceptance are not complete. The retained branded APK is a milestone build, not the final workspace build.

## Connection and install foundation

- Direct pairing-code exchange and separate connection endpoint, without a desktop ADB server or external adb executable in the app.
- Persistent, app-private RSA host identity. A new installation must be paired with the TV independently.
- TLS wireless-debugging connections; legacy unencrypted connections require an explicit option.
- Device model, Android version, SDK and ABI inspection.
- Native APK selection, app-private staging, binary-manifest package/version inspection and SHA-256 fingerprint.
- Streamed transfer, install/update for the selected Android profile, and installed-version readback before reporting success.
- Per-stage progress, structured failures, stale-session rejection and unknown-outcome handling. No automatic uninstall or installation retry.

These are implemented code paths, not a claim of complete physical-device acceptance. See `docs/validation/POC.md` for the actual validation matrix.

## Try it

1. Put the phone/computer and TV on the same reachable local network. Enable the TV's developer/debugging options.
2. On the TV, open Wireless debugging → Pair device with pairing code. Enter that screen's address/port and six-digit code in **Pair TV**.
3. Switch to **Connect** and enter the separate address/port from the main Wireless debugging screen. Pairing success is not an active connection.
4. Open **Install APK**, choose a standalone APK through the native picker, inspect the review, confirm the selected TV and trusted file source, and install.
5. Keep the app open until verification finishes. If the outcome is unknown, reconnect and inspect the TV before retrying.

For older TVs, enable their supported network debugging and use Connect with the explicit legacy option. First authorization may need approval on the TV. This app cannot turn debugging on remotely on an unauthorized TV.

## Development

Requirements: Node 22.22.2 or newer compatible with the lockfile, npm, Rust through rustup (Rust 1.90+), and the platform toolchain. The Tauri runner resolves the actual rustup compiler so another Rust installation cannot silently shadow mobile targets.

```sh
npm ci
npm run check
npm test
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run desktop
```

Browser-only interface preview: `npm run dev`. It explicitly marks native functionality unavailable; it never fakes a connection or installation result.

### Android

Set `ANDROID_HOME`, `NDK_HOME` and a compatible `JAVA_HOME` for the installed Android SDK/NDK/JDK, then:

```sh
npm run android -- init --ci
npm run android -- build --debug --target aarch64 --apk --ci
```

The first artifact is an ARM64 debug APK. Other ABIs need their own builds. The source includes a generated Android project with INTERNET permission and Android 17 local-network declaration/request handling. Real network policy and file-provider behavior still need testing on the target phones.

### iOS

Install Xcode, XcodeGen and CocoaPods, then:

```sh
npm run ios -- init --ci
npm run ios -- build --debug --target aarch64-sim --no-sign --ci
```

This creates an unsigned simulator app, not a physical-device IPA. A physical iPhone/iPad build needs signing/provisioning and its own validation. The native configuration includes a local-network purpose string. Android Shizuku/Termux dependencies are not installed on the controlling iPhone.

### macOS

```sh
npm run build:desktop
```

This creates `target/debug/bundle/macos/Bridgecoil.app`. Local debug builds are not store releases or notarized distributions.

## Current boundaries

- Manual IP/hostname and port entry; automatic mDNS discovery follows later. Android 17 requests local-network access just before pairing/connection, and denial points to app permission settings.
- One active device, Owner/user 0, standalone APKs up to 1 GiB. Split APK sets and AABs are rejected/deferred.
- Metadata inspection does not authenticate an APK publisher or pre-verify signing compatibility. Android's package installer enforces update/signature policy; the POC never uninstalls to work around a rejection.
- APKs are copied into app-private temporary storage, and removed when a selection is replaced, used or the app exits normally. Abrupt process termination can leave a cache file; clear the app cache if needed.
- Host keys are kept in app-private storage with restrictive Unix permissions. Production secure-vault/backup policy needs further platform work; do not treat this as a hardware-backed keystore.
- The libadb TLS profile follows ADB's host-authentication model; encryption is not independent cryptographic authentication of the target TV.
- Legacy transport still requires the TV’s shell_v2 capability. Older daemons without it are not supported by this POC.
- No background install guarantee or automatic replay after disconnection.
- Remote remapping, guided Shizuku/Termux installation, launcher changes and the full toolkit remain on the approved roadmap rather than appearing as simulated buttons in the POC.
- No packages or settings on the user's TV were changed during automated POC validation.
