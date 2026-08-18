-- House stamps for world-desk feeds. A named glyph by default; mark_url
-- holds a picture the keeper laid over it.

ALTER TABLE gazette_feeds
    ADD COLUMN IF NOT EXISTS mark_key TEXT NOT NULL DEFAULT 'letter',
    ADD COLUMN IF NOT EXISTS mark_url TEXT;

UPDATE gazette_feeds SET mark_key = 'pillar' WHERE url ILIKE '%thisiscolossal%' AND mark_key = 'letter';
UPDATE gazette_feeds SET mark_key = 'hive'   WHERE url ILIKE '%hyperallergic%'  AND mark_key = 'letter';
UPDATE gazette_feeds SET mark_key = 'boom'   WHERE url ILIKE '%designboom%'     AND mark_key = 'letter';
