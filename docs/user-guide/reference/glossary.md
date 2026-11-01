# Glossary

ADB
:   Android Debug Bridge. Android's own tool for managing a device from another device. Bridgecoil speaks it directly, so you don't need a computer.

Agent
:   Bridgecoil's small program in Termux. It connects to the TV's own ADB so that things can run with no phone. See [Why it needs this much access](../how-it-works/why-it-needs-access.md).

APK
:   The file an Android app is installed from.

Disable
:   Turn an app off without deleting it. Its data stays and it can be turned back on.

Launcher
:   The app that shows your home screen.

Legacy ADB
:   The older, unencrypted way to connect, often on port 5555.

Listener
:   The script on the TV that watches for your mapped buttons.

Mapping
:   A rule saying what a button should do.

Package name
:   An app's real identifier, such as `com.termux`. Names can be shared, and package names can't.

Pairing
:   Introducing a device to the TV once, with a six-digit code.

Profile
:   A separate user on the TV, each with its own apps and settings.

Shell
:   A command line on the TV. Bridgecoil's shell runs as the `shell` user, which has more access than an app and less than root.

Shizuku
:   An app that gives other apps shell access. Bridgecoil doesn't need it.

Stock launcher
:   The launcher your TV came with.

Termux
:   An app that provides a Linux environment on Android. Bridgecoil keeps its agent there.

Termux:Boot
:   A Termux add-on that runs scripts when the device starts.

Wireless debugging
:   The encrypted way to connect, on Android 11 and later.
