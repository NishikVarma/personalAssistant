use std::sync::Arc;

use assistant_lib::db::{self, resume_files_repo};
use assistant_lib::gmail::OauthCoordinator;
use assistant_lib::llm::secrets::MemoryStore;
use assistant_lib::models::resume::*;
use assistant_lib::state::AppState;
use std::path::PathBuf;
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

#[tokio::test]
async fn resume_file_roundtrip_and_lookup() {
    let (pool, _dir) = test_pool().await;

    let created = resume_files_repo::create(
        &pool,
        ResumeFileKind::PdfMaster,
        "Nishik_Resume.pdf",
        "/data/resumes/abc123.pdf",
        "abc123",
        12345,
    )
    .await
    .unwrap();
    assert_eq!(created.kind, "pdf_master");
    assert_eq!(created.original_filename, "Nishik_Resume.pdf");
    assert_eq!(created.file_size, 12345);

    let found = resume_files_repo::find_by_sha(&pool, "abc123")
        .await
        .unwrap()
        .expect("sha lookup should find the row");
    assert_eq!(found.id, created.id);
    assert!(resume_files_repo::find_by_sha(&pool, "nope").await.unwrap().is_none());

    let pdfs = resume_files_repo::list(&pool, Some(ResumeFileKind::PdfMaster))
        .await
        .unwrap();
    assert_eq!(pdfs.len(), 1);
    let all = resume_files_repo::list(&pool, None).await.unwrap();
    assert_eq!(all.len(), 1);

    // delete returns the stored path for the caller to clean up
    let stored = resume_files_repo::delete(&pool, created.id).await.unwrap();
    assert_eq!(stored, "/data/resumes/abc123.pdf");
    assert!(resume_files_repo::get(&pool, created.id).await.is_err());
}

#[tokio::test]
async fn upload_copies_content_addressed_and_dedups() {
    let (pool, data_dir) = test_pool().await;
    let resumes_dir = data_dir.path().join("resumes");
    std::fs::create_dir_all(&resumes_dir).unwrap();

    let source_dir = tempfile::tempdir().unwrap();
    let source = source_dir.path().join("My_Resume.pdf");
    std::fs::write(&source, b"%PDF-1.4 fake resume content").unwrap();

    let state = AppState {
        pool: pool.clone(),
        db_path: String::new(),
        secrets: Arc::new(MemoryStore::new()),
        oauth: Arc::new(OauthCoordinator::new()),
        resumes_dir: resumes_dir.clone(),
    };

    let uploaded =
        assistant_lib::commands::resumes::upload_inner(&state, ResumeFileKind::PdfMaster, source.to_str().unwrap())
            .await
            .unwrap();

    // stored content-addressed, original untouched
    let stored = PathBuf::from(&uploaded.stored_path);
    assert!(stored.starts_with(&resumes_dir));
    assert_eq!(stored.extension().unwrap(), "pdf");
    assert_eq!(std::fs::read(&stored).unwrap(), b"%PDF-1.4 fake resume content");
    assert_eq!(uploaded.original_filename, "My_Resume.pdf");
    assert_eq!(uploaded.file_size, 28);

    // re-uploading identical content returns the same row (idempotent)
    let again =
        assistant_lib::commands::resumes::upload_inner(&state, ResumeFileKind::PdfMaster, source.to_str().unwrap())
            .await
            .unwrap();
    assert_eq!(again.id, uploaded.id);
    assert_eq!(resume_files_repo::list(&pool, None).await.unwrap().len(), 1);

    // wrong extension rejected per kind
    let tex_source = source_dir.path().join("template.tex");
    std::fs::write(tex_source.to_str().unwrap(), b"\\documentclass{article}").unwrap();
    let wrong_kind = assistant_lib::commands::resumes::upload_inner(
        &state,
        ResumeFileKind::PdfMaster,
        tex_source.to_str().unwrap(),
    )
    .await;
    assert!(matches!(wrong_kind, Err(assistant_lib::error::AppError::InvalidInput(_))));

    // missing file rejected
    let missing = assistant_lib::commands::resumes::upload_inner(
        &state,
        ResumeFileKind::PdfMaster,
        source_dir.path().join("nope.pdf").to_str().unwrap(),
    )
    .await;
    assert!(missing.is_err());

    // tex template upload works and content can be read back
    let tex = assistant_lib::commands::resumes::upload_inner(
        &state,
        ResumeFileKind::TexTemplate,
        tex_source.to_str().unwrap(),
    )
    .await
    .unwrap();
    let content = assistant_lib::commands::resumes::tex_content_inner(
        &state, tex.id,
    )
    .await
    .unwrap();
    assert!(content.contains("documentclass"));
}

#[test]
fn extraction_prompt_contains_rules_and_text() {
    let prompt = assistant_lib::llm::extract_prompt::build_extraction_prompt(
        "Jane Doe — Backend Engineer at Acme",
    );
    assert!(prompt.contains("ONLY facts"));
    assert!(prompt.contains("Jane Doe — Backend Engineer at Acme"));
    assert!(prompt.contains("employmentType"));
    assert!(prompt.contains("\"fullName\""));
}

#[test]
fn parses_extracted_profile_from_fenced_json() {
    let raw = "Here you go:\n```json\n{\"fullName\": \"Jane Doe\", \"email\": \"jane@acme.com\", \
        \"phone\": \"\", \"location\": \"Hyderabad\", \"summary\": \"Engineer\", \
        \"education\": [{\"institution\": \"IIT\", \"degree\": \"B.Tech\", \"fieldOfStudy\": \"CS\", \
        \"startDate\": \"2022-08\", \"endDate\": null, \"grade\": null, \"location\": null, \"details\": \"\"}], \
        \"experience\": [], \"projects\": [], \
        \"skills\": [{\"name\": \"Rust\", \"category\": \"language\"}], \
        \"certifications\": [], \"achievements\": [], \"links\": []}\n```";
    let profile = assistant_lib::llm::extract_prompt::parse_extracted_profile(raw).unwrap();
    assert_eq!(profile.full_name, "Jane Doe");
    assert_eq!(profile.education[0].institution, "IIT");
    assert_eq!(profile.education[0].start_date.as_deref(), Some("2022-08"));
    assert_eq!(profile.skills[0].category.as_deref(), Some("language"));
    assert!(profile.experience.is_empty());
    assert!(assistant_lib::llm::extract_prompt::parse_extracted_profile("no json").is_err());
}

#[tokio::test]
async fn scanned_pdf_error_mentions_paste_fallback() {
    let (pool, data_dir) = test_pool().await;
    let resumes_dir = data_dir.path().join("resumes");
    std::fs::create_dir_all(&resumes_dir).unwrap();

    let stored = resumes_dir.join("emptyish.pdf");
    std::fs::write(&stored, b"%PDF-1.4\n%%EOF").unwrap();
    let file = resume_files_repo::create(
        &pool,
        ResumeFileKind::PdfMaster,
        "scan.pdf",
        &stored.to_string_lossy(),
        "deadbeef",
        13,
    )
    .await
    .unwrap();

    let result = assistant_lib::commands::resumes::pdf_text(&file.stored_path);
    match result {
        Err(e) => assert!(!e.to_string().is_empty()),
        Ok(text) => {
            let err = assistant_lib::commands::resumes::require_text_layer(&text).unwrap_err();
            assert!(err.to_string().contains("paste"));
        }
    }
}

#[tokio::test]
async fn import_creates_verified_profile_rows() {
    let (pool, _dir) = test_pool().await;

    let profile = ExtractedProfile {
        full_name: "Nishik Varma".to_string(),
        email: "nishik@example.com".to_string(),
        location: "India".to_string(),
        summary: "Backend engineer.".to_string(),
        education: vec![ExtractedEducation {
            institution: "IIT Hyderabad".to_string(),
            degree: "B.Tech".to_string(),
            field_of_study: "CS".to_string(),
            start_date: Some("2022-08".to_string()),
            ..Default::default()
        }],
        experience: vec![ExtractedExperience {
            organization: "Acme".to_string(),
            title: "SDE Intern".to_string(),
            employment_type: Some("internship".to_string()),
            currently_working: false,
            description: "Built things".to_string(),
            ..Default::default()
        }],
        projects: vec![ExtractedProject {
            name: "Copilot".to_string(),
            description: "Local-first copilot".to_string(),
            ..Default::default()
        }],
        skills: vec![
            ExtractedSkill { name: "Rust".to_string(), category: Some("language".to_string()) },
            ExtractedSkill { name: "rust".to_string(), category: Some("language".to_string()) },
        ],
        links: vec![ExtractedLink {
            label: String::new(),
            url: "https://github.com/nishikv".to_string(),
            kind: Some("github".to_string()),
        }],
        ..Default::default()
    };

    let counts = assistant_lib::commands::resumes::profile_import_extracted_inner(
        &pool, &profile, true,
    )
    .await
    .unwrap();

    assert!(counts.identity_updated);
    assert_eq!(counts.education, 1);
    assert_eq!(counts.experience, 1);
    assert_eq!(counts.projects, 1);
    assert_eq!(counts.skills, 1);
    assert_eq!(counts.skipped_duplicates, 1, "case-insensitive duplicate skipped");
    assert_eq!(counts.links, 1, "empty label auto-derives from kind");

    let education = assistant_lib::db::education_repo::list(&pool).await.unwrap();
    assert!(education[0].verified);

    let profile_row = assistant_lib::db::profile_repo::get(&pool).await.unwrap();
    assert_eq!(profile_row.full_name, "Nishik Varma");
}
