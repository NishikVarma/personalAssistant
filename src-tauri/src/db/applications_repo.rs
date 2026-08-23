use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::application::{Application, ApplicationInput, ApplicationStatus};

pub async fn list(
    pool: &SqlitePool,
    status: Option<ApplicationStatus>,
) -> AppResult<Vec<Application>> {
    match status {
        Some(status) => Ok(sqlx::query_as::<_, Application>(
            "SELECT * FROM applications WHERE status = ?1 ORDER BY priority DESC, updated_at DESC",
        )
        .bind(status.as_str())
        .fetch_all(pool)
        .await?),
        None => Ok(sqlx::query_as::<_, Application>(
            "SELECT * FROM applications ORDER BY priority DESC, updated_at DESC",
        )
        .fetch_all(pool)
        .await?),
    }
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Application> {
    sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("application {id}")))
}

pub async fn create(pool: &SqlitePool, input: &ApplicationInput) -> AppResult<Application> {
    let (company, role) = validate(input)?;
    let result = sqlx::query(
        "INSERT INTO applications
             (company, role, job_description, job_url, source, date_discovered, date_applied,
              follow_up_date, interview_status, priority, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
    )
    .bind(&company)
    .bind(&role)
    .bind(input.job_description.trim())
    .bind(optional(&input.job_url))
    .bind(optional(&input.source))
    .bind(optional(&input.date_discovered))
    .bind(optional(&input.date_applied))
    .bind(optional(&input.follow_up_date))
    .bind(optional(&input.interview_status))
    .bind(input.priority)
    .bind(input.notes.trim())
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &ApplicationInput,
) -> AppResult<Application> {
    let (company, role) = validate(input)?;
    let result = sqlx::query(
        "UPDATE applications
         SET company = ?1, role = ?2, job_description = ?3, job_url = ?4, source = ?5,
             date_discovered = ?6, date_applied = ?7, follow_up_date = ?8,
             interview_status = ?9, priority = ?10, notes = ?11, updated_at = ?12
         WHERE id = ?13",
    )
    .bind(&company)
    .bind(&role)
    .bind(input.job_description.trim())
    .bind(optional(&input.job_url))
    .bind(optional(&input.source))
    .bind(optional(&input.date_discovered))
    .bind(optional(&input.date_applied))
    .bind(optional(&input.follow_up_date))
    .bind(optional(&input.interview_status))
    .bind(input.priority)
    .bind(input.notes.trim())
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("application {id}")));
    }

    get(pool, id).await
}

/// Status transitions are owned by application code, never by the AI.
pub async fn set_status(
    pool: &SqlitePool,
    id: i64,
    status: ApplicationStatus,
) -> AppResult<Application> {
    let result =
        sqlx::query("UPDATE applications SET status = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(status.as_str())
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("application {id}")));
    }

    get(pool, id).await
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM applications WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

fn validate(input: &ApplicationInput) -> AppResult<(String, String)> {
    let company = required(&input.company, "company")?;
    let role = required(&input.role, "role")?;
    Ok((company, role))
}
