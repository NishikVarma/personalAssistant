use assistant_lib::db::{self, contacts_repo, tags_repo};
use assistant_lib::models::contact::*;
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

fn contact_input(name: &str, email: &str) -> ContactInput {
    ContactInput {
        name: name.to_string(),
        email: email.to_string(),
        organization: Some("Acme Corp".to_string()),
        role_title: Some("HR Manager".to_string()),
        linkedin_url: None,
        notes: "Met at career fair".to_string(),
    }
}

#[tokio::test]
async fn contact_crud_roundtrip() {
    let (pool, _dir) = test_pool().await;

    let created = contacts_repo::create(&pool, &contact_input("Jane Doe", "jane@acme.com"))
        .await
        .unwrap();
    assert_eq!(created.name, "Jane Doe");
    assert_eq!(created.organization.as_deref(), Some("Acme Corp"));
    assert!(created.last_contacted_at.is_none());

    let mut edited = contact_input("Jane D.", "jane@acme.com");
    edited.role_title = Some("Senior HR".to_string());
    let updated = contacts_repo::update(&pool, created.id, &edited).await.unwrap();
    assert_eq!(updated.name, "Jane D.");
    assert_eq!(updated.role_title.as_deref(), Some("Senior HR"));

    contacts_repo::set_last_contacted(&pool, created.id, Some("2026-08-20T10:00:00Z".to_string()))
        .await
        .unwrap();
    assert_eq!(
        contacts_repo::get(&pool, created.id)
            .await
            .unwrap()
            .last_contacted_at
            .as_deref(),
        Some("2026-08-20T10:00:00Z")
    );

    contacts_repo::set_last_contacted(&pool, created.id, None).await.unwrap();
    assert!(
        contacts_repo::get(&pool, created.id)
            .await
            .unwrap()
            .last_contacted_at
            .is_none()
    );

    assert!(contacts_repo::delete(&pool, created.id).await.unwrap());
    assert!(!contacts_repo::delete(&pool, created.id).await.unwrap());
}

#[tokio::test]
async fn contact_validation_and_duplicates() {
    let (pool, _dir) = test_pool().await;

    contacts_repo::create(&pool, &contact_input("Jane", "jane@acme.com"))
        .await
        .unwrap();

    // exact duplicate rejected with a friendly error
    let dup = contacts_repo::create(&pool, &contact_input("Other", "JANE@ACME.com")).await;
    assert!(matches!(dup, Err(assistant_lib::error::AppError::InvalidInput(_))));

    // rename collision on update is also caught
    let other =
        contacts_repo::create(&pool, &contact_input("Bob", "bob@corp.io")).await.unwrap();
    let clash = contacts_repo::update(&pool, other.id, &contact_input("Bob", "jane@acme.com")).await;
    assert!(matches!(clash, Err(assistant_lib::error::AppError::InvalidInput(_))));

    // missing/invalid email rejected
    assert!(contacts_repo::create(&pool, &contact_input("X", "")).await.is_err());
    assert!(contacts_repo::create(&pool, &contact_input("X", "not-an-email")).await.is_err());
}

#[tokio::test]
async fn contact_search_filters_by_name_email_org() {
    let (pool, _dir) = test_pool().await;

    contacts_repo::create(&pool, &contact_input("Jane Doe", "jane@acme.com")).await.unwrap();
    contacts_repo::create(&pool, &contact_input("Bob Smith", "bob@globex.io")).await.unwrap();
    let mut org_hit = contact_input("Carol Jones", "carol@initech.com");
    org_hit.organization = Some("Globex".to_string());
    contacts_repo::create(&pool, &org_hit).await.unwrap();

    assert_eq!(contacts_repo::list(&pool, "").await.unwrap().len(), 3);
    assert_eq!(contacts_repo::list(&pool, "globex").await.unwrap().len(), 2);
    assert_eq!(contacts_repo::list(&pool, "JANE").await.unwrap().len(), 1);
    assert_eq!(contacts_repo::list(&pool, "nobody@nowhere").await.unwrap().len(), 0);
}

#[tokio::test]
async fn tags_roundtrip_and_contact_linking() {
    let (pool, _dir) = test_pool().await;

    let contact = contacts_repo::create(&pool, &contact_input("Jane", "jane@acme.com"))
        .await
        .unwrap();
    let recruiter = tags_repo::create(&pool, &TagInput { name: "recruiter".to_string(), color: None })
        .await
        .unwrap();
    let priority = tags_repo::create(&pool, &TagInput { name: "priority".to_string(), color: Some("#f00".to_string()) })
        .await
        .unwrap();

    // duplicate tag name rejected case-insensitively
    let dup = tags_repo::create(&pool, &TagInput { name: "RECRUITER".to_string(), color: None }).await;
    assert!(matches!(dup, Err(assistant_lib::error::AppError::InvalidInput(_))));

    tags_repo::replace_contact_tags(&pool, contact.id, &[recruiter.id, priority.id])
        .await
        .unwrap();
    let linked = tags_repo::list_for_contact(&pool, contact.id).await.unwrap();
    assert_eq!(linked.len(), 2);

    // unknown tag id surfaces NotFound, not a raw FK error
    let missing = tags_repo::replace_contact_tags(&pool, contact.id, &[9999]).await;
    assert!(matches!(missing, Err(assistant_lib::error::AppError::NotFound(_))));
    // unknown contact rejected
    let no_contact = tags_repo::replace_contact_tags(&pool, 9999, &[]).await;
    assert!(matches!(no_contact, Err(assistant_lib::error::AppError::NotFound(_))));

    // deleting a tag removes its links but keeps the contact
    tags_repo::delete(&pool, recruiter.id).await.unwrap();
    let linked = tags_repo::list_for_contact(&pool, contact.id).await.unwrap();
    assert_eq!(linked.len(), 1);
    assert!(contacts_repo::get(&pool, contact.id).await.is_ok());

    // deleting the contact cascades its remaining links
    contacts_repo::delete(&pool, contact.id).await.unwrap();
    let orphaned = tags_repo::list_for_contact(&pool, contact.id).await.unwrap();
    assert!(orphaned.is_empty());
    assert!(tags_repo::get(&pool, priority.id).await.is_ok());
}
