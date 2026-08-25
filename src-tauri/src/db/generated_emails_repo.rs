use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::email::{EmailStatus, GeneratedEmail, GeneratedEmailInput};

pub async fn list(
    pool: &SqlitePool,
    status: Option<EmailStatus>,
) -> AppResult<Vec<GeneratedEmail>> {
    match status {
        Some(status) => Ok(sqlx::query_as::<_, GeneratedEmail>(
            "SELECT * FROM generated_emails WHERE status = ?1 ORDER BY id DESC",
        )
        .bind(status.as_str())
        .fetch_all(pool)
        .await?),
        None => Ok(sqlx::query_as::<_, GeneratedEmail>(
            "SELECT * FROM generated_emails ORDER BY id DESC",
        )
        .fetch_all(pool)
        .await?),
    }
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<GeneratedEmail> {
    sqlx::query_as::<_, GeneratedEmail>("SELECT * FROM generated_emails WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("generated email {id}")))
}

async fn ensure_linked_rows_exist(
    pool: &SqlitePool,
    input: &GeneratedEmailInput,
) -> AppResult<()> {
    if let Some(app_id) = input.application_id {
        let found: Option<(i64,)> = sqlx::query_as("SELECT id FROM applications WHERE id = ?1")
            .bind(app_id)
            .fetch_optional(pool)
            .await?;
        if found.is_none() {
            return Err(AppError::NotFound(format!("application {app_id}")));
        }
    }
    if let Some(contact_id) = input.contact_id {
        let found: Option<(i64,)> = sqlx::query_as("SELECT id FROM contacts WHERE id = ?1")
            .bind(contact_id)
            .fetch_optional(pool)
            .await?;
        if found.is_none() {
            return Err(AppError::NotFound(format!("contact {contact_id}")));
        }
    }
    Ok(())
}

fn validate_content(input: &GeneratedEmailInput) -> AppResult<String> {
    required(&input.body, "body")
}

pub async fn create(pool: &SqlitePool, input: &GeneratedEmailInput) -> AppResult<GeneratedEmail> {
    ensure_linked_rows_exist(pool, input).await?;
    let body = validate_content(input)?;
    let result = sqlx::query(
        "INSERT INTO generated_emails
             (application_id, contact_id, email_type, recipient_email, recipient_name,
              subject, body, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'draft', ?8, ?8)",
    )
    .bind(input.application_id)
    .bind(input.contact_id)
    .bind(input.email_type.as_str())
    .bind(optional(&input.recipient_email))
    .bind(optional(&input.recipient_name))
    .bind(optional(&input.subject))
    .bind(&body)
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

/// Saves edited subject/body. Editing a fresh draft marks it as `edited`.
pub async fn update_content(
    pool: &SqlitePool,
    id: i64,
    subject: Option<String>,
    body: String,
) -> AppResult<GeneratedEmail> {
    let body = required(&body, "body")?;
    let current = get(pool, id).await?;
    let next_status = if current.status == EmailStatus::Draft.as_str() {
        EmailStatus::Edited.as_str()
    } else {
        current.status.as_str()
    };
    sqlx::query(
        "UPDATE generated_emails
         SET subject = ?1, body = ?2, status = ?3, updated_at = ?4
         WHERE id = ?5",
    )
    .bind(optional(&subject))
    .bind(&body)
    .bind(next_status)
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;

    get(pool, id).await
}

/// Links a generated draft back to the scheduled follow-up it fulfils.
pub async fn set_follow_up_link(
    pool: &SqlitePool,
    generated_email_id: i64,
    follow_up_id: i64,
) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE generated_emails SET follow_up_id = ?1 WHERE id = ?2 AND status = 'draft'",
    )
    .bind(follow_up_id)
    .bind(generated_email_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "draft {generated_email_id} (must exist and still be a draft)"
        )));
    }
    Ok(())
}

pub async fn set_status(
    pool: &SqlitePool,
    id: i64,
    status: EmailStatus,
) -> AppResult<GeneratedEmail> {
    let current = get(pool, id).await?;
    let from = EmailStatus::try_from_str(&current.status)
        .ok_or_else(|| AppError::InvalidInput(format!("unknown email status '{}'", current.status)))?;
    if !from.can_transition_to(status) {
        return Err(AppError::InvalidInput(format!(
            "cannot move email from '{}' to '{}'",
            from.as_str(),
            status.as_str()
        )));
    }

    sqlx::query("UPDATE generated_emails SET status = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(status.as_str())
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;

    get(pool, id).await
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM generated_emails WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// All drafts belonging to a bulk batch, oldest first.
pub async fn list_by_batch(pool: &SqlitePool, batch_id: i64) -> AppResult<Vec<GeneratedEmail>> {
    Ok(sqlx::query_as::<_, GeneratedEmail>(
        "SELECT * FROM generated_emails WHERE bulk_batch_id = ?1 ORDER BY id",
    )
    .bind(batch_id)
    .fetch_all(pool)
    .await?)
}

pub async fn count_by_batch(pool: &SqlitePool, batch_id: i64) -> AppResult<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM generated_emails WHERE bulk_batch_id = ?1",
    )
    .bind(batch_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// Attaches a draft to a bulk batch (draft status only).
pub async fn set_bulk_batch_link(
    pool: &SqlitePool,
    generated_email_id: i64,
    batch_id: i64,
) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE generated_emails SET bulk_batch_id = ?1 WHERE id = ?2 AND status = 'draft'",
    )
    .bind(batch_id)
    .bind(generated_email_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "draft {generated_email_id} (must exist and still be a draft)"
        )));
    }
    Ok(())
}
