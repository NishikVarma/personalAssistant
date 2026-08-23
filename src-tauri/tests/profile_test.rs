use assistant_lib::db::{
    self, achievements_repo, bullets_repo, certifications_repo, education_repo, experience_repo,
    links_repo, profile_repo, projects_repo, skills_repo,
};
use assistant_lib::models::profile::*;
use sqlx::SqlitePool;

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = db::open(dir.path().join("test.db")).await.expect("pool");
    (pool, dir)
}

fn education_input() -> EducationInput {
    EducationInput {
        institution: "IIT Hyderabad".to_string(),
        degree: "B.Tech".to_string(),
        field_of_study: "Computer Science".to_string(),
        start_date: Some("2022-08-01".to_string()),
        end_date: None,
        grade: Some("8.9 CGPA".to_string()),
        location: Some("Hyderabad".to_string()),
        details: "Minor in AI".to_string(),
    }
}

#[tokio::test]
async fn user_profile_defaults_then_updates() {
    let (pool, _dir) = test_pool().await;

    let profile = profile_repo::get(&pool).await.unwrap();
    assert_eq!(profile.id, 1);
    assert!(!profile.verified);
    assert_eq!(profile.full_name, "");

    let updated = profile_repo::update(
        &pool,
        &UserProfileInput {
            full_name: "Nishik Varma".to_string(),
            email: "nishik@example.com".to_string(),
            phone: "+91 90000 00000".to_string(),
            location: "India".to_string(),
            summary: "Backend engineer.".to_string(),
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.full_name, "Nishik Varma");
    assert!(updated.email.contains('@'));

    profile_repo::set_verified(&pool, true).await.unwrap();
    assert!(profile_repo::get(&pool).await.unwrap().verified);

    let bad = profile_repo::update(
        &pool,
        &UserProfileInput {
            email: "not-an-email".to_string(),
            ..Default::default()
        },
    )
    .await;
    assert!(matches!(bad, Err(assistant_lib::error::AppError::InvalidInput(_))));
}

#[tokio::test]
async fn education_crud_and_validation() {
    let (pool, _dir) = test_pool().await;

    let created = education_repo::create(&pool, &education_input()).await.unwrap();
    assert_eq!(created.institution, "IIT Hyderabad");
    assert!(!created.verified);

    let list = education_repo::list(&pool).await.unwrap();
    assert_eq!(list.len(), 1);

    let mut edited = education_input();
    edited.institution = "NIT Warangal".to_string();
    let updated = education_repo::update(&pool, created.id, &edited).await.unwrap();
    assert_eq!(updated.institution, "NIT Warangal");

    education_repo::set_verified(&pool, created.id, true).await.unwrap();
    assert!(education_repo::get(&pool, created.id).await.unwrap().verified);

    assert!(education_repo::delete(&pool, created.id).await.unwrap());
    assert!(!education_repo::delete(&pool, created.id).await.unwrap());
    assert!(matches!(
        education_repo::get(&pool, created.id).await,
        Err(assistant_lib::error::AppError::NotFound(_))
    ));

    let mut invalid = education_input();
    invalid.institution = "   ".to_string();
    assert!(matches!(
        education_repo::create(&pool, &invalid).await,
        Err(assistant_lib::error::AppError::InvalidInput(_))
    ));
}

#[tokio::test]
async fn experience_crud_with_enum_type() {
    let (pool, _dir) = test_pool().await;

    let input = ExperienceInput {
        organization: "Acme Corp".to_string(),
        title: "SDE Intern".to_string(),
        employment_type: EmploymentType::Internship,
        location: Some("Remote".to_string()),
        start_date: Some("2025-05-01".to_string()),
        end_date: Some("2025-08-01".to_string()),
        currently_working: false,
        description: "Built internal tooling.".to_string(),
    };

    let created = experience_repo::create(&pool, &input).await.unwrap();
    assert_eq!(created.employment_type, "internship");

    let mut edited = input.clone();
    edited.employment_type = EmploymentType::FullTime;
    edited.currently_working = true;
    let updated = experience_repo::update(&pool, created.id, &edited).await.unwrap();
    assert_eq!(updated.employment_type, "full_time");
    assert!(updated.currently_working);

    experience_repo::set_verified(&pool, created.id, true).await.unwrap();
    assert!(experience_repo::get(&pool, created.id).await.unwrap().verified);

    let invalid = ExperienceInput {
        organization: "".to_string(),
        ..input.clone()
    };
    assert!(experience_repo::create(&pool, &invalid).await.is_err());
}

#[tokio::test]
async fn projects_crud_with_status() {
    let (pool, _dir) = test_pool().await;

    let input = ProjectInput {
        name: "Job Application Copilot".to_string(),
        description: "Local-first copilot.".to_string(),
        repo_url: Some("https://github.com/nishikv/copilot".to_string()),
        live_url: None,
        status: ProjectStatus::Ongoing,
        started_on: Some("2026-01-15".to_string()),
        ended_on: None,
    };

    let created = projects_repo::create(&pool, &input).await.unwrap();
    assert_eq!(created.status, "ongoing");

    let mut edited = input.clone();
    edited.status = ProjectStatus::Completed;
    let updated = projects_repo::update(&pool, created.id, &edited).await.unwrap();
    assert_eq!(updated.status, "completed");

    projects_repo::set_verified(&pool, created.id, true).await.unwrap();
    assert!(projects_repo::get(&pool, created.id).await.unwrap().verified);
}

#[tokio::test]
async fn skills_duplicate_names_rejected_and_entity_linking_works() {
    let (pool, _dir) = test_pool().await;

    let project = projects_repo::create(
        &pool,
        &ProjectInput {
            name: "Copilot".to_string(),
            description: String::new(),
            repo_url: None,
            live_url: None,
            status: ProjectStatus::Completed,
            started_on: None,
            ended_on: None,
        },
    )
    .await
    .unwrap();

    let rust = skills_repo::create(
        &pool,
        &SkillInput { name: "Rust".to_string(), category: SkillCategory::Language },
    )
    .await
    .unwrap();
    let sqlx_skill = skills_repo::create(
        &pool,
        &SkillInput { name: "sqlx".to_string(), category: SkillCategory::Framework },
    )
    .await
    .unwrap();

    // case-insensitive duplicate
    let dup = skills_repo::create(
        &pool,
        &SkillInput { name: "RUST".to_string(), category: SkillCategory::Language },
    )
    .await;
    assert!(matches!(dup, Err(assistant_lib::error::AppError::InvalidInput(_))));

    skills_repo::replace_entity_skills(
        &pool,
        ProfileEntityType::Project,
        project.id,
        &[rust.id, sqlx_skill.id],
    )
    .await
    .unwrap();

    let linked = skills_repo::list_for_entity(&pool, ProfileEntityType::Project, project.id)
        .await
        .unwrap();
    assert_eq!(linked.len(), 2);

    // missing skill id surfaces NotFound, not a raw FK error
    let missing =
        skills_repo::replace_entity_skills(&pool, ProfileEntityType::Project, project.id, &[9999])
            .await;
    assert!(matches!(missing, Err(assistant_lib::error::AppError::NotFound(_))));

    // nonexistent entity rejected
    let no_entity =
        skills_repo::replace_entity_skills(&pool, ProfileEntityType::Experience, 9999, &[]).await;
    assert!(matches!(no_entity, Err(assistant_lib::error::AppError::NotFound(_))));

    // deleting a skill cascades its entity_skills rows
    skills_repo::delete(&pool, rust.id).await.unwrap();
    let linked = skills_repo::list_for_entity(&pool, ProfileEntityType::Project, project.id)
        .await
        .unwrap();
    assert_eq!(linked.len(), 1);
}

#[tokio::test]
async fn bullets_belong_to_entities_and_cleanup_on_delete() {
    let (pool, _dir) = test_pool().await;

    let project = projects_repo::create(
        &pool,
        &ProjectInput {
            name: "Copilot".to_string(),
            description: String::new(),
            repo_url: None,
            live_url: None,
            status: ProjectStatus::Ongoing,
            started_on: None,
            ended_on: None,
        },
    )
    .await
    .unwrap();

    let b2 = bullets_repo::create(
        &pool,
        ProfileEntityType::Project,
        project.id,
        &BulletInput { content: "Second bullet".to_string(), display_order: 2 },
    )
    .await
    .unwrap();
    let b1 = bullets_repo::create(
        &pool,
        ProfileEntityType::Project,
        project.id,
        &BulletInput { content: "First bullet".to_string(), display_order: 1 },
    )
    .await
    .unwrap();

    // bullet for nonexistent entity is rejected
    let orphan = bullets_repo::create(
        &pool,
        ProfileEntityType::Experience,
        4242,
        &BulletInput { content: "orphan".to_string(), display_order: 0 },
    )
    .await;
    assert!(matches!(orphan, Err(assistant_lib::error::AppError::NotFound(_))));

    let ordered = bullets_repo::list_for_entity(&pool, ProfileEntityType::Project, project.id)
        .await
        .unwrap();
    let contents: Vec<&str> = ordered.iter().map(|b| b.content.as_str()).collect();
    assert_eq!(contents, vec!["First bullet", "Second bullet"]);

    bullets_repo::set_verified(&pool, b1.id, true).await.unwrap();
    assert!(bullets_repo::get(&pool, b1.id).await.unwrap().verified);

    bullets_repo::update(
        &pool,
        b2.id,
        &BulletInput { content: "Rewritten bullet".to_string(), display_order: 3 },
    )
    .await
    .unwrap();
    assert_eq!(
        bullets_repo::get(&pool, b2.id).await.unwrap().content,
        "Rewritten bullet"
    );

    // deleting the parent entity removes its bullets
    projects_repo::delete(&pool, project.id).await.unwrap();
    let remaining = bullets_repo::list_for_entity(&pool, ProfileEntityType::Project, project.id)
        .await
        .unwrap();
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn certifications_achievements_links_roundtrip() {
    let (pool, _dir) = test_pool().await;

    let cert = certifications_repo::create(
        &pool,
        &CertificationInput {
            name: "AWS CCP".to_string(),
            issuer: "Amazon".to_string(),
            issue_date: Some("2025-06-01".to_string()),
            expiry_date: None,
            credential_url: None,
        },
    )
    .await
    .unwrap();
    certifications_repo::set_verified(&pool, cert.id, true).await.unwrap();
    assert!(certifications_repo::get(&pool, cert.id).await.unwrap().verified);

    let achievement = achievements_repo::create(
        &pool,
        &AchievementInput {
            title: "Smart India Hackathon finalist".to_string(),
            description: "Top 5 of 40k teams.".to_string(),
            date: Some("2024-12-10".to_string()),
        },
    )
    .await
    .unwrap();
    achievements_repo::set_verified(&pool, achievement.id, true)
        .await
        .unwrap();
    assert!(achievements_repo::get(&pool, achievement.id).await.unwrap().verified);

    let link = links_repo::create(
        &pool,
        &LinkInput {
            label: "GitHub".to_string(),
            url: "https://github.com/nishikv".to_string(),
            kind: LinkKind::GitHub,
        },
    )
    .await
    .unwrap();
    assert_eq!(link.kind, "github");

    let edited = links_repo::update(
        &pool,
        link.id,
        &LinkInput {
            label: "Portfolio".to_string(),
            url: "https://nishik.dev".to_string(),
            kind: LinkKind::Portfolio,
        },
    )
    .await
    .unwrap();
    assert_eq!(edited.kind, "portfolio");

    assert!(links_repo::delete(&pool, link.id).await.unwrap());
    assert!(certifications_repo::delete(&pool, cert.id).await.unwrap());
    assert!(achievements_repo::delete(&pool, achievement.id).await.unwrap());

    assert!(links_repo::create(
        &pool,
        &LinkInput { label: String::new(), url: String::new(), kind: LinkKind::Other }
    )
    .await
    .is_err());
}

#[tokio::test]
async fn deleting_experience_cleans_up_linked_rows() {
    let (pool, _dir) = test_pool().await;

    let job = experience_repo::create(
        &pool,
        &ExperienceInput {
            organization: "Acme".to_string(),
            title: "Intern".to_string(),
            employment_type: EmploymentType::Internship,
            location: None,
            start_date: None,
            end_date: None,
            currently_working: false,
            description: String::new(),
        },
    )
    .await
    .unwrap();

    skills_repo::replace_entity_skills(&pool, ProfileEntityType::Experience, job.id, &[]).await.unwrap();
    bullets_repo::create(
        &pool,
        ProfileEntityType::Experience,
        job.id,
        &BulletInput { content: "Shipped thing".to_string(), display_order: 0 },
    )
    .await
    .unwrap();

    experience_repo::delete(&pool, job.id).await.unwrap();

    let bullets = bullets_repo::list_for_entity(&pool, ProfileEntityType::Experience, job.id)
        .await
        .unwrap();
    assert!(bullets.is_empty());
}
