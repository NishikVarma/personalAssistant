use assistant_lib::db::{self, email_history_repo, generated_emails_repo};
use assistant_lib::gmail::mime::{self, Attachment};
use assistant_lib::models::email::*;
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

#[test]
fn plain_mime_has_required_headers() {
    let message = mime::build_mime(
        "me@gmail.com",
        "hr@acme.com",
        "Application for Backend role",
        "Hello,\n\nI would like to apply.",
        None,
    );
    assert!(message.contains("From: me@gmail.com\r\n"));
    assert!(message.contains("To: hr@acme.com\r\n"));
    assert!(message.contains("Subject: Application for Backend role\r\n"));
    assert!(message.contains("MIME-Version: 1.0\r\n"));
    assert!(message.contains("Content-Type: text/plain; charset=UTF-8\r\n"));
    assert!(message.ends_with("I would like to apply."));
    assert!(!message.contains("boundary"));
}

#[test]
fn mime_with_attachment_is_multipart() {
    let attachment = Attachment {
        filename: "resume.pdf".to_string(),
        mime_type: "application/pdf".to_string(),
        content_base64: "JVBERi0xLjQK".repeat(40), // > 76 chars to exercise chunking
    };
    let message = mime::build_mime("me@gmail.com", "hr@acme.com", "Hi", "Body here", Some(&attachment));

    assert!(message.contains("multipart/mixed; boundary=\"copilot-"));
    assert!(message.contains("Content-Type: application/pdf;\r\n"));
    assert!(message.contains("Content-Disposition: attachment; filename=\"resume.pdf\"\r\n"));
    assert!(message.contains("Content-Transfer-Encoding: base64\r\n"));
    // chunking keeps lines <= 76 chars
    for line in message.lines() {
        assert!(line.len() <= 76 + 2, "line too long: {}", line.len());
    }
    assert!(message.trim_end().ends_with("--"));
}

#[test]
fn non_ascii_subjects_are_rfc2047_encoded() {
    let message =
        mime::build_mime("me@gmail.com", "hr@acme.com", "Héllo wörld", "body", None);
    assert!(message.contains("Subject: =?UTF-8?B?"));
    assert!(!message.contains("Héllo wörld\r\n"));
}

#[test]
fn gmail_raw_is_base64url_without_padding() {
    let raw = mime::to_gmail_raw("hello");
    assert_eq!(raw, "aGVsbG8");
    assert!(!raw.contains('+') && !raw.contains('/') && !raw.contains('='));
}

fn draft_input(recipient: &str) -> GeneratedEmailInput {
    GeneratedEmailInput {
        application_id: None,
        contact_id: None,
        email_type: EmailType::ColdOutreach,
        recipient_email: Some(recipient.to_string()),
        recipient_name: Some("Jane".to_string()),
        subject: Some("Hi".to_string()),
        body: "Hello".to_string(),
    }
}

#[tokio::test]
async fn migration_adds_recipient_columns_and_repo_stores_them() {
    let (pool, _dir) = test_pool().await;

    let created = generated_emails_repo::create(&pool, &draft_input("hr@acme.com"))
        .await
        .unwrap();
    assert_eq!(created.recipient_email.as_deref(), Some("hr@acme.com"));
    assert_eq!(created.recipient_name.as_deref(), Some("Jane"));

    // schema version advanced to 0002
    let version = db::schema_version(&pool).await.unwrap();
    assert_eq!(version, 4);
}

#[tokio::test]
async fn record_sent_flips_status_and_stamps_history() {
    let (pool, _dir) = test_pool().await;

    let created = generated_emails_repo::create(&pool, &draft_input("hr@acme.com"))
        .await
        .unwrap();
    generated_emails_repo::set_status(&pool, created.id, EmailStatus::Edited)
        .await
        .unwrap();
    generated_emails_repo::set_status(&pool, created.id, EmailStatus::Approved)
        .await
        .unwrap();

    email_history_repo::record_sent(
        &pool,
        &email_history_repo::SentRecord {
            generated_email_id: created.id,
            contact_id: None,
            recipient_email: "hr@acme.com".to_string(),
            email_type: created.email_type.clone(),
            subject: created.subject.clone(),
            body: created.body.clone(),
            gmail_message_id: "msg-123".to_string(),
            gmail_thread_id: Some("thread-9".to_string()),
        },
    )
    .await
    .unwrap();

    let sent = generated_emails_repo::get(&pool, created.id).await.unwrap();
    assert_eq!(sent.status, "sent");

    let history_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM email_history WHERE gmail_message_id = 'msg-123'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(history_count, 1);

    let direction: String = sqlx::query_scalar(
        "SELECT direction FROM email_history WHERE gmail_message_id = 'msg-123'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(direction, "outgoing");
}

#[tokio::test]
async fn recent_outreach_guard_detects_sends_within_window() {
    let (pool, _dir) = test_pool().await;

    let created = generated_emails_repo::create(&pool, &draft_input("hr@acme.com"))
        .await
        .unwrap();
    generated_emails_repo::set_status(&pool, created.id, EmailStatus::Approved)
        .await
        .unwrap();

    assert!(
        !email_history_repo::has_recent_outgoing(&pool, "hr@acme.com", 7)
            .await
            .unwrap()
    );

    email_history_repo::record_sent(
        &pool,
        &email_history_repo::SentRecord {
            generated_email_id: created.id,
            contact_id: None,
            recipient_email: "hr@acme.com".to_string(),
            email_type: created.email_type.clone(),
            subject: None,
            body: created.body.clone(),
            gmail_message_id: "msg-1".to_string(),
            gmail_thread_id: None,
        },
    )
    .await
    .unwrap();

    assert!(
        email_history_repo::has_recent_outgoing(&pool, "HR@ACME.com", 7)
            .await
            .unwrap(),
        "case-insensitive match within window"
    );
    assert!(
        !email_history_repo::has_recent_outgoing(&pool, "other@corp.io", 7)
            .await
            .unwrap(),
        "other addresses unaffected"
    );
    assert!(
        !email_history_repo::has_recent_outgoing(&pool, "hr@acme.com", 0)
            .await
            .unwrap(),
        "zero-day window excludes everything"
    );
}

#[tokio::test]
async fn record_sent_updates_contact_last_contacted() {
    use assistant_lib::db::contacts_repo;
    use assistant_lib::models::contact::ContactInput;

    let (pool, _dir) = test_pool().await;

    let contact = contacts_repo::create(
        &pool,
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
    assert!(contact.last_contacted_at.is_none());

    let mut draft = draft_input("jane@acme.com");
    draft.contact_id = Some(contact.id);
    let created = generated_emails_repo::create(&pool, &draft).await.unwrap();
    generated_emails_repo::set_status(&pool, created.id, EmailStatus::Approved)
        .await
        .unwrap();

    email_history_repo::record_sent(
        &pool,
        &email_history_repo::SentRecord {
            generated_email_id: created.id,
            contact_id: Some(contact.id),
            recipient_email: "jane@acme.com".to_string(),
            email_type: created.email_type.clone(),
            subject: None,
            body: created.body.clone(),
            gmail_message_id: "msg-2".to_string(),
            gmail_thread_id: None,
        },
    )
    .await
    .unwrap();

    let updated = contacts_repo::get(&pool, contact.id).await.unwrap();
    assert!(updated.last_contacted_at.is_some());
}
