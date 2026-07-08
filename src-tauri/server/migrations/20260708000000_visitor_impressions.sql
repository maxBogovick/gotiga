-- The "Book of Impressions": free-form visitor reactions to the exhibition as a
-- whole (not tied to a single figurine, unlike figurine_comments). Curator
-- moderates before anything is public, then hand-picks a subset to feature as
-- quotes on the site (is_featured).
CREATE TABLE IF NOT EXISTS visitor_impressions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message       TEXT NOT NULL CHECK (char_length(message) >= 1 AND char_length(message) <= 400),
    author_name   TEXT,
    mood          TEXT,
    is_approved   BOOLEAN NOT NULL DEFAULT FALSE,
    is_featured   BOOLEAN NOT NULL DEFAULT FALSE,
    ip            TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_visitor_impressions_pending  ON visitor_impressions(created_at) WHERE NOT is_approved;
CREATE INDEX idx_visitor_impressions_featured ON visitor_impressions(created_at) WHERE is_approved AND is_featured;
