-- Record originating IP, user-agent and (offline-resolved) geolocation for
-- login attempts and sessions. All nullable + best-effort: geo is filled from a
-- local MaxMind database when available, NULL otherwise (private IPs, no DB,
-- legacy rows). country_code is ISO-3166-1 alpha-2.

ALTER TABLE login_attempts
    ADD COLUMN ip           TEXT,
    ADD COLUMN user_agent   TEXT,
    ADD COLUMN country_code TEXT,
    ADD COLUMN city         TEXT;

ALTER TABLE user_sessions
    ADD COLUMN ip           TEXT,
    ADD COLUMN user_agent   TEXT,
    ADD COLUMN country_code TEXT,
    ADD COLUMN city         TEXT;

-- Speeds up "recent attempts from this IP" audit/abuse queries.
CREATE INDEX idx_login_attempts_ip_time ON login_attempts(ip, attempted_at);
