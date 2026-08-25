use sqlx::SqlitePool;

use super::now;
use crate::error::{AppError, AppResult};
use crate::models::bulk::BulkBatch;
use crate::models::email::EmailType;

pub async fn create(
    pool: &SqlitePool,
    email_type: EmailType,
    application_id: Option<i64>,
) -> AppResult<BulkBatch> {
    if let Some(application_id) = application_id {
        let found: Option<(i64,)> = sqlx::query_as("SELECT id FROM applications WHERE id = ?1")
            .bind(application_id)
            .fetch_optional(pool)
            .await?;
        if found.is_none() {
            return Err(AppError::NotFound(format!("application {application_id}")));
        }
    }
    let ts = now();
    let result = sqlx::query(
        "INSERT INTO bulk_batches (email_type, application_id, status, created_at, updated_at)
         VALUES (?1, ?2, 'draft', ?3, ?3)",
    )
    .bind(email_type.as_str())
    .bind(application_id)
    .bind(&ts)
    .execute(pool)
    .await?;
    get(pool, result.last_insert_rowid()).await
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<BulkBatch> {
    sqlx::query_as::<_, BulkBatch>("SELECT * FROM bulk_batches WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("bulk batch {id}")))
}

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<BulkBatch>> {
    Ok(sqlx::query_as::<_, BulkBatch>(
        "SELECT * FROM bulk_batches ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn set_status(pool: &SqlitePool, id: i64, status: &str) -> AppResult<()> {
    let result = sqlx::query("UPDATE bulk_batches SET status = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(status)
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("bulk batch {id}")));
    }
    Ok(())
}

pub async fn bump_counts(
    pool: &SqlitePool,
    id: i64,
    sent_delta: i64,
    failed_delta: i64,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE bulk_batches
         SET sent_count = sent_count + ?1, failed_count = failed_count + ?2, updated_at = ?3
         WHERE id = ?4",
    )
    .bind(sent_delta)
    .bind(failed_delta)
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_total(pool: &SqlitePool, id: i64, total: i64) -> AppResult<()> {
    sqlx::query("UPDATE bulk_batches SET total_count = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(total)
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
