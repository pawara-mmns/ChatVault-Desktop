use super::{models::*, *};
use tauri::{AppHandle, State};
#[tauri::command]
pub async fn prepare_backup(
    state: State<'_, BackupState>,
    storage: State<'_, StorageState>,
    devices: State<'_, DeviceState>,
    device_id: String,
    source: Source,
    options: Option<ScanOptions>,
    app: AppHandle,
) -> Result<PlanSummary> {
    state
        .prepare_with_options(
            storage.inner().clone(),
            devices.inner().clone(),
            device_id,
            source,
            options.unwrap_or_default(),
            Some(app),
        )
        .await
}
#[tauri::command]
pub fn get_backup_plan_files(
    state: State<'_, BackupState>,
    plan_id: String,
    category: Option<Category>,
    offset: usize,
) -> Result<Vec<FileEntry>> {
    state.plan_files(&plan_id, category, offset)
}
#[tauri::command]
pub async fn start_backup(
    state: State<'_, BackupState>,
    storage: State<'_, StorageState>,
    devices: State<'_, DeviceState>,
    plan_id: String,
    selection: Selection,
    app: AppHandle,
) -> Result<Progress> {
    state
        .start(
            storage.inner().clone(),
            devices.inner().clone(),
            plan_id,
            selection,
            Some(app),
        )
        .await
}
#[tauri::command]
pub fn get_backup_progress(state: State<'_, BackupState>) -> Result<Progress> {
    state.progress()
}
#[tauri::command]
pub fn cancel_backup(state: State<'_, BackupState>) -> Result<()> {
    state.cancel()
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    items: Vec<Snapshot>,
    total: u64,
    completed_count: u64,
    completed_bytes: u64,
}
#[tauri::command]
pub async fn list_backup_snapshots(
    storage: State<'_, StorageState>,
    offset: usize,
) -> Result<History> {
    let storage = storage.inner().clone();
    tauri::async_runtime::spawn_blocking(move||db(&storage,|s|{let(root,_)=s.read_vault()?;let items=repository::list(&s.connection,&root,offset,20)?;let(total,count,bytes):(i64,i64,i64)=s.connection.query_row("SELECT count(*),coalesce(sum(status='complete' AND local_integrity!='failed'),0),coalesce(sum(CASE WHEN status='complete' AND local_integrity!='failed' THEN total_bytes ELSE 0 END),0) FROM backup_snapshots",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;Ok(History{items,total:total as u64,completed_count:count as u64,completed_bytes:bytes as u64})})).await.map_err(|_|fail("failed"))?
}
#[tauri::command]
pub async fn get_backup_files(
    storage: State<'_, StorageState>,
    snapshot_id: String,
    offset: usize,
) -> Result<Vec<FileEntry>> {
    validate_id(&snapshot_id)?;
    let storage = storage.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        db(&storage, |s| {
            repository::entries(&s.connection, &snapshot_id, offset, 50)
        })
    })
    .await
    .map_err(|_| fail("failed"))?
}
#[tauri::command]
pub async fn reverify_backup(
    state: State<'_, BackupState>,
    storage: State<'_, StorageState>,
    snapshot_id: String,
    app: AppHandle,
) -> Result<Progress> {
    state
        .reverify(storage.inner().clone(), snapshot_id, Some(app))
        .await
}
#[tauri::command]
pub async fn cleanup_backup_staging(
    storage: State<'_, StorageState>,
    snapshot_id: String,
) -> Result<()> {
    validate_id(&snapshot_id)?;
    let storage = storage.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        db(&storage, |s| {
            let context = s.backup_context()?;
            let phase: String = s.connection.query_row(
                "SELECT phase FROM backup_snapshots WHERE id=?1",
                [&snapshot_id],
                |r| r.get(0),
            )?;
            if !matches!(phase.as_str(), "failed" | "cancelled" | "interrupted") {
                return Err(fail("invalid_selection"));
            }
            files::cleanup(
                &context
                    .root
                    .join("temp")
                    .join(format!("backup-{snapshot_id}")),
                &snapshot_id,
                &context.installation,
            )
        })
    })
    .await
    .map_err(|_| fail("failed"))?
}
#[tauri::command]
pub async fn open_backup_location(
    storage: State<'_, StorageState>,
    snapshot_id: String,
) -> Result<()> {
    validate_id(&snapshot_id)?;
    let storage = storage.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        db(&storage, |s| {
            let (root, installation) = s.read_vault()?;
            let count: i64 = s.connection.query_row(
                "SELECT count(*) FROM backup_snapshots WHERE id=?1 AND status='complete'",
                [&snapshot_id],
                |r| r.get(0),
            )?;
            if count != 1 {
                return Err(fail("not_found"));
            }
            let path = root.join("backups").join(&snapshot_id);
            files::verify_owner(&path, &snapshot_id, &installation)?;
            #[cfg(windows)]
            {
                use std::os::windows::ffi::OsStrExt;
                use windows_sys::Win32::UI::{
                    Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL,
                };
                let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
                let verb: Vec<_> = "open\0".encode_utf16().collect();
                let result = unsafe {
                    ShellExecuteW(
                        std::ptr::null_mut(),
                        verb.as_ptr(),
                        path.as_ptr(),
                        std::ptr::null(),
                        std::ptr::null(),
                        SW_SHOWNORMAL,
                    )
                };
                if result as isize <= 32 {
                    return Err(fail("failed"));
                }
                Ok(())
            }
            #[cfg(not(windows))]
            {
                Err(fail("unsupported"))
            }
        })
    })
    .await
    .map_err(|_| fail("failed"))?
}
