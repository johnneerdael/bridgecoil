pub mod apk;
pub mod backend;
pub mod identity;
mod manifest;
pub mod ratings;
pub mod remote;
pub mod toolkit;
pub mod workspace;
pub fn validate_endpoint(value: &str) -> Result<String, String> {
    let value = value.trim();
    let (host, port) = value
        .rsplit_once(':')
        .ok_or("Enter the TV address and port.")?;
    if host.is_empty()
        || host.contains("://")
        || host
            .chars()
            .any(|c| c.is_whitespace() || !c.is_ascii_alphanumeric() && !".-:[]%_".contains(c))
    {
        return Err("Enter a hostname or IP address followed by a port.".into());
    }
    if host.contains(':') && !(host.starts_with('[') && host.ends_with(']')) {
        return Err("Put IPv6 addresses in square brackets.".into());
    }
    if port.parse::<u16>().ok().filter(|p| *p != 0).is_none() {
        return Err("Port must be between 1 and 65535.".into());
    }
    Ok(value.to_owned())
}

pub fn validate_pairing_code(code: &str) -> Result<(), String> {
    if code.len() == 6 && code.bytes().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err("Enter the six-digit pairing code displayed on the TV.".into())
    }
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn install_succeeded(code: Option<u8>, output: &str) -> bool {
    code == Some(0) && output.trim() == "Success"
}

pub mod signing;

pub mod connection;
pub mod discovery;

pub mod apk_set;
pub mod wake;
