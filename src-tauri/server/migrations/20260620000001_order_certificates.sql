ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS certificate_token TEXT UNIQUE,
    ADD COLUMN IF NOT EXISTS certificate_number TEXT,
    ADD COLUMN IF NOT EXISTS certificate_issued_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS certificate_revoked_at TIMESTAMPTZ;

