use super::{
    StorageState,
    error::{Result, StorageError},
    models::StorageOverview,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

async fn worker<T: Send + 'static>(job: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|_| {
            StorageError::new(
                "storage_unavailable",
                "The storage task could not finish. Restart the application and retry.",
            )
        })?
}

#[tauri::command]
pub async fn get_storage_overview(state: State<'_, StorageState>) -> Result<StorageOverview> {
    let state = state.inner().clone();
    worker(move || state.run(false, |service| service.overview())).await
}

#[tauri::command]
pub async fn initialize_vault(state: State<'_, StorageState>) -> Result<StorageOverview> {
    let state = state.inner().clone();
    worker(move || state.run(true, |service| service.initialize_vault())).await
}

#[tauri::command]
pub async fn choose_vault_directory(
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<Option<StorageOverview>> {
    let state = state.inner().clone();
    worker(move || {
        state.run(false, |service| {
            service.ensure_switchable()?;
            // Executed on a worker, never the Tauri event/UI thread. No broad dialog permission is exposed.
            let mut dialog = app
                .dialog()
                .file()
                .set_title("Choose an empty local folder for your ChatVault vault");
            if let Some(window) = app.get_webview_window("main") {
                dialog = dialog.set_parent(&window);
            }
            let selected = dialog.blocking_pick_folder();
            match selected {
                None => Ok(None),
                Some(path) => {
                    let path = path.into_path().map_err(|_| StorageError::unsafe_path())?;
                    service.change_vault(&path).map(Some)
                }
            }
        })
    })
    .await
}
