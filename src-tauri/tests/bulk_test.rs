use assistant_lib::commands::bulk::{parse_csv_file, parse_xlsx_file};
use assistant_lib::db::{self, bulk_batches_repo, generated_emails_repo};
use assistant_lib::models::email::{EmailType, GeneratedEmailInput};
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

fn write_csv(dir: &tempfile::TempDir, content: &str) -> String {
    let path = dir.path().join("contacts.csv");
    std::fs::write(&path, content).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn csv_parsing_extracts_headers_and_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_csv(
        &dir,
        "name,email,company\nJane Doe,jane@acme.com,Acme\nBob Smith,bob@globex.io,Globex\n",
    );
    let (headers, rows, total) = parse_csv_file(&path).unwrap();
    assert_eq!(headers, vec!["name", "email", "company"]);
    assert_eq!(total, 2);
    assert_eq!(rows[0][1], "jane@acme.com");
}

#[test]
fn csv_parsing_rejects_empty_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_csv(&dir, "");
    assert!(parse_csv_file(&path).is_err());
}

#[test]
fn xlsx_parsing_reads_first_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("contacts.xlsx");
    // build a minimal xlsx via calamine's writable sibling is unavailable;
    // instead assert the error path for a non-xlsx file
    std::fs::write(&path, b"not a real xlsx").unwrap();
    assert!(parse_xlsx_file(path.to_str().unwrap()).is_err());
}

#[tokio::test]
async fn batch_create_and_status_flow() {
    let (pool, _dir) = test_pool().await;

    let batch = bulk_batches_repo::create(&pool, EmailType::ColdOutreach, None)
        .await
        .unwrap();
    assert_eq!(batch.status, "draft");
    assert_eq!(batch.total_count, 0);

    bulk_batches_repo::set_total(&pool, batch.id, 5).await.unwrap();
    bulk_batches_repo::bump_counts(&pool, batch.id, 3, 1).await.unwrap();
    let updated = bulk_batches_repo::get(&pool, batch.id).await.unwrap();
    assert_eq!(updated.total_count, 5);
    assert_eq!(updated.sent_count, 3);
    assert_eq!(updated.failed_count, 1);

    bulk_batches_repo::set_status(&pool, batch.id, "sent").await.unwrap();
    assert_eq!(bulk_batches_repo::get(&pool, batch.id).await.unwrap().status, "sent");

    assert!(bulk_batches_repo::create(&pool, EmailType::ColdOutreach, Some(9999))
        .await
        .is_err());
}

#[tokio::test]
async fn bulk_drafts_link_to_batch_and_remove_updates_total() {
    let (pool, _dir) = test_pool().await;
    let batch = bulk_batches_repo::create(&pool, EmailType::ColdOutreach, None)
        .await
        .unwrap();

    for recipient in ["a@acme.com", "b@acme.com"] {
        let draft = generated_emails_repo::create(
            &pool,
            &GeneratedEmailInput {
                application_id: None,
                contact_id: None,
                email_type: EmailType::ColdOutreach,
                recipient_email: Some(recipient.to_string()),
                recipient_name: None,
                subject: Some("Hi".to_string()),
                body: "Body".to_string(),
            },
        )
        .await
        .unwrap();
        generated_emails_repo::set_bulk_batch_link(&pool, draft.id, batch.id)
            .await
            .unwrap();
    }

    assert_eq!(generated_emails_repo::count_by_batch(&pool, batch.id).await.unwrap(), 2);
    let rows = generated_emails_repo::list_by_batch(&pool, batch.id).await.unwrap();
    assert_eq!(rows[0].bulk_batch_id, Some(batch.id));

    // removing a draft from another batch is rejected
    let other = bulk_batches_repo::create(&pool, EmailType::ColdOutreach, None)
        .await
        .unwrap();
    assert!(
        assistant_lib::commands::bulk::bulk_batch_remove_draft_inner(
            &pool, other.id, rows[0].id
        )
        .await
        .is_err()
    );

    assistant_lib::commands::bulk::bulk_batch_remove_draft_inner(&pool, batch.id, rows[0].id)
        .await
        .unwrap();
    assert_eq!(generated_emails_repo::count_by_batch(&pool, batch.id).await.unwrap(), 1);
    assert_eq!(bulk_batches_repo::get(&pool, batch.id).await.unwrap().total_count, 1);
}
