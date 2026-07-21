-- Platform layer for multi-tenancy.
--
-- The `platform` schema holds cross-tenant records that are NOT part of any single
-- site's content. Each tenant's content lives in its own Postgres schema
-- (`schema_name`); tenant #1 (Ritunia, the flagship) maps to the pre-existing
-- `public` schema, so introducing multi-tenancy moves no data.

CREATE SCHEMA IF NOT EXISTS platform;

CREATE TABLE IF NOT EXISTS platform.tenants (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Subdomain label, e.g. "ritunia" in ritunia.example.com.
    slug          TEXT NOT NULL UNIQUE,
    -- Postgres schema holding this tenant's content. Quoted into `SET search_path`,
    -- so it is constrained to a safe identifier shape.
    schema_name   TEXT NOT NULL UNIQUE CHECK (schema_name ~ '^[a-z_][a-z0-9_]*$'),
    -- Optional apex/custom domain that also maps to this tenant.
    custom_domain TEXT UNIQUE,
    -- active | suspended
    status        TEXT NOT NULL DEFAULT 'active',
    -- The flagship tenant; the request resolver falls back to it for unknown hosts.
    is_primary    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- At most one primary tenant.
CREATE UNIQUE INDEX IF NOT EXISTS tenants_single_primary
    ON platform.tenants (is_primary) WHERE is_primary;

-- Seed the flagship. Ritunia's content already lives in `public`, so it simply
-- becomes tenant #1 with schema_name='public' — no data movement.
INSERT INTO platform.tenants (slug, schema_name, is_primary)
VALUES ('ritunia', 'public', TRUE)
ON CONFLICT (slug) DO NOTHING;
