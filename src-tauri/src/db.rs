use crate::card_id;
use crate::error::{AppError, Result};
use crate::utils;
use sqlx::{migrate::Migrator, sqlite::SqlitePoolOptions, Row as _, SqlitePool};
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn init_db(app_handle: &AppHandle) -> Result<SqlitePool> {
    let db_path = get_database_path(app_handle)?.join("gym_data.sqlite");

    if let Some(parent_dir) = db_path.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }

    if !db_path.exists() {
        tracing::info!("Database file does not exist, creating it...");
        std::fs::File::create(&db_path)
            .map_err(|e| AppError::Config(format!("Failed to create database file: {}", e)))?;
        tracing::info!("Created empty database file at: {:?}", db_path);
    }

    prepare_database(&db_path).await
}

/// Opens a database file, applies migrations, creates the default admin user
/// when needed and normalizes legacy card identifiers.
///
/// This is also used by the import flow on a temporary copy, so an invalid or
/// incompatible backup is rejected before replacing the live database.
pub async fn prepare_database(db_path: &std::path::Path) -> Result<SqlitePool> {
    let db_url = format!(
        "sqlite://{}",
        db_path
            .to_str()
            .ok_or_else(|| AppError::Config("Invalid database path string".to_string()))?
    );

    tracing::info!("Connecting to database at: {}", db_url);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&db_url)
        .await
        .map_err(|e| AppError::Config(format!("Failed to connect to database: {}", e)))?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await
        .map_err(|e| AppError::Config(format!("Failed to set journal mode: {}", e)))?;

    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await
        .map_err(|e| AppError::Config(format!("Failed to set synchronous mode: {}", e)))?;

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .map_err(|e| AppError::Config(format!("Failed to enable foreign keys: {}", e)))?;

    tracing::info!("=== INICIO DE MIGRACIONES INCRUSTADAS EN EL BINARIO ===");
    for migration in MIGRATOR.migrations.iter() {
        tracing::info!(" - v{}: {}", migration.version, migration.description);
    }
    tracing::info!(
        "=== FIN DE MIGRACIONES INCRUSTADAS (Total: {}) ===",
        MIGRATOR.migrations.len()
    );

    let fixed = fix_migration_checksums(&pool).await?;
    if fixed > 0 {
        tracing::info!("Normalized {} migration checksums", fixed);
    }

    tracing::info!("Running database migrations...");
    MIGRATOR.run(&pool).await.map_err(|e| {
        tracing::error!("Migration failed: {:?}", e);
        AppError::Config(format!("Database migration failed: {}", e))
    })?;
    tracing::info!("Database migrations completed successfully.");

    create_default_admin_user_if_not_exists(&pool).await?;
    normalize_card_ids(&pool).await?;

    // Merge any WAL content into the main file before the pool is closed.
    let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await;

    Ok(pool)
}

/// Fixes migration checksum mismatches caused by CRLF line endings on Windows.

async fn fix_migration_checksums(pool: &SqlitePool) -> Result<usize> {
    let table_exists: bool = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations')",
    )
    .fetch_one(pool)
    .await
    .and_then(|row| row.try_get(0))
    .unwrap_or(false);

    if !table_exists {
        return Ok(0);
    }

    let mut fixed = 0usize;

    for migration in MIGRATOR.migrations.iter() {
        let row = sqlx::query("SELECT checksum FROM _sqlx_migrations WHERE version = ?")
            .bind(migration.version)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            let stored: Vec<u8> = row.try_get("checksum").unwrap_or_default();
            let expected: &[u8] = &migration.checksum;

            if stored == expected {
                continue;
            }

            if stored.len() == expected.len() {
                sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
                    .bind(expected)
                    .bind(migration.version)
                    .execute(pool)
                    .await?;
                fixed += 1;
                tracing::info!(
                    "Fixed CRLF→LF SHA-384 checksum for migration v{}",
                    migration.version
                );
            }
        }
    }

    Ok(fixed)
}

pub fn get_database_path(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|_| {
            AppError::Config("Could not determine application data directory".to_string())
        })?;

    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir)?;
    }

    tracing::debug!("Database will be stored at: {:?}", app_dir);

    Ok(app_dir)
}

async fn create_default_admin_user_if_not_exists(pool: &SqlitePool) -> Result<()> {
    let default_username = "admin";
    let default_password = "admin";

    let admin_exists: Option<bool> = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM users WHERE username = ?1 LIMIT 1) as "exists: bool""#,
        default_username
    )
    .fetch_one(pool)
    .await?;

    if admin_exists.unwrap_or(false) {
        tracing::info!("Admin user already exists. Skipping creation.");
        return Ok(());
    }

    tracing::info!("Creating default admin user...");

    let password_hash = utils::hash_password(default_password)?;
    let now = chrono::Utc::now().naive_utc();

    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"
        INSERT INTO users (username, password_hash, role, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
        default_username,
        password_hash,
        "admin",
        now,
        now
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!("Default admin user created successfully.");
    Ok(())
}

/// Normalizes legacy card identifiers and keeps the denormalized references
/// in entry_logs in sync. This runs once per database through the
/// data_migrations marker.
pub async fn normalize_card_ids(pool: &SqlitePool) -> Result<()> {
    let already_applied: Option<String> = sqlx::query_scalar(
        "SELECT key FROM data_migrations WHERE key = 'card_id_v2'",
    )
    .fetch_optional(pool)
    .await?;

    if already_applied.is_some() {
        return Ok(());
    }

    tracing::info!("Normalizing legacy card ids...");

    let mut tx = pool.begin().await?;
    let members = sqlx::query_as::<_, (i64, Option<String>, Option<String>)>(
        "SELECT id, card_id, short_card_id FROM members",
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut used_cards: HashSet<String> = members
        .iter()
        .filter_map(|(_, card, _)| card.clone())
        .filter(|card| card_id::is_valid_card_id(card))
        .collect();

    let mut used_short_cards: HashSet<String> = members
        .iter()
        .filter_map(|(_, _, short)| short.clone())
        .filter(|short| card_id::is_valid_short_card_id(short))
        .collect();

    let now = chrono::Utc::now().naive_utc();

    for (member_id, old_card_id, old_short_card_id) in members {
        let card_is_valid = old_card_id
            .as_deref()
            .map(card_id::is_valid_card_id)
            .unwrap_or(false);

        if !card_is_valid {
            let mut new_card_id = card_id::generate_card_id();
            while used_cards.contains(&new_card_id) {
                new_card_id = card_id::generate_card_id();
            }
            used_cards.insert(new_card_id.clone());

            sqlx::query(
                "UPDATE members SET card_id = ?, updated_at = ? WHERE id = ?",
            )
            .bind(&new_card_id)
            .bind(now)
            .bind(member_id)
            .execute(&mut *tx)
            .await?;

            if let Some(old_card_id) = old_card_id.as_deref() {
                if old_card_id != new_card_id {
                    sqlx::query("UPDATE entry_logs SET card_id = ? WHERE card_id = ?")
                        .bind(&new_card_id)
                        .bind(old_card_id)
                        .execute(&mut *tx)
                        .await?;
                }
            }

            tracing::info!(
                "Member {} card_id normalized from {:?} to {}",
                member_id,
                old_card_id,
                new_card_id
            );
        }

        let short_is_valid = old_short_card_id
            .as_deref()
            .map(card_id::is_valid_short_card_id)
            .unwrap_or(false);

        if !short_is_valid {
            let mut new_short_card_id = card_id::generate_short_card_id();
            while used_short_cards.contains(&new_short_card_id) {
                new_short_card_id = card_id::generate_short_card_id();
            }
            used_short_cards.insert(new_short_card_id.clone());

            sqlx::query(
                "UPDATE members SET short_card_id = ?, updated_at = ? WHERE id = ?",
            )
            .bind(&new_short_card_id)
            .bind(now)
            .bind(member_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    sqlx::query(
        "INSERT OR IGNORE INTO data_migrations (key) VALUES ('card_id_v2')",
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    tracing::info!("Card id normalization completed.");

    Ok(())
}
