-- Checkpoint 2: replace the plain binary mark with 3 private "tones" so the
-- admin ranking carries a weighted signal instead of a flat count. Still never
-- exposed publicly — see comment on the figurine_marks table itself.
ALTER TABLE figurine_marks
    ADD COLUMN tone TEXT NOT NULL DEFAULT 'touched';
