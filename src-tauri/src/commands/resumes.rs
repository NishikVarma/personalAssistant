use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tauri::State;

use crate::db::resume_files_repo;
use crate::error::{AppError, AppResult};
use crate::models::resume::{LatexStatus, ResumeFile, ResumeFileKind};
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
