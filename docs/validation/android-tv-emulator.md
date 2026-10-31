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

**Android closes a background app's connections.** On a phone running Android 15 or later, the socket closed about 6 seconds after the app left the screen. A file picker counts, so saving, uploading or choosing an APK could drop the TV while the app still said Connected. A foreground service now holds the connection.

**UAD's "safe to remove" includes YouTube.** Everything was ticked at first. Clean up now leaves any app with an icon on the TV unticked.

## The signed app against the TV

The signed release APK ran on an Android 16 phone emulator, connected to the TV emulator over the network.

| Check | Result |
|---|---|
| Certificate matches the release key | Passed |
| Update keeps the saved TV | Passed |
| Connect, approve on the TV | Passed |
| Screenshot of the TV, shown in the app | Passed |
| Native save dialog writes a valid 1920×1080 PNG | Passed |
| Connection survives 15 seconds behind the save dialog | Passed, was dropping after 6 |
| Recording continues after leaving the screen, and Stop is still there | Passed |
| Saved recording matches the TV's copy by SHA-256 and is removed from the TV | Passed |
| Real ratings downloaded, real preinstalled apps listed | Passed |
| App turned off through review, verified, and turned back on | Passed |
| Connection service stops on disconnect | Passed |

## Not covered

- **Hold gestures on this image.** The emulator console can only tap without root. Holds were proven with a real kernel key event on the phone image.
- **Start at boot in the manager.** It needs the owner to pair the manager with wireless debugging on the TV. The test started the server by hand after each boot.
- **Changing the launcher.** This image has only its own launcher.
- **A physical TV or remote.** Nothing here replaces that.
