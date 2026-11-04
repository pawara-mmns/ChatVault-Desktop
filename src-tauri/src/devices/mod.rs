pub(crate) mod adapter;
pub mod commands;
mod discovery;
pub(crate) mod models;
pub(crate) mod repository;
pub(crate) mod scan;
#[cfg(test)]
mod tests;
#[cfg(windows)]
mod windows;

use adapter::{Budget, DeviceAdapter, RawDevice, WindowsPortableDeviceAdapter};
use models::*;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

struct Entry {
    key: String,
    device: Device,
}
#[derive(Default)]
struct Cache {
    entries: HashMap<String, Entry>,
    scanned_at: Option<u64>,
}
struct Inner {
    adapter: Arc<dyn DeviceAdapter>,
    cache: Mutex<Cache>,
    active: Mutex<Option<Arc<AtomicBool>>>,
    backup_active: AtomicBool,
}
#[derive(Clone)]
pub struct DeviceState(Arc<Inner>);
impl Default for DeviceState {
    fn default() -> Self {
        Self::new(Arc::new(WindowsPortableDeviceAdapter))
    }
}
impl DeviceState {
    pub(crate) fn new(adapter: Arc<dyn DeviceAdapter>) -> Self {
        Self(Arc::new(Inner {
            adapter,
            cache: Mutex::new(Cache::default()),
            active: Mutex::new(None),
            backup_active: AtomicBool::new(false),
        }))
    }
    pub fn list(&self) -> Result<DeviceList> {
        let cache = self.0.cache.lock().map_err(|_| error("failed"))?;
        let stale = cache
            .scanned_at
            .is_none_or(|time| now().saturating_sub(time) > 30_000);
        let busy = self.0.active.lock().map_err(|_| error("failed"))?.is_some();
        let mut devices: Vec<_> = if stale {
            vec![]
        } else {
            cache.entries.values().map(|e| e.device.clone()).collect()
        };
        devices.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        Ok(DeviceList {
            devices,
            scanned_at: cache.scanned_at,
            stale,
            busy,
        })
    }
    fn selection(&self, id: &str) -> Result<(String, Device)> {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err(error("invalid_handle"));
        }
        let cache = self.0.cache.lock().map_err(|_| error("failed"))?;
        if cache
            .scanned_at
            .is_none_or(|t| now().saturating_sub(t) > 30_000)
        {
            return Err(error("invalid_handle"));
        }
        cache
            .entries
            .get(id)
            .map(|e| (e.key.clone(), e.device.clone()))
            .ok_or_else(|| error("disconnected"))
    }
    pub fn cancel(&self) -> Result<()> {
        let active = self.0.active.lock().map_err(|_| error("failed"))?;
        if self.0.backup_active.load(Ordering::Acquire) {
            return Err(error("busy"));
        }
        if let Some(cancel) = active.as_ref() {
            cancel.store(true, Ordering::Release);
        }
        Ok(())
    }
    // At most ONE native worker, including after a response timeout. A misbehaving driver
    // cannot cause an unbounded accumulation of threads. Never share COM interfaces.
    async fn operation<T: Send + 'static>(
        &self,
        job: impl FnOnce(DeviceState, &mut Budget) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.0.active.lock().map_err(|_| error("failed"))?;
            if active.is_some() {
                return Err(error("busy"));
            }
            *active = Some(cancel.clone());
        }
        let state = self.clone();
        let thread_cancel = cancel.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let spawned = std::thread::Builder::new()
            .name("chatvault-wpd".into())
            .spawn(move || {
                struct Finish(DeviceState);
                impl Drop for Finish {
                    fn drop(&mut self) {
                        if let Ok(mut active) = self.0.0.active.lock() {
                            *active = None;
                        }
                    }
                }
                let _finish = Finish(state.clone());
                let mut budget = Budget::new(thread_cancel);
                let result = job(state, &mut budget).and_then(|value| {
                    budget.check()?;
                    Ok(value)
                });
                // Release the single-operation slot before delivering completion.
                // A caller may immediately start its next explicit operation.
                drop(_finish);
                let _ = sender.send(result);
            });
        if spawned.is_err() {
            *self.0.active.lock().map_err(|_| error("failed"))? = None;
            return Err(error("failed"));
        }
        tauri::async_runtime::spawn_blocking(move || {
            match receiver.recv_timeout(Duration::from_secs(28)) {
                Ok(result) => result,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    cancel.store(true, Ordering::Release);
                    Err(error("timeout"))
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(error("failed")),
            }
        })
        .await
        .map_err(|_| error("failed"))?
    }
    pub async fn refresh(&self) -> Result<DeviceList> {
        if self.0.backup_active.load(Ordering::Acquire) {
            return self.list();
        }
        self.operation(|state, budget| {
            let result = state.0.adapter.enumerate(budget);
            if result.is_err() {
                let mut cache = state.0.cache.lock().map_err(|_| error("failed"))?;
                cache.entries.clear();
                cache.scanned_at = None;
            }
            let raw = result?;
            budget.check()?;
            state.reconcile(raw)?;
            state.list().map(|mut list| {
                list.busy = false;
                list
            })
        })
        .await
    }
    fn reconcile(&self, raw: Vec<RawDevice>) -> Result<()> {
        if raw.len() > adapter::MAX_DEVICES {
            return Err(error("limit"));
        }
        let mut cache = self.0.cache.lock().map_err(|_| error("failed"))?;
        let mut prior = std::mem::take(&mut cache.entries);
        let mut seen = HashSet::new();
        let detected = now();
        for raw in raw {
            if raw.key.is_empty() || raw.key.len() > 16384 || !seen.insert(raw.key.clone()) {
                continue;
            }
            let old_id = prior
                .iter()
                .find(|(_, e)| e.key == raw.key)
                .map(|(id, _)| id.clone());
            let old = old_id.and_then(|id| prior.remove(&id));
            let mut device = old.map(|e| e.device).unwrap_or_else(|| Device {
                id: uuid::Uuid::new_v4().to_string(),
                name: None,
                manufacturer: None,
                category: None,
                android_identity: "unknown",
                transport: "unavailable",
                access: Access::Unchecked,
                detected_at: detected,
                checked_at: None,
                diagnostic: None,
                discovery: None,
                saved_note_id: None,
            });
            device.name = raw.name.as_deref().and_then(clean);
            device.manufacturer = raw.manufacturer.as_deref().and_then(clean);
            device.transport = if raw.usb { "usb" } else { "unavailable" };
            device.detected_at = detected;
            // Enumeration does not imply current file permission, even with the same PnP ID.
            device.access = if raw.usb {
                Access::Unchecked
            } else {
                Access::Unsupported
            };
            device.checked_at = None;
            device.diagnostic = None;
            cache.entries.insert(
                device.id.clone(),
                Entry {
                    key: raw.key,
                    device,
                },
            );
        }
        cache.scanned_at = Some(detected);
        Ok(())
    }
    pub async fn inspect(&self, id: String, discover: bool) -> Result<Device> {
        let (key, mut device) = self.selection(&id)?;
        if discover {
            device.discovery = None;
        }
        self.operation(move |state, budget| {
            // Re-enumerate before every explicit access; stale application handles cannot
            // address an arbitrary/new OS device. No frontend path reaches the adapter.
            let live = state.0.adapter.enumerate(budget)?;
            if !live.iter().any(|d| d.key == key) {
                state
                    .0
                    .cache
                    .lock()
                    .map_err(|_| error("failed"))?
                    .entries
                    .remove(&id);
                return Err(error("disconnected"));
            }
            let result = (|| -> Result<()> {
                if device.transport != "usb" {
                    return Err(error("unsupported"));
                }
                let content = state.0.adapter.open(&key, budget)?;
                let identity = content.identity();
                device.category = identity.category.as_deref().and_then(clean);
                // WPD has no universal Android-version/OS property. Only an explicit
                // driver protocol report is affirmative; brand/phone/MTP is insufficient.
                device.android_identity = if identity
                    .protocol
                    .as_deref()
                    .is_some_and(|s| s.eq_ignore_ascii_case("android"))
                {
                    "driver_reported_android"
                } else {
                    "unknown"
                };
                let roots = discovery::storage_roots(content.as_ref(), budget)?;
                if roots.is_empty() {
                    return Err(error("access_unavailable"));
                }
                // A storage object alone does not establish folder enumeration permission.
                let mut accessible = false;
                let mut restriction = error("access_unavailable");
                for root in &roots {
                    match content.probe(&root.id, budget) {
                        Ok(()) => {
                            accessible = true;
                            break;
                        }
                        Err(e) if matches!(e.code, "access_denied" | "failed") => restriction = e,
                        Err(e) => return Err(e),
                    }
                }
                if !accessible {
                    return Err(restriction);
                }
                device.access = Access::Available;
                if discover {
                    device.discovery = Some(discovery::discover(content.as_ref(), budget));
                }
                Ok(())
            })();
            device.checked_at = Some(now());
            device.diagnostic = result.err();
            if let Some(e) = &device.diagnostic {
                device.access = access_for(e.code);
            }
            if let Some(report) = &device.discovery {
                if report.status == "disconnected" {
                    device.access = Access::Disconnected;
                }
                if report
                    .diagnostic
                    .as_ref()
                    .is_some_and(|e| e.code == "access_denied")
                {
                    device.access = Access::PermissionRequired;
                }
            }
            budget.check()?;
            let mut cache = state.0.cache.lock().map_err(|_| error("failed"))?;
            if let Some(entry) = cache.entries.get_mut(&id) {
                entry.device = device.clone();
            }
            Ok(device)
        })
        .await
    }
}
// A backup owns the same single-device-operation slot as Step 05. COM interfaces
// are still created and dropped only inside the consuming backup worker.
pub(crate) struct DeviceSession {
    state: DeviceState,
    key: String,
    pub name: Option<String>,
}
impl DeviceState {
    pub(crate) fn acquire_backup(
        &self,
        id: &str,
        cancel: Arc<AtomicBool>,
    ) -> Result<DeviceSession> {
        let (key, device) = self.selection(id)?;
        if device.transport != "usb" {
            return Err(error("unsupported"));
        }
        let mut active = self.0.active.lock().map_err(|_| error("failed"))?;
        if active.is_some() {
            return Err(error("busy"));
        }
        *active = Some(cancel.clone());
        self.0.backup_active.store(true, Ordering::Release);
        Ok(DeviceSession {
            state: self.clone(),
            key,
            name: device.name,
        })
    }
}
impl DeviceSession {
    pub(crate) fn open(&self, budget: &mut Budget) -> Result<Box<dyn adapter::Content>> {
        let live = self.state.0.adapter.enumerate(budget)?;
        if !live.iter().any(|d| d.key == self.key && d.usb) {
            return Err(error("disconnected"));
        }
        self.state.0.adapter.open(&self.key, budget)
    }
}
impl Drop for DeviceSession {
    fn drop(&mut self) {
        if let Ok(mut active) = self.state.0.active.lock() {
            *active = None;
            self.state.0.backup_active.store(false, Ordering::Release);
        }
    }
}
fn access_for(code: &str) -> Access {
    match code {
        "access_denied" => Access::PermissionRequired,
        "disconnected" => Access::Disconnected,
        "unsupported" => Access::Unsupported,
        "access_unavailable" => Access::Unavailable,
        _ => Access::Failed,
    }
}
