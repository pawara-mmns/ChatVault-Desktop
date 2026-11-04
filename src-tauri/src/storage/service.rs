use super::{
    database,
    error::{Result, StorageError},
    migrations,
    models::StorageOverview,
    paths, repository, vault,
};
use rusqlite::{Connection, TransactionBehavior};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub struct StorageService {
    pub(crate) connection: Connection,
    pub(super) database_path: PathBuf,
    protected: Vec<PathBuf>,
    pub(super) import_session: Option<super::imports::service::Session>,
    pub(super) import_limits: super::imports::models::ImportLimits,
    pub(super) media_grants: std::collections::HashMap<String, super::viewer::media::Grant>,
}

impl StorageService {
    pub fn open(app_local: &Path, extra_protected: &[PathBuf]) -> Result<Self> {
        // Even an unusual OS/path configuration must not put the database in source or system roots.
        paths::validate_vault_location(app_local, extra_protected)?;
        paths::create_safe_directory(app_local)?;
        let app_local = paths::canonical(app_local)?;
        let database_path = app_local.join("database").join("chatvault.sqlite3");
        let mut connection = database::open(&database_path)?;
        let mut protected = extra_protected.to_vec();
        protected.push(app_local.join("database"));
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::database())?;
        let identity = repository::setting(&transaction, "installation_id")?;
        let vault_path = repository::setting(&transaction, "vault_path")?;
        let initialized = repository::setting(&transaction, "vault_initialized")?;
        if identity.is_none() && vault_path.is_none() && initialized.is_none() {
            repository::save_setting(&transaction, "installation_id", &Uuid::new_v4().to_string())?;
            let default = app_local.join("ChatVault").join("vault");
            repository::save_setting(
                &transaction,
                "vault_path",
                default.to_str().ok_or_else(StorageError::unsafe_path)?,
            )?;
            repository::save_setting(&transaction, "vault_initialized", "false")?;
        } else if identity
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok())
            .is_none()
            || vault_path.as_deref().is_none_or(str::is_empty)
            || !matches!(initialized.as_deref(), Some("true" | "false"))
        {
            return Err(StorageError::new(
                "database_unavailable",
                "The saved vault configuration is incomplete or invalid. It has not been replaced or reset.",
            ));
        }
        transaction.commit().map_err(|_| StorageError::database())?;
        let mut service = Self {
            connection,
            database_path,
            protected,
            import_session: None,
            import_limits: Default::default(),
            media_grants: Default::default(),
        };
        // Initialization failures remain visible as vault errors while the database stays usable.
        // A persisted, initialized path is NEVER recreated automatically on restart.
        if repository::setting(&service.connection, "vault_initialized")?.as_deref()
            == Some("false")
        {
            let _ = service.initialize_vault();
        }
        let _ = service.recover_imports();
        let _ = service.backup_recovery();
        Ok(service)
    }

    fn config(&self) -> Result<(PathBuf, String)> {
        let path = repository::setting(&self.connection, "vault_path")?
            .ok_or_else(StorageError::database)?;
        let identity = repository::setting(&self.connection, "installation_id")?
            .ok_or_else(StorageError::database)?;
        Uuid::parse_str(&identity).map_err(|_| StorageError::database())?;
        Ok((PathBuf::from(path), identity))
    }

    pub(super) fn import_vault(&self) -> Result<(PathBuf, String)> {
        let (path, identity) = self.config()?;
        vault::inspect(&path, &identity, &self.protected)?;
        Ok((path, identity))
    }

    pub(super) fn read_vault(&self) -> Result<(PathBuf, String)> {
        let (path, identity) = self.config()?;
        paths::validate_vault_location(&path, &self.protected)?;
        paths::verify_ancestors(&path, false)?;
        vault::verify_marker(&path, &identity)?;
        Ok((paths::canonical(&path)?, identity))
    }

    pub fn overview(&self) -> Result<StorageOverview> {
        // Re-read SQLite so another app instance cannot leave this process using a stale location.
        let (path, identity) = self.config()?;
        let (backup_snapshots, chat_archives) = repository::counts(&self.connection)?;
        let checked = vault::inspect(&path, &identity, &self.protected);
        let (vault_status, vault_bytes, vault_error) = match checked {
            Ok(usage) => ("ready", Some(usage.bytes), None),
            Err(error) => ("unavailable", None, Some(error)),
        };
        Ok(StorageOverview {
            database_status: "ready",
            database_path: self.database_path.to_string_lossy().into_owned(),
            schema_version: migrations::SCHEMA_VERSION,
            vault_path: path.to_string_lossy().into_owned(),
            vault_status,
            vault_error,
            backup_snapshots,
            chat_archives,
            vault_bytes,
        })
    }

    pub fn initialize_vault(&mut self) -> Result<StorageOverview> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::database())?;
        let path = PathBuf::from(
            repository::setting(&transaction, "vault_path")?.ok_or_else(StorageError::database)?,
        );
        let identity = repository::setting(&transaction, "installation_id")?
            .ok_or_else(StorageError::database)?;
        let initialized =
            repository::setting(&transaction, "vault_initialized")?.as_deref() == Some("true");
        if initialized && fs::symlink_metadata(&path).is_err() {
            // Do not create an empty replacement on a missing removable drive or deleted root.
            return Err(StorageError::filesystem());
        }
        let actual = vault::prepare(&path, &identity, &self.protected, initialized)?;
        repository::save_setting(
            &transaction,
            "vault_path",
            actual.to_str().ok_or_else(StorageError::unsafe_path)?,
        )?;
        repository::save_setting(&transaction, "vault_initialized", "true")?;
        transaction.commit().map_err(|_| StorageError::database())?;
        self.overview()
    }

    pub fn ensure_switchable(&self) -> Result<()> {
        let _coordinator = self.import_coordinator()?;
        let (path, identity) = self.config()?;
        if self.has_import_jobs()?
            || self
                .connection
                .query_row("SELECT EXISTS(SELECT 1 FROM backup_snapshots)", [], |r| {
                    r.get::<_, bool>(0)
                })
                .map_err(|_| StorageError::database())?
            || repository::counts(&self.connection)? != (0, 0)
            || vault::inspect(&path, &identity, &self.protected)?.has_data
        {
            return Err(StorageError::not_empty());
        }
        Ok(())
    }

    /// Internal only: no IPC command accepts a frontend-provided path.
    pub fn change_vault(&mut self, selected: &Path) -> Result<StorageOverview> {
        let _coordinator = self.import_coordinator()?;
        if self.has_import_jobs()? {
            return Err(StorageError::not_empty());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::database())?;
        let current = PathBuf::from(
            repository::setting(&transaction, "vault_path")?.ok_or_else(StorageError::database)?,
        );
        let identity = repository::setting(&transaction, "installation_id")?
            .ok_or_else(StorageError::database)?;
        let current_usage = vault::inspect(&current, &identity, &self.protected)?;
        paths::validate_vault_location(selected, &self.protected)?;
        if selected.exists() && paths::canonical(selected)? == paths::canonical(&current)? {
            transaction.commit().map_err(|_| StorageError::database())?;
            return self.overview();
        }
        let has_backups: bool = transaction
            .query_row("SELECT EXISTS(SELECT 1 FROM backup_snapshots)", [], |r| {
                r.get(0)
            })
            .map_err(|_| StorageError::database())?;
        if has_backups || repository::counts(&transaction)? != (0, 0) || current_usage.has_data {
            return Err(StorageError::not_empty());
        }
        // Prevent nesting in either direction, including a destination inside the empty current vault.
        paths::validate_vault_location(selected, std::slice::from_ref(&current))?;
        let destination = vault::prepare(selected, &identity, &self.protected, false)?;
        paths::validate_vault_location(&destination, std::slice::from_ref(&current))?;
        // SQLite commit is the single location switch. No content is moved, removed, or copied.
        repository::save_setting(
            &transaction,
            "vault_path",
            destination.to_str().ok_or_else(StorageError::unsafe_path)?,
        )?;
        repository::save_setting(&transaction, "vault_initialized", "true")?;
        transaction.commit().map_err(|_| StorageError::database())?;
        self.overview()
    }

    fn has_import_jobs(&self) -> Result<bool> {
        self.connection
            .query_row("SELECT EXISTS(SELECT 1 FROM import_jobs)", [], |row| {
                row.get(0)
            })
            .map_err(|_| StorageError::database())
    }
}
