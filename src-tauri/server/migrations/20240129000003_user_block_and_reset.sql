ALTER TABLE users
  ADD COLUMN is_blocked          BOOLEAN   NOT NULL DEFAULT false,
  ADD COLUMN password_reset_token      TEXT,
  ADD COLUMN password_reset_expires_at TIMESTAMPTZ;

CREATE UNIQUE INDEX idx_users_reset_token ON users(password_reset_token) WHERE password_reset_token IS NOT NULL;
