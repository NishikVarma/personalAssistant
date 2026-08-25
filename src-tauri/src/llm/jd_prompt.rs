use crate::error::{AppError, AppResult};
use crate::models::resume::JdAnalysis;

/// Builds the JD analysis prompt. Matched skills must exist in the verified
/// profile; missing skills are truthful gaps, never invented.
pub fn build_match_prompt(jd_text: &str, profile_block: &str) -> String {
    format!(
        "You analyze job descriptions against a candidate's verified profile.\n\n\
         STRICT RULES:\n\
         - matchedSkills may ONLY contain skills present in the VERIFIED PROFILE below.\n\
         - missingSkills are JD requirements absent from the profile — report them honestly.\n\
         - recommendedCategory: one of backend, ai_ml, full_stack, general_swe, other.\n\n\
         JOB DESCRIPTION:\n---\n{jd_text}\n---\n\n\
         VERIFIED PROFILE:\n---\n{profile_block}\n---\n\n\
         Respond with ONLY a JSON object — no fences, no commentary — exactly:\n\
         {{\"role\": \"\", \"seniority\": \"\", \"requiredSkills\": [\"\"],\n\
          \"preferredSkills\": [\"\"], \"matchedSkills\": [\"\"], \"missingSkills\": [\"\"],\n\
          \"recommendedCategory\": \"backend|ai_ml|full_stack|general_swe|other\"}}"
    )
}

pub fn parse_jd_analysis(raw: &str) -> AppResult<JdAnalysis> {
    let json_text = super::extract_prompt::extract_json_object(raw)?;
    serde_json::from_str::<JdAnalysis>(&json_text)
        .map_err(|e| AppError::InvalidInput(format!("could not parse JD analysis: {e}")))
}

/// Builds the tailored-resume generation prompt: the user's template structure
/// is preserved; ONLY verified profile facts are projected; the JD steers
/// emphasis. Output is complete LaTeX.
pub fn build_generation_prompt(
    template_tex: &str,
    profile_block: &str,
    jd_text: &str,
    category: Option<&str>,
) -> String {
    let category = category.unwrap_or("general_swe");
    format!(
        "You tailor a LaTeX resume to a job description.\n\n\
         STRICT RULES:\n\
         - Project ONLY the facts in the VERIFIED PROFILE below. NEVER invent employers,\n\
           technologies, metrics, dates or achievements.\n\
         - Keep the template's documentclass, packages and overall formatting commands intact.\n\
         - Emphasize the experience and projects most relevant to the job description.\n\
         - Drop sections the template marks as optional when they add nothing for this JD.\n\
         - Output must compile with pdflatex.\n\
         - Target category: {category}.\n\n\
         LATEX TEMPLATE (preserve its structure):\n---\n{template_tex}\n---\n\n\
         JOB DESCRIPTION:\n---\n{jd_text}\n---\n\n\
         VERIFIED PROFILE:\n---\n{profile_block}\n---\n\n\
         Respond with ONLY the complete LaTeX source — no markdown fences, no commentary."
    )
}

/// Strips markdown fences / prose from a LaTeX model response.
pub fn strip_latex_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(start) = trimmed.find("\\documentclass") {
        let end = trimmed
            .rfind("\\end{document}")
            .map(|i| i + "\\end{document}".len())
            .unwrap_or(trimmed.len());
        return trimmed[start..end].to_string();
    }
    trimmed
        .trim_start_matches("```latex")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_prompt_has_rules_and_inputs() {
        let prompt = build_match_prompt("Backend role, needs Rust", "SKILLS\nRust, SQL");
        assert!(prompt.contains("ONLY contain skills present"));
        assert!(prompt.contains("Backend role, needs Rust"));
        assert!(prompt.contains("Rust, SQL"));
        assert!(prompt.contains("recommendedCategory"));
    }

    #[test]
    fn parses_jd_analysis_from_fenced_json() {
        let raw = "```json\n{\"role\": \"Backend Engineer\", \"seniority\": \"intern\",\n\
                   \"requiredSkills\": [\"Rust\"], \"preferredSkills\": [\"SQL\"],\n\
                   \"matchedSkills\": [\"Rust\"], \"missingSkills\": [\"Kafka\"],\n\
                   \"recommendedCategory\": \"backend\"}\n```";
        let analysis = parse_jd_analysis(raw).unwrap();
        assert_eq!(analysis.role, "Backend Engineer");
        assert_eq!(analysis.matched_skills, vec!["Rust"]);
        assert_eq!(analysis.missing_skills, vec!["Kafka"]);
        assert_eq!(analysis.recommended_category.as_deref(), Some("backend"));
        assert!(parse_jd_analysis("nothing here").is_err());
    }

    #[test]
    fn generation_prompt_contains_all_inputs_and_rules() {
        let prompt = build_generation_prompt(
            "\\documentclass{article}\n\\begin{document}x\\end{document}",
            "Name: Jane",
            "Needs Rust + Kafka",
            Some("backend"),
        );
        assert!(prompt.contains("NEVER invent"));
        assert!(prompt.contains("\\documentclass{article}"));
        assert!(prompt.contains("Needs Rust + Kafka"));
        assert!(prompt.contains("Name: Jane"));
        assert!(prompt.contains("Target category: backend"));
        assert!(prompt.contains("compile with pdflatex"));
    }

    #[test]
    fn latex_fences_are_stripped() {
        assert_eq!(
            strip_latex_fences("```latex\n\\documentclass{a}\n\\end{document}\n```"),
            "\\documentclass{a}\n\\end{document}"
        );
        assert_eq!(
            strip_latex_fences("Sure:\n\\documentclass{a}\n\\end{document}\nDone"),
            "\\documentclass{a}\n\\end{document}"
        );
        assert_eq!(strip_latex_fences("\\begin{document}"), "\\begin{document}");
    }
}
