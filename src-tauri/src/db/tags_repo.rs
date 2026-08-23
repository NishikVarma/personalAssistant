use sqlx::SqlitePool;

use super::required;
use crate::error::{AppError, AppResult};
use crate::models::contact::{Tag, TagInput};

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Tag>> {
    Ok(
        sqlx::query_as::<_, Tag>("SELECT * FROM tags ORDER BY name COLLATE NOCASE")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Tag> {
    sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("tag {id}")))
}

pub async fn create(pool: &SqlitePool, input: &TagInput) -> AppResult<Tag> {
    let name = required(&input.name, "name")?;
    let result = sqlx::query("INSERT INTO tags (name, color) VALUES (?1, ?2)")
        .bind(&name)
        .bind(input.color.as_deref().filter(|c| !c.trim().is_empty()))
        .execute(pool)
        .await;

    match result {
        Ok(r) => get(pool, r.last_insert_rowid()).await,
        Err(e) if e.as_database_error().is_some_and(|d| d.is_unique_violation()) => {
            Err(AppError::InvalidInput(format!("tag '{name}' already exists")))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM tags WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Replaces the full tag set attached to a contact.
pub async fn replace_contact_tags(
    pool: &SqlitePool,
    contact_id: i64,
    tag_ids: &[i64],
) -> AppResult<()> {
    let exists: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM contacts WHERE id = ?1")
            .bind(contact_id)
            .fetch_optional(pool)
            .await?;
    if exists.is_none() {
        return Err(AppError::NotFound(format!("contact {contact_id}")));
    }

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM contact_tags WHERE contact_id = ?1")
        .bind(contact_id)
        .execute(&mut *tx)
        .await?;
    for tag_id in tag_ids {
        let found: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM tags WHERE id = ?1")
                .bind(tag_id)
                .fetch_optional(&mut *tx)
                .await?;
        if found.is_none() {
            return Err(AppError::NotFound(format!("tag {tag_id}")));
        }
        sqlx::query("INSERT OR IGNORE INTO contact_tags (contact_id, tag_id) VALUES (?1, ?2)")
            .bind(contact_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn list_for_contact(pool: &SqlitePool, contact_id: i64) -> AppResult<Vec<Tag>> {
    Ok(sqlx::query_as::<_, Tag>(
        "SELECT t.* FROM tags t
         JOIN contact_tags ct ON ct.tag_id = t.id
         WHERE ct.contact_id = ?1
         ORDER BY t.name COLLATE NOCASE",
    )
    .bind(contact_id)
    .fetch_all(pool)
    .await?)
}
