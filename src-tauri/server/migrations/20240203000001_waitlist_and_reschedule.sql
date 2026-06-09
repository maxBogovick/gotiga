-- Waitlist: users who want to be notified when a figurine becomes available
CREATE TABLE IF NOT EXISTS figurine_waitlist (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id     UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    figurine_name   TEXT NOT NULL,
    requester_name  TEXT NOT NULL,
    requester_email TEXT NOT NULL,
    requester_phone TEXT,
    note            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_waitlist_figurine_id ON figurine_waitlist(figurine_id);
CREATE INDEX IF NOT EXISTS idx_waitlist_email       ON figurine_waitlist(requester_email);
