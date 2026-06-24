-- Per-image "keyhole" darkness override (0..1). NULL → use the global
-- keyhole darkness (theme setting), which itself falls back to the renderer
-- default — so existing images are unaffected until an editor sets one.
ALTER TABLE images ADD COLUMN IF NOT EXISTS darkness REAL;
