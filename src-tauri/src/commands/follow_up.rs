use tauri::State;

use crate::db::{
    applications_repo, contacts_repo, email_history_repo, follow_ups_repo,
    settings_repo,
};
use crate::error::{AppError, AppResult};
use crate::models::email::{EmailDraftRequest, EmailType};
use crate::models::follow_up::{FollowUp, FollowUpConfig, FollowUpStatus};
use crate::state::AppState;

use super::emails;

const DAYS_SETTING: &str = "follow_up.days";
const SECOND_DAYS_SETTING: &str = "follow_up.second_days";
const AUTO_SCHEDULE_SETTING: &str = "follow_up.auto_schedule";

pub(crate) async fn config(pool: &sqlx::SqlitePool) -> AppResult<FollowUpConfig> {
    let days = settings_repo::get(pool, DAYS_SETTING)
        .await?
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|d| *d >= 1)
        .unwrap_or(7);
    let second_days = settings_repo::get(pool, SECOND_DAYS_SETTING)
        .await?
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|d| *d >= 1);
    let auto_schedule = settings_repo::get(pool, AUTO_SCHEDULE_SETTING)
        .await?
        .map(|v| v != "false")
        .unwrap_or(true);
    Ok(FollowUpConfig { days, second_days, auto_schedule })
}

#[tauri::command]
pub async fn follow_up_config_get(state: State<'_, AppState>) -> AppResult<FollowUpConfig> {
    config(&state.pool).await
}

#[tauri::command]
pub async fn follow_up_config_set(
    state: State<'_, AppState>,
    days: i64,
    second_days: Option<i64>,
    auto_schedule: bool,
) -> AppResult<FollowUpConfig> {
    if days < 1 {
        return Err(AppError::InvalidInput(
            "follow-up interval must be at least 1 day".to_string(),
        ));
    }
    if let Some(second) = second_days {
        if second < 1 {
            return Err(AppError::InvalidInput(
                "second follow-up interval must be at least 1 day (or empty to disable)"
                    .to_string(),
            ));
        }
    }
    settings_repo::set(&state.pool, DAYS_SETTING, &days.to_string()).await?;
    match second_days {
        Some(second) => {
            settings_repo::set(&state.pool, SECOND_DAYS_SETTING, &second.to_string()).await?;
        }
        None => {
            settings_repo::delete(&state.pool, SECOND_DAYS_SETTING).await?;
        }
    }
    settings_repo::set(&state.pool, AUTO_SCHEDULE_SETTING, if auto_schedule { "true" } else { "false" })
        .await?;
    config(&state.pool).await
}

#[tauri::command]
pub async fn follow_up_list(
    state: State<'_, AppState>,
    status: Option<FollowUpStatus>,
) -> AppResult<Vec<FollowUp>> {
    follow_ups_repo::list(&state.pool, status).await
}

/// Applies deterministic suppressions, flips arrived follow-ups to `due`, and
/// returns them.
#[tauri::command]
pub async fn follow_up_due(state: State<'_, AppState>) -> AppResult<Vec<FollowUp>> {
    follow_ups_repo::apply_suppressions(&state.pool).await?;
    follow_ups_repo::list_due(&state.pool).await
}

#[tauri::command]
pub async fn follow_up_due_count(state: State<'_, AppState>) -> AppResult<i64> {
    follow_ups_repo::apply_suppressions(&state.pool).await?;
    follow_ups_repo::due_count(&state.pool).await
}

#[tauri::command]
pub async fn follow_up_sweep(state: State<'_, AppState>) -> AppResult<u64> {
    follow_ups_repo::apply_suppressions(&state.pool).await
}

#[tauri::command]
pub async fn follow_up_reschedule(
    state: State<'_, AppState>,
    id: i64,
    scheduled_for: String,
) -> AppResult<FollowUp> {
    if chrono::DateTime::parse_from_rfc3339(scheduled_for.trim()).is_err() {
        return Err(AppError::InvalidInput(
            "scheduled_for must be an RFC 3339 timestamp".to_string(),
        ));
    }
    follow_ups_repo::reschedule(&state.pool, id, scheduled_for).await
}

#[tauri::command]
pub async fn follow_up_cancel(state: State<'_, AppState>, id: i64) -> AppResult<FollowUp> {
    follow_ups_repo::cancel(&state.pool, id).await
}

/// Deterministic post-send scheduling: marks a fulfilled follow-up as sent and
/// auto-schedules the next one per config. Never fails the send itself.
pub(crate) async fn schedule_after_send(
    state: &AppState,
    application_id: Option<i64>,
    contact_id: Option<i64>,
    follow_up_id: Option<i64>,
    history_id: i64,
) -> AppResult<()> {
    let config = config(&state.pool).await?;
    if !config.auto_schedule {
        return Ok(());
    }
    let application_id = match application_id {
        Some(id) => id,
        None => return Ok(()), // follow-ups are anchored to applications
    };

    // closed applications never get follow-ups
    let application = applications_repo::get(&state.pool, application_id).await?;
    if matches!(application.status.as_str(), "rejected" | "withdrawn") {
        return Ok(());
    }

    let (sequence, days) = match follow_up_id {
        Some(follow_up_id) => {
            follow_ups_repo::mark_sent(&state.pool, follow_up_id).await?;
            let current = follow_ups_repo::get(&state.pool, follow_up_id).await?;
            match config.second_days {
                Some(days) => (current.sequence + 1, days),
                None => return Ok(()),
            }
        }
        None => (1, config.days),
    };

    let scheduled_for =
        (chrono::Utc::now() + chrono::Duration::days(days)).to_rfc3339();
    follow_ups_repo::create(
        &state.pool,
        application_id,
        contact_id,
        Some(history_id),
        sequence,
        scheduled_for,
    )
    .await?;
    Ok(())
}

fn build_follow_up_context(sequence: i64, sent_date: &str, subject: &str) -> String {
    format!(
        "This is follow-up number {sequence} to an email sent on {} with the subject \
         \"{subject}\". No reply has been received yet. Write a short, polite follow-up \
         that references the original message without repeating its full content.",
        &sent_date[..10.min(sent_date.len())]
    )
}

/// Generates a follow-up draft for a scheduled follow-up and links it back.
#[tauri::command]
pub async fn follow_up_draft(
    state: State<'_, AppState>,
    id: i64,
) -> AppResult<crate::models::email::GeneratedEmail> {
    let follow_up = follow_ups_repo::get(&state.pool, id).await?;
    match FollowUpStatus::try_from_str(&follow_up.status) {
        Some(FollowUpStatus::Pending) | Some(FollowUpStatus::Due) => {}
        other => {
            return Err(AppError::InvalidInput(format!(
                "cannot draft a {} follow-up",
                other.map(|s| s.as_str().to_string()).unwrap_or(follow_up.status.clone())
            )))
        }
    }

    let application = applications_repo::get(&state.pool, follow_up.application_id).await?;
    let contact = match follow_up.contact_id {
        Some(contact_id) => Some(contacts_repo::get(&state.pool, contact_id).await?),
        None => None,
    };
    let contact = contact.ok_or_else(|| {
        AppError::InvalidInput(
            "follow-up has no linked contact — add one first".to_string(),
        )
    })?;

    let originating = match follow_up.originating_email_id {
        Some(email_id) => Some(email_history_repo::get(&state.pool, email_id).await?),
        None => None,
    };
    let (sent_date, original_subject) = match &originating {
        Some(history) => (history.occurred_at.clone(), history.subject.clone()),
        None => (follow_up.created_at.clone(), None),
    };
    let subject = original_subject.unwrap_or_else(|| format!("{} application", application.role));

    let request = EmailDraftRequest {
        recipient_email: contact.email.clone(),
        recipient_name: if contact.name.trim().is_empty() { None } else { Some(contact.name.clone()) },
        company: Some(application.company.clone()),
        role: Some(application.role.clone()),
        job_description: if application.job_description.trim().is_empty() {
            None
        } else {
            Some(application.job_description.clone())
        },
        additional_context: Some(build_follow_up_context(
            follow_up.sequence,
            &sent_date,
            &subject,
        )),
        email_type: EmailType::FollowUp,
        application_id: Some(follow_up.application_id),
        contact_id: follow_up.contact_id,
    };

    let draft = emails::generate_email_inner(&state, request).await?;
    crate::db::generated_emails_repo::set_follow_up_link(&state.pool, draft.id, id).await?;
    crate::db::generated_emails_repo::get(&state.pool, draft.id).await
}
