# Releasing Bridgecoil

## How a release happens

Every push to `main`, which includes merged PRs, runs `.github/workflows/release.yml`. There's nothing to bump or tag by hand.

1. **Decide.** `scripts/next-release.mjs` reads what's new since the last tag. A release needs at least one user-facing note. If a newer commit is already on `main`, that commit's run takes over and includes these notes.
2. **Version.** The next patch number comes from the tags. To start a new minor version, change the version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`. The build stamps the released version in and never commits it.
3. **Android.** The tests run, a signed arm64 APK is built, its certificate is checked and the GitHub release is published with the tag.
4. **iOS.** The same version is uploaded to TestFlight, with the same notes as What to Test. If this fails, the Android release stays up and the run shows the failure.

## Release notes

| Where the change comes from | What becomes the note |
|---|---|
| A merged PR | Each bullet in its `## Release notes` section. The PR gate won't pass without it |
| A PR that says `Internal:` | Nothing, and it doesn't release by itself |
| A direct `feat:` or `fix:` commit | The commit subject, so write it for someone using the app |
| A direct `fix(ci):`, `fix(build):`, `fix(release):`, `fix(test):` or `fix(docs):` commit | Nothing, and it doesn't release by itself |
| Any other direct commit, such as `docs:` or `ci:` | Nothing, and it doesn't release by itself |

Internal changes wait for the next user-facing one. To release one anyway, run **Release** from the Actions tab with **publish** ticked. It still needs a user-facing note.

To test a build without publishing, run **Release** with **publish** left off. The APK appears as a workflow artifact.

## Release presentation

For a one-off iOS build, use the [iOS beta / TestFlight workflow](ios-beta.md). It supports unsigned validation without secrets and verified signed export/upload from `main`. Apple credentials and physical-device acceptance are tracked separately from Android releases. Before every beta release, update [the versioned What to Test source](testflight/README.md). Each TestFlight upload automatically posts and verifies notes for its exact processed build. Use the workflow's `notes` mode with an existing marketing version and build number to repair or revise notes without rebuilding.

The release workflow calls `scripts/release-notes.mjs` with the notes above. Every new release has:

- The original dark 1600 × 480 brand-kit README banner, pinned to the released source commit.
- Website, getting-started and issue links.
- Version-specific Android APK and `SHA256SUMS` download links matching the published artifact names.
- The changes, one bullet per note, with a full-changelog link.
- Current compatibility limits, with build and source verification in a collapsed section.

The script changes presentation; it does not choose a version, change signing or publish a release itself. Do not bump the app version just to change the website or release appearance. Existing published releases are not rewritten automatically.

Check the renderer without Android tooling:

```sh
node --test scripts/release-notes.test.mjs scripts/next-release.test.mjs
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
