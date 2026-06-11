-- Primary reference images attached to a commission request itself.

CREATE TABLE commission_attachments (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    commission_id UUID NOT NULL REFERENCES commissions(id) ON DELETE CASCADE,
    url           TEXT NOT NULL,
    thumb_url     TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX commission_attachments_commission_id_idx
    ON commission_attachments (commission_id);
