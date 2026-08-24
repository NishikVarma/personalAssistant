use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::email::{EmailHistory, HistoryFilter, IncomingEmailInput, ResponseStatus};

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

pub async fn list(pool: &SqlitePool, filter: &HistoryFilter) -> AppResult<Vec<EmailHistory>> {
    let mut qb = sqlx::QueryBuilder::new("SELECT * FROM email_history WHERE 1=1");
    if let Some(contact_id) = filter.contact_id {
        qb.push(" AND contact_id = ").push_bind(contact_id);
    }
    if let Some(application_id) = filter.application_id {
        qb.push(" AND application_id = ").push_bind(application_id);
    }
    qb.push(" ORDER BY occurred_at DESC, id DESC LIMIT ").push_bind(filter.limit.unwrap_or(100));
    Ok(qb.build_query_as::<EmailHistory>().fetch_all(pool).await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<EmailHistory> {
    sqlx::query_as::<_, EmailHistory>("SELECT * FROM email_history WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("email history {id}")))
}

pub async fn set_response_status(
    pool: &SqlitePool,
    id: i64,
    status: Option<ResponseStatus>,
) -> AppResult<EmailHistory> {
    let result =
        sqlx::query("UPDATE email_history SET response_status = ?1 WHERE id = ?2 AND direction = 'outgoing'")
            .bind(status.map(|s| s.as_str()))
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("outgoing email history {id}")));
    }
    get(pool, id).await
}

async fn ensure_linked_rows_exist(
    pool: &SqlitePool,
    contact_id: Option<i64>,
    application_id: Option<i64>,
) -> AppResult<()> {
    if let Some(id) = contact_id {
        let found: Option<(i64,)> = sqlx::query_as("SELECT id FROM contacts WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        if found.is_none() {
            return Err(AppError::NotFound(format!("contact {id}")));
        }
    }
    if let Some(id) = application_id {
        let found: Option<(i64,)> = sqlx::query_as("SELECT id FROM applications WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        if found.is_none() {
            return Err(AppError::NotFound(format!("application {id}")));
        }
    }
    Ok(())
}

pub async fn record_incoming(
    pool: &SqlitePool,
    input: &IncomingEmailInput,
) -> AppResult<EmailHistory> {
    ensure_linked_rows_exist(pool, input.contact_id, input.application_id).await?;
    let sender = required(&input.sender_email, "sender_email")?;
    let occurred_at = optional(&input.occurred_at).unwrap_or_else(now);
    let result = sqlx::query(
        "INSERT INTO email_history
             (direction, application_id, contact_id, email_type, recipient_email, subject,
              body, delivery_method, status, occurred_at, created_at)
         VALUES ('incoming', ?1, ?2, ?3, ?4, ?5, ?6, 'manual', 'received', ?7, ?8)",
    )
    .bind(input.application_id)
    .bind(input.contact_id)
    .bind(input.email_type.as_ref().map(|t| t.as_str()))
    .bind(&sender)
    .bind(optional(&input.subject))
    .bind(input.body.trim())
    .bind(&occurred_at)
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

/// Outgoing sent emails still awaiting a response that carry a Gmail thread —
/// the input set for reply sync.
pub async fn awaiting_with_threads(
    pool: &SqlitePool,
    limit: i64,
) -> AppResult<Vec<EmailHistory>> {
    Ok(sqlx::query_as::<_, EmailHistory>(
        "SELECT * FROM email_history
         WHERE direction = 'outgoing' AND status = 'sent'
           AND response_status = 'awaiting' AND gmail_thread_id IS NOT NULL
         ORDER BY occurred_at DESC LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

/// Records a detected reply and flips the originating sent row to `replied`
/// in one transaction.
pub async fn record_reply(
    pool: &SqlitePool,
    sent: &EmailHistory,
    reply: &super::super::gmail::ParsedReply,
) -> AppResult<()> {
    let ts = now();
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO email_history
             (direction, application_id, contact_id, generated_email_id, gmail_message_id,
              gmail_thread_id, email_type, recipient_email, subject, body, delivery_method,
              status, occurred_at, created_at)
         VALUES ('incoming', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'gmail_api', 'received', ?10, ?11)",
    )
    .bind(sent.application_id)
    .bind(sent.contact_id)
    .bind(sent.generated_email_id)
    .bind(&reply.gmail_message_id)
    .bind(&sent.gmail_thread_id)
    .bind(&sent.email_type)
    .bind(&reply.from_email)
    .bind(&reply.subject)
    .bind(reply.snippet.as_deref().unwrap_or(""))
    .bind(&reply.occurred_at)
    .bind(&ts)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE email_history SET response_status = 'replied' WHERE id = ?1")
        .bind(sent.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}
