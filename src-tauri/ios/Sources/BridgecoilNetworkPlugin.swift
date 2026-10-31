import Foundation
import Network
import Tauri
import Darwin

// Use declared Bonjour types so iOS discovery needs no raw-multicast entitlement.
final class BridgecoilNetworkPlugin: Plugin, NetServiceDelegate {
    private var browsers: [NWBrowser] = []
    private var resolvers: [String: NetService] = [:]
    private var live: [String: Set<String>] = [:]
    private var rows: [String: (String, [String: Any])] = [:]
    private var warnings: Set<String> = []
    private var pending: Invoke?
    private var deadline: DispatchWorkItem?
    private var deniedTypes: Set<String> = []
    private func key(_ type: String, _ name: String) -> String {
        "\(type.lowercased().trimmingCharacters(in: CharacterSet(charactersIn: ".")))/\(name)"
    }

    @objc public func discover(_ invoke: Invoke) {
        DispatchQueue.main.async { self.start(invoke) }
    }

    private func start(_ invoke: Invoke) {
        guard pending == nil else { invoke.reject("A discovery scan is already running."); return }
        pending = invoke
        rows = [:]; live = [:]; warnings = []; deniedTypes = []
        for type in ["_adb-tls-connect._tcp", "_adb-tls-pairing._tcp", "_adb._tcp"] {
            let browser = NWBrowser(for: .bonjour(type: type, domain: "local."), using: .tcp)
            browser.stateUpdateHandler = { [weak self] state in
                guard let self, self.pending != nil else { return }
                switch state {
                case .waiting(let error), .failed(let error):
                    if error == .dns(-65570) { self.deniedTypes.insert(type) }
                    else { self.warnings.insert("Some ADB services could not be browsed.") }
                case .ready: self.deniedTypes.remove(type)
                default: break
                }
            }
            browser.browseResultsChangedHandler = { [weak self] results, _ in
                guard let self, self.pending != nil else { return }
                var current: Set<String> = []
                for result in results.prefix(64) {
                    guard case let .service(name, serviceType, domain, _) = result.endpoint else { continue }
                    let id = self.key(serviceType, name)
                    current.insert(id)
                    if self.resolvers[id] == nil && self.resolvers.count < 64 {
                        let service = NetService(domain: domain, type: serviceType, name: name)
                        service.delegate = self
                        self.resolvers[id] = service
                        service.resolve(withTimeout: 3)
                    }
                }
                let removed = (self.live[type] ?? []).subtracting(current)
                for id in removed { self.resolvers.removeValue(forKey: id)?.stop() }
                self.rows = self.rows.filter { !removed.contains($0.value.0) }
                self.live[type] = current
            }
            browsers.append(browser)
            browser.start(queue: .main)
        }
        let end = DispatchWorkItem { [weak self] in self?.finish() }
        deadline = end
        DispatchQueue.main.asyncAfter(deadline: .now() + 4, execute: end)
    }

    func netServiceDidResolveAddress(_ sender: NetService) {
        guard pending != nil, sender.port > 0, sender.port <= 65535 else { return }
        let id = key(sender.type, sender.name)
        guard resolvers[id] === sender else { return }
        for data in (sender.addresses ?? []).prefix(4) {
            let host: String? = data.withUnsafeBytes { bytes in
                guard let address = bytes.baseAddress?.assumingMemoryBound(to: sockaddr.self), bytes.count >= MemoryLayout<sockaddr>.size else { return nil }
                var buffer = [CChar](repeating: 0, count: Int(NI_MAXHOST))
                guard getnameinfo(address, socklen_t(bytes.count), &buffer, socklen_t(buffer.count), nil, 0, NI_NUMERICHOST) == 0 else { return nil }
                return String(cString: buffer)
            }
            guard let host, rows.count < 128 else { continue }
            rows["\(sender.type)/\(host)/\(sender.port)"] = (id, ["name": String(sender.name.prefix(160)), "serviceType": sender.type, "host": host, "port": sender.port])
        }
    }

    func netService(_ sender: NetService, didNotResolve errorDict: [String: NSNumber]) {
        if pending != nil { warnings.insert("Some ADB addresses could not be resolved. Try Find again.") }
    }

    private func finish() {
        guard let invoke = pending else { return }
        pending = nil
        for browser in browsers { browser.cancel() }
        for service in resolvers.values { service.stop(); service.delegate = nil }
        browsers = []; resolvers = [:]; live = [:]
        deadline?.cancel(); deadline = nil
        if !deniedTypes.isEmpty && rows.isEmpty {
            invoke.reject("Local network access is off. Allow Bridgecoil in iOS Settings → Privacy & Security → Local Network, then retry.", code: "permission")
        } else {
            invoke.resolve(["services": rows.values.map { $0.1 }, "warnings": Array(warnings.prefix(3))])
        }
        rows = [:]
    }
}

@_cdecl("init_plugin_bridgecoil_network")
public func initBridgecoilNetworkPlugin() -> Plugin { BridgecoilNetworkPlugin() }
