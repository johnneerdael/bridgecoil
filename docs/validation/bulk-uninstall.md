# Bulk uninstall and app-list continuity

2 November 2026. Uses the existing native batch engine; no live TV applications were deleted during this change.

- Frontend regression tests cover preparing 20 uninstalls once, reviewing all candidates together with one confirmation, the 32-app selection limit, single-uninstall list identity/depth/position, partial-batch selection and refresh failure/retry.
- The Rust integration test uninstalls 20 simulated packages once, verifies every result and History record, and excludes system apps and the active launcher. Existing cancellation, stop-after-failure and single-uninstall checks also pass.
- The browser fixture at 390 × 844 and 1280 × 900 fits selection and a 20-app review without horizontal overflow. The fixture removes selected apps after the single batch confirmation and retains selection mode.
- In a real browser layout with 64 loaded fixture rows, uninstalling the first visible app retained the same list element and all 64 rows of browsing depth. Scroll changed from 1200 to 1140 to compensate for the deleted row; the next surviving app's viewport offset changed by **0 pixels**.
- Independent review caught pagination inconsistency following refresh failure. The fix retains loaded state and blocks incremental paging until an explicit refresh retry succeeds; a regression changes the underlying package list before failure and checks that no later app is skipped.

Browser fixture checks prove UI behavior; the Rust driver simulates Android shell responses. Physical Android/iPhone/iPad acceptance with a real TV remains separate.
