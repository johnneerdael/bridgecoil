# Exploration verification

30 October 2026. These checks concern the standalone mockup only.

- Opened the review board in Chrome and visually inspected the desktop composition, device overview, disable review and mapping review.
- Checked a 390 × 844 browser viewport. Refined focused-preview sizing so the bottom navigation remains visible. Measured document width equal to viewport width, and the app content width equal to its scroll width; no horizontal overflow in the inspected state. This is browser emulation, not native Android or iOS validation.
- Clicked Apps → sample app → disable review → sample result → History. Confirmed the simulated change and recovery action appear.
- Clicked pairing discovery → code entry. Confirmed an empty code produces feedback; entering six sample digits advances to a separate connection step with a different example port. No pairing traffic is sent.
- Clicked remote learning → listening preview → simulated key → installed app → mapping review. Confirmed Android key identity/source and TV-helper setup are visible.
- Opened stock launcher controls before testing a replacement. Confirmed the disable control is disabled.
- Saved `workspace-preview.png`, `mobile-preview.png` and `disable-review.png` from rendered browser views. The disable image predates a sample-state bookkeeping refinement; its visual layout is unchanged.
- Restored the browser's viewport override and kept the review tab open as the deliverable.

Not verified: native file-picker integration, APK parsing or installation, physical devices, ADB traffic, direct pairing, helper behavior, firmware support, every mockup branch, screen reader behavior or a full accessibility audit. No application test suite or Tauri build exists yet.

Navigation B was selected by the user after this verification. The later “looks good” accepted the revised direction and reviewed toolkit mockups for planning; physical validation and detailed implementation remain outstanding.

## Navigation comparison follow-up

- Added interactive A (Tasks / All tasks / History) and C (Remote / Apps / Customize / More), sharing the same sample workflows with B.
- Clicked A’s Install journey to the APK-selection sample and opened its complete task catalog.
- Clicked C’s More destination and confirmed Files, Diagnostics, All tools, Shell and History are discoverable there.
- Checked C at 390 × 844: document and viewport width both 390, app content and client width both 362; bottom navigation was visible. Saved alternative screenshots. A’s programmatic measurement was taken after focus-induced page scrolling and does not establish viewport positioning.
- Restored the normal viewport and direction B.

## Selected B workflow follow-up

- Recorded the user’s selection of B without treating it as full design approval.
- Added and reviewed the full product design draft; a fresh builder-check reviewer reported no important unresolved planning questions. See `BUILDER-REVIEW.md` for scope.
- Clicked bulk app selection → disable review → simulated partial results. Confirmed distinct per-package outcomes and stopped-batch behavior are described. Saved `batch-results-mobile.png` at 390 × 844. These are simulated outcomes, not device results.
- Clicked mobile network-access explanation → permission-denied recovery. Verified the mockup distinguishes local permission denial from a TV being offline.
- Opened the sample signing-identity conflict. Confirmed the interface preserves the installed app and offers another file or return to Apps, with no automatic uninstall action.
- Ran JavaScript syntax validation after the changes. All added state is confined to the mockup; no ADB execution or native app integration exists.

## Shizuku / Termux clarification and setup revision

- Read the saved TCL mapping, Termux boot-monitor and shell-wrapper files without connecting to the TV. Confirmed the saved path uses a background getevent listener launched through local ADB; it is not a dedicated remapper APK or Android AccessibilityService. This is static evidence, not proof of current TV processes or reboot behavior.
- Recorded user acceptance of TV-side Shizuku and rish/Termux dependencies and guided installation. Replaced the blanket companion/service requirement with detected native mappings or an explicitly described script/runtime path.
- Added TV setup screens: inspect/reuse tools, missing-tool installation review, TV authorization, startup-plan review, and separate physical-key / phone-disconnected / cold-boot test states. No tool was downloaded or installed onto a TV.
- Clicked through that mockup sequence. Save was initially disabled; after two simulated key/phone tests it became available while cold-boot verification remained pending. The saved mapping still displayed cold-boot test pending. Changing the selected mapping resets test state.
- The fresh builder review of the revised specification returned no important unresolved planning questions. See `BUILDER-REVIEW.md` for limits.

## Acceptance and planning handoff

- User replied “looks good” to the revised design and TV-toolkit setup. Recorded acceptance for planning without claiming any runtime feature is implemented.
- Wrote and self-reviewed `../docs/superpowers/plans/2026-10-30-adb-workspace.md`, preserving the full 17-task scope and the direct-Android/iOS validation gate. Execution method is awaiting user selection as required by the writing-plans skill.
- Inspected available local toolchains: Node 22.22.2, npm 12.2.1, Rust/Cargo 1.99.0, Xcode 26.3, Android SDK directory present. These are environment observations, not build or device-test results.
- Inspected the published libadb 0.7.0 source as an additional direct TLS/pairing candidate. No code was compiled or integrated. No tests, builds, pairing or TV changes were run during plan writing.
