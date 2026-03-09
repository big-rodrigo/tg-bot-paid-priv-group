-- Add 'info' to the question_type CHECK constraint
ALTER TABLE questions DROP CONSTRAINT IF EXISTS questions_question_type_check;
ALTER TABLE questions ADD CONSTRAINT questions_question_type_check
    CHECK(question_type IN ('button', 'text', 'image', 'info'));

-- Settings table for admin-configurable values
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO settings (key, value) VALUES (
    'welcome_message',
    '<b>Welcome!</b> 👋

This bot manages access to our exclusive private groups.

Here''s how it works:
1. Complete a short registration process
2. Make a payment
3. Receive your personal invite links to all groups

Let''s get started!'
) ON CONFLICT DO NOTHING;
