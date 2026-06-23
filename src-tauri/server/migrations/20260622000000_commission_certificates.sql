-- Certificate of authenticity for completed commissions.
-- Mirrors the order certificate columns (20260620000001_order_certificates.sql).
ALTER TABLE commissions
    ADD COLUMN IF NOT EXISTS certificate_token TEXT UNIQUE,
    ADD COLUMN IF NOT EXISTS certificate_number TEXT,
    ADD COLUMN IF NOT EXISTS certificate_issued_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS certificate_revoked_at TIMESTAMPTZ;
