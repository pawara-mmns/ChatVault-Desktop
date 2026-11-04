use super::error::StorageError;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageOverview {
    pub database_status: &'static str,
    pub database_path: String,
    pub schema_version: u32,
    pub vault_path: String,
    pub vault_status: &'static str,
    pub vault_error: Option<StorageError>,
    pub backup_snapshots: u64,
    pub chat_archives: u64,
    pub vault_bytes: Option<u64>,
}
