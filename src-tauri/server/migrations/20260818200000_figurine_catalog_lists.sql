-- Per-figurine Features / Perfect for checklists on the Specimen catalog leaf.
-- NULL → every built-in line is selected, no custom lines.
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS catalog_lists TEXT;
