-- Optional per-image multiplier for LivingDaguerreotype.
-- NULL keeps the renderer default, so existing catalogue images do not change.
ALTER TABLE images ADD COLUMN IF NOT EXISTS parallax_intensity REAL;
