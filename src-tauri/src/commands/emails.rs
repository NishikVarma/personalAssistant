use tauri::State;

use crate::db::{contacts_repo, generated_emails_repo, profile_snapshot};
use crate::error::{AppError, AppResult};
use crate::llm::{
    email_prompt::{
        build_email_prompt, build_extract_contact_prompt, parse_email_response,
        parse_extracted_contact,
    },
    GeminiProvider, LlmProvider,
};
use crate::models::email::{
    EmailDraftRequest, EmailStatus, ExtractedContact, GeneratedEmail, GeneratedEmailInput,
};
use crate::state::AppState;

#[tauri::command]
pub async fn generated_email_list(
    state: State<'_, AppState>,
    status: Option<EmailStatus>,
) -> AppResult<Vec<GeneratedEmail>> {
    generated_emails_repo::list(&state.pool, status).await
}

#[tauri::command]
pub async fn generated_email_get(
    state: State<'_, AppState>,
    id: i64,
) -> AppResult<GeneratedEmail> {
    generated_emails_repo::get(&state.pool, id).await
}

#[tauri::command]
pub async fn generated_email_create(
    state: State<'_, AppState>,
    input: GeneratedEmailInput,
) -> AppResult<GeneratedEmail> {
    generated_emails_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn generated_email_update(
    state: State<'_, AppState>,
    id: i64,
    subject: Option<String>,
    body: String,
) -> AppResult<GeneratedEmail> {
    generated_emails_repo::update_content(&state.pool, id, subject, body).await
}

#[tauri::command]
pub async fn generated_email_set_status(
    state: State<'_, AppState>,
    id: i64,
    status: EmailStatus,
) -> AppResult<GeneratedEmail> {
    generated_emails_repo::set_status(&state.pool, id, status).await
}

#[tauri::command]
pub async fn generated_email_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    generated_emails_repo::delete(&state.pool, id).await
}

/// Generates one personalized email with Gemini from the verified career profile
/// and the request details. The result is stored as a draft; nothing is sent.
#[tauri::command]
pub async fn ai_generate_email(
    state: State<'_, AppState>,
    request: EmailDraftRequest,
) -> AppResult<GeneratedEmail> {
    let recipient = request.recipient_email.trim();
    if recipient.is_empty() || !recipient.contains('@') {
        return Err(AppError::InvalidInput(
            "a valid recipient email is required".to_string(),
        ));
    }

    // Deterministic contact linking: attach an existing contact by email if the
    // caller did not pick one explicitly.
    let contact_id = match request.contact_id {
        Some(id) => Some(id),
        None => contacts_repo::find_id_by_email(&state.pool, recipient).await?,
    };
    let request = EmailDraftRequest { contact_id, ..request };

    let profile_block = profile_snapshot::collect(&state.pool).await?;
    let provider = super::ai::current_provider(&state).await?;
    let model = provider.model().to_string();

    let prompt = build_email_prompt(&request, &profile_block);
    let raw = provider.complete(prompt).await?;
    let (subject, body) = parse_email_response(&raw)?;

    let input = GeneratedEmailInput {
        application_id: request.application_id,
        contact_id: request.contact_id,
        email_type: request.email_type,
        subject,
        body,
    };
    let mut created = generated_emails_repo::create(&state.pool, &input).await?;

    sqlx::query("UPDATE generated_emails SET provider = ?1, model = ?2 WHERE id = ?3")
        .bind("gemini")
        .bind(&model)
        .bind(created.id)
        .execute(&state.pool)
        .await?;
    created.provider = Some("gemini".to_string());
    created.model = Some(model);
    Ok(created)
}

/// Guesses name/organization from an email address so the user can verify or correct.
#[tauri::command]
pub async fn ai_extract_contact(
    state: State<'_, AppState>,
    email: String,
) -> AppResult<ExtractedContact> {
    let trimmed = email.trim();
    if !trimmed.contains('@') {
        return Err(AppError::InvalidInput("invalid email address".to_string()));
    }
    let provider: GeminiProvider = super::ai::current_provider(&state).await?;
    let prompt = build_extract_contact_prompt(trimmed);
    let raw = provider.complete(prompt).await?;
    let (name, organization) = parse_extracted_contact(&raw)?;
    Ok(ExtractedContact { name, organization })
}
