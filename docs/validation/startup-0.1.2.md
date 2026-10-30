# Android startup origin race — 0.1.2

The user reported a physical-phone startup error: event.listen was rejected by the main-window capability because the IPC origin was still about:blank. Wry's Android RustWebViewClient initializes currentUrl to about:blank, updates it in onPageStarted, and Ipc.postMessage passes that value to native permission checks. Frontend document startup can reach IPC before that Android callback establishes the app URL.

Bridgecoil now completes a read-only native connection-status handshake before registering installation progress. It retries only the explicit about:blank origin rejection, at most eight attempts and under three seconds of scheduled waiting. Genuine permission errors remain visible. Aborting startup releases any listener registered during teardown. Native mutations stay guarded until initialization succeeds. The capability file and remote-origin permissions were not broadened.

Validation: four startup regression tests (transient blank origin, real permission denial, retry bound and teardown cleanup), original LAN-denial flow and the full frontend suite passed (19 tests); type check/build passed. The Android ARM64 debug APK built and was installed over USB at the user's request. A force-stop followed by start reported COLD and Status ok. The loaded phone screen was inspected without the startup error; the user separately confirmed no error. Phone screenshots remain local because they include private status-bar notifications.

The artifact and digest are retained in artifacts/startup-0.1.2.json. User TV pairing/install testing is pending feedback. No agent-driven TV connection or mutation was performed. This validates the observed startup failure on the tested phone, not full device/platform acceptance.
