-- Work texts in the other languages.
--
-- A work (`figurines`) is written in English, in its own columns. Its other
-- languages live here, one row per work and language: ten columns per language
-- on `figurines` would be fifty columns, and most of them empty. A field left
-- NULL falls back on the client to the English column of the work.
--
-- Only the human-readable texts are here: the ones the public page prints.
-- Search-only and admin-only texts (`visual_caption`, `pinterest_description`)
-- stay English.
CREATE TABLE IF NOT EXISTS figurine_translations (
    figurine_id       UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    lang              TEXT NOT NULL CHECK (lang IN ('ru', 'uk', 'es', 'de', 'it')),
    name              TEXT CHECK (name IS NULL OR char_length(name) <= 200),
    short_text        TEXT CHECK (short_text IS NULL OR char_length(short_text) <= 2000),
    full_description  TEXT CHECK (full_description IS NULL OR char_length(full_description) <= 20000),
    material          TEXT CHECK (material IS NULL OR char_length(material) <= 500),
    technique         TEXT CHECK (technique IS NULL OR char_length(technique) <= 500),
    dimensions        TEXT CHECK (dimensions IS NULL OR char_length(dimensions) <= 200),
    series            TEXT CHECK (series IS NULL OR char_length(series) <= 200),
    edition           TEXT CHECK (edition IS NULL OR char_length(edition) <= 500),
    created_period    TEXT CHECK (created_period IS NULL OR char_length(created_period) <= 500),
    care_instructions TEXT CHECK (care_instructions IS NULL OR char_length(care_instructions) <= 5000),
    provenance_note   TEXT CHECK (provenance_note IS NULL OR char_length(provenance_note) <= 5000),
    authenticity_note TEXT CHECK (authenticity_note IS NULL OR char_length(authenticity_note) <= 5000),
    included_items    TEXT CHECK (included_items IS NULL OR char_length(included_items) <= 5000),
    secret_text       TEXT CHECK (secret_text IS NULL OR char_length(secret_text) <= 5000),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (figurine_id, lang)
);

-- Photo descriptions (`images.alt_text`) in the other languages, one row per
-- photo and language.
--
-- No foreign key to `images`: saving a work deletes its photos and writes them
-- again under the same ids, and a cascade would wipe every translation on every
-- save. The row hangs on the work instead (gone with it), and the save removes
-- rows whose photo is no longer among the work's photos.
CREATE TABLE IF NOT EXISTS figurine_image_translations (
    image_id    UUID NOT NULL,
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    lang        TEXT NOT NULL CHECK (lang IN ('ru', 'uk', 'es', 'de', 'it')),
    alt_text    TEXT NOT NULL CHECK (char_length(alt_text) BETWEEN 1 AND 1000),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (image_id, lang)
);
CREATE INDEX IF NOT EXISTS idx_figurine_image_translations_figurine
    ON figurine_image_translations (figurine_id);
