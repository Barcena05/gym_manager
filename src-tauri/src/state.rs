use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::config::AppSettings;
use crate::error::{AppError, ErrorCodes, Result as AppResult, TranslatableError};

const SESSION_FILENAME: &str = "session.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUser {
    pub username: String,
    pub role: String,
}

fn session_path(app_handle: &AppHandle) -> AppResult<PathBuf> {
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|_| AppError::Config("Cannot determine app config directory".to_string()))?;
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join(SESSION_FILENAME))
}

pub async fn load_session(app_handle: &AppHandle) -> Option<SessionUser> {
    let path = session_path(app_handle).ok()?;
    let content = tokio::fs::read_to_string(path).await.ok()?;
    serde_json::from_str::<SessionUser>(&content).ok()
}

pub async fn save_session(app_handle: &AppHandle, session: &SessionUser) -> AppResult<()> {
    let path = session_path(app_handle)?;
    let content = serde_json::to_vec(session)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}

pub async fn clear_session(app_handle: &AppHandle) -> AppResult<()> {
    let path = session_path(app_handle)?;
    let _ = tokio::fs::remove_file(path).await;
    Ok(())
}

#[derive(Debug)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub settings: tokio::sync::RwLock<AppSettings>,
    pub last_membership_check: tokio::sync::RwLock<Option<chrono::NaiveDateTime>>,
    pub last_backup: tokio::sync::RwLock<Option<chrono::NaiveDateTime>>,
    pub current_user: tokio::sync::RwLock<Option<SessionUser>>,
}

impl AppState {
    pub fn new(
        db_pool: SqlitePool,
        settings: AppSettings,
        current_user: Option<SessionUser>,
    ) -> Self {
        Self {
            db_pool,
            settings: tokio::sync::RwLock::new(settings),
            last_membership_check: tokio::sync::RwLock::new(None),
            last_backup: tokio::sync::RwLock::new(None),
            current_user: tokio::sync::RwLock::new(current_user),
        }
    }

    pub async fn require_admin(&self) -> AppResult<()> {
        let current_user = self.current_user.read().await;
        match current_user.as_ref() {
            Some(user) if user.role == "admin" => Ok(()),
            _ => Err(AppError::Translatable(TranslatableError::new(
                ErrorCodes::ADMIN_REQUIRED,
                "Admin privileges are required for this action.",
            ))),
        }
    }
}
