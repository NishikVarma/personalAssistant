use sqlx::SqlitePool;

use super::{now, required};
use crate::error::{AppError, AppResult};
use crate::models::profile::{ProfileEntityType, Skill, SkillInput};

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Skill>> {
    Ok(
        sqlx::query_as::<_, Skill>("SELECT * FROM skills ORDER BY name COLLATE NOCASE")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Skill> {
    sqlx::query_as::<_, Skill>("SELECT * FROM skills WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("skill {id}")))
}

pub async fn create(pool: &SqlitePool, input: &SkillInput) -> AppResult<Skill> {
    let name = required(&input.name, "name")?;
    let result = sqlx::query("INSERT INTO skills (name, category, created_at) VALUES (?1, ?2, ?3)")
        .bind(&name)
        .bind(input.category.as_str())
        .bind(now())
        .execute(pool)
        .await;

    match result {
        Ok(r) => get(pool, r.last_insert_rowid()).await,
        Err(e) if is_unique_violation(&e) => Err(AppError::InvalidInput(format!(
            "skill '{name}' already exists"
        ))),
        Err(e) => Err(e.into()),
    }
}

pub async fn update(pool: &SqlitePool, id: i64, input: &SkillInput) -> AppResult<Skill> {
    let name = required(&input.name, "name")?;
    let result = sqlx::query("UPDATE skills SET name = ?1, category = ?2 WHERE id = ?3")
        .bind(&name)
        .bind(input.category.as_str())
        .bind(id)
        .execute(pool)
        .await;
    match result {
        Ok(r) if r.rows_affected() == 0 => Err(AppError::NotFound(format!("skill {id}"))),
        Ok(_) => get(pool, id).await,
        Err(e) if is_unique_violation(&e) => Err(AppError::InvalidInput(format!(
            "skill '{name}' already exists"
        ))),
        Err(e) => Err(e.into()),
    }
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM skills WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    err.as_database_error()
        .is_some_and(|d| d.is_unique_violation())
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

/// Replaces the full skill set attached to an entity (used by the chip editor UI).
pub async fn replace_entity_skills(
    pool: &SqlitePool,
    entity_type: ProfileEntityType,
    entity_id: i64,
    skill_ids: &[i64],
) -> AppResult<()> {
    ensure_entity_exists(pool, entity_type, entity_id).await?;

    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM entity_skills WHERE entity_type = ?1 AND entity_id = ?2")
        .bind(entity_type.as_str())
        .bind(entity_id)
        .execute(&mut *tx)
        .await?;
    for skill_id in skill_ids {
        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM skills WHERE id = ?1")
                .bind(skill_id)
                .fetch_optional(&mut *tx)
                .await?;
        if exists.is_none() {
            return Err(AppError::NotFound(format!("skill {skill_id}")));
        }
        sqlx::query(
            "INSERT OR IGNORE INTO entity_skills (entity_type, entity_id, skill_id)
             VALUES (?1, ?2, ?3)",
        )
        .bind(entity_type.as_str())
        .bind(entity_id)
        .bind(skill_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn list_for_entity(
    pool: &SqlitePool,
    entity_type: ProfileEntityType,
    entity_id: i64,
) -> AppResult<Vec<Skill>> {
    Ok(sqlx::query_as::<_, Skill>(
        "SELECT s.* FROM skills s
         JOIN entity_skills es ON es.skill_id = s.id
         WHERE es.entity_type = ?1 AND es.entity_id = ?2
         ORDER BY s.name COLLATE NOCASE",
    )
    .bind(entity_type.as_str())
    .bind(entity_id)
    .fetch_all(pool)
    .await?)
}
