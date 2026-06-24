-- Named "showing rooms" — a shared showing window several works point at, so a
-- group of pieces can be re-timed in one place ("Night hall" 23:00→04:00). A
-- figurine either references a room (showing_room_id) OR carries its own
-- open_from_min/open_until_min, never both. Deleting a room frees its works
-- (ON DELETE SET NULL → they resolve to always-open).
CREATE TABLE IF NOT EXISTS showing_rooms (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    open_from_min INTEGER NOT NULL,
    open_until_min INTEGER NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE figurines
    ADD COLUMN IF NOT EXISTS showing_room_id UUID REFERENCES showing_rooms(id) ON DELETE SET NULL;
