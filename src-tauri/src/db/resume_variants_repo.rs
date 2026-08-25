use sqlx::SqlitePool;

use super::now;
use crate::error::{AppError, AppResult};
use crate::models::resume::{ResumeCategory, ResumeVariant};

pub async fn list(
    pool: &SqlitePool,
    application_id: Option<i64>,
) -> AppResult<Vec<ResumeVariant>> {
    match application_id {
        Some(application_id) => Ok(sqlx::query_as::<_, ResumeVariant>(
            "SELECT * FROM resume_variants WHERE application_id = ?1 ORDER BY id DESC",
        )
        .bind(application_id)
        .fetch_all(pool)
        .await?),
        None => Ok(sqlx::query_as::<_, ResumeVariant>(
            "SELECT * FROM resume_variants ORDER BY id DESC",
        )
        .fetch_all(pool)
        .await?),
    }
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<ResumeVariant> {
    sqlx::query_as::<_, ResumeVariant>("SELECT * FROM resume_variants WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("resume variant {id}")))
}

pub async fn create(
    pool: &SqlitePool,
    base_file_id: Option<i64>,
    application_id: Option<i64>,
    category: ResumeCategory,
    label: &str,
) -> AppResult<ResumeVariant> {
    let ts = now();
    let result = sqlx::query(
        "INSERT INTO resume_variants
             (base_file_id, application_id, category, label, status, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'draft', '', ?5, ?5)",
    )
    .bind(base_file_id)
    .bind(application_id)
    .bind(category.as_str())
    .bind(label.trim())
    .bind(&ts)
    .execute(pool)
    .await?;
    get(pool, result.last_insert_rowid()).await
}

pub async fn set_paths(
    pool: &SqlitePool,
    id: i64,
    tex_path: Option<&str>,
    pdf_path: Option<&str>,
) -> AppResult<()> {
    sqlx::query("UPDATE resume_variants SET tex_path = ?1, pdf_path = ?2, updated_at = ?3 WHERE id = ?4")
        .bind(tex_path)
        .bind(pdf_path)
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn approve(pool: &SqlitePool, id: i64) -> AppResult<ResumeVariant> {
    let result =
        sqlx::query("UPDATE resume_variants SET status = 'approved', updated_at = ?1 WHERE id = ?2")
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("resume variant {id}")));
    }
    get(pool, id).await
}

/// Deletes the row and returns both stored paths for the caller to clean up.
pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<(Option<String>, Option<String>)> {
    let variant = get(pool, id).await?;
    sqlx::query("DELETE FROM resume_variants WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok((variant.tex_path, variant.pdf_path))
}
