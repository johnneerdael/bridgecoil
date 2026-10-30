use libadb::keys::{AdbKey, rsa::rand_core::OsRng, zeroize::Zeroizing};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub fn load_or_create(directory: &Path) -> Result<AdbKey, String> {
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let path = directory.join("host.pk8");
    match fs::read(&path) {
        Ok(bytes) => AdbKey::from_pkcs8_der(&Zeroizing::new(bytes), &mut OsRng, "ADB Studio")
            .map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let key = AdbKey::generate(&mut OsRng, "ADB Studio").map_err(|e| e.to_string())?;
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(mut file) => {
                    file.write_all(key.to_pkcs8_der().map_err(|e| e.to_string())?.as_bytes())
                        .map_err(|e| e.to_string())?;
                    file.sync_all().map_err(|e| e.to_string())?;
                    Ok(key)
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    load_or_create(directory)
                }
                Err(e) => Err(e.to_string()),
            }
        }
        Err(e) => Err(e.to_string()),
    }
}
