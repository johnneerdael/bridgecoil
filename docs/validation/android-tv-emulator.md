# Android TV emulator run

Run on 2026-10-31 against the `android-36/android-tv/arm64-v8a` system image (`sdk_google_atv64_arm64`). It's a `user` build with secure ADB, no root and the Google TV launcher, so it behaves far more like a retail TV than the phone image used before it.

| Check | Porter | Shizuku-Next |
|---|---|---|
| Guided setup from a bare TV to Done | Passed | Passed |
| Release fetched, certificate verified, installed | Passed | Passed |
| Server started over ADB | Passed | Passed |
| Termux and Termux:Boot installed | Passed, after Play Protect | Passed, after Play Protect |
| Termux connected, access prompt approved | Passed | Passed |

| Check | Result |
|---|---|
| A real key event captured | Passed, `KEY_A` from `qwerty2` |
| Mapped key opened its app | Passed, `com.android.tv.settings/.MainSettings` |
| Pausing silences mappings | Passed |
| Cold boot starts the Termux:Boot monitor | Passed |
| Listener returns once the manager is running | Passed, about 55 seconds after the server started |
| Bridgecoil reports it came back | Passed, `survivedRestart: true` |
| Screenshot | Passed |
| Three second recording, saved and playable header | Passed |
| 3 MB download matches SHA-256 | Passed |
| Unreadable file refused, connection stays up | Passed |
| 40 cancelled downloads, then a whole 8 MB file | Passed |
| 5000 line log | Passed, 607 KB |
| Log for an app that isn't running | Passed, reported as not running |

## Found by this run

**Play Protect blocks both Termux apps.** It shows "Unsafe app blocked" because they're built for an older version of Android, and `pm install` waits on the answer.

- **More details**, then **Install anyway** lets the install finish.
- **Got it** ends it with `INSTALL_FAILED_VERIFICATION_FAILURE`.

Porter installs without it. Bridgecoil warns before the install and explains the failure. It doesn't touch the verifier setting, because that is the owner's decision.

## Not covered

- **Hold gestures on this image.** The emulator console can only tap without root. Holds were proven with a real kernel key event on the phone image.
- **Start at boot in the manager.** It needs the owner to pair the manager with wireless debugging on the TV. The test started the server by hand after each boot.
- **Changing the launcher.** This image has only its own launcher.
- **A physical TV or remote.** Nothing here replaces that.
