use super::{Result, error, models::*};
use crate::storage::StorageState;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

async fn worker<T: Send + 'static>(job: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(job).await.map_err(|_|error("import_failed","The local import task could not finish. Check Archives before choosing the file again."))?
}
#[tauri::command]
pub async fn select_import_file(
    app: AppHandle,
    state: State<'_, StorageState>,
) -> Result<Option<SelectedImport>> {
    let state = state.inner().clone();
    worker(move || {
        state.run(false, |service| {
            service.ensure_import_selectable()?;
            let mut dialog = app
                .dialog()
                .file()
                .set_title("Choose a WhatsApp chat export")
                .add_filter("WhatsApp chat export", &["txt", "zip"]);
            if let Some(window) = app.get_webview_window("main") {
                dialog = dialog.set_parent(&window);
            }
            match dialog.blocking_pick_file() {
                None => Ok(None),
                Some(selected) => {
                    let path = selected
                        .into_path()
                        .map_err(|_| error("invalid_file", "Choose a local text or ZIP export."))?;
                    service.select_source(&path).map(Some)
                }
            }
        })
    })
    .await
}
#[tauri::command]
pub async fn analyze_import(
    token: String,
    date_order: DateOrder,
    state: State<'_, StorageState>,
) -> Result<ImportPreview> {
    let state = state.inner().clone();
    worker(move || state.run(false, |service| service.analyze_import(&token, date_order))).await
}
#[tauri::command]
pub async fn confirm_import(
    token: String,
    title: String,
    state: State<'_, StorageState>,
) -> Result<ArchiveSummary> {
    let state = state.inner().clone();
    worker(move || state.run(false, |service| service.confirm_import(&token, &title))).await
}
#[tauri::command]
pub async fn cancel_import(token: String, state: State<'_, StorageState>) -> Result<()> {
    let state = state.inner().clone();
    worker(move || state.run(false, |service| service.cancel_import(&token))).await
}
#[tauri::command]
pub async fn list_archives(
    query: String,
    offset: u32,
    limit: u32,
    state: State<'_, StorageState>,
) -> Result<ArchivePage> {
    let state = state.inner().clone();
    worker(move || {
        state.run(false, |service| {
            service.list_archives(&query, offset, limit)
        })
    })
    .await
}
#[tauri::command]
pub async fn get_archive_summary(
    id: String,
    state: State<'_, StorageState>,
) -> Result<ArchiveSummary> {
    let state = state.inner().clone();
    worker(move || state.run(false, |service| service.archive_summary(&id))).await
}
