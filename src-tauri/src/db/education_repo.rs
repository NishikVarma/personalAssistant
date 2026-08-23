use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::profile::{Education, EducationInput};

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Education>> {
    Ok(sqlx::query_as::<_, Education>("SELECT * FROM education ORDER BY id DESC")
        .fetch_all(pool)
        .await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Education> {
    sqlx::query_as::<_, Education>("SELECT * FROM education WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("education {id}")))
}

pub async fn create(pool: &SqlitePool, input: &EducationInput) -> AppResult<Education> {
    let institution = required(&input.institution, "institution")?;
    let result = sqlx::query(
        "INSERT INTO education
             (institution, degree, field_of_study, start_date, end_date, grade, location, details,
              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
    )
    .bind(&institution)
    .bind(input.degree.trim())
    .bind(input.field_of_study.trim())
    .bind(optional(&input.start_date))
    .bind(optional(&input.end_date))
    .bind(optional(&input.grade))
    .bind(optional(&input.location))
    .bind(input.details.trim())
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

pub async fn update(pool: &SqlitePool, id: i64, input: &EducationInput) -> AppResult<Education> {
    let institution = required(&input.institution, "institution")?;
    let result = sqlx::query(
        "UPDATE education
         SET institution = ?1, degree = ?2, field_of_study = ?3, start_date = ?4, end_date = ?5,
             grade = ?6, location = ?7, details = ?8, updated_at = ?9
         WHERE id = ?10",
    )
    .bind(&institution)
    .bind(input.degree.trim())
    .bind(input.field_of_study.trim())
    .bind(optional(&input.start_date))
    .bind(optional(&input.end_date))
    .bind(optional(&input.grade))
    .bind(optional(&input.location))
    .bind(input.details.trim())
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("education {id}")));
    }

    get(pool, id).await
}

pub async fn set_verified(pool: &SqlitePool, id: i64, verified: bool) -> AppResult<()> {
    let result =
        sqlx::query("UPDATE education SET verified = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(verified)
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("education {id}")));
    }
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM education WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
