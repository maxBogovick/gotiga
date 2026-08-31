-- The keeper's store of frame parts, and the sheets they were cut from.
--
-- A generator draws a whole frame's worth of parts on one sheet — corners,
-- bars, ornaments, each captioned with a letter. `src/sheet.rs` finds the
-- separate objects and cuts each into its own picture. What is stored here is
-- both halves of that: the sheet as it arrived, and every part taken off it.
--
-- The SHEET is kept whole, in the bytes it was uploaded in, and not merely as
-- provenance. The cut has settings, the settings sometimes need a nudge, and
-- re-cutting must not mean asking the keeper for the file again. Lossless in,
-- lossless kept: a JPEG rendition of a sheet with an alpha channel would throw
-- away the very thing the cut reads.

CREATE TABLE IF NOT EXISTS battle_asset_sheets (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    source_url  TEXT NOT NULL,
    width       INTEGER NOT NULL,
    height      INTEGER NOT NULL,
    -- The settings that last cut this sheet, as JSON (see `SliceSettings`).
    -- Kept so that reopening a sheet starts where the keeper left off, and so
    -- the numbering a proposal showed still means the same parts.
    settings    TEXT,
    sort_order  INTEGER,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- A part. `sheet_id` is where it came from, not who owns it: clearing a sheet
-- away must never take down a part that a frame is already wearing. Same
-- reasoning, same ON DELETE SET NULL, as a card and its race. A part with no
-- sheet is not an orphan — it is either loose from the start (uploaded on its
-- own) or outlived its sheet, and both are ordinary.
--
-- `role` is a CHECK and not a second dictionary. A role is a word the keeper
-- filters by, not a thing the game looks up: five fixed values that mirror the
-- slots of a `sliced` frame, plus `art` for a picture and `other` for the rest.
CREATE TABLE IF NOT EXISTS battle_assets (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sheet_id    UUID REFERENCES battle_asset_sheets(id) ON DELETE SET NULL,
    name        TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    role        TEXT NOT NULL
                CHECK (role IN ('corner', 'sideH', 'sideV', 'accent', 'art', 'other')),
    url         TEXT NOT NULL,
    width       INTEGER NOT NULL,
    height      INTEGER NOT NULL,
    sort_order  INTEGER,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- The two ways the store is ever read: everything off one sheet, in the
-- keeper's own order, and everything of one role across all sheets.
CREATE INDEX IF NOT EXISTS battle_assets_sheet_idx
    ON battle_assets (sheet_id, sort_order NULLS LAST);
CREATE INDEX IF NOT EXISTS battle_assets_role_idx ON battle_assets (role);
