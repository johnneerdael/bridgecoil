# Bridgecoil

Take back control of your Android TV from your phone, with no typing and no prior ADB knowledge.

Android TVs come with ad-filled launchers, apps you can't remove and buttons that only open what the manufacturer was paid to put there. The tools that fix this are command-line tools. Bridgecoil puts them behind plain questions, checks each result on the TV and keeps a way back.

Bridgecoil talks to the TV directly over wireless debugging. It needs no computer, no root and no ADB server.

## What it does

- **Clean up.** Shows which preinstalled apps the community rates as safe to turn off, with what each one does, and turns them off together. Turn them back on just as easily.
- **Replace the launcher.** Try a new Home app, make it the default, press Home on the real remote to prove it works, then turn off the old one. A way back stays on the screen until you're sure.
- **Remap buttons.** Press a button on your remote and choose what it does: open any app, or act as another button. Tapping and holding can do different things.
- **Keep it working.** A small listener runs on the TV and restarts itself after the TV does, without your phone. Bridgecoil tells you once it has seen that happen on your TV.
- **Advanced commands, no typing.** Pick from explained lists, see the exact command, review it and run it. Save the ones you use.
- **Apps.** Every app, grouped by how safe it is to remove, with community ratings from [Universal Android Debloater](https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation).
- **Files.** Browse TV storage, upload, save to your phone, rename, move and delete.
- **Screen capture and logs.** Screenshots, short recordings and filtered logs, saved to your phone.
- **Remote, settings, history.** A phone remote, common settings, and a record of every change with undo where one exists.

## How TV setup works

**TV setup** walks through this in order and only moves on when the TV proves each step worked:

1. Choose a helper app, [Shizuku-Next](https://github.com/rushiranpise/Shizuku-Next) or [Porter](https://github.com/d4rken-org/porter). Bridgecoil downloads the latest release, checks its signing certificate and installs it.
2. Start it over the same connection.
3. Install [Termux](https://github.com/termux/termux-app) and Termux:Boot the same way.
4. Connect Termux to the helper app. You approve one prompt on the TV.
5. Turn on start at boot in the helper app, and Bridgecoil adds a small startup script.

You don't download anything beforehand or type anything in Termux. Releases signed by anyone other than the expected developer are refused.

Google Play Protect may stop the Termux apps with "Unsafe app blocked", because they're built for an older version of Android. Use your remote on the TV to choose **More details**, then **Install anyway**. Bridgecoil leaves that decision to you and never turns Play Protect off.

## Get started

1. Install the APK from [Releases](https://github.com/johnneerdael/bridgecoil/releases) on an arm64 Android phone.
2. On the TV, open **Settings → System → About** and repeatedly select **Android TV OS build** to unlock Developer options. Menu names vary between brands.
3. In Developer options, turn on **Wireless debugging**. The phone and TV must be on the same network.
4. Open Bridgecoil. It looks for the TV on your network and walks you through pairing with the code the TV shows.

## What it can't do

- A remapped button still does its normal action first, then yours. Android gives apps no way to stop that without root. If a button opens a brand's app, disable that app.
- Power, voice and some brand buttons never reach apps, so they can't be remapped.
- Streaming apps usually appear black in screenshots and recordings.
- Some TVs block the access Termux needs. Bridgecoil says so, and mappings then work until the TV restarts.
- Shizuku-Next can't be installed over a different app that holds the Shizuku name. Porter can sit alongside it.
- Turning off an app only affects that TV. Unrated apps are unknown, not safe.

Guided setup has been run end to end on an Android TV 16 emulator image with secure ADB and no root. It has not been completed on a physical TV yet, and real TVs differ.

## Development

A Tauri 2 app with a Rust core in `crates/adb-core` and a Svelte 5 interface. ADB is spoken directly through [libadb](https://docs.rs/libadb).

Requires Node 22, Rust through rustup and each platform's toolchain.

```sh
npm ci
npm run check
npx vitest run
node scripts/cargo.mjs test --workspace
npm run dev
```

`npm run dev` shows the interface in a browser, where native features are marked unavailable.

```sh
npm run android -- build --debug --target aarch64 --apk --ci   # Android, needs ANDROID_HOME, NDK_HOME and JAVA_HOME
npm run ios -- build --debug --target aarch64-sim --no-sign --ci   # iOS simulator, needs Xcode, XcodeGen and CocoaPods
npm run build:desktop   # macOS
```

`crates/adb-core/tests/toolkit_live.rs` walks the whole setup on a disposable emulator. Run it by hand, and only against an emulator, because it installs and removes apps. See the header of that file.

Releases are described in [docs/release.md](docs/release.md).
