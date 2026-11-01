# Install an app from a file

<p class="lead">Put an app on your TV that isn't in its store, using an APK file on your phone.</p>

You can start from **Install APK** on the Apps page, or **Install from a file** in [Home app](../home-screen/install-a-launcher.md).

1. Tap **Choose APK file ＋** and pick the file.
2. Check the name, version and size. **File details & fingerprint ↗** shows more.
3. Tick **Install this APK on this device**.
4. Tap **Install app ↑**, and keep the app open while it transfers.

!!! screenshot "install-chosen.png"
    Install an app with a file chosen and the confirm box visible.

!!! screenshot "install-progress.png"
    Transferring APK with its progress bar.

When it's done, you'll see `Verified: {package} · version {code} is installed`.

**File details & fingerprint ↗** lists the **Package**, **Version code**, **Minimum SDK**, **Architecture**, size and **SHA-256 fingerprint**. **Change file** lets you pick another.

If you're not connected, you'll see **Connect your device first**, with **Set up connection →**.

## Only install what you trust

Bridgecoil shows the file's details, but those don't prove who made it. An installed app can do whatever Android allows, so install files from sources you trust. The fingerprint lets you compare a file with one published by its developer.

## If it doesn't install

| Message | What to do |
|---|---|
| "Google Play Protect stopped this install on the TV." | Try again. On the TV, choose **More details**, then **Install anyway** |
| "APK requires a newer Android version than this TV." | Find a build for your TV's Android version |
| "APK contains no native libraries compatible with this TV." | Find a build for your TV's processor. **Device details** shows it as ABI |
| "This POC installs standalone APKs, not individual split APKs." | This file is one part of a split app. Find a standalone APK |
| Android rejected the installation, with a signature error | A version signed by someone else is already installed. Remove it first |

## Limits

Files can be up to 1 GB. Split APK sets and Android App Bundles aren't supported. Nothing is ever uninstalled automatically. Installs don't appear in History, so there's no undo. To remove an app, use [Remove from profile](turn-off-or-remove-an-app.md).
