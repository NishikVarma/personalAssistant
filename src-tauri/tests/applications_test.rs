use assistant_lib::db::{self, applications_repo};
use assistant_lib::models::application::*;
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

fn app_input(company: &str, role: &str) -> ApplicationInput {
    ApplicationInput {
        company: company.to_string(),
        role: role.to_string(),
        job_description: "Build backend services in Rust.".to_string(),
        job_url: Some("https://jobs.acme.com/123".to_string()),
        source: Some("referral".to_string()),
        date_discovered: Some("2026-08-01".to_string()),
        date_applied: None,
        follow_up_date: None,
        interview_status: None,
        priority: 2,
        notes: "Referred by Jane.".to_string(),
    }
}

#[tokio::test]
async fn application_crud_roundtrip() {
    let (pool, _dir) = test_pool().await;

    let created = applications_repo::create(&pool, &app_input("Acme", "Backend Engineer"))
        .await
        .unwrap();
    assert_eq!(created.status, "saved");
    assert_eq!(created.priority, 2);

    let mut edited = app_input("Acme", "Senior Backend Engineer");
    edited.priority = 3;
    edited.date_applied = Some("2026-08-10".to_string());
    // update must not clobber metadata_json or created_at
    let updated = applications_repo::update(&pool, created.id, &edited).await.unwrap();
    assert_eq!(updated.role, "Senior Backend Engineer");
    assert_eq!(updated.date_applied.as_deref(), Some("2026-08-10"));
    assert_eq!(updated.created_at, created.created_at);
    assert!(updated.updated_at >= created.updated_at);

    assert!(applications_repo::delete(&pool, created.id).await.unwrap());
    assert!(!applications_repo::delete(&pool, created.id).await.unwrap());

    let mut invalid = app_input("", "");
    invalid.company = "  ".to_string();
    assert!(applications_repo::create(&pool, &invalid).await.is_err());
}

#[tokio::test]
async fn status_transitions_and_filtering() {
    let (pool, _dir) = test_pool().await;

    let a = applications_repo::create(&pool, &app_input("Acme", "BE")).await.unwrap();
    let b = applications_repo::create(&pool, &app_input("Globex", "FS")).await.unwrap();

    let moved =
        applications_repo::set_status(&pool, a.id, ApplicationStatus::Applied).await.unwrap();
    assert_eq!(moved.status, "applied");

    applications_repo::set_status(&pool, b.id, ApplicationStatus::Oa).await.unwrap();

    let applied =
        applications_repo::list(&pool, Some(ApplicationStatus::Applied)).await.unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].id, a.id);

    let oa = applications_repo::list(&pool, Some(ApplicationStatus::Oa)).await.unwrap();
    assert_eq!(oa.len(), 1);
    assert_eq!(oa[0].id, b.id);

    let all = applications_repo::list(&pool, None).await.unwrap();
    assert_eq!(all.len(), 2);

    assert!(
        applications_repo::get(&pool, 9999)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn deleting_application_leaves_no_orphans_in_list() {
    let (pool, _dir) = test_pool().await;
    let a = applications_repo::create(&pool, &app_input("Temp", "Role")).await.unwrap();
    applications_repo::delete(&pool, a.id).await.unwrap();
    assert!(matches!(
        applications_repo::get(&pool, a.id).await,
        Err(assistant_lib::error::AppError::NotFound(_))
    ));
}
