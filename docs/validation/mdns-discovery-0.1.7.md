# ADB discovery — 0.1.7

Find is available in connection setup, Saved TVs, and the connected-device dialog. A bounded four-second scan browses `_adb-tls-connect._tcp`, `_adb-tls-pairing._tcp`, and `_adb._tcp`. Results identify connection, pairing, and legacy ports separately; choosing a result fills the corresponding form. Pairing still requires the code and legacy ADB still requires explicit consent. Browsing while connected leaves that connection intact. Choosing another TV disconnects the current one, as explained in the picker.

Quick connect scans before using its saved endpoint. It matches a learned service instance, a pairing GUID when a stable saved device identity exists, or an unambiguous instance at the saved host. Pairing ports are never used for connection, and legacy discovery does not enable legacy consent. Multicast failure falls back to the saved endpoint; denied network permission remains a permission error. Native connect retains the saved reported-device-identity check. Cancelling during discovery prevents a later result from starting a connection.

Multiple advertised IPv4/IPv6 addresses are grouped into one visible row per service, preferring IPv4. Distinct service instances remain separate. Only the picker results scroll, with Close and Find again kept visible. Native browsers/resolvers stop at the scan deadline; desktop shuts down its temporary daemon. No background discovery loop or port sweep is introduced.

## Platform choice

Desktop uses pinned [`mdns-sd` 0.21.6](https://docs.rs/mdns-sd/0.21.6/mdns_sd/), which provides resolved DNS-SD services and explicit daemon shutdown. The alternatives were reviewed through their primary documentation: [`mdns` 3.0.0](https://docs.rs/mdns/3.0.0/mdns/) exposes DNS records; [`simple-mdns` 0.7.0](https://docs.rs/simple-mdns/0.7.0/simple_mdns/) offers sync/Tokio discovery and one-shot resolution; [`agnostic-mdns` 0.4.2](https://docs.rs/agnostic-mdns/0.4.2/agnostic_mdns/) supports several async runtimes. The additional general runtime options are not needed here.

Android uses [NsdManager](https://developer.android.com/develop/connectivity/wifi/use-nsd), resolving serially for compatibility with older phones. iOS uses NWBrowser and NetService with the three declared NSBonjourServices. This follows [Apple local-network privacy guidance](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy) and avoids raw multicast entitlement requirements for declared Bonjour types. The native Swift plugin is linked as a local static package through Tauri's Apple build helper. The [AOSP ADB Wi-Fi specification](https://android.googlesource.com/platform/packages/modules/adb/+/HEAD/docs/dev/adb_wifi.md) defines the service types.

## Evidence and limits

- 55 frontend tests passed, including changed-port refresh, separation of service kinds, ambiguous instances, address grouping, cancellation, permission denial, form filling, and browsing without disconnecting. Svelte check reported no errors or warnings.
- Three Rust discovery-normalization tests passed. The read-only desktop scanner discovered the TCL's two advertised TLS instances and a legacy ADB peer. Discovery itself opened no ADB connection.
- Production Android ARM64 debug APK and unsigned iOS device archive built successfully. The iOS archive contains version 0.1.7 and all three Bonjour declarations; no physical iOS discovery test has been performed.
- Installed to USB CPH2747 and cold-launched. Android native discovery returned the TCL advertisements at 192.168.50.12:46389 and :46833. These are advertised endpoints, not proof that both accept ADB. The initial landscape screenshot exposed a clipped retry control; the picker layout was then corrected to scroll its results independently. A final portrait phone screenshot showed both TCL service instances and the legacy peer at .89:5555, with Close and Find again fully visible. Landscape was not rechecked after that correction.
- Agent validation did not pair with, connect to, or modify a TV. Physical changed-port Quick connect and fresh pairing-service discovery remain user acceptance checks.

Artifact details are recorded in `artifacts/mdns-discovery-0.1.7.json`.
