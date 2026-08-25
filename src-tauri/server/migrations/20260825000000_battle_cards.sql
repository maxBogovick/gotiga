-- Скромные эпические битвы — the shelf of cards.
--
-- A card is a work of the house seen from another side: one card, one figurine.
-- Two things the schema keeps deliberately apart, because every mature card game
-- keeps them apart and merging them cannot be undone once people own copies:
--
--   `tier`  — the card's rank. Set by the keeper, drives the frame and the price.
--   `level` — the state of *your* copy (battle_owned_cards). Raised with dust.
--
-- The wallet is an append-only book: a row per credit and per debit, balance is
-- SUM(amount), never a stored column. Two reasons. A double-clicked "take" is the
-- classic double spend, and `idem_key` + ON CONFLICT makes the second click a
-- no-op instead of a second purchase. And the book is source-agnostic: if dust is
-- ever bought rather than earned, that is one more kind of credit row, not a new
-- design — which is the whole reason this shape was chosen now.
--
-- Only `battle_cards` is read by code at this stage (the shelf and the keeper's
-- desk). The other two tables are laid now so the first purchase is a handler,
-- not a migration over live ownership.

CREATE TABLE IF NOT EXISTS battle_cards (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug         TEXT NOT NULL,
    figurine_id  UUID REFERENCES figurines(id) ON DELETE SET NULL,
    status       TEXT NOT NULL CHECK (status IN ('draft', 'published', 'retired'))
                     DEFAULT 'draft',
    tier         SMALLINT NOT NULL CHECK (tier BETWEEN 1 AND 5) DEFAULT 1,
    title_en     TEXT NOT NULL CHECK (char_length(title_en) BETWEEN 1 AND 80),
    title_ru     TEXT NOT NULL CHECK (char_length(title_ru) BETWEEN 1 AND 80),
    effect_en    TEXT CHECK (effect_en IS NULL OR char_length(effect_en) <= 160),
    effect_ru    TEXT CHECK (effect_ru IS NULL OR char_length(effect_ru) <= 160),
    lore_en      TEXT CHECK (lore_en IS NULL OR char_length(lore_en) <= 400),
    lore_ru      TEXT CHECK (lore_ru IS NULL OR char_length(lore_ru) <= 400),
    -- The two corners of the sketch: cost top-left, power bottom-right.
    cost         SMALLINT NOT NULL CHECK (cost BETWEEN 0 AND 20) DEFAULT 1,
    power        SMALLINT NOT NULL CHECK (power BETWEEN 0 AND 99) DEFAULT 1,
    -- NULL means "not to be had for this coin at all", which is not the same as
    -- free. A card nobody can take in either coin is refused by the CHECK below.
    price_dust   INTEGER CHECK (price_dust IS NULL OR price_dust >= 0),
    price_feed   INTEGER CHECK (price_feed IS NULL OR price_feed >= 0),
    -- NULL art_url → the card wears the work's own photograph.
    art_url      TEXT,
    -- {"x":0.5,"y":0.42,"zoom":1.2} — how the photograph sits inside the frame,
    -- the same normalised shape the figurine keyhole focus already uses.
    art_focal    TEXT,
    shelf_order  INTEGER,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT battle_cards_has_price
        CHECK (price_dust IS NOT NULL OR price_feed IS NOT NULL)
);

CREATE UNIQUE INDEX IF NOT EXISTS battle_cards_slug_idx ON battle_cards (slug);

-- One card per work. Lifting this later is one dropped index; discovering
-- duplicate cards for one figurine after people own them is not.
CREATE UNIQUE INDEX IF NOT EXISTS battle_cards_figurine_idx
    ON battle_cards (figurine_id)
    WHERE figurine_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS battle_cards_shelf_idx
    ON battle_cards (shelf_order NULLS LAST, tier DESC, created_at DESC)
    WHERE status = 'published';

-- The wallet book. Only ever appended to; a mistake is corrected by a row with
-- the opposite sign, never by editing the row that was wrong.
CREATE TABLE IF NOT EXISTS battle_wallet_entries (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    currency   TEXT NOT NULL CHECK (currency IN ('dust', 'feed')),
    -- Signed: credits are positive, a purchase writes a negative row.
    amount     INTEGER NOT NULL CHECK (amount <> 0),
    reason     TEXT NOT NULL,
    ref_id     UUID,
    note       TEXT CHECK (note IS NULL OR char_length(note) <= 400),
    -- What makes a repeated request harmless. Also what stops the same act of
    -- attention (one like on one work) from being paid for twice.
    idem_key   TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS battle_wallet_idem_idx
    ON battle_wallet_entries (user_id, idem_key);
CREATE INDEX IF NOT EXISTS battle_wallet_user_idx
    ON battle_wallet_entries (user_id, currency);

CREATE TABLE IF NOT EXISTS battle_owned_cards (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    card_id     UUID NOT NULL REFERENCES battle_cards(id) ON DELETE CASCADE,
    level       SMALLINT NOT NULL CHECK (level BETWEEN 1 AND 5) DEFAULT 1,
    acquired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- NULL while the card still wears the "new" mark on the shelf.
    seen_at     TIMESTAMPTZ,
    CONSTRAINT battle_owned_unique UNIQUE (user_id, card_id)
);

CREATE INDEX IF NOT EXISTS battle_owned_user_idx ON battle_owned_cards (user_id);
