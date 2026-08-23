use sqlx::SqlitePool;

use super::{now, required};
use crate::error::{AppError, AppResult};
use crate::models::profile::{Link, LinkInput};

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Link>> {
    Ok(sqlx::query_as::<_, Link>("SELECT * FROM links ORDER BY id")
        .fetch_all(pool)
        .await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Link> {
    sqlx::query_as::<_, Link>("SELECT * FROM links WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("link {id}")))
}

pub async fn create(pool: &SqlitePool, input: &LinkInput) -> AppResult<Link> {
    let label = required(&input.label, "label")?;
    let url = required(&input.url, "url")?;
    let result =
        sqlx::query("INSERT INTO links (label, url, kind, created_at) VALUES (?1, ?2, ?3, ?4)")
            .bind(&label)
            .bind(&url)
            .bind(input.kind.as_str())
            .bind(now())
            .execute(pool)
            .await?;

    get(pool, result.last_insert_rowid()).await
}

pub async fn update(pool: &SqlitePool, id: i64, input: &LinkInput) -> AppResult<Link> {
    let label = required(&input.label, "label")?;
    let url = required(&input.url, "url")?;
    let result = sqlx::query("UPDATE links SET label = ?1, url = ?2, kind = ?3 WHERE id = ?4")
        .bind(&label)
        .bind(&url)
        .bind(input.kind.as_str())
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("link {id}")));
    }

    get(pool, id).await
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM links WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
