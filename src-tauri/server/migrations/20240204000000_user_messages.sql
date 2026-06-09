CREATE TABLE user_messages (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    from_admin  BOOLEAN NOT NULL DEFAULT true,
    subject     TEXT NOT NULL DEFAULT '',
    body        TEXT NOT NULL,
    read_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_messages_user_id    ON user_messages(user_id);
CREATE INDEX idx_user_messages_created_at ON user_messages(created_at DESC);
