use serde::Serialize;
use tauri::Manager;
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
    tauri::Builder::default()
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
            storage::imports::commands::get_archive_summary
        ])
        .run(tauri::generate_context!())
        .expect("failed to start ChatVault Desktop");
}
