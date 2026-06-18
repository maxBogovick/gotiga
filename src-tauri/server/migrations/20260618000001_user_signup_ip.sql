-- Record the IP and (offline-resolved) geolocation a user registered from.
-- Nullable + best-effort, same policy as login_attempts/user_sessions: filled
-- from the local MaxMind database when available, NULL otherwise.
ALTER TABLE users
    ADD COLUMN signup_ip           TEXT,
    ADD COLUMN signup_country_code TEXT,
    ADD COLUMN signup_city         TEXT;
