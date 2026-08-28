-- A race's own dress per level of an owned copy — five `FrameOverride`-shaped
-- slots (see battles.rs), stored as one JSON array, the same idiom as a
-- card's own `frame_override`. NULL when the race uses tier frames as-is.
ALTER TABLE battle_races ADD COLUMN IF NOT EXISTS level_frames TEXT;
