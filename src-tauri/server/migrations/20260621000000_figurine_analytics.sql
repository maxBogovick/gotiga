CREATE TABLE IF NOT EXISTS figurine_analytics_events (
    id BIGSERIAL PRIMARY KEY,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_date DATE NOT NULL,
    event_type TEXT NOT NULL,
    -- Deliberately no FK here: the public endpoint must never let one stale or
    -- fake figurine UUID poison an entire async batch insert. Aggregates below
    -- join against `figurines`, so invalid raw events are ignored there.
    figurine_id UUID NOT NULL,
    visitor_hash TEXT,
    page_view_id UUID,
    path TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'unknown',
    referrer_host TEXT,
    utm_source TEXT,
    utm_medium TEXT,
    utm_campaign TEXT,
    device_class TEXT,
    browser_family TEXT,
    country_code TEXT,
    duration_ms INTEGER,
    scroll_depth INTEGER,
    cta_type TEXT,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT figurine_analytics_event_type_chk CHECK (
        event_type IN ('figurine_view', 'figurine_engaged', 'figurine_cta_click')
    ),
    CONSTRAINT figurine_analytics_scroll_depth_chk CHECK (
        scroll_depth IS NULL OR (scroll_depth >= 0 AND scroll_depth <= 100)
    ),
    CONSTRAINT figurine_analytics_duration_chk CHECK (
        duration_ms IS NULL OR duration_ms >= 0
    )
);

CREATE INDEX IF NOT EXISTS idx_fig_analytics_events_fig_date
    ON figurine_analytics_events(figurine_id, event_date DESC);

CREATE INDEX IF NOT EXISTS idx_fig_analytics_events_date_type
    ON figurine_analytics_events(event_date DESC, event_type);

CREATE INDEX IF NOT EXISTS idx_fig_analytics_events_visitor_unique
    ON figurine_analytics_events(figurine_id, event_date, visitor_hash)
    WHERE visitor_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_fig_analytics_events_source_date
    ON figurine_analytics_events(source, event_date DESC);

CREATE TABLE IF NOT EXISTS figurine_analytics_daily (
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    day DATE NOT NULL,
    views INTEGER NOT NULL DEFAULT 0,
    unique_visitors INTEGER NOT NULL DEFAULT 0,
    engaged_views INTEGER NOT NULL DEFAULT 0,
    cta_clicks INTEGER NOT NULL DEFAULT 0,
    order_starts INTEGER NOT NULL DEFAULT 0,
    reserve_starts INTEGER NOT NULL DEFAULT 0,
    booking_starts INTEGER NOT NULL DEFAULT 0,
    waitlist_starts INTEGER NOT NULL DEFAULT 0,
    commission_starts INTEGER NOT NULL DEFAULT 0,
    orders_submitted INTEGER NOT NULL DEFAULT 0,
    bookings_submitted INTEGER NOT NULL DEFAULT 0,
    waitlist_submitted INTEGER NOT NULL DEFAULT 0,
    commissions_submitted INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (figurine_id, day)
);

CREATE INDEX IF NOT EXISTS idx_fig_analytics_daily_day
    ON figurine_analytics_daily(day DESC);

CREATE TABLE IF NOT EXISTS figurine_analytics_sources_daily (
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    day DATE NOT NULL,
    source TEXT NOT NULL,
    views INTEGER NOT NULL DEFAULT 0,
    unique_visitors INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (figurine_id, day, source)
);

CREATE INDEX IF NOT EXISTS idx_fig_analytics_sources_day
    ON figurine_analytics_sources_daily(day DESC);
