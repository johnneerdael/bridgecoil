# One-shot app metadata inspector

Bridgecoil owns this Java source and generated DEX JAR (`crates/adb-core/assets/bridgecoil-inspector.jar`). Rebuild with `scripts/build-inspector.py` using Android SDK 36, build-tools 36.1.0 and JDK 17. The adjacent JSON records source/JAR digests and SDK versions. It is a read-only PackageManager inspector under the existing ADB shell identity, started with app_process and then exited. It is not an installed companion APK, remapper or background service.

The controller stages the hash-named JAR under `/data/local/tmp/bridgecoil-inspector-<sha256>.jar`; no user script or package is replaced. Java profile context reflection can be denied by firmware. The UI then preserves package-ID browsing and leaves unavailable values unknown. This fallback is not evidence of successful metadata inspection.

The label index is capped at 1024 entries and 96 characters per label. Detailed inspection accepts at most 32 packages, 64 launch components, 256 permission entries and 16 signing fingerprints per app. Icons are drawn to 64x64 PNG, capped at 32 KiB and independently bounded/validated in Rust. The native shell response is capped at 1 MiB. Display labels are never interpolated into shell commands. A 30-second, session-bound label cache avoids repeating the index on every page; package-state operations always use fresh target-scoped probes.

Installed signing fingerprints come from Android PackageManager, not from authenticating a user-selected APK. APK signing compatibility review remains a separate implementation task. See the official [PackageManager API](https://developer.android.com/reference/android/content/pm/PackageManager) and [AOSP package shell implementation](https://android.googlesource.com/platform/frameworks/base/+/main/services/core/java/com/android/server/pm/PackageManagerShellCommand.java).

`Context.createContextAsUser` is missing on older Android, so the inspector falls back to `createPackageContextAsUser`. Without that, it failed on a real Android 9 box before reading a single package.

Run on real hardware on 2026-10-31: a Ugoos AM6 on Android 9 returned details, the certificate and 116 labels through the fallback, and an Android 16 phone returned details through the modern call. Android 10 and below are otherwise untested, and other brands' firmware may still deny this reflection.
