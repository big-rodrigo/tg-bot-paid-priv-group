-- Recreate questions table adding 'info' to the type CHECK constraint
-- (SQLite foreign key enforcement is off by default; sqlx wraps this in its own transaction)
CREATE TABLE questions_new (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id      INTEGER NOT NULL REFERENCES phases(id) ON DELETE CASCADE,
    text          TEXT    NOT NULL,
    question_type TEXT    NOT NULL CHECK(question_type IN ('button', 'text', 'image', 'info')),
    position      INTEGER NOT NULL DEFAULT 0,
    required      BOOLEAN NOT NULL DEFAULT TRUE
);

INSERT INTO questions_new SELECT * FROM questions;
DROP TABLE questions;
ALTER TABLE questions_new RENAME TO questions;

-- Settings table for admin-configurable values
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO settings (key, value) VALUES (
    'welcome_message',
    '<b>Welcome!</b> 👋

This bot manages access to our exclusive private groups.

Here''s how it works:
1. Complete a short registration process
2. Make a payment
3. Receive your personal invite links to all groups

Let''s get started!'
);
