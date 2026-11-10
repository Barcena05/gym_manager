fn main() {
    let attributes = tauri_build::Attributes::new().plugin(
        "pdf",
        tauri_build::InlinedPlugin::new()
            .commands(&["openPdf", "shareFile"])
            .default_permission(tauri_build::DefaultPermissionRule::AllowAllCommands),
    );

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
