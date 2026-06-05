-- Showings: admin-managed exhibition/private show schedule
CREATE TYPE showing_type AS ENUM ('exhibition', 'private');

-- Bookings: user requests to borrow/exhibit a figurine for a period
CREATE TYPE booking_status AS ENUM ('pending', 'confirmed', 'rejected', 'cancelled');

CREATE TABLE IF NOT EXISTS figurine_showings (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    showing_type showing_type NOT NULL DEFAULT 'exhibition',
    starts_at   DATE NOT NULL,
    ends_at     DATE NOT NULL,
    venue       TEXT,
    notes       TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_figurine_showings_figurine_id ON figurine_showings(figurine_id);
CREATE INDEX IF NOT EXISTS idx_figurine_showings_dates ON figurine_showings(starts_at, ends_at);

CREATE TABLE IF NOT EXISTS figurine_bookings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id     UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    figurine_name   TEXT NOT NULL,
    requester_name  TEXT NOT NULL,
    requester_email TEXT NOT NULL,
    purpose         TEXT,
    starts_at       DATE NOT NULL,
    ends_at         DATE NOT NULL,
    status          booking_status NOT NULL DEFAULT 'pending',
    admin_notes     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_figurine_bookings_figurine_id ON figurine_bookings(figurine_id);
CREATE INDEX IF NOT EXISTS idx_figurine_bookings_status ON figurine_bookings(status);
CREATE INDEX IF NOT EXISTS idx_figurine_bookings_dates ON figurine_bookings(starts_at, ends_at);
