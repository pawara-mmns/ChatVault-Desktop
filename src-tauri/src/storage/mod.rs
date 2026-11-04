pub mod commands;
mod database;
pub mod error;
pub mod imports;
mod migrations;
pub mod models;
mod paths;
mod repository;
mod service;
mod usage;
mod vault;
pub mod viewer;

use error::{Result, StorageError};
use service::StorageService;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct StorageState(Arc<Mutex<Runtime>>);
struct Runtime {
    app_local: Result<PathBuf>,
    protected: Vec<PathBuf>,
    service: Option<StorageService>,
    failure: Option<StorageError>,
}

impl StorageState {
    pub fn new(app_local: Result<PathBuf>) -> Self {
        let mut protected = vec![
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .to_path_buf(),
        ];
        if let Ok(executable) = std::env::current_exe() {
            if let Some(parent) = executable.parent() {
                protected.push(parent.to_path_buf());
            }
        }
        #[cfg(windows)]
        for variable in [
            "WINDIR",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramData",
            "OneDrive",
            "OneDriveConsumer",
            "OneDriveCommercial",
        ] {
            if let Some(path) = std::env::var_os(variable) {
                protected.push(PathBuf::from(path));
            }
        }
        Self(Arc::new(Mutex::new(Runtime {
            app_local,
            protected,
            service: None,
            failure: None,
        })))
    }

    pub(crate) fn run<T>(
        &self,
        retry: bool,
        operation: impl FnOnce(&mut StorageService) -> Result<T>,
    ) -> Result<T> {
        let mut runtime = self.0.lock().map_err(|_| {
            StorageError::new(
                "storage_busy",
                "The storage service could not be accessed. Restart the application.",
            )
        })?;
        if runtime.service.is_none() {
            if !retry {
                if let Some(error) = &runtime.failure {
                    return Err(error.clone());
                }
            }
            let opened = runtime
                .app_local
                .clone()
                .and_then(|path| StorageService::open(&path, &runtime.protected));
            match opened {
                Ok(service) => {
                    runtime.service = Some(service);
                    runtime.failure = None;
                }
                Err(error) => {
                    runtime.failure = Some(error.clone());
                    return Err(error);
                }
            }
        }
        operation(
            runtime
                .service
                .as_mut()
                .ok_or_else(StorageError::database)?,
        )
    }
}

#[cfg(test)]
mod tests;
