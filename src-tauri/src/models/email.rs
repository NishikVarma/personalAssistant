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
    pub subject: Option<String>,
    pub body: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedEmailInput {
    pub application_id: Option<i64>,
    pub contact_id: Option<i64>,
    pub email_type: EmailType,
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
