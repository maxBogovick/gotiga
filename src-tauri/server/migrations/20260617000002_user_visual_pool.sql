-- Per-user visual-password alphabet.
-- Each user is shown a personal subset of the master icon pool (8 of ~24 per
-- category), generated at registration and persisted here so the same grid can
-- be rebuilt at login. NULL = legacy account (registered before personal pools);
-- such accounts fall back to a deterministic decoy at login and must re-register.
ALTER TABLE users ADD COLUMN visual_pool JSONB;
