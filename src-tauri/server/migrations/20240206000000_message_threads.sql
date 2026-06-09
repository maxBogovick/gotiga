-- Drop old table and replace with threads model
DROP TABLE IF EXISTS user_messages;

CREATE TABLE message_threads (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category        TEXT NOT NULL DEFAULT 'general',  -- booking | waitlist | order | general | system
    reference_id    UUID,           -- optional: id of related booking/order
    subject         TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'open',     -- open | resolved
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_message_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_threads_user_id       ON message_threads(user_id);
CREATE INDEX idx_threads_status        ON message_threads(status);
CREATE INDEX idx_threads_category      ON message_threads(category);
CREATE INDEX idx_threads_last_message  ON message_threads(last_message_at DESC);

CREATE TABLE thread_messages (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id  UUID NOT NULL REFERENCES message_threads(id) ON DELETE CASCADE,
    from_admin BOOLEAN NOT NULL DEFAULT false,
    body       TEXT NOT NULL,
    read_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_thread_messages_thread_id  ON thread_messages(thread_id);
CREATE INDEX idx_thread_messages_created_at ON thread_messages(created_at);
