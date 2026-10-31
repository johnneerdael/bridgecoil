# Take Back Control Implementation Plan

Delivers tasks 11, 12, 13 and 15 of `2026-10-30-adb-workspace.md`, the core promise of `specs/2026-10-30-adb-workspace-design.md`: anyone can take back control of their TV without being an expert, remap its buttons, and have those mappings maintained by scripts on the TV.

The user requested this directly and asked for continuous work without questions. Decisions below are the implementer's, within the approved spec.

## Rules

- Bridgecoil is for the public, on any Android TV. Nothing may depend on the owner's TCL setup: no fixed scan codes, ABI paths, app targets or vendor assumptions. The TCL scripts are evidence that the approach works, not a template.
- Everything is detected per device, and anything unsupported says so plainly.
- Managers offered: Shizuku-Next and Porter only. An existing manager is recognised by its signing certificate, never replaced silently.
- Nobody pre-downloads anything: the app fetches the latest release, verifies it and installs it.
- No typing is required for any of this, including advanced commands.
- A step is only "done" after a probe proves it. Installed never means ready.
- Test on emulators only while unattended. Never change the user's TV or phone. Every user-facing push to `main` releases, at the owner's direction on 2026-10-31.

## Proven on an API 36 emulator, 2026-10-31

| Link | Evidence |
|---|---|
| Install managers and Termux with `pm install` | Success for all |
| Start the server over ADB | `<apk dir>/lib/<abi dir>/libshizuku.so`, then `pidof shizuku_server` |
| Termux bootstrap | One launch of `.app.TermuxActivity` |
| Configure Termux with no typing | GitHub Termux builds are debuggable, so `run-as com.termux` works from ADB |
| rish | Copy `assets/rish` and `assets/rish_shizuku.dex` from the manager APK into Termux, dex mode 400, set `RISH_APPLICATION_ID=com.termux` and `MANAGER_APPLICATION_ID` |
| rish authorization | One "Allow Termux" prompt on the TV, then `uid=2000(shell)` with the `input` group |
| Listener | `getevent` runs under rish and stays alive with `nohup` |
| Plain Termux | Can't read `/dev/input`, so the listener must run as the shell user |

Porter ships `assets/porsh`, `assets/porsh.dex` and `libporter.so`, and uses `PORSH_APPLICATION_ID`. Both managers have their own start-on-boot.

## Catalog

| Tool | Repo | Package | Certificate SHA-256 | Asset |
|---|---|---|---|---|
| Shizuku-Next | `rushiranpise/Shizuku-Next` | `moe.shizuku.privileged.api` | `06360bb800d1a117c3a22c91a58e638bcd37281cb4d644fcfac5d7238c654293` | `shizuku-v*-next.apk`, never `-debug` |
| Porter | `d4rken-org/porter` | `eu.darken.porter` | `c6302c2b163c7f42f6d8bd658e53387880511020136a083f0a6a4bdc20b4bfa0` | `porter-v*-release.apk` |
| Porter Compatibility | same | `moe.shizuku.privileged.api` | same as Porter | `porter-compat-v*-release.apk` |
| Termux | `termux/termux-app` | `com.termux` | `b6da01480eefd5fbf2cd3771b8d1021ec791304bdd6c4bf41d3faabad48ee5e1` | `termux-app_v*+github-debug_<abi>.apk` |
| Termux:Boot | `termux/termux-boot` | `com.termux.boot` | same as Termux | `termux-boot-app_v*+github.debug.apk` |

The certificate is pinned, so a release signed by anyone else is refused even if GitHub serves it. Asset digests are checked when GitHub provides them.

## Work

- [x] **A. Toolkit release backend.** `crates/adb-core/src/toolkit/{mod,catalog,release}.rs`: select the right asset for the TV's ABIs, fetch latest, download with a size limit, verify digest and pinned certificate, install through `Client::install`.
- [x] **B. Toolkit inventory.** `toolkit/inventory.rs`: distinct states for missing, wrong variant, installed, server running, Termux ready, shell authorized, boot monitor present, listener running. Recognise a stub or unknown signer.
- [x] **C. Guided setup steps.** `toolkit/setup.rs`: start server, launch Termux, deploy rish or porsh, request authorization, verify `uid=2000`, Tauri commands, wizard UI replacing the status list and "Choose a tool APK".
- [x] **D. Key learning.** `crates/adb-core/src/remote/{capture,mappings}.rs`: bounded `getevent` capture, parser keeping namespace, scan code and source name. Never invents friendly names.
- [x] **E. Listener and mapping.** Generate the listener from validated fields, with no `eval`. Deploy to `/data/local/tmp/bridgecoil/` through a reviewed action, detect conflicts, remove and pause, disclose that the original key action still happens.
- [x] **F. Startup and upkeep.** Reviewed Termux:Boot monitor written with `run-as`, bounded retry, health and logs, removal of only app-owned files, separate fields for phone-disconnected and cold-boot tests.
- [x] **G. Remote UI.** Learn, map, test, pause, remove, health.
- [x] **H. Command builder.** Task 15: a catalog of labelled commands with explained options, picked from dropdowns, previewed, reviewed and run with no typing.
- [x] **I. End-to-end emulator run.** Proven with `tests/toolkit_live.rs` for both managers. A real key press opened the mapped app, pause silenced it, and after a cold reboot the Termux:Boot monitor restored the listener within 60 seconds of the manager returning.
- [x] **Independent review.** No Critical findings. Fixed all five Important issues: endless prompts from the boot monitor, a listener that could outlive removal on older Android, an unflagged lockout risk and hardcoded profile in Commands, a stale monitor lock, and a dead end on TVs that block `run-as`.
- [x] **K. Launcher change.** Try, default through review, press Home on the real remote, then turn off the old launcher, with a way back.
- [x] **L. Clean up.** Offers the preinstalled apps rated Recommended, ticked and explained, disables through bulk review and can turn them back on.
- [x] **M. Richer mappings.** Tap and hold, and acting as a different button from a fixed list. A real long press was proven on an emulator. A tap mapping also fires on a slow press when no hold is mapped.
- [x] **N. Evidence of restarts.** The saved boot ID is compared with the running one, so "came back by itself" is only shown after it has happened.
- [x] **O. Capture and diagnostics.** Screenshots, bounded recordings, filtered logs and native save, with a live 3 MB download matched by SHA-256.
- [x] **P. Saved commands.** Filled-in commands are kept on the phone and rerun in one step.
- [x] **Q. Android TV image.** Both managers walked from a bare TV to Done on a `user` build with secure ADB and no root, plus a real key, a cold boot and capture. This found the Play Protect block on Termux, which is now guided. See `docs/validation/android-tv-emulator.md`.
- [x] **Second independent review.** No Critical findings. Fixed all four Important issues and all five Minor ones.
- [x] **R. Wired TVs.** Android refuses wireless debugging on Ethernet, so setup detects a wired TV and says what that costs. See `docs/validation/android-tv-emulator.md`.
- [x] **S. Launcher handover.** Android TV's launcher outranks any other, so choosing one changed nothing. The chosen launcher is now reported, and the one in use can be turned off once a different installed launcher holds the Home role. Proven through the engine with a real Home key, a reboot and the way back.
- [ ] **J. On a real TV.** The access prompt, a physical remote, start at boot and a cold boot passed on a Ugoos AM6 on Android 9. See `docs/validation/real-tv-checklist.md`. Code pairing on a real TV is still open, and needs Android 11 or later on Wi-Fi.

## Left out, on purpose

- Port forwarding and reverse. Nothing here needs them, and on a phone they'd serve developers rather than TV owners.
- Multi-step sequences. Each command still gets its own review, which suits a tool for non-experts.
- File copy and multi-select. Neither was asked for. Move, rename, delete, upload and save cover the requested file work.

## Verification

`npx vitest run`, `npm run -s check`, `node scripts/cargo.mjs test --workspace`, then the emulator run. Commit and push to `main` at tested milestones. A `feat:` or `fix:` push publishes a release, so don't push one before it's tested. See `docs/release.md`.
