-- A dress is no longer three fields. It carries the whole design — paper,
-- the six slice pictures, the assembly of sixteen copies, and the keeper's
-- own flourishes — the same object a preset wears onto a rank. 2000
-- characters held `{frameImage,frameMode,aspect}` and cuts a carved frame
-- in half, which is how saving a card that had taken a dress out of the
-- drawer came back as 23514. 48000 holds that whole object with room for
-- fields not yet named; still a bound, so the column cannot become a
-- second settings blob.
--
-- A race's five level slots are five of the same object. They never had a
-- CHECK at all; they get five times the room, matching FRAME_OVERRIDE_MAX
-- in battles.rs.

ALTER TABLE battle_cards DROP CONSTRAINT IF EXISTS battle_cards_frame_override_check;
ALTER TABLE battle_cards ADD CONSTRAINT battle_cards_frame_override_check
    CHECK (frame_override IS NULL OR char_length(frame_override) <= 48000);

ALTER TABLE battle_races DROP CONSTRAINT IF EXISTS battle_races_level_frames_check;
ALTER TABLE battle_races ADD CONSTRAINT battle_races_level_frames_check
    CHECK (level_frames IS NULL OR char_length(level_frames) <= 240000);
