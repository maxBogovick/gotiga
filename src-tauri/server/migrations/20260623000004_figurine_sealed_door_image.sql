-- Optional door asset for a gated work. While the showing window is closed the
-- card/detail shows a sealed door; when this is set that image is the door,
-- otherwise the door is drawn procedurally (carved oak) on the client. NULL →
-- procedural door, so existing works are unaffected.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS sealed_door_image TEXT;
