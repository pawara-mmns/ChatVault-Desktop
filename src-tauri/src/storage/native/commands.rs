use super::{models::*, *};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
#[tauri::command]
pub async fn inspect_native_backup(
    state: State<'_, NativeState>,
    storage: State<'_, StorageState>,
    snapshot_id: String,
) -> Result<Inspection> {
    state.inspect(storage.inner().clone(), snapshot_id).await
}
#[tauri::command]
pub async fn verify_native_key(
    state: State<'_, NativeState>,
    storage: State<'_, StorageState>,
    snapshot_id: String,
    file_id: String,
    key: String,
    process_consent: bool,
) -> Result<Preview> {
    let key = Zeroizing::new(key);
    if key.len() != 64 {
        return Err(fail("native_key"));
    }
    state
        .verify(
            storage.inner().clone(),
            snapshot_id,
            file_id,
            KeyInput::Hex(key),
            process_consent,
        )
        .await
}
#[tauri::command]
pub async fn verify_native_key_file(
    app: AppHandle,
    state: State<'_, NativeState>,
    storage: State<'_, StorageState>,
    snapshot_id: String,
    file_id: String,
    process_consent: bool,
) -> Result<Option<Preview>> {
    if !process_consent {
        return Err(fail("native_consent"));
    }
    let bytes =
        tauri::async_runtime::spawn_blocking(move || -> Result<Option<Zeroizing<Vec<u8>>>> {
            let mut dialog = app
                .dialog()
                .file()
                .set_title("Select your matching local crypt14 key file");
            if let Some(window) = app.get_webview_window("main") {
                dialog = dialog.set_parent(&window);
            }
            let Some(selected) = dialog.blocking_pick_file() else {
                return Ok(None);
            };
            let path = selected.into_path().map_err(|_| fail("native_key"))?;
            let mut bytes = Zeroizing::new(Vec::new());
            crate::storage::paths::open_regular_file(&path)?
                .take(159)
                .read_to_end(&mut bytes)
                .map_err(|_| fail("native_key"))?;
            if !matches!(bytes.len(), 131 | 158) {
                return Err(fail("native_key"));
            }
            Ok(Some(bytes))
        })
        .await
        .map_err(|_| fail("native_io"))??;
    match bytes {
        Some(bytes) => state
            .verify(
                storage.inner().clone(),
                snapshot_id,
                file_id,
                KeyInput::File(bytes),
                true,
            )
            .await
            .map(Some),
        None => Ok(None),
    }
}
#[tauri::command]
pub async fn import_native_backup(
    state: State<'_, NativeState>,
    storage: State<'_, StorageState>,
    token: String,
    plaintext_consent: bool,
) -> Result<Preview> {
    state
        .import(storage.inner().clone(), token, plaintext_consent)
        .await
}
#[tauri::command]
pub fn get_native_reader_progress(state: State<'_, NativeState>) -> Result<Progress> {
    state.progress()
}
#[tauri::command]
pub fn cancel_native_reader(state: State<'_, NativeState>) -> Result<()> {
    state.cancel()
}
