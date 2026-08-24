use assistant_lib::db::{self, settings_repo};
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

#[tokio::test]
async fn migrations_apply_and_settings_roundtrip() {
    let (pool, _dir) = test_pool().await;

    assert!(settings_repo::get(&pool, "theme").await.unwrap().is_none());

    settings_repo::set(&pool, "theme", "dark").await.unwrap();
    assert_eq!(
        settings_repo::get(&pool, "theme").await.unwrap().as_deref(),
        Some("dark")
    );

    settings_repo::set(&pool, "theme", "light").await.unwrap();
    assert_eq!(
        settings_repo::get(&pool, "theme").await.unwrap().as_deref(),
        Some("light")
    );

    assert!(settings_repo::delete(&pool, "theme").await.unwrap());
    assert!(!settings_repo::delete(&pool, "theme").await.unwrap());
}

#[tokio::test]
async fn schema_version_matches_migration_count() {
    let (pool, _dir) = test_pool().await;
    let version = db::schema_version(&pool).await.unwrap();
    assert_eq!(version, 2);
}

#[tokio::test]
async fn foreign_keys_enforced() {
    let (pool, _dir) = test_pool().await;
    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        "INSERT INTO follow_ups (application_id, sequence, scheduled_for, created_at, updated_at)
         VALUES (9999, 1, '2026-01-01', ?1, ?1)",
    )
    .bind(&now)
    .execute(&pool)
    .await;

    assert!(result.is_err(), "FK violation should be rejected");
}

#[tokio::test]
async fn cascade_delete_removes_follow_ups() {
    let (pool, _dir) = test_pool().await;
    let now = chrono::Utc::now().to_rfc3339();

    let app = sqlx::query(
        "INSERT INTO applications (company, role, created_at, updated_at)
         VALUES ('Acme', 'Backend Engineer', ?1, ?1)",
    )
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();
    let app_id: i64 = app.last_insert_rowid();

    sqlx::query(
        "INSERT INTO follow_ups (application_id, sequence, scheduled_for, created_at, updated_at)
         VALUES (?1, 1, '2026-09-01', ?2, ?2)",
    )
    .bind(app_id)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM follow_ups WHERE application_id = ?1")
            .bind(app_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count_before, 1);

    sqlx::query("DELETE FROM applications WHERE id = ?1")
        .bind(app_id)
        .execute(&pool)
        .await
        .unwrap();

    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM follow_ups WHERE application_id = ?1")
            .bind(app_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count_after, 0);
}
