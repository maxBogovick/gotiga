ALTER TABLE figurines
    ADD COLUMN IF NOT EXISTS passport_number TEXT,
    ADD COLUMN IF NOT EXISTS edition TEXT,
    ADD COLUMN IF NOT EXISTS created_period TEXT,
    ADD COLUMN IF NOT EXISTS care_instructions TEXT,
    ADD COLUMN IF NOT EXISTS provenance_note TEXT,
    ADD COLUMN IF NOT EXISTS authenticity_note TEXT,
    ADD COLUMN IF NOT EXISTS included_items TEXT;

