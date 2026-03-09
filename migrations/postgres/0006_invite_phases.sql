-- Add phase_type column to phases table
ALTER TABLE phases ADD COLUMN phase_type TEXT NOT NULL DEFAULT 'normal';

-- Invite rules: each rule belongs to an invite phase and links to a group
CREATE TABLE IF NOT EXISTS invite_rules (
    id          BIGSERIAL PRIMARY KEY,
    phase_id    BIGINT NOT NULL REFERENCES phases(id) ON DELETE CASCADE,
    group_id    BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    position    BIGINT NOT NULL DEFAULT 0,
    UNIQUE(phase_id, group_id)
);

-- Conditions for each invite rule (AND logic: all must pass)
CREATE TABLE IF NOT EXISTS invite_rule_conditions (
    id              BIGSERIAL PRIMARY KEY,
    invite_rule_id  BIGINT NOT NULL REFERENCES invite_rules(id) ON DELETE CASCADE,
    question_id     BIGINT NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    condition_type  TEXT NOT NULL CHECK(condition_type IN (
        'option_selected', 'option_not_selected',
        'text_contains', 'text_not_contains'
    )),
    option_id       BIGINT REFERENCES question_options(id) ON DELETE CASCADE,
    text_value      TEXT
);
