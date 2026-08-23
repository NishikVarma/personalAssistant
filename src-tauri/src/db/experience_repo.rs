use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::profile::{Experience, ExperienceInput};

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Experience>> {
    Ok(
        sqlx::query_as::<_, Experience>("SELECT * FROM experience ORDER BY id DESC")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Experience> {
    sqlx::query_as::<_, Experience>("SELECT * FROM experience WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("experience {id}")))
}

pub async fn create(pool: &SqlitePool, input: &ExperienceInput) -> AppResult<Experience> {
    let organization = required(&input.organization, "organization")?;
    let title = required(&input.title, "title")?;
    let result = sqlx::query(
        "INSERT INTO experience
             (organization, title, employment_type, location, start_date, end_date,
              currently_working, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
    )
    .bind(&organization)
    .bind(&title)
    .bind(input.employment_type.as_str())
    .bind(optional(&input.location))
    .bind(optional(&input.start_date))
    .bind(optional(&input.end_date))
    .bind(input.currently_working)
    .bind(input.description.trim())
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

pub async fn update(pool: &SqlitePool, id: i64, input: &ExperienceInput) -> AppResult<Experience> {
    let organization = required(&input.organization, "organization")?;
    let title = required(&input.title, "title")?;
    let result = sqlx::query(
        "UPDATE experience
         SET organization = ?1, title = ?2, employment_type = ?3, location = ?4,
             start_date = ?5, end_date = ?6, currently_working = ?7, description = ?8,
             updated_at = ?9
         WHERE id = ?10",
    )
    .bind(&organization)
    .bind(&title)
    .bind(input.employment_type.as_str())
    .bind(optional(&input.location))
    .bind(optional(&input.start_date))
    .bind(optional(&input.end_date))
    .bind(input.currently_working)
    .bind(input.description.trim())
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("experience {id}")));
    }

    get(pool, id).await
}

pub async fn set_verified(pool: &SqlitePool, id: i64, verified: bool) -> AppResult<()> {
    let result =
        sqlx::query("UPDATE experience SET verified = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(verified)
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("experience {id}")));
    }
    Ok(())
}

/// Deletes an experience and its dependent bullets/skill links (polymorphic tables have no FK).
pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query("DELETE FROM experience WHERE id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() > 0 {
        cleanup_entity_rows(&mut tx, "experience", id).await?;
    }
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}

pub(super) async fn cleanup_entity_rows(
    tx: &mut sqlx::SqliteConnection,
    entity_type: &str,
    entity_id: i64,
) -> AppResult<()> {
    sqlx::query("DELETE FROM bullets WHERE entity_type = ?1 AND entity_id = ?2")
        .bind(entity_type)
        .bind(entity_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM entity_skills WHERE entity_type = ?1 AND entity_id = ?2")
        .bind(entity_type)
        .bind(entity_id)
        .execute(tx)
        .await?;
    Ok(())
}
