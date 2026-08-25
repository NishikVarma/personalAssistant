use std::collections::HashSet;
use std::path::Path;

use calamine::Reader;
use tauri::State;

use crate::db::{bulk_batches_repo, contacts_repo, generated_emails_repo};
use crate::error::{AppError, AppResult};
use crate::models::bulk::{
    BulkBatch, BulkColumnMapping, BulkImportPreview, BulkRowStatus,
};
use crate::models::email::EmailType;
use crate::state::AppState;

use super::emails;

// ---------- spreadsheet parsing ----------

pub fn parse_csv_file(path: &str) -> AppResult<(Vec<String>, Vec<Vec<String>>, usize)> {
    let mut reader = csv::Reader::from_path(path)
        .map_err(|e| AppError::InvalidInput(format!("could not read CSV: {e}")))?;
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| AppError::InvalidInput(format!("could not read CSV headers: {e}")))?
        .iter()
        .map(|cell| cell.trim().to_string())
        .collect();
    if headers.is_empty() {
        return Err(AppError::InvalidInput("CSV has no header row".to_string()));
    }

    let mut sample: Vec<Vec<String>> = Vec::new();
    let mut total = 0usize;
    for record in reader.records() {
        let record =
            record.map_err(|e| AppError::InvalidInput(format!("bad CSV row: {e}")))?;
        let row: Vec<String> = record.iter().map(|cell| cell.trim().to_string()).collect();
        total += 1;
        if sample.len() < 10 {
            sample.push(row);
        }
    }
    Ok((headers, sample, total))
}

pub fn parse_xlsx_file(path: &str) -> AppResult<(Vec<String>, Vec<Vec<String>>, usize)> {
    let mut workbook: calamine::Xlsx<_> =
        calamine::open_workbook(path).map_err(|e| {
            AppError::InvalidInput(format!("could not read XLSX: {e}"))
        })?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| AppError::InvalidInput("workbook has no sheets".to_string()))?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::InvalidInput(format!("could not read first sheet: {e}")))?;

    let mut rows = range.rows();
    let headers: Vec<String> = rows
        .next()
        .ok_or_else(|| AppError::InvalidInput("sheet is empty".to_string()))?
        .iter()
        .map(|cell| cell.to_string().trim().to_string())
        .collect();

    let mut sample: Vec<Vec<String>> = Vec::new();
    let mut total = 0usize;
    for record in rows {
        let row: Vec<String> = record.iter().map(|cell| cell.to_string().trim().to_string()).collect();
        total += 1;
        if sample.len() < 10 {
            sample.push(row);
        }
    }
    Ok((headers, sample, total))
}

fn parse_spreadsheet(path: &str) -> AppResult<(Vec<String>, Vec<Vec<String>>, usize)> {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "csv" => parse_csv_file(path),
        "xlsx" | "xls" => parse_xlsx_file(path),
        other => Err(AppError::InvalidInput(format!(
            "unsupported file type '{other}' — use .csv or .xlsx"
        ))),
    }
}

#[tauri::command]
pub async fn bulk_import_preview(
    state: State<'_, AppState>,
    source_path: String,
) -> AppResult<BulkImportPreview> {
    let path = source_path.trim();
    if !Path::new(path).is_file() {
        return Err(AppError::InvalidInput(format!("file not found: {path}")));
    }
    let (headers, sample_rows, total_data_rows) = parse_spreadsheet(path)?;
    let _ = &state; // AppState unused today; kept for symmetry with other commands
    Ok(BulkImportPreview { headers, sample_rows, total_data_rows })
}

// ---------- batches ----------

#[tauri::command]
pub async fn bulk_batch_create(
    state: State<'_, AppState>,
    email_type: EmailType,
    application_id: Option<i64>,
) -> AppResult<BulkBatch> {
    bulk_batches_repo::create(&state.pool, email_type, application_id).await
}

#[tauri::command]
pub async fn bulk_batch_list(state: State<'_, AppState>) -> AppResult<Vec<BulkBatch>> {
    bulk_batches_repo::list(&state.pool).await
}

#[tauri::command]
pub async fn bulk_batch_get(state: State<'_, AppState>, id: i64) -> AppResult<BulkBatch> {
    bulk_batches_repo::get(&state.pool, id).await
}

// ---------- generation ----------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkGenerateConfig {
    pub role: Option<String>,
    pub company: Option<String>,
    pub job_description: Option<String>,
}

fn mapped(row: &[String], headers: &[String], column: &str) -> String {
    headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(column))
        .and_then(|idx| row.get(idx))
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

fn mapped_or_config(row_value: &str, config_value: &Option<String>) -> Option<String> {
    if row_value.trim().is_empty() {
        config_value.as_ref().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
    } else {
        Some(row_value.trim().to_string())
    }
}

/// Validates every mapped row, creates contacts and generates a personalized
/// draft per valid recipient. Invalid/duplicate rows are flagged, not fatal.
#[tauri::command]
pub async fn bulk_generate(
    state: State<'_, AppState>,
    batch_id: i64,
    source_path: String,
    mapping: BulkColumnMapping,
    config: BulkGenerateConfig,
) -> AppResult<Vec<BulkRowStatus>> {
    bulk_generate_inner(&state, batch_id, &source_path, &mapping, &config).await
}

pub async fn bulk_generate_inner(
    state: &AppState,
    batch_id: i64,
    source_path: &str,
    mapping: &BulkColumnMapping,
    config: &BulkGenerateConfig,
) -> AppResult<Vec<BulkRowStatus>> {
    let batch = bulk_batches_repo::get(&state.pool, batch_id).await?;
    generate_batch_inner(state, &batch, source_path, mapping, config, &HashSet::new()).await
}

/// Retries generation for rows that do not yet have a draft in the batch —
/// e.g. rows that failed on Gemini rate limits. Rows with existing drafts are
/// left untouched.
#[tauri::command]
pub async fn bulk_retry_failed(
    state: State<'_, AppState>,
    batch_id: i64,
    source_path: String,
    mapping: BulkColumnMapping,
    config: BulkGenerateConfig,
) -> AppResult<Vec<BulkRowStatus>> {
    bulk_retry_failed_inner(&state, batch_id, &source_path, &mapping, &config).await
}

pub async fn bulk_retry_failed_inner(
    state: &AppState,
    batch_id: i64,
    source_path: &str,
    mapping: &BulkColumnMapping,
    config: &BulkGenerateConfig,
) -> AppResult<Vec<BulkRowStatus>> {
    let batch = bulk_batches_repo::get(&state.pool, batch_id).await?;
    let existing = generated_emails_repo::list_by_batch(&state.pool, batch_id).await?;
    let done: HashSet<String> = existing
        .iter()
        .filter_map(|d| d.recipient_email.as_ref().map(|e| e.to_lowercase()))
        .collect();
    generate_batch_inner(state, &batch, source_path, mapping, config, &done).await
}

#[allow(clippy::too_many_arguments)]
async fn generate_batch_inner(
    state: &AppState,
    batch: &BulkBatch,
    source_path: &str,
    mapping: &BulkColumnMapping,
    config: &BulkGenerateConfig,
    skip_emails: &HashSet<String>,
) -> AppResult<Vec<BulkRowStatus>> {
    if batch.status != "draft" {
        return Err(AppError::InvalidInput(format!(
            "batch is {} — only draft batches can be generated",
            batch.status
        )));
    }
    let email_type = EmailType::try_from_str(&batch.email_type).ok_or_else(|| {
        AppError::InvalidInput(format!("unknown email type '{}'", batch.email_type))
    })?;

    let (headers, all_rows, _) = parse_spreadsheet_full(source_path.trim())?;
    if headers.is_empty() {
        return Err(AppError::InvalidInput("spreadsheet has no headers".to_string()));
    }
    if all_rows.is_empty() {
        return Err(AppError::InvalidInput("spreadsheet has no data rows".to_string()));
    }

    let email_column = mapping.email.trim();
    if email_column.is_empty() {
        return Err(AppError::InvalidInput(
            "map the email column before generating".to_string(),
        ));
    }

    let mut seen_emails: HashSet<String> = HashSet::new();
    let mut statuses: Vec<BulkRowStatus> = Vec::new();

    for (index, row) in all_rows.iter().enumerate() {
        let name = mapped(row, &headers, &mapping.name);
        let email_raw = mapped(row, &headers, email_column);
        let company = mapped_or_config(&mapped(row, &headers, &mapping.company), &config.company);
        let role = mapped_or_config(&mapped(row, &headers, &mapping.role), &config.role);
        let job_description =
            mapped_or_config(&mapped(row, &headers, &mapping.job_description), &config.job_description);

        let mut row_status = BulkRowStatus {
            row_index: index,
            name: name.clone(),
            email: email_raw.clone(),
            company: company.clone().unwrap_or_default(),
            role: role.clone().unwrap_or_default(),
            status: "ready".to_string(),
            detail: None,
            generated_email_id: None,
        };

        if email_raw.is_empty() || !email_raw.contains('@') {
            row_status.status = "invalid".to_string();
            row_status.detail = Some("invalid or missing email".to_string());
            statuses.push(row_status);
            continue;
        }
        let email = email_raw.to_lowercase();

        // rows already generated in this batch are silently skipped on retry
        if skip_emails.contains(&email) || seen_emails.contains(&email) {
            if seen_emails.contains(&email) {
                row_status.status = "duplicate".to_string();
                row_status.detail = Some("duplicate email in this file".to_string());
                statuses.push(row_status);
            }
            continue;
        }
        seen_emails.insert(email.clone());

        if contacts_repo::find_id_by_email(&state.pool, &email)
            .await?
            .is_some()
        {
            row_status.status = "duplicate".to_string();
            row_status.detail = Some("contact already exists".to_string());
            statuses.push(row_status);
            continue;
        }

        // auto-create the contact so history/follow-ups link up
        let contact = contacts_repo::create(
            &state.pool,
            &crate::models::contact::ContactInput {
                name: name.clone(),
                email: email.clone(),
                organization: company.clone(),
                role_title: role.clone(),
                linkedin_url: None,
                notes: "added via bulk outreach".to_string(),
            },
        )
        .await;

        let contact_id = match contact {
            Ok(c) => Some(c.id),
            Err(AppError::InvalidInput(detail)) => {
                row_status.status = "duplicate".to_string();
                row_status.detail = Some(detail);
                statuses.push(row_status);
                continue;
            }
            Err(e) => return Err(e),
        };

        let request = crate::models::email::EmailDraftRequest {
            recipient_email: email.clone(),
            recipient_name: if name.is_empty() { None } else { Some(name.clone()) },
            company,
            role,
            job_description,
            additional_context: None,
            email_type,
            application_id: batch.application_id,
            contact_id,
        };

        match emails::generate_email_inner(state, request).await {
            Ok(draft) => {
                generated_emails_repo::set_bulk_batch_link(&state.pool, draft.id, batch.id)
                    .await?;
                row_status.generated_email_id = Some(draft.id);
            }
            Err(e) => {
                row_status.status = "failed".to_string();
                row_status.detail = Some(e.to_string());
            }
        }
        statuses.push(row_status);
    }

    let generated = statuses
        .iter()
        .filter(|s| s.generated_email_id.is_some())
        .count() as i64;
    bulk_batches_repo::set_total(&state.pool, batch.id, generated).await?;

    Ok(statuses)
}

fn parse_spreadsheet_full(
    path: &str,
) -> AppResult<(Vec<String>, Vec<Vec<String>>, usize)> {
    // same parsing as preview, but returns ALL data rows
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "csv" => {
            let mut reader = csv::Reader::from_path(path)
                .map_err(|e| AppError::InvalidInput(format!("could not read CSV: {e}")))?;
            let headers: Vec<String> = reader
                .headers()
                .map_err(|e| AppError::InvalidInput(format!("could not read CSV headers: {e}")))?
                .iter()
                .map(|cell| cell.trim().to_string())
                .collect();
            let mut data = Vec::new();
            for record in reader.records() {
                let record =
                    record.map_err(|e| AppError::InvalidInput(format!("bad CSV row: {e}")))?;
                data.push(record.iter().map(|cell| cell.trim().to_string()).collect());
            }
            let count = data.len();
            Ok((headers, data, count))
        }
        "xlsx" | "xls" => {
            let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(path)
                .map_err(|e| AppError::InvalidInput(format!("could not read XLSX: {e}")))?;
            let sheet_name = workbook
                .sheet_names()
                .first()
                .cloned()
                .ok_or_else(|| AppError::InvalidInput("workbook has no sheets".to_string()))?;
            let range = workbook
                .worksheet_range(&sheet_name)
                .map_err(|e| AppError::InvalidInput(format!("could not read first sheet: {e}")))?;
            let mut rows = range.rows();
            let headers: Vec<String> = rows
                .next()
                .ok_or_else(|| AppError::InvalidInput("sheet is empty".to_string()))?
                .iter()
                .map(|cell| cell.to_string().trim().to_string())
                .collect();
            let data: Vec<Vec<String>> = rows
                .map(|record| {
                    record.iter().map(|cell| cell.to_string().trim().to_string()).collect()
                })
                .collect();
            let count = data.len();
            Ok((headers, data, count))
        }
        other => Err(AppError::InvalidInput(format!(
            "unsupported file type '{other}' — use .csv or .xlsx"
        ))),
    }
}

/// Removes a draft from a batch (the recipient was excluded after review).
#[tauri::command]
pub async fn bulk_batch_remove_draft(
    state: State<'_, AppState>,
    batch_id: i64,
    draft_id: i64,
) -> AppResult<bool> {
    bulk_batch_remove_draft_inner(&state.pool, batch_id, draft_id).await
}

pub async fn bulk_batch_remove_draft_inner(
    pool: &sqlx::SqlitePool,
    batch_id: i64,
    draft_id: i64,
) -> AppResult<bool> {
    // ensure the draft belongs to this batch before deleting
    let draft = generated_emails_repo::get(pool, draft_id).await?;
    if draft.bulk_batch_id != Some(batch_id) {
        return Err(AppError::InvalidInput(
            "draft does not belong to this batch".to_string(),
        ));
    }
    generated_emails_repo::delete(pool, draft_id).await?;
    let remaining = generated_emails_repo::count_by_batch(pool, batch_id).await?;
    bulk_batches_repo::set_total(pool, batch_id, remaining).await?;
    Ok(true)
}

/// Marks a batch sent/failed after the frontend-driven send loop finishes.
#[tauri::command]
pub async fn bulk_batch_finish(
    state: State<'_, AppState>,
    id: i64,
    status: String,
) -> AppResult<BulkBatch> {
    if !matches!(status.as_str(), "sent" | "failed") {
        return Err(AppError::InvalidInput(
            "final batch status must be sent or failed".to_string(),
        ));
    }
    bulk_batches_repo::set_status(&state.pool, id, &status).await?;
    bulk_batches_repo::get(&state.pool, id).await
}
