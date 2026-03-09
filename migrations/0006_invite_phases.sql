-- Add phase_type column to phases table
-- SQLite ALTER TABLE ADD COLUMN doesn't support CHECK constraints,
-- so phase_type validation is enforced at the application layer.
ALTER TABLE phases ADD COLUMN phase_type TEXT NOT NULL DEFAULT 'normal';

-- Invite rules: each rule belongs to an invite phase and links to a group
CREATE TABLE IF NOT EXISTS invite_rules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id    INTEGER NOT NULL REFERENCES phases(id) ON DELETE CASCADE,
    group_id    INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL DEFAULT 0,
    UNIQUE(phase_id, group_id)
);

-- Conditions for each invite rule (AND logic: all must pass)
CREATE TABLE IF NOT EXISTS invite_rule_conditions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    invite_rule_id  INTEGER NOT NULL REFERENCES invite_rules(id) ON DELETE CASCADE,
    question_id     INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    condition_type  TEXT NOT NULL CHECK(condition_type IN (
        'option_selected', 'option_not_selected',
        'text_contains', 'text_not_contains'
    )),
    option_id       INTEGER REFERENCES question_options(id) ON DELETE CASCADE,
    text_value      TEXT
);
