use super::*;
use crate::storage::backups::{models as backup, planner::PlannedFile, repository};
use rusqlite::{OptionalExtension, params};
use std::io::Write;
const MODERN: &[u8] = include_bytes!("fixtures/modern.crypt15");
const LEGACY: &[u8] = include_bytes!("fixtures/legacy.crypt14");
const KEY14: &[u8] = include_bytes!("fixtures/synthetic-key14.bin");
const PNG: &[u8] = include_bytes!("fixtures/synthetic.png");
fn block<T>(f: impl std::future::Future<Output = T>) -> T {
    tauri::async_runtime::block_on(f)
}
fn key() -> KeyInput {
    KeyInput::Hex(Zeroizing::new("07".repeat(32)))
}
#[test]
#[ignore = "read-only real snapshot inspection; no key access or decryption"]
fn inspect_saved_business_snapshot_readonly() {
    let path = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join("app.chatvault.desktop/database/chatvault.sqlite3");
    let c = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap();
    let vault: String = c
        .query_row(
            "SELECT value FROM app_settings WHERE key='vault_path'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let installation: String = c
        .query_row(
            "SELECT value FROM app_settings WHERE key='installation_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let id:String=c.query_row("SELECT id FROM backup_snapshots WHERE status='complete' ORDER BY created_at DESC LIMIT 1",[],|r|r.get(0)).unwrap();
    let report = inspector::inspect(&c, std::path::Path::new(&vault), &installation, &id, |_| {
        Ok(())
    })
    .unwrap();
    assert_eq!(report.total_files, 1096);
    assert_eq!(report.candidates.len(), 8);
    assert!(
        report
            .candidates
            .iter()
            .all(|c| c.format == "crypt14" && c.hash_verified)
    );
    println!(
        "READONLY REAL INSPECTION: {}",
        serde_json::to_string(&report).unwrap()
    );
}
fn entry(
    id: String,
    name: &str,
    relative: String,
    category: backup::Category,
    bytes: &[u8],
) -> backup::FileEntry {
    backup::FileEntry {
        id,
        category,
        source_relative: format!(
            "Storage 1/current/WhatsApp Business/{}/{}",
            if category == backup::Category::Databases {
                "Databases"
            } else {
                "Media/WhatsApp Business Images"
            },
            name
        ),
        local_relative: relative,
        original_name: name.into(),
        expected_size: Some(bytes.len() as u64),
        actual_bytes: bytes.len() as u64,
        sha256: Some(format!("{:x}", Sha256::digest(bytes))),
        local_verification: "verified".into(),
        source_verification: "verified".into(),
        size_validation: "matched".into(),
        state: "verified".into(),
        error_code: None,
    }
}
fn environment(
    bytes: &[u8],
    format: &str,
    with_media: bool,
) -> (
    tempfile::TempDir,
    StorageState,
    NativeState,
    String,
    String,
    std::path::PathBuf,
) {
    let temp = tempfile::tempdir().unwrap();
    let storage = StorageState::new(Ok(temp.path().join("profile")));
    let state = NativeState::default();
    let id = uuid::Uuid::new_v4().to_string();
    let fid = uuid::Uuid::new_v4().to_string();
    let root = db(&storage, |s| {
        let (vault, installation) = s.read_vault()?;
        let root = vault.join("backups").join(&id);
        fs::create_dir(&root).unwrap();
        crate::storage::backups::files::owner(&root, &id, &installation).unwrap();
        let name = format!("msgstore.db.{format}");
        let mut entries = vec![entry(
            fid.clone(),
            &name,
            format!("databases/{fid}-{name}"),
            backup::Category::Databases,
            bytes,
        )];
        if with_media {
            let mid = uuid::Uuid::new_v4().to_string();
            entries.push(entry(
                mid.clone(),
                "synthetic.png",
                format!("media/images/{mid}-synthetic.png"),
                backup::Category::Images,
                PNG,
            ));
        }
        for e in &entries {
            let path = root.join(&e.local_relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(
                &path,
                if e.category == backup::Category::Databases {
                    bytes
                } else {
                    PNG
                },
            )
            .unwrap();
        }
        let planned: Vec<_> = entries
            .iter()
            .map(|e| PlannedFile {
                object_id: String::new(),
                entry: e.clone(),
            })
            .collect();
        repository::start(
            &mut s.connection,
            &id,
            backup::Source::Business,
            Some("Synthetic"),
            &planned,
            &[],
        )
        .unwrap();
        for e in &entries {
            repository::file(&s.connection, &id, e).unwrap();
        }
        let p = backup::Progress {
            snapshot_id: Some(id.clone()),
            phase: "completed".into(),
            total_files: entries.len(),
            successful_files: entries.len(),
            copied_bytes: entries.iter().map(|e| e.actual_bytes).sum(),
            verified_bytes: entries.iter().map(|e| e.actual_bytes).sum(),
            ..Default::default()
        };
        repository::terminal(&mut s.connection, &p, &[]).unwrap();
        let manifest = backup::Manifest {
            scanned_categories: vec![],
            skipped_entries: 0,
            inaccessible_locations: 0,
            schema_version: 1,
            snapshot_id: id.clone(),
            created_at: crate::devices::models::now(),
            source: backup::Source::Business,
            device_display_name: None,
            device_association: None,
            backup_mode: "synthetic fixture".into(),
            phase: "completed".into(),
            inventory_complete: true,
            warnings: vec![],
            source_consistency: "synthetic".into(),
            files: entries,
        };
        crate::storage::backups::files::manifest(&root, &manifest).unwrap();
        Ok(root)
    })
    .unwrap();
    (temp, storage, state, id, fid, root)
}
#[test]
fn independent_node_openssl_vectors_authenticate_for_both_supported_envelopes() {
    for (bytes, keydata) in [(MODERN, None), (LEGACY, Some(KEY14))] {
        let h = crypto::header(bytes).unwrap();
        let k = match keydata {
            Some(k) => crypto::key_file(Zeroizing::new(k.to_vec()), &h).unwrap(),
            None => crypto::hex_key(&"07".repeat(32)).unwrap(),
        };
        let plain = crypto::decrypt(Zeroizing::new(bytes.to_vec()), &h, k, || Ok(())).unwrap();
        let mut output = vec![];
        crypto::plaintext(&plain, &mut output, |_| Ok(())).unwrap();
        assert!(output.starts_with(b"SQLite format 3\0"));
    }
}
#[test]
fn unsupported_keys_headers_truncation_and_authentication_fail_without_plaintext() {
    assert!(crypto::hex_key("password").is_err());
    assert!(crypto::header(&[0; 200]).is_err());
    let h = crypto::header(MODERN).unwrap();
    let bad = crypto::decrypt(
        Zeroizing::new(MODERN.to_vec()),
        &h,
        crypto::hex_key(&"00".repeat(32)).unwrap(),
        || Ok(()),
    )
    .err()
    .unwrap();
    assert_eq!(bad.code, "native_auth");
    assert!(!bad.message.contains(&"00".repeat(32)));
    assert!(
        crypto::decrypt(
            Zeroizing::new(MODERN[..30].to_vec()),
            &h,
            crypto::hex_key(&"07".repeat(32)).unwrap(),
            || Ok(())
        )
        .is_err()
    );
    let mut modified = MODERN.to_vec();
    modified[h.offset + 1] ^= 1;
    let end = modified.len() - 16;
    use md5::Md5;
    let checksum = Md5::digest(&modified[..end]);
    modified[end..].copy_from_slice(&checksum);
    assert_eq!(
        crypto::decrypt(
            Zeroizing::new(modified),
            &h,
            crypto::hex_key(&"07".repeat(32)).unwrap(),
            || Ok(())
        )
        .err()
        .unwrap()
        .code,
        "native_auth"
    );
    assert_eq!(
        crypto::decrypt(
            Zeroizing::new(MODERN.to_vec()),
            &h,
            crypto::hex_key(&"07".repeat(32)).unwrap(),
            || Err(fail("native_cancelled"))
        )
        .err()
        .unwrap()
        .code,
        "native_cancelled"
    );
    assert!(
        crypto::key_file(
            Zeroizing::new(vec![0; 131]),
            &crypto::header(LEGACY).unwrap()
        )
        .is_err()
    );
}
#[test]
fn key14_serialized_byte_array_is_narrowly_parsed() {
    let mut serialized = vec![
        0xac, 0xed, 0, 5, 0x75, 0x72, 0, 2, 0x5b, 0x42, 0xac, 0xf3, 0x17, 0xf8, 6, 8, 0x54, 0xe0,
        2, 0, 0, 0x78, 0x70,
    ];
    serialized.extend(131u32.to_be_bytes());
    serialized.extend(KEY14);
    assert_eq!(
        crypto::key_file(Zeroizing::new(serialized), &crypto::header(LEGACY).unwrap())
            .unwrap()
            .len(),
        32
    );
}
#[test]
fn modern_native_import_reuses_viewer_search_media_direction_and_survives_restart() {
    let (temp, storage, state, id, fid, root) = environment(MODERN, "crypt15", true);
    let inspection = block(state.inspect(storage.clone(), id.clone())).unwrap();
    assert_eq!(inspection.capability, "SUPPORTED_AND_KEY_REQUIRED");
    assert!(inspection.candidates[0].hash_verified);
    assert!(block(state.verify(storage.clone(), id.clone(), fid.clone(), key(), false)).is_err());
    let preview = block(state.verify(storage.clone(), id.clone(), fid, key(), true)).unwrap();
    assert_eq!(
        (
            preview.counts.conversations,
            preview.counts.messages,
            preview.counts.media_matched,
            preview.counts.media_missing,
            preview.counts.unsupported
        ),
        (3, 7, 1, 1, 1)
    );
    assert_eq!(
        storage
            .run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap()
            .1,
        0
    );
    assert!(block(state.import(storage.clone(), preview.token.clone(), false)).is_err());
    let result = block(state.import(storage.clone(), preview.token, true)).unwrap();
    let aid = result.archive_id.unwrap();
    storage
        .run(false, |s| {
            let diagnostics: String = s
                .connection
                .query_row(
                    "SELECT native_diagnostics_json FROM archives WHERE id=?1",
                    [&aid],
                    |r| r.get(0),
                )
                .unwrap();
            let diagnostics: serde_json::Value = serde_json::from_str(&diagnostics).unwrap();
            assert_eq!(diagnostics["counts"]["messages"], 7);
            assert_eq!(diagnostics["parserVersion"], PARSER_VERSION);
            let chats = s.list_conversations("", Some(&aid), 0, 50)?;
            assert_eq!(chats.total, 3);
            let personal = chats
                .items
                .iter()
                .find(|c| c.title == "123@s.whatsapp.net")
                .unwrap();
            let page = s.read_messages(
                &personal.id,
                None,
                crate::storage::viewer::models::Direction::Latest,
                50,
            )?;
            assert_eq!(page.items[0].native_direction.as_deref(), Some("incoming"));
            let outgoing = page
                .items
                .iter()
                .find(|m| m.native_direction.as_deref() == Some("outgoing"))
                .unwrap();
            assert!(outgoing.text.contains('\u{0dc3}'));
            assert!(outgoing.native_reply_id.is_some());
            assert!(page.items.iter().any(|m| m.classification == "deleted"));
            assert!(!s.search_messages("synthetic", None, None)?.items.is_empty());
            let photo = s
                .connection
                .query_row("SELECT media_id FROM native_media_links", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap();
            let resolved = s.resolve_media(&photo)?;
            assert_eq!(resolved.metadata.kind, "image");
            assert!(resolved.path.starts_with(&root));
            let vault = s.read_vault()?.0;
            assert_eq!(
                fs::read_dir(vault.join("archives").join(&aid))
                    .unwrap()
                    .count(),
                1
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(
        fs::read(root.join(format!(
            "databases/{}-msgstore.db.crypt15",
            inspection.candidates[0].file_id
        )))
        .unwrap(),
        MODERN
    );
    drop(storage);
    let reopened = StorageState::new(Ok(temp.path().join("profile")));
    assert_eq!(
        reopened
            .run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap(),
        (1, 1)
    );
    assert_eq!(
        block(state.inspect(reopened, id))
            .unwrap()
            .existing_archive
            .as_deref(),
        Some(aid.as_str())
    );
}
#[test]
fn legacy_local_key_file_imports_real_rows() {
    let (_t, s, state, id, fid, _root) = environment(LEGACY, "crypt14", false);
    let p = block(state.verify(
        s.clone(),
        id,
        fid,
        KeyInput::File(Zeroizing::new(KEY14.to_vec())),
        true,
    ))
    .unwrap();
    assert_eq!(p.schema, "messages-v1");
    assert_eq!(p.counts.messages, 2);
    assert!(
        block(state.import(s, p.token, true))
            .unwrap()
            .archive_id
            .is_some()
    );
}
#[test]
fn duplicate_prevention_and_atomic_rollback_preserve_backups() {
    let (_t, s, state, id, fid, root) = environment(MODERN, "crypt15", false);
    let p = block(state.verify(s.clone(), id.clone(), fid.clone(), key(), true)).unwrap();
    block(state.import(s.clone(), p.token, true)).unwrap();
    let p = block(state.verify(s.clone(), id, fid, key(), true)).unwrap();
    assert_eq!(
        block(state.import(s.clone(), p.token, true))
            .err()
            .unwrap()
            .code,
        "native_duplicate"
    );
    assert_eq!(
        s.run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap(),
        (1, 1)
    );
    assert!(root.is_dir());
    let (_t, s, state, id, fid, root) = environment(MODERN, "crypt15", false);
    s.run(false,|s|{s.connection.execute_batch("CREATE TRIGGER fail_native BEFORE INSERT ON messages BEGIN SELECT RAISE(ABORT,'synthetic rollback'); END;").unwrap();Ok(())}).unwrap();
    let p = block(state.verify(s.clone(), id, fid, key(), true)).unwrap();
    assert!(block(state.import(s.clone(), p.token, true)).is_err());
    assert_eq!(
        s.run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap(),
        (1, 0)
    );
    assert!(root.is_dir());
    assert_eq!(
        s.run(false, |s| s
            .connection
            .query_row("SELECT count(*) FROM message_search", [], |r| r
                .get::<_, i64>(0))
            .map_err(|_| fail("native_database")))
            .unwrap(),
        0
    );
}
#[test]
fn inspection_missing_manifest_corruption_hash_mismatch_and_no_database_are_honest() {
    let (_t, s, state, id, fid, root) = environment(MODERN, "crypt15", true);
    fs::write(
        root.join(format!("databases/{fid}-msgstore.db.crypt15")),
        b"changed",
    )
    .unwrap();
    let report = block(state.inspect(s.clone(), id.clone())).unwrap();
    assert_eq!(report.candidates[0].capability, "INCOMPLETE_BACKUP");
    fs::remove_file(root.join("manifest.json")).unwrap();
    assert!(block(state.inspect(s, id)).is_err());
    let (_t, s, state, id, fid, root) = environment(MODERN, "crypt15", true);
    db(&s, |s| {
        s.connection
            .execute("DELETE FROM backup_files WHERE id=?1", [&fid])
            .unwrap();
        Ok(())
    })
    .unwrap();
    let mut m = crate::storage::backups::files::read_manifest(&root).unwrap();
    m.files.retain(|f| f.id != fid);
    fs::remove_file(root.join("manifest.json")).unwrap();
    crate::storage::backups::files::manifest(&root, &m).unwrap();
    assert_eq!(
        block(state.inspect(s, id)).unwrap().capability,
        "NO_MESSAGE_DATABASE"
    );
}
#[test]
fn incomplete_inventory_unknown_header_and_duplicate_candidates_are_not_ready_to_read() {
    let (_t, s, state, id, fid, root) = environment(MODERN, "crypt15", false);
    let mut m = crate::storage::backups::files::read_manifest(&root).unwrap();
    m.inventory_complete = false;
    m.warnings.push("timeout".into());
    fs::remove_file(root.join("manifest.json")).unwrap();
    crate::storage::backups::files::manifest(&root, &m).unwrap();
    assert_eq!(
        block(state.inspect(s.clone(), id.clone()))
            .unwrap()
            .capability,
        "INCOMPLETE_BACKUP"
    );
    let mut extra = m.files[0].clone();
    extra.id = uuid::Uuid::new_v4().to_string();
    extra.local_relative = format!("databases/{}-duplicate.crypt15", extra.id);
    fs::write(root.join(&extra.local_relative), MODERN).unwrap();
    db(&s,|s|{s.connection.execute("INSERT INTO backup_files(id,snapshot_id,category,source_relative,local_relative,original_name,expected_size,actual_bytes,sha256,local_verification,source_verification,size_validation,state) VALUES(?1,?2,'databases',?3,?4,?5,?6,?6,?7,'verified','verified','matched','verified')",params![extra.id,id,extra.source_relative,extra.local_relative,extra.original_name,extra.actual_bytes as i64,extra.sha256]).unwrap();Ok(())}).unwrap();
    m.files.push(extra);
    fs::remove_file(root.join("manifest.json")).unwrap();
    crate::storage::backups::files::manifest(&root, &m).unwrap();
    assert!(
        block(state.inspect(s, id))
            .unwrap()
            .candidates
            .iter()
            .any(|c| c.duplicate_of.is_some())
    );
    let (_t, s, state, id, _, _) =
        environment(b"not an encrypted header with SQLite", "crypt14", false);
    assert_eq!(
        block(state.inspect(s, id)).unwrap().candidates[0].capability,
        "NEEDS_INVESTIGATION"
    );
    let _ = fid;
}
#[test]
fn preview_cancel_removes_owned_plaintext_and_wrong_keys_create_no_archive() {
    let (_t, s, state, id, fid, _) = environment(MODERN, "crypt15", false);
    let p = block(state.verify(s.clone(), id.clone(), fid.clone(), key(), true)).unwrap();
    let root = state
        .0
        .lock()
        .unwrap()
        .ready
        .as_ref()
        .unwrap()
        .stage
        .root()
        .to_path_buf();
    assert!(root.exists());
    // Exercise cancellation between publishing a preview and dropping Active.
    let active = state.begin(true).unwrap();
    state.cancel().unwrap();
    assert!(!root.exists());
    drop(active);
    assert_eq!(
        block(state.import(s.clone(), p.token, true))
            .err()
            .unwrap()
            .code,
        "native_expired"
    );
    assert_eq!(
        block(state.verify(
            s.clone(),
            id,
            fid,
            KeyInput::Hex(Zeroizing::new("00".repeat(32))),
            true
        ))
        .err()
        .unwrap()
        .code,
        "native_auth"
    );
    assert_eq!(
        s.run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap()
            .1,
        0
    );
}
#[test]
fn large_synthetic_parser_is_bounded_ordered_and_cancellable() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("large.sqlite");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch("CREATE TABLE messages(_id INTEGER PRIMARY KEY,key_remote_jid TEXT,key_from_me INTEGER,key_id TEXT,data TEXT,timestamp INTEGER,remote_resource TEXT,media_wa_type INTEGER);WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<30000) INSERT INTO messages SELECT x,'opaque@lid',x%2,CAST(x AS TEXT),'Synthetic large record',1700000000000+x,NULL,0 FROM n;").unwrap();
    drop(c);
    let cancel = Arc::new(AtomicBool::new(false));
    let c = parser::open(&path, cancel.clone()).unwrap();
    let schema = parser::schema(&c).unwrap();
    let mut last = 0;
    let counts = parser::walk(
        &c,
        &schema,
        |r| {
            let t = r.timestamp.unwrap();
            assert!(t >= last);
            last = t;
            Ok(())
        },
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(counts.messages, 30000);
    assert_eq!(
        parser::walk(
            &c,
            &schema,
            |_| Ok(()),
            |n| if n > 100 {
                Err(fail("native_cancelled"))
            } else {
                Ok(())
            }
        )
        .err()
        .unwrap()
        .code,
        "native_cancelled"
    );
}
fn encrypt_payload(payload: &[u8]) -> Vec<u8> {
    use aes_gcm::{
        AesGcm, Nonce,
        aead::{AeadInPlace, KeyInit, consts::U16},
    };
    use md5::Md5;
    let h = crypto::header(MODERN).unwrap();
    let mut derived = [0u8; 32];
    hkdf::Hkdf::<Sha256>::new(Some(&[0u8; 32]), &[7u8; 32])
        .expand(b"backup encryption", &mut derived)
        .unwrap();
    let cipher = AesGcm::<aes::Aes256, U16>::new_from_slice(&derived).unwrap();
    let mut data = payload.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(Nonce::<U16>::from_slice(&h.iv), b"", &mut data)
        .unwrap();
    let mut out = MODERN[..h.offset].to_vec();
    out.extend(data);
    out.extend(tag);
    out.extend(Md5::digest(&out));
    out
}
#[test]
fn authenticated_but_unknown_schema_or_invalid_sqlite_never_creates_an_archive() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("unrelated.sqlite");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch("CREATE TABLE unrelated(id INTEGER);")
        .unwrap();
    drop(c);
    let data = encrypt_payload(&fs::read(&path).unwrap());
    let (_t, s, state, id, fid, _) = environment(&data, "crypt15", false);
    assert_eq!(
        block(state.verify(s.clone(), id, fid, key(), true))
            .err()
            .unwrap()
            .code,
        "native_schema"
    );
    assert_eq!(
        s.run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap()
            .1,
        0
    );
    let mut bad = b"SQLite format 3\0".to_vec();
    bad.extend([0u8; 256]);
    let data = encrypt_payload(&bad);
    let (_t, s, state, id, fid, _) = environment(&data, "crypt15", false);
    assert!(block(state.verify(s.clone(), id, fid, key(), true)).is_err());
    assert_eq!(
        s.run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap()
            .1,
        0
    );
}
#[test]
fn decompression_ratio_and_trailing_bytes_are_rejected() {
    let mut plain = b"SQLite format 3\0".to_vec();
    plain.resize(8 * 1024 * 1024, 0);
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(&plain).unwrap();
    let compressed = enc.finish().unwrap();
    let mut out = vec![];
    assert_eq!(
        crypto::plaintext(&compressed, &mut out, |_| Ok(()))
            .err()
            .unwrap()
            .code,
        "native_limit"
    );
    assert!(out.len() < plain.len());
    let h = crypto::header(MODERN).unwrap();
    let mut compressed = crypto::decrypt(
        Zeroizing::new(MODERN.to_vec()),
        &h,
        crypto::hex_key(&"07".repeat(32)).unwrap(),
        || Ok(()),
    )
    .unwrap();
    compressed.push(0);
    assert!(crypto::plaintext(&compressed, &mut vec![], |_| Ok(())).is_err());
}
#[test]
fn key_modes_multifile_and_ambiguous_media_remain_unsupported() {
    let h = crypto::header(MODERN).unwrap();
    let mut password = MODERN.to_vec();
    password[h.offset - 1] = 2;
    assert!(crypto::header(&password).is_err());
    let stripped = &MODERN[..MODERN.len() - 16];
    let (_t, s, state, id, _, _root) = environment(stripped, "crypt15", false);
    let report = block(state.inspect(s, id)).unwrap();
    assert_eq!(report.candidates[0].capability, "UNSUPPORTED_FORMAT");
    assert!(report.candidates[0].hash_verified);
    let (_t, s, _, id, _, root) = environment(MODERN, "crypt15", true);
    let mut snapshot = db(&s, |s| {
        let (vault, installation) = s.read_vault()?;
        inspector::load(&s.connection, &vault, &installation, &id)
    })
    .unwrap();
    let original = snapshot.entries[1].clone();
    snapshot.entries.push(original.clone());
    let media = parser::Media {
        path: Some("Media/WhatsApp Business Images/synthetic.png".into()),
        size: Some(PNG.len() as u64),
        hash: original.sha256,
        mime: Some("image/png".into()),
    };
    assert!(parser::match_media(&snapshot, &media).is_none());
    assert!(root.exists());
}
#[test]
fn owned_plaintext_crash_staging_is_recovered_without_touching_snapshot() {
    let (temp, s, state, _id, _fid, snapshot) = environment(MODERN, "crypt15", false);
    let orphan = db(&s, |s| {
        let (vault, installation) = s.read_vault()?;
        let id = uuid::Uuid::new_v4().to_string();
        let root = vault
            .join("temp")
            .join(format!("chatvault-import-{id}-crash"));
        fs::create_dir(&root).unwrap();
        staging::write_owner(&root, staging::STAGE_MARKER, &id, &installation)?;
        fs::write(root.join("lease.lock"), b"").unwrap();
        fs::write(
            root.join("plaintext.sqlite"),
            b"synthetic interrupted plaintext",
        )
        .unwrap();
        Ok(root)
    })
    .unwrap();
    drop(s);
    drop(state);
    let s = StorageState::new(Ok(temp.path().join("profile")));
    assert_eq!(
        s.run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap(),
        (1, 0)
    );
    assert!(!orphan.exists());
    assert!(snapshot.exists());
}
#[test]
fn migrations_six_and_seven_preserve_existing_backup_and_export_rows() {
    let mut c = rusqlite::Connection::open_in_memory().unwrap();
    crate::storage::migrations::apply(&mut c, &crate::storage::migrations::MIGRATIONS[..5])
        .unwrap();
    c.execute_batch("INSERT INTO archives(id,title,source_type,storage_path) VALUES('old','Original export','chat-export','archives/old');INSERT INTO backup_snapshots(id,status,storage_path,total_bytes) VALUES('old-snapshot','complete','backups/old-snapshot',123);").unwrap();
    crate::storage::migrations::migrate(&mut c).unwrap();
    crate::storage::migrations::migrate(&mut c).unwrap();
    assert_eq!(
        c.query_row("SELECT title FROM archives WHERE id='old'", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "Original export"
    );
    assert_eq!(
        c.query_row(
            "SELECT total_bytes FROM backup_snapshots WHERE id='old-snapshot'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        123
    );
    assert_eq!(
        c.query_row("PRAGMA foreign_key_check", [], |r| r.get::<_, String>(0))
            .optional()
            .unwrap(),
        None
    );
}
