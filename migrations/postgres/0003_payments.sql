CREATE TABLE IF NOT EXISTS payments (
    id                 BIGSERIAL PRIMARY KEY,
    user_id            BIGINT NOT NULL REFERENCES users(id),
    provider           TEXT    NOT NULL CHECK(provider IN ('external', 'telegram')),
    external_ref       TEXT,
    telegram_charge_id TEXT,
    amount             INTEGER,
    currency           TEXT,
    status             TEXT NOT NULL DEFAULT 'pending'
                           CHECK(status IN ('pending', 'completed', 'failed', 'refunded')),
    payload            TEXT,
    created_at         TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS invite_links (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id),
    group_id    BIGINT NOT NULL REFERENCES groups(id),
    invite_link TEXT    NOT NULL UNIQUE,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW(),
    used_at     TIMESTAMP,
    revoked_at  TIMESTAMP,
    UNIQUE(user_id, group_id)
);
