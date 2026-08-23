use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::profile::{Achievement, AchievementInput};

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Achievement>> {
    Ok(sqlx::query_as::<_, Achievement>(
        "SELECT * FROM achievements ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Achievement> {
    sqlx::query_as::<_, Achievement>("SELECT * FROM achievements WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("achievement {id}")))
}

pub async fn create(pool: &SqlitePool, input: &AchievementInput) -> AppResult<Achievement> {
    let title = required(&input.title, "title")?;
    let result = sqlx::query(
        "INSERT INTO achievements (title, description, date, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
    )
    .bind(&title)
    .bind(input.description.trim())
    .bind(optional(&input.date))
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &AchievementInput,
) -> AppResult<Achievement> {
    let title = required(&input.title, "title")?;
    let result = sqlx::query(
        "UPDATE achievements
         SET title = ?1, description = ?2, date = ?3, updated_at = ?4
         WHERE id = ?5",
    )
    .bind(&title)
    .bind(input.description.trim())
    .bind(optional(&input.date))
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("achievement {id}")));
    }

    get(pool, id).await
}

pub async fn set_verified(pool: &SqlitePool, id: i64, verified: bool) -> AppResult<()> {
    let result =
        sqlx::query("UPDATE achievements SET verified = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(verified)
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("achievement {id}")));
    }
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM achievements WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
