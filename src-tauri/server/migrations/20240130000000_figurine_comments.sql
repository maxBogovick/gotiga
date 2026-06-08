CREATE TABLE figurine_comments (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id   UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    user_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    author_name   TEXT NOT NULL,
    author_email  TEXT,
    body          TEXT NOT NULL CHECK (char_length(body) >= 1 AND char_length(body) <= 1000),
    is_approved   BOOLEAN NOT NULL DEFAULT FALSE,
    admin_reply   TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_figurine_comments_figurine ON figurine_comments(figurine_id, is_approved, created_at);
CREATE INDEX idx_figurine_comments_pending  ON figurine_comments(created_at) WHERE NOT is_approved;
