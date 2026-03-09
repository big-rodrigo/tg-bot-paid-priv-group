-- Recreate payments table adding 'livepix' to provider CHECK constraint
-- and adding price_cents column (snapshot of required price at creation time).
CREATE TABLE payments_new (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id            INTEGER NOT NULL REFERENCES users(id),
    provider           TEXT    NOT NULL CHECK(provider IN ('external', 'telegram', 'livepix')),
    external_ref       TEXT,
    telegram_charge_id TEXT,
    amount             INTEGER,
    currency           TEXT,
    price_cents        INTEGER,
    status             TEXT NOT NULL DEFAULT 'pending'
                           CHECK(status IN ('pending', 'completed', 'failed', 'refunded')),
    payload            TEXT,
    created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO payments_new
    SELECT id, user_id, provider, external_ref, telegram_charge_id,
           amount, currency, NULL, status, payload, created_at, updated_at
    FROM payments;

DROP TABLE payments;
ALTER TABLE payments_new RENAME TO payments;

-- LivePix settings (editable via admin web UI)
INSERT OR IGNORE INTO settings (key, value) VALUES ('livepix_account_url', '');
INSERT OR IGNORE INTO settings (key, value) VALUES ('livepix_price_cents', '0');
INSERT OR IGNORE INTO settings (key, value) VALUES ('livepix_currency', 'BRL');
