# ADB mobile workspace: exploration notes

Status: research with B direction and reviewed mockups accepted for planning, 30 October 2026. No Tauri project, application backend, dependencies, or device operations have been created or run. The workspace was empty at the start of this exploration and is not a Git repository.

## Confirmed intent

- Build a cross-platform Tauri 2 app with a Rust backend, starting with mobile.
- Support **Android and iOS from the first release**, confirmed by the user during this discussion. Preserve desktop as part of the cross-platform design.
- Provide broad ADB functionality through discoverable controls and guided workflows. Shell access remains available but is not a prerequisite for ordinary tasks.
- Include pairing/connection, installed packages, disabling/enabling, removal, APK selection through a file picker, physical remote key capture and remapping to installed apps, launcher selection, and stock launcher controls.
- Obtain approval of ideas and mockups **before app implementation**.

## Review artifacts

Open `review.html` directly in a browser; it is self-contained and uses no external assets or dependencies. A local review server can also serve this directory:

```sh
python3 -m http.server 8842 --bind 127.0.0.1 --directory design
```

The review board compares three navigation approaches:

| Approach | Strength | Tradeoff |
|---|---|---|
| A. Task first | Clear guided journeys for occasional changes | A broad toolkit and repeated administration require more navigation |
| B. Device workspace, proposed recommendation | Device, Apps, Remote, Tools, History give broad functionality stable locations; guided journeys live inside them | Requires learning the five destinations |
| C. Remote first | Familiar daily TV controls | Administration becomes secondary and can disappear into a large More section |

All three directions now have interactive navigation into shared task samples. The user selected B, the device workspace, during this discussion. That initially selected navigation only; the subsequent “looks good” accepted the revised direction and reviewed toolkit mockups for planning. Execution has not started. The working name, palette, sample model, profiles, numbers, events, packages and results are illustrative.

Clickable samples cover connection, app inspection, disable and recovery, removal review, file selection and install review, remote learning and helper setup, launcher tests, connection loss, files, diagnostics and optional shell. The broader Tools categories and desktop pane are structural sketches. A bulk-disable review with partial results, denied network-access recovery and a sample APK signing conflict are now included. Other bulk actions, split APKs, settings, logs, forwarding, automation and several failure states still need detailed designs. No simulated success establishes device compatibility.

The mockup never connects to a TV, executes input commands, uploads a selected file, or installs an APK. File selection uses only the local filename; it does not parse the APK. Reloading resets the sample state.

## Protocol findings and library candidates

The necessary distinction is **ADB client-to-server** versus **host-to-device/adbd**. A Rust client for a server on port 5037 does not itself remove the need for that server. A self-contained phone app needs the direct device side, including first-time pairing, identity persistence and authentication, rather than depending on a desktop bridge.

| Candidate | Evidence inspected | Implication for this project |
|---|---|---|
| `cocool97/adb_client` | Current repository README, rustdoc, published 3.2.3 source archive | Strongest candidate of the supplied Rust libraries for further evaluation: direct TCP device API, legacy authentication, STLS upgrade, shell and sync operations. Direct phone pairing remains an unresolved gap. |
| `radb` | Rustdoc and published 0.1.8 source archive | Server-oriented APIs connect to an ADB server; some paths invoke the external executable. Not established as a self-contained mobile transport. Async support does not change this distinction. |
| `xDvir/ADBRClient` | Repository README | Explicitly requires a running ADB server; describes a server implementation as future work. Useful reference, not evidence of direct mobile operation. |

### `adb_client` detail

The published `adb_client` 3.2.3 archive associates itself with upstream commit `b88e3324627f4e7e38dbf7f9997ddacf73f12dbe` and path `adb_client`:

- `src/message_devices/adb_message_device.rs`: connection handles CNXN, AUTH and STLS; STLS calls transport upgrade. This is concrete source evidence of direct TLS handling, beyond a README claim.
- `src/message_devices/tcp/tcp_transport.rs`: implements the TCP-to-TLS transport upgrade using rustls.
- `src/server/commands/pair.rs`: `pair()` is implemented on `ADBServer` and sends a proxied host Pair request. It is **not** a demonstrated self-contained pairing-code exchange directly with the TV.
- The source search found no separate direct pairing / SPAKE implementation in the inspected archive. That is a scoped static finding, not a claim about every fork or future release.
- The TLS transport has a custom `NoCertificateVerification` verifier that accepts server certificates and signature callbacks. Evaluate its trust behavior against the actual ADB pairing model before adopting it. ADB uses its own pairing and identity model; a normal public-Web-PKI certificate check is not automatically the right replacement.
- APIs are synchronous at the inspected interface. Streaming, cancellation, connection timeouts, concurrency and mobile lifecycle behavior need validation before choosing execution architecture. Keep blocking work away from the UI thread.

The `radb` 0.1.8 archive associates itself with commit `5e534eeb5b89cf74e8d011ab7198ad74db202942`. Its default client address is `127.0.0.1:5037`; the archive includes external-process paths in client/device utilities.

Source archives were downloaded for read-only inspection into `/tmp/adb-gui-design-research/`. Nothing was compiled or integrated. Archive SHA-256 values:

```text
adb_client-3.2.3.crate 773b25a440e72829bbb1b96435ee0e65ef0f14a3da545590930ca881a68e59d7
radb-0.1.8.crate       f6658b462baa66814912cec3d591c9963bc1eda5f5daedeacdf79a37980dfb19
```

### Current AOSP changes the connection design

Current AOSP distinguishes legacy TCP debugging from modern wireless debugging. Modern pairing and subsequent TLS connection are separate services/endpoints; discovery advertises pairing and connection service types separately. The UI must not reuse the pairing port as the connection port or label pairing success as an operational connection.

The supplied historical `protocol.txt` is helpful for framing, not a complete modern implementation specification. Current AOSP protocol and wireless-debugging sources must govern compatibility details. The cstyan documentation itself warns that its research is old. The Synacktiv articles explain the protocol layers and Rust implementation but do not establish first-time mobile pairing acceptance.

## Constraints that affect the UI

| Area | Design consequence | Evidence status |
|---|---|---|
| Tauri on mobile | Do not assume the desktop shell plugin can spawn an `adb` executable. Investigate an embedded direct transport and native platform adapters. | Tauri shell documentation restricts mobile to opening URLs |
| Android + iOS | Share domain actions and interaction design; use platform-specific permissions, discovery, key storage and file handling where needed. | Direction proposed; mobile runtime untested |
| iOS network access | Explain local-network access before discovery; handle denial and retry distinctly from “TV offline.” Prefer evaluating native Bonjour for declared ADB service types rather than assuming a Rust raw-multicast implementation has equivalent requirements. | Apple TN3179; no device validation |
| Backgrounding | Show reconnecting and unknown operation results. Never blindly repeat an interrupted install or destructive operation. | Proposed interaction requirement; lifecycle behavior untested |
| Pair/connect | Pair → connect → inspect. Include expired/rejected code, TV approval, no discovery, changed port, permission denial, and legacy setup paths. | AOSP and Android docs support distinct stages |
| No command typing | Use names, icons, filters and pickers. Some setup still needs a pairing code/address. Optional search is not required to browse. | Product intent |
| App metadata | App labels/icons may require APK inspection or a helper; package listing alone does not prove friendly labels are available. Preserve a package-ID fallback and visible loading state. | Implementation research still needed |
| App actions | Distinguish disabled, force-stopped and uninstalled states; distinguish removal for a profile from system-partition removal. Record exact target profile and effects. | Must verify chosen commands and OEM behavior later |
| Recovery | Disable/enable can have an inverse. Reinstall is not data restoration. Do not label app-data deletion or arbitrary shell work “undoable.” | Interaction requirement |
| Remote capture | Probe allowed event sources; show source and namespace. Raw Linux event code and Android KeyEvent code are different. Never infer a mapping from a number without its context. | AOSP input docs; untested on target TVs |
| Persistent remapping | Detect native firmware support first; otherwise support the accepted Shizuku/Termux/rish toolkit and a managed TV-side listener. Distinguish privilege setup, runtime process, original-key handling and startup recovery. Do not require a new remapper APK by default. | Saved TCL scripts inspected; current TV runtime not checked |
| Special keys | Power, voice, IR-only or vendor-intercepted keys may not be visible/remappable through the selected method. Show unknown or unsupported with a reason. | Device acceptance required |
| Launcher | Find eligible Home activities; test launch; request default; verify physical Home; separately review stock disable. Never disable the only known working launcher during a switch. | Proposed workflow; firmware restrictions unresolved |

## Breadth to preserve while refining the design

Do not reduce the product to a remote or APK installer. The proposed information architecture reserves places for:

- Connection and device management: saved devices, pairing/approval, discovery, address entry, identity, OS/build, profiles, capabilities, connection health and restart/recovery.
- Apps: user/system/disabled filters, details, install/update and split sets, launch, stop, enable/disable, uninstall per profile, reinstall where possible, permissions/app ops, activities and app links, APK export, reviewed bulk actions.
- Remote: D-pad/media/volume/text, app shortcuts, input-source inventory, key capture, gestures, target selection, mapping tests, helper lifecycle and recovery.
- Personalization: launcher selection, stock launcher controls, supported display/density/animation/audio settings, input inspection.
- Files and diagnostics: permitted directories, transfers and progress, screenshots, recording where supported, log filters, reports, processes, services, memory and network inspection.
- Advanced: shell, saved actions, forward/reverse where supported, parameterized action sequences, previews and per-step results.
- History: device/profile, before/after state where obtainable, output, verification, partial/unknown results, export, and supported recovery.

These are capability-design requirements, not claims of universal availability or a finalized release sequence. Detail every selected flow and unsupported state before treating the design as ready to implement.

## Required evidence before dependency selection or runtime promises

After design approval and separately authorized implementation/testing, validate on physical Android and iOS phones against representative modern and legacy TVs:

1. First-time modern pairing without a laptop, desktop server or pre-seeded key; persist identity; reconnect after phone and TV restart; handle revocation and wrong/expired codes.
2. Direct TLS and legacy RSA-auth connections; confirm they are independent code paths and do not silently fall back to an external executable.
3. Native network permission denial/recovery, discovery on real networks, manual address fallback and port changes. iOS Simulator behavior does not prove local-network privacy behavior on a phone.
4. Native picker → local file/URI handling → package inspection → transfer → install verification; interrupted operations and same-package updates.
5. Concurrent shell/log streams and control actions, cancellation, disconnect and app background/foreground recovery.
6. Input visibility and namespace on the TV, helper setup, actual key interception, target launch, disconnect persistence, reboot persistence and removal of mappings.
7. Alternate launcher/default/Home behavior, stock-disable recovery and OEM restrictions.

No such runtime evidence exists yet. A library API or green build cannot substitute for these acceptance results.

## Sources

User-supplied:

- [Historical AOSP protocol](https://android.googlesource.com/platform/system/core/+/dd7bc3319deb2b77c5d07a51b7d6cd7e11b5beb0/adb/protocol.txt)
- [Synacktiv protocol internals, part 1](https://www.synacktiv.com/publications/diving-into-adb-protocol-internals-12)
- [Synacktiv protocol internals, part 2](https://www.synacktiv.com/publications/diving-into-adb-protocol-internals-22)
- [cstyan/adbDocumentation](https://github.com/cstyan/adbDocumentation)
- [adb_client rustdoc](https://docs.rs/adb_client/latest/adb_client/)
- [radb rustdoc](https://docs.rs/radb/latest/radb/)
- [ADBRClient](https://github.com/xDvir/ADBRClient)
- [cocool97/adb_client](https://github.com/cocool97/adb_client)

Current primary documentation and versioned source evidence:

- [AOSP modern protocol](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/dev/protocol.md)
- [AOSP wireless-debugging architecture](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/dev/adb_wifi.md)
- [Android ADB guide](https://developer.android.com/tools/adb)
- [AOSP getevent](https://source.android.com/docs/core/interaction/input/getevent)
- [Android AccessibilityService](https://developer.android.com/reference/android/accessibilityservice/AccessibilityService)
- [Android key-filtering capability](https://developer.android.com/reference/android/accessibilityservice/AccessibilityServiceInfo#FLAG_REQUEST_FILTER_KEY_EVENTS)
- [Tauri shell support](https://v2.tauri.app/plugin/shell/)
- [Tauri native mobile plugins](https://v2.tauri.app/develop/plugins/develop-mobile/)
- [Tauri dialog plugin](https://v2.tauri.app/plugin/dialog/)
- [Apple local-network privacy](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy)
- [Published adb_client 3.2.3 source archive](https://crates.io/api/v1/crates/adb_client/3.2.3/download)
- [Published radb 0.1.8 source archive](https://crates.io/api/v1/crates/radb/0.1.8/download)

## Pending decisions

- B direction and revised reviewed mockups accepted for planning. The implementation plan is written; execution method selection is pending.
- User accepts Shizuku and rish/Termux as requirements where needed and wants guided installation on the TV. Prefer native mappings or the existing toolkit; a new companion is not a default requirement. App implementation and actual device installation remain gated by design approval.
- Detailed flows for the wider toolkit, platform-specific connection states, large text/accessibility, and responsive tablet/desktop interactions.
- Written plan and execution-method handoff before app work. The broader objective remains unchanged.

## Selected direction and written design

The user selected B (device workspace). The draft design is at `../docs/superpowers/specs/2026-10-30-adb-workspace-design.md`. A fresh builder-readiness reviewer reported “nothing important left” after reading it; that is a check for significant planning ambiguities, not evidence of design approval or runtime support.

The user accepted Shizuku and rish/Termux requirements and requested installation assistance. The design now includes guided TV-toolkit installation and startup validation. The earlier unconditional service/companion framing was corrected. Keep Android and iOS in the launch design. No application implementation has started.

## Correction: persistent mapping does not imply a new remapper app

The user reported reboot-persistent mappings on their TCL and clarified that Shizuku and rish/Termux are acceptable dependencies, ideally installed with help from this app. Those are accepted design preferences, not permission to install/change the TV during this design phase.

Read-only inspection of current saved local files established:

- `/Users/jneerdael/Scripts/tcl/tcl-remote-buttons.sh` starts a background `getevent -l` listener; matches MSC_SCAN plus EV_KEY UP; launches the configured Android activity. It is not an Android AccessibilityService.
- `/Users/jneerdael/Scripts/tcl/start-tv-services` is a Termux:Boot monitor that repeatedly uses paired local ADB to start/check the listener and a Shizuku-compatible server. Its saved server package is Shevery. This does not establish the current live installed manager, nor make every Shizuku variant interchangeable.
- `/Users/jneerdael/Scripts/tcl/tv-shell` uses paired local ADB. The saved remapping path does not use rish.
- The script's comment explains that it launches after TCL's original handler, with original target apps disabled. That is not general event interception or evidence that every key can be remapped without changing OEM behavior.

Design consequence: model firmware-native mappings, shell-listener mappings and helper-service mappings separately. Show privileges, running dependencies and verified startup behavior. Support Shizuku/rish or local ADB as execution routes where actually supported; Termux:Boot as an evaluated startup route. Offer inspect/reuse/install/authorize/configure/test/recover steps without demanding command entry. Do not add a custom remapper APK as a prerequisite without a demonstrated capability gap.

The older October TCL notes located the saved files; the files were read in this session. Reboot claims from those notes remain historical evidence, not a fresh physical-device validation. No TV connection, restart, package install or settings mutation was performed here.

Relevant primary references: [rish description](https://github.com/RikkaApps/Shizuku-API/tree/master/rish), [Shizuku startup guide](https://shizuku.rikka.app/guide/setup/), [official Shizuku downloads](https://shizuku.rikka.app/download/), [Termux installation](https://github.com/termux/termux-app#installation), [Termux:Boot](https://github.com/termux/termux-boot).

## Design acceptance and implementation plan

The user replied “looks good” after the corrected Shizuku/Termux toolkit design and mockup. Record the B workspace and reviewed toolkit direction as accepted for planning; do not ask again whether a TV toolkit is acceptable. The implementation plan is `../docs/superpowers/plans/2026-10-30-adb-workspace.md`; execution has not started.

Additional direct-pairing candidate: source-inspected published `libadb` 0.7.0, repository `https://github.com/tarigo/libadb`, commit `f442be88c8f03b80303bb1b68e60ff16de1442a6`, SHA-256 `ff3e21b286bee2b60b09e5cc868db4ed1c15b433fe98b767fb5086edb0128c7b`. It contains direct TLS and pairing features, including an ADB-compatible SPAKE2 implementation; source presence is not physical interoperability or mobile-runtime proof. It is the first validation candidate in the plan, superseding the earlier assumption that only adb_client should lead evaluation. No dependency was integrated or compiled.
