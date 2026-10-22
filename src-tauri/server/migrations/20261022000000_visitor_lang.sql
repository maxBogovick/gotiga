-- The language a visitor reads the site in, so that what the server writes to
-- them (a reply in their conversation, a note in Telegram, a letter) is in that
-- language and not in Russian or English by default.
--
-- `users.lang` is the language of the person's last request that named one;
-- the record's own `lang` is the language of the page the request was sent
-- from. A record's lang wins; then the person's; then English. NULL means "not
-- known": rows written before this migration stay NULL.
ALTER TABLE users ADD COLUMN IF NOT EXISTS lang TEXT;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS lang TEXT;
ALTER TABLE figurine_bookings ADD COLUMN IF NOT EXISTS lang TEXT;
ALTER TABLE figurine_waitlist ADD COLUMN IF NOT EXISTS lang TEXT;
ALTER TABLE figurine_comments ADD COLUMN IF NOT EXISTS lang TEXT;

ALTER TABLE users DROP CONSTRAINT IF EXISTS users_lang_check;
ALTER TABLE users ADD CONSTRAINT users_lang_check
    CHECK (lang IS NULL OR lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE orders DROP CONSTRAINT IF EXISTS orders_lang_check;
ALTER TABLE orders ADD CONSTRAINT orders_lang_check
    CHECK (lang IS NULL OR lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE figurine_bookings DROP CONSTRAINT IF EXISTS figurine_bookings_lang_check;
ALTER TABLE figurine_bookings ADD CONSTRAINT figurine_bookings_lang_check
    CHECK (lang IS NULL OR lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE figurine_waitlist DROP CONSTRAINT IF EXISTS figurine_waitlist_lang_check;
ALTER TABLE figurine_waitlist ADD CONSTRAINT figurine_waitlist_lang_check
    CHECK (lang IS NULL OR lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));
ALTER TABLE figurine_comments DROP CONSTRAINT IF EXISTS figurine_comments_lang_check;
ALTER TABLE figurine_comments ADD CONSTRAINT figurine_comments_lang_check
    CHECK (lang IS NULL OR lang IN ('en', 'ru', 'uk', 'es', 'de', 'it'));

-- A language not stated used to mean Russian for the studio and for petitions;
-- now it means English, the same as everywhere else. Existing rows keep theirs.
ALTER TABLE studio_frames ALTER COLUMN lang SET DEFAULT 'en';
ALTER TABLE studio_cards ALTER COLUMN lang SET DEFAULT 'en';
ALTER TABLE studio_races ALTER COLUMN lang SET DEFAULT 'en';
ALTER TABLE studio_motions ALTER COLUMN lang SET DEFAULT 'en';
ALTER TABLE commissions ALTER COLUMN lang SET DEFAULT 'en';
