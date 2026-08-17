-- The cabinet gazette: authored house leaves (a new work laid out, a showing,
-- a guest's tale) and a world desk of RSS cuttings the keeper may pin or dismiss.
-- Public surface is a blotter on the hall table, never a news feed.

CREATE TABLE IF NOT EXISTS gazette_leaves (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL,
    kind          TEXT NOT NULL CHECK (kind IN (
                      'arrival', 'collage', 'showing', 'guest_story', 'tale', 'note', 'world'
                  )),
    status        TEXT NOT NULL CHECK (status IN ('draft', 'scheduled', 'published', 'archived'))
                      DEFAULT 'draft',
    title_en      TEXT NOT NULL CHECK (char_length(title_en) BETWEEN 1 AND 200),
    title_ru      TEXT NOT NULL CHECK (char_length(title_ru) BETWEEN 1 AND 200),
    dek_en        TEXT CHECK (dek_en IS NULL OR char_length(dek_en) <= 500),
    dek_ru        TEXT CHECK (dek_ru IS NULL OR char_length(dek_ru) <= 500),
    body_en       TEXT CHECK (body_en IS NULL OR char_length(body_en) <= 12000),
    body_ru       TEXT CHECK (body_ru IS NULL OR char_length(body_ru) <= 12000),
    figurine_id   UUID REFERENCES figurines(id) ON DELETE SET NULL,
    href          TEXT,
    source_name   TEXT,
    source_url    TEXT,
    image_url     TEXT,
    pinned        BOOLEAN NOT NULL DEFAULT FALSE,
    published_at  TIMESTAMPTZ,
    scheduled_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS gazette_leaves_slug_idx ON gazette_leaves (slug);
CREATE INDEX IF NOT EXISTS gazette_leaves_live_idx
    ON gazette_leaves (pinned DESC, COALESCE(published_at, scheduled_at, created_at) DESC)
    WHERE status IN ('published', 'scheduled');
CREATE INDEX IF NOT EXISTS gazette_leaves_figurine_idx
    ON gazette_leaves (figurine_id)
    WHERE figurine_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS gazette_feeds (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title           TEXT NOT NULL,
    url             TEXT NOT NULL,
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    last_fetched_at TIMESTAMPTZ,
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS gazette_feeds_url_idx ON gazette_feeds (url);

CREATE TABLE IF NOT EXISTS gazette_cuttings (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    feed_id       UUID NOT NULL REFERENCES gazette_feeds(id) ON DELETE CASCADE,
    guid          TEXT NOT NULL,
    title         TEXT NOT NULL,
    url           TEXT NOT NULL,
    summary       TEXT,
    published_at  TIMESTAMPTZ,
    dismissed     BOOLEAN NOT NULL DEFAULT FALSE,
    pinned        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS gazette_cuttings_guid_idx ON gazette_cuttings (feed_id, guid);
CREATE INDEX IF NOT EXISTS gazette_cuttings_desk_idx
    ON gazette_cuttings (pinned DESC, published_at DESC NULLS LAST, created_at DESC)
    WHERE NOT dismissed;

-- Quiet visual-art desks. The keeper can disable, add, or replace these.
INSERT INTO gazette_feeds (title, url, enabled) VALUES
    ('Colossal',      'https://www.thisiscolossal.com/feed/', true),
    ('Hyperallergic', 'https://hyperallergic.com/feed/',      true),
    ('Designboom',    'https://www.designboom.com/feed/',     true)
ON CONFLICT (url) DO NOTHING;
