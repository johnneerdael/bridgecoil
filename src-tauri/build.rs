fn main() {
    tauri_build::build();
    #[cfg(target_os = "macos")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        // Link the native plugin into Rust as well as the final app archive.
        // Cargo also builds a dylib, so app-only Swift sources cannot provide its initializer.
        let root =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("network-swift");
        let api = std::env::var_os("DEP_TAURI_IOS_LIBRARY_PATH").expect("Tauri iOS API path");
        copy_sources(std::path::Path::new(&api), &root.join(".tauri/tauri-api"));
        copy_sources(std::path::Path::new("ios"), &root.join("ios"));
        println!("cargo:rerun-if-changed=ios");
        println!("cargo:rerun-if-env-changed=DEP_TAURI_IOS_LIBRARY_PATH");
        tauri_utils::build::link_apple_library("BridgecoilNetwork", root.join("ios"));
    }
}

#[cfg(target_os = "macos")]
fn copy_sources(source: &std::path::Path, target: &std::path::Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if [".build", "Tests", "Package.resolved"]
            .iter()
            .any(|name| entry.file_name() == *name)
        {
            continue;
        }
        let dest = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_sources(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).unwrap();
        }
    }
}
