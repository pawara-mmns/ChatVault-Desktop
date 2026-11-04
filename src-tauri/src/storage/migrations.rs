use super::error::{Result, StorageError};
use rusqlite::{Connection, TransactionBehavior};

pub const SCHEMA_VERSION: u32 = 5;
const APPLICATION_ID: u32 = 0x43565431; // CVT1
pub const MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("../../migrations/001_initial.sql")),
    (2, include_str!("../../migrations/002_chat_import.sql")),
    (3, include_str!("../../migrations/003_chat_viewer.sql")),
    (4, include_str!("../../migrations/004_device_notes.sql")),
    (5, include_str!("../../migrations/005_local_backups.sql")),
];

pub fn migrate(connection: &mut Connection) -> Result<()> {
    apply(connection, MIGRATIONS)
}

/// One transaction covers the ledger, all pending DDL, and both schema pragmas.
pub(crate) fn apply(connection: &mut Connection, migrations: &[(u32, &str)]) -> Result<()> {
    super::viewer::search::register(connection)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| StorageError::database())?;
    let version: u32 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| StorageError::database())?;
    let identity: u32 = transaction
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .map_err(|_| StorageError::database())?;
    let latest = migrations.last().map_or(0, |entry| entry.0);
    if version > latest || (identity != APPLICATION_ID && (identity != 0 || version != 0)) {
        return Err(StorageError::new(
            "unsupported_database",
            "This database belongs to another application or a newer ChatVault version. It has not been changed.",
        ));
    }
    if identity == 0 {
        let tables: u32 = transaction
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::database())?;
        if tables != 0 {
            return Err(StorageError::new(
                "unsupported_database",
                "The database is not a recognized ChatVault database. It has not been adopted or reset.",
            ));
        }
    }
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY NOT NULL, applied_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER))) STRICT;").map_err(|_| StorageError::database())?;
    let ledger: Vec<u32> = {
        let mut statement = transaction
            .prepare("SELECT version FROM schema_migrations ORDER BY version")
            .map_err(|_| StorageError::database())?;
        let rows = statement
            .query_map([], |row| row.get(0))
            .map_err(|_| StorageError::database())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| StorageError::database())?
    };
    let expected: Vec<u32> = (1..=version).collect();
    if ledger != expected
        || migrations
            .iter()
            .enumerate()
            .any(|(index, migration)| migration.0 != index as u32 + 1)
    {
        return Err(StorageError::new(
            "migration_failed",
            "The database migration history is inconsistent. No migration was committed.",
        ));
    }
    for &(number, sql) in migrations.iter().filter(|entry| entry.0 > version) {
        transaction.execute_batch(sql).map_err(|_| StorageError::new("migration_failed", "A local database migration failed. Its changes were rolled back; the database was not reset."))?;
        transaction
            .execute(
                "INSERT INTO schema_migrations(version) VALUES (?1)",
                [number],
            )
            .map_err(|_| StorageError::database())?;
    }
    transaction
        .pragma_update(None, "application_id", APPLICATION_ID)
        .map_err(|_| StorageError::database())?;
    transaction
        .pragma_update(None, "user_version", latest)
        .map_err(|_| StorageError::database())?;
    transaction.commit().map_err(|_| StorageError::database())
}
