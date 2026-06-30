-- "First look": a timed early-release window for a work. While
-- now < first_look_until the piece is held back from the public archive
-- listing and shown only on the book-holders' "first look" shelf; once the hour
-- passes it becomes an ordinary public work everywhere. NULL → no window (the
-- default: a normal public work, today's behaviour for every existing row).
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS first_look_until TIMESTAMPTZ;

-- Only a handful of works are ever in-window; a partial index keeps both the
-- "hide from archive" filter and the "first look shelf" lookup cheap.
CREATE INDEX IF NOT EXISTS idx_figurines_first_look
    ON figurines (first_look_until)
    WHERE first_look_until IS NOT NULL;
