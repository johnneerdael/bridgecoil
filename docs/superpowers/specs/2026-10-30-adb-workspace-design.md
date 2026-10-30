# ADB device workspace: proposed product design

Status: **design direction and reviewed mockups approved**. The user selected navigation B, accepted TV-side Shizuku/rish/Termux with guided installation, and replied “looks good” to the revised design/mockup. Proceed to implementation planning. Runtime capabilities remain unvalidated; approval is not evidence that the app is implemented or compatible with every TV.

Date: 30 October 2026.

Review alongside the standalone mockup at `design/review.html` and its evidence notes in `design/RESEARCH.md`. The working name “ADB Studio” is a placeholder.

## Intent and agreed direction

Provide extensive control of Android TV / Google TV devices from a friendly mobile interface. People should find an app, install an APK, change a launcher, identify a physical remote button or diagnose a problem without knowing an ADB command. Preserve an optional shell for work beyond the visual catalog.

The user specified Tauri 2 and a Rust backend, then confirmed Android and iOS from the first release. Desktop remains part of the cross-platform product. The user selected B: a device workspace with **Device, Apps, Remote, Tools and History**. Ideas and mockups must be approved before application implementation.

The user has also accepted Shizuku and rish/Termux as requirements where needed and requested that the app help install them on the TV. The subsequent “looks good” accepts the revised design direction and reviewed mockups for planning. The TV-side toolkit installation and mapping-runtime paths are included; dependency choice and runtime compatibility still require evidence.

## Approaches considered

| Approach | Benefit | Cost | Decision |
|---|---|---|---|
| A. Task first | Guided jobs are easy to discover on occasional visits | Repeated inspection and a large toolkit take more navigation | Considered; not selected |
| B. Device workspace | Stable feature destinations support both guided tasks and deep inspection | Five destinations require initial orientation | **Selected by user** |
| C. Remote first | Daily TV controls are immediately familiar | Administration is pushed into Customize and More | Considered; not selected |

Keep task-first wording and guidance inside B. The selected direction is a functional workspace, not a dashboard of command names.

## Goals, boundaries and failure conditions

### Goals

- Make the full toolkit browsable; ordinary work must not depend on typing a command, package name or Android activity.
- Cover pairing, connection, package inspection and changes, picker-based installation, shell access, physical key learning, mappings to installed apps, launcher selection and stock launcher controls.
- Design for Android and iOS together. Match behavior where capabilities permit; use native platform permissions and pickers.
- Support broad inspection, settings, file transfer, diagnostics and advanced operations without destabilizing the five destinations.
- Show the selected device and profile, actual capability state, operation progress and verified result throughout.
- Make repeatable changes easy to review and recover when a real inverse exists.

### Boundaries

- The primary target is an Android TV / Google TV device, controlled by a separate phone. Android phone/tablet targets can share supported operations but do not redefine the TV-first experience.
- Propose direct, local network connections as the mobile baseline. A desktop ADB server or laptop must not be required to pair a supported modern TV. Do not substitute a bridge-only architecture if direct pairing proves difficult.
- Retain legacy network debugging when the target exposes it. Some older TVs require initial external setup. Show that limitation; do not promise first-time phone-only setup for every television.
- USB transport is a desktop extension and a capability-dependent Android-host extension. Do not promise USB parity on iOS. It does not replace the required mobile wireless path.
- Root-only operations remain discoverable with their prerequisites. Rooting a TV or bypassing device authorization is outside the proposed product.
- No cloud account, relay, analytics or remote internet access is required in the proposed baseline. Export/share occurs only through a user action.
- Mirroring, video streaming and a full remote desktop are not prerequisites for this design. Screenshots/recording and input controls remain in scope where supported.

### Failure conditions

The design fails if it becomes a command cheat sheet, hides most functionality behind search, silently targets the wrong TV/profile, reports a requested change as verified, implies every package can be safely removed, or saves a remote mapping that cannot actually run.

“No command typing” permits necessary pairing codes, manual addresses, filenames and optional search. It does not require an on-screen keyboard for a user who is simply browsing installed apps or choosing a target app.

## Information architecture and navigation

Use a persistent device selector in the header. Show friendly name, connection state and active Android profile. A device switch cancels pending review forms for the old target after explaining unsaved choices; never carry a prepared destructive action to another target. Continue a running operation only against its original recorded identity.

On mobile use five labelled bottom destinations. Within a flow, Back returns to its origin and preserves list position/filter. A change requiring review uses a dedicated screen. Short choices can use a sheet; lengthy reviews must not be forced into a small bottom sheet.

On tablets/desktop use the same destination names in a navigation rail. Use list/detail panes for Apps, Files and History. Keep operations and capabilities equivalent; screen size changes density and placement, not the meaning of an action.

| Destination | First screen | Main content |
|---|---|---|
| Device | Selected TV and useful status | Connect, identity, profiles, capabilities, storage, restart/recovery and shortcuts to common jobs |
| Apps | Browsable installed apps | Filters, details, install/update, launch/stop, enable/disable, uninstall, permissions and bulk review |
| Remote | TV controls | D-pad, media/volume/text, app shortcuts, physical-button learning and mappings |
| Tools | Categorized toolkit | Personalization, Files, Capture, Diagnostics, Network and Advanced |
| History | Operations for this TV | Requested/running/results, details, output, verification and available recovery |

Keep Device concise: connection and profile, OS/model, meaningful storage data and a few action shortcuts. Do not fill it with decorative statistics or maintenance recommendations lacking evidence. Unknown values show Unknown, not a sample number or zero.

Search supplements browsing. Apps can search friendly names or package IDs. Tools search can match task synonyms such as “home screen” → Launcher and “debloat” → installed/system app management. No generated command executes merely because a search matched it.

## Visual and interaction language

Propose the current mockup's warm light background, deep teal action color, restrained surface hierarchy, readable lists and clear typography. The palette and working name remain open to feedback. Dark mode should use the same hierarchy and semantics; do not treat a dark terminal aesthetic as a prerequisite for advanced tools.

Use at least 44 × 44 logical-pixel touch targets, visible keyboard focus, persistent text labels, accessible names for icon controls, sufficient contrast and states distinguishable without color. Validate large text and screen-reader reading order. Allow long app names, localized labels and RTL without clipping controls.

Reserve motion for navigation, showing a captured key and progress changes. Honor reduced-motion settings. Do not animate a listener as evidence that an event was received.

Use specific action labels: Enable app, Disable app, Remove for Owner, Set as Home, Stop recording. Show package IDs and commands in expandable details. Avoid success toasts as the sole record of a change.

## Common capability and operation model

Every action reports one of: **available**, **needs setup**, **unsupported**, or **not checked**. Include the reason, how it was determined and a next step when possible. Retain unsupported tools in a browsable location so absence is understandable.

Capability results are scoped to device identity, firmware/build, profile, transport and required helper state. Invalidate relevant results after a build/profile/helper change or reconnect when necessary. A static model name alone cannot establish capability.

For a change use this pattern:

1. Pick the object or task by human-readable name.
2. Inspect current state and compatibility.
3. Review affected TV/profile, exact intended change, data effects, prerequisites and recovery limits.
4. Apply once, with progress and cancellation semantics appropriate to the operation.
5. Verify by reading state or performing the defined check.
6. Record the result and expose supported follow-up/recovery.

Read-only browsing and routine remote key presses do not need confirmation screens. Reserve review for changes with meaningful effects, bulk jobs, settings changes and TV-toolkit installation or additional helper installation. Do not make the user confirm every remote tap.

Distinguish queued, running, awaiting TV/user action, verified, failed, partial and unknown outcomes. Unknown includes a lost connection after a mutation was sent. Cancellation is a request to stop, not a guarantee the TV was unchanged. Reconnect and inspect before offering a retry of an interrupted install or change.

Batch review lists every target, the actual action and any excluded items. Results remain per item. Stop or skip on failure according to the reviewed policy; never imply an atomic rollback when ADB operations are independent.

## Connection and first-use flow

### Initial setup

1. Explain that the phone controls a TV on its local network and requests access for that purpose.
2. Present native local-network access where required. Handle denied/undetermined/allowed states with platform-appropriate guidance; do not label permission denial as TV offline.
3. Offer a short, illustrated TV setup guide using the physical remote: enable developer options, locate the debugging option available on this firmware, and open pairing when provided. Exact menu names can vary.
4. Discover advertised ADB services. Provide manual address/port entry alongside discovery, not behind an error-only path.

### Modern wireless debugging

Keep pairing endpoint, pairing code and connection endpoint distinct. Discovery may fill addresses, but code entry is explicit. Pairing completion advances to connection; it is not the connected state.

Display code expiry/rejection, pairing service closed, network timeout, local access denied, known identity revoked and changed connection port as distinct recoverable states. Do not repeatedly try an expired code or retain it in history/log exports.

After connection, read device identity/build/profile and capabilities. A saved IP is not a device identity. Explain a mismatch rather than automatically applying the previous TV's settings to a different target now occupying its address.

Remember authorized host identity in appropriate protected local storage and provide Forget this TV. Explain that deleting a local saved item does not necessarily revoke trust on the TV; provide TV-side revoke instructions. Never require private-key import for normal first use.

### Legacy debugging

Identify the available target method. Offer the address entry and TV approval path when supported; explain lack of transport encryption where applicable. State when external/USB setup is necessary and why the app cannot enable an unauthorized target's debugging itself.

### Reconnect and lifecycle

Show foreground/background disconnect and reconnect state. Keep cached lists readable and visibly stale. Disable mutations against an unavailable target. Re-read uncertain operation state after reconnection. Avoid a product promise of permanent background connectivity on iOS.

## Apps

### Browse and inspect

Show app icon/name when obtained, package-ID fallback, system/user classification, enabled state and selected profile. Load expensive metadata progressively without blocking the first useful list. Do not mistake metadata lookup failure for an absent package. Keep optional details separate from primary app actions.

Provide All, User, System and Disabled filters; add removed-for-this-profile entries where the device can enumerate them. Expose system packages deliberately, with clear effects rather than blanket “safe to remove” ratings.

Details show version, package ID, profile, install paths, launchable activities, permissions/app ops where readable, storage values where obtainable, state and capabilities. A package with no launchable activity has no Open action; it remains inspectable.

### State-changing actions

- **Open** launches a discovered activity. The UI should not ask the user to type its component name.
- **Force stop** explains that it stops the present process and the app may start again. It is distinct from disabling.
- **Disable** preserves the installed package/data while changing availability for a profile. Offer Enable only when applicable. Show risks for system dependencies without claiming to have exhaustively identified them.
- **Remove** distinguishes ordinary uninstall from uninstall-for-profile and explains data effects. Preserve-data options appear only when supported and accurately understood. A system-partition APK remaining is not full device removal.
- **Reinstall/restore availability** is offered only when a source or built-in package is actually available. Never imply data restoration from package reinstall.
- **Clear cache/data** and permission/app-op changes have separate, specific reviews. Unsupported or unreadable operations remain explained rather than guessed.
- **Bulk** uses selection, a per-app review and per-app verification/results. Do not supply an unverified default debloat set.

### Install/update through a native picker

Use the native Android/iOS picker. Treat the returned URI or file handle according to platform rules; do not require a desktop filesystem path or ask the user to move the APK manually to a special folder.

Inspect the file before enabling the real Install action. Show app/package, version, architecture/SDK compatibility, signing/update relationship and selected profile. Distinguish new install, same-package update, signature mismatch, downgrade, corrupt file and incompatible target. Do not silently uninstall an existing app to work around an update failure.

Support a single APK and a reviewed multi-file split APK set. Validate that a set belongs together and includes required splits; do not treat an AAB as directly installable. Archive/container support can be added only with explicit parsing and review rather than guessing based on the extension.

Show preparation, transfer, install and verify stages with bytes/progress where available. After disconnect, inspect the installed version/state before offering Retry. Verification checks the expected package/version, not only the existence of a success string.

## Remote controls, key learning and mappings

### Daily control

Provide D-pad/OK, Back, Home, Menu, media and volume controls; show unsupported controls with a reason. Keep touch feedback distinct from TV acknowledgement. Text entry uses a dedicated field and reports limitations rather than assuming every Unicode string can be injected through the same method. App shortcuts choose from launchable installed apps.

### Learning a physical button

1. Inspect available input sources and capture methods.
2. Explain that pressing the key may still perform its normal TV action.
3. Start a bounded, cancellable listening session and show its state.
4. On an event, display key name, namespace (Linux input vs Android KeyEvent), code, source/device identity when available, and press/release information. Unknown keys retain raw context without fabricated friendly names.
5. Let the user press again to confirm the source and key. Debounce duplicate press/repeat messages for presentation without throwing away diagnostic detail.

No event is different from unsupported capture. Provide access-restricted, timeout, wrong source, event-only/observe-only and TV-intercepted guidance. Some IR, power, voice and vendor keys may never reach an accessible capture/filter path.

### Mapping a button to an installed app

Choose the captured source/key, gesture (single press, long press or double press only where supported), action and target launchable app. Show conflicts with existing mappings and effects on the normal key action. Do not assign a guessed key merely because its number resembles an Android code.

Mapping validation must test the physical button and intended app, not only an app-launch command. Include Home/Back/D-pad navigation checks when the interception mechanism could affect them. Do not mark a mapping active until its chosen event-handling path is usable and its original-action behavior is disclosed.

### Mapping mechanisms and accepted TV-side toolkit

Do not require a newly developed remapper APK for every persistent mapping. Separate the privilege mechanism used to configure/run an action from the mechanism that handles each physical key press:

| Mechanism | Setup dependency | Runtime dependency | Persistence claim |
|---|---|---|---|
| Firmware-native mapping, where positively identified | Supported vendor setting/API and sufficient privilege to write it | Existing firmware handler | Setting survives reboot only after readback and physical cold-boot testing on that firmware |
| Shell listener / script mapping | Accessible input events, an authorized shell execution route, script deployment and startup configuration | A TV-side listener process plus its startup/recovery mechanism | Phone-independent behavior and reboot recovery are separate tests |
| Accessibility or other helper integration | Compatible app plus its explicit TV-side permissions/setup | That app's service/process | Requires its own physical-key and restart validation |

The user accepts **Shizuku and rish/Termux** as requirements for features that need them, and wants guided installation. These Android dependencies belong on the controlled TV; the iOS controller does not run Shizuku. Detect and reuse existing compatible installations. Basic remote ADB operations can run directly when available and should not be blocked by unrelated missing tools.

Shizuku/rish provides an execution/privilege path; it is not itself a universal remapping engine. Termux is not privileged merely because it is installed. When rish is used, record whether its backend runs with shell or root privileges and validate the needed access. An ordinary Termux shell, shell UID 2000 and root are different capabilities.

The user's saved TCL setup is a concrete reference for the script path: a `getevent` listener reads MSC_SCAN plus EV_KEY release events and launches recorded app activities; a Termux:Boot monitor starts/restarts the listener through paired local ADB. It uses no dedicated remapper APK/accessibility service, but it does run a background listener. The saved `tv-shell` also uses local ADB, not rish. Do not rewrite this as evidence that rish was the actual mapping backend or that the TV currently has the saved state. No live TV changes/validation were performed in this design session.

Do not copy the TCL script as a universal interception solution. Its recorded behavior launches after the original OEM handler and depended on the original target apps being disabled. Make such dependencies explicit in a per-device review; never silently disable the original apps. Observation/launch-after-event does not establish suppression of the original action. Pin source-device identity as well as scan code when the input source can be determined, and test duplicate events, press/release, long presses and repeated inputs before generalizing.

Prefer a detected firmware-native mapping when it can satisfy the requested behavior. Otherwise offer the accepted toolkit with a managed listener and clearly stated startup requirements. Retain a separate helper integration only for capabilities that cannot be provided by the supported native/script route. Do not silently install a new app-owned companion or require an accessibility service without establishing the need.

### Guided installation, authorization and startup

Make **TV setup** available from Device and Remote. Show distinct states for missing, installed, not authorized, running, startup not configured, restart test pending and unavailable. Do not label “package installed” as “ready.”

1. Connect to the TV through authorized ADB; inspect model/ABI/OS, installed packages, versions and available privilege/startup routes. Recognize compatible Shizuku variants without silently replacing an existing manager.
2. Show a review of missing tools, their purpose, download source, signing identity, compatibility, permissions and install/update effects. Offer official project release sources or the user's APK file picker. Avoid automatic uninstallation on signature or shared-permission conflicts.
3. Install the reviewed Android APKs onto the TV using the existing ADB transport, with transfer/install/verification status. Phone native pickers and downloads must work on Android and iOS. This design does not perform those installations now.
4. Guide first launch, Shizuku startup and authorization, and any required TV-side approval. Use direct ADB to automate supported setup steps; leave genuine OS authorization prompts as guided TV interactions rather than claiming they can all be bypassed. Configure rish through its supported setup/export path when needed; validate execution with a harmless capability/identity probe.
5. Prepare Termux and, when the chosen startup method needs it, Termux:Boot and the scoped listener/monitor. Reuse compatible existing configuration; show the exact additions before changing it. Keep Termux and its add-ons compatible in source/signing identity. Do not bundle the user's unrelated SSH setup or expose an SSH server as a hidden requirement.
6. Review background/auto-start permissions appropriate to this firmware. Configure only the selected feature's requirements and preserve unrelated user tools/mappings. Rootless Shizuku normally needs startup again after reboot; a tested boot automation path is separate work, not an automatic property of Shizuku/rish.
7. Test a physical key, test with the phone disconnected, and offer a separately reviewed TV-reboot test. Record which tests passed, on which firmware, network mode and startup route. Distinguish cold boot from standby/wake and do not promise Ethernet recovery from a Wi-Fi test.
8. Provide health status, inspect logs, restart/re-authorize, pause mappings, remove a mapping, and undo app-owned startup additions. Uninstall tools separately, explaining impact on other apps that may use them.

Store mapping configuration on the TV/runtime, with a phone-side record for display/export. Show the selected mechanism and dependencies on the mapping details screen. “Works without this phone” and “starts after TV reboot” are independent status fields; neither means “nothing runs on the TV.”

If no supported mechanism can provide the requested mapping, preserve input inspection where possible and show the exact limitation. A user's permission to mandate tools does not prove that the tools support every key or firmware.

## Launcher workflow

1. Read the existing default and enumerate eligible Home activities. Do not infer eligibility from an app name containing “Launcher.”
2. Launch the selected replacement while keeping the stock launcher enabled. Ask the user to confirm that it renders and works with the physical remote.
3. Request the new default using the supported firmware path; handle any TV chooser/approval. Restricted firmware is a supported outcome with a reason.
4. Test the actual physical Home button and confirm the replacement opens. A launch request alone does not establish default behavior.
5. Offer **Disable stock launcher** separately, after those checks, with exact package/profile and recovery limitations. Do not remove the stock package as a hidden side effect of changing defaults.

Record prior default and package state. Recovery re-enables the recorded package when necessary and restores/requests its default status, then tests Home. A timeout rollback must only be promised if a reliable TV-side mechanism is implemented and verified; a suspended phone is not a dependable recovery timer.

Manufacturer integrations may depend on the stock launcher. Report observed limitations and do not describe the workflow as universally safe. Handle a failed default change without proceeding to stock disable.

## Wider toolkit

Keep this breadth in the design and implementation plan. Device-specific restrictions alter availability, not the existence of a discoverable category. The entries below do not imply root access or universal support.

| Area under Tools | User tasks | Interaction and result |
|---|---|---|
| Personalization | Launcher, supported density/display, animation, sound and input settings | Read current value; choose a labelled value; show scope; review change; verify/read back; offer known inverse |
| Files | Browse permitted storage, upload, download, rename, move and delete | Breadcrumbs, native pickers/save/share, destination review, transfer progress, explicit overwrite/data effects; show denied folders honestly |
| Capture | Screenshot, recording where supported | Choose action/options; explicit Start/Stop; bounded progress; inspect artifact and use native Save/Share; indicate protected content or unsupported capture |
| Diagnostics | Logcat filters, device report, processes/services, memory and storage, bug report | Read-only first; finite or cancellable streaming; filters by app/priority/time; sensitive content review before export; readable summary plus raw detail |
| Network | Connection details, diagnostics, supported forward/reverse operations | Pick direction/protocol/endpoints; show host vs TV addresses and lifecycle; verify mapping; list and remove active mappings; fail clearly if direct transport lacks needed semantics |
| Advanced | Interactive shell, saved parameterized actions, reviewed action sequences | Clear target/profile; streamed output and exit state; cancel; explicit parameters; review effects; per-step results; never auto-execute from imported text |

Do not hide app-specific permissions, app ops or activity launching in a general terminal screen. Link those tools from Apps, with cross-links from the catalog.

For settings with no dependable readback or inverse, state that limitation before Apply. Backups of APKs, settings records or package lists are not a full backup of protected app data.

## History and recovery

Keep a local operation record keyed by device identity and profile, with timestamp, requested action, targets, known prior state, progress/result, exit/error details, verification evidence and recovery availability. Allow filtering to failures/unknown outcomes and expanding raw output on demand.

A recovery action is a new reviewed operation checked against the current state. An Enable inverse does not warrant a generic Undo button for uninstall, data deletion or arbitrary shell commands. Avoid replaying historical actions against a different device/build without review.

Do not log pairing codes or private keys. Propose configurable retention and a deliberate Clear history action, explaining that clearing local records does not reverse TV changes. Export selected records/reports through an explicit save/share flow with content review.

## Proposed architecture boundaries

Use the frontend for presentation, navigation, input and review. Use Rust for typed domain operations, device identity/profile scoping, capability results, command construction/parsing, operations/history and transport coordination. Use Kotlin/Swift platform adapters where native discovery, permissions, picker/URI access, secure storage or lifecycle handling need them.

Separate direct pairing, direct authenticated transport, shell streams and file-sync streams from high-level tasks. The UI asks for an operation such as DisablePackage on a known target; it must not concatenate package names or filenames into shell strings itself.

The optional shell is intentionally flexible, but it must be an explicit user-entered workspace rather than a route for untrusted metadata or imported documents to execute commands. Display target and raw execution effects accurately; do not claim automatic reversibility or perfect danger detection for arbitrary commands.

`adb_client` is the leading evaluated candidate among the supplied libraries for direct device operations. It is not a final dependency decision. The inspected published pairing API proxies to an existing server; direct first-time phone pairing and its trust model remain a technical validation gate. `radb` and ADBRClient do not establish a self-contained mobile solution in the inspected evidence.

Do not begin broad product implementation by assuming that gap is solved. After design approval, plan a bounded technical validation stage for direct pairing/authentication on Android and iOS. If it fails, report the evidence and revisit the transport strategy without silently dropping iOS, first-time pairing or requiring a desktop server.

## Decisions explicitly left to the builder

- Frontend framework and component library, provided they preserve Tauri 2, mobile behavior, accessibility and the approved design. The user has not selected React/Svelte/Vue.
- Routing/state-management mechanism, repository structure and storage implementation.
- Exact typography assets and icon library, after visual review and licensing checks.
- Which native APIs implement permission, discovery, key storage and file access on each platform.
- ADB library version/fork/extension after the direct-pairing validation gate; technical choice cannot reduce the approved capability requirements.
- Capability probe details, parsers and safe invocation/argument handling, grounded in actual firmware behavior.
- Specific minimum OS versions and desktop packaging support after dependency/toolchain validation. Android and iOS launch support cannot be deferred by this choice.
- Bounded defaults for stream buffers, capture duration, history retention, timeouts and retry backoff. Destructive actions must not be automatically retried based only on transport failure.

## Review and acceptance evidence

### Design approval gate

Before application implementation, obtain explicit approval of this design and the relevant mockups. Navigation selection alone does not satisfy this gate. All visual/interaction prototypes remain simulated and must be labelled as such.

Refine the mockups to cover at least these complete journeys and failure states:

- Fresh TV setup → pair/approve → connect → select profile → capability inspection; permission denial, expired code and changed connection port.
- Browse packages without search → inspect → disable → verified result → enable; uninstall data review and bulk partial results.
- Native-picker intent → APK/split-set review → transfer/install progress → verified result; signature mismatch and interrupted result.
- Physical key listening → observed key/source → installed target → native/toolkit setup → actual-test confirmation → mapping active; no key, observe-only and conflict states.
- Replacement launcher test → default request → Home confirmation → optional stock disable → recovery; restricted firmware and lost connection.
- Browse Tools → logs/files/capture/settings/network/shell → review/apply or stream → recorded result. Keep full catalog discoverable without command input.

Some listed detailed states remain prose requirements or placeholders. The user’s “looks good” approves the reviewed direction and core/toolkit mockups for planning; it does not claim that every detailed journey is implemented or device-validated. Preserve the specified behavior when refining feature tasks, and bring material product changes back for review.

### Runtime evidence required later

The implementation plan must retain both mobile platforms and representative TV firmware in acceptance coverage. Physical-device checks must establish first-time direct pairing without a server, identity persistence/revocation, direct TLS/legacy authentication as applicable, local-network permission recovery, native file handling, interrupted operations, input capture/filtering, app targeting and launcher behavior.

Verify operation results against their actual targets and states. A screenshot, successful command exit, green build or static library API cannot prove persistent mappings, working Home behavior or TV firmware support. Preserve unknown/unsupported results instead of relaxing the acceptance claim.

## Current state and next step

- Complete: initial research, three navigation proposals, interactive core samples, user selection of navigation B.
- Approved for planning: the selected B workspace and reviewed core/toolkit mockups, after the user replied “looks good.” A fresh builder-readiness review of the revised design found no important unanswered planning questions. Detailed feature states still need implementation-time refinement and validation.
- Plan written: `docs/superpowers/plans/2026-10-30-adb-workspace.md`. Tauri scaffolding, Rust backend, TV runtime integration and device modification have not started.
- Next following design/mockup approval: write an implementation plan with direct mobile pairing validation first, followed by the full approved UI and backend scope. Approval does not turn unvalidated capabilities into shipped claims.
