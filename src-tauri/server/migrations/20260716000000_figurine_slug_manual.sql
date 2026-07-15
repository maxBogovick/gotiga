-- Track whether a work's URL slug was hand-typed by the admin (manual) or
-- auto-generated from its name. Lets the «Work addresses» admin table show which
-- slugs are safe to regenerate. Existing rows default to auto (false); the flag
-- flips true only when an admin saves a slug that differs from the stored one.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS slug_manual BOOLEAN NOT NULL DEFAULT false;
