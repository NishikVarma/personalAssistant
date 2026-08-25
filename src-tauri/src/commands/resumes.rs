use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tauri::State;

use crate::db::resume_files_repo;
use crate::error::{AppError, AppResult};
use crate::llm::{extract_prompt, LlmProvider};
use crate::models::resume::{ExtractedProfile, LatexStatus, ResumeFile, ResumeFileKind};
use crate::state::AppState;

fn expected_extension(kind: ResumeFileKind) -> &'static str {
    match kind {
        ResumeFileKind::PdfMaster => "pdf",
        ResumeFileKind::TexTemplate => "tex",
    }
}

/// Copies an uploaded file into the immutable resumes directory, deduplicating
/// by content hash. Originals are never modified after upload.
#[tauri::command]
pub async fn resume_file_upload(
    state: State<'_, AppState>,
    kind: ResumeFileKind,
    source_path: String,
) -> AppResult<ResumeFile> {
    upload_inner(&state, kind, &source_path).await
}

pub async fn upload_inner(
    state: &AppState,
    kind: ResumeFileKind,
    source_path: &str,
) -> AppResult<ResumeFile> {
    let source = Path::new(source_path.trim());
    if !source.is_file() {
        return Err(AppError::InvalidInput(format!(
            "file not found: {}",
            source.display()
        )));
    }
    let extension = source
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .ok_or_else(|| AppError::InvalidInput("file has no extension".to_string()))?;
    if extension != expected_extension(kind) {
        return Err(AppError::InvalidInput(format!(
            "expected a .{} file for this upload",
            expected_extension(kind)
        )));
    }

    let bytes = tokio::fs::read(source)
        .await
        .map_err(|e| AppError::InvalidInput(format!("could not read file: {e}")))?;
    if bytes.is_empty() {
        return Err(AppError::InvalidInput("file is empty".to_string()));
    }

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let sha256: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();

    if let Some(existing) = resume_files_repo::find_by_sha(&state.pool, &sha256).await? {
        return Ok(existing); // identical content already stored — idempotent upload
    }

    let stored_path: PathBuf = state.resumes_dir.join(format!("{sha256}.{extension}"));
    tokio::fs::write(&stored_path, &bytes)
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(e)))?;

    let original_filename = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("upload.{extension}"));

    resume_files_repo::create(
        &state.pool,
        kind,
        &original_filename,
        &stored_path.to_string_lossy(),
        &sha256,
        bytes.len() as i64,
    )
    .await
}

#[tauri::command]
pub async fn resume_file_list(
    state: State<'_, AppState>,
    kind: Option<ResumeFileKind>,
) -> AppResult<Vec<ResumeFile>> {
    resume_files_repo::list(&state.pool, kind).await
}

/// Deletes the row and the stored file. Variants keep working via ON DELETE SET NULL.
#[tauri::command]
pub async fn resume_file_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    let stored_path = resume_files_repo::delete(&state.pool, id).await?;
    if let Err(e) = tokio::fs::remove_file(&stored_path).await {
        eprintln!("stored file already gone ({stored_path}): {e}");
    }
    Ok(true)
}

/// Returns the raw LaTeX source of a stored .tex template.
#[tauri::command]
pub async fn resume_file_tex_content(state: State<'_, AppState>, id: i64) -> AppResult<String> {
    tex_content_inner(&state, id).await
}

pub async fn tex_content_inner(state: &AppState, id: i64) -> AppResult<String> {
    let file = resume_files_repo::get(&state.pool, id).await?;
    if file.kind != ResumeFileKind::TexTemplate.as_str() {
        return Err(AppError::InvalidInput(
            "only .tex templates can be viewed as source".to_string(),
        ));
    }
    tokio::fs::read_to_string(&file.stored_path)
        .await
        .map_err(|e| AppError::InvalidInput(format!("could not read template: {e}")))
}

/// Probes PATH for an installed LaTeX engine (pdflatex, xelatex, tectonic).
#[tauri::command]
pub async fn latex_detect() -> LatexStatus {
    for engine in ["pdflatex", "xelatex", "tectonic"] {
        let probe = tokio::process::Command::new(engine)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        if matches!(probe, Ok(status) if status.success()) {
            return LatexStatus { available: true, engine: Some(engine.to_string()) };
        }
    }
    LatexStatus { available: false, engine: None }
}

// ---------- AI extraction ----------

pub fn pdf_text(path: &str) -> AppResult<String> {
    pdf_extract::extract_text(path)
        .map_err(|e| AppError::InvalidInput(format!("could not read PDF text: {e}")))
}

pub fn require_text_layer(text: &str) -> AppResult<String> {
    let trimmed = text.trim();
    if trimmed.len() < 40 {
        return Err(AppError::InvalidInput(
            "this PDF has no readable text layer (likely a scan). Use the paste option instead."
                .to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Extracts structured profile data from a stored master resume via Gemini.
/// Read-only: nothing touches the career profile until you approve an import.
#[tauri::command]
pub async fn resume_extract_profile(
    state: State<'_, AppState>,
    id: i64,
) -> AppResult<ExtractedProfile> {
    let file = resume_files_repo::get(&state.pool, id).await?;
    if file.kind != ResumeFileKind::PdfMaster.as_str() {
        return Err(AppError::InvalidInput(
            "only master resume PDFs can be extracted".to_string(),
        ));
    }
    let text = require_text_layer(&pdf_text(&file.stored_path)?)?;
    extract_from_text_inner(&state, &text).await
}

/// Same structuring for manually pasted resume text (scanned-PDF fallback).
#[tauri::command]
pub async fn resume_extract_from_text(
    state: State<'_, AppState>,
    text: String,
) -> AppResult<ExtractedProfile> {
    let text = require_text_layer(&text)?;
    extract_from_text_inner(&state, &text).await
}

async fn extract_from_text_inner(
    state: &AppState,
    text: &str,
) -> AppResult<ExtractedProfile> {
    let provider = super::ai::current_provider(state).await?;
    let prompt = extract_prompt::build_extraction_prompt(text);
    let raw = provider.complete(prompt).await?;
    extract_prompt::parse_extracted_profile(&raw)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCounts {
    pub identity_updated: bool,
    pub education: u64,
    pub experience: u64,
    pub projects: u64,
    pub skills: u64,
    pub certifications: u64,
    pub achievements: u64,
    pub links: u64,
    pub skipped_duplicates: u64,
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Imports reviewed extraction data into the career profile. Rows are created
/// through the standard repos and marked verified (the user approved them in
/// the review UI).
#[tauri::command]
pub async fn profile_import_extracted(
    state: State<'_, AppState>,
    profile: ExtractedProfile,
    mark_verified: bool,
) -> AppResult<ImportCounts> {
    profile_import_extracted_inner(&state.pool, &profile, mark_verified).await
}

pub async fn profile_import_extracted_inner(
    pool: &sqlx::SqlitePool,
    profile: &ExtractedProfile,
    mark_verified: bool,
) -> AppResult<ImportCounts> {
    let mut counts = ImportCounts {
        identity_updated: false,
        education: 0,
        experience: 0,
        projects: 0,
        skills: 0,
        certifications: 0,
        achievements: 0,
        links: 0,
        skipped_duplicates: 0,
    };

    // identity: merge non-empty extracted fields over the existing profile
    if non_empty(&profile.full_name).is_some()
        || non_empty(&profile.email).is_some()
        || non_empty(&profile.phone).is_some()
        || non_empty(&profile.location).is_some()
        || non_empty(&profile.summary).is_some()
    {
        let current = crate::db::profile_repo::get(pool).await?;
        let merged = crate::models::profile::UserProfileInput {
            full_name: non_empty(&profile.full_name).unwrap_or(current.full_name),
            email: non_empty(&profile.email).unwrap_or(current.email),
            phone: non_empty(&profile.phone).unwrap_or(current.phone),
            location: non_empty(&profile.location).unwrap_or(current.location),
            summary: non_empty(&profile.summary).unwrap_or(current.summary),
        };
        crate::db::profile_repo::update(pool, &merged).await?;
        counts.identity_updated = true;
    }

    for education in &profile.education {
        if education.institution.trim().is_empty() {
            continue;
        }
        let created = crate::db::education_repo::create(
            pool,
            &crate::models::profile::EducationInput {
                institution: education.institution.clone(),
                degree: education.degree.clone(),
                field_of_study: education.field_of_study.clone(),
                start_date: education.start_date.clone(),
                end_date: education.end_date.clone(),
                grade: education.grade.clone(),
                location: education.location.clone(),
                details: education.details.clone(),
            },
        )
        .await?;
        if mark_verified {
            crate::db::education_repo::set_verified(pool, created.id, true).await?;
        }
        counts.education += 1;
    }

    for experience in &profile.experience {
        if experience.organization.trim().is_empty() || experience.title.trim().is_empty() {
            continue;
        }
        let employment_type = experience
            .employment_type
            .as_deref()
            .and_then(crate::models::profile::EmploymentType::try_from_str)
            .unwrap_or(crate::models::profile::EmploymentType::FullTime);
        let created = crate::db::experience_repo::create(
            pool,
            &crate::models::profile::ExperienceInput {
                organization: experience.organization.clone(),
                title: experience.title.clone(),
                employment_type,
                location: experience.location.clone(),
                start_date: experience.start_date.clone(),
                end_date: experience.end_date.clone(),
                currently_working: experience.currently_working,
                description: experience.description.clone(),
            },
        )
        .await?;
        if mark_verified {
            crate::db::experience_repo::set_verified(pool, created.id, true).await?;
        }
        counts.experience += 1;
    }

    for project in &profile.projects {
        if project.name.trim().is_empty() {
            continue;
        }
        let created = crate::db::projects_repo::create(
            pool,
            &crate::models::profile::ProjectInput {
                name: project.name.clone(),
                description: project.description.clone(),
                repo_url: project.repo_url.clone(),
                live_url: project.live_url.clone(),
                status: crate::models::profile::ProjectStatus::Completed,
                started_on: project.started_on.clone(),
                ended_on: project.ended_on.clone(),
            },
        )
        .await?;
        if mark_verified {
            crate::db::projects_repo::set_verified(pool, created.id, true).await?;
        }
        counts.projects += 1;
    }

    for skill in &profile.skills {
        if skill.name.trim().is_empty() {
            continue;
        }
        let category = skill
            .category
            .as_deref()
            .and_then(crate::models::profile::SkillCategory::try_from_str)
            .unwrap_or(crate::models::profile::SkillCategory::Other);
        match crate::db::skills_repo::create(
            pool,
            &crate::models::profile::SkillInput { name: skill.name.clone(), category },
        )
        .await
        {
            Ok(created) => {
                counts.skills += 1;
                if mark_verified {
                    // skills have no verified column; linking is the source of truth
                    let _ = created;
                }
            }
            Err(AppError::InvalidInput(_)) => counts.skipped_duplicates += 1,
            Err(e) => return Err(e),
        }
    }

    for certification in &profile.certifications {
        if certification.name.trim().is_empty() {
            continue;
        }
        let created = crate::db::certifications_repo::create(
            pool,
            &crate::models::profile::CertificationInput {
                name: certification.name.clone(),
                issuer: certification.issuer.clone(),
                issue_date: certification.issue_date.clone(),
                expiry_date: certification.expiry_date.clone(),
                credential_url: certification.credential_url.clone(),
            },
        )
        .await?;
        if mark_verified {
            crate::db::certifications_repo::set_verified(pool, created.id, true).await?;
        }
        counts.certifications += 1;
    }

    for achievement in &profile.achievements {
        if achievement.title.trim().is_empty() {
            continue;
        }
        let created = crate::db::achievements_repo::create(
            pool,
            &crate::models::profile::AchievementInput {
                title: achievement.title.clone(),
                description: achievement.description.clone(),
                date: achievement.date.clone(),
            },
        )
        .await?;
        if mark_verified {
            crate::db::achievements_repo::set_verified(pool, created.id, true).await?;
        }
        counts.achievements += 1;
    }

    for link in &profile.links {
        if link.url.trim().is_empty() {
            continue;
        }
        let kind = link
            .kind
            .as_deref()
            .and_then(crate::models::profile::LinkKind::try_from_str)
            .unwrap_or(crate::models::profile::LinkKind::Other);
        crate::db::links_repo::create(
            pool,
            &crate::models::profile::LinkInput {
                label: link.label.clone(),
                url: link.url.clone(),
                kind,
            },
        )
        .await?;
        counts.links += 1;
    }

    Ok(counts)
}

// ---------- JD matching & tailored generation ----------

use crate::db::{profile_snapshot, resume_variants_repo};
use crate::llm::jd_prompt;
use crate::models::resume::{JdAnalysis, ResumeVariant};

/// Analyzes a job description against the verified career profile.
#[tauri::command]
pub async fn resume_match_jd(
    state: State<'_, AppState>,
    jd_text: String,
) -> AppResult<JdAnalysis> {
    let jd = jd_text.trim();
    if jd.len() < 40 {
        return Err(AppError::InvalidInput(
            "paste a fuller job description (at least 40 characters)".to_string(),
        ));
    }
    let profile_block = profile_snapshot::collect(&state.pool).await?;
    let provider = super::ai::current_provider(&state).await?;
    let prompt = jd_prompt::build_match_prompt(jd, &profile_block);
    let raw = provider.complete(prompt).await?;
    jd_prompt::parse_jd_analysis(&raw)
}

/// Compiles a .tex file with the detected engine. Returns the PDF path on
/// success, None when no engine is installed or compilation fails.
async fn compile_tex(tex_path: &Path) -> AppResult<Option<PathBuf>> {
    for engine in ["pdflatex", "xelatex", "tectonic"] {
        let probe = tokio::process::Command::new(engine)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        if !matches!(probe, Ok(status) if status.success()) {
            continue;
        }

        let dir = tex_path
            .parent()
            .ok_or_else(|| AppError::InvalidInput("invalid tex path".to_string()))?;
        let pdf_path = tex_path.with_extension("pdf");
        let output = tokio::process::Command::new(engine)
            .arg("-interaction=nonstopmode")
            .arg("-output-directory")
            .arg(dir)
            .arg(tex_path)
            .current_dir(dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;

        if matches!(output, Ok(status) if status.success()) && pdf_path.is_file() {
            return Ok(Some(pdf_path));
        }
        return Ok(None); // engine present but compilation failed — tex-only fallback
    }
    Ok(None) // no engine installed
}

/// Generates a tailored LaTeX resume from the user's template, verified
/// profile and a job description. Compiles to PDF when LaTeX is available.
#[tauri::command]
pub async fn resume_generate_variant(
    state: State<'_, AppState>,
    jd_text: String,
    template_id: Option<i64>,
    application_id: Option<i64>,
) -> AppResult<ResumeVariant> {
    let jd = jd_text.trim().to_string();
    if jd.len() < 40 {
        return Err(AppError::InvalidInput(
            "paste a fuller job description (at least 40 characters)".to_string(),
        ));
    }

    let template = match template_id {
        Some(id) => Some(resume_files_repo::get(&state.pool, id).await?),
        None => resume_files_repo::list(&state.pool, Some(ResumeFileKind::TexTemplate))
            .await?
            .into_iter()
            .next(),
    };
    let template = template
        .ok_or_else(|| AppError::InvalidInput("upload a .tex template first".to_string()))?;
    if template.kind != ResumeFileKind::TexTemplate.as_str() {
        return Err(AppError::InvalidInput(
            "the selected file is not a .tex template".to_string(),
        ));
    }

    let profile_block = profile_snapshot::collect(&state.pool).await?;
    let provider = super::ai::current_provider(&state).await?;
    let analysis: JdAnalysis = {
        let prompt = jd_prompt::build_match_prompt(&jd, &profile_block);
        let raw = provider.complete(prompt).await?;
        jd_prompt::parse_jd_analysis(&raw)?
    };
    let category = analysis
        .recommended_category
        .as_deref()
        .and_then(crate::models::resume::ResumeCategory::try_from_str)
        .unwrap_or(crate::models::resume::ResumeCategory::GeneralSwe);

    let template_tex = tokio::fs::read_to_string(&template.stored_path)
        .await
        .map_err(|e| AppError::InvalidInput(format!("could not read template: {e}")))?;
    let gen_prompt = jd_prompt::build_generation_prompt(
        &template_tex,
        &profile_block,
        &jd,
        Some(category.as_str()),
    );
    let raw = provider.complete(gen_prompt).await?;
    let latex = jd_prompt::strip_latex_fences(&raw);
    if !latex.contains("\\documentclass") {
        return Err(AppError::InvalidInput(
            "the AI response was not valid LaTeX — try regenerating".to_string(),
        ));
    }

    let label = if analysis.role.is_empty() {
        "Tailored resume".to_string()
    } else {
        format!("Tailored — {}", analysis.role)
    };
    let variant = resume_variants_repo::create(
        &state.pool,
        Some(template.id),
        application_id,
        category,
        &label,
    )
    .await?;

    let tex_path = state.resumes_dir.join(format!("variant-{}.tex", variant.id));
    tokio::fs::write(&tex_path, &latex)
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(e)))?;

    let pdf_path = compile_tex(&tex_path).await?;
    let tex_str = tex_path.to_string_lossy().into_owned();
    let pdf_str = pdf_path.as_ref().map(|p| p.to_string_lossy().into_owned());
    resume_variants_repo::set_paths(
        &state.pool,
        variant.id,
        Some(&tex_str),
        pdf_str.as_deref(),
    )
    .await?;

    let mut variant = variant;
    variant.tex_path = Some(tex_path.to_string_lossy().into_owned());
    variant.pdf_path = pdf_path.as_ref().map(|p| p.to_string_lossy().into_owned());
    Ok(variant)
}

#[tauri::command]
pub async fn resume_variant_list(
    state: State<'_, AppState>,
    application_id: Option<i64>,
) -> AppResult<Vec<ResumeVariant>> {
    resume_variants_repo::list(&state.pool, application_id).await
}

#[tauri::command]
pub async fn resume_variant_tex_content(
    state: State<'_, AppState>,
    id: i64,
) -> AppResult<String> {
    let variant = resume_variants_repo::get(&state.pool, id).await?;
    let tex_path = variant
        .tex_path
        .ok_or_else(|| AppError::InvalidInput("this variant has no .tex source".to_string()))?;
    tokio::fs::read_to_string(&tex_path)
        .await
        .map_err(|e| AppError::InvalidInput(format!("could not read variant: {e}")))
}

#[tauri::command]
pub async fn resume_variant_approve(
    state: State<'_, AppState>,
    id: i64,
) -> AppResult<ResumeVariant> {
    resume_variants_repo::approve(&state.pool, id).await
}

/// Deletes a variant and its stored .tex/.pdf files.
#[tauri::command]
pub async fn resume_variant_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    let (tex_path, pdf_path) = resume_variants_repo::delete(&state.pool, id).await?;
    for path in [tex_path, pdf_path].into_iter().flatten() {
        if let Err(e) = tokio::fs::remove_file(&path).await {
            eprintln!("stored file already gone ({path}): {e}");
        }
    }
    Ok(true)
}
