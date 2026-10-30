use crate::backend::PocError;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub device_id: String,
    pub session_id: String,
    pub profile_id: u32,
}
pub fn validate_target(current: Option<&Target>, requested: &Target) -> Result<(), PocError> {
    if current == Some(requested) {
        Ok(())
    } else {
        Err(PocError::new(
            "staleTarget",
            "The device, connection or profile changed. Review this action again.",
        ))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationStatus {
    Queued,
    Running,
    AwaitingUser,
    Sent,
    Verified,
    Failed,
    Partial,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub id: String,
    pub target: Target,
    pub title: String,
    pub kind: String,
    pub status: OperationStatus,
    pub created_ms: i64,
    pub updated_ms: i64,
    pub output: String,
    pub error: Option<String>,
    pub recovery: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}
