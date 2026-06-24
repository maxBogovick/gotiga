-- Per-image "keyhole" reveal: focal point + radius for the card teaser.
-- All NULL keeps the renderer default (centre, default radius), so existing
-- catalogue images keep showing as before until an editor places a focus.
ALTER TABLE images ADD COLUMN IF NOT EXISTS focal_x REAL;
ALTER TABLE images ADD COLUMN IF NOT EXISTS focal_y REAL;
ALTER TABLE images ADD COLUMN IF NOT EXISTS reveal_radius REAL;
