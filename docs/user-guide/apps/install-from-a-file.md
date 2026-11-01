# Install an app from a file

<p class="lead">Put an app on your TV that isn't in its store, using an APK, APKS, XAPK or APKM file on your phone.</p>

You can start from **Install APK** on the Apps page, or **Install from a file** in [Home app](../home-screen/install-a-launcher.md).

1. Tap **Choose file ＋** and pick the file.
2. Check the name, version and size. **File details & fingerprint ↗** shows more.
3. Tick **Install this app on this device**.
4. Tap **Install app ↑**, and keep the app open while it transfers.

!!! screenshot "install-chosen.png"
    Install an app with a file chosen and the confirm box visible.

!!! screenshot "install-progress.png"
    Transferring app with its progress bar.

When it's done, you'll see `Verified: {package} · version {code} is installed`.

**File details & fingerprint ↗** lists the **Package**, **Version code**, **Minimum SDK**, **Architecture**, size and **SHA-256 fingerprint**. **Change file** lets you pick another.

If you're not connected, you'll see **Connect your device first**, with **Set up connection →**.

## Apps in several parts

Many apps are published as a set of APKs in one file, usually ending in `.apks`, `.xapk` or `.apkm`. Bridgecoil also opens `.apk+` files and ZIPs of APKs, and goes by what's inside rather than the file's name.

A set like this holds parts for every kind of device. Bridgecoil reads each part and sends only what this TV needs:

- the part for its processor, such as `arm64-v8a` or `armeabi-v7a`
- the part closest to its screen density
- the TV's languages, plus English
- every feature of the app

The review shows how many parts that comes to, such as *4 of 58 parts in this file are needed for this TV*. All of them are installed together, so if one fails nothing is installed.

Some apps and games also have game data, in `.obb` files. The review shows its size, and Bridgecoil copies it to the TV after the app is installed. If that copy fails, the app stays installed and you'll be told.

Bridgecoil unpacks the file on your phone first, so you need free space there for about twice its size for a moment. That copy is removed once the app is installed, or the next time Bridgecoil opens if it was closed before then. Your original file isn't touched.

## Only install what you trust

Bridgecoil shows the file's details, but those don't prove who made it. An installed app can do whatever Android allows, so install files from sources you trust. The fingerprint lets you compare a file with one published by its developer.

## If it doesn't install

| Message | What to do |
|---|---|
| "Google Play Protect stopped this install on the TV." | Try again. On the TV, choose **More details**, then **Install anyway** |
| "APK requires a newer Android version than this TV." | Find a build for your TV's Android version |
| "APK contains no native libraries compatible with this TV." | Find a build for your TV's processor. **Device details** shows it as ABI |
| "This POC installs standalone APKs, not individual split APKs." | This file is one part of a split app. Find the whole set, as an APKS, XAPK or APKM |
| "This file has no version for this TV's processor" | The set leaves out your TV's processor, which is common for 32-bit TVs. Find a build that includes it |
| "This file is password protected" | Download a version without a password |
| "This is an Android App Bundle (.aab)" | Android can't install these directly. Find an APK, APKS, XAPK or APKM |
| "This file has no base APK" | This is only part of an app. Find the whole set |
| "The app is installed, but its game data couldn't be copied." | The TV may be out of space. Free some, then install the file again |
| Android rejected the installation, with a signature error | A version signed by someone else is already installed. Remove it first |

## Limits

Files can be up to 4 GB, and the APKs installed from one can be up to 1 GB together. Android App Bundles (`.aab`) can't be installed directly. Nothing is ever uninstalled automatically. Installs don't appear in History, so there's no undo. To remove an app, use [Uninstall](turn-off-or-remove-an-app.md).
