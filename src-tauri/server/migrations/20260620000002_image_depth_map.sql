-- Monocular depth map for the "living daguerreotype" 2.5D parallax.
-- Grayscale image variant generated offline (Depth-Anything-class), stored as a
-- relative media path like 'images/depth/{uuid}.webp'. NULL means the frontend
-- falls back to luminance-derived depth.
ALTER TABLE images ADD COLUMN IF NOT EXISTS depth_path TEXT;
