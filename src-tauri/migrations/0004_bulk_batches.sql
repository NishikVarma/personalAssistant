CREATE TABLE bulk_batches (
    id INTEGER PRIMARY KEY,
    email_type TEXT NOT NULL
        CHECK (email_type IN ('cold_outreach','job_application','referral_request',
                              'follow_up','internship_inquiry','application_status')),
    application_id INTEGER REFERENCES applications(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft','sending','sent','failed')),
    total_count INTEGER NOT NULL DEFAULT 0,
    sent_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

ALTER TABLE generated_emails ADD COLUMN bulk_batch_id INTEGER REFERENCES bulk_batches(id) ON DELETE SET NULL;
