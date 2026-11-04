pub mod commands;
pub mod media;
pub mod models;
pub mod protocol;
mod repository;
pub mod search;
#[cfg(test)]
mod tests;

use super::error::{Result, StorageError};
fn failure(code: &'static str, message: &'static str) -> StorageError {
    StorageError::new(code, message)
}
fn id(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(failure(
            "invalid_request",
            "The requested record identifier is invalid.",
        ));
    }
    Ok(())
}
