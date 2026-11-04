// Synthetic fixtures only. All databases/vaults are isolated temporary directories.
use super::{models::DateOrder, parser, staging, zip_processor};
use crate::storage::{migrations, repository, service::StorageService};
use rusqlite::Connection;
use std::{
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const CHAT: &str =
    "31/12/2026, 09:30 - Alice: Synthetic hello\n31/12/2026, 09:31 - Bob: Second fixture\n";
fn parsed(text: &str, order: DateOrder) -> super::models::ParsedChat {
    parser::parse(text.as_bytes(), order, 1000).unwrap()
}
fn service(temp: &tempfile::TempDir) -> StorageService {
    StorageService::open(&temp.path().join("app"), &[]).unwrap()
}
fn source(temp: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = temp.path().join(name);
    fs::write(&path, bytes).unwrap();
    path
}
fn zip_bytes(files: &[(&str, &[u8])], compression: CompressionMethod) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(compression),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
fn prepare(service: &mut StorageService, path: &Path) -> String {
    let selected = service.select_source(path).unwrap();
    service
        .analyze_import(&selected.token, DateOrder::Auto)
        .unwrap();
    selected.token
}
fn temp_entries(service: &StorageService) -> usize {
    fs::read_dir(Path::new(&service.overview().unwrap().vault_path).join("temp"))
        .unwrap()
        .count()
}

#[test]
fn android_24_hour_and_optional_seconds() {
    let chat = parsed(
        "31/12/2026, 09:30 - Alice: Hello\n31/12/2026, 14:05:12 - Bob: Hi",
        DateOrder::Auto,
    );
    assert_eq!(chat.messages.len(), 2);
    assert_eq!(chat.messages[0].sender.as_deref(), Some("Alice"));
    assert_eq!(
        chat.messages[1].local_timestamp.as_deref(),
        Some("2026-12-31T14:05:12")
    );
}
#[test]
fn ios_12_hour_and_directional_space() {
    let chat = parsed(
        "\u{200e}[12/10/26, 9:30:15 AM] Bob: Hi there\n[12/10/26, 9:31 PM] Alice: Evening",
        DateOrder::DayFirst,
    );
    assert_eq!(
        chat.messages[0].local_timestamp.as_deref(),
        Some("2026-10-12T09:30:15")
    );
    assert_eq!(
        chat.messages[1].local_timestamp.as_deref(),
        Some("2026-10-12T21:31:00")
    );
    assert_eq!(
        chat.messages[0].original_timestamp.as_deref(),
        Some("12/10/26, 9:30:15 AM")
    );
}
#[test]
fn dot_dash_and_year_first_formats() {
    for text in [
        "31.12.2026, 14:05 - Alice: Fixture",
        "31-12-2026, 14:05 - Alice: Fixture",
        "2026-12-31, 14:05 - Alice: Fixture",
    ] {
        assert_eq!(
            parsed(text, DateOrder::Auto).messages[0]
                .local_timestamp
                .as_deref(),
            Some("2026-12-31T14:05:00")
        );
    }
}
#[test]
fn multiline_unicode_sinhala_and_crlf_are_preserved() {
    let chat = parsed(
        "31/12/2026, 09:30 - නිමල්: ආයුබෝවන් 👋\r\nදෙවන පේළිය\r\n\r\n31/12/2026, 09:31 - Alice: Other",
        DateOrder::Auto,
    );
    assert_eq!(chat.messages.len(), 2);
    assert!(chat.messages[0].text.contains("ආයුබෝවන් 👋\nදෙවන පේළිය\n"));
    assert!(chat.messages[0].original_record.contains("නිමල්"));
}
#[test]
fn groups_system_deleted_and_unknown_senders() {
    let chat = parsed(
        "31/12/2026, 09:00 - Messages and calls are end-to-end encrypted\n31/12/2026, 09:01 - Alice: This message was deleted\n31/12/2026, 09:02 - Bob: Hi\n31/12/2026, 09:03 - Carol: Group fixture\n31/12/2026, 09:04 - : Missing sender",
        DateOrder::Auto,
    );
    assert_eq!(chat.participants, vec!["Alice", "Bob", "Carol"]);
    assert_eq!(chat.messages[0].classification, "system");
    assert_eq!(chat.messages[1].classification, "deleted");
    assert_eq!(chat.messages[4].sender, None);
    assert!(
        chat.diagnostics
            .iter()
            .any(|item| item.code == "sender_unknown")
    );
}
#[test]
fn ambiguous_dates_require_explicit_choice() {
    let text = "04/05/2026, 09:30 - Alice: Ambiguous fixture";
    let automatic = parsed(text, DateOrder::Auto);
    assert!(automatic.date_order_required);
    assert_eq!(automatic.messages[0].local_timestamp, None);
    assert_eq!(
        parsed(text, DateOrder::DayFirst).messages[0]
            .local_timestamp
            .as_deref(),
        Some("2026-05-04T09:30:00")
    );
    assert_eq!(
        parsed(text, DateOrder::MonthFirst).messages[0]
            .local_timestamp
            .as_deref(),
        Some("2026-04-05T09:30:00")
    );
}
#[test]
fn invalid_records_are_diagnosed_and_retained() {
    let chat = parsed(
        "Unparsed preamble\n99/99/2026, 25:99 - Alice: Invalid timestamp\n31/12/2026, 09:30 - Bob: Valid",
        DateOrder::Auto,
    );
    assert_eq!(chat.messages.len(), 3);
    assert!(chat.messages[0].text.contains("Unparsed preamble"));
    assert_eq!(chat.messages[1].local_timestamp, None);
    assert!(
        chat.diagnostics
            .iter()
            .any(|item| item.code == "invalid_timestamp")
    );
    assert!(parser::parse(b"not a conversation", DateOrder::Auto, 100).is_err());
}
#[test]
fn utf16_encodings_and_invalid_utf8() {
    for little in [true, false] {
        let mut bytes = if little {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        };
        for value in CHAT.encode_utf16() {
            bytes.extend(if little {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            });
        }
        assert_eq!(
            parser::parse(&bytes, DateOrder::Auto, 100)
                .unwrap()
                .messages
                .len(),
            2
        );
    }
    assert_eq!(
        parser::parse(&[0xff, 0x80], DateOrder::Auto, 100)
            .err()
            .unwrap()
            .code,
        "invalid_encoding"
    );
    assert!(parser::parse(&[0xff, 0xfe, 0x00], DateOrder::Auto, 100).is_err());
    assert!(parser::parse(&[0xff, 0xfe, 0x00, 0xd8], DateOrder::Auto, 100).is_err());
}
#[test]
fn parser_limits_are_enforced() {
    assert_eq!(
        parser::parse(CHAT.as_bytes(), DateOrder::Auto, 1)
            .err()
            .unwrap()
            .code,
        "import_limit"
    );
}

#[test]
fn text_import_persists_messages_metadata_and_original_without_utc_guess() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let original = source(&temp, "Synthetic.txt", CHAT.as_bytes());
    let token = prepare(&mut app, &original);
    let saved = app.confirm_import(&token, "Synthetic archive").unwrap();
    assert_eq!((saved.message_count, saved.media_count), (2, 0));
    assert_eq!(saved.import_status, "complete");
    assert_eq!(repository::counts(&app.connection).unwrap(), (0, 1));
    let (local,utc,raw):(String,Option<i64>,String)=app.connection.query_row("SELECT local_timestamp,timestamp,original_timestamp FROM messages ORDER BY sequence_number LIMIT 1",[],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(local, "2026-12-31T09:30:00");
    assert_eq!(utc, None);
    assert_eq!(raw, "31/12/2026, 09:30");
    let root = PathBuf::from(app.overview().unwrap().vault_path)
        .join("archives")
        .join(&saved.id);
    assert_eq!(
        fs::read(root.join("original/source.txt")).unwrap(),
        CHAT.as_bytes()
    );
    assert_eq!(fs::read(original).unwrap(), CHAT.as_bytes());
    assert_eq!(temp_entries(&app), 0);
}
#[test]
fn imports_survive_database_and_service_restart() {
    let temp = tempfile::tempdir().unwrap();
    let path = source(&temp, "Restart.txt", CHAT.as_bytes());
    let id = {
        let mut app = service(&temp);
        let token = prepare(&mut app, &path);
        app.confirm_import(&token, "Restart fixture").unwrap().id
    };
    let app = service(&temp);
    let library = app.list_archives("Restart", 0, 20).unwrap();
    assert_eq!(library.total, 1);
    assert_eq!(library.items[0].id, id);
    assert_eq!(app.archive_summary(&id).unwrap().message_count, 2);
    assert!(app.overview().unwrap().vault_bytes.unwrap() > 0);
}
#[test]
fn duplicate_hash_blocks_second_archive_and_original_is_untouched() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let path = source(&temp, "Duplicate.txt", CHAT.as_bytes());
    let token = prepare(&mut app, &path);
    let saved = app.confirm_import(&token, "First").unwrap();
    let selection = app.select_source(&path).unwrap();
    assert_eq!(
        selection.duplicate_archive.as_deref(),
        Some(saved.id.as_str())
    );
    app.cancel_import(&selection.token).unwrap();
    assert_eq!(repository::counts(&app.connection).unwrap(), (0, 1));
    assert_eq!(fs::read(path).unwrap(), CHAT.as_bytes());
}
#[test]
fn overlapping_exports_are_independent_not_merged() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    for (index, text) in [
        CHAT,
        "31/12/2026, 09:30 - Alice: Synthetic hello\n31/12/2026, 10:00 - Bob: Additional fixture",
    ]
    .iter()
    .enumerate()
    {
        let path = source(&temp, &format!("Export{index}.txt"), text.as_bytes());
        let token = prepare(&mut app, &path);
        app.confirm_import(&token, "Same conversation").unwrap();
    }
    assert_eq!(app.list_archives("", 0, 20).unwrap().total, 2);
    assert_eq!(
        app.connection
            .query_row("SELECT count(*) FROM messages", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        4
    );
}
#[test]
fn cancel_and_invalid_input_release_staging_without_deletion_of_source() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let path = source(&temp, "Cancel.txt", CHAT.as_bytes());
    let selected = app.select_source(&path).unwrap();
    app.cancel_import(&selected.token).unwrap();
    assert_eq!(temp_entries(&app), 0);
    let invalid = source(&temp, "Bad.txt", b"not a chat export");
    let selected = app.select_source(&invalid).unwrap();
    assert!(
        app.analyze_import(&selected.token, DateOrder::Auto)
            .is_err()
    );
    assert_eq!(temp_entries(&app), 0);
    assert_eq!(repository::counts(&app.connection).unwrap(), (0, 0));
    assert_eq!(fs::read(path).unwrap(), CHAT.as_bytes());
}
#[test]
fn extension_is_not_used_as_content_proof() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let path = source(&temp, "Renamed.zip", CHAT.as_bytes());
    assert_eq!(
        app.select_source(&path).err().unwrap().code,
        "unsupported_format"
    );
    let crypt = source(&temp, "Backup.crypt15", b"encrypted fixture");
    assert_eq!(
        app.select_source(&crypt).err().unwrap().code,
        "unsupported_format"
    );
    assert_eq!(temp_entries(&app), 0);
}
#[test]
fn zip_media_and_missing_references_are_stored_safely() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let chat=b"31/12/2026, 09:30 - Alice: IMG-20261231-WA0001.jpg (file attached)\n31/12/2026, 09:31 - Bob: <attached: missing.jpg>";
    let bytes = zip_bytes(
        &[
            ("_chat.txt", chat),
            ("media/IMG-20261231-WA0001.jpg", b"synthetic-media"),
        ],
        CompressionMethod::Stored,
    );
    let path = source(&temp, "Media.zip", &bytes);
    let selected = app.select_source(&path).unwrap();
    let preview = app
        .analyze_import(&selected.token, DateOrder::Auto)
        .unwrap();
    assert_eq!(
        (
            preview.message_count,
            preview.media_count,
            preview.attachment_count,
            preview.missing_attachment_count
        ),
        (2, 1, 2, 1)
    );
    let saved = app
        .confirm_import(&selected.token, "Media fixture")
        .unwrap();
    assert_eq!((saved.media_count, saved.missing_media_count), (1, 1));
    let stored: String = app
        .connection
        .query_row("SELECT storage_path FROM archive_media", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(stored.starts_with(&format!("archives/{}/media/", saved.id)));
    assert!(!stored.contains("IMG-"));
    assert_eq!(
        fs::read(Path::new(&app.overview().unwrap().vault_path).join(stored)).unwrap(),
        b"synthetic-media"
    );
    assert_eq!(
        app.connection
            .query_row(
                "SELECT count(*) FROM attachments WHERE is_missing=1 AND storage_path IS NULL",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}
#[test]
fn zip_without_media_retains_omitted_media_diagnostic() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let bytes = zip_bytes(
        &[("_chat.txt", b"31/12/2026, 09:30 - Alice: <Media omitted>")],
        CompressionMethod::Deflated,
    );
    let path = source(&temp, "NoMedia.zip", &bytes);
    let selected = app.select_source(&path).unwrap();
    let preview = app
        .analyze_import(&selected.token, DateOrder::Auto)
        .unwrap();
    assert_eq!(preview.missing_attachment_count, 1);
    app.cancel_import(&selected.token).unwrap();
}
#[test]
fn zip_traversal_absolute_reserved_and_colliding_names_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    for (index, name) in [
        "../outside.jpg",
        "/absolute.jpg",
        "C:/outside.jpg",
        "media/CON.jpg",
        "media/file:stream.jpg",
        "media\\escape.jpg",
    ]
    .iter()
    .enumerate()
    {
        let bytes = zip_bytes(
            &[("_chat.txt", CHAT.as_bytes()), (name, b"malicious fixture")],
            CompressionMethod::Stored,
        );
        let path = source(&temp, &format!("Unsafe{index}.zip"), &bytes);
        let selected = app.select_source(&path).unwrap();
        assert!(
            app.analyze_import(&selected.token, DateOrder::Auto)
                .is_err()
        );
        assert_eq!(temp_entries(&app), 0);
    }
    let bytes = zip_bytes(
        &[
            ("_chat.txt", CHAT.as_bytes()),
            ("IMAGE.jpg", b"one"),
            ("image.JPG", b"two"),
        ],
        CompressionMethod::Stored,
    );
    let selected = app
        .select_source(&source(&temp, "Collision.zip", &bytes))
        .unwrap();
    assert_eq!(
        app.analyze_import(&selected.token, DateOrder::Auto)
            .err()
            .unwrap()
            .code,
        "unsafe_zip"
    );
    assert_eq!(repository::counts(&app.connection).unwrap(), (0, 0));
    assert!(!temp.path().join("outside.jpg").exists());
}
#[test]
fn zip_bomb_size_entry_count_and_corruption_limits() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let huge = vec![0; 32768];
    let bytes = zip_bytes(
        &[("_chat.txt", CHAT.as_bytes()), ("large.jpg", &huge)],
        CompressionMethod::Deflated,
    );
    let selected = app
        .select_source(&source(&temp, "Bomb.zip", &bytes))
        .unwrap();
    assert_eq!(
        app.analyze_import(&selected.token, DateOrder::Auto)
            .err()
            .unwrap()
            .code,
        "import_limit"
    );
    app.import_limits.entry_bytes = 8;
    let bytes = zip_bytes(&[("_chat.txt", CHAT.as_bytes())], CompressionMethod::Stored);
    let selected = app
        .select_source(&source(&temp, "Oversize.zip", &bytes))
        .unwrap();
    assert_eq!(
        app.analyze_import(&selected.token, DateOrder::Auto)
            .err()
            .unwrap()
            .code,
        "import_limit"
    );
    app.import_limits = Default::default();
    app.import_limits.entries = 1;
    let bytes = zip_bytes(
        &[("_chat.txt", CHAT.as_bytes()), ("x.jpg", b"small")],
        CompressionMethod::Stored,
    );
    let selected = app
        .select_source(&source(&temp, "Entries.zip", &bytes))
        .unwrap();
    assert_eq!(
        app.analyze_import(&selected.token, DateOrder::Auto)
            .err()
            .unwrap()
            .code,
        "import_limit"
    );
    app.import_limits = Default::default();
    let selected = app
        .select_source(&source(&temp, "Broken.zip", b"PK\x03\x04broken"))
        .unwrap();
    assert_eq!(
        app.analyze_import(&selected.token, DateOrder::Auto)
            .err()
            .unwrap()
            .code,
        "invalid_zip"
    );
    assert_eq!(temp_entries(&app), 0);
}
#[test]
fn zip_symlinks_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let mut bytes = zip_bytes(&[("_chat.txt", CHAT.as_bytes())], CompressionMethod::Stored);
    let central = bytes
        .windows(4)
        .position(|value| value == b"PK\x01\x02")
        .unwrap();
    bytes[central + 5] = 3;
    bytes[central + 38..central + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
    let selected = app
        .select_source(&source(&temp, "Symlink.zip", &bytes))
        .unwrap();
    assert_eq!(
        app.analyze_import(&selected.token, DateOrder::Auto)
            .err()
            .unwrap()
            .code,
        "unsafe_zip"
    );
}
#[test]
fn transaction_failure_rolls_back_rows_and_new_files() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let path = source(&temp, "Rollback.txt", CHAT.as_bytes());
    let token = prepare(&mut app, &path);
    app.connection.execute_batch("CREATE TRIGGER synthetic_failure BEFORE INSERT ON messages BEGIN SELECT RAISE(ABORT,'synthetic test failure'); END;").unwrap();
    assert_eq!(
        app.confirm_import(&token, "Rollback fixture")
            .err()
            .unwrap()
            .code,
        "import_database"
    );
    assert_eq!(repository::counts(&app.connection).unwrap(), (0, 0));
    assert_eq!(
        app.connection
            .query_row("SELECT count(*) FROM import_jobs", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        fs::read_dir(Path::new(&app.overview().unwrap().vault_path).join("archives"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(temp_entries(&app), 0);
    assert_eq!(fs::read(path).unwrap(), CHAT.as_bytes());
}
#[test]
fn custom_vault_and_library_filtering_are_real() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let custom = temp.path().join("custom");
    app.change_vault(&custom).unwrap();
    let path = source(&temp, "Percent_100%.txt", CHAT.as_bytes());
    let token = prepare(&mut app, &path);
    let saved = app.confirm_import(&token, "Local fixture").unwrap();
    assert!(custom.join("archives").join(&saved.id).exists());
    assert_eq!(app.list_archives("100%", 0, 20).unwrap().total, 1);
    assert_eq!(app.list_archives("absent", 0, 20).unwrap().total, 0);
    assert!(
        app.change_vault(&temp.path().join("unsafe-switch"))
            .is_err()
    );
}
#[test]
fn cross_instance_import_lock_prevents_conflicting_sessions_and_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let mut first = service(&temp);
    let mut second = service(&temp);
    let path = source(&temp, "Exclusive.txt", CHAT.as_bytes());
    let selection = first.select_source(&path).unwrap();
    second.recover_imports().unwrap();
    assert_eq!(
        second.select_source(&path).err().unwrap().code,
        "import_busy"
    );
    assert!(second.change_vault(&temp.path().join("blocked")).is_err());
    first
        .analyze_import(&selection.token, DateOrder::Auto)
        .unwrap();
    first.cancel_import(&selection.token).unwrap();
    let selection = second.select_source(&path).unwrap();
    second.cancel_import(&selection.token).unwrap();
}
#[test]
fn recovery_cleans_only_owned_uncommitted_files_and_preserves_other_archives() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let (vault, installation) = app.import_vault().unwrap();
    let retained = vault.join("archives/existing-user-folder");
    fs::create_dir(&retained).unwrap();
    fs::write(retained.join("keep"), b"preserve").unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let relative = format!("archives/{id}");
    let root = vault.join(&relative);
    fs::create_dir(&root).unwrap();
    staging::write_owner(&root, staging::FINAL_MARKER, &id, &installation).unwrap();
    fs::write(root.join("partial"), b"synthetic partial").unwrap();
    app.connection.execute("INSERT INTO import_jobs(id,vault_path,storage_path,installation_id) VALUES (?1,?2,?3,?4)",rusqlite::params![id,vault.to_str(),relative,installation]).unwrap();
    app.recover_imports().unwrap();
    assert!(!root.exists());
    assert_eq!(fs::read(retained.join("keep")).unwrap(), b"preserve");
    assert_eq!(
        app.connection
            .query_row("SELECT count(*) FROM import_jobs", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn schema_v1_upgrade_preserves_existing_rows_and_is_repeatable() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    migrations::apply(&mut connection, &migrations::MIGRATIONS[..1]).unwrap();
    connection.execute_batch("INSERT INTO archives(id,title,source_type,storage_path) VALUES ('old','Existing','chat-export','archives/old'); INSERT INTO conversations(id,archive_id,title) VALUES ('c','old','Existing'); INSERT INTO messages(id,conversation_id,message_text) VALUES ('m','c','Synthetic preserved text'); INSERT INTO attachments(id,message_id,storage_path) VALUES ('a','m','media/existing.jpg');").unwrap();
    migrations::migrate(&mut connection).unwrap();
    migrations::migrate(&mut connection).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT message_text FROM messages WHERE id='m'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "Synthetic preserved text"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT storage_path FROM attachments WHERE id='a'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "media/existing.jpg"
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM schema_migrations", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        migrations::SCHEMA_VERSION as i64
    );
}
#[test]
fn filename_safety_rejects_controls_and_reserved_paths() {
    for name in [
        "../x", "a/../b", "/root", "a//b", "C:/x", "NUL", "a/file.", "a/file ", "a/<bad>",
        "a/\n.txt",
    ] {
        assert!(
            zip_processor::safe_name(name).is_err(),
            "synthetic unsafe name accepted"
        );
    }
}

#[test]
fn text_attachments_are_retained_and_multiple_conversations_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let chat = b"31/12/2026, 09:30 - Alice: notes.txt (file attached)";
    let bytes = zip_bytes(
        &[
            ("_chat.txt", chat),
            ("notes.txt", b"Synthetic attached note"),
        ],
        CompressionMethod::Stored,
    );
    let selected = app
        .select_source(&source(&temp, "TextAttachment.zip", &bytes))
        .unwrap();
    let preview = app
        .analyze_import(&selected.token, DateOrder::Auto)
        .unwrap();
    assert_eq!(
        (preview.media_count, preview.missing_attachment_count),
        (1, 0)
    );
    app.cancel_import(&selected.token).unwrap();
    let bytes = zip_bytes(
        &[("one.txt", CHAT.as_bytes()), ("two.txt", CHAT.as_bytes())],
        CompressionMethod::Stored,
    );
    let selected = app
        .select_source(&source(&temp, "Multiple.zip", &bytes))
        .unwrap();
    assert_eq!(
        app.analyze_import(&selected.token, DateOrder::Auto)
            .err()
            .unwrap()
            .code,
        "unsupported_format"
    );
}

#[test]
fn crc_corruption_is_rejected_without_persistence() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = service(&temp);
    let mut bytes = zip_bytes(&[("_chat.txt", CHAT.as_bytes())], CompressionMethod::Stored);
    let name_length = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
    let extra_length = u16::from_le_bytes([bytes[28], bytes[29]]) as usize;
    bytes[30 + name_length + extra_length] ^= 1;
    let selected = app
        .select_source(&source(&temp, "Crc.zip", &bytes))
        .unwrap();
    assert!(
        app.analyze_import(&selected.token, DateOrder::Auto)
            .is_err()
    );
    assert_eq!(repository::counts(&app.connection).unwrap(), (0, 0));
    assert_eq!(temp_entries(&app), 0);
}
