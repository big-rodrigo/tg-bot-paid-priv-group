CREATE TABLE IF NOT EXISTS phases (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT    NOT NULL,
    description TEXT,
    position    BIGINT NOT NULL DEFAULT 0,
    active      BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS questions (
    id            BIGSERIAL PRIMARY KEY,
    phase_id      BIGINT NOT NULL REFERENCES phases(id) ON DELETE CASCADE,
    text          TEXT    NOT NULL,
    question_type TEXT    NOT NULL CHECK(question_type IN ('button', 'text', 'image')),
    position      BIGINT NOT NULL DEFAULT 0,
    required      BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS question_options (
    id          BIGSERIAL PRIMARY KEY,
    question_id BIGINT NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    label       TEXT    NOT NULL,
    value       TEXT    NOT NULL,
    position    BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS answers (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES users(id),
    question_id   BIGINT NOT NULL REFERENCES questions(id),
    text_value    TEXT,
    option_id     BIGINT REFERENCES question_options(id),
    image_file_id TEXT,
    created_at    TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, question_id)
);

CREATE TABLE IF NOT EXISTS user_registration (
    user_id             BIGINT NOT NULL REFERENCES users(id) PRIMARY KEY,
    current_phase_id    BIGINT REFERENCES phases(id),
    current_question_id BIGINT REFERENCES questions(id),
    completed_at        TIMESTAMP
);
