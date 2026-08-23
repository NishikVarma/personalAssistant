use sqlx::SqlitePool;

use crate::error::AppResult;
use crate::models::profile::{Achievement, Certification, Education, Experience, Link, Project, Skill, UserProfile};

const MAX_CHARS: usize = 8000;
const ITEM_CLIP: usize = 240;

fn clip(value: &str, max: usize) -> String {
    let trimmed = value.trim();
    let mut out = String::new();
    for (i, ch) in trimmed.chars().enumerate() {
        if i >= max {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

fn push_line(out: &mut String, line: &str) {
    if !line.trim().is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line.trim_end());
    }
}

fn verified_suffix(verified: bool) -> &'static str {
    if verified { " (verified)" } else { "" }
}

async fn load_bullets(pool: &SqlitePool, entity_type: &str, entity_id: i64) -> Vec<String> {
    #[derive(sqlx::FromRow)]
    struct Row {
        content: String,
    }
    sqlx::query_as::<_, Row>(
        "SELECT content FROM bullets WHERE entity_type = ?1 AND entity_id = ?2
         ORDER BY verified DESC, display_order, id",
    )
    .bind(entity_type)
    .bind(entity_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| clip(&r.content, ITEM_CLIP))
    .collect()
}

/// Builds a compact plain-text block describing everything in the career profile.
/// This is the ONLY candidate information the AI is allowed to use.
pub async fn collect(pool: &SqlitePool) -> AppResult<String> {
    let mut out = String::new();

    let profile = sqlx::query_as::<_, UserProfile>("SELECT * FROM user_profile WHERE id = 1")
        .fetch_optional(pool)
        .await?
        .unwrap_or(UserProfile {
            id: 1,
            full_name: String::new(),
            email: String::new(),
            phone: String::new(),
            location: String::new(),
            summary: String::new(),
            verified: false,
            created_at: String::new(),
            updated_at: String::new(),
        });
    {
        push_line(&mut out, "ABOUT THE CANDIDATE");
        if !profile.full_name.is_empty() {
            push_line(&mut out, &format!("Name: {}", profile.full_name));
        }
        if !profile.email.is_empty() {
            push_line(&mut out, &format!("Email: {}", profile.email));
        }
        if !profile.location.is_empty() {
            push_line(&mut out, &format!("Location: {}", profile.location));
        }
        if !profile.summary.is_empty() {
            push_line(&mut out, &format!("Summary: {}", clip(&profile.summary, 600)));
        }
    }

    let education = sqlx::query_as::<_, Education>("SELECT * FROM education ORDER BY id")
        .fetch_all(pool)
        .await?;
    if !education.is_empty() {
        push_line(&mut out, "\nEDUCATION");
        for e in education {
            let dates = match (e.start_date.as_deref(), e.end_date.as_deref()) {
                (Some(s), Some(t)) => format!(" ({s} – {t})"),
                (Some(s), None) => format!(" (from {s})"),
                _ => String::new(),
            };
            push_line(
                &mut out,
                &format!(
                    "- {}{}{}{}",
                    e.institution,
                    if e.degree.is_empty() {
                        String::new()
                    } else {
                        format!(", {} {}", e.degree, e.field_of_study)
                            .trim()
                            .to_string()
                    },
                    dates,
                    verified_suffix(e.verified),
                ),
            );
        }
    }

    let experience = sqlx::query_as::<_, Experience>("SELECT * FROM experience ORDER BY id")
        .fetch_all(pool)
        .await?;
    for x in experience {
        push_line(&mut out, "\nEXPERIENCE");
        let period = match (x.start_date.as_deref(), x.end_date.as_deref()) {
            (Some(s), Some(t)) => format!("{s} – {t}"),
            (Some(s), None) if x.currently_working => format!("{s} – present"),
            (Some(s), None) => s.to_string(),
            (_, Some(t)) => t.to_string(),
            _ => String::new(),
        };
        push_line(
            &mut out,
            &format!(
                "- {} — {} [{}] {}{}",
                x.organization,
                x.title,
                x.employment_type,
                period,
                verified_suffix(x.verified),
            ),
        );
        if !x.description.is_empty() {
            push_line(&mut out, &format!("  {}", clip(&x.description, ITEM_CLIP)));
        }
        for bullet in load_bullets(pool, "experience", x.id).await {
            push_line(&mut out, &format!("  · {bullet}"));
        }
    }

    let projects = sqlx::query_as::<_, Project>("SELECT * FROM projects ORDER BY id")
        .fetch_all(pool)
        .await?;
    for p in projects {
        push_line(&mut out, "\nPROJECTS");
        push_line(
            &mut out,
            &format!(
                "- {} [{}]{}",
                p.name,
                p.status,
                verified_suffix(p.verified),
            ),
        );
        if !p.description.is_empty() {
            push_line(&mut out, &format!("  {}", clip(&p.description, ITEM_CLIP)));
        }
        for bullet in load_bullets(pool, "project", p.id).await {
            push_line(&mut out, &format!("  · {bullet}"));
        }
    }

    let skills = sqlx::query_as::<_, Skill>(
        "SELECT * FROM skills ORDER BY category, name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await?;
    if !skills.is_empty() {
        push_line(&mut out, "\nSKILLS");
        let names: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();
        push_line(&mut out, &clip(&names.join(", "), 1200));
    }

    let certifications = sqlx::query_as::<_, Certification>(
        "SELECT * FROM certifications ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    if !certifications.is_empty() {
        push_line(&mut out, "\nCERTIFICATIONS");
        for c in certifications {
            push_line(
                &mut out,
                &format!(
                    "- {}{}{}",
                    c.name,
                    if c.issuer.is_empty() {
                        String::new()
                    } else {
                        format!(" — {}", c.issuer)
                    },
                    verified_suffix(c.verified),
                ),
            );
        }
    }

    let achievements =
        sqlx::query_as::<_, Achievement>("SELECT * FROM achievements ORDER BY id")
            .fetch_all(pool)
            .await?;
    if !achievements.is_empty() {
        push_line(&mut out, "\nACHIEVEMENTS");
        for a in achievements {
            push_line(
                &mut out,
                &format!(
                    "- {}{}{}",
                    a.title,
                    a.date.as_deref().map(|d| format!(" ({})", &d[..10.min(d.len())])).unwrap_or_default(),
                    verified_suffix(a.verified),
                ),
            );
            if !a.description.is_empty() {
                push_line(&mut out, &format!("  {}", clip(&a.description, ITEM_CLIP)));
            }
        }
    }

    let links = sqlx::query_as::<_, Link>("SELECT * FROM links ORDER BY id")
        .fetch_all(pool)
        .await?;
    if !links.is_empty() {
        push_line(&mut out, "\nLINKS");
        for l in links {
            push_line(&mut out, &format!("- {} — {}", l.label, l.url));
        }
    }

    if out.chars().count() > MAX_CHARS {
        let clipped: String = out.chars().take(MAX_CHARS).collect();
        return Ok(format!("{clipped}\n[profile truncated]"));
    }
    Ok(if out.is_empty() {
        "(no profile data filled in yet)".to_string()
    } else {
        out
    })
}
