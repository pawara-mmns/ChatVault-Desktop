use super::*;
use crate::devices::adapter::{
    Content, DeviceAdapter, DeviceRead, FileMetadata, Identity, Object, RawDevice,
};
use crate::devices::models::{Result as DeviceResult, error};
use std::sync::atomic::AtomicUsize;
use tempfile::TempDir;

#[cfg(windows)]
#[test]
#[ignore = "requires authorized connected phone; metadata only"]
fn live_business_scoped_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let storage = StorageState::new(Ok(temp.path().join("profile")));
    let devices = DeviceState::default();
    let list = block(devices.refresh()).unwrap();
    assert_eq!(list.devices.len(), 1, "expected one authorized phone");
    let state = BackupState::default();
    let plan = block(state.prepare_with_options(
        storage.clone(),
        devices,
        list.devices[0].id.clone(),
        Source::Business,
        ScanOptions {
            categories: vec![Category::Databases],
            max_objects: 2000,
            timeout_seconds: 20,
            ..Default::default()
        },
        None,
    ))
    .unwrap();
    println!(
        "LIVE SCOPED DISCOVERY: complete={}; files={}; known_bytes={}; warnings={:?}",
        plan.complete, plan.total_files, plan.known_bytes, plan.warnings
    );
    assert!(plan.complete, "scoped inventory incomplete");
    assert!(
        plan.total_files > 0,
        "no accessible encrypted database metadata found"
    );
    assert_eq!(
        storage
            .run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap()
            .0,
        0
    );
}

#[derive(Clone)]
struct Fixture {
    bytes: u64,
    reported: Option<u64>,
    files: usize,
    short: usize,
    failure: Option<&'static str>,
    changed: bool,
    slow: bool,
    reads: Arc<AtomicUsize>,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            bytes: 8193,
            reported: Some(8193),
            files: 1,
            short: 701,
            failure: None,
            changed: false,
            slow: false,
            reads: Arc::new(AtomicUsize::new(0)),
        }
    }
}
fn object(id: &str, name: &str, folder: bool, storage: bool) -> Object {
    Object {
        metadata: None,
        id: id.into(),
        name: name.into(),
        folder,
        storage,
    }
}
impl DeviceAdapter for Fixture {
    fn enumerate(&self, _: &mut Budget) -> DeviceResult<Vec<RawDevice>> {
        Ok(vec![RawDevice {
            key: "synthetic-private-key".into(),
            name: Some("Synthetic phone".into()),
            manufacturer: None,
            usb: true,
        }])
    }
    fn open(&self, _: &str, _: &mut Budget) -> DeviceResult<Box<dyn Content>> {
        Ok(Box::new(self.clone()))
    }
}
impl Content for Fixture {
    fn identity(&self) -> Identity {
        Identity::default()
    }
    fn children(&self, parent: &str, budget: &mut Budget) -> DeviceResult<Vec<Object>> {
        budget.check()?;
        let v = match parent {
            "DEVICE" => vec![object("storage", "Storage", false, true)],
            "storage" => vec![object("app", "WhatsApp Business", true, false)],
            "app" => vec![object("media", "Media", true, false)],
            "media" => vec![object("images", "WhatsApp Business Images", true, false)],
            "images" => (0..self.files)
                .map(|i| object(&format!("file{i}"), "same.jpg", false, false))
                .collect(),
            _ => vec![],
        };
        for _ in &v {
            budget.entry()?;
        }
        Ok(v)
    }
    fn file_metadata(&self, _: &str, _: &mut Budget) -> DeviceResult<FileMetadata> {
        Ok(FileMetadata {
            name: "same.jpg".into(),
            size: self.reported,
        })
    }
    fn read_stream(&self, _: &str, _: &mut Budget) -> DeviceResult<Box<dyn DeviceRead>> {
        if self.failure == Some("unsupported") {
            return Err(error("unsupported"));
        }
        let read = self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(Reader {
            remaining: self.bytes,
            chunk: self.short,
            failure: self.failure,
            changed: self.changed && read > 0,
            slow: self.slow,
        }))
    }
}
struct Reader {
    remaining: u64,
    chunk: usize,
    failure: Option<&'static str>,
    changed: bool,
    slow: bool,
}
impl DeviceRead for Reader {
    fn read(&mut self, buffer: &mut [u8]) -> DeviceResult<usize> {
        if let Some(code) = self.failure {
            return Err(error(code));
        }
        if self.slow {
            std::thread::sleep(Duration::from_millis(2));
        }
        let n = buffer.len().min(self.chunk).min(self.remaining as usize);
        buffer[..n].fill(if self.changed { 43 } else { 42 });
        self.remaining -= n as u64;
        Ok(n)
    }
}
fn block<T>(f: impl std::future::Future<Output = T>) -> T {
    tauri::async_runtime::block_on(f)
}
fn selection() -> Selection {
    Selection {
        categories: vec![Category::Images],
        file_ids: None,
        source_reread: true,
        acknowledge_incomplete: true,
        acknowledge_capacity: true,
    }
}
fn environment(f: Fixture) -> (TempDir, StorageState, DeviceState, BackupState, PlanSummary) {
    let temp = tempfile::tempdir().unwrap();
    let storage = StorageState::new(Ok(temp.path().join("profile")));
    let devices = DeviceState::new(Arc::new(f));
    let state = BackupState::default();
    let device = block(devices.refresh()).unwrap().devices.remove(0);
    let plan = block(state.prepare(
        storage.clone(),
        devices.clone(),
        device.id,
        Source::Business,
        None,
    ))
    .unwrap();
    (temp, storage, devices, state, plan)
}
fn wait(state: &BackupState) -> Progress {
    let started = Instant::now();
    loop {
        let p = state.progress().unwrap();
        if !matches!(
            p.phase.as_str(),
            "preparing" | "copying" | "verifying" | "committing"
        ) && state.0.lock().unwrap().cancel.is_none()
        {
            return p;
        }
        assert!(
            started.elapsed() < Duration::from_secs(45),
            "job did not finish"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn run(f: Fixture) -> (TempDir, StorageState, BackupState, Progress, PathBuf) {
    let (temp, storage, devices, state, plan) = environment(f);
    let vault = PathBuf::from(&plan.vault_path);
    block(state.start(storage.clone(), devices, plan.id, selection(), None)).unwrap();
    let p = wait(&state);
    (temp, storage, state, p, vault)
}

#[test]
fn streams_short_reads_and_independently_verifies_source_and_local_copy() {
    let (_temp, storage, state, p, vault) = run(Fixture::default());
    assert_eq!(p.phase, "completed");
    assert_eq!(
        (p.copied_bytes, p.verified_bytes, p.successful_files),
        (8193, 8193, 1)
    );
    let id = p.snapshot_id.unwrap();
    let manifest = files::read_manifest(&vault.join("backups").join(&id)).unwrap();
    assert_eq!(manifest.files[0].source_verification, "verified");
    assert_eq!(manifest.files[0].local_verification, "verified");
    assert_eq!(manifest.files[0].size_validation, "matched");
    assert_eq!(manifest.files[0].sha256.as_ref().unwrap().len(), 64);
    assert_eq!(
        block(state.reverify(storage.clone(), id, None))
            .unwrap()
            .phase,
        "verification_completed"
    );
    assert_eq!(
        storage
            .run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap()
            .0,
        1
    );
}
#[test]
fn empty_files_and_case_colliding_names_are_unique_and_preserved() {
    let (_temp, _, _, p, vault) = run(Fixture {
        bytes: 0,
        reported: Some(0),
        files: 3,
        ..Default::default()
    });
    assert_eq!(p.phase, "completed");
    let manifest =
        files::read_manifest(&vault.join("backups").join(p.snapshot_id.unwrap())).unwrap();
    let paths: std::collections::HashSet<_> = manifest
        .files
        .iter()
        .map(|f| f.local_relative.to_lowercase())
        .collect();
    assert_eq!(paths.len(), 3);
    assert!(
        manifest
            .files
            .iter()
            .all(|f| f.original_name == "same.jpg" && f.actual_bytes == 0)
    );
}
#[test]
fn large_generated_file_is_copied_in_bounded_chunks() {
    let bytes = 20 * 1024 * 1024 + 17;
    let (_t, _, _, p, _) = run(Fixture {
        bytes,
        reported: Some(bytes),
        short: 131_071,
        ..Default::default()
    });
    assert_eq!(p.phase, "completed");
    assert_eq!(p.verified_bytes, bytes);
}
#[test]
fn truncated_file_never_publishes_completed_snapshot() {
    let (_t, storage, _, p, vault) = run(Fixture {
        reported: Some(9000),
        ..Default::default()
    });
    assert_eq!(p.phase, "failed");
    assert_eq!(p.failed_files, 1);
    assert!(
        !vault
            .join("backups")
            .join(p.snapshot_id.as_ref().unwrap())
            .exists()
    );
    assert_eq!(
        storage
            .run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap()
            .0,
        0
    );
    let entry = db(&storage, |s| {
        repository::entries(&s.connection, p.snapshot_id.as_ref().unwrap(), 0, 50)
    })
    .unwrap()
    .remove(0);
    assert_eq!(entry.error_code.as_deref(), Some("source_changed"));
}
#[test]
fn source_reread_detects_same_size_content_changes() {
    let (_t, _, _, p, _) = run(Fixture {
        changed: true,
        ..Default::default()
    });
    assert_eq!(p.phase, "failed");
    assert_eq!(p.successful_files, 0);
    assert_eq!(p.verified_bytes, 0);
}
#[test]
fn disconnected_and_unsupported_resources_remain_incomplete() {
    for failure in ["disconnected", "unsupported", "access_denied"] {
        let (_t, _, _, p, vault) = run(Fixture {
            failure: Some(failure),
            ..Default::default()
        });
        assert_eq!(p.phase, "failed");
        assert!(!vault.join("backups").join(p.snapshot_id.unwrap()).exists());
    }
}
#[test]
fn unknown_sizes_are_truthful_and_require_capacity_acknowledgement() {
    let (_t, storage, devices, state, plan) = environment(Fixture {
        reported: None,
        ..Default::default()
    });
    assert_eq!(plan.unknown_sizes, 1);
    assert_eq!(plan.known_bytes, 0);
    let mut sel = selection();
    sel.acknowledge_capacity = false;
    assert_eq!(
        block(state.start(storage, devices, plan.id, sel, None))
            .unwrap_err()
            .code,
        "capacity_unknown"
    );
}
#[test]
fn cancellation_releases_device_and_preserves_owned_incomplete_work() {
    let (_t, storage, devices, state, plan) = environment(Fixture {
        bytes: 32 * 1024 * 1024,
        reported: Some(32 * 1024 * 1024),
        slow: true,
        ..Default::default()
    });
    let vault = PathBuf::from(&plan.vault_path);
    block(state.start(storage.clone(), devices.clone(), plan.id, selection(), None)).unwrap();
    state.cancel().unwrap();
    let p = wait(&state);
    assert_eq!(p.phase, "cancelled");
    assert!(!devices.list().unwrap().busy);
    assert!(!vault.join("backups").join(p.snapshot_id.unwrap()).exists());
}
#[test]
fn repeated_backups_and_restart_preserve_history_and_detect_local_corruption() {
    let (temp, storage, devices, state, plan) = environment(Fixture::default());
    let vault = PathBuf::from(&plan.vault_path);
    let device = plan.device_id.clone();
    block(state.start(storage.clone(), devices.clone(), plan.id, selection(), None)).unwrap();
    let first = wait(&state).snapshot_id.unwrap();
    let plan = block(state.prepare(
        storage.clone(),
        devices.clone(),
        device,
        Source::Business,
        None,
    ))
    .unwrap();
    block(state.start(storage.clone(), devices, plan.id, selection(), None)).unwrap();
    let second = wait(&state).snapshot_id.unwrap();
    assert_ne!(first, second);
    let manifest = files::read_manifest(&vault.join("backups").join(&first)).unwrap();
    fs::write(
        files::join(
            &vault.join("backups").join(&first),
            &manifest.files[0].local_relative,
        )
        .unwrap(),
        b"corrupt",
    )
    .unwrap();
    assert_eq!(
        block(state.reverify(storage.clone(), first.clone(), None))
            .unwrap()
            .phase,
        "verification_failed"
    );
    drop(storage);
    let restarted = StorageState::new(Ok(temp.path().join("profile")));
    let rows = db(&restarted, |s| {
        repository::list(&s.connection, &vault, 0, 20)
    })
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .any(|s| s.id == first && s.local_integrity == "failed")
    );
    assert!(vault.join("backups").join(second).exists());
}
#[test]
fn source_selection_and_file_handles_are_authorized() {
    let f = Fixture::default();
    let mut budget = Budget::backup(Arc::new(AtomicBool::new(false)), 30);
    let plan = planner::inventory(
        &f,
        "device".into(),
        None,
        Source::Whatsapp,
        "vault".into(),
        None,
        &mut budget,
    )
    .unwrap();
    assert_eq!(plan.summary.total_files, 0);
    let (_t, _, _, state, plan) = environment(f);
    let mut sel = selection();
    sel.file_ids = Some(vec![uuid::Uuid::new_v4().to_string()]);
    let lock = state.0.lock().unwrap();
    assert_eq!(
        planner::select(lock.plan.as_ref().unwrap(), &sel)
            .err()
            .unwrap()
            .code,
        "invalid_selection"
    );
    assert_eq!(lock.plan.as_ref().unwrap().summary.id, plan.id);
}
#[test]
fn unsafe_names_traversal_ownership_and_collision_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    for path in [
        "../escape",
        "a/../../escape",
        "C:/escape",
        "a\\escape",
        "CON/file",
        "a/aux.txt",
        "a/x:y",
        "/absolute",
        "a/evil*",
    ] {
        assert!(files::join(temp.path(), path).is_err(), "{path}");
    }
    for name in ["CON", "nul.txt", "../../evil", "bad:name?.jpg", "   "] {
        let sanitized = files::safe_component(name);
        assert!(
            files::join(
                temp.path(),
                &format!("{}/{}", uuid::Uuid::new_v4(), sanitized)
            )
            .is_ok()
        );
    }
    let id = uuid::Uuid::new_v4().to_string();
    files::owner(temp.path(), &id, "owner").unwrap();
    assert!(files::verify_owner(temp.path(), &id, "other").is_err());
    let old = temp.path().join("old");
    let new = temp.path().join("new");
    fs::write(&old, "first").unwrap();
    fs::write(&new, "previous").unwrap();
    assert!(files::rename_new(&old, &new).is_err());
    assert_eq!(fs::read(new).unwrap(), b"previous");
    assert_eq!(
        io(std::io::Error::from_raw_os_error(112)).code,
        "storage_full"
    );
}
#[test]
fn interrupted_jobs_recover_without_publishing_or_deleting_unrelated_files() {
    let (_t, storage, _, state, plan) = environment(Fixture::default());
    let id = uuid::Uuid::new_v4().to_string();
    let planned = state.0.lock().unwrap().plan.as_ref().unwrap().files.clone();
    db(&storage, |s| {
        repository::start(
            &mut s.connection,
            &id,
            Source::Business,
            None,
            &planned,
            &[],
        )
    })
    .unwrap();
    let unrelated = PathBuf::from(&plan.vault_path).join("temp/unrelated.txt");
    fs::write(&unrelated, "preserve").unwrap();
    db(&storage, |s| s.backup_recovery()).unwrap();
    let rows = db(&storage, |s| {
        repository::list(&s.connection, &PathBuf::from(&plan.vault_path), 0, 20)
    })
    .unwrap();
    assert_eq!(rows[0].phase, "interrupted");
    assert_eq!(fs::read(unrelated).unwrap(), b"preserve");
}

#[test]
fn disk_full_denied_write_and_removed_destination_fail_at_real_writer_boundary() {
    struct BrokenWriter(i32);
    impl std::io::Write for BrokenWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from_raw_os_error(self.0))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    for (os, expected) in [
        (112, "storage_full"),
        (5, "vault_unavailable"),
        (21, "vault_unavailable"),
    ] {
        let f = Fixture::default();
        let mut budget = Budget::backup(Arc::new(AtomicBool::new(false)), 30);
        let mut stream = f.read_stream("file", &mut budget).unwrap();
        let mut progress = Progress::default();
        let result = transfer::receive(
            stream.as_mut(),
            &mut BrokenWriter(os),
            &mut [0; 4096],
            &budget,
            &mut progress,
            &mut |_| {},
        );
        assert_eq!(result.unwrap_err().code, expected);
        assert_eq!(progress.copied_bytes, 0);
    }
}
#[test]
fn missing_vault_is_not_recreated_for_backup_and_recovery_still_records_interruption() {
    let (_t, storage, devices, state, plan) = environment(Fixture::default());
    let id = uuid::Uuid::new_v4().to_string();
    let planned = state.0.lock().unwrap().plan.as_ref().unwrap().files.clone();
    db(&storage, |s| {
        repository::start(
            &mut s.connection,
            &id,
            Source::Business,
            None,
            &planned,
            &[],
        )
    })
    .unwrap();
    let root = PathBuf::from(&plan.vault_path);
    let disconnected = root.with_file_name("disconnected-vault");
    fs::rename(&root, &disconnected).unwrap();
    db(&storage, |s| s.backup_recovery()).unwrap();
    let phase: String = storage
        .run(false, |s| {
            s.connection
                .query_row(
                    "SELECT phase FROM backup_snapshots WHERE id=?1",
                    [&id],
                    |r| r.get(0),
                )
                .map_err(|_| crate::storage::error::StorageError::database())
        })
        .unwrap();
    assert_eq!(phase, "interrupted");
    assert_eq!(
        block(state.start(storage, devices, plan.id, selection(), None))
            .unwrap_err()
            .code,
        "vault_unavailable"
    );
    assert!(!root.exists());
}
#[test]
fn owned_cleanup_rejects_hardlinks_and_foreign_markers() {
    let temp = tempfile::tempdir().unwrap();
    let stage = temp.path().join("stage");
    fs::create_dir(&stage).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    files::owner(&stage, &id, "installation").unwrap();
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, "unchanged").unwrap();
    fs::hard_link(&outside, stage.join("linked.txt")).unwrap();
    assert!(files::cleanup(&stage, &id, "installation").is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"unchanged");
    fs::remove_file(stage.join("linked.txt")).unwrap();
    assert!(files::cleanup(&stage, &id, "foreign").is_err());
    files::cleanup(&stage, &id, "installation").unwrap();
    assert!(!stage.exists());
    assert!(outside.exists());
}
#[test]
fn migration_five_preserves_existing_rows_and_reapplication_is_safe() {
    use crate::storage::migrations;
    let mut c = rusqlite::Connection::open_in_memory().unwrap();
    migrations::apply(&mut c, &migrations::MIGRATIONS[..4]).unwrap();
    c.execute_batch("INSERT INTO archives(id,title,source_type,storage_path) VALUES('old-archive','Synthetic','chat-export','archives/old'); INSERT INTO backup_snapshots(id,status,storage_path,total_bytes) VALUES('old-backup','complete','backups/old',123);").unwrap();
    migrations::migrate(&mut c).unwrap();
    migrations::migrate(&mut c).unwrap();
    let archive: String = c
        .query_row(
            "SELECT title FROM archives WHERE id='old-archive'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let old: (String, i64) = c
        .query_row(
            "SELECT phase,total_bytes FROM backup_snapshots WHERE id='old-backup'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(archive, "Synthetic");
    assert_eq!(old, ("completed".into(), 123));
    assert_eq!(
        c.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn failed_snapshot_records_block_vault_switch_even_without_files() {
    let temp = tempfile::tempdir().unwrap();
    let mut service = StorageService::open(&temp.path().join("profile"), &[]).unwrap();
    service.connection.execute("INSERT INTO backup_snapshots(id,status,phase,storage_path) VALUES(?1,'failed','failed',?2)",rusqlite::params![uuid::Uuid::new_v4().to_string(),"backups/empty-failed"]).unwrap();
    assert!(service.ensure_switchable().is_err());
    assert!(
        service
            .change_vault(&temp.path().join("different-vault"))
            .is_err()
    );
    assert!(!temp.path().join("different-vault").exists());
}

// Run only after explicit user authorization. Copies ONE <=256 KiB image, never
// prints names/identifiers/content, and retains the isolated vault for inspection.
#[cfg(windows)]
#[test]
#[ignore = "requires connected authorized Android USB/MTP phone"]
fn live_one_small_image_readonly_transfer() {
    let temp = tempfile::tempdir().unwrap().keep();
    let storage = StorageState::new(Ok(temp.join("profile")));
    let devices = DeviceState::default();
    let list = block(devices.refresh()).unwrap();
    assert_eq!(
        list.devices.len(),
        1,
        "expected exactly one authorized phone"
    );
    let state = BackupState::default();
    let plan = block(state.prepare_with_options(
        storage.clone(),
        devices.clone(),
        list.devices[0].id.clone(),
        Source::Business,
        ScanOptions {
            categories: vec![Category::Images],
            max_objects: 500,
            timeout_seconds: 30,
            ..Default::default()
        },
        None,
    ))
    .unwrap();
    let file = state
        .0
        .lock()
        .unwrap()
        .plan
        .as_ref()
        .unwrap()
        .files
        .iter()
        .filter(|f| {
            f.entry.category == Category::Images
                && f.entry
                    .expected_size
                    .is_some_and(|n| n > 0 && n <= 256 * 1024)
        })
        .min_by_key(|f| f.entry.expected_size)
        .cloned()
        .expect("no small accessible image");
    let mut sel = selection();
    sel.file_ids = Some(vec![file.entry.id]);
    block(state.start(storage.clone(), devices, plan.id, sel, None)).unwrap();
    let started = Instant::now();
    let p = loop {
        let p = state.progress().unwrap();
        if state.0.lock().unwrap().cancel.is_none() {
            break p;
        }
        assert!(
            started.elapsed() < Duration::from_secs(180),
            "live transfer timed out"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(
        matches!(p.phase.as_str(), "completed" | "completed_with_warnings"),
        "live operation failed: {:?}",
        p.error
    );
    assert_eq!(p.successful_files, 1);
    let id = p.snapshot_id.unwrap();
    let root = PathBuf::from(plan.vault_path).join("backups").join(&id);
    let manifest = files::read_manifest(&root).unwrap();
    assert_eq!(manifest.files[0].source_verification, "verified");
    assert_eq!(manifest.files[0].local_verification, "verified");
    assert_eq!(
        manifest.files[0].expected_size,
        Some(manifest.files[0].actual_bytes)
    );
    assert_eq!(
        block(state.reverify(storage.clone(), id, None))
            .unwrap()
            .phase,
        "verification_completed"
    );
    drop(storage);
    let reopened = StorageState::new(Ok(temp.join("profile")));
    assert_eq!(
        reopened
            .run(false, |s| crate::storage::repository::counts(&s.connection))
            .unwrap()
            .0,
        1
    );
    println!(
        "LIVE EVIDENCE: one file; bytes={}; incoming/local/source SHA-256 matched; local reverify passed; reopened history=1; isolated profile={}",
        manifest.files[0].actual_bytes,
        temp.display()
    );
}
