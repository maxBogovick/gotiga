-- Record where a password reset was actually applied from (the user opening the
-- admin-issued reset link and setting a new password). The reset *request* is
-- admin-initiated, so only the apply step has a meaningful user IP. Best-effort,
-- same policy as signup/login geo. "last_*" — only the most recent reset is kept.
ALTER TABLE users
    ADD COLUMN last_reset_ip           TEXT,
    ADD COLUMN last_reset_country_code TEXT,
    ADD COLUMN last_reset_city         TEXT,
    ADD COLUMN last_reset_at           TIMESTAMPTZ;
