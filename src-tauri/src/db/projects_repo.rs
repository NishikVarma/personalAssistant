use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::profile::{Project, ProjectInput};

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Project>> {
    Ok(sqlx::query_as::<_, Project>("SELECT * FROM projects ORDER BY id DESC")
        .fetch_all(pool)
        .await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Project> {
    sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {id}")))
}

pub async fn create(pool: &SqlitePool, input: &ProjectInput) -> AppResult<Project> {
    let name = required(&input.name, "name")?;
    let result = sqlx::query(
        "INSERT INTO projects
             (name, description, repo_url, live_url, status, started_on, ended_on,
              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
    )
    .bind(&name)
    .bind(input.description.trim())
    .bind(optional(&input.repo_url))
    .bind(optional(&input.live_url))
    .bind(input.status.as_str())
    .bind(optional(&input.started_on))
    .bind(optional(&input.ended_on))
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

pub async fn update(pool: &SqlitePool, id: i64, input: &ProjectInput) -> AppResult<Project> {
    let name = required(&input.name, "name")?;
    let result = sqlx::query(
        "UPDATE projects
         SET name = ?1, description = ?2, repo_url = ?3, live_url = ?4, status = ?5,
             started_on = ?6, ended_on = ?7, updated_at = ?8
         WHERE id = ?9",
    )
    .bind(&name)
    .bind(input.description.trim())
    .bind(optional(&input.repo_url))
    .bind(optional(&input.live_url))
    .bind(input.status.as_str())
    .bind(optional(&input.started_on))
    .bind(optional(&input.ended_on))
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("project {id}")));
    }

    get(pool, id).await
}

pub async fn set_verified(pool: &SqlitePool, id: i64, verified: bool) -> AppResult<()> {
    let result =
        sqlx::query("UPDATE projects SET verified = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(verified)
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("project {id}")));
    }
    Ok(())
}

/// Deletes a project and its dependent bullets/skill links (polymorphic tables have no FK).
pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let mut tx = pool.begin().await?;
    let result = sqlx::query("DELETE FROM projects WHERE id = ?1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() > 0 {
        super::experience_repo::cleanup_entity_rows(&mut tx, "project", id).await?;
    }
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}
