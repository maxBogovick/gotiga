-- Deduplicate the waitlist at the DB level so two concurrent requests from the
-- same email can't create duplicate rows (previously a SELECT-then-INSERT race).

-- 1. Drop existing duplicates, keeping the earliest row per (figurine, email).
DELETE FROM figurine_waitlist a
USING figurine_waitlist b
WHERE a.figurine_id = b.figurine_id
  AND lower(a.requester_email) = lower(b.requester_email)
  AND (a.created_at > b.created_at OR (a.created_at = b.created_at AND a.id > b.id));

-- 2. Enforce uniqueness — also the conflict target for the ON CONFLICT upsert.
CREATE UNIQUE INDEX IF NOT EXISTS idx_waitlist_figurine_email
    ON figurine_waitlist (figurine_id, lower(requester_email));
