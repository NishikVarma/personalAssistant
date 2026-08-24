use sqlx::SqlitePool;

use super::now;
use crate::error::AppResult;

/// Default window for the duplicate-outreach guard.
pub const RECENT_OUTREACH_DAYS: i64 = 7;

pub struct SentRecord {
    pub generated_email_id: i64,
    pub contact_id: Option<i64>,
    pub recipient_email: String,
    pub email_type: String,
    pub subject: Option<String>,
    pub body: String,
    pub gmail_message_id: String,
    pub gmail_thread_id: Option<String>,
}

/// Records a successful send and flips the draft to `sent` in one transaction,
/// also stamping the contact's last-contacted timestamp when linked.
pub async fn record_sent(pool: &SqlitePool, record: &SentRecord) -> AppResult<()> {
    let ts = now();
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO email_history
             (direction, application_id, contact_id, generated_email_id, gmail_message_id,
              gmail_thread_id, email_type, recipient_email, subject, body, delivery_method,
              status, response_status, occurred_at, created_at)
         VALUES ('outgoing',
                 (SELECT application_id FROM generated_emails WHERE id = ?1),
                 ?2, ?1, ?3, ?4, ?5, ?6, ?7, ?8, 'gmail_api', 'sent', 'awaiting', ?9, ?9)",
    )
    .bind(record.generated_email_id)
    .bind(record.contact_id)
    .bind(&record.gmail_message_id)
    .bind(&record.gmail_thread_id)
    .bind(&record.email_type)
    .bind(&record.recipient_email)
    .bind(&record.subject)
    .bind(&record.body)
    .bind(&ts)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE generated_emails SET status = 'sent', updated_at = ?1 WHERE id = ?2")
        .bind(&ts)
        .bind(record.generated_email_id)
        .execute(&mut *tx)
        .await?;

    if let Some(contact_id) = record.contact_id {
        sqlx::query("UPDATE contacts SET last_contacted_at = ?1, updated_at = ?1 WHERE id = ?2")
            .bind(&ts)
            .bind(contact_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// True when an email was already sent to this address within the guard window.
/// RFC3339 timestamps sort lexicographically, so string comparison is safe.
pub async fn has_recent_outgoing(
    pool: &SqlitePool,
    recipient_email: &str,
    days: i64,
) -> AppResult<bool> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM email_history
         WHERE direction = 'outgoing' AND status = 'sent'
           AND recipient_email = ?1 COLLATE NOCASE AND occurred_at >= ?2",
    )
    .bind(recipient_email.trim())
    .bind(&cutoff)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}
