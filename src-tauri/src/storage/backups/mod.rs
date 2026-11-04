pub mod commands;
mod files;
pub mod models;
mod planner;
mod repository;
#[cfg(test)]
mod scan_tests;
#[cfg(test)]
mod tests;
mod transfer;
use crate::{
    devices::{DeviceState, adapter::Budget},
    storage::{StorageState, service::StorageService},
};
use models::*;
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::Emitter;
pub struct Context {
    root: PathBuf,
    installation: String,
    _lease: fs::File,
}
#[derive(Clone, Default)]
pub struct BackupState(Arc<Mutex<Runtime>>);
#[derive(Default)]
struct Runtime {
    plan: Option<planner::Plan>,
    #[cfg(test)]
    observations: Vec<Progress>,
    progress: Progress,
    cancel: Option<Arc<AtomicBool>>,
    operation_id: Option<String>,
    revision: u64,
    scan: Option<crate::devices::scan::ScanTrace>,
}
struct Active {
    state: BackupState,
    cancel: Arc<AtomicBool>,
    id: String,
}
impl Drop for Active {
    fn drop(&mut self) {
        if let Ok(mut r) = self.state.0.lock() {
            r.cancel = None;
        }
    }
}
pub(crate) fn db<T>(
    storage: &StorageState,
    f: impl FnOnce(&mut StorageService) -> Result<T>,
) -> Result<T> {
    storage.run(false, |s| Ok(f(s))).map_err(Failure::from)?
}
pub fn validate_id(id: &str) -> Result<()> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| fail("not_found"))
}
impl BackupState {
    fn begin(&self) -> Result<Active> {
        let mut r = self.0.lock().map_err(|_| fail("failed"))?;
        if r.cancel.is_some() {
            return Err(fail("busy"));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        r.cancel = Some(cancel.clone());
        let id = uuid::Uuid::new_v4().to_string();
        r.operation_id = Some(id.clone());
        r.scan = None;
        Ok(Active {
            state: self.clone(),
            cancel,
            id,
        })
    }
    pub fn progress(&self) -> Result<Progress> {
        let r = self.0.lock().map_err(|_| fail("failed"))?;
        let mut p = r.progress.clone();
        if r.cancel.is_some() {
            if let Some(trace) = &r.scan {
                p.scan = Some(trace.snapshot());
            }
            p.cancel_requested |= r.cancel.as_ref().is_some_and(|c| c.load(Ordering::Acquire));
        }
        Ok(p)
    }
    pub fn cancel(&self) -> Result<()> {
        let mut r = self.0.lock().map_err(|_| fail("failed"))?;
        if let Some(cancel) = &r.cancel {
            cancel.store(true, Ordering::Release);
            r.progress.cancel_requested = true;
        }
        Ok(())
    }
    fn publish(&self, p: &Progress, app: Option<&tauri::AppHandle>) {
        if let Ok(mut r) = self.0.lock() {
            if p.operation_id.is_some() && p.operation_id != r.operation_id {
                return;
            }
            if p.operation_id == r.progress.operation_id
                && matches!(p.phase.as_str(), "scanning" | "scan_waiting")
                && !matches!(r.progress.phase.as_str(), "scanning" | "scan_waiting")
            {
                return;
            }
            let mut p = p.clone();
            r.revision += 1;
            p.revision = r.revision;
            p.cancel_requested |= r.cancel.as_ref().is_some_and(|c| c.load(Ordering::Acquire));
            p.updated_at = crate::devices::models::now();
            #[cfg(test)]
            r.observations.push(p.clone());
            r.progress = p.clone();
            if let Some(app) = app {
                let _ = app.emit_to("main", "backup-progress", p);
            }
        }
    }
    #[cfg(test)]
    pub async fn prepare(
        &self,
        storage: StorageState,
        devices: DeviceState,
        device_id: String,
        source: Source,
        app: Option<tauri::AppHandle>,
    ) -> Result<PlanSummary> {
        self.prepare_with_options(
            storage,
            devices,
            device_id,
            source,
            ScanOptions::default(),
            app,
        )
        .await
    }
    pub async fn prepare_with_options(
        &self,
        storage: StorageState,
        devices: DeviceState,
        device_id: String,
        source: Source,
        options: ScanOptions,
        app: Option<tauri::AppHandle>,
    ) -> Result<PlanSummary> {
        options.validate()?;
        let active = self.begin()?;
        let trace = crate::devices::scan::ScanTrace::new(
            options.timeout_seconds,
            options.max_objects,
            options.max_depth,
        );
        {
            let mut r = self.0.lock().map_err(|_| fail("failed"))?;
            r.plan = None;
            r.scan = Some(trace.clone());
        }
        let initial = Progress {
            operation_id: Some(active.id.clone()),
            phase: "scanning".into(),
            source: Some(source),
            scan: Some(trace.snapshot()),
            ..Default::default()
        };
        self.publish(&initial, app.as_ref());
        log_scan(&initial);
        let state = self.clone();
        let monitor = self.clone();
        let monitor_trace = trace.clone();
        let monitor_app = app.clone();
        let abandoned = Arc::new(AtomicBool::new(false));
        let worker_abandoned = abandoned.clone();
        let worker_initial = initial.clone();
        let worker_options = options.clone();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("chatvault-scan".into())
            .spawn(move || {
                let _active = active;
                let mut p = worker_initial;
                let result: Result<planner::Plan> = (|| {
                    let context = db(&storage, |s| s.backup_context())?;
                    let session = devices.acquire_backup(&device_id, _active.cancel.clone())?;
                    let mut budget = Budget::scan(
                        _active.cancel.clone(),
                        worker_options.timeout_seconds,
                        worker_options.max_objects,
                        trace.clone(),
                    );
                    let content = session.open(&mut budget)?;
                    let plan = planner::inventory_with_options(
                        content.as_ref(),
                        device_id,
                        session.name.clone(),
                        source,
                        context.root.to_string_lossy().into_owned(),
                        files::free_bytes(&context.root),
                        &mut budget,
                        &worker_options,
                    )?;
                    drop(content); // Close and release the session on this worker before publishing Ready.
                    Ok(plan)
                })();
                let result = if worker_abandoned.load(Ordering::Acquire) {
                    Err(fail("timeout"))
                } else {
                    result
                };
                let reply = match result {
                    Ok(plan) if !_active.cancel.load(Ordering::Acquire) => {
                        p.phase = "ready".into();
                        p.total_files = plan.summary.total_files;
                        p.known_total_bytes = plan.summary.known_bytes;
                        p.unknown_sizes = plan.summary.unknown_sizes;
                        let summary = plan.summary.clone();
                        if let Ok(mut r) = state.0.lock() {
                            r.plan = Some(plan);
                        }
                        Ok(summary)
                    }
                    result => {
                        let e = result.err().unwrap_or_else(|| fail("cancelled"));
                        p.phase = if e.code == "cancelled" {
                            "scan_cancelled"
                        } else {
                            "scan_failed"
                        }
                        .into();
                        p.error = Some(e.clone());
                        Err(e)
                    }
                };
                p.scan = Some(trace.snapshot());
                state.publish(&p, app.as_ref());
                log_scan(&p);
                drop(_active);
                let _ = tx.send(reply);
            })
            .map_err(|_| fail("failed"))?;
        tauri::async_runtime::spawn_blocking(move || {
            let mut log_at = Instant::now();
            loop {
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Ok(reply) => return reply,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        return Err(fail("failed"));
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        let scan = monitor_trace.snapshot();
                        let p = monitor.progress()?;
                        if p.operation_id != initial.operation_id
                            || !matches!(p.phase.as_str(), "scanning" | "scan_waiting")
                        {
                            continue;
                        }
                        let mut p = p;
                        p.scan = Some(scan.clone());
                        if scan.elapsed_ms >= (options.timeout_seconds + 1) * 1000 {
                            // Do not release the Active guard or COM objects while a vendor call is alive.
                            abandoned.store(true, Ordering::Release);
                            monitor.cancel()?;
                            p.phase = "scan_waiting".into();
                            p.cancel_requested = true;
                            p.error = Some(fail("driver_pending"));
                            monitor.publish(&p, monitor_app.as_ref());
                            log_scan(&p);
                            return Err(fail("driver_pending"));
                        }
                        monitor.publish(&p, monitor_app.as_ref());
                        if log_at.elapsed() >= Duration::from_secs(10) {
                            log_scan(&p);
                            log_at = Instant::now();
                        }
                    }
                }
            }
        })
        .await
        .map_err(|_| fail("failed"))?
    }
    pub fn plan_files(
        &self,
        id: &str,
        category: Option<Category>,
        offset: usize,
    ) -> Result<Vec<FileEntry>> {
        let r = self.0.lock().map_err(|_| fail("failed"))?;
        let plan = r
            .plan
            .as_ref()
            .filter(|p| p.summary.id == id)
            .ok_or_else(|| fail("plan_expired"))?;
        Ok(plan
            .files
            .iter()
            .filter(|f| category.is_none_or(|c| c == f.entry.category))
            .skip(offset.min(50_000))
            .take(50)
            .map(|f| f.entry.clone())
            .collect())
    }
    pub async fn start(
        &self,
        storage: StorageState,
        devices: DeviceState,
        plan_id: String,
        selection: Selection,
        app: Option<tauri::AppHandle>,
    ) -> Result<Progress> {
        let active = self.begin()?;
        devices.refresh().await?;
        let state = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let plan = state.0.lock().map_err(|_| fail("failed"))?.plan.take()
                .filter(|p| p.summary.id == plan_id && crate::devices::models::now().saturating_sub(p.created) <= 600_000)
                .ok_or_else(|| fail("plan_expired"))?;
            let selected = planner::select(&plan, &selection)?;
            let context = db(&storage, |s| s.backup_context())?;
            if context.root.to_string_lossy() != plan.summary.vault_path {
                return Err(fail("plan_expired"));
            }
            let known: u64 = selected.iter().map(|f| f.entry.expected_size.unwrap_or(0)).sum();
            let unknown = selected.iter().filter(|f| f.entry.expected_size.is_none()).count();
            let available = files::free_bytes(&context.root);
            if available.is_some_and(|n| n < known.saturating_add(16 * 1024 * 1024)) {
                return Err(fail("insufficient_space"));
            }
            if (available.is_none() || unknown > 0) && !selection.acknowledge_capacity {
                return Err(fail("capacity_unknown"));
            }
            let session = devices.acquire_backup(&plan.summary.device_id, active.cancel.clone())?;
            let id = uuid::Uuid::new_v4().to_string();
            let mut warnings = plan.summary.warnings.clone();
            if unknown > 0 { warnings.push("unknown_source_sizes".into()); }
            if available.is_none() { warnings.push("capacity_unknown".into()); }
            if !selection.source_reread { warnings.push("source_not_independently_reread".into()); }
            db(&storage, |s| {
                repository::start(&mut s.connection, &id, plan.summary.source, plan.summary.device_name.as_deref(), &selected, &warnings)?;
                s.connection.execute("UPDATE backup_snapshots SET skipped_count=?2 WHERE id=?1", rusqlite::params![id, plan.summary.skipped_entries as i64])?;
                Ok(())
            })?;
            let initial = Progress {
                operation_id: Some(active.id.clone()),
                snapshot_id: Some(id.clone()), phase: "preparing".into(), source: Some(plan.summary.source),
                total_files: selected.len(), known_total_bytes: known, unknown_sizes: unknown,
                ..Default::default()
            };
            state.publish(&initial, app.as_ref());
            let p = initial.clone();
            let thread_state = state.clone();
            let source = plan.summary.source;
            let name = plan.summary.device_name;
            let complete = plan.summary.complete;
            let skipped = plan.summary.skipped_entries;
            let inaccessible = plan.summary.inaccessible_locations;
            let scanned_categories = plan.summary.scanned_categories;
            let worker_storage = storage.clone();
            let spawn = std::thread::Builder::new().name("chatvault-backup".into()).spawn(move || {
                let storage = worker_storage;
                let _active = active;
                let _session = session;
                let mut budget = Budget::backup(_active.cancel.clone(), 7200);
                let mut p = p;
                let started = Instant::now();
                let mut last = Instant::now() - Duration::from_secs(1);
                let mut persisted = Instant::now();
                let mut update = |progress: &Progress| {
                    if persisted.elapsed() >= Duration::from_secs(2) {
                        let _ = db(&storage, |s| repository::progress(&s.connection, progress));
                        persisted = Instant::now();
                    }
                    if last.elapsed() >= Duration::from_millis(250) {
                        let mut value = progress.clone();
                        value.rate_bytes_per_second = (started.elapsed().as_secs_f64() > 0.5)
                            .then(|| (value.copied_bytes as f64 / started.elapsed().as_secs_f64()) as u64);
                        thread_state.publish(&value, app.as_ref());
                        last = Instant::now();
                    }
                };
                let stage = context.root.join("temp").join(format!("backup-{id}"));
                let outcome = (|| -> Result<()> {
                    crate::storage::paths::verify_ancestors(&context.root.join("temp"), false)?;
                    fs::create_dir(&stage).map_err(io)?;
                    files::owner(&stage, &id, &context.installation)?;
                    let content = _session.open(&mut budget)?;
                    let mut records = vec![];
                    for file in &selected {
                        budget.check()?;
                        let verified_before = p.verified_bytes;
                        match transfer::copy(content.as_ref(), file, &stage, &mut budget, selection.source_reread, &mut p, &mut update) {
                            Ok(entry) => {
                                db(&storage, |s| repository::file(&s.connection, &id, &entry))?;
                                records.push(entry);
                                p.successful_files += 1;
                            }
                            Err(e) => {
                                p.verified_bytes = verified_before;
                                p.error = Some(e.clone());
                                let mut entry = file.entry.clone();
                                entry.state = "failed".into();
                                entry.error_code = Some(e.code.clone());
                                entry.local_verification = "failed".into();
                                db(&storage, |s| repository::file(&s.connection, &id, &entry))?;
                                p.failed_files += 1;
                                if matches!(e.code.as_str(), "disconnected" | "cancelled" | "storage_full" | "vault_unavailable" | "timeout") {
                                    return Err(e);
                                }
                            }
                        }
                        p.updated_at = crate::devices::models::now();
                        db(&storage, |s| repository::progress(&s.connection, &p))?;
                        thread_state.publish(&p, app.as_ref());
                    }
                    if p.failed_files > 0 || records.len() != selected.len() {
                        return Err(p.error.clone().unwrap_or_else(|| fail("required_files_failed")));
                    }
                    budget.check()?;
                    p.phase = "committing".into();
                    p.updated_at = crate::devices::models::now();
                    db(&storage, |s| repository::progress(&s.connection, &p))?;
                    thread_state.publish(&p, app.as_ref());
                    let phase = if warnings.is_empty() { "completed" } else { "completed_with_warnings" };
                    let manifest = Manifest {
                        scanned_categories: scanned_categories.clone(),
                        skipped_entries: skipped, inaccessible_locations: inaccessible, schema_version: 1,
                        snapshot_id: id.clone(), created_at: p.updated_at, source, device_display_name: name.clone(),
                        device_association: None, backup_mode: "read_only_wpd_full_selection".into(),
                        phase: phase.into(), inventory_complete: complete, warnings: warnings.clone(),
                        source_consistency: "Live files are not an atomic WhatsApp snapshot; same-size changes may escape metadata checks. Source reread results apply only to the recorded read interval.".into(),
                        files: records,
                    };
                    files::manifest(&stage, &manifest)?;
                    files::verify_owner(&stage, &id, &context.installation)?;
                    files::validate_tree(&stage)?;
                    crate::storage::paths::verify_ancestors(&context.root.join("backups"), false)?;
                    files::rename_new(&stage, &context.root.join("backups").join(&id))?;
                    p.phase = phase.into();
                    Ok(())
                })();
                if let Err(e) = outcome {
                    p.phase = if e.code == "cancelled" { "cancelled" } else { "failed" }.into();
                    if let Some(diagnostic) = &e.diagnostic { warnings.push(diagnostic.clone()); }
                    p.error = Some(e);
                    if files::verify_owner(&stage, &id, &context.installation).is_ok() && !stage.join("manifest.json").exists() {
                        if let Ok(records) = db(&storage, |s| repository::entries(&s.connection, &id, 0, 50_001)) {
                            let manifest = Manifest {
                        scanned_categories: scanned_categories.clone(),
                                skipped_entries: skipped, inaccessible_locations: inaccessible, schema_version: 1,
                                snapshot_id: id.clone(), created_at: crate::devices::models::now(), source,
                                device_display_name: name.clone(), device_association: None,
                                backup_mode: "read_only_wpd_full_selection".into(), phase: p.phase.clone(),
                                inventory_complete: complete, warnings: warnings.clone(),
                                source_consistency: "Incomplete selection; never a completed or atomic WhatsApp backup.".into(), files: records,
                            };
                            let _ = files::manifest(&stage, &manifest);
                        }
                    }
                }
                p.current_filename = None;
                p.updated_at = crate::devices::models::now();
                if db(&storage, |s| repository::terminal(&mut s.connection, &p, &warnings)).is_err() {
                    p.phase = "interrupted".into();
                    p.error = Some(fail("database"));
                }
                thread_state.publish(&p, app.as_ref());
            });
            if spawn.is_err() {
                let mut failed = initial.clone();
                failed.phase = "failed".into();
                failed.error = Some(fail("failed"));
                db(&storage, |s| repository::terminal(&mut s.connection, &failed, &[]))?;
                state.publish(&failed, None);
                return Err(fail("failed"));
            }
            Ok(initial)
        }).await.map_err(|_| fail("failed"))?
    }

    pub async fn reverify(
        &self,
        storage: StorageState,
        id: String,
        app: Option<tauri::AppHandle>,
    ) -> Result<Progress> {
        validate_id(&id)?;
        let active = self.begin()?;
        let state = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _active = active;
            let context = db(&storage, |s| s.backup_context())?;
            let root = context.root.join("backups").join(&id);
            files::verify_owner(&root, &id, &context.installation)?;
            let manifest = files::read_manifest(&root)?;
            if manifest.snapshot_id != id || manifest.schema_version != 1
                || !matches!(manifest.phase.as_str(), "completed" | "completed_with_warnings") || manifest.files.is_empty() {
                return Err(fail("unsafe_path"));
            }
            let entries = db(&storage, |s| repository::entries(&s.connection, &id, 0, 50_001))?;
            if entries.len() != manifest.files.len() || entries.len() > planner::MAX_FILES
                || !entries.iter().zip(&manifest.files).all(|(a,b)| a.id == b.id && a.sha256 == b.sha256 && a.local_relative == b.local_relative && a.actual_bytes == b.actual_bytes) {
                return Err(fail("unsafe_path"));
            }
            let mut p = Progress { operation_id: Some(_active.id.clone()), snapshot_id: Some(id.clone()), phase: "verifying_local".into(), total_files: entries.len(), ..Default::default() };
            let budget = Budget::backup(_active.cancel.clone(), 7200);
            state.publish(&p, app.as_ref());
            db(&storage, |s| {
                s.connection.execute("UPDATE backup_snapshots SET local_integrity='verifying' WHERE id=?1", [&id])?;
                Ok(())
            })?;
            let mut last = Instant::now();
            let result = (|| -> Result<()> {
                for entry in &entries {
                    p.current_filename = Some(entry.original_name.clone());
                    let before = p.verified_bytes;
                    let (bytes, hash) = files::hash_local(&files::join(&root, &entry.local_relative)?, &budget, |bytes| {
                        p.verified_bytes = before + bytes;
                        if last.elapsed() > Duration::from_millis(250) {
                            state.publish(&p, app.as_ref());
                            last = Instant::now();
                        }
                    })?;
                    let valid = Some(hash) == entry.sha256 && bytes == entry.actual_bytes;
                    db(&storage, |s| {
                        s.connection.execute("UPDATE backup_files SET local_verification=?3 WHERE id=?1 AND snapshot_id=?2", rusqlite::params![entry.id, id, if valid { "verified" } else { "failed" }])?;
                        Ok(())
                    })?;
                    if !valid { return Err(fail("hash_mismatch")); }
                    p.successful_files += 1;
                }
                Ok(())
            })();
            let integrity = match &result {
                Ok(()) => "verified",
                Err(e) if e.code == "cancelled" => "verification_cancelled",
                Err(_) => "failed",
            };
            db(&storage, |s| {
                s.connection.execute("UPDATE backup_snapshots SET local_integrity=?2,local_verified_at=?3 WHERE id=?1", rusqlite::params![id, integrity, crate::devices::models::now() as i64])?;
                Ok(())
            })?;
            p.phase = if result.is_ok() { "verification_completed" }
                else if integrity == "verification_cancelled" { "cancelled" } else { "verification_failed" }.into();
            p.error = result.err();
            p.current_filename = None;
            state.publish(&p, app.as_ref());
            Ok(p)
        }).await.map_err(|_| fail("failed"))?
    }
}

fn log_scan(p: &Progress) {
    // Only code-defined phase labels, counters, duration and numeric HRESULTs.
    if let Ok(value) = serde_json::to_string(
        &serde_json::json!({"event":"backup_scan", "phase":p.phase, "operationId":p.operation_id, "revision":p.revision, "source":p.source, "errorCode":p.error.as_ref().map(|e|e.code.as_str()), "diagnostic":p.error.as_ref().and_then(|e|e.diagnostic.as_deref()), "scan":p.scan}),
    ) {
        eprintln!("{value}");
    }
}
