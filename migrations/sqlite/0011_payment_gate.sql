-- New columns on phases for payment gate configuration
ALTER TABLE phases ADD COLUMN rejection_text TEXT;
ALTER TABLE phases ADD COLUMN clean_chat BOOLEAN NOT NULL DEFAULT FALSE;

-- Track first message ID for chat cleanup on rejection
ALTER TABLE user_registration ADD COLUMN first_message_id BIGINT;

-- Payment gate conditions (same condition types as invite_rule_conditions, linked to phase directly)
CREATE TABLE IF NOT EXISTS payment_gate_conditions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id        INTEGER NOT NULL REFERENCES phases(id) ON DELETE CASCADE,
    question_id     INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    condition_type  TEXT NOT NULL CHECK(condition_type IN (
        'option_selected', 'option_not_selected',
        'text_contains', 'text_not_contains'
    )),
    option_id       INTEGER REFERENCES question_options(id) ON DELETE CASCADE,
    text_value      TEXT
);
