# App ratings validation

Not yet installed on a TV-paired phone in signed form.

Automated: 114 frontend tests and 148 Rust tests passed. Svelte reported zero errors. Clippy's four remaining issues are in `apk_set.rs`, `manifest.rs` and `discovery.rs`, outside this work.

Real list: `GithubSource` reached GitHub over TLS from a Mac on 2026-10-31 and fetched commit `5c179a04`. All 5,382 packages had a recognised level, the file passed the acceptance rule, `com.android.tv.settings` was Unsafe and `com.google.android.tvlauncher` was Expert.

Contrast: every level's badge text measured at least 6.15:1 in both themes.

Screens: checked at 412×915 in both themes using fixture data. All six groups appear in order, paging stops at collapsed Unsafe, and neither the page nor the list scrolls sideways.

TV coverage: a read-only package listing from a TCL Smart TV Pro on 2026-10-31, compared with list commit `5c179a04`, gave 209 packages: 48 installed by the user and 161 preinstalled. The preinstalled split was 15 Recommended, 18 Advanced, 65 Not rated, 37 Expert and 26 Unsafe. Not rated is 40% of preinstalled apps, and 36 of those 65 are `com.tcl` packages. An API 36 emulator had 64 of 245 preinstalled apps Not rated, or 26%.

Still to record on the TV: the opt-in download on the phone, pairing with the signed app, and the Unsafe block on screen.

Files on a real Android system: on 2026-10-31 the real `Client` ran over ADB against an API 36 emulator as the shell user. Uploads matched by SHA-256, including 20 MB with progress and an unusual name. Existing names were refused, cancel left no files and kept the connection, and a refused name or read-only folder also kept it. Move, undo, rename, new folder and delete verified by readback. A changed item invalidated its review, protected and read-only paths were refused, and deleting a link kept its target.

Shared storage rejects `" * : < > ? \\ |`, so the app now says so before review. A name ending in a dot or space was accepted.
