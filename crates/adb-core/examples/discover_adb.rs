//! Read-only DNS-SD diagnostic: never opens an ADB connection.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = adb_core::discovery::discover(std::time::Duration::from_secs(4))?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
#[cfg(any(target_os = "android", target_os = "ios"))]
fn main() {}
