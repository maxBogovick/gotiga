-- Image attachments on individual thread messages. Applies to every thread
-- category (booking | waitlist | order | commission | general), so photos can
-- be exchanged in any conversation, not only commissions.

CREATE TABLE thread_message_attachments (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES thread_messages(id) ON DELETE CASCADE,
    url        TEXT NOT NULL,
    thumb_url  TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX thread_message_attachments_message_id_idx
    ON thread_message_attachments (message_id);
