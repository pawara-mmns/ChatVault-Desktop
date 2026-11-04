fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "health_check",
            "get_storage_overview",
            "initialize_vault",
            "choose_vault_directory",
            "select_import_file",
            "analyze_import",
            "confirm_import",
            "cancel_import",
            "list_archives",
            "get_archive_summary",
        ]),
    ))
    .expect("failed to build Tauri application permissions");
}
