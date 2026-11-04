use super::{database, migrations, paths, repository, service::StorageService, usage, vault};
use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

fn sandbox() -> TempDir {
    tempfile::tempdir().expect("isolated temporary directory")
}
fn app_path(temp: &TempDir) -> PathBuf {
    temp.path().join("application-local")
}
fn open(temp: &TempDir) -> StorageService {
    StorageService::open(&app_path(temp), &[]).unwrap()
}
fn identity() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[test]
fn repository_counts_decode_empty_and_populated_sqlite_integers() {
    let mut connection = Connection::open_in_memory().unwrap();
    migrations::migrate(&mut connection).unwrap();
    assert_eq!(repository::counts(&connection).unwrap(), (0, 0));

    connection
        .execute_batch(
            "INSERT INTO backup_snapshots(id,status,storage_path) VALUES
                ('first','pending','backups/first'),
                ('second','pending','backups/second');
             INSERT INTO archives(id,title,source_type,storage_path) VALUES
                ('archive','Test fixture','chat-export','archives/archive');",
        )
        .unwrap();

    assert_eq!(repository::counts(&connection).unwrap(), (0, 1));
    connection
        .execute(
            "UPDATE backup_snapshots SET status='complete' WHERE id='first'",
            [],
        )
        .unwrap();
    assert_eq!(repository::counts(&connection).unwrap(), (1, 1));
}

#[test]
fn first_launch_creates_schema_vault_and_real_zeros() {
    let temp = sandbox();
    let service = open(&temp);
    let overview = service.overview().unwrap();
    assert_eq!(overview.database_status, "ready");
    assert_eq!(overview.vault_status, "ready");
    assert_eq!(
        (
            overview.backup_snapshots,
            overview.chat_archives,
            overview.vault_bytes
        ),
        (0, 0, Some(0))
    );
    assert_eq!(overview.schema_version, migrations::SCHEMA_VERSION);
    assert!(Path::new(&overview.database_path).is_file());
    for name in vault::DIRECTORIES {
        assert!(Path::new(&overview.vault_path).join(name).is_dir());
    }
    assert!(
        Path::new(&overview.vault_path)
            .join(vault::MARKER)
            .is_file()
    );
}

#[test]
fn database_reopens_and_settings_survive() {
    let temp = sandbox();
    let path = temp.path().join("database/chatvault.sqlite3");
    {
        let connection = database::open(&path).unwrap();
        repository::save_setting(&connection, "test_setting", "local value").unwrap();
    }
    let connection = database::open(&path).unwrap();
    assert_eq!(
        repository::setting(&connection, "test_setting")
            .unwrap()
            .as_deref(),
        Some("local value")
    );
    assert_eq!(
        connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    assert_eq!(
        connection
            .query_row("PRAGMA busy_timeout", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        5000
    );
}

#[test]
fn migrations_are_repeatable_and_preserve_rows() {
    let mut connection = Connection::open_in_memory().unwrap();
    migrations::migrate(&mut connection).unwrap();
    repository::save_setting(&connection, "keep", "untouched").unwrap();
    migrations::migrate(&mut connection).unwrap();
    assert_eq!(
        repository::setting(&connection, "keep").unwrap().as_deref(),
        Some("untouched")
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM schema_migrations", [], |row| row
                .get::<_, u32>(0))
            .unwrap(),
        migrations::SCHEMA_VERSION
    );
}

#[test]
fn failed_migration_rolls_back_ddl_ledger_and_version() {
    let mut connection = Connection::open_in_memory().unwrap();
    migrations::migrate(&mut connection).unwrap();
    repository::save_setting(&connection, "keep", "untouched").unwrap();
    let mut migrations_with_failure = migrations::MIGRATIONS.to_vec();
    migrations_with_failure.push((
        migrations::SCHEMA_VERSION + 1,
        "CREATE TABLE must_rollback(id INTEGER); INSERT INTO missing_table VALUES (1);",
    ));
    assert!(migrations::apply(&mut connection, &migrations_with_failure).is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        migrations::SCHEMA_VERSION
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name = 'must_rollback'",
                [],
                |row| row.get::<_, u32>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM schema_migrations", [], |row| row
                .get::<_, u32>(0))
            .unwrap(),
        migrations::SCHEMA_VERSION
    );
    assert_eq!(
        repository::setting(&connection, "keep").unwrap().as_deref(),
        Some("untouched")
    );
}

#[test]
fn first_migration_failure_leaves_no_partial_schema() {
    let mut connection = Connection::open_in_memory().unwrap();
    assert!(
        migrations::apply(
            &mut connection,
            &[(1, "CREATE TABLE partial(id INTEGER); invalid syntax;")]
        )
        .is_err()
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get::<_, u32>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        0
    );
}

#[test]
fn foreign_keys_and_constraints_reject_orphans_and_unsafe_paths() {
    let temp = sandbox();
    let connection = database::open(&temp.path().join("db/chatvault.sqlite3")).unwrap();
    assert!(
        connection
            .execute(
                "INSERT INTO conversations(id,archive_id,title) VALUES ('c','missing','test')",
                []
            )
            .is_err()
    );
    assert!(connection.execute("INSERT INTO messages(id,conversation_id,message_text) VALUES ('m','missing','test')", []).is_err());
    assert!(connection.execute("INSERT INTO attachments(id,message_id,storage_path) VALUES ('a','missing','media/a')", []).is_err());
    assert!(connection.execute("INSERT INTO archives(id,title,source_type,storage_path) VALUES ('a','test','chat-export','../outside')", []).is_err());
    assert!(connection.execute("INSERT INTO backup_snapshots(id,status,storage_path,total_bytes) VALUES ('b','complete','backups/b',-1)", []).is_err());
}

#[test]
fn unknown_or_newer_databases_are_not_adopted_or_reset() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE user_data(value TEXT); INSERT INTO user_data VALUES ('preserved');",
        )
        .unwrap();
    assert_eq!(
        migrations::migrate(&mut connection).unwrap_err().code,
        "unsupported_database"
    );
    assert_eq!(
        connection
            .query_row("SELECT value FROM user_data", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "preserved"
    );
    let mut newer = Connection::open_in_memory().unwrap();
    migrations::migrate(&mut newer).unwrap();
    newer.pragma_update(None, "user_version", 999).unwrap();
    assert_eq!(
        migrations::migrate(&mut newer).unwrap_err().code,
        "unsupported_database"
    );
    assert_eq!(
        newer
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        999
    );
}

#[test]
fn custom_vault_location_survives_full_service_restart() {
    let temp = sandbox();
    let target = temp.path().join("custom-vault");
    let original;
    {
        let mut service = open(&temp);
        original = service.overview().unwrap().vault_path;
        let changed = service.change_vault(&target).unwrap();
        assert_eq!(
            Path::new(&changed.vault_path),
            paths::canonical(&target).unwrap()
        );
    }
    let reopened = open(&temp).overview().unwrap();
    assert_eq!(
        Path::new(&reopened.vault_path),
        paths::canonical(&target).unwrap()
    );
    assert_eq!(reopened.vault_bytes, Some(0));
    assert!(
        Path::new(&original).join(vault::MARKER).is_file(),
        "old vault was not deleted"
    );
}

#[test]
fn invalid_protected_and_nested_paths_are_rejected() {
    let temp = sandbox();
    let mut service = open(&temp);
    let current = PathBuf::from(service.overview().unwrap().vault_path);
    assert!(service.change_vault(Path::new("relative/path")).is_err());
    assert!(
        service
            .change_vault(&temp.path().join("parent/../escape"))
            .is_err()
    );
    assert!(service.change_vault(&app_path(&temp)).is_err());
    assert!(service.change_vault(&current.join("nested")).is_err());
    assert!(!current.join("nested").exists());
}

#[test]
fn read_only_destination_fails_without_overwriting_files() {
    let temp = sandbox();
    let mut service = open(&temp);
    let target = temp.path().join("readonly");
    fs::create_dir(&target).unwrap();
    let original_permissions = fs::metadata(&target).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&target, readonly).unwrap();
    let result = service.change_vault(&target);
    fs::set_permissions(&target, original_permissions).unwrap();
    assert_eq!(result.unwrap_err().code, "read_only");
    assert!(!target.join(vault::MARKER).exists());
}

#[test]
fn missing_saved_drive_is_preserved_and_never_recreated() {
    let temp = sandbox();
    let target = temp.path().join("removable/vault");
    {
        let mut service = open(&temp);
        service.change_vault(&target).unwrap();
    }
    let disconnected = temp.path().join("disconnected");
    fs::rename(temp.path().join("removable"), &disconnected).unwrap();
    let mut service = open(&temp);
    let overview = service.overview().unwrap();
    assert_eq!(overview.vault_status, "unavailable");
    assert_eq!(overview.vault_bytes, None);
    assert_eq!(Path::new(&overview.vault_path), target);
    assert!(service.initialize_vault().is_err());
    assert!(
        service
            .change_vault(&temp.path().join("replacement"))
            .is_err()
    );
    assert!(!target.exists());
    assert!(!temp.path().join("replacement").exists());
    fs::rename(disconnected, temp.path().join("removable")).unwrap();
    assert_eq!(service.overview().unwrap().vault_status, "ready");
}

#[test]
fn usage_counts_only_vault_files_and_excludes_identity_metadata() {
    let temp = sandbox();
    let service = open(&temp);
    let root = PathBuf::from(service.overview().unwrap().vault_path);
    fs::write(root.join("archives/export.txt"), b"12345").unwrap();
    fs::write(root.join("media/photo.bin"), [0; 7]).unwrap();
    fs::write(root.join("temp/working.bin"), [0; 3]).unwrap();
    fs::write(temp.path().join("outside.bin"), [0; 99]).unwrap();
    assert_eq!(service.overview().unwrap().vault_bytes, Some(15));
    assert!(usage::calculate(&root).unwrap().has_data);
}

#[test]
fn data_and_database_records_block_relocation_without_deletion() {
    let temp = sandbox();
    let mut service = open(&temp);
    let original = service.overview().unwrap().vault_path;
    let sentinel = Path::new(&original).join("backups/keep.bin");
    fs::write(&sentinel, b"preserve").unwrap();
    let target = temp.path().join("new");
    assert_eq!(
        service.change_vault(&target).unwrap_err().code,
        "vault_not_empty"
    );
    assert_eq!(fs::read(&sentinel).unwrap(), b"preserve");
    assert!(!target.exists());
    // This only removes this test's own isolated fixture, never real user storage.
    fs::remove_file(&sentinel).unwrap();
    let connection =
        database::open(&PathBuf::from(service.overview().unwrap().database_path)).unwrap();
    connection.execute("INSERT INTO archives(id,title,source_type,storage_path) VALUES ('test','Test fixture','chat-export','archives/test')", []).unwrap();
    assert_eq!(service.overview().unwrap().chat_archives, 1);
    assert_eq!(
        service.change_vault(&target).unwrap_err().code,
        "vault_not_empty"
    );
    assert_eq!(service.overview().unwrap().vault_path, original);
    assert!(!target.exists());
}

#[test]
fn nonempty_foreign_and_corrupt_destinations_are_preserved() {
    let temp = sandbox();
    let mut service = open(&temp);
    let occupied = temp.path().join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("keep"), b"preserve").unwrap();
    assert_eq!(
        service.change_vault(&occupied).unwrap_err().code,
        "destination_not_empty"
    );
    assert_eq!(fs::read(occupied.join("keep")).unwrap(), b"preserve");
    let foreign = temp.path().join("foreign");
    vault::prepare(&foreign, &identity(), &[], false).unwrap();
    let marker_before = fs::read(foreign.join(vault::MARKER)).unwrap();
    assert_eq!(
        service.change_vault(&foreign).unwrap_err().code,
        "foreign_vault"
    );
    assert_eq!(
        fs::read(foreign.join(vault::MARKER)).unwrap(),
        marker_before
    );
    fs::write(foreign.join(vault::MARKER), b"malformed test fixture").unwrap();
    assert_eq!(
        service.change_vault(&foreign).unwrap_err().code,
        "unrecognized_vault"
    );
    assert_eq!(
        fs::read(foreign.join(vault::MARKER)).unwrap(),
        b"malformed test fixture"
    );
}

#[test]
fn empty_known_vault_can_be_selected_again_but_never_with_data() {
    let temp = sandbox();
    let mut service = open(&temp);
    let first = PathBuf::from(service.overview().unwrap().vault_path);
    let second = temp.path().join("second");
    service.change_vault(&second).unwrap();
    service.change_vault(&first).unwrap();
    fs::write(second.join("media/keep"), b"preserve").unwrap();
    assert_eq!(
        service.change_vault(&second).unwrap_err().code,
        "destination_not_empty"
    );
    assert_eq!(fs::read(second.join("media/keep")).unwrap(), b"preserve");
}

#[cfg(unix)]
#[test]
fn symlinks_are_not_followed_or_included_in_usage() {
    let temp = sandbox();
    let service = open(&temp);
    let root = PathBuf::from(service.overview().unwrap().vault_path);
    let outside = temp.path().join("outside");
    fs::write(&outside, b"preserve").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("media/link")).unwrap();
    assert_eq!(service.overview().unwrap().vault_status, "unavailable");
    assert_eq!(fs::read(outside).unwrap(), b"preserve");
}

#[test]
fn hard_links_cannot_alias_files_outside_the_vault() {
    let temp = sandbox();
    let service = open(&temp);
    let root = PathBuf::from(service.overview().unwrap().vault_path);
    let outside = temp.path().join("outside");
    fs::write(&outside, b"preserve").unwrap();
    fs::hard_link(&outside, root.join("media/hard-link")).unwrap();
    assert_eq!(service.overview().unwrap().vault_status, "unavailable");
    assert_eq!(fs::read(outside).unwrap(), b"preserve");
}

#[test]
fn orphaned_sqlite_sidecars_do_not_trigger_creation_of_a_replacement_database() {
    let temp = sandbox();
    let root = temp.path().join("database");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("chatvault.sqlite3-wal"), b"preserve").unwrap();
    assert!(database::open(&root.join("chatvault.sqlite3")).is_err());
    assert!(!root.join("chatvault.sqlite3").exists());
    assert_eq!(
        fs::read(root.join("chatvault.sqlite3-wal")).unwrap(),
        b"preserve"
    );
}

#[test]
fn incomplete_configuration_is_not_silently_replaced() {
    let temp = sandbox();
    let overview = open(&temp).overview().unwrap();
    let connection = database::open(Path::new(&overview.database_path)).unwrap();
    let marker_before = fs::read(Path::new(&overview.vault_path).join(vault::MARKER)).unwrap();
    connection
        .execute("DELETE FROM app_settings WHERE key = 'installation_id'", [])
        .unwrap();
    assert!(StorageService::open(&app_path(&temp), &[]).is_err());
    assert_eq!(
        repository::setting(&connection, "vault_path")
            .unwrap()
            .as_deref(),
        Some(overview.vault_path.as_str())
    );
    assert!(
        repository::setting(&connection, "installation_id")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fs::read(Path::new(&overview.vault_path).join(vault::MARKER)).unwrap(),
        marker_before
    );
}

#[test]
fn protected_application_data_paths_fail_before_creating_a_database() {
    let temp = sandbox();
    let protected_source = temp.path().join("source");
    fs::create_dir(&protected_source).unwrap();
    let misplaced = protected_source.join("application-local");
    assert!(StorageService::open(&misplaced, &[protected_source]).is_err());
    assert!(!misplaced.exists());
}

#[cfg(windows)]
#[test]
fn windows_device_unc_and_reserved_names_are_rejected() {
    for path in [
        r"\\server\share\vault",
        r"\\?\C:\vault",
        r"C:\vault\NUL",
        r"C:\vault\file:stream",
        r"C:\vault\trailing.",
    ] {
        assert!(paths::validate_shape(Path::new(path)).is_err());
    }
}
