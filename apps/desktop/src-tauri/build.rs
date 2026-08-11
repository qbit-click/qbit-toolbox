fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["get_core_status"])),
    )
    .expect("Tauri application command manifest must generate during the desktop build");
}
