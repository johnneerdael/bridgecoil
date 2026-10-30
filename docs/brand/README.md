# Bridgecoil appearance contract

Use the user-supplied Bridgecoil Brand Kit 1.0 (archive digest and imported assets in `assets.json`). The brand kit governs public display naming, fonts, colors, logos and platform icons. The existing product decisions govern layout and navigation: hamburger menu, pinned compact header, connection badge, no separate device strip or permanent bottom tabs, bounded page content with pagination/details rather than document scrolling.

`src/lib/theme/tokens.css` is the supplied semantic palette. Inter serves body/control text, Manrope headings, and IBM Plex Mono technical values. WOFF fonts and their SIL Open Font Licenses ship in `public/brand/fonts`. Wordmarks use supplied outlined SVGs. System appearance is the default, with explicit Light/Dark options in the drawer. Android status/navigation bars and desktop decorations follow the selected appearance. Native iOS decoration overrides remain to be implemented and tested.

The display name is Bridgecoil. Keep the existing `nl.neerdael.adbstudio.poc` identifier and private identity storage for upgrade/pairing continuity. Historical POC documents and artifact names retain their original names as evidence.

Validation: frontend check/build and 11 tests passed; Android ARM64 debug APK built; light connection form and dark drawer/system bars inspected on the dedicated API36 emulator at 360x640. See `../validation/bridgecoil-brand/`. No app connection to a TV or live ADB target was made. This milestone does not claim full implementation or transport acceptance.
