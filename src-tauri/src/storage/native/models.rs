use crate::storage::error::{Result, StorageError};
use serde::Serialize;
pub const PARSER_VERSION: i64 = 1;
pub fn fail(code: &'static str) -> StorageError {
    StorageError::new(
        code,
        match code {
            "native_busy" => {
                "A native reader or another vault operation is running. Finish or cancel its preview first."
            }
            "native_cancelled" => {
                "Native backup processing was cancelled. No readable archive was saved."
            }
            "native_key" => {
                "The provided key has an unsupported structure. Select a matching local crypt14 key file or a supported crypt15 64-character hexadecimal root key."
            }
            "native_auth" => {
                "Authentication failed. The key does not match, or the encrypted file is damaged. No readable archive was saved."
            }
            "native_format" => {
                "This encrypted envelope or key mode is not supported. Original files remain encrypted and unchanged."
            }
            "native_integrity" => {
                "Snapshot ownership, inventory, size or SHA-256 could not be verified. The original snapshot was preserved."
            }
            "native_schema" => {
                "The authenticated database does not have a supported message-table schema. No conversations were fabricated."
            }
            "native_database" => {
                "The authenticated data is not a valid supported SQLite database, or a database operation failed. Partial readable imports were rolled back."
            }
            "native_limit" => {
                "The operation reached a bounded file, decompression, row, field or time limit. No partial readable archive was saved."
            }
            "native_consent" => {
                "Explicit consent is required before processing plaintext or saving readable messages into the unencrypted vault and SQLite database."
            }
            "native_expired" => {
                "This verified preview has expired or is no longer available. Unlock the selected backup again."
            }
            "native_duplicate" => {
                "This snapshot has already been imported by this parser. Open the existing archive. A newer parser can create a separate revision without overwriting it."
            }
            _ => {
                "The native reader could not finish. Check the selected snapshot and local storage, then retry. Original backups were preserved."
            }
        },
    )
}
pub fn sql<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|_| fail("native_database"))
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub file_id: String,
    pub filename: String,
    pub bytes: u64,
    pub format: String,
    pub capability: String,
    pub key_requirement: String,
    pub hash_verified: bool,
    pub duplicate_of: Option<String>,
    pub issue: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub local_integrity: String,
    pub snapshot_id: String,
    pub source_app: Option<String>,
    pub created_at: u64,
    pub phase: String,
    pub inventory_complete: bool,
    pub warnings: Vec<String>,
    pub capability: String,
    pub candidates: Vec<Candidate>,
    pub media: Vec<(String, usize, u64)>,
    pub missing_files: usize,
    pub total_files: usize,
    pub existing_archive: Option<String>,
}
#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub conversations: u64,
    pub messages: u64,
    pub media_matched: u64,
    pub media_missing: u64,
    pub unsupported: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub token: String,
    pub snapshot_id: String,
    pub file_id: String,
    pub format: String,
    pub schema: String,
    pub parser_version: i64,
    pub counts: Counts,
    pub warnings: Vec<String>,
    pub archive_id: Option<String>,
}
#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: String,
    pub bytes: u64,
    pub messages: u64,
    pub cancel_requested: bool,
    pub error: Option<StorageError>,
}
