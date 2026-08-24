ALTER TABLE generated_emails ADD COLUMN follow_up_id INTEGER REFERENCES follow_ups(id) ON DELETE SET NULL;
