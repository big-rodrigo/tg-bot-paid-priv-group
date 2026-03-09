-- Add 'livepix' to provider CHECK constraint and add price_cents column
ALTER TABLE payments DROP CONSTRAINT IF EXISTS payments_provider_check;
ALTER TABLE payments ADD CONSTRAINT payments_provider_check
    CHECK(provider IN ('external', 'telegram', 'livepix'));

ALTER TABLE payments ADD COLUMN IF NOT EXISTS price_cents BIGINT;

-- LivePix settings (editable via admin web UI)
INSERT INTO settings (key, value) VALUES ('livepix_account_url', '') ON CONFLICT DO NOTHING;
INSERT INTO settings (key, value) VALUES ('livepix_price_cents', '0') ON CONFLICT DO NOTHING;
INSERT INTO settings (key, value) VALUES ('livepix_currency', 'BRL') ON CONFLICT DO NOTHING;
