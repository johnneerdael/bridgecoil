use crate::{backend::PocError, shell_quote};
pub fn package_arg(value: &str) -> Result<String, PocError> {
    if value.len() <= 255 && (value == "android" || crate::apk::valid_package(value)) {
        Ok(shell_quote(value))
    } else {
        Err(PocError::new(
            "input",
            "Invalid Android package identifier.",
        ))
    }
}
pub fn component_arg(value: &str) -> Result<String, PocError> {
    let Some((package, class)) = value.split_once('/') else {
        return Err(PocError::new("input", "Choose an installed app activity."));
    };
    package_arg(package)?;
    if class.is_empty()
        || class.len() > 1024
        || !class
            .chars()
            .all(|c| c.is_alphanumeric() || "._$".contains(c))
    {
        return Err(PocError::new(
            "input",
            "Invalid Android activity component.",
        ));
    }
    Ok(shell_quote(value))
}
pub fn remote_path(value: &str) -> Result<String, PocError> {
    if !value.starts_with('/') || value.contains('\0') || value.len() > 4096 {
        return Err(PocError::new("input", "Choose an absolute device path."));
    }
    let mut parts = Vec::new();
    for p in value.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(PocError::new("input", "Path escapes the device root."));
                }
            }
            _ => parts.push(p),
        }
    }
    Ok(format!("/{}", parts.join("/")))
}
