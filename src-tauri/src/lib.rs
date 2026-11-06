pub mod backup;
pub mod commands;
pub mod config;
pub mod db;
pub mod dto;
pub mod error;
pub mod models;
pub mod state;
pub mod utils;

pub use error::{AppError, Result};

pub use state::AppState;

use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[allow(dead_code)]
struct LogGuard(std::sync::Mutex<tracing_appender::non_blocking::WorkerGuard>);

fn trim_log_files(log_dir: &std::path::Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return;
    };
    let mut files: Vec<_> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.starts_with("gym_manager.log"))
        })
        .collect();
    if files.len() <= keep {
        return;
    }
    files.sort_by_key(|e| e.file_name());
    for entry in &files[..files.len() - keep] {
        std::fs::remove_file(entry.path()).ok();
    }
}

#[tauri::command]
async fn save_pdf(
    app_handle: tauri::AppHandle,
    base64: String,
) -> std::result::Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let bytes = STANDARD.decode(base64).map_err(|e| e.to_string())?;
    let dir = app_handle
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;

    let path = dir.join("barcodes.pdf");
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize the Tauri application builder.
    // `tauri::Builder::default()` works on desktop and mobile. The mobile
    // entry point is selected by `mobile_entry_point`.
    let builder = tauri::Builder::default().plugin(tauri_plugin_process::init());

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = builder.plugin(tauri_plugin_updater::Builder::default().build());

    #[cfg(target_os = "android")]
    let builder = builder.plugin(
        tauri::plugin::Builder::<tauri::Wry, ()>::new("pdf")
            .setup(|_app, api| {
                api.register_android_plugin("com.gym_manager.dc", "PdfPlugin")?;
                Ok(())
            })
            .build(),
    );

    // --- Run Application Setup ---
    let final_builder = builder.setup(|app| {
        // --- App Handle ---
        let app_handle = app.handle().clone();

        // Logging must use Tauri's path resolver: `dirs::data_local_dir()` can
        // return None on Android and the rolling appender panics when it cannot
        // create the log file, aborting the app at startup.
        let log_dir = app_handle
            .path()
            .app_log_dir()
            .or_else(|_| app_handle.path().app_data_dir().map(|dir| dir.join("logs")))
            .unwrap_or_else(|_| std::env::temp_dir().join("gym_manager_logs"));
        let log_dir_ready = std::fs::create_dir_all(&log_dir).is_ok();
        trim_log_files(&log_dir, 7);

        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "gym_manager=debug,tauri=info,sqlx=warn".into());

        let file_layer = if log_dir_ready {
            let file_appender = tracing_appender::rolling::daily(&log_dir, "gym_manager.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            app.manage(LogGuard(std::sync::Mutex::new(guard)));
            Some(
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false),
            )
        } else {
            None
        };

        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
            .with(file_layer)
            .init();

        tracing::info!("=== Application starting ===");
        tracing::info!("Log directory: {:?}", log_dir);

        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

        let settings = tauri::async_runtime::block_on(config::load_settings(&app_handle))
            .map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to load settings: {}", e),
                ))
            })?;
        tracing::info!("Loaded settings: {:?}", settings);

        // Verify app data directory access
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .expect("Failed to get app data directory");
        tracing::info!("App data directory: {:?}", app_dir);

        if !app_dir.exists() {
            std::fs::create_dir_all(&app_dir).expect("Failed to create app data directory");
            tracing::info!("Created app data directory");
        }

        // Verify permissions by writing a test file
        let test_file_path = app_dir.join(".write_test");
        match std::fs::File::create(&test_file_path) {
            Ok(_) => {
                // Clean up test file
                let _ = std::fs::remove_file(test_file_path);
                tracing::info!("Directory is writable");
            }
            Err(e) => {
                tracing::error!("Directory is not writable: {}", e);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("App data directory is not writable: {}", e),
                )));
            }
        }

        // --- Initialize Database ---
        let pool = match rt.block_on(db::init_db(&app_handle)) {
            Ok(pool) => pool,
            Err(e) => {
                tracing::error!("Database initialization failed: {:?}", e);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Database initialization failed: {:?}", e),
                )));
            }
        };
        tracing::info!("Database pool initialized and migrations run.");

        // --- Create and Manage State ---
        let app_state = AppState::new(pool.clone(), settings);
        app.manage(app_state); // Register the state with Tauri
        tracing::info!("Application state created and managed.");

        // --- Spawn Background Tasks ---
        let handle_for_membership_check = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            utils::spawn_membership_check_task(handle_for_membership_check);
        });

        let handle_for_backup_check = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            backup::spawn_backup_check_task(handle_for_backup_check);
        });
        tracing::info!("Background task(s) spawned.");

        Ok(())
    });

    // --- Register Commands and Run ---
    final_builder
        .invoke_handler(tauri::generate_handler![
            save_pdf,
            commands::admin_commands::login,
            commands::admin_commands::get_app_settings,
            commands::admin_commands::update_app_settings,
            commands::admin_commands::get_all_users,
            commands::admin_commands::get_user_by_id,
            commands::admin_commands::change_user_password,
            commands::admin_commands::delete_user,
            commands::admin_commands::get_remote_backup_metadata,
            commands::admin_commands::trigger_backup,
            commands::admin_commands::save_user,
            commands::admin_commands::upload_logo,
            commands::admin_commands::remove_logo,
            commands::admin_commands::upload_login_background,
            commands::admin_commands::remove_login_background,
            backup::restore_from_backup,
            commands::membership_type_commands::add_membership_type,
            commands::membership_type_commands::get_all_membership_types,
            commands::membership_type_commands::get_membership_type_by_id,
            commands::membership_type_commands::update_membership_type,
            commands::membership_type_commands::delete_membership_type,
            commands::member_commands::add_member,
            commands::member_commands::get_member_by_id,
            commands::member_commands::get_member_by_card_id,
            commands::member_commands::delete_member,
            commands::member_commands::update_member,
            commands::member_commands::get_members_with_memberships_paginated,
            commands::member_commands::get_member_by_id_with_membership,
            commands::member_commands::save_member_photo,
            commands::member_commands::delete_member_photo,
            commands::membership_commands::get_all_memberships_for_member,
            commands::membership_commands::save_membership,
            commands::membership_commands::delete_membership,
            commands::membership_commands::get_membership_by_id,
            commands::entry_log_commands::process_scan,
            commands::entry_log_commands::process_scan_single,
            commands::entry_log_commands::get_recent_entry_logs,
            commands::entry_log_commands::get_entry_logs,
            commands::entry_log_commands::delete_entry_log,
            commands::entry_log_commands::get_member_entry_logs,
            commands::entry_log_commands::delete_entry_logs,
            commands::analytics_commands::get_membership_type_distribution,
            commands::analytics_commands::get_revenue_by_membership_type,
            commands::analytics_commands::get_daily_hourly_visit_count,
            commands::analytics_commands::get_active_memberships_over_time,
        ])
        // --- Run the App ---
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application"); // Use expect for fatal errors on startup
}
