use serde::{Deserialize, Serialize};

use crate::models::profile::str_enum;

str_enum!(BulkBatchStatus {
    Draft => "draft",
    Sending => "sending",
    Sent => "sent",
    Failed => "failed",
});

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BulkBatch {
    pub id: i64,
    pub email_type: String,
    pub application_id: Option<i64>,
    pub status: String,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// One row of an imported spreadsheet, after column mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkRow {
    pub name: String,
    pub email: String,
    pub company: String,
    pub role: String,
    pub job_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportPreview {
    pub headers: Vec<String>,
    pub sample_rows: Vec<Vec<String>>,
    pub total_data_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkColumnMapping {
    pub name: String,
    pub email: String,
    pub company: String,
    pub role: String,
    pub job_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkRowStatus {
    pub row_index: usize,
    pub name: String,
    pub email: String,
    pub company: String,
    pub role: String,
    pub status: String,
    pub detail: Option<String>,
    pub generated_email_id: Option<i64>,
}
