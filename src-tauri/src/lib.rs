use serde::Serialize;

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
        .invoke_handler(tauri::generate_handler![health_check])
        .run(tauri::generate_context!())
        .expect("failed to start ChatVault Desktop");
}
