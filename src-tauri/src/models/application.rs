use serde::{Deserialize, Serialize};

use crate::models::profile::str_enum;

str_enum!(ApplicationStatus {
    Saved => "saved",
    Preparing => "preparing",
    Applied => "applied",
    Contacted => "contacted",
    FollowUpDue => "follow_up_due",
    ResponseReceived => "response_received",
    Oa => "oa",
    Interview => "interview",
    Offer => "offer",
    Rejected => "rejected",
    Withdrawn => "withdrawn",
});

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    pub id: i64,
    pub company: String,
    pub role: String,
    pub job_description: String,
    pub job_url: Option<String>,
    pub source: Option<String>,
    pub status: String,
    pub date_discovered: Option<String>,
    pub date_applied: Option<String>,
    pub follow_up_date: Option<String>,
    pub interview_status: Option<String>,
    pub priority: i64,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInput {
    pub company: String,
    pub role: String,
    pub job_description: String,
    pub job_url: Option<String>,
    pub source: Option<String>,
    pub date_discovered: Option<String>,
    pub date_applied: Option<String>,
    pub follow_up_date: Option<String>,
    pub interview_status: Option<String>,
    pub priority: i64,
    pub notes: String,
}
