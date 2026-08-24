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
