-- Record where a self-service password-reset link was *requested* from
-- (POST /auth/forgot-password). Distinct from last_reset_* which records where
-- the reset was actually applied. Best-effort geo, only set when the email maps
-- to a real account. "last_*" — only the most recent request is kept.
ALTER TABLE users
    ADD COLUMN last_reset_request_ip           TEXT,
    ADD COLUMN last_reset_request_country_code TEXT,
    ADD COLUMN last_reset_request_city         TEXT,
    ADD COLUMN last_reset_request_at           TIMESTAMPTZ;
