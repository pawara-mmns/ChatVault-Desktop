use super::*;
use crate::devices::{
    adapter::*,
    models::{Result as DeviceResult, error},
};
use std::{
    collections::HashMap,
    sync::{Condvar, atomic::AtomicUsize},
};
#[derive(Clone)]
struct ScanFixture {
    tree: HashMap<String, Vec<Object>>,
    delay: Duration,
    fail_after: Option<usize>,
    failure_code: &'static str,
    block: Option<Arc<(Mutex<bool>, Condvar)>>,
    metadata_calls: Arc<AtomicUsize>,
    visits: Arc<Mutex<Vec<String>>>,
}
fn object(id: impl Into<String>, name: &str, folder: bool) -> Object {
    Object {
        id: id.into(),
        name: name.into(),
        folder,
        storage: false,
        metadata: (!folder).then(|| FileMetadata {
            name: name.into(),
            size: Some(7),
        }),
    }
}
fn fixture(files: usize) -> ScanFixture {
    let mut storage = object("s", "Storage", true);
    storage.storage = true;
    ScanFixture {
        tree: HashMap::from([
            ("DEVICE".into(), vec![storage]),
            ("s".into(), vec![object("app", "WhatsApp Business", true)]),
            (
                "app".into(),
                vec![
                    object("db", "Databases", true),
                    object("media", "Media", true),
                ],
            ),
            (
                "db".into(),
                vec![object("encrypted", "msgstore.db.crypt15", false)],
            ),
            (
                "media".into(),
                vec![
                    object("images", "WhatsApp Business Images", true),
                    object("video", "WhatsApp Business Video", true),
                ],
            ),
            (
                "images".into(),
                (0..files)
                    .map(|i| object(format!("file{i}"), "synthetic.jpg", false))
                    .collect(),
            ),
        ]),
        delay: Duration::ZERO,
        fail_after: None,
        failure_code: "access_denied",
        block: None,
        metadata_calls: Arc::new(AtomicUsize::new(0)),
        visits: Arc::new(Mutex::new(vec![])),
    }
}
impl DeviceAdapter for ScanFixture {
    fn enumerate(&self, _: &mut Budget) -> DeviceResult<Vec<RawDevice>> {
        Ok(vec![RawDevice {
            key: "opaque-private-fixture".into(),
            name: Some("Fixture".into()),
            manufacturer: None,
            usb: true,
        }])
    }
    fn open(&self, _: &str, _: &mut Budget) -> DeviceResult<Box<dyn Content>> {
        Ok(Box::new(self.clone()))
    }
}
impl Content for ScanFixture {
    fn identity(&self) -> Identity {
        Identity::default()
    }
    fn children(&self, parent: &str, _: &mut Budget) -> DeviceResult<Vec<Object>> {
        Ok(self.tree.get(parent).cloned().unwrap_or_default())
    }
    fn visit_children(
        &self,
        parent: &str,
        budget: &mut Budget,
        visitor: &mut dyn FnMut(Object) -> DeviceResult<()>,
    ) -> DeviceResult<()> {
        budget.check()?;
        self.visits.lock().unwrap().push(parent.into());
        budget.operation("enum_objects", || Ok(()))?;
        if parent == "images" {
            if let Some(gate) = &self.block {
                budget.operation("next_objects", || {
                    let mut released = gate.0.lock().unwrap();
                    while !*released {
                        released = gate.1.wait(released).unwrap();
                    }
                    Ok(())
                })?;
            }
        }
        for (i, o) in self
            .tree
            .get(parent)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            budget.entry()?;
            budget.operation("object_properties", || {
                if parent == "images" {
                    std::thread::sleep(self.delay);
                    if self.fail_after == Some(i) {
                        return Err(error(self.failure_code));
                    }
                }
                Ok(())
            })?;
            if let Some(trace) = &budget.trace {
                trace.object(o.folder || o.storage);
            }
            visitor(o)?;
        }
        Ok(())
    }
    fn file_metadata(&self, _: &str, _: &mut Budget) -> DeviceResult<FileMetadata> {
        self.metadata_calls.fetch_add(1, Ordering::SeqCst);
        Err(error("failed"))
    }
}
fn block<T>(f: impl std::future::Future<Output = T>) -> T {
    tauri::async_runtime::block_on(f)
}
fn environment(
    f: &ScanFixture,
) -> (
    tempfile::TempDir,
    StorageState,
    DeviceState,
    BackupState,
    String,
) {
    let temp = tempfile::tempdir().unwrap();
    let storage = StorageState::new(Ok(temp.path().join("profile")));
    // Time enumeration, not first-run migrations competing with parallel tests.
    storage.run(false, |_| Ok(())).unwrap();
    let devices = DeviceState::new(Arc::new(f.clone()));
    let state = BackupState::default();
    let id = block(devices.refresh()).unwrap().devices[0].id.clone();
    (temp, storage, devices, state, id)
}
fn prepare(f: ScanFixture, options: ScanOptions) -> (PlanSummary, BackupState) {
    let (_temp, storage, devices, state, id) = environment(&f);
    let plan =
        block(state.prepare_with_options(storage, devices, id, Source::Business, options, None))
            .unwrap();
    (plan, state)
}
#[test]
fn large_directory_preserves_partial_entries_and_reuses_enumeration_metadata() {
    let f = fixture(10_000);
    let calls = f.metadata_calls.clone();
    let (p, state) = prepare(
        f,
        ScanOptions {
            max_objects: 100,
            ..Default::default()
        },
    );
    assert!(!p.complete);
    assert!(p.total_files > 0 && p.total_files < 100);
    assert!(p.warnings.iter().any(|s| s == "limit"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(state.progress().unwrap().scan.unwrap().objects <= 100);
}
#[test]
fn scope_avoids_media_library_and_zero_categories_remain_zero() {
    let f = fixture(100_000);
    let visits = f.visits.clone();
    let (p, _) = prepare(
        f,
        ScanOptions {
            categories: vec![Category::Databases],
            ..Default::default()
        },
    );
    assert!(p.complete);
    assert_eq!(p.total_files, 1);
    assert!(
        !visits
            .lock()
            .unwrap()
            .iter()
            .any(|s| s == "images" || s == "media")
    );
    let (p, _) = prepare(
        fixture(0),
        ScanOptions {
            categories: vec![Category::Images],
            ..Default::default()
        },
    );
    assert!(p.complete);
    assert_eq!(p.total_files, 0);
}
#[test]
fn nested_cycles_and_depth_limits_are_explicitly_incomplete() {
    let mut f = fixture(0);
    f.tree
        .insert("images".into(), vec![object("n0", "nested", true)]);
    for n in 0..10 {
        f.tree.insert(
            format!("n{n}"),
            vec![object(format!("n{}", n + 1), "nested", true)],
        );
    }
    let (p, _) = prepare(
        f.clone(),
        ScanOptions {
            max_depth: 3,
            ..Default::default()
        },
    );
    assert!(!p.complete);
    assert!(p.warnings.contains(&"limit".into()));
    f.tree
        .insert("images".into(), vec![object("images", "cycle", true)]);
    let (p, _) = prepare(f, ScanOptions::default());
    assert!(!p.complete);
    assert!(p.warnings.contains(&"duplicate_object".into()));
}
#[test]
fn mid_directory_errors_keep_preceding_files_and_never_mark_a_backup_complete() {
    let mut f = fixture(20);
    f.fail_after = Some(3);
    let (p, state) = prepare(
        f,
        ScanOptions {
            categories: vec![Category::Images],
            ..Default::default()
        },
    );
    assert!(!p.complete);
    assert_eq!(p.total_files, 3);
    assert!(p.warnings.contains(&"access_denied".into()));
    assert_eq!(state.progress().unwrap().snapshot_id, None);
    assert_eq!(state.progress().unwrap().phase, "ready");
}
#[test]
fn slow_scan_emits_increasing_observations_and_cooperative_cancel_allows_retry() {
    let mut f = fixture(100);
    f.delay = Duration::from_millis(15);
    let (_temp, storage, devices, state, id) = environment(&f);
    let work = state.clone();
    let ds = devices.clone();
    let ss = storage.clone();
    let first_id = id.clone();
    let worker = std::thread::spawn(move || {
        block(work.prepare_with_options(
            ss,
            ds,
            first_id,
            Source::Business,
            ScanOptions::default(),
            None,
        ))
    });
    let start = Instant::now();
    while state
        .progress()
        .unwrap()
        .scan
        .is_none_or(|s| s.objects < 10)
    {
        assert!(start.elapsed() < Duration::from_secs(4));
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(550));
    state.cancel().unwrap();
    assert_eq!(worker.join().unwrap().unwrap_err().code, "cancelled");
    {
        let runtime = state.0.lock().unwrap();
        let observations = &runtime.observations;
        assert!(
            observations
                .iter()
                .any(|p| p.scan.as_ref().is_some_and(|s| s.objects > 10))
        );
        assert!(
            observations
                .windows(2)
                .all(|w| w[0].revision < w[1].revision)
        );
        assert!(observations.iter().all(|p| p.snapshot_id.is_none()));
    } // no additional worker is created by polling
    let ds = devices.clone();
    let p = block(BackupState::default().prepare_with_options(
        storage,
        ds,
        id,
        Source::Business,
        ScanOptions {
            categories: vec![Category::Databases],
            ..Default::default()
        },
        None,
    ))
    .unwrap();
    assert!(p.complete);
}
#[test]
fn blocked_call_returns_pending_but_keeps_device_reserved_until_safe_return() {
    let mut f = fixture(1);
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    f.block = Some(gate.clone());
    let (_temp, storage, devices, state, id) = environment(&f);
    let result = block(state.prepare_with_options(
        storage.clone(),
        devices.clone(),
        id.clone(),
        Source::Business,
        ScanOptions {
            timeout_seconds: 1,
            ..Default::default()
        },
        None,
    ));
    let before = state.progress().unwrap();
    let busy = devices.list().unwrap().busy;
    let rejected = state.begin().err().map(|e| e.code);
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    let started = Instant::now();
    while state.0.lock().unwrap().cancel.is_some() {
        assert!(started.elapsed() < Duration::from_secs(4));
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(result.unwrap_err().code, "driver_pending");
    assert_eq!(before.phase, "scan_waiting");
    assert!(before.scan.unwrap().operation_pending);
    assert!(busy);
    assert_eq!(rejected.as_deref(), Some("busy"));
    assert!(!devices.list().unwrap().busy);
    assert!(state.0.lock().unwrap().plan.is_none());
    assert_eq!(state.progress().unwrap().phase, "scan_failed");
    let p = block(state.prepare_with_options(
        storage,
        devices,
        id,
        Source::Business,
        ScanOptions {
            categories: vec![Category::Databases],
            ..Default::default()
        },
        None,
    ))
    .unwrap();
    assert!(p.complete);
}
#[test]
fn stale_scan_publications_cannot_replace_terminal_or_new_jobs() {
    let state = BackupState::default();
    let active = state.begin().unwrap();
    let mut p = Progress {
        operation_id: Some(active.id.clone()),
        phase: "scanning".into(),
        ..Default::default()
    };
    state.publish(&p, None);
    p.phase = "ready".into();
    state.publish(&p, None);
    let revision = state.progress().unwrap().revision;
    p.phase = "scanning".into();
    state.publish(&p, None);
    assert_eq!(state.progress().unwrap().revision, revision);
    drop(active);
    let active = state.begin().unwrap();
    state.publish(
        &Progress {
            operation_id: Some(active.id.clone()),
            phase: "scanning".into(),
            ..Default::default()
        },
        None,
    );
    p.phase = "scan_failed".into();
    state.publish(&p, None);
    assert_eq!(
        state.progress().unwrap().operation_id.as_deref(),
        Some(active.id.as_str())
    );
    assert_eq!(state.progress().unwrap().phase, "scanning");
}

#[test]
fn disconnected_scan_reports_failure_instead_of_a_complete_empty_inventory() {
    let mut f = fixture(10);
    f.fail_after = Some(2);
    f.failure_code = "disconnected";
    let (_temp, storage, devices, state, id) = environment(&f);
    let result = block(state.prepare_with_options(
        storage,
        devices,
        id,
        Source::Business,
        ScanOptions {
            categories: vec![Category::Images],
            ..Default::default()
        },
        None,
    ));
    assert_eq!(result.unwrap_err().code, "disconnected");
    assert_eq!(state.progress().unwrap().phase, "scan_failed");
    assert_eq!(
        state.progress().unwrap().scan.unwrap().error_category,
        Some("disconnected")
    );
    assert!(state.0.lock().unwrap().plan.is_none());
}
#[test]
fn cooperative_deadline_returns_observed_partial_inventory_without_pending_driver_claim() {
    let mut f = fixture(300);
    f.delay = Duration::from_millis(10);
    let (p, state) = prepare(
        f,
        ScanOptions {
            categories: vec![Category::Images],
            timeout_seconds: 1,
            ..Default::default()
        },
    );
    assert!(!p.complete);
    assert!(p.total_files > 0 && p.total_files < 300);
    assert!(p.warnings.contains(&"timeout".into()));
    assert_eq!(state.progress().unwrap().phase, "ready");
    assert!(!state.progress().unwrap().scan.unwrap().operation_pending);
}
