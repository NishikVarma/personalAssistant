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

/// Structured profile extracted from a resume. Mirrors the profile input
/// shapes so import can reuse the existing repos directly. Enum-ish fields
/// arrive as free strings and are normalized on import.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractedProfile {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub summary: String,
    pub education: Vec<ExtractedEducation>,
    pub experience: Vec<ExtractedExperience>,
    pub projects: Vec<ExtractedProject>,
    pub skills: Vec<ExtractedSkill>,
    pub certifications: Vec<ExtractedCertification>,
    pub achievements: Vec<ExtractedAchievement>,
    pub links: Vec<ExtractedLink>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractedEducation {
    pub institution: String,
    pub degree: String,
    pub field_of_study: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub grade: Option<String>,
    pub location: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractedExperience {
    pub organization: String,
    pub title: String,
    pub employment_type: Option<String>,
    pub location: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub currently_working: bool,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractedProject {
    pub name: String,
    pub description: String,
    pub repo_url: Option<String>,
    pub live_url: Option<String>,
    pub started_on: Option<String>,
    pub ended_on: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractedSkill {
    pub name: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractedCertification {
    pub name: String,
    pub issuer: String,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
    pub credential_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractedAchievement {
    pub title: String,
    pub description: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExtractedLink {
    pub label: String,
    pub url: String,
    pub kind: Option<String>,
}
