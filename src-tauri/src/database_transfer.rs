use crate::config::{load_settings, save_settings, AppSettings};
use crate::error::{AppError, Result as AppResult};
use crate::state::AppState;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Local;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const FORMAT_VERSION: u32 = 1;
const MAX_IMPORT_BYTES: usize = 512 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn app_data_dir(app_handle: &AppHandle) -> AppResult<PathBuf> {
    app_handle
        .path()
        .app_data_dir()
        .map_err(|_| AppError::Config("Cannot determine app data directory".to_string()))
}

fn app_cache_dir(app_handle: &AppHandle) -> AppResult<PathBuf> {
    app_handle
        .path()
        .app_cache_dir()
        .map_err(|_| AppError::Config("Cannot determine app cache directory".to_string()))
}

fn timestamp() -> String {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    if path.exists() {
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn safe_file_name(file_name: &str) -> String {
    Path::new(file_name)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("import.bin")
        .to_string()
}

fn is_zip_file(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"PK\x05\x06")
        || bytes.starts_with(b"PK\x07\x08")
}

fn validate_sqlite_file(path: &Path) -> AppResult<()> {
    let mut file = File::open(path)?;
    let mut header = [0u8; 16];
    file.read_exact(&mut header)?;
    if &header != b"SQLite format 3\0" {
        return Err(AppError::Validation(
            "The selected file is not a SQLite database".to_string(),
        ));
    }
    Ok(())
}

async fn validate_gym_manager_database(path: &Path) -> AppResult<()> {
    validate_sqlite_file(path)?;

    let db_url = format!(
        "sqlite://{}",
        path.to_str()
            .ok_or_else(|| AppError::Config("Invalid database path string".to_string()))?
    );
    let pool = sqlx::SqlitePool::connect(&db_url).await?;
    let members_table: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'members' LIMIT 1",
    )
    .fetch_optional(&pool)
    .await?;
    pool.close().await;

    if members_table.is_none() {
        return Err(AppError::Validation(
            "The selected database is not a Gym Manager database".to_string(),
        ));
    }

    Ok(())
}

async fn snapshot_database(pool: &SqlitePool, target: &Path) -> AppResult<()> {
    remove_if_exists(target)?;
    let escaped = target.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{}'", escaped))
        .execute(pool)
        .await?;
    Ok(())
}

fn add_file_to_zip(
    zip: &mut ZipWriter<File>,
    path: &Path,
    archive_name: &str,
) -> AppResult<()> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut source = File::open(path)?;
    zip.start_file(archive_name, options)
        .map_err(|e| AppError::Config(format!("Failed to write zip entry: {}", e)))?;
    io::copy(&mut source, zip)
        .map_err(|e| AppError::Config(format!("Failed to copy file into zip: {}", e)))?;
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut ZipWriter<File>,
    source_dir: &Path,
    archive_prefix: &str,
) -> AppResult<()> {
    if !source_dir.exists() {
        return Ok(());
    }

    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.add_directory(format!("{}/", archive_prefix), options)
        .map_err(|e| AppError::Config(format!("Failed to write zip directory: {}", e)))?;

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let archive_name = format!("{}/{}", archive_prefix, name);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            add_dir_to_zip(zip, &path, &archive_name)?;
        } else if file_type.is_file() {
            add_file_to_zip(zip, &path, &archive_name)?;
        }
    }

    Ok(())
}

fn create_backup_zip(
    target: &Path,
    database_path: &Path,
    settings_path: &Path,
    app_data: &Path,
    exported_at: &str,
) -> AppResult<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(target)?;
    let mut zip = ZipWriter::new(file);

    add_file_to_zip(&mut zip, database_path, "gym_data.sqlite")?;

    if settings_path.exists() {
        add_file_to_zip(&mut zip, settings_path, "app_settings.json")?;
    }

    add_dir_to_zip(&mut zip, &app_data.join("photos"), "photos")?;
    add_dir_to_zip(&mut zip, &app_data.join("logos"), "logos")?;
    add_dir_to_zip(&mut zip, &app_data.join("backgrounds"), "backgrounds")?;

    let manifest = json!({
        "format": "gym-manager-backup",
        "format_version": FORMAT_VERSION,
        "app_version": env!("CARGO_PKG_VERSION"),
        "exported_at": exported_at,
        "database": "gym_data.sqlite",
        "includes_photos": app_data.join("photos").exists(),
        "includes_settings": settings_path.exists(),
    });

    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)
        .map_err(|e| AppError::Config(format!("Failed to write manifest: {}", e)))?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)
        .map_err(|e| AppError::Config(format!("Failed to write manifest: {}", e)))?;

    zip.finish()
        .map_err(|e| AppError::Config(format!("Failed to finish zip: {}", e)))?;

    Ok(())
}

fn extract_zip_to(source: &Path, target: &Path) -> AppResult<()> {
    let file = File::open(source)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Config(format!("Invalid zip archive: {}", e)))?;

    fs::create_dir_all(target)?;
    let mut total_size = 0u64;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| AppError::Config(format!("Failed to read zip entry: {}", e)))?;

        total_size += entry.size();
        if total_size > MAX_UNCOMPRESSED_BYTES {
            return Err(AppError::Config(
                "The backup archive is too large when uncompressed".to_string(),
            ));
        }

        let Some(enclosed_name) = entry.enclosed_name() else {
            continue;
        };

        if let Some(mode) = entry.unix_mode() {
            if mode & 0o170000 == 0o120000 {
                return Err(AppError::Config(
                    "The backup archive contains a symbolic link".to_string(),
                ));
            }
        }

        let output_path = target.join(enclosed_name);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output = File::create(&output_path)?;
        io::copy(&mut entry, &mut output)?;
    }

    Ok(())
}

fn find_file(root: &Path, file_name: &str) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }

    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        let file_type = entry.file_type().ok()?;
        if file_type.is_file() && entry.file_name().to_string_lossy() == file_name {
            return Some(path);
        }
        if file_type.is_dir() {
            if let Some(found) = find_file(&path, file_name) {
                return Some(found);
            }
        }
    }

    None
}

fn find_dir(root: &Path, dir_name: &str) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }

    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        let file_type = entry.file_type().ok()?;
        if file_type.is_dir() {
            if entry.file_name().to_string_lossy() == dir_name {
                return Some(path);
            }
            if let Some(found) = find_dir(&path, dir_name) {
                return Some(found);
            }
        }
    }

    None
}

fn merge_imported_settings(
    current: AppSettings,
    imported: Option<&Value>,
    app_data: &Path,
) -> AppSettings {
    let Some(imported) = imported else {
        return current;
    };

    let mut merged = current.clone();

    if let Some(value) = imported.get("gym_name").and_then(Value::as_str) {
        if !value.trim().is_empty() {
            merged.gym_name = value.to_string();
        }
    }
    if let Some(value) = imported.get("language").and_then(Value::as_str) {
        if !value.trim().is_empty() {
            merged.language = value.to_string();
        }
    }
    if let Some(value) = imported.get("theme").and_then(Value::as_str) {
        if !value.trim().is_empty() {
            merged.theme = value.to_string();
        }
    }
    if let Some(value) = imported.get("timezone").and_then(Value::as_str) {
        if !value.trim().is_empty() {
            merged.timezone = value.to_string();
        }
    }
    if let Some(value) = imported.get("usd_to_cup_rate").and_then(Value::as_f64) {
        merged.usd_to_cup_rate = value;
    }
    if let Some(value) = imported.get("gym_code").and_then(Value::as_str) {
        if !value.trim().is_empty() {
            merged.gym_code = value.to_string();
        }
    }

    // Never import backup credentials from another device.
    merged.backup_url = current.backup_url.clone();
    merged.backup_enabled = current.backup_enabled;
    merged.backup_period_hours = current.backup_period_hours;

    if let Some(value) = imported.get("logo_path").and_then(Value::as_str) {
        if !value.trim().is_empty() && app_data.join(value).exists() {
            merged.logo_path = Some(value.to_string());
        }
    }
    if let Some(value) = imported
        .get("login_background_path")
        .and_then(Value::as_str)
    {
        if !value.trim().is_empty() && app_data.join(value).exists() {
            merged.login_background_path = Some(value.to_string());
        }
    }

    merged
}

fn replace_database(
    imported_database: &Path,
    app_data: &Path,
    backup_dir: &Path,
) -> AppResult<()> {
    fs::create_dir_all(backup_dir)?;

    let live_database = app_data.join("gym_data.sqlite");
    let live_wal = app_data.join("gym_data.sqlite-wal");
    let live_shm = app_data.join("gym_data.sqlite-shm");

    if live_database.exists() {
        fs::copy(&live_database, backup_dir.join("gym_data.sqlite"))?;
    }
    if live_wal.exists() {
        fs::copy(&live_wal, backup_dir.join("gym_data.sqlite-wal"))?;
    }
    if live_shm.exists() {
        fs::copy(&live_shm, backup_dir.join("gym_data.sqlite-shm"))?;
    }

    for directory in ["photos", "logos", "backgrounds"] {
        let source = app_data.join(directory);
        if source.exists() {
            copy_dir_all(&source, &backup_dir.join(directory))?;
        }
    }

    remove_if_exists(&live_database)?;
    remove_if_exists(&live_wal)?;
    remove_if_exists(&live_shm)?;

    fs::copy(imported_database, &live_database)?;

    let imported_wal = imported_database.with_extension("sqlite-wal");
    let imported_shm = imported_database.with_extension("sqlite-shm");
    if imported_wal.exists() {
        fs::copy(&imported_wal, &live_wal)?;
    }
    if imported_shm.exists() {
        fs::copy(&imported_shm, &live_shm)?;
    }

    Ok(())
}

fn restore_database_from_backup(app_data: &Path, backup_dir: &Path) {
    let live_database = app_data.join("gym_data.sqlite");
    let live_wal = app_data.join("gym_data.sqlite-wal");
    let live_shm = app_data.join("gym_data.sqlite-shm");

    let _ = remove_if_exists(&live_database);
    let _ = remove_if_exists(&live_wal);
    let _ = remove_if_exists(&live_shm);

    let backup_database = backup_dir.join("gym_data.sqlite");
    if backup_database.exists() {
        let _ = fs::copy(&backup_database, &live_database);
    }
    let backup_wal = backup_dir.join("gym_data.sqlite-wal");
    if backup_wal.exists() {
        let _ = fs::copy(&backup_wal, &live_wal);
    }
    let backup_shm = backup_dir.join("gym_data.sqlite-shm");
    if backup_shm.exists() {
        let _ = fs::copy(&backup_shm, &live_shm);
    }

    for directory in ["photos", "logos", "backgrounds"] {
        let destination = app_data.join(directory);
        let _ = remove_if_exists(&destination);
        let backup = backup_dir.join(directory);
        if backup.exists() {
            let _ = copy_dir_all(&backup, &destination);
        }
    }
}

fn copy_imported_directories(extracted_root: &Path, app_data: &Path) -> AppResult<()> {
    for directory in ["photos", "logos", "backgrounds"] {
        let Some(source) = find_dir(extracted_root, directory) else {
            continue;
        };
        let destination = app_data.join(directory);
        remove_if_exists(&destination)?;
        copy_dir_all(&source, &destination)?;
    }
    Ok(())
}

fn read_imported_settings(settings_path: Option<PathBuf>) -> Option<Value> {
    let path = settings_path?;
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<Value>(&bytes).ok()
}

#[tauri::command]
pub async fn export_database_zip(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    target_path: Option<String>,
) -> AppResult<String> {
    state.require_admin().await?;

    let app_data = app_data_dir(&app_handle)?;
    let cache_dir = app_cache_dir(&app_handle)?;
    fs::create_dir_all(&cache_dir)?;

    let exported_at = Local::now().to_rfc3339();
    let work_dir = cache_dir.join(format!("export_{}", timestamp()));
    fs::create_dir_all(&work_dir)?;

    let database_path = work_dir.join("gym_data.sqlite");
    snapshot_database(&state.db_pool, &database_path).await?;

    let mut settings = load_settings(&app_handle).await?;
    // Do not export backup credentials.
    settings.backup_url = None;
    settings.backup_enabled = false;

    let settings_path = work_dir.join("app_settings.json");
    fs::write(&settings_path, serde_json::to_vec_pretty(&settings)?)?;

    let target = match target_path {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => {
            let filename = format!("GymManager_backup_{}.zip", timestamp());

            #[cfg(any(target_os = "android", target_os = "ios"))]
            {
                cache_dir.join(filename)
            }

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                let directory = dirs::download_dir()
                    .or_else(dirs::desktop_dir)
                    .unwrap_or_else(|| cache_dir.clone());
                fs::create_dir_all(&directory)?;
                directory.join(filename)
            }
        }
    };

    let target = if target.extension().is_none() {
        target.with_extension("zip")
    } else {
        target
    };

    create_backup_zip(
        &target,
        &database_path,
        &settings_path,
        &app_data,
        &exported_at,
    )?;

    let _ = fs::remove_dir_all(&work_dir);

    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_database_file(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    base64: String,
    file_name: String,
) -> AppResult<()> {
    state.require_admin().await?;

    let bytes = STANDARD
        .decode(base64)
        .map_err(|e| AppError::Base64Decode(e))?;

    if bytes.is_empty() {
        return Err(AppError::Validation(
            "The backup file is empty".to_string(),
        ));
    }

    if bytes.len() > MAX_IMPORT_BYTES {
        return Err(AppError::Validation(
            "The backup file is too large".to_string(),
        ));
    }

    let app_data = app_data_dir(&app_handle)?;
    let cache_dir = app_cache_dir(&app_handle)?;
    fs::create_dir_all(&cache_dir)?;

    let import_id = timestamp();
    let work_dir = cache_dir.join(format!("import_{}", import_id));
    fs::create_dir_all(&work_dir)?;

    let source_path = work_dir.join(safe_file_name(&file_name));
    fs::write(&source_path, &bytes)?;

    let (database_path, settings_path, extracted_root) = if is_zip_file(&bytes) {
        let extracted_root = work_dir.join("extracted");
        extract_zip_to(&source_path, &extracted_root)?;

        let database_path = find_file(&extracted_root, "gym_data.sqlite")
            .ok_or_else(|| AppError::Validation("gym_data.sqlite was not found in the backup".to_string()))?;
        let settings_path = find_file(&extracted_root, "app_settings.json");
        (database_path, settings_path, Some(extracted_root))
    } else {
        let database_path = work_dir.join("gym_data.sqlite");
        fs::copy(&source_path, &database_path)?;
        (database_path, None, None)
    };

    // Prepare and validate a copy before touching the live database.
    validate_gym_manager_database(&database_path).await?;
    let prepared_pool = crate::db::prepare_database(&database_path).await?;
    prepared_pool.close().await;

    let imported_settings = read_imported_settings(settings_path);
    let current_settings = load_settings(&app_handle).await?;

    // At this point the imported database and files are valid. Close the live
    // pool and swap the files, keeping a local backup for rollback/manual use.
    let backup_dir = app_data.join(format!("import_backup_{}", import_id));
    fs::create_dir_all(&backup_dir)?;

    state.db_pool.close().await;

    let replacement_result = replace_database(&database_path, &app_data, &backup_dir);

    if let Err(error) = replacement_result {
        restore_database_from_backup(&app_data, &backup_dir);
        let _ = fs::remove_dir_all(&work_dir);
        return Err(error);
    }

    if let Some(root) = &extracted_root {
        if let Err(error) = copy_imported_directories(root, &app_data) {
            restore_database_from_backup(&app_data, &backup_dir);
            let _ = fs::remove_dir_all(&work_dir);
            return Err(error);
        }
    }

    let merged_settings =
        merge_imported_settings(current_settings, imported_settings.as_ref(), &app_data);

    if let Err(error) = save_settings(&app_handle, &merged_settings).await {
        restore_database_from_backup(&app_data, &backup_dir);
        let _ = fs::remove_dir_all(&work_dir);
        return Err(error);
    }

    let _ = fs::remove_dir_all(&work_dir);

    Ok(())
}
