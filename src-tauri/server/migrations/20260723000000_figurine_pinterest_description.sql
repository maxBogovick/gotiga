-- Admin-only SEO copy for the Pinterest RSS auto-publish feed (feed.xml). Kept
-- separate from short_text so keyword-dense "OOAK, hand-sculpted polymer clay,
-- handmade collectible" search copy never leaks onto the public figurine page —
-- the site's quiet museum-label voice stays untouched there. NULL/blank → the
-- feed falls back to its existing short_text + material/technique/year composite.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS pinterest_description TEXT;
