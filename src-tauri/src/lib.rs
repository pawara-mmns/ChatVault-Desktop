use serde::Serialize;
use tauri::Manager;
mod devices;
mod storage;

#[derive(Serialize)]
struct BackendHealth {
    status: &'static str,
    version: &'static str,
    platform: &'static str,
}

/// Side-effect-free IPC check. Reads no files and returns no user data.
#[tauri::command]
fn health_check() -> BackendHealth {
    BackendHealth {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    storage::viewer::protocol::register(tauri::Builder::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let local = app.path().app_local_data_dir().map_err(|_| {
                storage::error::StorageError::new(
                    "storage_unavailable",
                    "The application-local storage directory could not be resolved.",
                )
            });
            let state = storage::StorageState::new(local);
            app.manage(state.clone());
            app.manage(devices::DeviceState::default());
            // Bootstrap off the event thread. Commands serialize with this task through the same state.
            tauri::async_runtime::spawn_blocking(move || {
                let _ = state.run(false, |service| service.overview());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health_check,
            storage::commands::get_storage_overview,
            storage::commands::initialize_vault,
            storage::commands::choose_vault_directory,
            storage::imports::commands::select_import_file,
            storage::imports::commands::analyze_import,
            storage::imports::commands::confirm_import,
            storage::imports::commands::cancel_import,
            storage::imports::commands::list_archives,
            storage::imports::commands::get_archive_summary,
            storage::viewer::commands::list_conversations,
            storage::viewer::commands::get_conversation,
            storage::viewer::commands::read_messages,
            storage::viewer::commands::get_message_context,
            storage::viewer::commands::search_messages,
            storage::viewer::commands::list_conversation_media,
            storage::viewer::commands::get_media_metadata,
            storage::viewer::commands::authorize_media,
            storage::viewer::commands::revoke_media,
            storage::viewer::commands::open_document,
            devices::commands::list_connected_devices,
            devices::commands::refresh_connected_devices,
            devices::commands::get_device_details,
            devices::commands::check_device_file_access,
            devices::commands::discover_whatsapp_storage,
            devices::commands::cancel_device_operation,
            devices::commands::get_known_devices,
            devices::commands::remember_device
        ])
        .build(tauri::generate_context!())
        .expect("failed to start ChatVault Desktop")
        .run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                if let Some(devices) = app.try_state::<devices::DeviceState>() {
                    let _ = devices.cancel();
                }
            }
        });
}
