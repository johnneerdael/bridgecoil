// Build only: fixture APK uses a distinct Android application ID. Never installs a device.
process.env.BRIDGECOIL_UI_VALIDATION = "1";
process.argv.splice(2, process.argv.length - 2, "android", "build", "--debug", "--target", "aarch64", "--apk", "--ci", "--config", "src-tauri/tauri.ui-validation.conf.json");
await import("./tauri.mjs");
