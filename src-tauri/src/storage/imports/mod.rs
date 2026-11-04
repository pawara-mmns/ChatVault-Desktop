mod archive_repository;
pub mod commands;
mod input;
mod media;
pub mod models;
mod parser;
pub(super) mod service;
pub(super) mod staging;
#[cfg(test)]
mod tests;
mod zip_processor;

use super::error::{Result, StorageError};
fn error(code: &'static str, message: &'static str) -> StorageError {
    StorageError::new(code, message)
}
