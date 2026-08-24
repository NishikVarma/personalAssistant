use tauri::State;

use crate::db::{
    contacts_repo, email_history_repo, email_templates_repo, generated_emails_repo,
    profile_snapshot,
};
use crate::error::{AppError, AppResult};
use crate::gmail;
use crate::gmail::mime::{self, Attachment};
use crate::llm::{
    email_prompt::{
        build_email_prompt, build_extract_contact_prompt, parse_email_response,
        parse_extracted_contact,
    },
    GeminiProvider, LlmProvider,
};
use crate::models::email::{
    EmailDraftRequest, EmailHistory, EmailStatus, EmailTemplate, EmailTemplateInput,
    EmailType, ExtractedContact, GeneratedEmail, GeneratedEmailInput, HistoryFilter,
    IncomingEmailInput, ResponseStatus,
};
use crate::state::AppState;

use super::gmail as gmail_commands;

const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024;

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
    generate_email_inner(&state, request).await
}

pub async fn generate_email_inner(
    state: &AppState,
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
    let provider = super::ai::current_provider(state).await?;
    let model = provider.model().to_string();

    // template memory: reuse a stored email of this type instead of writing
    // from scratch when one exists (most relevant first)
    let templates =
        email_templates_repo::list(&state.pool, Some(request.email_type)).await?;
    let chosen = email_templates_repo::choose_template(
        &templates,
        request.role.as_deref(),
        request.company.as_deref(),
    );
    if let Some(template) = &chosen {
        email_templates_repo::mark_used(&state.pool, template.id).await?;
    }
    let template_hint = chosen.as_ref().map(|t| {
        format!(
            "Subject: {}\n\n{}",
            t.subject_template.as_deref().unwrap_or("(none)"),
            t.body_template
        )
    });

    let prompt = build_email_prompt(&request, &profile_block, template_hint.as_deref());
    let raw = provider.complete(prompt).await?;
    let (subject, body) = parse_email_response(&raw)?;

    let input = GeneratedEmailInput {
        application_id: request.application_id,
        contact_id: request.contact_id,
        email_type: request.email_type,
        recipient_email: Some(request.recipient_email.trim().to_string()),
        recipient_name: request.recipient_name.clone(),
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

fn guess_mime_type(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("pdf") => "application/pdf",
        Some("doc") => "application/msword",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("txt" | "md" | "tex") => "text/plain",
        _ => "application/octet-stream",
    }
}

/// Sends an approved draft via Gmail. Enforces the duplicate-outreach guard
/// (override with `force`) and records history + contact timestamp on success.
#[tauri::command]
pub async fn email_send(
    state: State<'_, AppState>,
    id: i64,
    attachment_path: Option<String>,
    force: bool,
) -> AppResult<GeneratedEmail> {
    let email = generated_emails_repo::get(&state.pool, id).await?;
    if email.status != EmailStatus::Approved.as_str() {
        return Err(AppError::InvalidInput(
            "only approved drafts can be sent".to_string(),
        ));
    }

    let recipient = match &email.recipient_email {
        Some(address) if !address.trim().is_empty() => address.trim().to_string(),
        _ => match email.contact_id {
            Some(contact_id) => {
                let contact = contacts_repo::get(&state.pool, contact_id).await?;
                contact.email
            }
            None => {
                return Err(AppError::InvalidInput(
                    "draft has no recipient — regenerate it with a recipient email".to_string(),
                ))
            }
        },
    };
    if !recipient.contains('@') {
        return Err(AppError::InvalidInput(
            "recipient address is not a valid email".to_string(),
        ));
    }

    if !force
        && email_history_repo::has_recent_outgoing(
            &state.pool,
            &recipient,
            email_history_repo::RECENT_OUTREACH_DAYS,
        )
        .await?
    {
        return Err(AppError::RecentOutreach(format!(
            "an email was already sent to {recipient} in the last {} days",
            email_history_repo::RECENT_OUTREACH_DAYS
        )));
    }

    let attachment = match attachment_path.as_deref().filter(|p| !p.trim().is_empty()) {
        Some(path) => {
            let path = std::path::Path::new(path.trim());
            let bytes = tokio::fs::read(path).await.map_err(|e| {
                AppError::InvalidInput(format!("could not read attachment: {e}"))
            })?;
            if bytes.len() > MAX_ATTACHMENT_BYTES {
                return Err(AppError::InvalidInput(
                    "attachment is larger than 20 MB".to_string(),
                ));
            }
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "attachment".to_string());
            use base64::Engine as _;
            Some(Attachment {
                filename,
                mime_type: guess_mime_type(path).to_string(),
                content_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
            })
        }
        None => None,
    };

    let (access_token, account_email) = gmail_commands::fresh_access_token(&state).await?;
    let mime_message = mime::build_mime(
        &account_email,
        &recipient,
        email.subject.as_deref().unwrap_or(""),
        &email.body,
        attachment.as_ref(),
    );
    let (message_id, thread_id) =
        gmail::send_message(&access_token, &mime::to_gmail_raw(&mime_message)).await?;

    let history_id = email_history_repo::record_sent(
        &state.pool,
        &email_history_repo::SentRecord {
            generated_email_id: email.id,
            contact_id: email.contact_id,
            recipient_email: recipient.clone(),
            email_type: email.email_type.clone(),
            subject: email.subject.clone(),
            body: email.body.clone(),
            gmail_message_id: message_id,
            gmail_thread_id: thread_id,
        },
    )
    .await?;

    // deterministic follow-up scheduling (marking fulfilled ones sent, chaining
    // the next round); failures here must not fail the send
    if let Err(e) =
        super::follow_up::schedule_after_send(state.inner(), email.application_id, email.contact_id, email.follow_up_id, history_id)
            .await
    {
        eprintln!("follow-up scheduling skipped: {e}");
    }

    generated_emails_repo::get(&state.pool, email.id).await
}

// ---------- email history ----------

#[tauri::command]
pub async fn email_history_list(
    state: State<'_, AppState>,
    filter: HistoryFilter,
) -> AppResult<Vec<EmailHistory>> {
    email_history_repo::list(&state.pool, &filter).await
}

#[tauri::command]
pub async fn email_history_set_response(
    state: State<'_, AppState>,
    id: i64,
    status: Option<ResponseStatus>,
) -> AppResult<EmailHistory> {
    email_history_repo::set_response_status(&state.pool, id, status).await
}

#[tauri::command]
pub async fn email_history_record_incoming(
    state: State<'_, AppState>,
    input: IncomingEmailInput,
) -> AppResult<EmailHistory> {
    email_history_repo::record_incoming(&state.pool, &input).await
}

// ---------- email templates ----------

#[tauri::command]
pub async fn email_template_list(
    state: State<'_, AppState>,
    email_type: Option<EmailType>,
) -> AppResult<Vec<EmailTemplate>> {
    email_templates_repo::list(&state.pool, email_type).await
}

#[tauri::command]
pub async fn email_template_create(
    state: State<'_, AppState>,
    input: EmailTemplateInput,
) -> AppResult<EmailTemplate> {
    email_templates_repo::create(&state.pool, &input, "user").await
}

#[tauri::command]
pub async fn email_template_update(
    state: State<'_, AppState>,
    id: i64,
    input: EmailTemplateInput,
) -> AppResult<EmailTemplate> {
    email_templates_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn email_template_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    email_templates_repo::delete(&state.pool, id).await
}

/// Copies an existing draft (its subject + body) into a reusable template.
#[tauri::command]
pub async fn email_template_save_from_email(
    state: State<'_, AppState>,
    id: i64,
) -> AppResult<EmailTemplate> {
    let email = generated_emails_repo::get(&state.pool, id).await?;
    if email.status == EmailStatus::Discarded.as_str() {
        return Err(AppError::InvalidInput(
            "discarded drafts cannot become templates".to_string(),
        ));
    }
    let email_type = EmailType::try_from_str(&email.email_type)
        .ok_or_else(|| AppError::InvalidInput(format!("unknown email type '{}'", email.email_type)))?;

    // role/company come from the linked application when there is one
    let (role, company) = match email.application_id {
        Some(app_id) => {
            let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
                "SELECT role, company FROM applications WHERE id = ?1",
            )
            .bind(app_id)
            .fetch_optional(&state.pool)
            .await?;
            row.unwrap_or((None, None))
        }
        None => (None, None),
    };

    let input = EmailTemplateInput {
        email_type,
        role,
        company_or_industry: company,
        subject_template: email.subject.clone(),
        body_template: email.body.clone(),
    };
    email_templates_repo::create(&state.pool, &input, "generated").await
}
