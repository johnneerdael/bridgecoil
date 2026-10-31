# Automatic iOS delivery — 0.1.8 (22.1)

31 October 2026. [Release run 41227206811](https://github.com/johnneerdael/bridgecoil/actions/runs/41227206811), source `b116c1ac69b456e11612c1716b7addd12f5d6e86`, tag `v0.1.8`.

The push-triggered release workflow published Android first, then called the reusable iOS workflow with marketing version `0.1.8` and the release's testing focus. No separate iOS dispatch was required. The iOS job used build number `22.1`.

- App checks, native build, archive checks, exported IPA signature and provisioning checks passed.
- Apple validation and IPA upload passed.
- App Store Connect's **Build Uploads** list shows **0.1.8 (22.1)** with status **Processing**, confirming automatic delivery reached Apple.
- The workflow is waiting for Apple processing before publishing and reading back What to Test; that step has not yet been verified for this build.

GitHub publication precedes the iOS build/upload. The release page now links to its iOS job and explains this delay. Apple encryption requirements, beta review, group/tester access and physical-device acceptance remain separate checks; an accepted upload does not prove an installable or tested beta.
