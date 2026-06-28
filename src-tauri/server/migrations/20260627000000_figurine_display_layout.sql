-- Display layout selector for the figurine detail page.
-- NULL → 'specimen' (default). Other values: showcase | codex | diptych | broadside.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS display_layout TEXT;
