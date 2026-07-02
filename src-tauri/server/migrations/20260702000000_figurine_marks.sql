-- "Marks of attention" — a quiet, single wax-seal gesture a visitor can leave on a
-- figurine. Deliberately NOT a rating: no numeric value, no public count. Counts are
-- only ever surfaced in the admin panel (curation signal), never on the public site —
-- see project decision to avoid vanity-metric / negative-social-proof effects.
CREATE TABLE IF NOT EXISTS figurine_marks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    visitor_token VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT figurine_marks_unique UNIQUE (figurine_id, visitor_token)
);

CREATE INDEX IF NOT EXISTS idx_figurine_marks_figurine ON figurine_marks(figurine_id);
CREATE INDEX IF NOT EXISTS idx_figurine_marks_created ON figurine_marks(created_at DESC);
