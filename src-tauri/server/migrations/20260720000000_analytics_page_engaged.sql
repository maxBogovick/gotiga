-- Site-wide engagement tracking for the generic (non-figurine) pages. Until now
-- home/archive/author/workshop/commission fired only a single `page_view` event
-- with no time-on-page and no scroll depth, so there was no way to tell how long
-- a visitor stayed or how far they scrolled — the exact "what makes people leave"
-- signal the admin wants.
--
-- `page_engaged` is the site-wide sibling of `figurine_engaged`: figurine_id is
-- NULL (like `page_view`), and it carries duration_ms + scroll_depth, plus a new
-- `works_seen` count (how many distinct work tiles entered the viewport on the
-- home and archive grids; NULL on pages without a grid).
ALTER TABLE figurine_analytics_events DROP CONSTRAINT figurine_analytics_event_type_chk;
ALTER TABLE figurine_analytics_events ADD CONSTRAINT figurine_analytics_event_type_chk CHECK (
    event_type IN ('figurine_view', 'figurine_engaged', 'figurine_cta_click', 'page_view', 'page_engaged')
);

-- Distinct work tiles seen during a home/archive visit. NULL for gridless pages
-- and for every non-`page_engaged` event.
ALTER TABLE figurine_analytics_events ADD COLUMN works_seen INTEGER;

-- Feeds the on-the-fly percentile medians per path_group (no aggregate table —
-- computed straight from raw events within the retention window, same as the
-- per-figurine engagement medians).
CREATE INDEX IF NOT EXISTS idx_fig_analytics_events_page_engaged
    ON figurine_analytics_events(event_date DESC, path)
    WHERE event_type = 'page_engaged';
