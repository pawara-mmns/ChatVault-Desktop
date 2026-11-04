use super::*;
use adapter::{Content, Identity, Object};
use std::sync::atomic::AtomicUsize;
#[derive(Clone, Default)]
struct Fixture {
    keys: Vec<String>,
    missing_metadata: bool,
    usb: bool,
    failure: Option<&'static str>,
    tree: HashMap<String, Vec<Object>>,
    denied: Option<String>,
    android: bool,
}
#[derive(Clone)]
struct Mock(Arc<Mutex<Fixture>>, Arc<AtomicUsize>);
impl DeviceAdapter for Mock {
    fn enumerate(&self, _: &mut Budget) -> Result<Vec<RawDevice>> {
        let f = self.0.lock().unwrap();
        Ok(f.keys
            .iter()
            .map(|key| RawDevice {
                key: key.clone(),
                name: (!f.missing_metadata)
                    .then(|| "Synthetic phone <img onerror=bad>\u{202e}\n".into()),
                manufacturer: (!f.missing_metadata).then(|| "Synthetic manufacturer".into()),
                usb: f.usb,
            })
            .collect())
    }
    fn open(&self, _: &str, _: &mut Budget) -> Result<Box<dyn Content>> {
        self.1.fetch_add(1, Ordering::Relaxed);
        let f = self.0.lock().unwrap().clone();
        if let Some(code) = f.failure {
            return Err(error(code));
        }
        Ok(Box::new(f))
    }
}
impl Content for Fixture {
    fn identity(&self) -> Identity {
        Identity {
            category: Some("Phone".into()),
            protocol: self.android.then(|| "Android".into()),
        }
    }
    fn children(&self, parent: &str, budget: &mut Budget) -> Result<Vec<Object>> {
        budget.check()?;
        if self.denied.as_deref() == Some(parent) {
            return Err(error("access_denied"));
        }
        let children = self.tree.get(parent).cloned().unwrap_or_default();
        if children.len() > adapter::MAX_CHILDREN {
            return Err(error("limit"));
        }
        for _ in &children {
            budget.entry()?;
        }
        Ok(children)
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
fn fixture() -> Fixture {
    let mut f = Fixture {
        keys: vec!["private-pnp-id".into()],
        usb: true,
        ..Default::default()
    };
    for (parent, objects) in [
        (
            "DEVICE",
            vec![object("s", "Internal shared storage", false, true)],
        ),
        ("s", vec![object("a", "aNdRoId", true, false)]),
        ("a", vec![object("m", "media", true, false)]),
        ("m", vec![object("p", "com.whatsapp", true, false)]),
        ("p", vec![object("w", "WhatsApp", true, false)]),
        (
            "w",
            vec![
                object("d", "Databases", true, false),
                object("media", "Media", true, false),
            ],
        ),
        ("d", vec![object("db", "msgstore.db.crypt15", false, false)]),
        (
            "media",
            vec![
                object("images", "WhatsApp Images", true, false),
                object("audio", "WhatsApp Audio", true, false),
            ],
        ),
    ] {
        f.tree.insert(parent.into(), objects);
    }
    f
}
fn setup(f: Fixture) -> (DeviceState, Mock) {
    let mock = Mock(Arc::new(Mutex::new(f)), Arc::new(AtomicUsize::new(0)));
    (DeviceState::new(Arc::new(mock.clone())), mock)
}
fn block<T>(f: impl std::future::Future<Output = T>) -> T {
    tauri::async_runtime::block_on(f)
}
fn first(state: &DeviceState) -> Device {
    block(state.refresh()).unwrap().devices.remove(0)
}
#[test]
fn no_devices_and_not_scanned_are_distinct() {
    let (s, _) = setup(Fixture::default());
    assert!(s.list().unwrap().stale);
    let list = block(s.refresh()).unwrap();
    assert!(!list.stale);
    assert!(list.devices.is_empty());
    assert!(list.scanned_at.is_some());
}
#[test]
fn multiple_deduplicated_opaque_handles() {
    let mut f = fixture();
    f.keys = vec!["one".into(), "two".into(), "one".into()];
    let (s, _) = setup(f);
    let list = block(s.refresh()).unwrap();
    assert_eq!(list.devices.len(), 2);
    for d in list.devices {
        assert!(uuid::Uuid::parse_str(&d.id).is_ok());
        assert!(!serde_json::to_string(&d).unwrap().contains("private-pnp"));
    }
}
#[test]
fn unavailable_metadata_not_invented() {
    let mut f = fixture();
    f.missing_metadata = true;
    let (s, _) = setup(f);
    let d = first(&s);
    assert!(d.name.is_none());
    assert!(d.manufacturer.is_none());
    assert_eq!(d.android_identity, "unknown");
    assert_eq!(d.access, Access::Unchecked);
}
#[test]
fn metadata_cleaned_and_bounded() {
    assert_eq!(clean(" \n\u{202e}phone\0 ").unwrap(), "phone");
    assert_eq!(clean(&"x".repeat(1000)).unwrap().len(), 160);
    let (s, _) = setup(fixture());
    let d = first(&s);
    assert!(!d.name.unwrap().contains('\n'));
}
#[test]
fn phone_and_mtp_do_not_confirm_android() {
    let (s, _) = setup(fixture());
    let id = first(&s).id;
    let d = block(s.inspect(id, false)).unwrap();
    assert_eq!(d.category.as_deref(), Some("Phone"));
    assert_eq!(d.android_identity, "unknown");
    assert_eq!(d.access, Access::Available);
    assert!(d.discovery.is_none());
}
#[test]
fn explicit_driver_android_report() {
    let mut f = fixture();
    f.android = true;
    let (s, _) = setup(f);
    let id = first(&s).id;
    assert_eq!(
        block(s.inspect(id, false)).unwrap().android_identity,
        "driver_reported_android"
    );
}
#[test]
fn non_usb_never_opened() {
    let mut f = fixture();
    f.usb = false;
    let (s, m) = setup(f);
    let id = first(&s).id;
    assert_eq!(
        block(s.inspect(id, true)).unwrap().access,
        Access::Unsupported
    );
    assert_eq!(m.1.load(Ordering::Relaxed), 0);
}
#[test]
fn locked_access_denied() {
    let mut f = fixture();
    f.failure = Some("access_denied");
    let (s, _) = setup(f);
    let id = first(&s).id;
    let d = block(s.inspect(id, true)).unwrap();
    assert_eq!(d.access, Access::PermissionRequired);
    assert!(d.discovery.is_none());
    assert_eq!(d.diagnostic.unwrap().code, "access_denied");
}
#[test]
fn empty_storage_is_not_file_access() {
    let mut f = fixture();
    f.tree.clear();
    let (s, _) = setup(f);
    let id = first(&s).id;
    assert_eq!(
        block(s.inspect(id, false)).unwrap().access,
        Access::Unavailable
    );
}
#[test]
fn discovery_metadata_only_success() {
    let (s, _) = setup(fixture());
    let id = first(&s).id;
    let d = block(s.inspect(id, true)).unwrap();
    let report = d.discovery.unwrap();
    assert_eq!(report.status, "complete");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.category == "images" && f.status == "accessible")
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.encrypted_backup_found == Some(true))
    );
    assert!(report.examined < 2500);
}
#[test]
fn missing_directory_is_candidate_only() {
    let mut f = fixture();
    f.tree.insert("s".into(), vec![]);
    let (s, _) = setup(f);
    let id = first(&s).id;
    let report = block(s.inspect(id, true)).unwrap().discovery.unwrap();
    assert_eq!(report.status, "complete");
    assert_eq!(report.findings.len(), 4);
    assert!(report.findings.iter().all(|f| f.status == "not_found"));
}
#[test]
fn denied_folder_is_not_absent() {
    let mut f = fixture();
    f.denied = Some("p".into());
    let (s, _) = setup(f);
    let id = first(&s).id;
    let report = block(s.inspect(id, true)).unwrap().discovery.unwrap();
    assert_eq!(report.status, "incomplete");
    assert!(report.findings.iter().any(|f| f.status == "access_denied"));
}
#[test]
fn unplug_and_reconnect_revoke_handle() {
    let (s, m) = setup(fixture());
    let id = first(&s).id;
    m.0.lock().unwrap().keys.clear();
    assert_eq!(
        block(s.inspect(id.clone(), true)).unwrap_err().code,
        "disconnected"
    );
    assert!(s.list().unwrap().devices.is_empty());
    m.0.lock().unwrap().keys.push("private-pnp-id".into());
    let next = first(&s);
    assert_ne!(next.id, id);
    assert_eq!(next.access, Access::Unchecked);
    assert!(s.selection(&id).is_err());
}
#[test]
fn unplug_during_native_open_is_reported() {
    let mut f = fixture();
    f.failure = Some("disconnected");
    let (s, _) = setup(f);
    let id = first(&s).id;
    assert_eq!(
        block(s.inspect(id, true)).unwrap().access,
        Access::Disconnected
    );
}
#[test]
fn refresh_does_not_reuse_permission() {
    let (s, _) = setup(fixture());
    let id = first(&s).id;
    block(s.inspect(id.clone(), false)).unwrap();
    let d = first(&s);
    assert_eq!(d.id, id);
    assert_eq!(d.access, Access::Unchecked);
}
#[test]
fn restart_never_claims_historical_connection() {
    let (s, _) = setup(fixture());
    let id = first(&s).id;
    let (s2, _) = setup(fixture());
    assert!(s2.list().unwrap().devices.is_empty());
    assert_ne!(first(&s2).id, id);
}
#[test]
fn invalid_handle_never_reaches_adapter() {
    let (s, m) = setup(fixture());
    first(&s);
    assert_eq!(
        block(s.inspect("C:\\arbitrary".into(), true))
            .unwrap_err()
            .code,
        "invalid_handle"
    );
    assert_eq!(m.1.load(Ordering::Relaxed), 0);
}
#[test]
fn stale_scan_cannot_claim_live_connection() {
    let (s, _) = setup(fixture());
    first(&s);
    s.0.cache.lock().unwrap().scanned_at = Some(0);
    assert!(s.list().unwrap().devices.is_empty());
}
#[test]
fn budgets_and_cancel_are_bounded() {
    let cancel = Arc::new(AtomicBool::new(true));
    let mut budget = Budget::new(cancel);
    assert_eq!(budget.check().unwrap_err().code, "cancelled");
    budget.cancel.store(false, Ordering::Release);
    budget.examined = adapter::MAX_ENTRIES;
    assert_eq!(budget.check().unwrap_err().code, "limit");
}
#[test]
fn busy_and_cancellation_do_not_spawn_more_workers() {
    let (s, _) = setup(fixture());
    *s.0.active.lock().unwrap() = Some(Arc::new(AtomicBool::new(false)));
    assert_eq!(block(s.refresh()).unwrap_err().code, "busy");
    s.cancel().unwrap();
    assert!(
        s.0.active
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .load(Ordering::Acquire)
    );
}
#[test]
fn entry_limit_is_incomplete() {
    let mut f = fixture();
    f.tree.insert(
        "s".into(),
        (0..257)
            .map(|i| object(&format!("id{i}"), "x", true, false))
            .collect(),
    );
    let mut budget = Budget::new(Arc::new(AtomicBool::new(false)));
    let report = discovery::discover(&f, &mut budget);
    assert_eq!(report.status, "incomplete");
    assert!(report.findings.iter().any(|f| f.status == "limit"));
}
#[test]
fn migration_four_preserves_legacy_records_and_notes() {
    let mut c = rusqlite::Connection::open_in_memory().unwrap();
    crate::storage::migrations::apply(&mut c, &crate::storage::migrations::MIGRATIONS[..3])
        .unwrap();
    c.execute_batch("INSERT INTO devices(id,display_name,platform) VALUES('legacy','Old phone','android'); INSERT INTO archives(id,title,source_type,storage_path) VALUES('a','Saved archive','chat-export','archives/a'); INSERT INTO conversations(id,archive_id,title) VALUES('c','a','Saved'); INSERT INTO messages(id,conversation_id,message_text) VALUES('m','c','Preserved');").unwrap();
    crate::storage::migrations::migrate(&mut c).unwrap();
    crate::storage::migrations::migrate(&mut c).unwrap();
    let (s, _) = setup(fixture());
    let d = first(&s);
    assert!(repository::known(&c).unwrap().is_empty());
    let n = repository::save(&c, &d, true).unwrap();
    assert!(n.preferred);
    assert_eq!(repository::known(&c).unwrap().len(), 1);
    for table in ["devices", "archives", "conversations", "messages"] {
        let count: i64 = c
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version, crate::storage::migrations::SCHEMA_VERSION);
    let columns: String = c
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='portable_device_notes'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!columns.contains("serial"));
    assert!(!columns.contains("hardware"));
}
#[test]
#[ignore = "manual native Windows WPD enumeration; no mock, no writes, no hardware identifiers printed"]
fn native_wpd_enumeration() {
    let state = DeviceState::default();
    let list = block(state.refresh()).unwrap();
    println!(
        "Real WPD enumeration: {} portable devices; scan timestamp present: {}",
        list.devices.len(),
        list.scanned_at.is_some()
    );
}

#[test]
fn business_folder_conventions_are_separate_candidates() {
    let mut f = fixture();
    f.tree.insert(
        "m".into(),
        vec![object("p", "com.whatsapp.w4b", true, false)],
    );
    f.tree.insert(
        "p".into(),
        vec![object("w", "WhatsApp Business", true, false)],
    );
    f.tree.insert(
        "media".into(),
        vec![object("images", "WhatsApp Business Images", true, false)],
    );
    let mut budget = Budget::new(Arc::new(AtomicBool::new(false)));
    let report = discovery::discover(&f, &mut budget);
    assert_eq!(report.status, "complete");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.application == "WhatsApp Business"
                && f.category == "images"
                && f.status == "accessible")
    );
}

#[test]
fn consented_notes_survive_database_restart_without_connection_state() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("notes.sqlite3");
    let mut c = rusqlite::Connection::open(&path).unwrap();
    crate::storage::migrations::migrate(&mut c).unwrap();
    let (state, _) = setup(fixture());
    let device = first(&state);
    let saved = repository::save(&c, &device, true).unwrap();
    drop(c);
    let mut c = rusqlite::Connection::open(path).unwrap();
    crate::storage::migrations::migrate(&mut c).unwrap();
    let notes = repository::known(&c).unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id, saved.id);
    assert!(notes[0].preferred);
    let json = serde_json::to_string(&notes).unwrap();
    assert!(!json.contains("private-pnp"));
    assert!(!json.contains("connected\""));
    let (restarted, _) = setup(fixture());
    assert!(restarted.list().unwrap().devices.is_empty());
}

#[test]
fn cooperative_worker_cancellation_discards_result_and_releases_slot() {
    struct Slow(std::sync::mpsc::SyncSender<()>);
    impl DeviceAdapter for Slow {
        fn enumerate(&self, budget: &mut Budget) -> Result<Vec<RawDevice>> {
            self.0.send(()).unwrap();
            loop {
                budget.check()?;
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        fn open(&self, _: &str, _: &mut Budget) -> Result<Box<dyn Content>> {
            panic!("not used")
        }
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let state = DeviceState::new(Arc::new(Slow(sender)));
    let worker = state.clone();
    let join = std::thread::spawn(move || block(worker.refresh()));
    receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(block(state.refresh()).unwrap_err().code, "busy");
    state.cancel().unwrap();
    assert_eq!(join.join().unwrap().unwrap_err().code, "cancelled");
    assert!(state.0.active.lock().unwrap().is_none());
    assert!(state.list().unwrap().devices.is_empty());
}
