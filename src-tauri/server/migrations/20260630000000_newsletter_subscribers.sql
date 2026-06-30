-- The house "visitor book": email capture for "letters from the workshop".
-- Single opt-in — a signature is active immediately. A soft unsubscribe keeps a
-- suppression record (unsubscribed_at) so a returning unsubscribe link, or a
-- later re-sign of the same email, stays idempotent and never duplicates.
CREATE TABLE IF NOT EXISTS newsletter_subscribers (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email             TEXT NOT NULL,
    name              TEXT,
    source            TEXT NOT NULL DEFAULT 'home',
    lang              TEXT NOT NULL DEFAULT 'en',
    unsubscribe_token TEXT NOT NULL,
    ip                TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    unsubscribed_at   TIMESTAMPTZ
);

-- One row per email (case-insensitive); also the conflict target for the
-- subscribe upsert, so two concurrent sign-ups from the same address can't
-- create duplicate rows.
CREATE UNIQUE INDEX IF NOT EXISTS idx_subscribers_email
    ON newsletter_subscribers (lower(email));

-- Unguessable token for the unsubscribe link (the visitor's way out).
CREATE UNIQUE INDEX IF NOT EXISTS idx_subscribers_unsub_token
    ON newsletter_subscribers (unsubscribe_token);
