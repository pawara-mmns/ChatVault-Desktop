use serde::Serialize;
use std::fmt;

/// Safe, stable errors for IPC. Never include SQL values, chat content, or OS diagnostics.
#[derive(Debug, Clone, Serialize)]
pub struct StorageError {
    pub code: &'static str,
    pub message: &'static str,
}

impl StorageError {
    pub const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
    pub fn database() -> Self {
        Self::new(
            "database_unavailable",
            "The local database could not be opened or updated. Check local storage permissions and retry. No database was reset.",
        )
    }
    pub fn filesystem() -> Self {
        Self::new(
            "vault_unavailable",
            "The vault is unavailable. Reconnect its drive and check folder permissions, then retry. Its location has been preserved.",
        )
    }
    pub fn unsafe_path() -> Self {
        Self::new(
            "unsafe_path",
            "Choose a dedicated local folder outside system, application, database, and source directories. Links, network locations, and device paths are not supported.",
        )
    }
    pub fn not_empty() -> Self {
        Self::new(
            "vault_not_empty",
            "The current vault contains files or database records. Changing location requires migration, which is not supported yet. Nothing was moved or deleted.",
        )
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}
impl std::error::Error for StorageError {}
pub type Result<T> = std::result::Result<T, StorageError>;
