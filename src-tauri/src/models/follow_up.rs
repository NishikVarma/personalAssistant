use serde::{Deserialize, Serialize};

use crate::models::profile::str_enum;

str_enum!(FollowUpStatus {
    Pending => "pending",
    Due => "due",
    Sent => "sent",
    Cancelled => "cancelled",
    Suppressed => "suppressed",
});

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FollowUp {
    pub id: i64,
    pub application_id: i64,
    pub contact_id: Option<i64>,
    pub originating_email_id: Option<i64>,
    pub sequence: i64,
    pub scheduled_for: String,
    pub status: String,
    pub suppressed_reason: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FollowUpConfig {
    pub days: i64,
    pub second_days: Option<i64>,
    pub auto_schedule: bool,
}
