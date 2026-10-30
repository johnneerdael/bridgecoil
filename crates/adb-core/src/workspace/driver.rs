use super::types::Target;
use crate::backend::PocError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<u8>,
}
impl ShellOutput {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
    pub fn combined(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}
#[async_trait]
pub trait ShellDriver: Send {
    fn current_target(&self) -> Option<Target>;
    async fn shell(&mut self, target: &Target, command: &str) -> Result<ShellOutput, PocError>;
}
pub fn device_identity(serial: &str, manufacturer: &str, session: &str) -> String {
    use sha2::{Digest, Sha256};
    let serial = serial.trim();
    if serial.is_empty()
        || ["unknown", "0", "0123456789ABCDEF", "1234567890"]
            .iter()
            .any(|v| serial.eq_ignore_ascii_case(v))
    {
        return format!("session:{session}");
    }
    let mut hash = Sha256::new();
    hash.update(manufacturer.as_bytes());
    hash.update([0]);
    hash.update(serial.as_bytes());
    format!("android:{}", hex::encode(hash.finalize()))
}
