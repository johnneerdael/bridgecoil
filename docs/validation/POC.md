# Connection and APK installation POC validation

Scope: the user explicitly prioritized an early pairing/connection/native-picker installation POC. The full approved workspace remains on the roadmap. The user asked to perform TCL testing **after the POC is finished**, so no current TV pairing, package installation or TV settings changes were attempted.

## Implemented boundary

Direct Rust pairing and separate connection endpoint; persistent app-private host identity; TLS with explicitly approved legacy fallback; native picker → bounded private staging → binary manifest metadata/fingerprint review → streamed sync upload → Android install/update for Owner/user 0 → installed-version readback. No external ADB executable or desktop server is used by the app.

## Checks performed

| Check | Result | Scope |
|---|---|---|
| `npm run check` | Pass, no Svelte/TypeScript diagnostics | Source/type checks |
| `npm test` | 6 passing tests | Browser honesty, eligibility/stale target, duplicate eligibility, error formatting and mocked-native LAN denial |
| `cargo test --workspace` | 22 passing tests | Endpoint/code validation, host identity, private permissions, real binary APK metadata, bounded malformed AXML cases, long UTF-8/UTF-16 handling, independent raw TCP upload/install/readback, interrupted unknown outcome, LAN-denial result |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass | Rust application and tests |
| Frontend production build | Pass | Packaged web assets |
| macOS debug `.app` | Built | Apple Silicon desktop bundle; native UI launch automation unavailable |
| Android ARM64 debug APK | Built | SDK target 37, NDK 28.2.13676358, JDK 17; includes native LAN permission check/request |
| iOS ARM64 simulator `.app` | Built unsigned | Xcode 26.3; not a physical-device IPA |
| Browser render at 390px | Inspected | No horizontal overflow; vertical scrolling is intentional; native-only operations visibly unavailable |

Unit/integration tests were observed failing before corresponding implementation/fixes. The wire peer is an independent minimal test implementation, not Android's package installer or a physical TV. Its installation result cannot establish physical TV compatibility or actual pairing interoperability.

## Independent review and fix pass

A fresh read-only reviewer inspected implementation commit `53b38ca` and independently reran the original 15 Rust and five frontend tests. It found no Critical issues and three Important issues. The single fix pass addressed all three:

1. **Unchecked third-party manifest allocation:** removed axmldecoder. The metadata-only reader checks chunk lengths, count/offset tables, per-string limits, selected attributes and parser-work bounds before allocating. Malformed size/count/offset/truncation cases return errors. Allocation-bomb input was not run against the former unsafe parser.
2. **Long UTF-8 strings:** the AAPT-built long-name fixture was losslessly re-encoded to valid Android UTF-8 string-pool encoding and independently decoded by AAPT2. Its regression failed with the old decoder and passes with the bounded reader. The normal AAPT UTF-16 fixture also passes.
3. **Android 17 LAN permission:** target-37 manifest declares ACCESS_LOCAL_NETWORK. The private native plugin checks/requests access before Rust pair/connect calls, skips the new request on older OS versions, and returns actionable denial guidance. The native denial UI regression was observed failing and passing; platform code compiled into the Android APK. The actual Android 17 system prompt still needs on-device validation.

A wire-fixture teardown race also surfaced under the full suite: the peer could send its final close acknowledgement after the production client disconnected, causing a fixture panic on a normal TCP reset. Only that final close acknowledgement now tolerates reset/broken pipe; upload, install count and readback assertions remain strict. The complete suite then passed.

No second reviewer was dispatched after fixes; regression checks and the complete green suite are the fix evidence.

## Deferred minor findings

- More native IPC component tests for connect/install failures, duplicate invocation and stale response ordering. Current tests cover the model/guard and the LAN-denial native-error path, not every native UI lifecycle permutation.
- Friendlier unsupported-target copy for missing shell_v2. Legacy transport currently still requires shell_v2; this limitation is documented in README.

## Limits and deliberately unperformed checks

- Fresh pairing, target authentication prompts, real package install and lifecycle behavior on the user's TCL/phones: deferred by user until delivery.
- iOS physical signing/provisioning and local-network permission behavior: not validated. The simulator bundle is not an IPA.
- iOS simulator UI automation: XcodeBuildMCP is not connected. The test-xcode skill says “Do NOT proceed until XcodeBuildMCP is confirmed working”; its UI workflow was not run. This does not affect the independently completed Tauri simulator build.
- Native desktop screenshot automation failed to start its native pipe. The interface was inspected using the browser preview, with no fake native results.
- Hardware-backed key vault/backup policy, independent cryptographic authentication of TV certificates, automatic discovery, split APKs, all Android ABIs and broader toolkit features remain outside this first POC.

## Environment notes

Homebrew Rust shadowed rustup inside npm scripts, despite installed rustup mobile targets. `scripts/tauri.mjs` resolves the actual rustup compiler/cargo paths and passes them explicitly without changing system defaults. The native build commands then succeeded.

The machine's Cargo registry auto-clean emitted a permission warning for an unrelated cached yoke source file. Existing user npm configuration emitted an unknown allow-scripts warning. These did not fail compilation/tests; no global cache ownership or user configuration was changed. Generated Android tooling also emits upstream Gradle/Kotlin deprecation warnings.

## Decisions and their consequences

- Prioritize pair/connect/install and defer the broad toolkit per user instruction. This delivers a useful early POC, not the completed full product.
- Execute in one session, with a fresh independent review, following the earlier recommendation and approval to proceed.
- Keep file selection handles opaque and session-bound. A changed connection requires selecting/reviewing the APK again.
- Allow POC delivery before physical TCL acceptance because the user explicitly postponed that test; runtime claims remain limited accordingly.
- Use a bounded metadata-only parser rather than a general resource/XML decoder. Unsupported/oversized metadata returns a readable error instead of attempting full resource resolution.
- Retain SDK 37 with explicit native LAN permission handling, rather than lowering the target to avoid the requirement. System-prompt behavior still needs device testing.
