use serde::{Deserialize, Serialize};

use crate::models::profile::str_enum;

str_enum!(ResumeFileKind {
    PdfMaster => "pdf_master",
    TexTemplate => "tex_template",
});

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ResumeFile {
    pub id: i64,
    pub kind: String,
    pub original_filename: String,
    pub stored_path: String,
    pub sha256: String,
    pub file_size: i64,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatexStatus {
    pub available: bool,
    pub engine: Option<String>,
}
