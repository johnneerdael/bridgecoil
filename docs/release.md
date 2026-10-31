# Releasing Bridgecoil

## Publish a release

1. Set the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
2. Commit that to `main`.
3. Push a matching tag, such as `v0.2.0`.

`.github/workflows/release.yml` then runs the tests, builds a signed arm64 APK, checks its certificate and publishes a GitHub release. A tag that doesn't match the app version stops the build.

To test the build without publishing, run **Android release** from the Actions tab. The APK appears as a workflow artifact.

## Signing

All releases must use the same key, or Android refuses to install them as updates.

Certificate SHA-256:

```text
04:EE:27:FC:C7:0E:AB:BC:F8:AE:47:1D:36:35:C7:14:C9:D0:C8:9E:2D:D2:1D:CD:D3:EF:5B:A7:C6:A5:CC:D0
```

Repository settings:

| Name | Type | Contents |
|---|---|---|
| `ANDROID_KEY_BASE64` | Secret | Base64 of the PKCS#12 keystore |
| `ANDROID_KEY_PASSWORD` | Secret | Keystore and key password |
| `ANDROID_KEY_ALIAS` | Secret | Key alias |
| `ANDROID_SIGNER_SHA256` | Variable | Certificate SHA-256 in lowercase, without colons |

GitHub secrets can't be read back, so keep the keystore and its password backed up outside this repository. Never commit either one: `.gitignore` excludes `keystore.properties`, `*.p12`, `*.jks` and `*.keystore`.

Without `keystore.properties`, local release builds are unsigned and debug builds use the debug key.

The workflow must never run on `pull_request`, because it can read the signing key.
