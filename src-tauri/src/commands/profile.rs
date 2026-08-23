use tauri::State;

use crate::db::{
    achievements_repo, bullets_repo, certifications_repo, education_repo, experience_repo,
    links_repo, profile_repo, projects_repo, skills_repo,
};
use crate::error::AppResult;
use crate::models::profile::{
    Achievement, AchievementInput, Bullet, BulletInput, Certification, CertificationInput,
    Education, EducationInput, Experience, ExperienceInput, Link, LinkInput, ProfileEntityType,
    Project, ProjectInput, Skill, SkillInput, UserProfile, UserProfileInput,
};
use crate::state::AppState;

// ---------- user profile ----------

#[tauri::command]
pub async fn profile_get(state: State<'_, AppState>) -> AppResult<UserProfile> {
    profile_repo::get(&state.pool).await
}

#[tauri::command]
pub async fn profile_update(
    state: State<'_, AppState>,
    input: UserProfileInput,
) -> AppResult<UserProfile> {
    profile_repo::update(&state.pool, &input).await
}

#[tauri::command]
pub async fn profile_set_verified(
    state: State<'_, AppState>,
    verified: bool,
) -> AppResult<()> {
    profile_repo::set_verified(&state.pool, verified).await
}

// ---------- education ----------

#[tauri::command]
pub async fn education_list(state: State<'_, AppState>) -> AppResult<Vec<Education>> {
    education_repo::list(&state.pool).await
}

#[tauri::command]
pub async fn education_create(
    state: State<'_, AppState>,
    input: EducationInput,
) -> AppResult<Education> {
    education_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn education_update(
    state: State<'_, AppState>,
    id: i64,
    input: EducationInput,
) -> AppResult<Education> {
    education_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn education_set_verified(
    state: State<'_, AppState>,
    id: i64,
    verified: bool,
) -> AppResult<()> {
    education_repo::set_verified(&state.pool, id, verified).await
}

#[tauri::command]
pub async fn education_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    education_repo::delete(&state.pool, id).await
}

// ---------- experience ----------

#[tauri::command]
pub async fn experience_list(state: State<'_, AppState>) -> AppResult<Vec<Experience>> {
    experience_repo::list(&state.pool).await
}

#[tauri::command]
pub async fn experience_create(
    state: State<'_, AppState>,
    input: ExperienceInput,
) -> AppResult<Experience> {
    experience_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn experience_update(
    state: State<'_, AppState>,
    id: i64,
    input: ExperienceInput,
) -> AppResult<Experience> {
    experience_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn experience_set_verified(
    state: State<'_, AppState>,
    id: i64,
    verified: bool,
) -> AppResult<()> {
    experience_repo::set_verified(&state.pool, id, verified).await
}

#[tauri::command]
pub async fn experience_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    experience_repo::delete(&state.pool, id).await
}

// ---------- projects ----------

#[tauri::command]
pub async fn project_list(state: State<'_, AppState>) -> AppResult<Vec<Project>> {
    projects_repo::list(&state.pool).await
}

#[tauri::command]
pub async fn project_create(
    state: State<'_, AppState>,
    input: ProjectInput,
) -> AppResult<Project> {
    projects_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn project_update(
    state: State<'_, AppState>,
    id: i64,
    input: ProjectInput,
) -> AppResult<Project> {
    projects_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn project_set_verified(
    state: State<'_, AppState>,
    id: i64,
    verified: bool,
) -> AppResult<()> {
    projects_repo::set_verified(&state.pool, id, verified).await
}

#[tauri::command]
pub async fn project_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    projects_repo::delete(&state.pool, id).await
}

// ---------- skills ----------

#[tauri::command]
pub async fn skill_list(state: State<'_, AppState>) -> AppResult<Vec<Skill>> {
    skills_repo::list(&state.pool).await
}

#[tauri::command]
pub async fn skill_create(state: State<'_, AppState>, input: SkillInput) -> AppResult<Skill> {
    skills_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn skill_update(
    state: State<'_, AppState>,
    id: i64,
    input: SkillInput,
) -> AppResult<Skill> {
    skills_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn skill_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    skills_repo::delete(&state.pool, id).await
}

#[tauri::command]
pub async fn skill_list_for_entity(
    state: State<'_, AppState>,
    entity_type: ProfileEntityType,
    entity_id: i64,
) -> AppResult<Vec<Skill>> {
    skills_repo::list_for_entity(&state.pool, entity_type, entity_id).await
}

#[tauri::command]
pub async fn skill_replace_for_entity(
    state: State<'_, AppState>,
    entity_type: ProfileEntityType,
    entity_id: i64,
    skill_ids: Vec<i64>,
) -> AppResult<()> {
    skills_repo::replace_entity_skills(&state.pool, entity_type, entity_id, &skill_ids).await
}

// ---------- bullets ----------

#[tauri::command]
pub async fn bullet_list_for_entity(
    state: State<'_, AppState>,
    entity_type: ProfileEntityType,
    entity_id: i64,
) -> AppResult<Vec<Bullet>> {
    bullets_repo::list_for_entity(&state.pool, entity_type, entity_id).await
}

#[tauri::command]
pub async fn bullet_create(
    state: State<'_, AppState>,
    entity_type: ProfileEntityType,
    entity_id: i64,
    input: BulletInput,
) -> AppResult<Bullet> {
    bullets_repo::create(&state.pool, entity_type, entity_id, &input).await
}

#[tauri::command]
pub async fn bullet_update(
    state: State<'_, AppState>,
    id: i64,
    input: BulletInput,
) -> AppResult<Bullet> {
    bullets_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn bullet_set_verified(
    state: State<'_, AppState>,
    id: i64,
    verified: bool,
) -> AppResult<()> {
    bullets_repo::set_verified(&state.pool, id, verified).await
}

#[tauri::command]
pub async fn bullet_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    bullets_repo::delete(&state.pool, id).await
}

// ---------- certifications ----------

#[tauri::command]
pub async fn certification_list(state: State<'_, AppState>) -> AppResult<Vec<Certification>> {
    certifications_repo::list(&state.pool).await
}

#[tauri::command]
pub async fn certification_create(
    state: State<'_, AppState>,
    input: CertificationInput,
) -> AppResult<Certification> {
    certifications_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn certification_update(
    state: State<'_, AppState>,
    id: i64,
    input: CertificationInput,
) -> AppResult<Certification> {
    certifications_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn certification_set_verified(
    state: State<'_, AppState>,
    id: i64,
    verified: bool,
) -> AppResult<()> {
    certifications_repo::set_verified(&state.pool, id, verified).await
}

#[tauri::command]
pub async fn certification_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    certifications_repo::delete(&state.pool, id).await
}

// ---------- achievements ----------

#[tauri::command]
pub async fn achievement_list(state: State<'_, AppState>) -> AppResult<Vec<Achievement>> {
    achievements_repo::list(&state.pool).await
}

#[tauri::command]
pub async fn achievement_create(
    state: State<'_, AppState>,
    input: AchievementInput,
) -> AppResult<Achievement> {
    achievements_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn achievement_update(
    state: State<'_, AppState>,
    id: i64,
    input: AchievementInput,
) -> AppResult<Achievement> {
    achievements_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn achievement_set_verified(
    state: State<'_, AppState>,
    id: i64,
    verified: bool,
) -> AppResult<()> {
    achievements_repo::set_verified(&state.pool, id, verified).await
}

#[tauri::command]
pub async fn achievement_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    achievements_repo::delete(&state.pool, id).await
}

// ---------- links ----------

#[tauri::command]
pub async fn link_list(state: State<'_, AppState>) -> AppResult<Vec<Link>> {
    links_repo::list(&state.pool).await
}

#[tauri::command]
pub async fn link_create(state: State<'_, AppState>, input: LinkInput) -> AppResult<Link> {
    links_repo::create(&state.pool, &input).await
}

#[tauri::command]
pub async fn link_update(
    state: State<'_, AppState>,
    id: i64,
    input: LinkInput,
) -> AppResult<Link> {
    links_repo::update(&state.pool, id, &input).await
}

#[tauri::command]
pub async fn link_delete(state: State<'_, AppState>, id: i64) -> AppResult<bool> {
    links_repo::delete(&state.pool, id).await
}
