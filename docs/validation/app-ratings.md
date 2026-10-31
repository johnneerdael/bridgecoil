# App ratings validation

Not yet in a built APK, and not yet tried on a TV.

Automated: 109 frontend tests and 140 Rust tests passed. Svelte reported zero errors. Clippy's four remaining issues are in `apk_set.rs`, `manifest.rs` and `discovery.rs`, outside this work.

Real list: `GithubSource` reached GitHub over TLS from a Mac on 2026-10-31 and fetched commit `5c179a04`. All 5,382 packages had a recognised level, the file passed the acceptance rule, `com.android.tv.settings` was Unsafe and `com.google.android.tvlauncher` was Expert.

Contrast: every level's badge text measured at least 6.15:1 in both themes.

Screens: checked at 412×915 in both themes using fixture data. All six groups appear in order, paging stops at collapsed Unsafe, and neither the page nor the list scrolls sideways.

Still to record on the TV: the opt-in download on the phone, list order, the Unsafe block, and the share of packages in Not rated.
