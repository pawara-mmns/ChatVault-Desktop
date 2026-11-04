use super::{DeviceState, models::*, repository};
use crate::storage::{StorageState, error::StorageError};
use tauri::State;
#[tauri::command]
pub fn list_connected_devices(state: State<'_, DeviceState>) -> Result<DeviceList> {
    state.list()
}
#[tauri::command]
pub async fn refresh_connected_devices(state: State<'_, DeviceState>) -> Result<DeviceList> {
    state.refresh().await
}
#[tauri::command]
pub fn get_device_details(state: State<'_, DeviceState>, device_id: String) -> Result<Device> {
    state.selection(&device_id).map(|(_, d)| d)
}
#[tauri::command]
pub async fn check_device_file_access(
    state: State<'_, DeviceState>,
    device_id: String,
) -> Result<Device> {
    state.inspect(device_id, false).await
}
#[tauri::command]
pub async fn discover_whatsapp_storage(
    state: State<'_, DeviceState>,
    device_id: String,
) -> Result<Device> {
    state.inspect(device_id, true).await
}
#[tauri::command]
pub fn cancel_device_operation(state: State<'_, DeviceState>) -> Result<()> {
    state.cancel()
}
#[tauri::command]
pub async fn get_known_devices(storage: State<'_, StorageState>) -> Result<Vec<KnownDevice>> {
    let storage = storage.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        storage.run(false, |s| {
            repository::known(&s.connection).map_err(|_| StorageError::database())
        })
    })
    .await
    .map_err(|_| error("storage"))?
    .map_err(|_| error("storage"))
}
#[tauri::command]
pub async fn remember_device(
    state: State<'_, DeviceState>,
    storage: State<'_, StorageState>,
    device_id: String,
    preferred: bool,
) -> Result<KnownDevice> {
    let (_, device) = state.selection(&device_id)?;
    let storage = storage.inner().clone();
    let note = tauri::async_runtime::spawn_blocking(move || {
        storage.run(false, |s| {
            repository::save(&s.connection, &device, preferred)
                .map_err(|_| StorageError::database())
        })
    })
    .await
    .map_err(|_| error("storage"))?
    .map_err(|_| error("storage"))?;
    if let Ok(mut cache) = state.0.cache.lock() {
        if let Some(entry) = cache.entries.get_mut(&device_id) {
            entry.device.saved_note_id = Some(note.id.clone());
        }
    }
    Ok(note)
}
