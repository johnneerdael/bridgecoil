// swift-tools-version:5.5
import PackageDescription
let package = Package(
    name: "BridgecoilNetwork",
    platforms: [.iOS(.v15)],
    products: [.library(name: "BridgecoilNetwork", type: .static, targets: ["BridgecoilNetwork"])],
    dependencies: [.package(name: "Tauri", path: "../.tauri/tauri-api")],
    targets: [.target(name: "BridgecoilNetwork", dependencies: [.byName(name: "Tauri")], path: "Sources")]
)
