-- Add original_path and thumb_path to images table
ALTER TABLE images ADD COLUMN IF NOT EXISTS original_path TEXT;
ALTER TABLE images ADD COLUMN IF NOT EXISTS thumb_path TEXT;

-- Create app_resources table
CREATE TABLE IF NOT EXISTS app_resources (
    key TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Expand zone_type enum with new values
DO $$ BEGIN
    ALTER TYPE zone_type ADD VALUE IF NOT EXISTS 'curator';
EXCEPTION WHEN others THEN null; END $$;

DO $$ BEGIN
    ALTER TYPE zone_type ADD VALUE IF NOT EXISTS 'cabinet';
EXCEPTION WHEN others THEN null; END $$;

DO $$ BEGIN
    ALTER TYPE zone_type ADD VALUE IF NOT EXISTS 'portrait';
EXCEPTION WHEN others THEN null; END $$;

DO $$ BEGIN
    ALTER TYPE zone_type ADD VALUE IF NOT EXISTS 'windows';
EXCEPTION WHEN others THEN null; END $$;
