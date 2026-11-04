use super::{
    error::{Result, StorageError},
    migrations, paths,
};
use rusqlite::Connection;
use std::{fs, path::Path, time::Duration};

pub fn open(path: &Path) -> Result<Connection> {
    let parent = path.parent().ok_or_else(StorageError::database)?;
    paths::create_safe_directory(parent)?;
    let database_exists = path.exists();
    for candidate in [
        path.to_path_buf(),
        path.with_file_name("chatvault.sqlite3-wal"),
        path.with_file_name("chatvault.sqlite3-shm"),
        path.with_file_name("chatvault.sqlite3-journal"),
    ] {
        match fs::symlink_metadata(&candidate) {
            Ok(metadata) if paths::is_link(&metadata) || !metadata.is_file() => {
                return Err(StorageError::unsafe_path());
            }
            Ok(_) => {
                if !database_exists && candidate != path {
                    return Err(StorageError::database());
                }
                paths::open_regular_file(&candidate)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(StorageError::database()),
        }
    }
    let mut connection = Connection::open(path).map_err(|_| StorageError::database())?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|_| StorageError::database())?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|_| StorageError::database())?;
    connection
        .pragma_update(None, "trusted_schema", false)
        .map_err(|_| StorageError::database())?;
    migrations::migrate(&mut connection)?;
    let mode: String = connection
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .map_err(|_| StorageError::database())?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(StorageError::database());
    }
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|_| StorageError::database())?;
    let integrity: String = connection
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .map_err(|_| StorageError::database())?;
    if integrity != "ok" {
        return Err(StorageError::database());
    }
    Ok(connection)
}
