use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileEntityType {
    Project,
    Experience,
}

impl ProfileEntityType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Experience => "experience",
        }
    }
}

macro_rules! str_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $value),+
                }
            }

            pub fn all() -> &'static [$name] {
                &[$($name::$variant),+]
            }
        }
    };
}

str_enum!(EmploymentType {
    Internship => "internship",
    FullTime => "full_time",
    PartTime => "part_time",
    Contract => "contract",
    Freelance => "freelance",
});

str_enum!(ProjectStatus {
    Ongoing => "ongoing",
    Completed => "completed",
    Archived => "archived",
});

str_enum!(SkillCategory {
    Language => "language",
    Framework => "framework",
    Tool => "tool",
    Database => "database",
    Cloud => "cloud",
    SoftSkill => "soft_skill",
    Other => "other",
});

str_enum!(LinkKind {
    LinkedIn => "linkedin",
    GitHub => "github",
    Portfolio => "portfolio",
    Other => "other",
});

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub id: i64,
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub summary: String,
    pub verified: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileInput {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Education {
    pub id: i64,
    pub institution: String,
    pub degree: String,
    pub field_of_study: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub grade: Option<String>,
    pub location: Option<String>,
    pub details: String,
    pub verified: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EducationInput {
    pub institution: String,
    pub degree: String,
    pub field_of_study: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub grade: Option<String>,
    pub location: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Experience {
    pub id: i64,
    pub organization: String,
    pub title: String,
    pub employment_type: String,
    pub location: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub currently_working: bool,
    pub description: String,
    pub verified: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperienceInput {
    pub organization: String,
    pub title: String,
    pub employment_type: EmploymentType,
    pub location: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub currently_working: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub repo_url: Option<String>,
    pub live_url: Option<String>,
    pub status: String,
    pub started_on: Option<String>,
    pub ended_on: Option<String>,
    pub verified: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub name: String,
    pub description: String,
    pub repo_url: Option<String>,
    pub live_url: Option<String>,
    pub status: ProjectStatus,
    pub started_on: Option<String>,
    pub ended_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInput {
    pub name: String,
    pub category: SkillCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Bullet {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub content: String,
    pub verified: bool,
    pub display_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulletInput {
    pub content: String,
    pub display_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Certification {
    pub id: i64,
    pub name: String,
    pub issuer: String,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
    pub credential_url: Option<String>,
    pub verified: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificationInput {
    pub name: String,
    pub issuer: String,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
    pub credential_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Achievement {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub date: Option<String>,
    pub verified: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementInput {
    pub title: String,
    pub description: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: i64,
    pub label: String,
    pub url: String,
    pub kind: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkInput {
    pub label: String,
    pub url: String,
    pub kind: LinkKind,
}
