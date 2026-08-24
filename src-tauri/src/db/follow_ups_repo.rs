use sqlx::SqlitePool;

use super::now;
use crate::error::{AppError, AppResult};
use crate::models::follow_up::{FollowUp, FollowUpStatus};

pub async fn list(pool: &SqlitePool, status: Option<FollowUpStatus>) -> AppResult<Vec<FollowUp>> {
    match status {
        Some(status) => Ok(sqlx::query_as::<_, FollowUp>(
            "SELECT * FROM follow_ups WHERE status = ?1 ORDER BY scheduled_for, id",
        )
        .bind(status.as_str())
        .fetch_all(pool)
        .await?),
        None => Ok(
            sqlx::query_as::<_, FollowUp>("SELECT * FROM follow_ups ORDER BY scheduled_for, id")
                .fetch_all(pool)
                .await?,
        ),
    }
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<FollowUp> {
    sqlx::query_as::<_, FollowUp>("SELECT * FROM follow_ups WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("follow-up {id}")))
}

pub async fn create(
    pool: &SqlitePool,
    application_id: i64,
    contact_id: Option<i64>,
    originating_email_id: Option<i64>,
    sequence: i64,
    scheduled_for: String,
) -> AppResult<FollowUp> {
    let application_exists: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM applications WHERE id = ?1")
            .bind(application_id)
            .fetch_optional(pool)
            .await?;
    if application_exists.is_none() {
        return Err(AppError::NotFound(format!("application {application_id}")));
    }
    if let Some(email_id) = originating_email_id {
        let email_exists: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM email_history WHERE id = ?1")
                .bind(email_id)
                .fetch_optional(pool)
                .await?;
        if email_exists.is_none() {
            return Err(AppError::NotFound(format!("email history {email_id}")));
        }
    }

    let ts = now();
    let result = sqlx::query(
        "INSERT INTO follow_ups
             (application_id, contact_id, originating_email_id, sequence, scheduled_for,
              status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?6)",
    )
    .bind(application_id)
    .bind(contact_id)
    .bind(originating_email_id)
    .bind(sequence)
    .bind(scheduled_for.trim())
    .bind(&ts)
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

/// Pending follow-ups whose date has arrived, flipped to `due` in one sweep.
pub async fn list_due(pool: &SqlitePool) -> AppResult<Vec<FollowUp>> {
    let ts = now();
    sqlx::query(
        "UPDATE follow_ups SET status = 'due', updated_at = ?1
         WHERE status = 'pending' AND scheduled_for <= ?2",
    )
    .bind(&ts)
    .bind(&ts)
    .execute(pool)
    .await?;

    Ok(sqlx::query_as::<_, FollowUp>(
        "SELECT * FROM follow_ups WHERE status = 'due' ORDER BY scheduled_for, id",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn due_count(pool: &SqlitePool) -> AppResult<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM follow_ups WHERE status IN ('pending','due') AND scheduled_for <= ?1",
    )
    .bind(now())
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn reschedule(pool: &SqlitePool, id: i64, scheduled_for: String) -> AppResult<FollowUp> {
    let current = get(pool, id).await?;
    if matches!(
        FollowUpStatus::try_from_str(&current.status),
        Some(FollowUpStatus::Sent) | Some(FollowUpStatus::Cancelled)
    ) {
        return Err(AppError::InvalidInput(format!(
            "cannot reschedule a {} follow-up",
            current.status
        )));
    }
    sqlx::query("UPDATE follow_ups SET scheduled_for = ?1, status = 'pending', updated_at = ?2 WHERE id = ?3")
        .bind(scheduled_for.trim())
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    get(pool, id).await
}

pub async fn cancel(pool: &SqlitePool, id: i64) -> AppResult<FollowUp> {
    let result =
        sqlx::query("UPDATE follow_ups SET status = 'cancelled', updated_at = ?1 WHERE id = ?2")
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("follow-up {id}")));
    }
    get(pool, id).await
}

pub async fn mark_sent(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE follow_ups SET status = 'sent', completed_at = ?1, updated_at = ?1 WHERE id = ?2",
    )
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("follow-up {id}")));
    }
    Ok(())
}

/// Suppresses all pending/due follow-ups for an application (or one contact's
/// rows within it) with a reason. Returns how many rows were suppressed.
pub async fn suppress_for(
    pool: &SqlitePool,
    application_id: i64,
    contact_id: Option<i64>,
    reason: &str,
) -> AppResult<u64> {
    let query = match contact_id {
        Some(_) => {
            "UPDATE follow_ups
             SET status = 'suppressed', suppressed_reason = ?1, updated_at = ?2
             WHERE application_id = ?3 AND contact_id = ?4 AND status IN ('pending','due')"
        }
        None => {
            "UPDATE follow_ups
             SET status = 'suppressed', suppressed_reason = ?1, updated_at = ?2
             WHERE application_id = ?3 AND status IN ('pending','due')"
        }
    };
    let result = sqlx::query(query)
        .bind(reason)
        .bind(now())
        .bind(application_id)
        .bind(contact_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Deterministic sweep: suppress pending/due follow-ups whose application is
/// closed (rejected/withdrawn) or whose thread got a reply. Returns rows changed.
pub async fn apply_suppressions(pool: &SqlitePool) -> AppResult<u64> {
    let mut suppressed: u64 = 0;

    // closed applications
    let closed: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM applications WHERE status IN ('rejected','withdrawn')",
    )
    .fetch_all(pool)
    .await?;
    for (application_id,) in closed {
        suppressed += suppress_for(pool, application_id, None, "application closed").await?;
    }

    // contacts who replied after the originating send
    let replied: Vec<(i64, Option<i64>)> = sqlx::query_as(
        "SELECT DISTINCT f.application_id, f.contact_id
         FROM follow_ups f
         JOIN email_history sent ON sent.id = f.originating_email_id
         WHERE f.status IN ('pending','due')
           AND EXISTS (
             SELECT 1 FROM email_history inc
             WHERE inc.direction = 'incoming'
               AND inc.occurred_at > sent.occurred_at
               AND (inc.contact_id IS NOT NULL AND inc.contact_id = f.contact_id
                    OR (inc.contact_id IS NULL AND inc.recipient_email IS NOT NULL
                        AND inc.recipient_email = sent.recipient_email))
           )",
    )
    .fetch_all(pool)
    .await?;
    for (application_id, contact_id) in replied {
        suppressed +=
            suppress_for(pool, application_id, contact_id, "contact replied").await?;
    }

    Ok(suppressed)
}
