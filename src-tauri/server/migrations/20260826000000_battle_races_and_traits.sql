-- The card, divided into four bands: a header (race and type), the work's
-- photograph, the card's properties, and a footer.
--
-- Two new kinds of thing, and they are stored differently on purpose.
--
-- A RACE is shared: many cards belong to one, the keeper wants to rename it in
-- one place, and a rule of the game will one day read it. That is a table.
--
-- A TRAIT ("Вихрь Души: каждое третье заклинание…") belongs to exactly one card,
-- is always read with it, is never searched across cards, and its order matters.
-- That is a JSON column — the same choice already made for `art_focal` and
-- `display_config`. A table here would add a join and an ordering column to buy
-- nothing.

CREATE TABLE IF NOT EXISTS battle_races (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug        TEXT NOT NULL,
    name_en     TEXT NOT NULL CHECK (char_length(name_en) BETWEEN 1 AND 60),
    name_ru     TEXT NOT NULL CHECK (char_length(name_ru) BETWEEN 1 AND 60),
    note_en     TEXT CHECK (note_en IS NULL OR char_length(note_en) <= 200),
    note_ru     TEXT CHECK (note_ru IS NULL OR char_length(note_ru) <= 200),
    sort_order  INTEGER,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS battle_races_slug_idx ON battle_races (slug);

-- A race removed from the dictionary leaves its cards standing, without one.
-- Deleting a race must never delete the cards that wore it.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS race_id UUID
    REFERENCES battle_races(id) ON DELETE SET NULL;

-- The other half of the header: what kind of card this is ("Существо",
-- "Заклинание"). Free text rather than a second dictionary — a type is a word,
-- not a thing the game will look up.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS type_en TEXT;
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS type_ru TEXT;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_type_en_check;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_type_ru_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_type_en_check
    CHECK (type_en IS NULL OR char_length(type_en) <= 40);
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_type_ru_check
    CHECK (type_ru IS NULL OR char_length(type_ru) <= 40);

-- `cost` and `power` already exist; power is the Strength of the properties
-- band. These two join them.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS health SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS mana SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_health_check;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_mana_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_health_check
    CHECK (health BETWEEN 0 AND 99);
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_mana_check
    CHECK (mana BETWEEN 0 AND 99);

-- JSON array: [{"nameEn","nameRu","textEn","textRu"}, …]. Order is the order
-- written. Capped in bytes here and in count by the service, so a broken client
-- cannot post a megabyte of properties onto one card.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS traits TEXT;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_traits_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_traits_check
    CHECK (traits IS NULL OR char_length(traits) <= 4000);
