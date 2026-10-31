# Pairing form correction — 0.1.4

A valid six-digit pairing code was rejected by browser constraint validation before the pairing command ran. The Svelte attribute `pattern="[0-9]{6}"` interpolated the braces, producing the wrong pattern. The corrected explicit string expression preserves the regular expression quantifier.

A regression test reproduced rejection of a valid code before the fix. After the fix it accepts six digits, including a leading zero, and rejects short or nonnumeric input. All 27 frontend tests passed; Svelte reported no errors or warnings. The production ARM64 debug APK built successfully.

Installed over USB to the connected CPH2747, preserving the existing application ID and app data. Package Manager reports version 0.1.4 / code 1004. The app launched into its pairing screen without a startup error. This verifies installation and form validation; a successful TV pairing and subsequent ADB connection remain for the physical-device test.

Artifact: `artifacts/Bridgecoil-0.1.4-arm64.apk`  
Size: 45,281,600 bytes  
SHA-256: `a4d9298ebb791248065aa09a171b9ed9936882dc43164b987819a0f26b1434f4`

No TV pairing code was retained in source, tests, or these notes. No TV commands or automatic connection attempts were performed during this fix.
