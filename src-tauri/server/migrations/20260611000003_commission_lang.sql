-- Remember the language the petition was written in, so system messages the
-- master triggers (accepted / in progress / completed / declined) reach the
-- petitioner in the same language they used.
ALTER TABLE commissions ADD COLUMN lang TEXT NOT NULL DEFAULT 'ru';
