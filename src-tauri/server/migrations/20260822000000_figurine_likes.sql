-- Heart likes: one row per distinct visitor (or logged-in account) per figurine,
-- so the house can later show how many different people liked a work.
-- Public pages still do not render the count (same vanity-metric caution as marks).
CREATE TABLE IF NOT EXISTS figurine_likes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    visitor_token VARCHAR(64) NOT NULL,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT figurine_likes_visitor UNIQUE (figurine_id, visitor_token)
);

CREATE UNIQUE INDEX IF NOT EXISTS figurine_likes_user
    ON figurine_likes (figurine_id, user_id)
    WHERE user_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_figurine_likes_figurine ON figurine_likes (figurine_id);
CREATE INDEX IF NOT EXISTS idx_figurine_likes_created ON figurine_likes (created_at DESC);
