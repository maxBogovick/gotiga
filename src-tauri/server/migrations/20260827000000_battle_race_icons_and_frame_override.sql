-- Two small exceptions to "design lives in the dictionary, not the card".
--
-- A race icon is shared: uploaded once per race, shown on every card of
-- that race, exactly like a frame is uploaded once per tier.
ALTER TABLE battle_races ADD COLUMN IF NOT EXISTS icon_url TEXT;

-- A frame override is the one deliberate exception: the keeper can dress a
-- single card in its own picture without touching the shared tier frame
-- every other card of that rank still wears. {"frameImage","frameMode",
-- "aspect"} JSON, all optional, same shape and spirit as art_focal. NULL
-- (the common case) means "wear the tier's frame, unmodified".
ALTER TABLE battle_cards ADD COLUMN IF NOT EXISTS frame_override TEXT;
ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_frame_override_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_frame_override_check
    CHECK (frame_override IS NULL OR char_length(frame_override) <= 2000);
