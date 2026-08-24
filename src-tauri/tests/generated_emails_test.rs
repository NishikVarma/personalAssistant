use assistant_lib::db::{self, contacts_repo, generated_emails_repo, profile_snapshot};
use assistant_lib::llm::email_prompt::{
    build_email_prompt, build_extract_contact_prompt, parse_email_response,
    parse_extracted_contact,
};
use assistant_lib::models::contact::ContactInput;
use assistant_lib::models::email::*;
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

fn draft_input() -> GeneratedEmailInput {
    GeneratedEmailInput {
        application_id: None,
        contact_id: None,
        email_type: EmailType::ColdOutreach,
        recipient_email: Some("jane@acme.com".to_string()),
        recipient_name: None,
        subject: Some("Exploring backend roles".to_string()),
        body: "Hi Jane, I am a backend engineer...".to_string(),
    }
}

#[tokio::test]
async fn generated_email_crud_roundtrip() {
    let (pool, _dir) = test_pool().await;

    let created = generated_emails_repo::create(&pool, &draft_input()).await.unwrap();
    assert_eq!(created.status, "draft");
    assert_eq!(created.email_type, "cold_outreach");

    let edited = generated_emails_repo::update_content(
        &pool,
        created.id,
        Some("Updated subject".to_string()),
        "Updated body.".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(edited.status, "edited", "editing a draft flips it to edited");
    assert_eq!(edited.subject.as_deref(), Some("Updated subject"));

    let approved = generated_emails_repo::set_status(&pool, created.id, EmailStatus::Approved)
        .await
        .unwrap();
    assert_eq!(approved.status, "approved");

    // editing an approved email keeps it approved (no silent downgrade)
    let still_approved = generated_emails_repo::update_content(
        &pool,
        created.id,
        None,
        "Final body.".to_string(),
    )
    .await
    .unwrap();
    assert_eq!(still_approved.status, "approved");

    assert!(generated_emails_repo::delete(&pool, created.id).await.unwrap());
}

#[tokio::test]
async fn status_transitions_are_enforced() {
    let (pool, _dir) = test_pool().await;
    let created = generated_emails_repo::create(&pool, &draft_input()).await.unwrap();

    // draft -> sent is not allowed; must pass through approved
    let skip = generated_emails_repo::set_status(&pool, created.id, EmailStatus::Sent).await;
    assert!(matches!(skip, Err(assistant_lib::error::AppError::InvalidInput(_))));

    generated_emails_repo::set_status(&pool, created.id, EmailStatus::Approved)
        .await
        .unwrap();
    let sent = generated_emails_repo::set_status(&pool, created.id, EmailStatus::Sent)
        .await
        .unwrap();
    assert_eq!(sent.status, "sent");

    // sent and discarded are terminal
    for terminal in [EmailStatus::Draft, EmailStatus::Edited, EmailStatus::Approved] {
        assert!(
            generated_emails_repo::set_status(&pool, created.id, terminal).await.is_err(),
            "{} must be terminal",
            sent.status
        );
    }
}

#[tokio::test]
async fn create_validates_content_and_links() {
    let (pool, _dir) = test_pool().await;

    let mut empty_body = draft_input();
    empty_body.body = "   ".to_string();
    assert!(matches!(
        generated_emails_repo::create(&pool, &empty_body).await,
        Err(assistant_lib::error::AppError::InvalidInput(_))
    ));

    let mut bad_link = draft_input();
    bad_link.application_id = Some(9999);
    assert!(matches!(
        generated_emails_repo::create(&pool, &bad_link).await,
        Err(assistant_lib::error::AppError::NotFound(_))
    ));

    let mut bad_contact = draft_input();
    bad_contact.contact_id = Some(9999);
    assert!(generated_emails_repo::create(&pool, &bad_contact).await.is_err());
}

#[tokio::test]
async fn list_filters_by_status() {
    let (pool, _dir) = test_pool().await;
    generated_emails_repo::create(&pool, &draft_input()).await.unwrap();
    let mut second = draft_input();
    second.body = "Another".to_string();
    let second = generated_emails_repo::create(&pool, &second).await.unwrap();
    generated_emails_repo::set_status(&pool, second.id, EmailStatus::Discarded)
        .await
        .unwrap();

    assert_eq!(generated_emails_repo::list(&pool, None).await.unwrap().len(), 2);
    assert_eq!(
        generated_emails_repo::list(&pool, Some(EmailStatus::Draft))
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        generated_emails_repo::list(&pool, Some(EmailStatus::Discarded))
            .await
            .unwrap()
            .iter()
            .all(|e| e.status == "discarded")
    );
}

#[test]
fn prompt_contains_profile_and_anti_fabrication_rules() {
    let request = EmailDraftRequest {
        recipient_email: "jane@acme.com".to_string(),
        recipient_name: Some("Jane".to_string()),
        company: Some("Acme".to_string()),
        role: Some("Backend Engineer".to_string()),
        job_description: Some("Rust services.".to_string()),
        additional_context: Some("Met at a career fair.".to_string()),
        email_type: EmailType::JobApplication,
        application_id: None,
        contact_id: None,
    };
    let profile_block = "ABOUT THE CANDIDATE\nName: Nishik Varma";
    let prompt = build_email_prompt(&request, profile_block);

    assert!(prompt.contains("NEVER invent"));
    assert!(prompt.contains("jane@acme.com"));
    assert!(prompt.contains("job_application"));
    assert!(prompt.contains(profile_block));
    assert!(prompt.contains("\"subject\""));

    let unknown = EmailDraftRequest { recipient_name: None, ..request };
    assert!(build_email_prompt(&unknown, profile_block).contains("generic greeting"));
}

#[test]
fn response_parsing_handles_fences_and_prose() {
    let (subject, body) =
        parse_email_response("{\"subject\": \"Hello\", \"body\": \"Line1\\nLine2\"}").unwrap();
    assert_eq!(subject.as_deref(), Some("Hello"));
    assert_eq!(body, "Line1\nLine2");

    let fenced = "Here you go:\n```json\n{\"subject\":\"S\",\"body\":\"B\"}\n```\nDone.";
    assert_eq!(parse_email_response(fenced).unwrap(), (Some("S".into()), "B".into()));

    assert!(parse_email_response("no json here").is_err());
    assert!(parse_email_response("{\"subject\": null, \"body\": \"   \"}").is_err());

    let no_subject = parse_email_response("{\"body\": \"Only body\"}").unwrap();
    assert_eq!(no_subject.0, None);
}

#[test]
fn contact_extraction_prompt_and_parse() {
    let prompt = build_extract_contact_prompt("jane.smith@acme.com");
    assert!(prompt.contains("jane.smith@acme.com"));
    assert!(prompt.contains("null"));

    let (name, org) = parse_extracted_contact(
        "{\"name\": \"Jane Smith\", \"organization\": \"Acme Corp\"}",
    )
    .unwrap();
    assert_eq!(name.as_deref(), Some("Jane Smith"));
    assert_eq!(org.as_deref(), Some("Acme Corp"));

    let (name, org) = parse_extracted_contact("{\"name\": null, \"organization\": null}").unwrap();
    assert!(name.is_none() && org.is_none());

    assert!(parse_extracted_contact("{broken").is_err());
}

/// Verifies the deterministic link-if-exists lookup used by ai_generate_email.
#[tokio::test]
async fn contact_lookup_by_exact_email() {
    let (pool, _dir) = test_pool().await;

    assert!(contacts_repo::find_id_by_email(&pool, "missing@acme.com")
        .await
        .unwrap()
        .is_none());

    contacts_repo::create(
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

    assert!(contacts_repo::find_id_by_email(&pool, "JANE@ACME.COM")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn profile_snapshot_includes_all_filled_sections() {
    use assistant_lib::db::{
        bullets_repo, education_repo, experience_repo, profile_repo, projects_repo, skills_repo,
    };
    use assistant_lib::models::profile::*;

    let (pool, _dir) = test_pool().await;

    // empty profile still produces a usable block
    profile_snapshot::collect(&pool).await.unwrap();

    profile_repo::update(
        &pool,
        &UserProfileInput {
            full_name: "Nishik Varma".to_string(),
            email: "nishik@example.com".to_string(),
            phone: String::new(),
            location: "India".to_string(),
            summary: "Backend engineer.".to_string(),
        },
    )
    .await
    .unwrap();

    education_repo::create(
        &pool,
        &EducationInput {
            institution: "IIT Hyderabad".to_string(),
            degree: "B.Tech".to_string(),
            field_of_study: "Computer Science".to_string(),
            start_date: None,
            end_date: None,
            grade: None,
            location: None,
            details: String::new(),
        },
    )
    .await
    .unwrap();

    experience_repo::create(
        &pool,
        &ExperienceInput {
            organization: "Acme".to_string(),
            title: "SDE Intern".to_string(),
            employment_type: EmploymentType::Internship,
            location: None,
            start_date: Some("2025-05-01".to_string()),
            end_date: None,
            currently_working: false,
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let project = projects_repo::create(
        &pool,
        &ProjectInput {
            name: "Copilot".to_string(),
            description: "Local-first copilot.".to_string(),
            repo_url: None,
            live_url: None,
            status: ProjectStatus::Completed,
            started_on: None,
            ended_on: None,
        },
    )
    .await
    .unwrap();
    bullets_repo::create(
        &pool,
        ProfileEntityType::Project,
        project.id,
        &BulletInput { content: "Built the thing".to_string(), display_order: 0 },
    )
    .await
    .unwrap();
    skills_repo::create(
        &pool,
        &SkillInput { name: "Rust".to_string(), category: SkillCategory::Language },
    )
    .await
    .unwrap();

    let snapshot = profile_snapshot::collect(&pool).await.unwrap();
    for needle in [
        "Nishik Varma",
        "IIT Hyderabad",
        "Acme",
        "Copilot",
        "Built the thing",
        "Rust",
        "ABOUT THE CANDIDATE",
        "PROJECTS",
    ] {
        assert!(snapshot.contains(needle), "snapshot should contain {needle}");
    }
}
