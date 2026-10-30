# Compact 0.1.1 UI milestone

User corrections: no early-build badge or separate device strip; connection status in a compact pinned header; hamburger navigation; compact task titles; dismissible debugging help; fixed-screen pages instead of document scrolling. Keep fonts/colors as replaceable tokens for the later brand kit.

Implemented and built Android ARM64 0.1.1. Nine frontend tests and Svelte/TypeScript checks pass. A dedicated fresh API-36 emulator `adb_studio_ui_api36` / emulator-5592 was used; no app pairing/connection request was sent to any ADB target. The user's USB phone and TCL were not modified.

Native emulator checks: all pairing controls visible on normal Pixel 7 viewport and a 360×640 display; dark system status-bar icons; help dialog opens, displays all paragraphs and closes; drawer opens and selects Install; disconnected installation view fits; numeric-keyboard-open pairing controls remain within the application area and no WebView scrolling node is exposed. Screenshots are in `compact-0.1.1/`.

The data-intensive full workspace will use bounded, paginated content and drill-down views under this shell. This milestone does not mean the full backend/toolkit is implemented. Work continues under `2026-10-30-adb-workspace.md`.

The emulator's initial boot failed because its system image enforced a 6 GiB userdata partition and the host had insufficient free space. All retained POC artifacts were checksum-verified, then only this worktree's generated target directory was cleaned. Development debug information was disabled and crypto arithmetic optimized, reducing rebuild footprint and the new APK to approximately 70 MiB. No other checkout or global cache was cleaned.
