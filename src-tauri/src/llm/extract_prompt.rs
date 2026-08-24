use crate::error::{AppError, AppResult};
use crate::models::resume::ExtractedProfile;

/// Builds the extraction prompt: strict JSON, facts only, no invention.
pub fn build_extraction_prompt(resume_text: &str) -> String {
    format!(
        "You extract structured data from resumes.\n\n\
         STRICT RULES:\n\
         - Use ONLY facts present in the resume text below. Never invent or infer\n\
           employers, technologies, dates, metrics or achievements.\n\
         - Omit fields you cannot find; use empty strings and empty arrays.\n\
         - Dates: YYYY-MM when only month/year is known, YYYY-MM-DD when a full date exists.\n\
         - employmentType: one of internship, full_time, part_time, contract, freelance.\n\
         - skill category: one of language, framework, tool, database, cloud, soft_skill, other.\n\
         - link kind: one of linkedin, github, portfolio, other.\n\n\
         RESUME TEXT:\n---\n{resume_text}\n---\n\n\
         Respond with ONLY a JSON object — no markdown fences, no commentary — exactly:\n\
         {{\"fullName\": \"\", \"email\": \"\", \"phone\": \"\", \"location\": \"\", \"summary\": \"\",\n\
          \"education\": [{{\"institution\": \"\", \"degree\": \"\", \"fieldOfStudy\": \"\",\n\
            \"startDate\": null, \"endDate\": null, \"grade\": null, \"location\": null, \"details\": \"\"}}],\n\
          \"experience\": [{{\"organization\": \"\", \"title\": \"\", \"employmentType\": \"\",\n\
            \"location\": null, \"startDate\": null, \"endDate\": null,\n\
            \"currentlyWorking\": false, \"description\": \"\"}}],\n\
          \"projects\": [{{\"name\": \"\", \"description\": \"\", \"repoUrl\": null,\n\
            \"liveUrl\": null, \"startedOn\": null, \"endedOn\": null}}],\n\
          \"skills\": [{{\"name\": \"\", \"category\": \"\"}}],\n\
          \"certifications\": [{{\"name\": \"\", \"issuer\": \"\", \"issueDate\": null,\n\
            \"expiryDate\": null, \"credentialUrl\": null}}],\n\
          \"achievements\": [{{\"title\": \"\", \"description\": \"\", \"date\": null}}],\n\
          \"links\": [{{\"label\": \"\", \"url\": \"\", \"kind\": \"\"}}]}}"
    )
}

/// Shared tolerant JSON-object extraction (strips fences and surrounding prose).
pub fn extract_json_object(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    let start = trimmed
        .find('{')
        .ok_or_else(|| AppError::InvalidInput("AI response did not contain JSON".to_string()))?;
    let bytes = trimmed.as_bytes();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut end = None;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if escaped {
            escaped = false;
            continue;
        }
        match b {
            b'\\' if in_string => escaped = true,
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.ok_or_else(|| {
        AppError::InvalidInput("AI response contained incomplete JSON".to_string())
    })?;
    Ok(trimmed[start..=end].to_string())
}

pub fn parse_extracted_profile(raw: &str) -> AppResult<ExtractedProfile> {
    let json_text = extract_json_object(raw)?;
    serde_json::from_str::<ExtractedProfile>(&json_text)
        .map_err(|e| AppError::InvalidInput(format!("could not parse extracted profile: {e}")))
}
