-- Add in_progress to figurine_status enum
DO $$ BEGIN
    ALTER TYPE figurine_status ADD VALUE IF NOT EXISTS 'in_progress';
EXCEPTION
    WHEN others THEN null;
END $$;

-- Add is_featured column to figurines
ALTER TABLE figurines ADD COLUMN IF NOT EXISTS is_featured BOOLEAN NOT NULL DEFAULT false;
