-- Server-side wishlist so a logged-in user's saved figurines survive a device
-- change or a cleared browser. Anonymous visitors keep using localStorage only.
ALTER TABLE users ADD COLUMN IF NOT EXISTS wishlist TEXT[] NOT NULL DEFAULT '{}';
