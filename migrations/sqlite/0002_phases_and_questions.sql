CREATE TABLE IF NOT EXISTS phases (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    description TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    active      BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS questions (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id      INTEGER NOT NULL REFERENCES phases(id) ON DELETE CASCADE,
    text          TEXT    NOT NULL,
    question_type TEXT    NOT NULL CHECK(question_type IN ('button', 'text', 'image')),
    position      INTEGER NOT NULL DEFAULT 0,
    required      BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS question_options (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    label       TEXT    NOT NULL,
    value       TEXT    NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS answers (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id       INTEGER NOT NULL REFERENCES users(id),
    question_id   INTEGER NOT NULL REFERENCES questions(id),
    text_value    TEXT,
    option_id     INTEGER REFERENCES question_options(id),
    image_file_id TEXT,
    created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, question_id)
);

CREATE TABLE IF NOT EXISTS user_registration (
    user_id             INTEGER NOT NULL REFERENCES users(id) PRIMARY KEY,
    current_phase_id    INTEGER REFERENCES phases(id),
    current_question_id INTEGER REFERENCES questions(id),
    completed_at        DATETIME
);
