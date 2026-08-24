use assistant_lib::db::{self, email_history_repo, email_templates_repo, generated_emails_repo};
use assistant_lib::gmail::{
    extract_email_address, find_reply, parse_thread_messages, ParsedReply, ThreadMessage,
};
use assistant_lib::models::email::*;
use serde_json::json;
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

fn draft_input(recipient: &str) -> GeneratedEmailInput {
    GeneratedEmailInput {
        application_id: None,
        contact_id: None,
        email_type: EmailType::ColdOutreach,
        recipient_email: Some(recipient.to_string()),
        recipient_name: None,
        subject: Some("Hi".to_string()),
        body: "Hello".to_string(),
    }
}

async fn send_draft(pool: &SqlitePool, recipient: &str, thread_id: &str) -> EmailHistory {
    let created = generated_emails_repo::create(pool, &draft_input(recipient))
        .await
        .unwrap();
    generated_emails_repo::set_status(pool, created.id, EmailStatus::Approved)
        .await
        .unwrap();
    email_history_repo::record_sent(
        pool,
        &email_history_repo::SentRecord {
            generated_email_id: created.id,
            contact_id: None,
            recipient_email: recipient.to_string(),
            email_type: created.email_type.clone(),
            subject: created.subject.clone(),
            body: created.body.clone(),
            gmail_message_id: format!("sent-{recipient}"),
            gmail_thread_id: Some(thread_id.to_string()),
        },
    )
    .await
    .unwrap();

    let mut filter = HistoryFilter::default();
    filter.contact_id = None;
    let rows = email_history_repo::list(pool, &filter).await.unwrap();
    rows.into_iter()
        .find(|r| r.direction == "outgoing")
        .unwrap()
}

#[test]
fn parses_gmail_thread_metadata() {
    let body = json!({
        "id": "thread-1",
        "messages": [
            {
                "id": "msg-1",
                "snippet": "First message",
                "internalDate": "1756000000000",
                "payload": { "headers": [
                    { "name": "From", "value": "Me <me@gmail.com>" },
                    { "name": "Subject", "value": "Outreach" }
                ]}
            },
            {
                "id": "msg-2",
                "snippet": "Thanks for reaching out!",
                "internalDate": "1756086400000",
                "payload": { "headers": [
                    { "name": "FROM", "value": "jane@acme.com" },
                    { "name": "Subject", "value": "Re: Outreach" }
                ]}
            }
        ]
    });

    let messages = parse_thread_messages(&body);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].from_email.as_deref(), Some("me@gmail.com"));
    assert_eq!(messages[1].subject.as_deref(), Some("Re: Outreach"));
    assert_eq!(messages[1].internal_date_ms, Some(1756086400000));
    assert_eq!(messages[1].snippet.as_deref(), Some("Thanks for reaching out!"));
}

#[test]
fn extracts_bare_addresses_from_from_headers() {
    assert_eq!(
        extract_email_address("Jane Doe <jane@acme.com>").as_deref(),
        Some("jane@acme.com")
    );
    assert_eq!(extract_email_address("jane@acme.com").as_deref(), Some("jane@acme.com"));
    assert!(extract_email_address("no address here").is_none());
}

#[test]
fn reply_detection_ignores_own_and_older_messages() {
    let messages = vec![
        ThreadMessage {
            id: "sent-1".to_string(),
            from_email: Some("me@gmail.com".to_string()),
            subject: None,
            snippet: None,
            internal_date_ms: Some(1000),
        },
        ThreadMessage {
            id: "older-foreign".to_string(),
            from_email: Some("jane@acme.com".to_string()),
            subject: None,
            snippet: Some("before our send".to_string()),
            internal_date_ms: Some(500),
        },
        ThreadMessage {
            id: "reply-1".to_string(),
            from_email: Some("jane@acme.com".to_string()),
            subject: Some("Re: Hi".to_string()),
            snippet: Some("Sounds good".to_string()),
            internal_date_ms: Some(2000),
        },
        ThreadMessage {
            id: "my-followup".to_string(),
            from_email: Some("me@gmail.com".to_string()),
            subject: None,
            snippet: None,
            internal_date_ms: Some(3000),
        },
    ];

    let reply = find_reply(&messages, "me@gmail.com", Some("sent-1"), 1000).unwrap();
    assert_eq!(reply.gmail_message_id, "reply-1");
    assert_eq!(reply.from_email, "jane@acme.com");
    assert_eq!(reply.subject.as_deref(), Some("Re: Hi"));
    assert!(reply.occurred_at.contains("1970") || reply.occurred_at.contains("00:33:20"));

    // our own newer message must never count as a reply
    assert!(find_reply(&messages, "me@gmail.com", Some("sent-1"), 3500).is_none());
    // unknown own message id: our address is still ignored, the genuine reply counts
    assert_eq!(
        find_reply(&messages, "me@gmail.com", None, 1000)
            .map(|r| r.gmail_message_id),
        Some("reply-1".to_string())
    );
}

#[tokio::test]
async fn history_filters_and_response_status() {
    let (pool, _dir) = test_pool().await;
    let sent = send_draft(&pool, "hr@acme.com", "thread-1").await;

    // default response status is awaiting
    assert_eq!(sent.response_status.as_deref(), Some("awaiting"));

    let updated = email_history_repo::set_response_status(
        &pool,
        sent.id,
        Some(ResponseStatus::Replied),
    )
    .await
    .unwrap();
    assert_eq!(updated.response_status.as_deref(), Some("replied"));

    email_history_repo::set_response_status(&pool, sent.id, None)
        .await
        .unwrap();
    assert!(
        email_history_repo::get(&pool, sent.id)
            .await
            .unwrap()
            .response_status
            .is_none()
    );

    // incoming rows cannot take a response status
    let incoming = email_history_repo::record_incoming(
        &pool,
        &IncomingEmailInput {
            contact_id: None,
            application_id: None,
            sender_email: "someone@corp.io".to_string(),
            email_type: None,
            subject: Some("Hello".to_string()),
            body: "Hi there".to_string(),
            occurred_at: None,
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        email_history_repo::set_response_status(&pool, incoming.id, Some(ResponseStatus::Replied)).await,
        Err(assistant_lib::error::AppError::NotFound(_))
    ));
}

#[tokio::test]
async fn history_list_filters_by_contact_and_application() {
    use assistant_lib::db::applications_repo;
    use assistant_lib::models::application::*;

    let (pool, _dir) = test_pool().await;

    let app = applications_repo::create(
        &pool,
        &ApplicationInput {
            company: "Acme".to_string(),
            role: "BE".to_string(),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    email_history_repo::record_incoming(
        &pool,
        &IncomingEmailInput {
            contact_id: None,
            application_id: Some(app.id),
            sender_email: "jane@acme.com".to_string(),
            email_type: None,
            subject: None,
            body: "reply".to_string(),
            occurred_at: None,
        },
    )
    .await
    .unwrap();
    email_history_repo::record_incoming(
        &pool,
        &IncomingEmailInput {
            contact_id: None,
            application_id: None,
            sender_email: "other@corp.io".to_string(),
            email_type: None,
            subject: None,
            body: "unrelated".to_string(),
            occurred_at: None,
        },
    )
    .await
    .unwrap();

    let mut by_app = HistoryFilter::default();
    by_app.application_id = Some(app.id);
    assert_eq!(email_history_repo::list(&pool, &by_app).await.unwrap().len(), 1);

    let mut bogus = HistoryFilter::default();
    bogus.contact_id = Some(9999);
    assert!(email_history_repo::list(&pool, &bogus).await.unwrap().is_empty());

    let all = email_history_repo::list(&pool, &HistoryFilter::default()).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn record_reply_inserts_incoming_row_and_flips_status() {
    let (pool, _dir) = test_pool().await;
    let sent = send_draft(&pool, "hr@acme.com", "thread-42").await;

    let reply = ParsedReply {
        gmail_message_id: "reply-77".to_string(),
        from_email: "hr@acme.com".to_string(),
        subject: Some("Re: Hi".to_string()),
        snippet: Some("Sounds interesting".to_string()),
        occurred_at: "2026-08-25T10:00:00+00:00".to_string(),
    };
    email_history_repo::record_reply(&pool, &sent, &reply).await.unwrap();

    // originating row flipped to replied
    let updated = email_history_repo::get(&pool, sent.id).await.unwrap();
    assert_eq!(updated.response_status.as_deref(), Some("replied"));

    // incoming row linked back to the same contact/application/draft
    let mut filter = HistoryFilter::default();
    filter.contact_id = sent.contact_id;
    filter.application_id = sent.application_id;
    let rows = email_history_repo::list(&pool, &filter).await.unwrap();
    let incoming = rows.iter().find(|r| r.direction == "incoming").unwrap();
    assert_eq!(incoming.gmail_message_id.as_deref(), Some("reply-77"));
    assert_eq!(incoming.recipient_email.as_deref(), Some("hr@acme.com"));
    assert_eq!(incoming.body, "Sounds interesting");
    assert_eq!(incoming.status, "received");
    assert_eq!(incoming.generated_email_id, sent.generated_email_id);
}

#[tokio::test]
async fn awaiting_with_threads_only_returns_pending_outgoing() {
    let (pool, _dir) = test_pool().await;

    let sent = send_draft(&pool, "a@acme.com", "thread-a").await;
    send_draft(&pool, "b@acme.com", "thread-b").await;

    // reply one of them
    email_history_repo::record_reply(
        &pool,
        &sent,
        &ParsedReply {
            gmail_message_id: "r1".to_string(),
            from_email: "a@acme.com".to_string(),
            subject: None,
            snippet: None,
            occurred_at: "2026-08-25T10:00:00+00:00".to_string(),
        },
    )
    .await
    .unwrap();

    let awaiting = email_history_repo::awaiting_with_threads(&pool, 50).await.unwrap();
    assert_eq!(awaiting.len(), 1);
    assert_eq!(awaiting[0].recipient_email.as_deref(), Some("b@acme.com"));
}

fn template_input(email_type: EmailType, role: Option<&str>, company: Option<&str>) -> EmailTemplateInput {
    EmailTemplateInput {
        email_type,
        role: role.map(String::from),
        company_or_industry: company.map(String::from),
        subject_template: Some("Subject".to_string()),
        body_template: "Body".to_string(),
    }
}

#[tokio::test]
async fn template_crud_and_mark_used() {
    let (pool, _dir) = test_pool().await;

    let created = email_templates_repo::create(
        &pool,
        &template_input(EmailType::ColdOutreach, Some("Backend".into()), None),
        "user",
    )
    .await
    .unwrap();
    assert_eq!(created.source, "user");
    assert_eq!(created.times_used, 0);

    let mut edited = template_input(EmailType::FollowUp, None, None);
    edited.body_template = "Updated body".to_string();
    let updated = email_templates_repo::update(&pool, created.id, &edited).await.unwrap();
    assert_eq!(updated.email_type, "follow_up");
    assert_eq!(updated.body_template, "Updated body");

    email_templates_repo::mark_used(&pool, created.id).await.unwrap();
    email_templates_repo::mark_used(&pool, created.id).await.unwrap();
    assert_eq!(email_templates_repo::get(&pool, created.id).await.unwrap().times_used, 2);

    assert!(email_templates_repo::delete(&pool, created.id).await.unwrap());
    assert!(email_templates_repo::create(
        &pool,
        &EmailTemplateInput {
            body_template: "   ".to_string(),
            ..template_input(EmailType::ColdOutreach, None, None)
        },
        "user",
    )
    .await
    .is_err());
}

fn template_row(id: i64, email_type: EmailType, role: Option<&str>, company: Option<&str>, times_used: i64) -> EmailTemplate {
    EmailTemplate {
        id,
        email_type: email_type.as_str().to_string(),
        role: role.map(String::from),
        company_or_industry: company.map(String::from),
        subject_template: Some("Subject".to_string()),
        body_template: "Body".to_string(),
        variables_json: "[]".to_string(),
        source: "user".to_string(),
        success_count: 0,
        times_used,
        last_used_at: None,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

#[test]
fn template_ranking_prefers_company_then_role_then_usage() {
    let generic = template_row(1, EmailType::ColdOutreach, None, None, 0);
    let role_match = template_row(2, EmailType::ColdOutreach, Some("Backend"), None, 0);
    let company_match = template_row(3, EmailType::ColdOutreach, Some("Backend"), Some("Acme"), 0);
    let heavily_used = template_row(4, EmailType::ColdOutreach, None, None, 50);

    // exact company beats role beats generic
    let pool_of = vec![generic.clone(), role_match.clone(), company_match.clone()];
    assert_eq!(
        email_templates_repo::choose_template(&pool_of, Some("Backend"), Some("acme"))
            .unwrap()
            .company_or_industry,
        Some("Acme".to_string())
    );

    // no company in request: role match wins over usage count
    let pool_of = vec![generic.clone(), heavily_used.clone(), role_match.clone()];
    assert_eq!(
        email_templates_repo::choose_template(&pool_of, Some("Backend"), None)
            .unwrap()
            .role,
        Some("Backend".to_string())
    );

    // nothing matches: most used wins
    let pool_of = vec![generic, heavily_used];
    assert_eq!(
        email_templates_repo::choose_template(&pool_of, Some("Frontend"), Some("Globex"))
            .unwrap()
            .times_used,
        50
    );

    // empty list
    assert!(email_templates_repo::choose_template(&[], None, None).is_none());
}
