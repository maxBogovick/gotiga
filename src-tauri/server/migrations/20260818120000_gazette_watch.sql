-- A sketch leaf may whisper when the work might be laid out,
-- and a visitor may leave a name to be told when it is.

ALTER TABLE gazette_leaves
    ADD COLUMN IF NOT EXISTS expected_from DATE,
    ADD COLUMN IF NOT EXISTS expected_to DATE;

ALTER TABLE gazette_leaves
    DROP CONSTRAINT IF EXISTS gazette_leaves_expected_range;

ALTER TABLE gazette_leaves
    ADD CONSTRAINT gazette_leaves_expected_range CHECK (
        (expected_from IS NULL AND expected_to IS NULL)
        OR (
            expected_from IS NOT NULL
            AND expected_to IS NOT NULL
            AND expected_from <= expected_to
        )
    );

CREATE TABLE IF NOT EXISTS gazette_watches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    leaf_id UUID NOT NULL REFERENCES gazette_leaves(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    name TEXT,
    lang TEXT NOT NULL DEFAULT 'en',
    cancel_token TEXT NOT NULL UNIQUE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    notified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS gazette_watches_leaf_email
    ON gazette_watches (leaf_id, lower(email));

CREATE INDEX IF NOT EXISTS gazette_watches_user
    ON gazette_watches (user_id)
    WHERE user_id IS NOT NULL;
