# Compact shell and full workspace execution

User-directed sequence: fix the photographed UI, build a new APK, then immediately continue the full approved implementation. Do not wait for another design approval or a live TV pairing test. Test the app on a dedicated emulator without using the app to pair/connect to an ADB target; live target testing comes after implementation is finished.

## Immediate UI milestone

- Replace Early build badge with connection status and remove the separate device strip.
- Pin header/page heading; replace bottom Connect/Install tabs with a dismissible hamburger navigation drawer that can grow with the full workspace. Keep normal connection/install screens within the viewport.
- Replace oversized promotional headings with compact task titles.
- Open debugging options in a dismissible dialog; put lengthy device/file detail there as needed.
- Respect system-bar/keyboard insets and contrast on Android.
- Run component/layout checks and dedicated emulator UI checks, then build/export a versioned 0.1.1 APK.

## Full implementation, immediately afterward

Resume the full plan at `2026-10-30-adb-workspace.md` Tasks 3–17, preserving the original five destinations and full capabilities. Existing POC transport/picker/install code is the foundation, not the finished product. Prioritize target-scoped operations/history, package browsing/actions, native file workflows, remote key learning/mappings, launcher flow, accepted Shizuku/Termux setup/startup, Files/Capture/Diagnostics/Settings/Network/Shell/Sequences, and cross-platform validation/documentation.

Use pure/fixture and native IPC component tests for target-dependent behavior. Keep production disconnected state truthful. Emulator testing must not connect the app to a live ADB target or change the user's TV/phone. Lists may scroll inside a bounded workspace while headings remain pinned; the document/page itself must not scroll. Long help/detail content belongs in dismissible dialogs.

User clarification: Brand kit will follow later. Preserve current fonts/colors as replaceable tokens; prioritize page geometry and navigation. Use pagination/drill-down for long lists rather than vertically expanding pages.
