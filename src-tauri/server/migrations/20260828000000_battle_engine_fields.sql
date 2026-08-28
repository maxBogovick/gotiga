-- The body of a card, as the engine reads it.
--
-- Until now a card carried what a *reader* needs: a name, a tier, a paragraph of
-- prose. The rules cannot be run on a paragraph. These columns are the same card
-- said a second time, in numbers — and the prose stays exactly where it was,
-- because it is what gets printed on the parchment.
--
-- Why two descriptions rather than one. Parsing the prose is natural-language
-- parsing; it breaks on every other card. Showing the JSON is a card nobody can
-- read. So both, written by the same hand in the same form, with the preview
-- showing what the numbers actually produce.

-- What this is for the engine. `type_en`/`type_ru` stay as they are — free text
-- for the header band, which no rule may ever read.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS kind TEXT NOT NULL DEFAULT 'unit';
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_kind_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_kind_check
    CHECK (kind IN ('unit', 'spell', 'relic'));

-- Two defences, one per channel that has one. Flat subtraction, not percentage:
-- these numbers live between 0 and 9, and a percentage of nine is a fraction
-- nobody can print on a card or work out in their head.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS armor SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS ward SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_armor_check;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_ward_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_armor_check CHECK (armor BETWEEN 0 AND 20);
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_ward_check CHECK (ward BETWEEN 0 AND 20);

-- The ordinary blow: `power` (already here) says how hard, these two say how far
-- and which defence answers it.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS attack_channel TEXT NOT NULL DEFAULT 'physical';
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_attack_channel_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_attack_channel_check
    CHECK (attack_channel IN ('physical', 'magic', 'pure', 'none'));

-- Range and step are both counted in king's moves on a field three wide and six
-- deep, so 5 means "the whole field, corner to corner" and needs no other name.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS reach SMALLINT NOT NULL DEFAULT 1;
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS step SMALLINT NOT NULL DEFAULT 1;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_reach_check;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_step_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_reach_check CHECK (reach BETWEEN 0 AND 5);
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_step_check CHECK (step BETWEEN 0 AND 3);

-- Who acts first. Three is the middle; a five moves before everything.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS speed SMALLINT NOT NULL DEFAULT 3;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_speed_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_speed_check CHECK (speed BETWEEN 1 AND 5);

-- Mending, as a property of the body.
--
-- Deliberately interim. Once abilities exist (stage 2) a mender will carry a
-- `heal` verb in `abilities` like everything else, and this column will be
-- dropped. It is here because the engine reads it today and a keeper cannot
-- write a healer without it — not because a healer is a different kind of body.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS mend SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_mend_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_mend_check CHECK (mend BETWEEN 0 AND 20);

-- The executable half of a card's abilities, beside the prose in `traits`.
-- A JSON array of {id, verb, channel, amount, shape, range, duration, trigger,
-- manaCost, cooldown}. Capped in bytes here and in count by the service, so a
-- broken client cannot post a megabyte of rules onto one card.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS abilities TEXT;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_abilities_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_abilities_check
    CHECK (abilities IS NULL OR char_length(abilities) <= 8000);

-- What the balance calculator worked out when the card was last saved. A mirror,
-- never a source: recomputed on every save, and safe to throw away. Kept so the
-- desk can colour a card and sort the shelf by overload without recounting.
-- DOUBLE PRECISION rather than NUMERIC: these are a computed mirror, not money,
-- and NUMERIC would need a decimal crate in the server for nothing.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS budget_points DOUBLE PRECISION;
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS balance_index DOUBLE PRECISION;

-- Editing a card's numbers is a new version of the rules, not a silent change.
-- A match records the version it was played under, so a rebalance does not
-- rewrite the history of matches already played.
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS rules_version INTEGER NOT NULL DEFAULT 1;

-- The dictionary of keywords: Шипы, Немота, Покров, Яд.
--
-- A table for the same three reasons a race is one: shared by many cards,
-- renamed in one place, and read by a rule. And one reason of its own —
-- `point_value` is the exchange rate from the balance formula, so rebalancing
-- the whole game becomes an edit in a dictionary rather than a deployment.
CREATE TABLE IF NOT EXISTS battle_keywords (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug        TEXT NOT NULL,
    name_en     TEXT NOT NULL CHECK (char_length(name_en) BETWEEN 1 AND 60),
    name_ru     TEXT NOT NULL CHECK (char_length(name_ru) BETWEEN 1 AND 60),
    -- The canonical wording of the rule, one per game. A card says "Шипы 3";
    -- what Шипы means is said here, once.
    rules_en    TEXT CHECK (rules_en IS NULL OR char_length(rules_en) <= 300),
    rules_ru    TEXT CHECK (rules_ru IS NULL OR char_length(rules_ru) <= 300),
    icon_url    TEXT,
    -- Points per unit, from the exchange table. NULL means "not priced yet".
    point_value DOUBLE PRECISION,
    sort_order  INTEGER,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS battle_keywords_slug_idx ON battle_keywords (slug);
