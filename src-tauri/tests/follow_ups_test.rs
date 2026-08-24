use assistant_lib::db::{
    self, applications_repo, contacts_repo, email_history_repo, follow_ups_repo,
    generated_emails_repo,
};
use assistant_lib::models::application::*;
use assistant_lib::models::contact::ContactInput;
use assistant_lib::models::email::*;
use assistant_lib::models::follow_up::*;
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

async fn seed_application(pool: &SqlitePool, status: &str) -> i64 {
    let app = applications_repo::create(
        pool,
        &ApplicationInput {
            company: "Acme".to_string(),
            role: "Backend".to_string(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    if status != "saved" {
        applications_repo::set_status(pool, app.id, ApplicationStatus::try_from_str(status).unwrap())
            .await
            .unwrap();
    }
    app.id
}

async fn seed_contact(pool: &SqlitePool) -> i64 {
    let contact = contacts_repo::create(
        pool,
        &ContactInput {
            name: "Jane".to_string(),
            email: "jane@acme.com".to_string(),
            organization: None,
            role_title: None,
            linkedin_url: None,
            notes: String::new(),
        },
    )
    .await
    .unwrap();
    contact.id
}

async fn send_outreach(pool: &SqlitePool, application_id: i64, contact_id: i64) -> i64 {
    let draft = generated_emails_repo::create(
        pool,
        &GeneratedEmailInput {
            application_id: Some(application_id),
            contact_id: Some(contact_id),
            email_type: EmailType::ColdOutreach,
            recipient_email: Some("jane@acme.com".to_string()),
            recipient_name: None,
            subject: Some("Hello Acme".to_string()),
            body: "I would like to apply.".to_string(),
        },
    )
    .await
    .unwrap();
    generated_emails_repo::set_status(pool, draft.id, EmailStatus::Approved)
        .await
        .unwrap();
    email_history_repo::record_sent(
        pool,
        &email_history_repo::SentRecord {
            generated_email_id: draft.id,
            contact_id: Some(contact_id),
            recipient_email: "jane@acme.com".to_string(),
            email_type: draft.email_type.clone(),
            subject: draft.subject.clone(),
            body: draft.body.clone(),
            gmail_message_id: format!("msg-{}", draft.id),
            gmail_thread_id: Some(format!("thread-{}", draft.id)),
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn follow_up_crud_reschedule_and_cancel() {
    let (pool, _dir) = test_pool().await;
    let app_id = seed_application(&pool, "applied").await;

    let created = follow_ups_repo::create(
        &pool,
        app_id,
        None,
        None,
        1,
        "2026-09-01T00:00:00+00:00".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(created.status, "pending");
    assert_eq!(created.sequence, 1);

    // invalid references rejected
    assert!(follow_ups_repo::create(&pool, 9999, None, None, 1, "x".into()).await.is_err());
    assert!(follow_ups_repo::create(&pool, app_id, None, Some(9999), 1, "x".into())
        .await
        .is_err());

    let rescheduled = follow_ups_repo::reschedule(
        &pool,
        created.id,
        "2026-09-10T00:00:00+00:00".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(rescheduled.scheduled_for, "2026-09-10T00:00:00+00:00");

    follow_ups_repo::mark_sent(&pool, created.id).await.unwrap();
    let sent = follow_ups_repo::get(&pool, created.id).await.unwrap();
    assert_eq!(sent.status, "sent");
    assert!(sent.completed_at.is_some());
    assert!(follow_ups_repo::reschedule(&pool, created.id, "y".into()).await.is_err());

    let other = follow_ups_repo::create(
        &pool,
        app_id,
        None,
        None,
        2,
        "2026-09-15T00:00:00+00:00".to_string(),
    )
    .await
    .unwrap();
    let cancelled = follow_ups_repo::cancel(&pool, other.id).await.unwrap();
    assert_eq!(cancelled.status, "cancelled");
}

#[tokio::test]
async fn due_listing_flips_arrived_pending_rows() {
    let (pool, _dir) = test_pool().await;
    let app_id = seed_application(&pool, "applied").await;

    follow_ups_repo::create(
        &pool,
        app_id,
        None,
        None,
        1,
        "2020-01-01T00:00:00+00:00".to_string(), // long past
    )
    .await
    .unwrap();
    follow_ups_repo::create(
        &pool,
        app_id,
        None,
        None,
        2,
        "2099-01-01T00:00:00+00:00".to_string(), // far future
    )
    .await
    .unwrap();

    let due = follow_ups_repo::list_due(&pool).await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].status, "due");
    assert_eq!(due[0].scheduled_for, "2020-01-01T00:00:00+00:00");

    // due_count includes the row that just became due
    assert_eq!(follow_ups_repo::due_count(&pool).await.unwrap(), 1);
}

#[tokio::test]
async fn suppression_sweep_handles_closed_applications_and_replies() {
    let (pool, _dir) = test_pool().await;

    // closed application
    let closed_app = seed_application(&pool, "applied").await;
    follow_ups_repo::create(
        &pool,
        closed_app,
        None,
        None,
        1,
        "2099-01-01T00:00:00+00:00".to_string(),
    )
    .await
    .unwrap();
    applications_repo::set_status(&pool, closed_app, ApplicationStatus::Rejected)
        .await
        .unwrap();

    // replied contact
    let open_app = seed_application(&pool, "applied").await;
    let contact_id = seed_contact(&pool).await;
    let history_id = send_outreach(&pool, open_app, contact_id).await;
    follow_ups_repo::create(
        &pool,
        open_app,
        Some(contact_id),
        Some(history_id),
        1,
        "2099-01-01T00:00:00+00:00".to_string(),
    )
    .await
    .unwrap();

    // the contact replies after our send
    let sent_row = email_history_repo::get(&pool, history_id).await.unwrap();
    email_history_repo::record_reply(
        &pool,
        &sent_row,
        &assistant_lib::gmail::ParsedReply {
            gmail_message_id: "reply-1".to_string(),
            from_email: "jane@acme.com".to_string(),
            subject: Some("Re: Hello Acme".to_string()),
            snippet: Some("Interested".to_string()),
            occurred_at: "2099-06-01T00:00:00+00:00".to_string(),
        },
    )
    .await
    .unwrap();

    let suppressed = follow_ups_repo::apply_suppressions(&pool).await.unwrap();
    assert_eq!(suppressed, 2, "one for closed app, one for replied contact");

    let rows = follow_ups_repo::list(&pool, None).await.unwrap();
    for row in rows {
        assert_eq!(row.status, "suppressed");
        assert!(!row.suppressed_reason.unwrap_or_default().is_empty());
    }

    // sweeping again changes nothing
    assert_eq!(follow_ups_repo::apply_suppressions(&pool).await.unwrap(), 0);
}

#[tokio::test]
async fn suppression_targets_only_the_replied_contacts_rows() {
    let (pool, _dir) = test_pool().await;
    let app_id = seed_application(&pool, "applied").await;
    let jane = seed_contact(&pool).await;

    let bob = contacts_repo::create(
        &pool,
        &ContactInput {
            name: "Bob".to_string(),
            email: "bob@acme.com".to_string(),
            organization: None,
            role_title: None,
            linkedin_url: None,
            notes: String::new(),
        },
    )
    .await
    .unwrap()
    .id;

    let jane_history = send_outreach(&pool, app_id, jane).await;
    let bob_history = send_outreach(&pool, app_id, bob).await;

    follow_ups_repo::create(&pool, app_id, Some(jane), Some(jane_history), 1, "2099-01-01T00:00:00+00:00".to_string()).await.unwrap();
    follow_ups_repo::create(&pool, app_id, Some(bob), Some(bob_history), 1, "2099-01-01T00:00:00+00:00".to_string()).await.unwrap();

    // Jane replies
    let sent_row = email_history_repo::get(&pool, jane_history).await.unwrap();
    email_history_repo::record_reply(
        &pool,
        &sent_row,
        &assistant_lib::gmail::ParsedReply {
            gmail_message_id: "r1".to_string(),
            from_email: "jane@acme.com".to_string(),
            subject: Some("Re: Hello Acme".to_string()),
            snippet: Some("Interested!".to_string()),
            occurred_at: "2099-06-01T00:00:00+00:00".to_string(),
        },
    )
    .await
    .unwrap();

    follow_ups_repo::apply_suppressions(&pool).await.unwrap();

    let jane_fu = follow_ups_repo::list(&pool, Some(FollowUpStatus::Suppressed))
        .await
        .unwrap();
    assert_eq!(jane_fu.len(), 1);
    assert_eq!(jane_fu[0].contact_id, Some(jane));

    // Bob's follow-up untouched
    let pending = follow_ups_repo::list(&pool, Some(FollowUpStatus::Pending))
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].contact_id, Some(bob));
}
