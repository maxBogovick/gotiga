-- Permanent (non-retention-bound) country aggregates, mirroring the existing
-- figurine_analytics_sources_daily / site_page_views_daily pattern: raw
-- figurine_analytics_events rows are pruned after analytics::RETENTION_DAYS,
-- so without a permanent rollup, country breakdowns (and any map built on
-- top of them) would silently go empty for date ranges older than 30 days.

-- Per-figurine daily views by country. Only figurine_view events land here
-- (mirrors figurine_analytics_sources_daily), so the country dimension is
-- restricted to figurine detail pages, not the site's other pages.
CREATE TABLE IF NOT EXISTS figurine_analytics_geo_daily (
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    day DATE NOT NULL,
    country_code TEXT NOT NULL, -- 'unknown' when GeoIP couldn't resolve one
    views INTEGER NOT NULL DEFAULT 0,
    unique_visitors INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (figurine_id, day, country_code)
);

CREATE INDEX IF NOT EXISTS idx_fig_analytics_geo_day
    ON figurine_analytics_geo_daily(day DESC);

CREATE INDEX IF NOT EXISTS idx_fig_analytics_geo_country
    ON figurine_analytics_geo_daily(country_code, day DESC);

-- Site-wide daily views by country, across every page (figurine detail pages
-- and the generic pages alike) — the source for the admin geography map.
CREATE TABLE IF NOT EXISTS site_geo_daily (
    day DATE NOT NULL,
    country_code TEXT NOT NULL,
    views INTEGER NOT NULL DEFAULT 0,
    unique_visitors INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (day, country_code)
);

CREATE INDEX IF NOT EXISTS idx_site_geo_daily_day
    ON site_geo_daily(day DESC);
