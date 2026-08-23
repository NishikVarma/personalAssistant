use crate::error::{AppError, AppResult};
use crate::models::email::{EmailDraftRequest, EmailType};

fn type_purpose(email_type: EmailType) -> &'static str {
    match email_type {
        EmailType::ColdOutreach => {
            "A concise cold outreach to a recruiter/HR introducing the candidate and asking about relevant openings."
        }
        EmailType::JobApplication => {
            "A job application email for a specific role, highlighting the most relevant verified experience."
        }
        EmailType::ReferralRequest => {
            "A polite referral request asking the recipient to refer the candidate for a role."
        }
        EmailType::FollowUp => {
            "A short follow-up on previous communication. Reference that earlier contact happened without restating everything."
        }
        EmailType::InternshipInquiry => {
            "An internship inquiry asking about internship opportunities and requirements."
        }
        EmailType::ApplicationStatus => {
            "A brief status check on an already-submitted application."
        }
    }
}

/// Builds the generation prompt. The candidate facts block is the ONLY source
/// of truth the model may draw from — fabrication is explicitly forbidden.
pub fn build_email_prompt(request: &EmailDraftRequest, profile_block: &str) -> String {
    let mut prompt = String::new();

    prompt.push_str(
        "You write professional, concise emails for a job applicant.\n\n\
         STRICT RULES:\n\
         - Use ONLY the facts in CANDIDATE VERIFIED PROFILE and REQUEST DETAILS below.\n\
         - NEVER invent work experience, technologies, metrics, achievements, companies,\n\
           or qualifications that are not present there.\n\
         - Omit anything you do not have information for; never fill gaps with guesses.\n\
         - Keep it under 180 words unless the job description clearly requires more.\n\n",
    );

    prompt.push_str(&format!(
        "EMAIL TYPE: {}\nPURPOSE: {}\n\n",
        request.email_type.as_str(),
        type_purpose(request.email_type),
    ));

    prompt.push_str("REQUEST DETAILS:\n");
    prompt.push_str(&format!("- Recipient email: {}\n", request.recipient_email.trim()));
    if let Some(name) = request.recipient_name.as_deref().filter(|v| !v.trim().is_empty()) {
        prompt.push_str(&format!("- Recipient name: {name}\n"));
    } else {
        prompt.push_str("- Recipient name: unknown (use a suitable generic greeting)\n");
    }
    if let Some(company) = request.company.as_deref().filter(|v| !v.trim().is_empty()) {
        prompt.push_str(&format!("- Company: {company}\n"));
    }
    if let Some(role) = request.role.as_deref().filter(|v| !v.trim().is_empty()) {
        prompt.push_str(&format!("- Role: {role}\n"));
    }
    if let Some(jd) = request.job_description.as_deref().filter(|v| !v.trim().is_empty()) {
        prompt.push_str(&format!("- Job description (may be truncated):\n{jd}\n"));
    }
    if let Some(ctx) = request.additional_context.as_deref().filter(|v| !v.trim().is_empty()) {
        prompt.push_str(&format!("- Additional context from the user:\n{ctx}\n"));
    }

    prompt.push_str("\nCANDIDATE VERIFIED PROFILE:\n");
    prompt.push_str(profile_block);
    prompt.push_str("\n\n");

    prompt.push_str(
        "Respond with ONLY a JSON object — no markdown fences, no commentary — exactly:\n\
         {\"subject\": \"...\", \"body\": \"...\"}\n\
         The body is plain text; use \\n for line breaks.",
    );
    prompt
}

/// Parses `{subject, body}` out of a model response. Tolerates markdown fences
/// and surrounding prose by extracting the first balanced JSON object.
pub fn parse_email_response(raw: &str) -> AppResult<(Option<String>, String)> {
    let trimmed = raw.trim();
    let start = trimmed.find('{').ok_or_else(|| {
        AppError::InvalidInput("AI response did not contain JSON".to_string())
    })?;
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
    let end = end.ok_or_else(|| AppError::InvalidInput("AI response contained incomplete JSON".to_string()))?;
    let json_text = &trimmed[start..=end];

    #[derive(serde::Deserialize)]
    struct Parsed {
        subject: Option<String>,
        body: Option<String>,
    }

    let parsed: Parsed = serde_json::from_str(json_text)
        .map_err(|e| AppError::InvalidInput(format!("could not parse AI email payload: {e}")))?;

    let subject = parsed.subject.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let body = parsed
        .body
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .ok_or_else(|| AppError::InvalidInput("AI returned an empty email body".to_string()))?;

    Ok((subject, body))
}

/// Builds the contact-extraction prompt for a raw email address.
pub fn build_extract_contact_prompt(email: &str) -> String {
    format!(
        "Given this email address: {email}\n\n\
         Guess the most likely person's full name and organization based ONLY on the \
         address itself and its domain. Do not use outside knowledge.\n\
         Return ONLY JSON, no fences or commentary, exactly:\n\
         {{\"name\": \"...\" or null, \"organization\": \"...\" or null}}\n\
         Use null whenever you are not reasonably confident."
    )
}

#[derive(serde::Deserialize)]
struct ExtractedContactRaw {
    name: Option<String>,
    organization: Option<String>,
}

pub fn parse_extracted_contact(raw: &str) -> AppResult<(Option<String>, Option<String>)> {
    let start = raw.find('{').ok_or_else(|| {
        AppError::InvalidInput("AI response did not contain JSON".to_string())
    })?;
    let json_text = &raw[start..raw.rfind('}').map_or(raw.len(), |i| i + 1)];
    let parsed: ExtractedContactRaw = serde_json::from_str(json_text)
        .map_err(|e| AppError::InvalidInput(format!("could not parse extracted contact: {e}")))?;

    let clean = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    Ok((clean(parsed.name), clean(parsed.organization)))
}
