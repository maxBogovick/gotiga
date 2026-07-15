-- Human-readable, transliterated slug for share-links and SEO. NULL for legacy
-- rows until they are next saved (the service back-fills a slug on save). The
-- detail route resolves EITHER a UUID or a slug, so a NULL slug never breaks a
-- link — the work stays reachable by its UUID.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS slug TEXT;

-- Slugs must be unique so a slug resolves to exactly one work. Partial index:
-- many legacy rows may share NULL, which a plain UNIQUE would reject.
CREATE UNIQUE INDEX IF NOT EXISTS idx_figurines_slug
    ON figurines (slug)
    WHERE slug IS NOT NULL;
