use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::email::{EmailTemplate, EmailTemplateInput, EmailType};

pub async fn list(
    pool: &SqlitePool,
    email_type: Option<EmailType>,
) -> AppResult<Vec<EmailTemplate>> {
    match email_type {
        Some(t) => Ok(sqlx::query_as::<_, EmailTemplate>(
            "SELECT * FROM email_templates WHERE email_type = ?1 ORDER BY id DESC",
        )
        .bind(t.as_str())
        .fetch_all(pool)
        .await?),
        None => Ok(
            sqlx::query_as::<_, EmailTemplate>("SELECT * FROM email_templates ORDER BY id DESC")
                .fetch_all(pool)
                .await?,
        ),
    }
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<EmailTemplate> {
    sqlx::query_as::<_, EmailTemplate>("SELECT * FROM email_templates WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("email template {id}")))
}

fn validate(input: &EmailTemplateInput) -> AppResult<String> {
    required(&input.body_template, "body_template")
}

pub async fn create(
    pool: &SqlitePool,
    input: &EmailTemplateInput,
    source: &str,
) -> AppResult<EmailTemplate> {
    let body = validate(input)?;
    let result = sqlx::query(
        "INSERT INTO email_templates
             (email_type, role, company_or_industry, subject_template, body_template,
              variables_json, source, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, '[]', ?6, ?7, ?7)",
    )
    .bind(input.email_type.as_str())
    .bind(optional(&input.role))
    .bind(optional(&input.company_or_industry))
    .bind(optional(&input.subject_template))
    .bind(&body)
    .bind(source)
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &EmailTemplateInput,
) -> AppResult<EmailTemplate> {
    let body = validate(input)?;
    let result = sqlx::query(
        "UPDATE email_templates
         SET email_type = ?1, role = ?2, company_or_industry = ?3, subject_template = ?4,
             body_template = ?5, updated_at = ?6
         WHERE id = ?7",
    )
    .bind(input.email_type.as_str())
    .bind(optional(&input.role))
    .bind(optional(&input.company_or_industry))
    .bind(optional(&input.subject_template))
    .bind(&body)
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("email template {id}")));
    }

    get(pool, id).await
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM email_templates WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn mark_used(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE email_templates
         SET times_used = times_used + 1, last_used_at = ?1
         WHERE id = ?2",
    )
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("email template {id}")));
    }
    Ok(())
}

/// Deterministic pick for generation: company match beats role match beats
/// usage count. Ties resolve to the later element (highest id from the query).
pub fn choose_template(
    templates: &[EmailTemplate],
    role: Option<&str>,
    company: Option<&str>,
) -> Option<EmailTemplate> {
    let score = |t: &EmailTemplate| -> (i32, i64) {
        let company_score = match (&t.company_or_industry, company) {
            (Some(tc), Some(c)) if tc.eq_ignore_ascii_case(c.trim()) => 4,
            _ => 0,
        };
        let role_score = match (&t.role, role) {
            (Some(tr), Some(r)) if tr.eq_ignore_ascii_case(r.trim()) => 2,
            _ => 0,
        };
        (company_score + role_score, t.times_used)
    };

    templates
        .iter()
        .enumerate()
        .max_by(|(ia, a), (ib, b)| score(a).cmp(&score(b)).then(ib.cmp(ia)))
        .map(|(_, t)| t.clone())
}
