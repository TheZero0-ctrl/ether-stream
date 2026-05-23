use std::fs;
use std::path::PathBuf;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

pub struct AppDatabase(pub SqlitePool);

pub fn initialize(app: &AppHandle) -> Result<AppDatabase, String> {
    let db_path = database_path(app)?;
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = tauri::async_runtime::block_on(async {
        SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
    })
    .map_err(|err| format!("failed to connect sqlite pool: {err}"))?;

    tauri::async_runtime::block_on(async {
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|err| format!("failed to run migrations: {err}"))
    })?;

    Ok(AppDatabase(pool))
}

fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data dir: {err}"))?;

    fs::create_dir_all(&dir).map_err(|err| format!("failed to create app data dir: {err}"))?;

    dir.push("ether.sqlite3");
    Ok(dir)
}
