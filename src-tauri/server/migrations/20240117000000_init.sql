-- Enums
DO $$ BEGIN
    CREATE TYPE figurine_status AS ENUM ('available', 'sold', 'reserved');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE image_type AS ENUM ('face', 'detail', 'full');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE step_type AS ENUM ('sketch', 'prototype', 'modeling', 'painting', 'finish');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE zone_type AS ENUM ('showcase', 'desk', 'shelf', 'note');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE text_category AS ENUM ('author', 'workshop');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Tables
CREATE TABLE IF NOT EXISTS figurines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    short_text TEXT,
    full_description TEXT,
    dimensions TEXT,
    material TEXT,
    technique TEXT,
    year INTEGER,
    ambience_path TEXT,
    video_url TEXT,
    secret_text TEXT,
    is_visible BOOLEAN NOT NULL DEFAULT true,
    status figurine_status NOT NULL DEFAULT 'available',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS images (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    image_type image_type NOT NULL,
    file_path TEXT NOT NULL,
    alt_text TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS process_steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    step_type step_type NOT NULL,
    description TEXT,
    image_path TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS texts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category text_category NOT NULL,
    content TEXT NOT NULL,
    caption TEXT,
    image_path TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cabinet_zones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    zone_type zone_type NOT NULL,
    x_percent DOUBLE PRECISION NOT NULL,
    y_percent DOUBLE PRECISION NOT NULL,
    width_percent DOUBLE PRECISION NOT NULL,
    height_percent DOUBLE PRECISION NOT NULL,
    target_route TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);
