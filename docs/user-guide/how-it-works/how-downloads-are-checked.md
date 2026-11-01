# How downloads are checked

<p class="lead">When Bridgecoil downloads an app for you, it checks who signed it before anything reaches your TV.</p>

This applies to Termux, Termux:Boot, Shizuku-Next, Porter and the four listed launchers. Bridgecoil asks GitHub for the project's latest release, then:

1. Picks the right file for your TV's processor.
2. Compares the download with GitHub's checksum, when GitHub provides one. A mismatch gives "The {name} download was damaged."
3. Checks the APK's signature and compares the signing certificate with one pinned in the app.
4. Checks the package name.

If the signature or package doesn't match, you see "This {name} download is not signed by its publisher, so it was refused." and the file is deleted. Nothing is sent to the TV.

## Pinned certificates

| App | Repository | SHA-256 of the signing certificate |
|---|---|---|
| Termux, Termux:Boot | `termux/termux-app`, `termux/termux-boot` | `b6da01480eefd5fbf2cd3771b8d1021ec791304bdd6c4bf41d3faabad48ee5e1` |
| Shizuku-Next | `rushiranpise/Shizuku-Next` | `06360bb800d1a117c3a22c91a58e638bcd37281cb4d644fcfac5d7238c654293` |
| Porter | `d4rken-org/porter` | `c6302c2b163c7f42f6d8bd658e53387880511020136a083f0a6a4bdc20b4bfa0` |
| Backdrop | `johnneerdael/backdrop-launcher` | `18a076c8f5666fb103184caf1a98c753a044cc54ffc8ddc54cdd13ccc339f93d` |
| Projectivy | `spocky/miproja1` | `f6697bf4082ee97511e4de07863193884a015b7ab5860430321bda1042b0aadd` |
| Monet | `Klevico/Monet-Launcher` | `4bec386a6baae56f0694d3be54f67e0625b973109e0dbb947df2a5d0254975e0` |
| AT4K | `avadhesh18/at4k` | `bfc84ab1296c72653ebca20a2f4cd293bc342a67dd4c61df931edfbad5271685` |

## What this proves

A matching signature shows the file came from whoever holds that signing key, and that nobody changed it afterwards. It doesn't prove the app is free of bugs, or that a developer's key was never stolen. If a developer changes their key, Bridgecoil refuses the new release until the app is updated.

## The agent

The agent isn't downloaded. It's built into Bridgecoil, with one build each for `arm64-v8a`, `armeabi-v7a` and `x86_64`. The file `crates/adb-core/assets/agent/agent-build.json` records the SHA-256 of each build and of the source it came from, and the tests check those match. You can rebuild it from source with `scripts/build-agent.py`.

Each time you open TV setup, Bridgecoil compares the agent on the TV with the one in the app and replaces it if it's different. Your pairing is kept.

## Apps you install from a file

A file you pick yourself isn't checked this way. Bridgecoil shows its package name and fingerprint, and those don't prove who made it. See [Install an app from a file](../apps/install-from-a-file.md).
