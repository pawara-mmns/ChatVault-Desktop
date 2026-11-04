use serde::{Deserialize, Serialize};
pub type Result<T> = std::result::Result<T, Failure>;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub code: String,
    pub message: String,
    pub diagnostic: Option<String>,
}
pub fn fail(code: &str) -> Failure {
    Failure { code:code.into(), message:match code {
 "cancelled"=>"Backup cancelled. Incomplete work is isolated; previous snapshots and phone files are unchanged.",
 "driver_pending"=>"The scan time budget elapsed while Windows was still returning from an operation. Cancellation is requested; the device stays reserved until its worker safely exits. Do not start another scan yet.",
 "busy"=>"Another device, import, backup or vault operation is running. Wait for it to finish and retry.",
 "plan_expired"=>"The backup inventory expired. Refresh the device and scan again before copying.",
 "invalid_selection"=>"Select actual inventoried files from one WhatsApp source. Refresh the inventory if it changed.",
 "insufficient_space"=>"The vault drive does not have enough available space for this selection. Free space or choose fewer files.",
 "capacity_unknown"=>"Available space or source sizes could not be established. Acknowledge that uncertainty before starting.",
 "incomplete_inventory"=>"The inventory is incomplete. Review its restrictions and acknowledge that only inventoried files will be copied.",
 "source_changed"=>"A source file changed or was truncated. This snapshot is incomplete; rescan and retry when WhatsApp is idle.",
 "hash_mismatch"=>"Integrity verification failed. This snapshot is not a completed backup; preserve it for inspection or retry.",
 "unsafe_path"=>"Snapshot ownership or a local path could not be validated. Existing files were preserved.",
 "limit"=>"This operation reached its bounded file, depth, size or time limit. Choose fewer categories or files.",
 "not_found"=>"This snapshot or backup inventory is unavailable. Refresh the list.",
 "vault_unavailable"=>"The configured vault cannot be written or reached. Reconnect its drive and check available space and permissions.",
 "unsupported"=>"The device driver does not provide this read-only resource stream. No simulated copy was made.",
 "disconnected"=>"The phone disconnected or its file access changed. Unlock it, choose File Transfer, refresh and rescan.",
 "access_denied"=>"The phone denied file access. Unlock it, approve File Transfer and retry.",
 "timeout"=>"The operation timed out between Windows calls. A blocked driver call may still be returning; wait before retrying.",
 "storage_full"=>"Writing the local copy failed because the destination is full. Free space and retry.",
 "database"=>"Backup metadata could not be committed. Incomplete files remain isolated for recovery; no database was reset.",
 _=>"The backup operation could not finish. Check the device, vault permissions and disk space, then retry."
 }.into(), diagnostic:None }
}
impl From<crate::devices::models::DeviceError> for Failure {
    fn from(e: crate::devices::models::DeviceError) -> Self {
        let mut f = fail(e.code);
        f.diagnostic = e.diagnostic;
        f
    }
}
impl From<super::super::error::StorageError> for Failure {
    fn from(_: super::super::error::StorageError) -> Self {
        fail("vault_unavailable")
    }
}
impl From<rusqlite::Error> for Failure {
    fn from(_: rusqlite::Error) -> Self {
        fail("database")
    }
}
pub fn io(e: std::io::Error) -> Failure {
    fail(if matches!(e.raw_os_error(), Some(28 | 39 | 112)) {
        "storage_full"
    } else {
        "vault_unavailable"
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Whatsapp,
    Business,
}
impl Source {
    pub fn key(self) -> &'static str {
        match self {
            Self::Whatsapp => "whatsapp",
            Self::Business => "business",
        }
    }
    pub fn base(self) -> &'static str {
        match self {
            Self::Whatsapp => "WhatsApp",
            Self::Business => "WhatsApp Business",
        }
    }
    pub fn package(self) -> &'static str {
        match self {
            Self::Whatsapp => "com.whatsapp",
            Self::Business => "com.whatsapp.w4b",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Databases,
    Images,
    Videos,
    Documents,
    Audio,
    VoiceNotes,
    Other,
}
impl Category {
    pub fn key(self) -> &'static str {
        match self {
            Self::Databases => "databases",
            Self::Images => "images",
            Self::Videos => "videos",
            Self::Documents => "documents",
            Self::Audio => "audio",
            Self::VoiceNotes => "voice_notes",
            Self::Other => "other",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub id: String,
    pub category: Category,
    pub source_relative: String,
    pub local_relative: String,
    pub original_name: String,
    pub expected_size: Option<u64>,
    pub actual_bytes: u64,
    pub sha256: Option<String>,
    pub local_verification: String,
    pub source_verification: String,
    pub size_validation: String,
    pub state: String,
    pub error_code: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySummary {
    pub category: Category,
    pub files: usize,
    pub known_bytes: u64,
    pub unknown_sizes: usize,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub scanned_categories: Vec<Category>,
    pub skipped_entries: usize,
    pub inaccessible_locations: usize,
    pub id: String,
    pub device_id: String,
    pub device_name: Option<String>,
    pub source: Source,
    pub complete: bool,
    pub warnings: Vec<String>,
    pub categories: Vec<CategorySummary>,
    pub total_files: usize,
    pub known_bytes: u64,
    pub unknown_sizes: usize,
    pub available_bytes: Option<u64>,
    pub vault_path: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub categories: Vec<Category>,
    pub file_ids: Option<Vec<String>>,
    pub source_reread: bool,
    pub acknowledge_incomplete: bool,
    pub acknowledge_capacity: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ScanOptions {
    pub categories: Vec<Category>,
    pub max_objects: usize,
    pub max_files: usize,
    pub max_depth: usize,
    pub timeout_seconds: u64,
}
impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            categories: vec![],
            max_objects: 100_000,
            max_files: 50_000,
            max_depth: 16,
            timeout_seconds: 60,
        }
    }
}
impl ScanOptions {
    pub fn validate(&self) -> Result<()> {
        if self.categories.len() > 7
            || !(1..=100_000).contains(&self.max_objects)
            || !(1..=50_000).contains(&self.max_files)
            || self.max_depth > 16
            || !(1..=180).contains(&self.timeout_seconds)
        {
            return Err(fail("invalid_selection"));
        }
        Ok(())
    }
    pub fn includes(&self, category: Category) -> bool {
        self.categories.is_empty() || self.categories.contains(&category)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub operation_id: Option<String>,
    pub revision: u64,
    #[serde(skip_deserializing)]
    pub scan: Option<crate::devices::scan::ScanReport>,
    pub snapshot_id: Option<String>,
    pub phase: String,
    pub source: Option<Source>,
    pub current_filename: Option<String>,
    pub total_files: usize,
    pub successful_files: usize,
    pub failed_files: usize,
    pub copied_bytes: u64,
    pub known_total_bytes: u64,
    pub unknown_sizes: usize,
    pub verified_bytes: u64,
    pub rate_bytes_per_second: Option<u64>,
    pub cancel_requested: bool,
    pub error: Option<Failure>,
    pub updated_at: u64,
}
impl Default for Progress {
    fn default() -> Self {
        Self {
            operation_id: None,
            revision: 0,
            scan: None,
            snapshot_id: None,
            phase: "idle".into(),
            source: None,
            current_filename: None,
            total_files: 0,
            successful_files: 0,
            failed_files: 0,
            copied_bytes: 0,
            known_total_bytes: 0,
            unknown_sizes: 0,
            verified_bytes: 0,
            rate_bytes_per_second: None,
            cancel_requested: false,
            error: None,
            updated_at: crate::devices::models::now(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub source_app: Option<String>,
    pub device_name: Option<String>,
    pub phase: String,
    pub created_at: u64,
    pub completed_at: Option<u64>,
    pub file_count: usize,
    pub successful_count: usize,
    pub failed_count: usize,
    pub skipped_count: usize,
    pub total_bytes: u64,
    pub verified_bytes: u64,
    pub manifest_path: Option<String>,
    pub local_integrity: String,
    pub local_verified_at: Option<u64>,
    pub warnings: Vec<String>,
    pub destination: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    #[serde(default)]
    pub scanned_categories: Vec<Category>,
    #[serde(default)]
    pub skipped_entries: usize,
    #[serde(default)]
    pub inaccessible_locations: usize,
    pub schema_version: u32,
    pub snapshot_id: String,
    pub created_at: u64,
    pub source: Source,
    pub device_display_name: Option<String>,
    pub device_association: Option<String>,
    pub backup_mode: String,
    pub phase: String,
    pub inventory_complete: bool,
    pub warnings: Vec<String>,
    pub source_consistency: String,
    pub files: Vec<FileEntry>,
}
