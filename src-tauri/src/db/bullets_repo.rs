use sqlx::SqlitePool;

use super::{now, required};
use crate::error::{AppError, AppResult};
use crate::models::profile::{Bullet, BulletInput, ProfileEntityType};

pub async fn list_for_entity(
    pool: &SqlitePool,
    entity_type: ProfileEntityType,
    entity_id: i64,
) -> AppResult<Vec<Bullet>> {
    Ok(sqlx::query_as::<_, Bullet>(
        "SELECT * FROM bullets
         WHERE entity_type = ?1 AND entity_id = ?2
         ORDER BY display_order, id",
    )
    .bind(entity_type.as_str())
    .bind(entity_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Bullet> {
    sqlx::query_as::<_, Bullet>("SELECT * FROM bullets WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("bullet {id}")))
}

pub async fn create(
    pool: &SqlitePool,
    entity_type: ProfileEntityType,
    entity_id: i64,
    input: &BulletInput,
) -> AppResult<Bullet> {
    ensure_entity_exists(pool, entity_type, entity_id).await?;
    let content = required(&input.content, "content")?;
    let result = sqlx::query(
        "INSERT INTO bullets (entity_type, entity_id, content, display_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
    )
    .bind(entity_type.as_str())
    .bind(entity_id)
    .bind(&content)
    .bind(input.display_order)
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

pub async fn update(pool: &SqlitePool, id: i64, input: &BulletInput) -> AppResult<Bullet> {
    let content = required(&input.content, "content")?;
    let result =
        sqlx::query("UPDATE bullets SET content = ?1, display_order = ?2, updated_at = ?3 WHERE id = ?4")
            .bind(&content)
            .bind(input.display_order)
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("bullet {id}")));
    }

    get(pool, id).await
}

pub async fn set_verified(pool: &SqlitePool, id: i64, verified: bool) -> AppResult<()> {
    let result =
        sqlx::query("UPDATE bullets SET verified = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(verified)
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("bullet {id}")));
    }
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM bullets WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

async fn ensure_entity_exists(
    pool: &SqlitePool,
    entity_type: ProfileEntityType,
    entity_id: i64,
) -> AppResult<()> {
    let table = entity_type.as_str();
    let found: Option<(i64,)> = match entity_type {
        ProfileEntityType::Project => {
            sqlx::query_as("SELECT id FROM projects WHERE id = ?1")
                .bind(entity_id)
                .fetch_optional(pool)
                .await?
        }
        ProfileEntityType::Experience => {
            sqlx::query_as("SELECT id FROM experience WHERE id = ?1")
                .bind(entity_id)
                .fetch_optional(pool)
                .await?
        }
    };
    found
        .map(|_| ())
        .ok_or_else(|| AppError::NotFound(format!("{table} {entity_id}")))
}
