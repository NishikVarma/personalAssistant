CREATE TABLE user_profile (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    full_name TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    phone TEXT NOT NULL DEFAULT '',
    location TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE education (
    id INTEGER PRIMARY KEY,
    institution TEXT NOT NULL,
    degree TEXT NOT NULL DEFAULT '',
    field_of_study TEXT NOT NULL DEFAULT '',
    start_date TEXT,
    end_date TEXT,
    grade TEXT,
    location TEXT,
    details TEXT NOT NULL DEFAULT '',
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE experience (
    id INTEGER PRIMARY KEY,
    organization TEXT NOT NULL,
    title TEXT NOT NULL,
    employment_type TEXT NOT NULL DEFAULT 'full_time'
        CHECK (employment_type IN ('internship','full_time','part_time','contract','freelance')),
    location TEXT,
    start_date TEXT,
    end_date TEXT,
    currently_working INTEGER NOT NULL DEFAULT 0,
    description TEXT NOT NULL DEFAULT '',
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE projects (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    repo_url TEXT,
    live_url TEXT,
    status TEXT NOT NULL DEFAULT 'completed'
        CHECK (status IN ('ongoing','completed','archived')),
    started_on TEXT,
    ended_on TEXT,
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE skills (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    category TEXT NOT NULL DEFAULT 'other'
        CHECK (category IN ('language','framework','tool','database','cloud','soft_skill','other')),
    created_at TEXT NOT NULL
);

CREATE TABLE entity_skills (
    entity_type TEXT NOT NULL CHECK (entity_type IN ('project','experience')),
    entity_id INTEGER NOT NULL,
    skill_id INTEGER NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    PRIMARY KEY (entity_type, entity_id, skill_id)
);
CREATE INDEX idx_entity_skills_entity ON entity_skills(entity_type, entity_id);

CREATE TABLE bullets (
    id INTEGER PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('project','experience')),
    entity_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    verified INTEGER NOT NULL DEFAULT 0,
    display_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_bullets_entity ON bullets(entity_type, entity_id);

CREATE TABLE certifications (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    issuer TEXT NOT NULL DEFAULT '',
    issue_date TEXT,
    expiry_date TEXT,
    credential_url TEXT,
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE achievements (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    date TEXT,
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE links (
    id INTEGER PRIMARY KEY,
    label TEXT NOT NULL,
    url TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'other'
        CHECK (kind IN ('linkedin','github','portfolio','other')),
    created_at TEXT NOT NULL
);

CREATE TABLE resume_files (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('pdf_master','tex_template')),
    original_filename TEXT NOT NULL,
    stored_path TEXT NOT NULL UNIQUE,
    sha256 TEXT NOT NULL,
    file_size INTEGER NOT NULL DEFAULT 0,
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_resume_files_kind ON resume_files(kind);

CREATE TABLE applications (
    id INTEGER PRIMARY KEY,
    company TEXT NOT NULL,
    role TEXT NOT NULL,
    job_description TEXT NOT NULL DEFAULT '',
    job_url TEXT,
    source TEXT,
    status TEXT NOT NULL DEFAULT 'saved'
        CHECK (status IN ('saved','preparing','applied','contacted','follow_up_due',
                          'response_received','oa','interview','offer','rejected','withdrawn')),
    date_discovered TEXT,
    date_applied TEXT,
    follow_up_date TEXT,
    interview_status TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    notes TEXT NOT NULL DEFAULT '',
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_applications_status ON applications(status);
CREATE INDEX idx_applications_company ON applications(company COLLATE NOCASE);

CREATE TABLE contacts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    organization TEXT,
    role_title TEXT,
    linkedin_url TEXT,
    notes TEXT NOT NULL DEFAULT '',
    last_contacted_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_contacts_organization ON contacts(organization COLLATE NOCASE);

CREATE TABLE tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    color TEXT
);

CREATE TABLE contact_tags (
    contact_id INTEGER NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (contact_id, tag_id)
);

CREATE TABLE application_contacts (
    application_id INTEGER NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    contact_id INTEGER NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    relationship TEXT NOT NULL DEFAULT 'other'
        CHECK (relationship IN ('recruiter','hr','referral','hiring_manager','colleague','other')),
    is_primary INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (application_id, contact_id)
);

CREATE TABLE email_templates (
    id INTEGER PRIMARY KEY,
    email_type TEXT NOT NULL
        CHECK (email_type IN ('cold_outreach','job_application','referral_request',
                              'follow_up','internship_inquiry','application_status')),
    role TEXT,
    company_or_industry TEXT,
    subject_template TEXT,
    body_template TEXT NOT NULL,
    variables_json TEXT NOT NULL DEFAULT '[]',
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user','generated','imported')),
    success_count INTEGER NOT NULL DEFAULT 0,
    times_used INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_email_templates_type ON email_templates(email_type);

CREATE TABLE generated_emails (
    id INTEGER PRIMARY KEY,
    application_id INTEGER REFERENCES applications(id) ON DELETE SET NULL,
    contact_id INTEGER REFERENCES contacts(id) ON DELETE SET NULL,
    email_type TEXT NOT NULL
        CHECK (email_type IN ('cold_outreach','job_application','referral_request',
                              'follow_up','internship_inquiry','application_status')),
    subject TEXT,
    body TEXT NOT NULL,
    provider TEXT,
    model TEXT,
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft','edited','approved','sent','discarded')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_generated_emails_application ON generated_emails(application_id);
CREATE INDEX idx_generated_emails_contact ON generated_emails(contact_id);
CREATE INDEX idx_generated_emails_status ON generated_emails(status);

CREATE TABLE email_history (
    id INTEGER PRIMARY KEY,
    direction TEXT NOT NULL CHECK (direction IN ('outgoing','incoming')),
    application_id INTEGER REFERENCES applications(id) ON DELETE SET NULL,
    contact_id INTEGER REFERENCES contacts(id) ON DELETE SET NULL,
    generated_email_id INTEGER REFERENCES generated_emails(id) ON DELETE SET NULL,
    gmail_message_id TEXT,
    gmail_thread_id TEXT,
    email_type TEXT,
    subject TEXT,
    body TEXT NOT NULL DEFAULT '',
    delivery_method TEXT
        CHECK (delivery_method IS NULL OR delivery_method IN ('gmail_api','clipboard','manual')),
    status TEXT NOT NULL DEFAULT 'sent'
        CHECK (status IN ('sent','draft_saved','received','failed')),
    response_status TEXT
        CHECK (response_status IS NULL OR response_status IN ('awaiting','replied','no_reply_needed')),
    occurred_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_email_history_application ON email_history(application_id);
CREATE INDEX idx_email_history_contact ON email_history(contact_id);
CREATE INDEX idx_email_history_gmail_message ON email_history(gmail_message_id);

CREATE TABLE follow_ups (
    id INTEGER PRIMARY KEY,
    application_id INTEGER NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    contact_id INTEGER REFERENCES contacts(id) ON DELETE SET NULL,
    originating_email_id INTEGER REFERENCES email_history(id) ON DELETE SET NULL,
    sequence INTEGER NOT NULL DEFAULT 1,
    scheduled_for TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending','due','sent','cancelled','suppressed')),
    suppressed_reason TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_follow_ups_due ON follow_ups(status, scheduled_for);
CREATE INDEX idx_follow_ups_application ON follow_ups(application_id);

CREATE TABLE resume_variants (
    id INTEGER PRIMARY KEY,
    base_file_id INTEGER REFERENCES resume_files(id) ON DELETE SET NULL,
    application_id INTEGER REFERENCES applications(id) ON DELETE SET NULL,
    category TEXT NOT NULL DEFAULT 'general_swe'
        CHECK (category IN ('backend','ai_ml','full_stack','general_swe','other')),
    label TEXT NOT NULL DEFAULT '',
    tex_path TEXT,
    pdf_path TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','approved','archived')),
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_resume_variants_category ON resume_variants(category);
CREATE INDEX idx_resume_variants_application ON resume_variants(application_id);

CREATE TABLE variant_sections (
    id INTEGER PRIMARY KEY,
    variant_id INTEGER NOT NULL REFERENCES resume_variants(id) ON DELETE CASCADE,
    section_key TEXT NOT NULL
        CHECK (section_key IN ('summary','education','experience','projects','skills',
                               'certifications','achievements','links','custom')),
    title TEXT,
    content_json TEXT NOT NULL DEFAULT '{}',
    display_order INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_variant_sections_variant ON variant_sections(variant_id);

CREATE TABLE variant_bullets (
    id INTEGER PRIMARY KEY,
    section_id INTEGER NOT NULL REFERENCES variant_sections(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    source_bullet_id INTEGER REFERENCES bullets(id) ON DELETE SET NULL,
    display_order INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_variant_bullets_section ON variant_bullets(section_id);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE oauth_accounts (
    id INTEGER PRIMARY KEY,
    provider TEXT NOT NULL DEFAULT 'google',
    account_email TEXT NOT NULL,
    scopes_json TEXT NOT NULL DEFAULT '[]',
    token_expires_at TEXT,
    keyring_service TEXT NOT NULL,
    keyring_account TEXT NOT NULL,
    connected_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (provider, account_email)
);
