-- Store the source work for "create similar" petitions.
-- The source is intentionally soft-linked: old archive entries may be hidden or
-- renamed, but the petition must keep the original source id.

ALTER TABLE commissions
    ADD COLUMN source_figurine_id TEXT,
    ADD COLUMN similar_keep_note TEXT,
    ADD COLUMN similar_change_note TEXT,
    ADD COLUMN similar_tags TEXT[] NOT NULL DEFAULT '{}';

CREATE INDEX commissions_source_figurine_id_idx ON commissions (source_figurine_id);
