# APK metadata fixtures

Both fixtures were generated locally from a manifest with package `nl.neerdael.adbstudio.fixture` and `android:hasCode="false"`. They contain no private app code and are unsigned; they are parser fixtures, not installable signed releases.

`metadata.apk`: AAPT2-built binary manifest, version code 7, version name 0.0.7, min SDK 23. Compiled with Android SDK build-tools 36.1.0 and platform android-35.

`long-name.apk`: Same minimal shape with version code 8 and a 260-character ASCII version name. AAPT2's string pool was losslessly re-encoded to the Android UTF-8 pool format with two-byte length prefixes, preserving indices and chunk sizes. `aapt2 dump xmltree --file AndroidManifest.xml tests/fixtures/long-name.apk` independently decodes the result. It detects the previous decoder's rejection of valid long UTF-8 strings; the original AAPT2 UTF-16 encoding alone did not reproduce that failure.

Adversarial declared chunk sizes, counts, offsets and truncated input are generated in Rust tests, without executing an allocation-bomb input against the former unbounded decoder.
