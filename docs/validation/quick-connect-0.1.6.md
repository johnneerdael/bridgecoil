# Saved devices and Quick connect — 0.1.6

Saved TVs appear on the disconnected Device/connection screen and remain available across process restarts, explicit disconnects, and failed reconnects. Quick connect uses the last successful connection endpoint and the existing app-private host key. Successfully paired TVs are also retained, with a request for their separate connection address when none is known. The previous build's stored connection endpoint and pairing-status records migrate into this view, without requesting a new pairing code.

After a successful connection, the entry stores the model, stable reported device identity when available, endpoint, transport preference, and connection time. Native reconnect checks a known saved identity before exposing the workspace, disconnecting if a different device reports itself at that address. Session-only identities are not persisted as stable identities. A saved legacy connection retains its previously granted legacy transport choice; migrated entries require explicit consent if legacy ADB is needed. No pairing code or private key is stored in the saved-device metadata.

Validation: 36 frontend tests passed; Svelte reported zero errors and warnings; the production Android ARM64 debug APK built successfully. Tests cover previous-address migration, distinct pairing/connection ports, stable identities across address changes, reused addresses, reload/reconnect, failed reconnect retention, and address editing. Package Manager reports 0.1.6 / code 1006 on the USB-connected CPH2747. A phone screenshot after installation showed the connected TCL workspace. The user then reported that this worked well. Agent actions did not trigger a TV connection attempt during this update.

Quick connect currently uses the last known address. Automatic mDNS discovery and changed-port refresh remain pending.

Artifact: `artifacts/Bridgecoil-0.1.6-arm64.apk`  
Size: 45,287,956 bytes  
SHA-256: `48abe8706a6129aa5c13380bfc211f235621d0e7a12a19ddb20a055f30a20b23`
