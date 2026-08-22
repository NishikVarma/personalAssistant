pub mod settings_repo;

use std::path::PathBuf;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};

use crate::error::AppResult;

const DB_FILE: &str = "assistant.db";

pub const MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

fn connect_options(path: PathBuf) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5))
}

pub async fn open(path: PathBuf) -> AppResult<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options(path))
        .await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> AppResult<()> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

pub async fn init(data_dir: PathBuf) -> AppResult<(SqlitePool, String)> {
    tokio::fs::create_dir_all(&data_dir).await?;
    let db_path = data_dir.join(DB_FILE);
    let pool = open(db_path.clone()).await?;
    Ok((pool, db_path.to_string_lossy().into_owned()))
}

pub async fn schema_version(pool: &SqlitePool) -> AppResult<i64> {
    let version: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await?;
    Ok(version)
}
