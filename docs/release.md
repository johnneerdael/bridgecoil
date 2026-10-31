# Releasing Bridgecoil

## Publish a release

1. Set the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
2. Commit that to `main`.
3. Push a matching tag, such as `v0.2.0`.

`.github/workflows/release.yml` then runs the tests, builds a signed arm64 APK, checks its certificate and publishes a GitHub release. A tag that doesn't match the app version stops the build.

To test the build without publishing, run **Android release** from the Actions tab. The APK appears as a workflow artifact.

## Release presentation

The release workflow calls `scripts/release-notes.mjs` after GitHub generates the change notes. Every new release has:

- The original dark 1600 × 480 brand-kit README banner, pinned to the released source commit.
- Website, getting-started and issue links.
- Version-specific Android APK and `SHA256SUMS` download links matching the published artifact names.
- GitHub's generated changes and full-changelog link, preserved as supplied.
- Current compatibility limits, with build and source verification in a collapsed section.

The script changes presentation; it does not choose a version, change signing or publish a release itself. Do not bump the app version just to change the website or release appearance. Existing published releases are not rewritten automatically.

Check the renderer without Android tooling:

```sh
node --test scripts/release-notes.test.mjs
node scripts/release-notes.mjs --repository johnneerdael/bridgecoil --version 0.1.7 --sha <full-source-commit-sha> --changes /path/to/generated-notes.md
```

The banner is `site/assets/marketing/bridgecoil-readme-dark.png`. Keep this file at that path and preserve the supplied artwork. See [website maintenance](website.md) for shared assets and the Pages workflow.

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
