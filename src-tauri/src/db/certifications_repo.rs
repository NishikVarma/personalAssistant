use sqlx::SqlitePool;

use super::{now, optional, required};
use crate::error::{AppError, AppResult};
use crate::models::profile::{Certification, CertificationInput};

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Certification>> {
    Ok(sqlx::query_as::<_, Certification>(
        "SELECT * FROM certifications ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Certification> {
    sqlx::query_as::<_, Certification>("SELECT * FROM certifications WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("certification {id}")))
}

pub async fn create(
    pool: &SqlitePool,
    input: &CertificationInput,
) -> AppResult<Certification> {
    let name = required(&input.name, "name")?;
    let result = sqlx::query(
        "INSERT INTO certifications
             (name, issuer, issue_date, expiry_date, credential_url, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
    )
    .bind(&name)
    .bind(input.issuer.trim())
    .bind(optional(&input.issue_date))
    .bind(optional(&input.expiry_date))
    .bind(optional(&input.credential_url))
    .bind(now())
    .execute(pool)
    .await?;

    get(pool, result.last_insert_rowid()).await
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &CertificationInput,
) -> AppResult<Certification> {
    let name = required(&input.name, "name")?;
    let result = sqlx::query(
        "UPDATE certifications
         SET name = ?1, issuer = ?2, issue_date = ?3, expiry_date = ?4, credential_url = ?5,
             updated_at = ?6
         WHERE id = ?7",
    )
    .bind(&name)
    .bind(input.issuer.trim())
    .bind(optional(&input.issue_date))
    .bind(optional(&input.expiry_date))
    .bind(optional(&input.credential_url))
    .bind(now())
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("certification {id}")));
    }

    get(pool, id).await
}

pub async fn set_verified(pool: &SqlitePool, id: i64, verified: bool) -> AppResult<()> {
    let result =
        sqlx::query("UPDATE certifications SET verified = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(verified)
            .bind(now())
            .bind(id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("certification {id}")));
    }
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM certifications WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
