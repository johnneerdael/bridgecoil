# App list and first-tap interaction — 0.1.5

The Apps screen now keeps its heading, filter, install action, and search controls fixed. Only the app-list region scrolls, with additional batches of up to 32 apps appended near its end. Previous/Next navigation is removed from the inventory. Search/filter changes reset the list; stale batch responses are discarded. A failed extra batch leaves existing rows available and provides retry. The duplicate connection-success banner is removed because the header already shows connection state.

Touch reproduction before the fix showed pointerdown/pointerup on Pair TV followed by input blur and a click on the form instead of its submit button: blur immediately expanded the layout and moved the button. While editing an input, primary pointer presses on buttons now keep input focus through activation. Normal click/submit dispatch remains responsible for the action, and keyboard activation is unchanged. This also covers Search and other buttons affected by the same layout change. A browser touch check against the updated source observed a click on Connect followed by form submission on the first touch.

Validation: all 30 frontend tests passed, Svelte reported zero errors/warnings, and the production ARM64 debug APK built successfully. Tests cover incremental list appending, preserving existing rows, ignoring stale batches after filter changes, and preserving input focus during Pair/Connect presses.

Browser fixtures with 205 apps verified a 563px-high list with 1919px content, then 64 loaded rows after scrolling. Header/control rectangles stayed identical, document scroll remained zero, and document height equalled the viewport (844px). Screenshots were visually checked in light mode. This is fixture UI evidence, not a fresh TV test.

Installed over USB to the connected CPH2747; Package Manager readback reports 0.1.5 / code 1005. Existing application identity and app data were preserved. TV pairing/connection success on 0.1.4 was reported by the user. Agent actions did not connect to or change the TV during this update.

Artifact: `artifacts/Bridgecoil-0.1.5-arm64.apk`  
Size: 45,281,356 bytes  
SHA-256: `0fc5ea5ae17f16569f7ceb63aaaaaacb40bad9e34a46d007688b5a2fd5ccceb4`
