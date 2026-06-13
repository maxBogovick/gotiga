-- Queue management: give each waitlist entry an unguessable cancel token so
-- anonymous visitors (no account) get a receipt and can leave the queue.
ALTER TABLE figurine_waitlist
    ADD COLUMN IF NOT EXISTS cancel_token TEXT;

-- Backfill existing rows with a short random token (XXXX-XXXX), matching the
-- booking cancel-token format.
UPDATE figurine_waitlist
SET cancel_token = upper(
        substr(replace(gen_random_uuid()::text, '-', ''), 1, 4) || '-' ||
        substr(replace(gen_random_uuid()::text, '-', ''), 1, 4)
    )
WHERE cancel_token IS NULL;

ALTER TABLE figurine_waitlist ALTER COLUMN cancel_token SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_waitlist_cancel_token ON figurine_waitlist(cancel_token);
