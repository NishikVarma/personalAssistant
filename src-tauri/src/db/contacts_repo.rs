use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::contact::{Contact, ContactInput};

pub async fn list(pool: &SqlitePool, search: &str) -> AppResult<Vec<Contact>> {
    let term = search.trim();
    Ok(sqlx::query_as::<_, Contact>(
        "SELECT * FROM contacts
         WHERE (?1 = ''
               OR name LIKE '%' || ?1 || '%'
               OR email LIKE '%' || ?1 || '%'
               OR IFNULL(organization, '') LIKE '%' || ?1 || '%')
         ORDER BY name COLLATE NOCASE, email COLLATE NOCASE",
    )
    .bind(term)
    .fetch_all(pool)
    .await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Contact> {
    sqlx::query_as::<_, Contact>("SELECT * FROM contacts WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("contact {id}")))
}

/// Looks up a contact id by exact email (case-insensitive) without creating one.
pub async fn find_id_by_email(pool: &SqlitePool, email: &str) -> AppResult<Option<i64>> {
    let found: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM contacts WHERE email = ?1 COLLATE NOCASE")
            .bind(email.trim())
            .fetch_optional(pool)
            .await?;
    Ok(found.map(|(id,)| id))
}

fn validate(input: &ContactInput) -> AppResult<(String, String)> {
    let email = required(&input.email, "email")?;
    if !email.contains('@') {
        return Err(AppError::InvalidInput(
            "email must be a valid address".to_string(),
        ));
    }
    Ok((input.name.trim().to_string(), email))
}

pub async fn create(pool: &SqlitePool, input: &ContactInput) -> AppResult<Contact> {
    let (name, email) = validate(input)?;
    let result = sqlx::query(
        "INSERT INTO contacts
             (name, email, organization, role_title, linkedin_url, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
    )
    .bind(&name)
    .bind(&email)
    .bind(optional(&input.organization))
    .bind(optional(&input.role_title))
    .bind(optional(&input.linkedin_url))
    .bind(input.notes.trim())
    .bind(now())
    .execute(pool)
    .await;

    match result {
        Ok(r) => get(pool, r.last_insert_rowid()).await,
        Err(e) if e.as_database_error().is_some_and(|d| d.is_unique_violation()) => {
            Err(AppError::InvalidInput(format!(
                "a contact with email '{email}' already exists"
            )))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn update(pool: &SqlitePool, id: i64, input: &ContactInput) -> AppResult<Contact> {
    let (name, email) = validate(input)?;
    let result = sqlx::query(
        "UPDATE contacts
         SET name = ?1, email = ?2, organization = ?3, role_title = ?4, linkedin_url = ?5,
             notes = ?6, updated_at = ?7
         WHERE id = ?8",
    )
    .bind(&name)
    .bind(&email)
    .bind(optional(&input.organization))
    .bind(optional(&input.role_title))
    .bind(optional(&input.linkedin_url))
    .bind(input.notes.trim())
    .bind(now())
    .bind(id)
    .execute(pool)
    .await;
    match result {
        Ok(r) if r.rows_affected() == 0 => Err(AppError::NotFound(format!("contact {id}"))),
        Ok(_) => get(pool, id).await,
        Err(e) if e.as_database_error().is_some_and(|d| d.is_unique_violation()) => {
            Err(AppError::InvalidInput(format!(
                "a contact with email '{email}' already exists"
            )))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn set_last_contacted(
    pool: &SqlitePool,
    id: i64,
    last_contacted_at: Option<String>,
) -> AppResult<()> {
    let result =
        sqlx::query("UPDATE contacts SET last_contacted_at = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(optional(&last_contacted_at))
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("contact {id}")));
    }
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM contacts WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
