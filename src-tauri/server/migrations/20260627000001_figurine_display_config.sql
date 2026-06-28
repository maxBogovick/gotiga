-- Per-figurine display customisation config (background preset + block order).
-- NULL → no customisation applied; layout defaults are used.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS display_config TEXT;
