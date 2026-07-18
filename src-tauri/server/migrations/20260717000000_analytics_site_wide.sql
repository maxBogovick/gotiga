-- Site-wide page tracking: figurine_id becomes optional so home/archive/author/
-- workshop/commission page views can share the existing analytics pipeline
-- (batching, daily visitor hash, DNT/bot filtering) instead of a second one.
--
-- Every existing per-figurine query is already safe against NULL figurine_id
-- rows: the aggregate rollups INNER JOIN figurines (a NULL never matches), and
-- every other query filters WHERE figurine_id = <a specific uuid> (also never
-- matches NULL). Verified by reading every raw-event query in db/mod.rs.
ALTER TABLE figurine_analytics_events ALTER COLUMN figurine_id DROP NOT NULL;

ALTER TABLE figurine_analytics_events DROP CONSTRAINT figurine_analytics_event_type_chk;
ALTER TABLE figurine_analytics_events ADD CONSTRAINT figurine_analytics_event_type_chk CHECK (
    event_type IN ('figurine_view', 'figurine_engaged', 'figurine_cta_click', 'page_view')
);

-- Visitor language (RU/EN, from the i18n store) as a dimension.
ALTER TABLE figurine_analytics_events ADD COLUMN lang TEXT;

-- Which on-site block a figurine-card click came from (e.g. "home_afisha",
-- "home_featured", "archive"), separate from utm_source (external) so the two
-- don't collide when both happen to be present.
ALTER TABLE figurine_analytics_events ADD COLUMN internal_source TEXT;

CREATE INDEX IF NOT EXISTS idx_fig_analytics_events_page_view
    ON figurine_analytics_events(event_date DESC, path)
    WHERE figurine_id IS NULL;

-- Daily rollup for the generic (non-figurine) pages, mirroring
-- figurine_analytics_daily's shape but keyed by a coarse path_group instead of
-- a figurine. Permanent (not retention-pruned), same as figurine_analytics_daily.
CREATE TABLE IF NOT EXISTS site_page_views_daily (
    day DATE NOT NULL,
    path_group TEXT NOT NULL,
    views INTEGER NOT NULL DEFAULT 0,
    unique_visitors INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (day, path_group)
);

CREATE INDEX IF NOT EXISTS idx_site_page_views_daily_day ON site_page_views_daily(day DESC);
