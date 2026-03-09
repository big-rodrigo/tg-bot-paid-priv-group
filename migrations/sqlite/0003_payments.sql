CREATE TABLE IF NOT EXISTS payments (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id            INTEGER NOT NULL REFERENCES users(id),
    provider           TEXT    NOT NULL CHECK(provider IN ('external', 'telegram')),
    external_ref       TEXT,
    telegram_charge_id TEXT,
    amount             INTEGER,
    currency           TEXT,
    status             TEXT NOT NULL DEFAULT 'pending'
                           CHECK(status IN ('pending', 'completed', 'failed', 'refunded')),
    payload            TEXT,
    created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS invite_links (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id),
    group_id    INTEGER NOT NULL REFERENCES groups(id),
    invite_link TEXT    NOT NULL UNIQUE,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    used_at     DATETIME,
    revoked_at  DATETIME,
    UNIQUE(user_id, group_id)
);
