use serde::{Deserialize, Serialize};

use crate::models::profile::str_enum;

str_enum!(EmailType {
    ColdOutreach => "cold_outreach",
    JobApplication => "job_application",
    ReferralRequest => "referral_request",
    FollowUp => "follow_up",
    InternshipInquiry => "internship_inquiry",
    ApplicationStatus => "application_status",
});

str_enum!(EmailStatus {
    Draft => "draft",
    Edited => "edited",
    Approved => "approved",
    Sent => "sent",
    Discarded => "discarded",
});

str_enum!(ResponseStatus {
    Awaiting => "awaiting",
    Replied => "replied",
    NoReplyNeeded => "no_reply_needed",
});

impl EmailStatus {
    /// Deterministic transition rules owned by application code, never the AI.
    pub fn can_transition_to(self, next: EmailStatus) -> bool {
        if self == next {
            return true;
        }
        matches!(
            (self, next),
            (EmailStatus::Draft, EmailStatus::Edited)
                | (EmailStatus::Draft, EmailStatus::Approved)
                | (EmailStatus::Draft, EmailStatus::Discarded)
                | (EmailStatus::Edited, EmailStatus::Approved)
                | (EmailStatus::Edited, EmailStatus::Draft)
                | (EmailStatus::Edited, EmailStatus::Discarded)
                | (EmailStatus::Approved, EmailStatus::Sent)
                | (EmailStatus::Approved, EmailStatus::Edited)
                | (EmailStatus::Approved, EmailStatus::Discarded)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedEmail {
    pub id: i64,
    pub application_id: Option<i64>,
    pub contact_id: Option<i64>,
    pub email_type: String,
    pub recipient_email: Option<String>,
    pub recipient_name: Option<String>,
    pub subject: Option<String>,
    pub body: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub follow_up_id: Option<i64>,
    pub bulk_batch_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedEmailInput {
    pub application_id: Option<i64>,
    pub contact_id: Option<i64>,
    pub email_type: EmailType,
    pub recipient_email: Option<String>,
    pub recipient_name: Option<String>,
    pub subject: Option<String>,
    pub body: String,
}

/// Everything the generator needs to compose one personalized email.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailDraftRequest {
    pub recipient_email: String,
    pub recipient_name: Option<String>,
    pub company: Option<String>,
    pub role: Option<String>,
    pub job_description: Option<String>,
    pub additional_context: Option<String>,
    pub email_type: EmailType,
    pub application_id: Option<i64>,
    pub contact_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedContact {
    pub name: Option<String>,
    pub organization: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EmailHistory {
    pub id: i64,
    pub direction: String,
    pub application_id: Option<i64>,
    pub contact_id: Option<i64>,
    pub generated_email_id: Option<i64>,
    pub gmail_message_id: Option<String>,
    pub gmail_thread_id: Option<String>,
    pub email_type: Option<String>,
    pub recipient_email: Option<String>,
    pub subject: Option<String>,
    pub body: String,
    pub delivery_method: Option<String>,
    pub status: String,
    pub response_status: Option<String>,
    pub occurred_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryFilter {
    pub contact_id: Option<i64>,
    pub application_id: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingEmailInput {
    pub contact_id: Option<i64>,
    pub application_id: Option<i64>,
    pub sender_email: String,
    pub email_type: Option<EmailType>,
    pub subject: Option<String>,
    pub body: String,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EmailTemplate {
    pub id: i64,
    pub email_type: String,
    pub role: Option<String>,
    pub company_or_industry: Option<String>,
    pub subject_template: Option<String>,
    pub body_template: String,
    pub variables_json: String,
    pub source: String,
    pub success_count: i64,
    pub times_used: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailTemplateInput {
    pub email_type: EmailType,
    pub role: Option<String>,
    pub company_or_industry: Option<String>,
    pub subject_template: Option<String>,
    pub body_template: String,
}
