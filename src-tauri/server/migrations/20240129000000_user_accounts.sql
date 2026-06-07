-- User accounts with visual password authentication

CREATE TABLE users (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email        TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    visual_password_hash TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_users_email ON users(email);

-- Sessions: 30-day tokens
CREATE TABLE user_sessions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token      TEXT UNIQUE NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_sessions_token   ON user_sessions(token);
CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);

-- Login challenges: one-time token grid, expires after 5 minutes
-- tokens_json: [{token, category, icon_id}, ...]
CREATE TABLE login_challenges (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT NOT NULL,
    tokens_json JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '5 minutes',
    used_at     TIMESTAMPTZ
);

CREATE INDEX idx_login_challenges_email ON login_challenges(email);

-- Login attempts: for lockout logic (5 failures → 15 min block)
CREATE TABLE login_attempts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email        TEXT NOT NULL,
    success      BOOLEAN NOT NULL DEFAULT false,
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_login_attempts_email_time ON login_attempts(email, attempted_at);

-- Link existing bookings and orders to user accounts
ALTER TABLE figurine_bookings ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE orders             ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE SET NULL;
