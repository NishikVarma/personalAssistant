use sqlx::SqlitePool;

use super::now;
use crate::error::{AppError, AppResult};
use crate::models::profile::{UserProfile, UserProfileInput};

const ROW_ID: i64 = 1;

pub async fn get(pool: &SqlitePool) -> AppResult<UserProfile> {
    ensure_row(pool).await?;
    Ok(
        sqlx::query_as::<_, UserProfile>("SELECT * FROM user_profile WHERE id = ?1")
            .bind(ROW_ID)
            .fetch_one(pool)
            .await?,
    )
}

async fn ensure_row(pool: &SqlitePool) -> AppResult<()> {
    let ts = now();
    sqlx::query(
        "INSERT INTO user_profile (id, created_at, updated_at) VALUES (?1, ?2, ?2)
         ON CONFLICT(id) DO NOTHING",
    )
    .bind(ROW_ID)
    .bind(&ts)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update(pool: &SqlitePool, input: &UserProfileInput) -> AppResult<UserProfile> {
    let full_name = input.full_name.trim();
    let email = input.email.trim();
    if !email.is_empty() && !email.contains('@') {
        return Err(AppError::InvalidInput(
            "email must be a valid address".to_string(),
        ));
    }

    ensure_row(pool).await?;
    sqlx::query(
        "UPDATE user_profile
         SET full_name = ?1, email = ?2, phone = ?3, location = ?4, summary = ?5, updated_at = ?6
         WHERE id = ?7",
    )
    .bind(full_name)
    .bind(email)
    .bind(input.phone.trim())
    .bind(input.location.trim())
    .bind(input.summary.trim())
    .bind(now())
    .bind(ROW_ID)
    .execute(pool)
    .await?;

    get(pool).await
}

pub async fn set_verified(pool: &SqlitePool, verified: bool) -> AppResult<()> {
    let result =
        sqlx::query("UPDATE user_profile SET verified = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(verified)
            .bind(now())
            .bind(ROW_ID)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("user_profile".to_string()));
    }
    Ok(())
}
