use super::error::{Result, StorageError};
use rusqlite::{Connection, OptionalExtension};

pub fn setting(connection: &Connection, key: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| StorageError::database())
}

pub fn save_setting(connection: &Connection, key: &str, value: &str) -> Result<()> {
    connection.execute("INSERT INTO app_settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CAST(unixepoch('subsec') * 1000 AS INTEGER)", (key, value)).map_err(|_| StorageError::database())?;
    Ok(())
}

pub fn counts(connection: &Connection) -> Result<(u64, u64)> {
    // SQLite INTEGER values are signed i64; rusqlite does not decode them directly as u64.
    let (snapshots, archives) = connection
        .query_row(
            "SELECT (SELECT count(*) FROM backup_snapshots WHERE status='complete' AND local_integrity!='failed'), (SELECT count(*) FROM archives)",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|_| StorageError::database())?;

    Ok((
        u64::try_from(snapshots).map_err(|_| StorageError::database())?,
        u64::try_from(archives).map_err(|_| StorageError::database())?,
    ))
}
