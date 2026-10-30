# ADB Device Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the approved mobile-first, cross-platform ADB workspace with direct Android/iOS connections, a broad visual toolkit, physical-key mapping and guided TV-side setup.

**Architecture:** Share typed device operations and direct ADB transport in Rust; keep presentation in a Svelte/TypeScript Tauri 2 frontend. Use small native Android/iOS adapters for discovery, permissions, protected identity storage and native file access. Validate direct pairing first; then build the five destinations on the same device/profile and operation model.

**Tech Stack:** Tauri 2, Rust workspace, Tokio, Svelte 5, TypeScript, Vite, Vitest, Svelte Testing Library, SQLite for local records, native Kotlin/Swift adapters. Evaluate exact-pinned `libadb = 0.7.0` with direct TLS/pairing features before adopting it. Use npm and committed lockfiles; no external ADB executable on the mobile production path.

**Spec:** `docs/superpowers/specs/2026-10-30-adb-workspace-design.md`. Read it alongside `design/review.html`, `design/RESEARCH.md` and the research addendum below. The user approved the revised design direction and reviewed mockups with “looks good”; runtime compatibility is still unproven.

## Global Constraints

- Tauri 2 and a Rust backend; Android and iOS from the first release. Preserve desktop support.
- Navigation: Device, Apps, Remote, Tools and History. Ordinary tasks must not require command/package/activity typing.
- A supported modern TV must pair directly from a phone without a laptop, desktop server or pre-seeded trusted key.
- Shizuku and rish/Termux may be required on the controlled TV, with guided installation and configuration; do not require them on an iOS phone.
- Distinguish native firmware mappings, TV-side shell listeners and optional helper apps. Do not mandate a new remapper APK by default.
- Installed, authorized, running, startup configured, phone-independent and reboot-verified are separate states.
- Read-only browsing and routine remote taps do not require confirmation. Meaningful changes receive targeted reviews.
- Unknown, unsupported, needs setup and available are distinct. Requested, running, verified, partial, failed and unknown results are distinct.
- Preserve device identity/profile and operation target across reconnects. Never silently replay interrupted mutations.
- No cloud account/relay or telemetry is required. No publishing, remote repository creation or release is included without a later request.
- Use at least 44 × 44 logical-pixel touch targets, text labels, visible keyboard focus, sufficient contrast, reduced motion and large-text/RTL validation.
- Reuse existing compatible TV tools and user configuration. Do not silently uninstall conflicting packages, disable original button targets or add SSH exposure.
- No rooting, authorization bypass or universal remapping/backup/reboot claims. APK reinstall does not restore deleted app data.

## Review Focus

1. A TV address changes owners or profile changes during review: refuse to apply a prepared change to the new target (Tasks 3, 6).
2. An install completes just as a phone is backgrounded: show an unknown result until reconnection verifies package/version; do not install twice (Tasks 3, 8).
3. A vendor reports `KEY_UNKNOWN` for several physical buttons: preserve source and scan namespace; never collapse all keys into one mapping (Task 12).
4. An existing Termux add-on or Shizuku variant conflicts with the proposed package: preserve it and its data; show a separate compatibility decision (Task 11).
5. Reboot automation works over Wi-Fi but not Ethernet, or the TV only wakes from standby: retain scoped startup evidence instead of showing a universal reboot-ready badge (Tasks 13, 17).

## Delivery structure

The subsystems share device identities, operation results and TV capabilities, so use one phased plan with independently reviewable tasks. Finish the transport validation gate before implementing broad device-changing features. Do not redefine the project as a connection demo if a gate fails.

| Phase | Tasks | Exit condition |
|---|---|---|
| Direct mobile feasibility | 1–2 | Direct pairing, connection, shell and transfer demonstrated on physical Android and iOS |
| Shared product foundation | 3–6 | Real five-destination UI, identities, capability/results model and operation history |
| Apps and daily control | 7–10 | Picker installation, package changes, remote controls and launcher recovery |
| Persistent mappings and TV setup | 11–13 | Guided dependencies, real key learning, mapped apps and separately tested startup |
| Wide toolkit and release acceptance | 14–17 | Visual settings/files/diagnostics/advanced tools, desktop adaptation and platform evidence |

## Planned file boundaries

No application exists yet. Preserve `design/` and `docs/` when initializing the project.

| Paths | Responsibility |
|---|---|
| `Cargo.toml`, `crates/adb-core/` | Shared Rust workspace and platform-independent domain/transport |
| `crates/adb-core/src/transport/` | Direct pairing/authentication, sessions, shell and sync adaptation |
| `crates/adb-core/src/{device,operation,apps,remote,launcher,toolkit,files,diagnostics,network,settings}/` | Focused domain modules; split by actual feature, not speculative service hierarchies |
| `src-tauri/src/{lib,commands,state}.rs` | Tauri host, bounded IPC and app state; delegate domain work to core |
| `src-tauri/permissions/`, `src-tauri/capabilities/` | Explicit permitted IPC, picker/storage scopes and mobile access |
| `src-tauri/gen/android/`, `src-tauri/gen/apple/` | Generated native projects plus versioned native adapters/configuration |
| `src/lib/platform/` | Typed frontend/native interface and picker/discovery result normalization |
| `src/lib/features/` | Svelte feature screens colocated with feature state, types and component tests |
| `src/lib/components/`, `src/lib/theme/` | Shared reviewed controls, typography, spacing and accessible state styles |
| `assets/tv-runtime/` | Audited mapping listener/configuration and startup integration templates |
| `tests/fixtures/`, `tests/e2e/` | Synthetic protocol/metadata fixtures and complete UI journeys |
| `docs/validation/` | Reproducible test commands, environment, actual outcomes and remaining limits |

Create feature directories only when implementing the task that owns them. Do not replace the working HTML mockup by shipping its simulated state as a production backend.

## Shared interfaces

Define these in `crates/adb-core/src/device/types.rs`, `operation/types.rs` and `transport/mod.rs`; export serialized equivalents to `src/lib/platform/types.ts` in Task 3:

```rust
struct DeviceId(String);
struct ProfileId(u32);
struct SessionId(uuid::Uuid);
struct Target { device: DeviceId, profile: ProfileId, session: SessionId }
struct Endpoint { host: String, port: std::num::NonZeroU16 }
enum CapabilityState { Available, NeedsSetup, Unsupported, NotChecked }
enum OperationStatus { Queued, Running, AwaitingUser, Verified, Failed, Partial, Unknown }
struct OperationId(uuid::Uuid);
struct ShellResult { stdout: Vec<u8>, stderr: Vec<u8>, exit_code: Option<u8> }
```

`DeviceId` is a locally recorded identity associated with pairing/observed device evidence, not the current IP address. Do not describe banner/serial matching as cryptographic server authentication. Define cancellation and limits in `OperationContext { id, cancel, limits }`; all long operations take `&OperationContext`. Represent imported filenames/bytes and Android components as validated types before command construction.

## Task 1: Establish direct Rust transport and an honest validation harness

**Files:** Create `Cargo.toml`, `crates/adb-core/Cargo.toml`, `crates/adb-core/src/{lib,error}.rs`, `crates/adb-core/src/transport/{mod,libadb_backend,identity,endpoint}.rs`, `crates/adb-core/examples/direct_probe.rs`, `crates/adb-core/tests/direct_contract.rs`, `docs/validation/direct-adb.md`.

**Interfaces:** Produce `Endpoint::parse(&str) -> Result<Endpoint>`, `HostIdentityStore::load_or_create() -> Result<HostIdentity>`, and `DirectConnector::{pair,connect}`. `pair(endpoint, code, identity, context) -> PairingReceipt`; `connect(endpoint, identity, context) -> DirectSession`. Pairing and connection are separate calls/endpoints. `DirectSession` offers bounded `shell(command, context) -> ShellResult` and streamed `push/pull`; high-level app operations follow later.

- [ ] Write contract tests `pairing_port_is_not_reused_as_connection_port`, `wrong_code_is_not_reported_connected`, `identity_roundtrip_preserves_public_key`, `disconnect_after_send_is_not_a_retryable_success`, and `command_is_not_sent_after_cancel`. Assert typed outcomes and no unrequested second mutation; use a stub backend at this boundary, not a fake server sharing the production pairing crypto as sole evidence.
- [ ] Run `cargo test -p adb-core --test direct_contract`; observe missing implementation failures before adding the adapter.
- [ ] Implement the direct adapter around exact-pinned `libadb` 0.7.0 if source/interop checks support it. Disable unused USB/default features; explicitly enable the required Tokio, TLS, key and pairing features. Preserve original errors as sources, redact codes/private keys, bound reads and stream memory. Use the library's identity import/export rather than a desktop-home-directory convention.
- [ ] Add a minimal desktop probe using explicit operator-supplied pairing/connection endpoints and a protected local identity store. Never scan/mutate a TV merely because its old address appears in docs. No installed APK or package/settings mutations are part of this probe.
- [ ] Run contract tests and `cargo clippy -p adb-core --all-targets -- -D warnings`. Run a consented physical-TV pairing/shell/temporary-transfer probe when endpoints and TV-side pairing approval are supplied. Record native `adbd` interoperability and compare with AOSP/platform-tools as an independent reference. A crate self-test alone does not establish interoperability.
- [ ] Initialize local Git if still absent, preserve all design/docs, add a task-scoped ignore file, and commit only the inspected task changes. Create no remote or release.

## Task 2: Prove the same direct path in minimal Android and iOS Tauri hosts

**Files:** Create `package.json`, lockfile, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`, `src/{main,App.svelte}`, `src-tauri/{Cargo.toml,tauri.conf.json}`, `src-tauri/src/{lib,commands,state}.rs`, native generated projects/adapters, `docs/validation/mobile-direct-adb.md`.

**Interfaces:** Consume Task 1. Produce IPC `pair_tv(PairInput)`, `connect_tv(ConnectInput)`, `probe_shell(SessionId)`, `probe_transfer(SessionId)` and cancellation; return structured progress/results. Native `IdentityVault` reads/writes the host key through protected app storage, not a user-supplied key path.

- [ ] Add meaningful command-boundary tests for input rejection, code redaction and cancellation; run them failing before the host wiring.
- [ ] Scaffold Tauri 2 with Svelte 5/TypeScript and the already chosen light design tokens. Pin compatible dependencies/lockfiles. Define npm scripts: `check` = `svelte-check --tsconfig ./tsconfig.json`, `test` = `vitest run`, `test:e2e` = `playwright test`, `build` = `vite build`, `tauri` = `tauri`. Add the corresponding development dependencies and an e2e test-target configuration. Configure Android/iOS minimum versions from the selected dependencies' actual support; record the choices rather than guessing from desktop compilation.
- [ ] Implement the minimal manual-address/code harness and platform identity storage. Run network/protocol operations away from the UI thread. Include native local-network permission explanations/recovery and native-service discovery adapters when ready; manual endpoints must still work when discovery is absent.
- [ ] Run `npm run check`, `npm test`, `cargo test --workspace`, `npm run tauri -- android build --help` only to establish CLI arguments, then the actual Android debug build command supported by that installed CLI. Use the same procedure for `tauri ios build`; document exact tested invocations. Help output is not a build result.
- [ ] On physical Android and physical iOS, pair a fresh host identity without a desktop server, connect via the separate endpoint, run a harmless shell command, transfer/read back/delete only the probe's own temporary file, background/foreground and reconnect. Test wrong/expired code, revoked identity and an unavailable target. Record device models/OS/TV build/network, evidence and untested cases. Do not call simulator-only tests physical validation.
- [ ] **Gate:** resolve any failure to achieve direct pairing on both mobile platforms before broad mutation features. Report an actual external blocker if physical devices/signing access are missing. Do not substitute a desktop bridge, pre-seeded key or Android-only implementation. Commit the harness and evidence separately from broad UI work.

## Task 3: Add target-scoped operation state and identity-safe reconnect

**Files:** Create `crates/adb-core/src/device/{mod,types,registry}.rs`, `operation/{mod,types,runner}.rs`, `src/lib/platform/{types,client,events}.ts`, `crates/adb-core/tests/operation_targets.rs`.

**Interfaces:** Produce `DeviceRegistry::resolve(Target) -> Result<SessionHandle>`, `OperationRunner::start(Target, DeviceAction) -> OperationId`, `cancel(OperationId)` and `watch(OperationId) -> stream<OperationEvent>`. Every event carries operation ID and original target. Keep serialized field names consistent at both boundaries.

- [ ] Write failing tests `address_reuse_rejects_old_target`, `profile_switch_invalidates_pending_review`, `old_session_event_does_not_update_new_device`, `disconnect_after_install_send_becomes_unknown`, and `unknown_mutation_is_not_replayed_on_reconnect`.
- [ ] Implement session epochs, explicit target comparison at Apply, bounded event streams and outcome transitions. Reconcile unknown mutations through feature-specific readback, never a generic retry loop.
- [ ] Run `cargo test -p adb-core --test operation_targets` and the IPC tests; verify the target/device/profile/session fields survive serialization and stale events are ignored in frontend tests. Commit the shared contract.

**Deterministic verification:** `cargo test -p adb-core --test operation_targets` and `npm test -- src/lib/platform`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 4: Build the responsive five-destination shell

**Files:** Create `src/lib/{theme/tokens.css,components/AppShell.svelte,components/DeviceHeader.svelte}`, `src/lib/features/navigation/{navigation.ts,navigation.test.ts}`, `src/lib/components/AppShell.test.ts`; replace the harness-only entry screen while preserving its diagnostic route for development.

**Interfaces:** Consume typed platform client. Produce `NavigationState { destination, target, origin, listContext }` with destinations exactly Device/Apps/Remote/Tools/History. Retain filter and scroll context on Back; switching target discards prepared mutations.

- [ ] Add failing component tests for target labels, preserved Back/list context, unsaved target-switch review, keyboard navigation and readable disconnected/stale state.
- [ ] Implement mobile bottom navigation and tablet/desktop rail, safe-area handling, reviewed palette, responsive list/detail placement and accessible reusable controls. Icons supplement labels. No mock sample device is presented as a live TV.
- [ ] Run frontend checks/component tests, inspect real renders at 390px, tablet and desktop widths with large text/reduced motion, and capture representative screenshots. Commit after resolving clipped controls, focus traps or inaccessible labels.

**Deterministic verification:** `npm run check` and `npm test -- src/lib/components src/lib/features/navigation`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 5: Complete Device setup, discovery and capability inspection

**Files:** Create `src/lib/features/device/{DeviceScreen,ConnectionFlow,CapabilityList,ProfilePicker}.svelte`, `crates/adb-core/src/device/{capabilities,profiles}.rs`, native discovery adapters, `tests/e2e/connection.spec.ts`.

**Interfaces:** Produce `discover_adb() -> stream<DiscoveredEndpoint>`, `inspect_device(SessionId) -> DeviceSnapshot`, `list_profiles(SessionId) -> Vec<AndroidProfile>` and capability results with reason/evidence timestamp. Discovery distinguishes legacy, pairing and connection services.

- [ ] Add tests for empty discovery/manual fallback, permission denial rather than offline, separate pairing/connection ports, changed port, profile unavailability, and unknown capability display. Use independent network-adapter stubs, not fake successful native APIs in production.
- [ ] Implement saved devices, TV setup guide, native access prompt/recovery, explicit pairing, TV approval/connection, profile selection, identity mismatch, capability inspection and truthful model/storage/status values.
- [ ] Run connection flow tests and native discovery/permission checks on both physical phone platforms. Verify denial/re-enable and changed endpoint recovery. Commit UI and adapter behavior with scoped evidence.

**Deterministic verification:** `cargo test -p adb-core device` and `npm run test:e2e -- tests/e2e/connection.spec.ts`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 6: Persist operation history and supported recovery

**Files:** Create `crates/adb-core/src/operation/{history,recovery}.rs`, `src/lib/features/history/{HistoryScreen,OperationDetail}.svelte`, schema/migrations under `src-tauri/migrations/`, `crates/adb-core/tests/history_recovery.rs`.

**Interfaces:** Produce `HistoryStore::{append,list,get,clear}` and `plan_recovery(OperationId, CurrentSnapshot) -> RecoveryPlan`. Recovery is a new target-checked operation; arbitrary shell/uninstall/data deletion are not inherently undoable.

- [ ] Test process restart with a running mutation (reopens Unknown), profile/device mismatch, inverse no longer applicable, redacted output, and clearing local records without executing TV changes.
- [ ] Implement transactional records, result filters, raw-detail expansion, configurable retention and selected-record native export/share review. Keep pairing codes/keys out of logs and database.
- [ ] Run history/migration tests with reopen/upgrade fixtures; inspect the recovery UX against current state. Commit before domain features rely on the record format.

**Deterministic verification:** `cargo test -p adb-core --test history_recovery` and `npm test -- src/lib/features/history`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 7: Implement package browsing, details and reviewed state changes

**Files:** Create `crates/adb-core/src/apps/{mod,inventory,metadata,actions,permissions}.rs`, `crates/adb-core/tests/apps_inventory.rs`, `src/lib/features/apps/{AppsScreen,AppDetail,AppActionReview,BulkReview}.svelte`, fixture-backed parser and operation tests.

**Interfaces:** Produce `list_packages(Target, PackageFilter) -> stream<PackageRecord>`, `inspect_package(Target, PackageId) -> PackageDetail`, and `plan_package_action(Target, PackageAction) -> PackageActionPlan`. Validate `PackageId`; derive launchable components rather than asking for typed activity names.

- [ ] Test progressive metadata failures with package-ID fallback, no launch activity, disabled vs stopped vs removed-for-profile, OEM parser variants, shell metacharacters in untrusted metadata, active-launcher protection and bulk partial outcomes. Pin data-loss semantics separately from inverse availability.
- [ ] Implement All/User/System/Disabled/profile-removed filters, icon/name enrichment, explicit force-stop/enable/disable/remove/reinstall/clear-cache/data reviews, permissions/app-op inspection/change and activity/app-link controls. Show unavailable values honestly; never guess a safe debloat list.
- [ ] Implement per-item bulk plans, explicit exclusion reasons, stop-after-failure policy and verification/history; do not claim transactions across package commands.
- [ ] Run Rust parser/action tests and the no-search Apps UI journey. Perform only explicitly reviewed package mutations on an authorized test TV/fixture app, verify readback and recovery. Commit with real vs synthetic evidence separated.

**Deterministic verification:** `cargo test -p adb-core --test apps_inventory` and `npm test -- src/lib/features/apps`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 8: Deliver native-picker APK and split-set installation

**Files:** Create `crates/adb-core/src/apps/{apk,install}.rs`, `crates/adb-core/tests/apk_install.rs`, native picker/stream adapters, `src/lib/features/apps/{InstallPicker,InstallReview,InstallProgress}.svelte`, `tests/fixtures/apks/` using own generated test APKs and fixture descriptions.

**Interfaces:** Produce `inspect_install(InputSet, TargetSnapshot) -> InstallPlan`, `install(Target, InstallPlan, context) -> OperationId`, and `reconcile_install(Target, ExpectedPackage) -> InstallState`. `InputSet` carries openable handles/streams, not guessed phone filesystem paths.

- [ ] Test missing/inconsistent splits, corrupt APK, incompatible ABI/SDK, signature mismatch, downgrade, same-package update, a URI permission revoked mid-read and disconnect/background after install completion. Assert no automatic uninstall or duplicate install retry.
- [ ] Implement single/multi-picker access, manifest/signing compatibility review, bounded staged/native-stream handling, install-session commit and package/version readback. Reject direct AAB installation; explain unsupported containers rather than treating extensions as metadata.
- [ ] Run fixture/parser/flow tests, then native picker and real test-APK install/update on Android and iOS controllers. Verify cancelled/unknown/reconciled outcomes and safe temporary-file cleanup. Commit only distributable synthetic fixtures and code, not private APKs.

**Deterministic verification:** `cargo test -p adb-core --test apk_install` and `npm test -- src/lib/features/apps`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 9: Add daily Remote controls and app shortcuts

**Files:** Create `crates/adb-core/src/remote/{mod,control,text}.rs`, `crates/adb-core/tests/remote_controls.rs`, `src/lib/features/remote/{RemoteScreen,TextEntry,AppShortcuts}.svelte`, `tests/e2e/remote-controls.spec.ts`.

**Interfaces:** Produce `send_input(Target, RemoteAction, context)` for supported D-pad/media/volume/Home/Back/Menu and `send_text(Target, TextInput, context)`. App shortcuts use discovered `LaunchableComponent` values.

- [ ] Test bounded rapid/repeated presses, disconnected target, Unicode/unsupported text path, escaping, cancelling a held gesture, and visible sent-vs-confirmed feedback. A successful write does not prove TV focus changed.
- [ ] Implement the approved touch layout, capability-specific disabled/explained controls, text sheet and browsable installed-app picker.
- [ ] Run UI/input tests and physical remote-control checks on the target TVs. Verify Home/Back still behave and inaccessible keys do not silently fall through to arbitrary commands. Commit with documented text limitations.

**Deterministic verification:** `cargo test -p adb-core --test remote_controls` and `npm run test:e2e -- tests/e2e/remote-controls.spec.ts`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 10: Implement launcher selection, Home verification and recovery

**Files:** Create `crates/adb-core/src/launcher/{mod,inventory,workflow}.rs`, `src/lib/features/tools/launcher/{LauncherScreen,LauncherTest,StockLauncherReview}.svelte`, `crates/adb-core/tests/launcher_flow.rs`.

**Interfaces:** Produce `eligible_homes(Target)`, `plan_launcher_change(Target, HomeComponent) -> LauncherPlan`, `verify_home(Target) -> HomeEvidence`, `plan_stock_disable(Target, HomeEvidence)` and recovery through Task 6.

- [ ] Test no replacement HOME activity, OEM restriction, failed launch/default request, stale Home evidence after target/profile change, disconnect before stock disable and recovery with stock already disabled.
- [ ] Implement launch test → default request/TV chooser → physical-Home confirmation → separate optional stock disable. Store prior default/package state, show impact and prepare an actual available inverse.
- [ ] Run workflow/UI tests and explicitly reviewed device tests. Do not install timer rollback without a dependable tested TV-side mechanism. Commit with OEM limits and recovery evidence.

**Deterministic verification:** `cargo test -p adb-core --test launcher_flow` and `npm test -- src/lib/features/tools/launcher`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 11: Guide existing-tool detection and reviewed TV installation

**Files:** Create `crates/adb-core/src/toolkit/{mod,inventory,install,authorization}.rs`, `src/lib/features/device/toolkit/{ToolkitScreen,ToolInstallReview,AuthorizationGuide}.svelte`, `assets/tool-catalog.json`, `crates/adb-core/tests/toolkit_compatibility.rs`.

**Interfaces:** Produce `inspect_toolkit(Target) -> ToolkitSnapshot`, `plan_tool_install(Target, SelectedArtifacts) -> ToolInstallPlan`, `verify_authorized_shell(Target, ShellRoute) -> PrivilegeEvidence`. `ShellRoute` distinguishes direct ADB, Shizuku/rish and ordinary Termux; root is never inferred from installed package names.

- [ ] Test missing vs installed/not-authorized, compatible existing variants, duplicate Shizuku permission owner, Termux/plugin signing mismatch, download failure and declined TV authorization. Assert no automatic uninstall, data replacement, SSH setup or broad permission grant.
- [ ] Implement official-project source selection or native user-APK picker, artifact/ABI/version/signature review, normal Task 8 installation and guided first-run/authorization/rish setup. Package catalog artifacts are explicit versioned sources with expected identity; no arbitrary remote shell recipe execution.
- [ ] Implement source-compatible Termux/add-on choices and safe inspection of existing tools. Keep genuine OS prompts as TV handoffs; never label their instructions as completed authorization without a probe.
- [ ] Run compatibility tests and guided Android/iOS-controller installation on a reviewed test TV configuration. Preserve existing environment and record exactly what changed. Commit source metadata and setup UI.

**Deterministic verification:** `cargo test -p adb-core --test toolkit_compatibility` and `npm test -- src/lib/features/device/toolkit`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 12: Learn physical keys and implement explicit mapping mechanisms

**Files:** Create `crates/adb-core/src/remote/{capture,key_identity,mappings,native_mapping}.rs`, `assets/tv-runtime/{key-listener.sh,README.md}`, `src/lib/features/remote/{KeyLearn,MappingEditor,MappingReview}.svelte`, raw-event fixtures and `crates/adb-core/tests/key_mapping.rs`.

**Interfaces:** Produce `capture_keys(Target, CaptureOptions, context) -> stream<KeyObservation>`, `KeyIdentity { namespace, code, source, scan }`, `plan_mapping(Target, KeyIdentity, Gesture, LaunchableComponent, MappingMethod) -> MappingPlan`. Methods: positively identified firmware-native, managed shell listener, supported helper integration. Unknown native methods remain unavailable.

- [ ] Test multiple `KEY_UNKNOWN` buttons with distinct scan/source IDs, USB reordering, repeat/down/up events, duplicate sources, timeout, permission denial, conflicting mappings and changed target activity. Assert namespace preservation and no success from a listener animation alone.
- [ ] Implement raw event parsing and a bounded live capture UI. Prefer supported firmware-native mapping adapters when actually verified; do not write guessed OEM settings. Use a validated component picker and reset all mapping-specific tests whenever trigger/gesture/target/method changes.
- [ ] Implement an audited listener/configuration format with validated input-source/key/component fields and no `eval` or imported command snippets. The TCL saved script is a reference, not a universal deployable artifact. Disclose launch-after-original behavior and separately review any proposed original-app disable action; do not call observation general key interception.
- [ ] Run parser/synthetic mapping tests, then physical capture/target launch checks. Verify normal Home/Back/D-pad behavior, single/long/double-press capabilities separately and remove/pause controls. Commit with per-method capability evidence.

**Deterministic verification:** `cargo test -p adb-core --test key_mapping` and `npm test -- src/lib/features/remote`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 13: Configure and verify TV-side startup/recovery

**Files:** Create `crates/adb-core/src/toolkit/{startup,health,cleanup}.rs`, `assets/tv-runtime/{startup-monitor.sh,startup-manifest.json}`, `src/lib/features/device/toolkit/{StartupReview,RuntimeHealth,StartupTests}.svelte`, `crates/adb-core/tests/startup_scope.rs`.

**Interfaces:** Produce `plan_startup(Target, ToolkitSnapshot, MappingPlan) -> StartupPlan`, `inspect_runtime(Target) -> RuntimeHealth`, `record_startup_test(Target, TestKind, Evidence)` and `plan_remove_owned_setup(Target)`. Evidence includes firmware/network mode/cold-boot vs wake and tested runtime identity.

- [ ] Test idempotent startup changes, multiple monitors, preexisting user scripts, privilege loss, changed TLS port, process death, Wi-Fi/Ethernet divergence and standby falsely labelled reboot. Assert removal touches only verified app-owned files/configuration and never stops unrelated user tools.
- [ ] Implement reviewed scoped startup/recovery entries using the chosen supported Termux:Boot/local-ADB or Shizuku route. Keep permission needs explicit and firmware-specific. Avoid perpetual aggressive retries; report stalled authorization/network conditions.
- [ ] Implement separate physical-key, phone-disconnected and cold-boot status fields, logs and recover/re-authorize controls. Permit a mapping to remain active with startup unverified; never show reboot-ready before its specific test passes.
- [ ] Run startup fixture tests and separately reviewed physical tests for phone disconnect, runtime crash/restart and TV cold boot. Record tested network mode; leave other modes unknown. Commit templates and evidence, excluding private keys or copied user configuration.

**Deterministic verification:** `cargo test -p adb-core --test startup_scope` and `npm test -- src/lib/features/device/toolkit`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 14: Complete Files, Capture and Diagnostics interfaces

**Files:** Create `crates/adb-core/src/files/{mod,browse,transfer}.rs`, `diagnostics/{mod,capture,logs,report,processes}.rs`, `crates/adb-core/tests/files_diagnostics.rs`, `src/lib/features/tools/{files,capture,diagnostics}/` screens and focused test files.

**Interfaces:** Produce `browse_files(Target, RemotePath)`, `transfer(Target, TransferPlan, context)`, `capture_screen(Target, CaptureOptions, context)`, `stream_logs(Target, LogFilter, context)` and `collect_report(Target, ReportSelection, context)`. Artifacts use native save/share handles and explicit ownership cleanup.

- [ ] Test permission-denied directories, large/zero-byte files, overwrite review, Unicode/space/path injection, transfer cancellation, protected screen capture, recording bounds, rapid log output and a report containing sensitive fields. Assert bounded memory and reviewed export selection.
- [ ] Implement breadcrumbs, native upload/download/save, transfer/readback checks, rename/move/delete effects, screenshots/recording Start/Stop, log filters by app/priority/time and readable processes/services/memory/device reports with raw detail.
- [ ] Run feature tests and native file/artifact flows on Android/iOS/desktop. Verify real unsupported capture/permission outcomes remain explained. Commit with representative rendered screens and artifact validation.

**Deterministic verification:** `cargo test -p adb-core --test files_diagnostics` and `npm test -- src/lib/features/tools`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 15: Add Settings, Network and Advanced tools without command typing

**Files:** Create `crates/adb-core/src/settings/{mod,catalog,actions}.rs`, `crates/adb-core/tests/settings_network.rs`, `network/{mod,forwarding,inspection}.rs`, `operation/sequences.rs`, `src/lib/features/tools/{settings,network,advanced}/` screens, focused tests.

**Interfaces:** Produce `SettingDescriptor { id, scope, value, allowed_values, capability, inverse }`, `plan_setting_change(Target, SettingId, SettingValue)`, `ForwardPlan`, `run_shell(Target, UserCommand, context)` and `run_sequence(Target, ReviewedSequence, context)`.

- [ ] Test unreadable/current value unknown, absent inverse, unsupported write, forward direction and endpoint confusion, connection lifetime expiry, arbitrary command redaction/cancellation, imported sequence requiring review and partial step completion.
- [ ] Implement supported display/density/animation/audio settings with named controls/readback; connection details and forward/reverse mapping list/create/remove when direct transport actually supports the semantics. Keep unsupported entries visible.
- [ ] Implement interactive shell with target, output/error/exit state, saveable parameterized actions and reviewed sequences. Imported metadata/documents never execute automatically; arbitrary shell is not presented as reversible or perfectly danger-classified.
- [ ] Run setting/network/sequence tests, verify direct-forward/reverse behavior against authorized temporary test endpoints, and inspect the discoverable Tools catalog. Commit with exact capability limits.

**Deterministic verification:** `cargo test -p adb-core --test settings_network` and `npm test -- src/lib/features/tools`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 16: Complete accessibility and desktop/tablet parity

**Files:** Update shared components/theme and feature layouts; create `tests/e2e/{navigation,accessibility,desktop-parity}.spec.ts`, `docs/validation/ui-matrix.md` and platform build jobs under `.github/workflows/` only when a remote workflow is actually needed.

**Interfaces:** Preserve existing domain/IPC contracts; adapt layout only. Keep picker/permission implementations native and platform-specific without narrowing user-visible capability requirements.

- [ ] Add journey tests at mobile/tablet/desktop widths for keyboard focus, safe areas, long names, RTL/large text, reduced motion and all five destinations. Test actual controls and effects, not snapshots that simply mirror source markup.
- [ ] Finish split list/detail panes, desktop file/window behavior and native packaging configuration. Validate Windows/macOS/Linux builds on their platforms; cross-compilation or configuration presence is not runtime proof.
- [ ] Run frontend/Rust/native build checks appropriate to changed files, perform screen-reader/keyboard review and capture current screenshots. Commit scoped fixes; do not broaden UI regressions into repeated whole-suite runs without cause.

**Deterministic verification:** `npm run test:e2e -- tests/e2e/navigation.spec.ts tests/e2e/accessibility.spec.ts tests/e2e/desktop-parity.spec.ts`, `npm run check`, `npm run build`, and `cargo test --workspace`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Task 17: Perform full acceptance audit and prepare a reviewable local build

**Files:** Create `docs/validation/acceptance-matrix.md`, `README.md`, `docs/user/{connect,apps,remote,toolkit,recovery}.md`; update actual build/package metadata and validation records.

**Interfaces:** No new speculative abstraction. Consume the implemented product and evidence from Tasks 1–16.

- [ ] Map every spec requirement to implementation, rendered UI, tests and actual physical evidence; classify unsupported firmware paths and outstanding limitations explicitly. Incomplete evidence stays incomplete.
- [ ] Run the complete core journeys on physical Android and iOS controllers plus representative Google TV/Android TV targets: fresh pair/connect, profile targeting, APK/update, app disable/enable/remove review, raw key capture/mapped app, disconnect/startup, launcher/Home/recovery, files/capture/logs/settings/network/shell and history. Use reviewed test objects and preserve real user configurations.
- [ ] Repeat the selected startup tests after a cold boot and over each network mode being claimed; distinguish Wi-Fi, Ethernet and standby. Inspect interruption recovery and data-loss copy across the same journeys.
- [ ] Produce local installable builds and user-facing setup/recovery documentation based on observed behavior. Do not upload/store-publish, create a release or call a local build deployed.
- [ ] Obtain a current-code review, fix valid findings and run checks appropriate to fixes. Summarize what works, tested platforms/TVs, unresolved gates and exact build locations. Keep the goal active until the full requested outcome is verified.

**Deterministic verification:** `npm run check`, `npm test`, `npm run build`, `npm run test:e2e`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings`. Expect exit 0 and passing tests. Run the named new tests before implementation to establish the intended failure, then after implementation to confirm the assertions. Physical evidence remains a separate gate.

## Planning self-review

- Scope retained: every explicit user requirement is owned by a task: direct pair/connect (1–5), shell (1, 15), packages (7), picker installation (8), physical keys and installed targets (12), persistent startup and accepted tools (11–13), launcher/stock disable (10), wider tools (14–15), cross-platform/mobile UI (2, 4, 16–17).
- Result/identity/recovery contracts precede their consumers (3, 6). The five Review Focus conditions are pinned to tests in 3/6, 8, 12, 11 and 13/17 respectively.
- Native physical acceptance is deliberately separate from deterministic CI tests. Missing phones/signing/TV access is a concrete validation blocker, not permission to narrow launch support.
- Source-library research does not settle compatibility or trust behavior. Task 1 and Task 2 must produce independent evidence before broad implementation proceeds.
- No application code or Tauri scaffold was created while writing this plan. Execution has not started.

## Research addendum: direct-pairing candidate

During planning, read the published `libadb` 0.7.0 source archive (crate commit `f442be88c8f03b80303bb1b68e60ff16de1442a6`, archive SHA-256 `ff3e21b286bee2b60b09e5cc868db4ed1c15b433fe98b767fb5086edb0128c7b`). It includes a `pairing` feature, a direct async pairing client using TLS exporter material and an ADB/BoringSSL-compatible SPAKE2 implementation, plus direct TLS transport and key handling. Its fake pairing server shares the library's SPAKE2 primitive, so those tests alone are not independent interoperability evidence. The API is pre-1.0 and discovery is caller-supplied. Nothing was compiled, paired or integrated during this source inspection.

Primary references: [published crate](https://docs.rs/crate/libadb/0.7.0), [source archive](https://crates.io/api/v1/crates/libadb/0.7.0/download), [AOSP wireless-debugging architecture](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/dev/adb_wifi.md), [AOSP pairing implementation](https://android.googlesource.com/platform/packages/modules/adb/+/refs/tags/android-13.0.0_r71/pairing_auth/pairing_auth.cpp).

## Execution handoff

Review this concrete plan and choose Native execution (implement in this session with final independent review) or Subagent-driven execution (fresh implementer/reviewer per task). Native is recommended because these phases share evolving transport/domain interfaces and the physical validation gate should be managed by one continuous session. Full scope and final review remain the same.
