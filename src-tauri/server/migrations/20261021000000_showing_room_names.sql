-- Names of showing rooms in the other languages.
--
-- `name` stays the English base (it is also the sort key). The other languages
-- are optional: an empty one falls back on the client to English, the same as
-- every other text on the site.
ALTER TABLE showing_rooms
    ADD COLUMN IF NOT EXISTS name_ru TEXT,
    ADD COLUMN IF NOT EXISTS name_uk TEXT,
    ADD COLUMN IF NOT EXISTS name_es TEXT,
    ADD COLUMN IF NOT EXISTS name_de TEXT,
    ADD COLUMN IF NOT EXISTS name_it TEXT;
ALTER TABLE showing_rooms DROP CONSTRAINT IF EXISTS showing_rooms_name_langs_check;
ALTER TABLE showing_rooms ADD CONSTRAINT showing_rooms_name_langs_check CHECK (
    (name_ru IS NULL OR char_length(name_ru) <= 200)
    AND (name_uk IS NULL OR char_length(name_uk) <= 200)
    AND (name_es IS NULL OR char_length(name_es) <= 200)
    AND (name_de IS NULL OR char_length(name_de) <= 200)
    AND (name_it IS NULL OR char_length(name_it) <= 200)
);
