fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["health_check"])),
    )
    .expect("failed to build Tauri application permissions");
}
