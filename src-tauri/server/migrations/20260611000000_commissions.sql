-- Commission requests: a petition to the master to create a NEW figurine.
-- Can be submitted by a guest (user_id NULL) and later claimed via claim_token.

CREATE TYPE commission_status AS ENUM (
    'new', 'reviewing', 'accepted', 'in_progress', 'completed', 'declined'
);

CREATE TABLE commissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID REFERENCES users(id) ON DELETE SET NULL,
    claim_token     TEXT NOT NULL UNIQUE,
    -- contact
    requester_name  TEXT NOT NULL DEFAULT '',
    requester_email TEXT NOT NULL,
    requester_phone TEXT,
    -- the idea
    title           TEXT NOT NULL DEFAULT '',
    description     TEXT NOT NULL,
    -- optional details
    size_note       TEXT,
    mood            TEXT,
    deadline        DATE,
    budget_note     TEXT,
    occasion        TEXT,
    -- lifecycle
    figurine_id     TEXT,                                   -- linked real figurine once accepted
    status          commission_status NOT NULL DEFAULT 'new',
    admin_notes     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX commissions_status_idx      ON commissions (status);
CREATE INDEX commissions_created_at_idx  ON commissions (created_at DESC);
CREATE INDEX commissions_user_id_idx     ON commissions (user_id);
CREATE INDEX commissions_claim_token_idx ON commissions (claim_token);
