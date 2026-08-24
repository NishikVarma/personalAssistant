use sqlx::SqlitePool;

use super::now;
use crate::error::{AppError, AppResult};
use crate::models::resume::{ResumeFile, ResumeFileKind};

pub async fn list(pool: &SqlitePool, kind: Option<ResumeFileKind>) -> AppResult<Vec<ResumeFile>> {
    match kind {
        Some(kind) => Ok(sqlx::query_as::<_, ResumeFile>(
            "SELECT * FROM resume_files WHERE kind = ?1 ORDER BY id DESC",
        )
        .bind(kind.as_str())
        .fetch_all(pool)
        .await?),
        None => Ok(
            sqlx::query_as::<_, ResumeFile>("SELECT * FROM resume_files ORDER BY id DESC")
                .fetch_all(pool)
                .await?,
        ),
    }
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<ResumeFile> {
    sqlx::query_as::<_, ResumeFile>("SELECT * FROM resume_files WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("resume file {id}")))
}

pub async fn find_by_sha(pool: &SqlitePool, sha256: &str) -> AppResult<Option<ResumeFile>> {
    sqlx::query_as::<_, ResumeFile>("SELECT * FROM resume_files WHERE sha256 = ?1")
        .bind(sha256)
        .fetch_optional(pool)
        .await
        .map_err(AppError::from)
}

pub async fn create(
    pool: &SqlitePool,
    kind: ResumeFileKind,
    original_filename: &str,
    stored_path: &str,
    sha256: &str,
    file_size: i64,
) -> AppResult<ResumeFile> {
    let ts = now();
    let result = sqlx::query(
        "INSERT INTO resume_files
             (kind, original_filename, stored_path, sha256, file_size, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, '', ?6, ?6)",
    )
    .bind(kind.as_str())
    .bind(original_filename.trim())
    .bind(stored_path)
    .bind(sha256)
    .bind(file_size)
    .bind(&ts)
    .execute(pool)
    .await?;
    get(pool, result.last_insert_rowid()).await
}

/// Removes the row; the caller is responsible for deleting the stored file.
pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<String> {
    let file = get(pool, id).await?;
    sqlx::query("DELETE FROM resume_files WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(file.stored_path)
}
