-- "The house wakes" — per-figurine showing window. A work is only enterable
-- while the visitor's local clock is inside [open_from_min, open_until_min],
-- both expressed as minutes from midnight (0..1439). The window may wrap past
-- midnight (until < from) for night-only rooms. Both NULL → always open, so
-- existing works are unaffected. A repeating daily window, never a vanishing
-- drop: miss it and the door simply opens again tomorrow.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS open_from_min INTEGER;
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS open_until_min INTEGER;
