ALTER TABLE generated_emails ADD COLUMN recipient_email TEXT;
ALTER TABLE generated_emails ADD COLUMN recipient_name TEXT;
ALTER TABLE email_history ADD COLUMN recipient_email TEXT;
