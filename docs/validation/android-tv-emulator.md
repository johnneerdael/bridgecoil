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

## Wired TVs and launchers

Run on the same image, also on 2026-10-31. The launcher used was a small test app with a Home activity, signed with a throwaway key.

**A wired TV can't turn on wireless debugging.** This image is on Ethernet. Android logged `Not connected to any wireless network. Not enabling adbwifi` and set the option back to off, three times out of three. Joining Wi-Fi as well wasn't possible here, because Android stayed on the validated Ethernet network. So pairing and start at boot can't be tested on this image, and a wired TV normally loses both. Bridgecoil now detects a TV with an Ethernet address and no Wi-Fi address, and says so during setup. That the managers can't start at boot on a wired TV follows from this, but wasn't observed separately.

**Android TV's launcher outranks any other.** It has `priority=2`, and an installed app gets 0. Android compares priority first, so `cmd package set-home-activity` reports `Success` and moves the Home role, while Home keeps opening the stock launcher. Before this run, Bridgecoil had no way past that. It refused to disable the active launcher and only offered that after Home had moved. It now reports the chosen launcher, explains the block, and allows the active launcher to be disabled once a different installed launcher holds the Home role. Removing it or clearing its data stays refused.

| Check | Result |
|---|---|
| Set the default while stock outranks it | Reported `Unknown`, role moved, Home unchanged |
| Turn stock off through review | Passed, verified |
| Real `KEYCODE_HOME` opens the new launcher | Passed |
| Turning off the launcher in use | Refused |
| Stock stays off after a reboot, and the new launcher opens at boot | Passed, by hand, with no crashes |
| Turn stock back on and make it the default | Passed, Home returned |

**A change needs a moment to stick.** Two reboots within about 8 seconds of disabling brought the stock launcher back. A third, after 45 seconds, kept it off. The likely cause is Android writing this state to disk late, so a power cut right after a change can undo it.

## Code pairing

The TV image can't do this, so it ran against the Android 16 phone image on its virtual Wi-Fi, reached through `adb forward`. It proves the protocol against a real `adbd`, not on a TV.

| Check | Result |
|---|---|
| New identity pairs with the six-digit code | Passed |
| Connects over TLS with legacy refused | Passed, encrypted |
| Shell runs as the shell user | Passed, uid 2000 |

Turning on wireless debugging first shows **Allow wireless debugging on this network?** on the device. Without an answer the option stays off.

## Not covered

- **Hold gestures on this image.** The emulator console can only tap without root. Holds were proven with a real kernel key event on the phone image.
- **Pairing on a TV, and start at boot in the manager.** Both need wireless debugging, which this wired image refuses. The test started the server by hand after each boot.
- **A brand's own launcher.** Priorities and the number of stock launchers vary, and Google TV may have more than one. A system update may also turn one back on.
- **A physical TV or remote.** Nothing here replaces that.
