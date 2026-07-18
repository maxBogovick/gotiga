-- Lightweight "write to the author" letters: anonymous, not tied to a
-- figurine or a logged-in account (unlike orders/message_threads). A
-- deliberately two-field form (email + message) — no name field.
CREATE TABLE IF NOT EXISTS contact_messages (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email      TEXT NOT NULL,
    message    TEXT NOT NULL,
    source     TEXT NOT NULL DEFAULT 'home',
    lang       TEXT NOT NULL DEFAULT 'en',
    ip         TEXT,
    is_read    BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_contact_messages_created_at
    ON contact_messages (created_at DESC);
