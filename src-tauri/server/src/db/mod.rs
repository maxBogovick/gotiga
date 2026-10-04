use crate::error::{AppError, Result};
use crate::models::*;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

mod prepared;
pub use prepared::{clear_stale_prepared_statements, note_stale_cached_plan};

/// Parse an optional `YYYY-MM-DD` deadline. Empty/absent → None; a present but
/// unparseable value is a client error (400) rather than a silent drop.
fn parse_optional_deadline(raw: Option<&str>) -> Result<Option<chrono::NaiveDate>> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| {
                AppError::BadRequest("Invalid deadline date (expected YYYY-MM-DD)".to_string())
            }),
    }
}

/// Which room of the house a site-wide event happened in, as SQL.
///
/// One string, embedded in every query that groups by room: it was written
/// twice — once in the daily rollup, once in the engagement query — and the
/// two are required to line up row for row, which two copies of a growing
/// CASE eventually do not.
///
/// Matched on the path with the query string cut off (`split_part`), because
/// a room is a room whether or not the visitor is filtering inside it. Order
/// matters where one prefix contains another: a gazette watch link is a
/// letter, not the gazette; a passport is not the archive.
///
/// `work` is here for the record only — the figurine page reports itself as
/// `figurine_view` and sends no `page_view` (see `OWN_TRACKING` in
/// `analytics.ts`), so the branch catches nothing but old rows.
const PATH_GROUP_SQL: &str = r#"
                CASE
                    WHEN split_part(path, '?', 1) = '/' THEN 'home'
                    WHEN split_part(path, '?', 1) = '/figurines' THEN 'archive'
                    WHEN split_part(path, '?', 1) LIKE '/figurines/%/passport' THEN 'passport'
                    WHEN split_part(path, '?', 1) LIKE '/figurines/%' THEN 'work'
                    WHEN split_part(path, '?', 1) LIKE '/author%' THEN 'author'
                    WHEN split_part(path, '?', 1) LIKE '/workshop%' THEN 'workshop'
                    WHEN split_part(path, '?', 1) LIKE '/commission%' THEN 'commission'
                    WHEN split_part(path, '?', 1) LIKE '/cellar%' THEN 'cellar'
                    WHEN split_part(path, '?', 1) LIKE '/tales%' THEN 'tale'
                    WHEN split_part(path, '?', 1) LIKE '/gazette/watch%' THEN 'letter'
                    WHEN split_part(path, '?', 1) LIKE '/gazette%' THEN 'gazette'
                    WHEN split_part(path, '?', 1) LIKE '/battles%' THEN 'battles'
                    WHEN split_part(path, '?', 1) LIKE '/studio%' THEN 'studio'
                    WHEN split_part(path, '?', 1) LIKE '/impressions%' THEN 'impressions'
                    WHEN split_part(path, '?', 1) LIKE '/upcoming%' THEN 'upcoming'
                    WHEN split_part(path, '?', 1) LIKE '/acquire%' THEN 'acquire'
                    WHEN split_part(path, '?', 1) LIKE '/hall/%' THEN 'hall'
                    WHEN split_part(path, '?', 1) LIKE '/privacy%'
                      OR split_part(path, '?', 1) LIKE '/rights%' THEN 'legal'
                    WHEN split_part(path, '?', 1) LIKE '/profile%'
                      OR split_part(path, '?', 1) LIKE '/bookings%' THEN 'account'
                    WHEN split_part(path, '?', 1) LIKE '/login%'
                      OR split_part(path, '?', 1) LIKE '/register%'
                      OR split_part(path, '?', 1) LIKE '/confirm%'
                      OR split_part(path, '?', 1) LIKE '/set-password%' THEN 'auth'
                    WHEN split_part(path, '?', 1) LIKE '/cancel/%'
                      OR split_part(path, '?', 1) LIKE '/certificate/%'
                      OR split_part(path, '?', 1) LIKE '/unsubscribe/%' THEN 'letter'
                    ELSE 'other'
                END
"#;

#[derive(Clone)]
pub struct Repository {
    pg_pool: PgPool,
}

/// Result of `Repository::get_favorite_tiers` — `house_favorite` is always a
/// subset of `noticed` (same ranking, narrower percentile cutoff), so callers
/// can check `house_favorite` first and fall back to `noticed`.
#[derive(Debug, Default, Clone)]
pub struct FavoriteTiers {
    pub noticed: std::collections::HashSet<Uuid>,
    pub house_favorite: std::collections::HashSet<Uuid>,
}

/// Result of attempting to attach a token-bearing guest request to a user.
pub struct ClaimMatch {
    /// Whether the row's requester email matched the account's email.
    pub email_ok: bool,
    /// Whether this call set (or confirmed) the row's user_id to the account.
    pub linked: bool,
    /// Figurine / petition name for a human-readable confirmation.
    pub name: String,
}

/// Работа вместе с тем, чего клиенту не показывают: чья она и на каком языке
/// писалась. Отдельным типом, а не полями в `StudioCardDto`: владелец в теле
/// ответа — это то, что однажды уедет на страницу по недосмотру.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StudioCardRow {
    #[sqlx(flatten)]
    pub card: crate::models::StudioCardDto,
    pub owner_id: Uuid,
    pub lang: String,
}

impl Repository {
    pub fn new(pg_pool: PgPool) -> Self {
        Self { pg_pool }
    }

    pub fn pg_pool(&self) -> &PgPool {
        &self.pg_pool
    }

    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pg_pool).await?;
        Ok(())
    }

    // === FIGURINE ANALYTICS ===

    pub async fn bulk_insert_analytics_events(
        &self,
        events: &[AnalyticsEventRecord],
    ) -> Result<u64> {
        if events.is_empty() {
            return Ok(0);
        }

        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO figurine_analytics_events (
                occurred_at, event_date, event_type, figurine_id, visitor_hash,
                page_view_id, path, source, referrer_host, utm_source, utm_medium,
                utm_campaign, device_class, browser_family, country_code,
                duration_ms, scroll_depth, works_seen, cta_type, user_id, lang, internal_source
            ) ",
        );
        builder.push_values(events, |mut b, event| {
            b.push_bind(event.occurred_at)
                .push_bind(event.event_date)
                .push_bind(event.event_type)
                .push_bind(event.figurine_id)
                .push_bind(&event.visitor_hash)
                .push_bind(event.page_view_id)
                .push_bind(&event.path)
                .push_bind(&event.source)
                .push_bind(&event.referrer_host)
                .push_bind(&event.utm_source)
                .push_bind(&event.utm_medium)
                .push_bind(&event.utm_campaign)
                .push_bind(&event.device_class)
                .push_bind(&event.browser_family)
                .push_bind(&event.country_code)
                .push_bind(event.duration_ms)
                .push_bind(event.scroll_depth)
                .push_bind(event.works_seen)
                .push_bind(&event.cta_type)
                .push_bind(event.user_id)
                .push_bind(&event.lang)
                .push_bind(&event.internal_source);
        });

        let result = builder.build().execute(&self.pg_pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn refresh_analytics_aggregates(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<()> {
        let mut tx = self.pg_pool.begin().await?;

        sqlx::query("DELETE FROM figurine_analytics_daily WHERE day BETWEEN $1 AND $2")
            .bind(from)
            .bind(to)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM figurine_analytics_sources_daily WHERE day BETWEEN $1 AND $2")
            .bind(from)
            .bind(to)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            r#"
            INSERT INTO figurine_analytics_daily (
                figurine_id, day, views, unique_visitors, engaged_views, cta_clicks,
                order_starts, reserve_starts, booking_starts, waitlist_starts,
                commission_starts, orders_submitted, bookings_submitted,
                waitlist_submitted, commissions_submitted, updated_at
            )
            SELECT
                figurine_id,
                day,
                SUM(views)::int,
                SUM(unique_visitors)::int,
                SUM(engaged_views)::int,
                SUM(cta_clicks)::int,
                SUM(order_starts)::int,
                SUM(reserve_starts)::int,
                SUM(booking_starts)::int,
                SUM(waitlist_starts)::int,
                SUM(commission_starts)::int,
                SUM(orders_submitted)::int,
                SUM(bookings_submitted)::int,
                SUM(waitlist_submitted)::int,
                SUM(commissions_submitted)::int,
                NOW()
            FROM (
                SELECT
                    e.figurine_id,
                    e.event_date AS day,
                    COUNT(*) FILTER (WHERE event_type = 'figurine_view') AS views,
                    COUNT(DISTINCT visitor_hash) FILTER (
                        WHERE event_type = 'figurine_view' AND visitor_hash IS NOT NULL
                    ) AS unique_visitors,
                    COUNT(*) FILTER (WHERE event_type = 'figurine_engaged') AS engaged_views,
                    COUNT(*) FILTER (WHERE event_type = 'figurine_cta_click') AS cta_clicks,
                    COUNT(*) FILTER (
                        WHERE event_type = 'figurine_cta_click' AND cta_type = 'request'
                    ) AS order_starts,
                    COUNT(*) FILTER (
                        WHERE event_type = 'figurine_cta_click' AND cta_type = 'reserve'
                    ) AS reserve_starts,
                    COUNT(*) FILTER (
                        WHERE event_type = 'figurine_cta_click' AND cta_type = 'booking'
                    ) AS booking_starts,
                    COUNT(*) FILTER (
                        WHERE event_type = 'figurine_cta_click' AND cta_type IN ('waitlist', 'notify')
                    ) AS waitlist_starts,
                    COUNT(*) FILTER (
                        WHERE event_type = 'figurine_cta_click' AND cta_type = 'create_similar'
                    ) AS commission_starts,
                    0::bigint AS orders_submitted,
                    0::bigint AS bookings_submitted,
                    0::bigint AS waitlist_submitted,
                    0::bigint AS commissions_submitted
                FROM figurine_analytics_events e
                INNER JOIN figurines f ON f.id = e.figurine_id
                WHERE e.event_date BETWEEN $1 AND $2
                GROUP BY e.figurine_id, e.event_date

                UNION ALL

                SELECT
                    figurine_id::uuid,
                    created_at::date AS day,
                    0, 0, 0, 0,
                    0, 0, 0, 0, 0,
                    COUNT(*) FILTER (WHERE mode IN ('request', 'reserve')) AS orders_submitted,
                    0, 0, 0
                FROM orders
                WHERE created_at::date BETWEEN $1 AND $2
                  AND figurine_id ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                GROUP BY figurine_id::uuid, created_at::date

                UNION ALL

                SELECT
                    figurine_id,
                    created_at::date AS day,
                    0, 0, 0, 0,
                    0, 0, 0, 0, 0,
                    0,
                    COUNT(*) FILTER (WHERE status != 'cancelled') AS bookings_submitted,
                    0,
                    0
                FROM figurine_bookings
                WHERE created_at::date BETWEEN $1 AND $2
                GROUP BY figurine_id, created_at::date

                UNION ALL

                SELECT
                    figurine_id,
                    created_at::date AS day,
                    0, 0, 0, 0,
                    0, 0, 0, 0, 0,
                    0, 0,
                    COUNT(*) AS waitlist_submitted,
                    0
                FROM figurine_waitlist
                WHERE created_at::date BETWEEN $1 AND $2
                GROUP BY figurine_id, created_at::date

                UNION ALL

                SELECT
                    source_figurine_id::uuid,
                    created_at::date AS day,
                    0, 0, 0, 0,
                    0, 0, 0, 0, 0,
                    0, 0, 0,
                    COUNT(*) AS commissions_submitted
                FROM commissions
                WHERE created_at::date BETWEEN $1 AND $2
                  -- Attribute to the work that inspired the "create similar" petition
                  -- (set at submission time), not `figurine_id`, which is only filled
                  -- in later if/when an admin accepts and links the commission to a
                  -- real piece — using that column here silently undercounted this
                  -- funnel step to near-zero.
                  AND source_figurine_id ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                GROUP BY source_figurine_id::uuid, created_at::date
            ) rows
            GROUP BY figurine_id, day
            HAVING SUM(views) > 0
                OR SUM(engaged_views) > 0
                OR SUM(cta_clicks) > 0
                OR SUM(orders_submitted) > 0
                OR SUM(bookings_submitted) > 0
                OR SUM(waitlist_submitted) > 0
                OR SUM(commissions_submitted) > 0
            "#,
        )
        .bind(from)
        .bind(to)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO figurine_analytics_sources_daily (
                figurine_id, day, source, views, unique_visitors, updated_at
            )
            SELECT
                e.figurine_id,
                e.event_date AS day,
                e.source,
                COUNT(*)::int AS views,
                COUNT(DISTINCT visitor_hash) FILTER (WHERE visitor_hash IS NOT NULL)::int AS unique_visitors,
                NOW()
            FROM figurine_analytics_events e
            INNER JOIN figurines f ON f.id = e.figurine_id
            WHERE e.event_date BETWEEN $1 AND $2
              AND e.event_type = 'figurine_view'
            GROUP BY e.figurine_id, e.event_date, e.source
            "#,
        )
        .bind(from)
        .bind(to)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM figurine_analytics_geo_daily WHERE day BETWEEN $1 AND $2")
            .bind(from)
            .bind(to)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            r#"
            INSERT INTO figurine_analytics_geo_daily (
                figurine_id, day, country_code, views, unique_visitors, updated_at
            )
            SELECT
                e.figurine_id,
                e.event_date AS day,
                COALESCE(e.country_code, 'unknown'),
                COUNT(*)::int AS views,
                COUNT(DISTINCT visitor_hash) FILTER (WHERE visitor_hash IS NOT NULL)::int AS unique_visitors,
                NOW()
            FROM figurine_analytics_events e
            INNER JOIN figurines f ON f.id = e.figurine_id
            WHERE e.event_date BETWEEN $1 AND $2
              AND e.event_type = 'figurine_view'
            GROUP BY e.figurine_id, e.event_date, COALESCE(e.country_code, 'unknown')
            "#,
        )
        .bind(from)
        .bind(to)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM site_geo_daily WHERE day BETWEEN $1 AND $2")
            .bind(from)
            .bind(to)
            .execute(&mut *tx)
            .await?;

        // Every page view site-wide (figurine detail pages + the generic
        // pages), unlike figurine_analytics_geo_daily above which is
        // figurine-detail-only — this is the total "where do visits come
        // from" picture the admin geography map needs.
        sqlx::query(
            r#"
            INSERT INTO site_geo_daily (day, country_code, views, unique_visitors, updated_at)
            SELECT
                event_date AS day,
                COALESCE(country_code, 'unknown'),
                COUNT(*)::int AS views,
                COUNT(DISTINCT visitor_hash) FILTER (WHERE visitor_hash IS NOT NULL)::int AS unique_visitors,
                NOW()
            FROM figurine_analytics_events
            WHERE event_date BETWEEN $1 AND $2
              AND event_type IN ('page_view', 'figurine_view')
            GROUP BY event_date, COALESCE(country_code, 'unknown')
            "#,
        )
        .bind(from)
        .bind(to)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM site_page_views_daily WHERE day BETWEEN $1 AND $2")
            .bind(from)
            .bind(to)
            .execute(&mut *tx)
            .await?;

        // Only figurine_id IS NULL 'page_view' events land here — figurine
        // detail pages are tracked separately via 'figurine_view' above, so
        // there's no overlap/double-count between this table and
        // figurine_analytics_daily.
        sqlx::query(&format!(
            r#"
            INSERT INTO site_page_views_daily (day, path_group, views, unique_visitors, updated_at)
            SELECT
                event_date AS day,
{path_group} AS path_group,
                COUNT(*)::int AS views,
                COUNT(DISTINCT visitor_hash) FILTER (WHERE visitor_hash IS NOT NULL)::int AS unique_visitors,
                NOW()
            FROM figurine_analytics_events
            WHERE event_date BETWEEN $1 AND $2
              AND event_type = 'page_view'
              AND figurine_id IS NULL
            GROUP BY event_date, path_group
            "#,
            path_group = PATH_GROUP_SQL,
        ))
        .bind(from)
        .bind(to)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Earliest day already present in figurine_analytics_daily — the sensible
    /// default start for a manual backfill (no point re-aggregating days
    /// before analytics existed).
    pub async fn get_earliest_analytics_day(&self) -> Result<Option<chrono::NaiveDate>> {
        let row: (Option<chrono::NaiveDate>,) =
            sqlx::query_as("SELECT MIN(day) FROM figurine_analytics_daily")
                .fetch_one(&self.pg_pool)
                .await?;
        Ok(row.0)
    }

    pub async fn get_admin_figurine_analytics_list(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
        sort: &str,
        dir: &str,
    ) -> Result<Vec<AdminFigurineAnalyticsListItem>> {
        let order_col = match sort {
            "name" => "f.name",
            "status" => "f.status::text",
            "uniqueVisitors" | "unique_visitors" => "unique_visitors",
            "engagedViews" | "engaged_views" => "engaged_views",
            "ctaClicks" | "cta_clicks" => "cta_clicks",
            "submissions" => "submissions",
            "conversionRate" | "conversion_rate" => "conversion_rate",
            _ => "views",
        };
        let order_dir = if dir.eq_ignore_ascii_case("asc") {
            "ASC"
        } else {
            "DESC"
        };
        let sql = format!(
            r#"
            WITH face AS (
                SELECT DISTINCT ON (figurine_id)
                    figurine_id,
                    COALESCE(thumb_path, file_path) AS face_url
                FROM images
                WHERE image_type = 'face'
                ORDER BY figurine_id, sort_order
            ),
            stats AS (
                SELECT
                    figurine_id,
                    COALESCE(SUM(views), 0)::bigint AS views,
                    COALESCE(SUM(unique_visitors), 0)::bigint AS unique_visitors,
                    COALESCE(SUM(engaged_views), 0)::bigint AS engaged_views,
                    COALESCE(SUM(cta_clicks), 0)::bigint AS cta_clicks,
                    COALESCE(SUM(
                        orders_submitted + bookings_submitted + waitlist_submitted + commissions_submitted
                    ), 0)::bigint AS submissions
                FROM figurine_analytics_daily
                WHERE day BETWEEN $1 AND $2
                GROUP BY figurine_id
            ),
            top_source AS (
                SELECT figurine_id, source AS top_source
                FROM (
                    SELECT
                        figurine_id,
                        source,
                        COUNT(*) AS views,
                        ROW_NUMBER() OVER (PARTITION BY figurine_id ORDER BY COUNT(*) DESC, source ASC) AS rn
                    FROM figurine_analytics_events
                    WHERE event_date BETWEEN $1 AND $2 AND event_type = 'figurine_view'
                    GROUP BY figurine_id, source
                ) ranked
                WHERE rn = 1
            ),
            top_country AS (
                -- Sourced from the permanent geo rollup, not raw events: raw
                -- figurine_analytics_events are pruned after
                -- analytics::RETENTION_DAYS, which would silently blank this
                -- column for any range reaching further back than that.
                SELECT figurine_id, country_code AS top_country
                FROM (
                    SELECT
                        figurine_id,
                        country_code,
                        SUM(views) AS views,
                        ROW_NUMBER() OVER (PARTITION BY figurine_id ORDER BY SUM(views) DESC, country_code ASC) AS rn
                    FROM figurine_analytics_geo_daily
                    WHERE day BETWEEN $1 AND $2
                    GROUP BY figurine_id, country_code
                ) ranked
                WHERE rn = 1
            ),
            all_countries AS (
                SELECT
                    figurine_id,
                    array_agg(DISTINCT country_code ORDER BY country_code)
                        FILTER (WHERE country_code <> 'unknown') AS countries
                FROM figurine_analytics_geo_daily
                WHERE day BETWEEN $1 AND $2
                GROUP BY figurine_id
            ),
            top_device AS (
                SELECT figurine_id, device_class AS top_device
                FROM (
                    SELECT
                        figurine_id,
                        COALESCE(device_class, 'unknown') AS device_class,
                        COUNT(*) AS views,
                        ROW_NUMBER() OVER (PARTITION BY figurine_id ORDER BY COUNT(*) DESC, COALESCE(device_class, 'unknown') ASC) AS rn
                    FROM figurine_analytics_events
                    WHERE event_date BETWEEN $1 AND $2 AND event_type = 'figurine_view'
                    GROUP BY figurine_id, COALESCE(device_class, 'unknown')
                ) ranked
                WHERE rn = 1
            ),
            top_browser AS (
                SELECT figurine_id, browser_family AS top_browser
                FROM (
                    SELECT
                        figurine_id,
                        COALESCE(browser_family, 'unknown') AS browser_family,
                        COUNT(*) AS views,
                        ROW_NUMBER() OVER (PARTITION BY figurine_id ORDER BY COUNT(*) DESC, COALESCE(browser_family, 'unknown') ASC) AS rn
                    FROM figurine_analytics_events
                    WHERE event_date BETWEEN $1 AND $2 AND event_type = 'figurine_view'
                    GROUP BY figurine_id, COALESCE(browser_family, 'unknown')
                ) ranked
                WHERE rn = 1
            )
            SELECT
                f.id::text AS figurine_id,
                f.name,
                f.status,
                f.series,
                face.face_url,
                top_source.top_source,
                top_country.top_country,
                top_device.top_device,
                top_browser.top_browser,
                COALESCE(all_countries.countries, ARRAY[]::text[]) AS countries,
                COALESCE(stats.views, 0)::bigint AS views,
                COALESCE(stats.unique_visitors, 0)::bigint AS unique_visitors,
                COALESCE(stats.engaged_views, 0)::bigint AS engaged_views,
                COALESCE(stats.cta_clicks, 0)::bigint AS cta_clicks,
                COALESCE(stats.submissions, 0)::bigint AS submissions,
                CASE
                    WHEN COALESCE(stats.engaged_views, 0) > 0
                    THEN ROUND((COALESCE(stats.submissions, 0)::numeric / stats.engaged_views::numeric) * 100, 2)::float8
                    ELSE 0::float8
                END AS conversion_rate
            FROM figurines f
            LEFT JOIN stats ON stats.figurine_id = f.id
            LEFT JOIN face ON face.figurine_id = f.id
            LEFT JOIN top_source ON top_source.figurine_id = f.id
            LEFT JOIN top_country ON top_country.figurine_id = f.id
            LEFT JOIN top_device ON top_device.figurine_id = f.id
            LEFT JOIN top_browser ON top_browser.figurine_id = f.id
            LEFT JOIN all_countries ON all_countries.figurine_id = f.id
            ORDER BY {order_col} {order_dir}, f.name ASC
            "#
        );

        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                FigurineStatus,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Vec<String>,
                i64,
                i64,
                i64,
                i64,
                i64,
                f64,
            ),
        >(&sql)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    figurine_id,
                    name,
                    status,
                    series,
                    face_url,
                    top_source,
                    top_country,
                    top_device,
                    top_browser,
                    countries,
                    views,
                    unique_visitors,
                    engaged_views,
                    cta_clicks,
                    submissions,
                    conversion_rate,
                )| {
                    AdminFigurineAnalyticsListItem {
                        figurine_id,
                        name,
                        status,
                        series,
                        face_url,
                        // The service layer overwrites both of these once it has
                        // merged in the growth-window query (signal/growth depend
                        // on more than this one row can see — see
                        // `AppService::analytics_signal`).
                        signal: AnalyticsSignal::Normal,
                        is_growing: false,
                        top_source,
                        top_country,
                        top_device,
                        top_browser,
                        countries,
                        views,
                        unique_visitors,
                        engaged_views,
                        cta_clicks,
                        submissions,
                        conversion_rate,
                        sparkline: Vec::new(),
                    }
                },
            )
            .collect())
    }

    /// Last-14-days-ending-at-`to` daily view counts for every figurine, for the
    /// works-table row sparklines. A fixed window regardless of the selected
    /// range so sparklines stay a comparable shape.
    pub async fn get_figurine_sparklines(
        &self,
        to: chrono::NaiveDate,
    ) -> Result<std::collections::HashMap<Uuid, Vec<(chrono::NaiveDate, i64)>>> {
        let from = to - chrono::Duration::days(13);
        let rows: Vec<(Uuid, chrono::NaiveDate, i64)> = sqlx::query_as(
            r#"
            SELECT figurine_id, day, views::bigint
            FROM figurine_analytics_daily
            WHERE day BETWEEN $1 AND $2
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?;

        let mut map: std::collections::HashMap<Uuid, Vec<(chrono::NaiveDate, i64)>> =
            std::collections::HashMap::new();
        for (figurine_id, day, views) in rows {
            map.entry(figurine_id).or_default().push((day, views));
        }
        Ok(map)
    }

    /// Week-over-week view totals for every figurine, anchored at `anchor`
    /// (typically the query's `to` date): `last7` = anchor-6..=anchor, `prior7`
    /// = anchor-13..=anchor-7. Used to compute the `growing_interest` signal.
    pub async fn get_admin_growth_window(
        &self,
        anchor: chrono::NaiveDate,
    ) -> Result<std::collections::HashMap<Uuid, (i64, i64)>> {
        let last7_from = anchor - chrono::Duration::days(6);
        let prior7_from = anchor - chrono::Duration::days(13);
        let prior7_to = anchor - chrono::Duration::days(7);
        let rows: Vec<(Uuid, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                figurine_id,
                COALESCE(SUM(views) FILTER (WHERE day BETWEEN $2 AND $1), 0)::bigint AS last7,
                COALESCE(SUM(views) FILTER (WHERE day BETWEEN $3 AND $4), 0)::bigint AS prior7
            FROM figurine_analytics_daily
            WHERE day BETWEEN $3 AND $1
            GROUP BY figurine_id
            "#,
        )
        .bind(anchor)
        .bind(last7_from)
        .bind(prior7_from)
        .bind(prior7_to)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, last7, prior7)| (id, (last7, prior7)))
            .collect())
    }

    /// Single-figurine variant of `get_admin_growth_window`, for the detail view.
    pub async fn get_figurine_growth_window(
        &self,
        figurine_id: Uuid,
        anchor: chrono::NaiveDate,
    ) -> Result<(i64, i64)> {
        let last7_from = anchor - chrono::Duration::days(6);
        let prior7_from = anchor - chrono::Duration::days(13);
        let prior7_to = anchor - chrono::Duration::days(7);
        let row: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(views) FILTER (WHERE day BETWEEN $2 AND $1), 0)::bigint AS last7,
                COALESCE(SUM(views) FILTER (WHERE day BETWEEN $3 AND $4), 0)::bigint AS prior7
            FROM figurine_analytics_daily
            WHERE figurine_id = $5 AND day BETWEEN $3 AND $1
            "#,
        )
        .bind(anchor)
        .bind(last7_from)
        .bind(prior7_from)
        .bind(prior7_to)
        .bind(figurine_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row)
    }

    /// The starts -> submitted funnel per CTA family for one figurine. Starts
    /// come from the pre-aggregated daily table (client-side clicks, so subject
    /// to DNT/bot/direct-link undercounting). Submitted is counted directly from
    /// the real orders/bookings/waitlist/commissions tables — the source of
    /// truth — not from the daily table's own submitted columns, since those
    /// merge request+reserve into one column and (for commissions) previously
    /// joined on the wrong figurine column; querying the source tables directly
    /// sidesteps both issues without a schema migration.
    pub async fn get_admin_figurine_cta_funnel(
        &self,
        figurine_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<CtaFunnelStep>> {
        let starts: (i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(order_starts), 0)::bigint,
                COALESCE(SUM(reserve_starts), 0)::bigint,
                COALESCE(SUM(booking_starts), 0)::bigint,
                COALESCE(SUM(waitlist_starts), 0)::bigint,
                COALESCE(SUM(commission_starts), 0)::bigint
            FROM figurine_analytics_daily
            WHERE figurine_id = $1 AND day BETWEEN $2 AND $3
            "#,
        )
        .bind(figurine_id)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pg_pool)
        .await?;

        let figurine_id_text = figurine_id.to_string();
        let submitted: (i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM orders
                    WHERE figurine_id = $1 AND mode = 'request'
                    AND created_at::date BETWEEN $2 AND $3)::bigint,
                (SELECT COUNT(*) FROM orders
                    WHERE figurine_id = $1 AND mode = 'reserve'
                    AND created_at::date BETWEEN $2 AND $3)::bigint,
                (SELECT COUNT(*) FROM figurine_bookings
                    WHERE figurine_id = $4 AND status != 'cancelled'
                    AND created_at::date BETWEEN $2 AND $3)::bigint,
                (SELECT COUNT(*) FROM figurine_waitlist
                    WHERE figurine_id = $4
                    AND created_at::date BETWEEN $2 AND $3)::bigint,
                (SELECT COUNT(*) FROM commissions
                    WHERE source_figurine_id = $1
                    AND created_at::date BETWEEN $2 AND $3)::bigint
            "#,
        )
        .bind(&figurine_id_text)
        .bind(from)
        .bind(to)
        .bind(figurine_id)
        .fetch_one(&self.pg_pool)
        .await?;

        Ok(vec![
            CtaFunnelStep {
                cta_type: "request".into(),
                starts: starts.0,
                submitted: submitted.0,
            },
            CtaFunnelStep {
                cta_type: "reserve".into(),
                starts: starts.1,
                submitted: submitted.1,
            },
            CtaFunnelStep {
                cta_type: "booking".into(),
                starts: starts.2,
                submitted: submitted.2,
            },
            CtaFunnelStep {
                cta_type: "waitlist".into(),
                starts: starts.3,
                submitted: submitted.3,
            },
            CtaFunnelStep {
                cta_type: "commission".into(),
                starts: starts.4,
                submitted: submitted.4,
            },
        ])
    }

    /// Median engagement duration/scroll-depth for one figurine, from raw
    /// `figurine_engaged` events. NULL samples are excluded before the
    /// percentile is computed (never treated as 0); returns `None` for either
    /// value when there are no qualifying events in range (e.g. the whole range
    /// predates raw-event retention).
    pub async fn get_admin_figurine_engagement_medians(
        &self,
        figurine_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<(Option<f64>, Option<f64>)> {
        let row: (Option<f64>, Option<f64>) = sqlx::query_as(
            r#"
            SELECT
                percentile_cont(0.5) WITHIN GROUP (ORDER BY duration_ms)
                    FILTER (WHERE duration_ms IS NOT NULL),
                percentile_cont(0.5) WITHIN GROUP (ORDER BY scroll_depth)
                    FILTER (WHERE scroll_depth IS NOT NULL)
            FROM figurine_analytics_events
            WHERE figurine_id = $1
              AND event_type = 'figurine_engaged'
              AND event_date BETWEEN $2 AND $3
            "#,
        )
        .bind(figurine_id)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row)
    }

    /// Site-wide daily trend (all figurines summed) — the Overview screen's main
    /// chart, built from the same pre-aggregated table so it isn't bound by raw
    /// event retention.
    pub async fn get_admin_site_overview_daily(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<AnalyticsDailyPoint>> {
        Ok(sqlx::query_as::<_, AnalyticsDailyPoint>(
            r#"
            SELECT
                day,
                SUM(views)::bigint AS views,
                SUM(unique_visitors)::bigint AS unique_visitors,
                SUM(engaged_views)::bigint AS engaged_views,
                SUM(cta_clicks)::bigint AS cta_clicks,
                SUM(orders_submitted + bookings_submitted + waitlist_submitted + commissions_submitted)::bigint AS submissions
            FROM figurine_analytics_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY day
            ORDER BY day ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Site-wide channel breakdown (all figurines summed) for the Sources screen.
    pub async fn get_admin_site_analytics_sources(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<AnalyticsSourcePoint>> {
        Ok(sqlx::query_as::<_, AnalyticsSourcePoint>(
            r#"
            SELECT
                source,
                COALESCE(SUM(views), 0)::bigint AS views,
                COALESCE(SUM(unique_visitors), 0)::bigint AS unique_visitors
            FROM figurine_analytics_sources_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY source
            ORDER BY views DESC, source ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Site-wide daily views by country (every page, not just figurine
    /// detail pages) — the geography map's data source. Permanent, not
    /// retention-bound, unlike a raw-event query would be.
    pub async fn get_admin_site_geo(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<AnalyticsBreakdownPoint>> {
        Ok(sqlx::query_as::<_, AnalyticsBreakdownPoint>(
            r#"
            SELECT
                country_code AS key,
                COALESCE(SUM(views), 0)::bigint AS views,
                COALESCE(SUM(unique_visitors), 0)::bigint AS unique_visitors
            FROM site_geo_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY country_code
            ORDER BY views DESC, country_code ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Daily views/uniques for the generic (non-figurine) pages, keyed by
    /// coarse path_group. Permanent, not retention-bound.
    pub async fn get_admin_site_page_views_daily(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<(chrono::NaiveDate, i64, i64)>> {
        Ok(sqlx::query_as(
            r#"
            SELECT day, SUM(views)::bigint, SUM(unique_visitors)::bigint
            FROM site_page_views_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY day
            ORDER BY day ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Per-generic-page engagement from raw `page_engaged` events: engaged
    /// count, quick-exit count (visits under `QUICK_EXIT_MS`), reached-works
    /// count (grid visits that saw ≥1 tile), and the time/scroll/works medians.
    /// Same NULL-excluding percentile approach as
    /// `get_admin_figurine_engagement_medians` (NULLs never counted as 0), and
    /// same retention limit — the caller clamps `from` to the raw-event floor.
    /// `views`/`unique_visitors` are left 0 here and filled by the caller from
    /// the permanent page-views rollup. Rooms come from the same
    /// `PATH_GROUP_SQL` the `site_page_views_daily` rollup uses, so the two
    /// line up row-for-row.
    pub async fn get_admin_site_page_engagement(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<SitePageEngagement>> {
        Ok(
            sqlx::query_as::<_, (String, i64, i64, i64, Option<f64>, Option<f64>, Option<f64>)>(
                &format!(
                    r#"
            SELECT
{path_group} AS path_group,
                COUNT(*)::bigint AS engaged_events,
                COUNT(*) FILTER (WHERE duration_ms IS NOT NULL AND duration_ms < $3)::bigint
                    AS quick_exit_events,
                COUNT(*) FILTER (WHERE works_seen IS NOT NULL AND works_seen >= 1)::bigint
                    AS reached_works_events,
                percentile_cont(0.5) WITHIN GROUP (ORDER BY duration_ms)
                    FILTER (WHERE duration_ms IS NOT NULL) AS median_duration_ms,
                percentile_cont(0.5) WITHIN GROUP (ORDER BY scroll_depth)
                    FILTER (WHERE scroll_depth IS NOT NULL) AS median_scroll_depth,
                percentile_cont(0.5) WITHIN GROUP (ORDER BY works_seen)
                    FILTER (WHERE works_seen IS NOT NULL) AS median_works_seen
            FROM figurine_analytics_events
            WHERE event_date BETWEEN $1 AND $2
              AND event_type = 'page_engaged'
              AND figurine_id IS NULL
            GROUP BY path_group
            ORDER BY engaged_events DESC
            "#,
                    path_group = PATH_GROUP_SQL,
                ),
            )
            .bind(from)
            .bind(to)
            .bind(crate::analytics::QUICK_EXIT_MS)
            .fetch_all(&self.pg_pool)
            .await?
            .into_iter()
            .map(
                |(
                    path_group,
                    engaged_events,
                    quick_exit_events,
                    reached_works_events,
                    median_duration_ms,
                    median_scroll_depth,
                    median_works_seen,
                )| SitePageEngagement {
                    path_group,
                    views: 0,
                    unique_visitors: 0,
                    engaged_events,
                    quick_exit_events,
                    reached_works_events,
                    median_duration_ms,
                    median_scroll_depth,
                    median_works_seen,
                },
            )
            .collect(),
        )
    }

    /// Per-`path_group` views/unique-visitors from the permanent
    /// `site_page_views_daily` rollup (not retention-bound) — the denominator
    /// that gives the engagement rates and "works seen" their meaning. Sibling
    /// of `get_admin_site_page_views_daily`, which collapses the groups into a
    /// single daily total.
    pub async fn get_admin_site_page_views_by_group(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<(String, i64, i64)>> {
        Ok(sqlx::query_as(
            r#"
            SELECT path_group, SUM(views)::bigint, SUM(unique_visitors)::bigint
            FROM site_page_views_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY path_group
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// One row per anonymous visitor (daily `visitor_hash`) active in range,
    /// newest last-seen first. Representative device/browser/country/lang/source
    /// are taken from the visit's earliest event (array_agg ORDER BY occurred_at,
    /// first non-null). Raw-event derived, so the caller clamps to retention.
    pub async fn get_admin_visitor_sessions(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
        limit: i64,
        offset: i64,
        only_actions: bool,
    ) -> Result<Vec<AdminVisitorSession>> {
        Ok(sqlx::query_as::<_, AdminVisitorSession>(
            r#"
            SELECT
                visitor_hash,
                MAX(event_date) AS day,
                MIN(occurred_at) AS first_seen,
                MAX(occurred_at) AS last_seen,
                COUNT(*)::bigint AS event_count,
                COUNT(*) FILTER (WHERE event_type = 'page_view')::bigint AS page_views,
                COUNT(*) FILTER (WHERE event_type = 'figurine_view')::bigint AS figurine_views,
                COUNT(*) FILTER (WHERE event_type = 'figurine_cta_click')::bigint AS cta_clicks,
                COALESCE(
                    array_agg(DISTINCT cta_type) FILTER (WHERE cta_type IS NOT NULL),
                    ARRAY[]::text[]
                ) AS cta_types,
                MAX(works_seen) AS max_works_seen,
                MAX(scroll_depth) AS max_scroll_depth,
                (array_agg(country_code ORDER BY occurred_at) FILTER (WHERE country_code IS NOT NULL))[1] AS country_code,
                (array_agg(device_class ORDER BY occurred_at) FILTER (WHERE device_class IS NOT NULL))[1] AS device_class,
                (array_agg(browser_family ORDER BY occurred_at) FILTER (WHERE browser_family IS NOT NULL))[1] AS browser_family,
                (array_agg(lang ORDER BY occurred_at) FILTER (WHERE lang IS NOT NULL))[1] AS lang,
                (array_agg(source ORDER BY occurred_at) FILTER (WHERE source IS NOT NULL))[1] AS source
            FROM figurine_analytics_events
            WHERE event_date BETWEEN $1 AND $2 AND visitor_hash IS NOT NULL
            GROUP BY visitor_hash
            HAVING (NOT $5 OR COUNT(*) FILTER (WHERE event_type = 'figurine_cta_click') > 0)
            ORDER BY last_seen DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .bind(only_actions)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Total distinct anonymous visitors in range — the count behind the paged
    /// `get_admin_visitor_sessions` list. Honours the same `only_actions`
    /// filter so the pager total matches the filtered list.
    pub async fn count_admin_visitor_sessions(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
        only_actions: bool,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM (
                SELECT visitor_hash
                FROM figurine_analytics_events
                WHERE event_date BETWEEN $1 AND $2 AND visitor_hash IS NOT NULL
                GROUP BY visitor_hash
                HAVING (NOT $3 OR COUNT(*) FILTER (WHERE event_type = 'figurine_cta_click') > 0)
            ) t
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(only_actions)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row.0)
    }

    /// One anonymous visitor's full event timeline, oldest first, with figurine
    /// names resolved for the works they opened. Capped so a pathological visit
    /// can't return an unbounded payload.
    pub async fn get_admin_visitor_timeline(
        &self,
        visitor_hash: &str,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
        limit: i64,
    ) -> Result<Vec<AdminVisitorEvent>> {
        Ok(sqlx::query_as::<_, AdminVisitorEvent>(
            r#"
            SELECT
                e.occurred_at,
                e.event_type,
                e.path,
                e.figurine_id,
                f.name AS figurine_name,
                e.duration_ms,
                e.scroll_depth,
                e.works_seen,
                e.cta_type,
                e.source,
                e.internal_source
            FROM figurine_analytics_events e
            LEFT JOIN figurines f ON f.id = e.figurine_id
            WHERE e.visitor_hash = $1 AND e.event_date BETWEEN $2 AND $3
            ORDER BY e.occurred_at ASC
            LIMIT $4
            "#,
        )
        .bind(visitor_hash)
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Every real submission site-wide by day, regardless of which figurine
    /// (or none) it's attributed to — orders/bookings/waitlist/commissions
    /// have full history (not retention-pruned), and a commission submitted
    /// via the general /commission form has no figurine attribution at all,
    /// so this is the only honest site-wide "submissions" total (the
    /// per-figurine sum in figurine_analytics_daily necessarily excludes those).
    pub async fn get_admin_site_submissions_daily(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<(chrono::NaiveDate, i64)>> {
        Ok(sqlx::query_as(
            r#"
            SELECT day, SUM(cnt)::bigint FROM (
                SELECT created_at::date AS day, COUNT(*) AS cnt
                FROM orders
                WHERE mode IN ('request', 'reserve') AND created_at::date BETWEEN $1 AND $2
                GROUP BY created_at::date

                UNION ALL

                SELECT created_at::date, COUNT(*)
                FROM figurine_bookings
                WHERE status != 'cancelled' AND created_at::date BETWEEN $1 AND $2
                GROUP BY created_at::date

                UNION ALL

                SELECT created_at::date, COUNT(*)
                FROM figurine_waitlist
                WHERE created_at::date BETWEEN $1 AND $2
                GROUP BY created_at::date

                UNION ALL

                SELECT created_at::date, COUNT(*)
                FROM commissions
                WHERE created_at::date BETWEEN $1 AND $2
                GROUP BY created_at::date
            ) x
            GROUP BY day
            ORDER BY day ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Site → works → /commission → started form → submitted. The first four
    /// counts are distinct visitors from raw events (retention-bound —
    /// callers must clamp `from`); `submitted` is exact.
    pub async fn get_admin_commission_funnel(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<(i64, i64, i64, i64, i64)> {
        sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(DISTINCT visitor_hash) FROM figurine_analytics_events
                    WHERE event_date BETWEEN $1 AND $2 AND visitor_hash IS NOT NULL)::bigint,
                (SELECT COUNT(DISTINCT visitor_hash) FROM figurine_analytics_events
                    WHERE event_date BETWEEN $1 AND $2 AND event_type = 'figurine_view'
                    AND visitor_hash IS NOT NULL)::bigint,
                (SELECT COUNT(DISTINCT visitor_hash) FROM figurine_analytics_events
                    WHERE event_date BETWEEN $1 AND $2 AND event_type = 'page_view'
                    AND path LIKE '/commission%' AND visitor_hash IS NOT NULL)::bigint,
                (SELECT COUNT(DISTINCT visitor_hash) FROM figurine_analytics_events
                    WHERE event_date BETWEEN $1 AND $2 AND event_type = 'figurine_cta_click'
                    AND cta_type = 'commission_form_start' AND visitor_hash IS NOT NULL)::bigint,
                (SELECT COUNT(*) FROM commissions
                    WHERE created_at::date BETWEEN $1 AND $2)::bigint
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_one(&self.pg_pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create_analytics_annotation(
        &self,
        req: &CreateAnnotationRequest,
    ) -> Result<AnalyticsAnnotation> {
        Ok(sqlx::query_as::<_, AnalyticsAnnotation>(
            "INSERT INTO analytics_annotations (day, label) VALUES ($1, $2) RETURNING *",
        )
        .bind(req.day)
        .bind(&req.label)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn list_analytics_annotations(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<AnalyticsAnnotation>> {
        Ok(sqlx::query_as::<_, AnalyticsAnnotation>(
            "SELECT * FROM analytics_annotations WHERE day BETWEEN $1 AND $2 ORDER BY day ASC",
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn delete_analytics_annotation(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM analytics_annotations WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    /// Daily marks/subscribers/comments — none retention-pruned, so a plain
    /// date-range query is honest for any range (no raw_data_from clamp needed).
    pub async fn get_admin_life_of_house_daily(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<LifeOfHouseDailyPoint>> {
        Ok(sqlx::query_as::<_, LifeOfHouseDailyPoint>(
            r#"
            SELECT day, SUM(marks)::bigint AS marks, SUM(subscribers)::bigint AS subscribers, SUM(comments)::bigint AS comments
            FROM (
                SELECT created_at::date AS day, COUNT(*) AS marks, 0::bigint AS subscribers, 0::bigint AS comments
                FROM figurine_marks
                WHERE created_at::date BETWEEN $1 AND $2
                GROUP BY created_at::date

                UNION ALL

                SELECT created_at::date, 0::bigint, COUNT(*), 0::bigint
                FROM newsletter_subscribers
                WHERE created_at::date BETWEEN $1 AND $2
                GROUP BY created_at::date

                UNION ALL

                SELECT created_at::date, 0::bigint, 0::bigint, COUNT(*)
                FROM figurine_comments
                WHERE created_at::date BETWEEN $1 AND $2
                GROUP BY created_at::date
            ) x
            GROUP BY day
            ORDER BY day ASC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_admin_figurine_analytics_daily(
        &self,
        figurine_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<AnalyticsDailyPoint>> {
        Ok(sqlx::query_as::<_, AnalyticsDailyPoint>(
            r#"
            SELECT
                day,
                views::bigint,
                unique_visitors::bigint,
                engaged_views::bigint,
                cta_clicks::bigint,
                (orders_submitted + bookings_submitted + waitlist_submitted + commissions_submitted)::bigint AS submissions
            FROM figurine_analytics_daily
            WHERE figurine_id = $1 AND day BETWEEN $2 AND $3
            ORDER BY day ASC
            "#,
        )
        .bind(figurine_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_admin_figurine_analytics_sources(
        &self,
        figurine_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<AnalyticsSourcePoint>> {
        Ok(sqlx::query_as::<_, AnalyticsSourcePoint>(
            r#"
            SELECT
                source,
                COALESCE(SUM(views), 0)::bigint AS views,
                COALESCE(SUM(unique_visitors), 0)::bigint AS unique_visitors
            FROM figurine_analytics_sources_daily
            WHERE figurine_id = $1 AND day BETWEEN $2 AND $3
            GROUP BY source
            ORDER BY views DESC, source ASC
            "#,
        )
        .bind(figurine_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_admin_figurine_analytics_breakdown(
        &self,
        figurine_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
        dimension: &str,
        limit: i64,
    ) -> Result<Vec<AnalyticsBreakdownPoint>> {
        let expr = match dimension {
            "country" => "COALESCE(country_code, 'unknown')",
            "device" => "COALESCE(device_class, 'unknown')",
            "browser" => "COALESCE(browser_family, 'unknown')",
            "referrer" => "COALESCE(referrer_host, 'direct')",
            "utm_source" => "COALESCE(utm_source, 'none')",
            "visitor" => "COALESCE(SUBSTRING(visitor_hash FROM 1 FOR 12), 'unknown')",
            "lang" => "COALESCE(lang, 'unknown')",
            "internal_source" => "COALESCE(internal_source, 'none')",
            _ => {
                return Err(AppError::BadRequest(
                    "Invalid analytics breakdown".to_string(),
                ));
            }
        };
        let sql = format!(
            r#"
            SELECT
                {expr} AS key,
                COUNT(*)::bigint AS views,
                COUNT(DISTINCT visitor_hash) FILTER (WHERE visitor_hash IS NOT NULL)::bigint AS unique_visitors
            FROM figurine_analytics_events
            WHERE figurine_id = $1
              AND event_date BETWEEN $2 AND $3
              AND event_type = 'figurine_view'
            GROUP BY {expr}
            ORDER BY views DESC, key ASC
            LIMIT $4
            "#
        );
        Ok(sqlx::query_as::<_, AnalyticsBreakdownPoint>(&sql)
            .bind(figurine_id)
            .bind(from)
            .bind(to)
            .bind(limit)
            .fetch_all(&self.pg_pool)
            .await?)
    }

    /// Per-figurine country breakdown from the permanent geo rollup, not raw
    /// events — unlike `get_admin_figurine_analytics_breakdown("country", ...)`,
    /// this isn't bound by `analytics::RETENTION_DAYS`, so it's the one used
    /// for the drilldown's country list (and the only one that stays correct
    /// for a range older than the raw-event retention window).
    pub async fn get_admin_figurine_geo_breakdown(
        &self,
        figurine_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
        limit: i64,
    ) -> Result<Vec<AnalyticsBreakdownPoint>> {
        Ok(sqlx::query_as::<_, AnalyticsBreakdownPoint>(
            r#"
            SELECT
                country_code AS key,
                COALESCE(SUM(views), 0)::bigint AS views,
                COALESCE(SUM(unique_visitors), 0)::bigint AS unique_visitors
            FROM figurine_analytics_geo_daily
            WHERE figurine_id = $1 AND day BETWEEN $2 AND $3
            GROUP BY country_code
            ORDER BY views DESC, country_code ASC
            LIMIT $4
            "#,
        )
        .bind(figurine_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Full (day, country) granularity for one figurine, from the permanent
    /// geo rollup — unlike `get_admin_figurine_geo_breakdown` above (which
    /// collapses to one row per country for the whole range), this keeps
    /// every day so the map's "one figurine" mode can list actual visit
    /// dates per country, not just a total.
    pub async fn get_admin_figurine_geo_daily(
        &self,
        figurine_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<FigurineGeoDailyPoint>> {
        Ok(sqlx::query_as::<_, FigurineGeoDailyPoint>(
            r#"
            SELECT day, country_code, views::bigint, unique_visitors::bigint
            FROM figurine_analytics_geo_daily
            WHERE figurine_id = $1 AND day BETWEEN $2 AND $3
            ORDER BY day ASC, country_code ASC
            "#,
        )
        .bind(figurine_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn prune_old_analytics_events_chunked(
        &self,
        retention_days: i64,
        batch_size: i64,
    ) -> Result<u64> {
        let result = sqlx::query(
            r#"
            WITH doomed AS (
                SELECT id
                FROM figurine_analytics_events
                WHERE event_date < (CURRENT_DATE - ($1::int * INTERVAL '1 day'))
                ORDER BY id
                LIMIT $2
            )
            DELETE FROM figurine_analytics_events e
            USING doomed
            WHERE e.id = doomed.id
            "#,
        )
        .bind(retention_days as i32)
        .bind(batch_size)
        .execute(&self.pg_pool)
        .await?;
        Ok(result.rows_affected())
    }

    // === ORDERS (Postgres) ===

    pub async fn save_order(
        &self,
        order: &crate::models::OrderRequest,
        user_id: Option<Uuid>,
    ) -> Result<crate::models::Order> {
        let rec = sqlx::query_as::<_, crate::models::Order>(
            "INSERT INTO orders (
                figurine_id, figurine_name, requester_name, requester_email,
                requester_phone, message, mode, user_id, reserve_status
             )
             VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                CASE WHEN $7 = 'reserve'::order_mode THEN 'requested'::reserve_status ELSE NULL END
             )
             RETURNING *",
        )
        .bind(&order.figurine_id)
        .bind(&order.figurine_name)
        .bind(&order.requester_name)
        .bind(&order.requester_email)
        .bind(&order.requester_phone)
        .bind(&order.message)
        .bind(&order.mode)
        .bind(user_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(rec)
    }

    /// Adopt any guest orders whose email matches a verified account but that were
    /// never tied to a user_id (submitted while logged out, or before order linking
    /// existed). Mirrors the email-based booking linking.
    pub async fn link_orders_to_user(&self, user_id: Uuid, email: &str) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE orders SET user_id = $1 WHERE user_id IS NULL AND lower(requester_email) = lower($2)"
        )
        .bind(user_id)
        .bind(email)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    /// Create or refresh a "notify me" subscription. Deduplicates by
    /// (figurine, email): a repeat request updates the existing row in place
    /// and keeps its token, rather than piling up duplicates.
    pub async fn upsert_notify_order(
        &self,
        order: &crate::models::OrderRequest,
        user_id: Option<Uuid>,
    ) -> Result<crate::models::Order> {
        let existing = sqlx::query_as::<_, crate::models::Order>(
            "SELECT * FROM orders WHERE figurine_id = $1 AND lower(requester_email) = lower($2) AND mode = 'notify'::order_mode LIMIT 1"
        )
        .bind(&order.figurine_id)
        .bind(&order.requester_email)
        .fetch_optional(&self.pg_pool).await?;

        if let Some(ex) = existing {
            Ok(sqlx::query_as::<_, crate::models::Order>(
                "UPDATE orders SET requester_name = $2, requester_phone = $3, message = $4, status = 'new'::order_status,
                        user_id = COALESCE($5, user_id)
                 WHERE id = $1 RETURNING *"
            )
            .bind(ex.id)
            .bind(&order.requester_name)
            .bind(&order.requester_phone)
            .bind(&order.message)
            .bind(user_id)
            .fetch_one(&self.pg_pool).await?)
        } else {
            let token = Self::generate_cancel_token();
            Ok(sqlx::query_as::<_, crate::models::Order>(
                "INSERT INTO orders (figurine_id, figurine_name, requester_name, requester_email, requester_phone, message, mode, cancel_token, user_id)
                 VALUES ($1, $2, $3, $4, $5, $6, 'notify'::order_mode, $7, $8) RETURNING *"
            )
            .bind(&order.figurine_id)
            .bind(&order.figurine_name)
            .bind(&order.requester_name)
            .bind(&order.requester_email)
            .bind(&order.requester_phone)
            .bind(&order.message)
            .bind(&token)
            .bind(user_id)
            .fetch_one(&self.pg_pool).await?)
        }
    }

    pub async fn get_order_by_cancel_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::Order>> {
        Ok(sqlx::query_as::<_, crate::models::Order>(
            "SELECT * FROM orders WHERE cancel_token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn delete_order_by_cancel_token(&self, token: &str) -> Result<()> {
        sqlx::query("DELETE FROM orders WHERE cancel_token = $1")
            .bind(token)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_orders_page(
        &self,
        status_filter: Option<&str>,
        mode_filter: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<crate::models::Order>, i64)> {
        let (items, total) = match (status_filter, mode_filter) {
            (Some(status), Some(mode)) => {
                let items = sqlx::query_as::<_, crate::models::Order>(
                    "SELECT * FROM orders
                     WHERE status = $1::order_status AND mode = $2::order_mode
                     ORDER BY created_at DESC LIMIT $3 OFFSET $4",
                )
                .bind(status)
                .bind(mode)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pg_pool)
                .await?;

                let (total,): (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM orders
                     WHERE status = $1::order_status AND mode = $2::order_mode",
                )
                .bind(status)
                .bind(mode)
                .fetch_one(&self.pg_pool)
                .await?;

                (items, total)
            }
            (Some(status), None) => {
                let items = sqlx::query_as::<_, crate::models::Order>(
                    "SELECT * FROM orders WHERE status = $1::order_status ORDER BY created_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(status).bind(limit).bind(offset)
                .fetch_all(&self.pg_pool).await?;

                let (total,): (i64,) =
                    sqlx::query_as("SELECT COUNT(*) FROM orders WHERE status = $1::order_status")
                        .bind(status)
                        .fetch_one(&self.pg_pool)
                        .await?;

                (items, total)
            }
            (None, Some(mode)) => {
                let items = sqlx::query_as::<_, crate::models::Order>(
                    "SELECT * FROM orders WHERE mode = $1::order_mode ORDER BY created_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(mode).bind(limit).bind(offset)
                .fetch_all(&self.pg_pool).await?;

                let (total,): (i64,) =
                    sqlx::query_as("SELECT COUNT(*) FROM orders WHERE mode = $1::order_mode")
                        .bind(mode)
                        .fetch_one(&self.pg_pool)
                        .await?;

                (items, total)
            }
            (None, None) => {
                let items = sqlx::query_as::<_, crate::models::Order>(
                    "SELECT * FROM orders ORDER BY created_at DESC LIMIT $1 OFFSET $2",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pg_pool)
                .await?;

                let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders")
                    .fetch_one(&self.pg_pool)
                    .await?;

                (items, total)
            }
        };
        Ok((items, total))
    }

    pub async fn get_new_orders_count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE status = 'new'")
            .fetch_one(&self.pg_pool)
            .await?;
        Ok(count)
    }

    pub async fn get_order_by_id(&self, id: uuid::Uuid) -> Result<Option<crate::models::Order>> {
        Ok(
            sqlx::query_as::<_, crate::models::Order>("SELECT * FROM orders WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pg_pool)
                .await?,
        )
    }

    pub async fn issue_order_certificate(
        &self,
        id: uuid::Uuid,
        token: &str,
        certificate_number: &str,
    ) -> Result<crate::models::Order> {
        let order = sqlx::query_as::<_, crate::models::Order>(
            "UPDATE orders
             SET certificate_token = COALESCE(certificate_token, $2),
                 certificate_number = COALESCE(certificate_number, $3),
                 certificate_issued_at = COALESCE(certificate_issued_at, NOW()),
                 certificate_revoked_at = NULL
             WHERE id = $1
               AND mode = 'reserve'::order_mode
               AND reserve_status = 'confirmed'::reserve_status
             RETURNING *",
        )
        .bind(id)
        .bind(token)
        .bind(certificate_number)
        .fetch_optional(&self.pg_pool)
        .await?;
        order.ok_or_else(|| {
            AppError::BadRequest(
                "Certificate can be issued only for confirmed reserve orders".to_string(),
            )
        })
    }

    pub async fn revoke_order_certificate(&self, id: uuid::Uuid) -> Result<crate::models::Order> {
        let order = sqlx::query_as::<_, crate::models::Order>(
            "UPDATE orders
             SET certificate_revoked_at = NOW()
             WHERE id = $1 AND certificate_token IS NOT NULL
             RETURNING *",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?;
        order.ok_or_else(|| AppError::NotFound("Certificate not found".to_string()))
    }

    pub async fn get_order_by_certificate_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::Order>> {
        Ok(sqlx::query_as::<_, crate::models::Order>(
            "SELECT * FROM orders WHERE certificate_token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn get_user_certificate_orders(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::Order>> {
        Ok(sqlx::query_as::<_, crate::models::Order>(
            "SELECT * FROM orders
             WHERE user_id = $1 AND certificate_token IS NOT NULL
             ORDER BY certificate_issued_at DESC NULLS LAST, created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn issue_commission_certificate(
        &self,
        id: uuid::Uuid,
        token: &str,
        certificate_number: &str,
    ) -> Result<crate::models::Commission> {
        let commission = sqlx::query_as::<_, crate::models::Commission>(
            "UPDATE commissions
             SET certificate_token = COALESCE(certificate_token, $2),
                 certificate_number = COALESCE(certificate_number, $3),
                 certificate_issued_at = COALESCE(certificate_issued_at, NOW()),
                 certificate_revoked_at = NULL
             WHERE id = $1
               AND status = 'completed'::commission_status
             RETURNING *",
        )
        .bind(id)
        .bind(token)
        .bind(certificate_number)
        .fetch_optional(&self.pg_pool)
        .await?;
        commission.ok_or_else(|| {
            AppError::BadRequest(
                "Certificate can be issued only for completed commissions".to_string(),
            )
        })
    }

    pub async fn revoke_commission_certificate(
        &self,
        id: uuid::Uuid,
    ) -> Result<crate::models::Commission> {
        let commission = sqlx::query_as::<_, crate::models::Commission>(
            "UPDATE commissions
             SET certificate_revoked_at = NOW()
             WHERE id = $1 AND certificate_token IS NOT NULL
             RETURNING *",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?;
        commission.ok_or_else(|| AppError::NotFound("Certificate not found".to_string()))
    }

    pub async fn get_commission_by_certificate_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::Commission>> {
        Ok(sqlx::query_as::<_, crate::models::Commission>(
            "SELECT * FROM commissions WHERE certificate_token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn get_user_certificate_commissions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::Commission>> {
        Ok(sqlx::query_as::<_, crate::models::Commission>(
            "SELECT * FROM commissions
             WHERE user_id = $1 AND certificate_token IS NOT NULL
             ORDER BY certificate_issued_at DESC NULLS LAST, created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// All "notify me" orders left for a figurine — used to alert the author
    /// (personally) when the work becomes available again.
    pub async fn get_notify_orders_for_figurine(
        &self,
        figurine_id: uuid::Uuid,
    ) -> Result<Vec<crate::models::Order>> {
        Ok(sqlx::query_as::<_, crate::models::Order>(
            "SELECT * FROM orders WHERE figurine_id = $1 AND mode = 'notify'::order_mode ORDER BY created_at ASC"
        )
        .bind(figurine_id.to_string())
        .fetch_all(&self.pg_pool).await?)
    }

    pub async fn update_order_status(
        &self,
        id: uuid::Uuid,
        status: &crate::models::OrderStatus,
        admin_notes: Option<&str>,
        reserve_status: Option<&crate::models::ReserveStatus>,
        reserve_expires_at: Option<&str>,
        admin_terms_note: Option<&str>,
        invoice_note: Option<&str>,
    ) -> Result<()> {
        let reserve_expires_at_provided = reserve_expires_at.is_some();
        let reserve_expires_at = parse_optional_deadline(reserve_expires_at)?;
        let affected = sqlx::query(
            "UPDATE orders
             SET status = $1,
                 admin_notes = COALESCE($2, admin_notes),
                 reserve_status = COALESCE($3::reserve_status, reserve_status),
                 reserve_expires_at = CASE WHEN $4 THEN $5 ELSE reserve_expires_at END,
                 admin_terms_note = COALESCE($6, admin_terms_note),
                 invoice_note = COALESCE($7, invoice_note)
             WHERE id = $8",
        )
        .bind(status)
        .bind(admin_notes)
        .bind(reserve_status)
        .bind(reserve_expires_at_provided)
        .bind(reserve_expires_at)
        .bind(admin_terms_note)
        .bind(invoice_note)
        .bind(id)
        .execute(&self.pg_pool)
        .await?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::NotFound(format!("Order {} not found", id)));
        }
        Ok(())
    }

    // === CONTENT (Postgres) ===

    /// Returns `(items, total_count)`. When `q.per_page` is None — no LIMIT/OFFSET,
    /// all matching rows are returned (used by sitemap and admin calls).
    pub async fn get_all_figurines(
        &self,
        visible_only: bool,
        q: &crate::models::FigurineQuery,
    ) -> Result<(Vec<Figurine>, i64)> {
        // Clone filter values upfront so binds own their data ('static).
        let status = q.status.clone();
        let search = q.search.clone();

        let mut count_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM figurines WHERE 1=1");
        if visible_only {
            count_builder.push(" AND is_visible = true");
            // Hold first-look works out of the public archive until their hour.
            count_builder.push(" AND (first_look_until IS NULL OR first_look_until <= NOW())");
        }
        if let Some(ref s) = status {
            count_builder
                .push(" AND status::text = ")
                .push_bind(s.clone());
        }
        if let Some(ref s) = search {
            count_builder
                .push(" AND name ILIKE '%' || ")
                .push_bind(s.clone())
                .push(" || '%'");
        }
        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&self.pg_pool)
            .await?;

        // Every ordering ends in `id` — a total order, so the sort is deterministic.
        //
        // Without it these clauses only PARTIALLY order the rows: `sort_order` is NOT NULL
        // DEFAULT 0, so most of a young collection ties on it, and `created_at`/`name` can
        // tie too. Postgres is then free to return tied rows in any order it likes, and it
        // does change its mind — an UPDATE rewrites the tuple and moves it in a seq scan. On
        // an unpaginated read that only reshuffles equals; with LIMIT/OFFSET it decides WHICH
        // rows you get, so two identical requests can return overlapping-but-different pages,
        // and a work can sit in the gap between page 1 and page 2 forever.
        let order_clause = match q.sort.as_deref() {
            Some("newest") => " ORDER BY created_at DESC, id",
            Some("oldest") => " ORDER BY created_at ASC, id",
            Some("name") => " ORDER BY name ASC, id",
            _ => " ORDER BY sort_order, created_at DESC, id",
        };

        let mut items_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT * FROM figurines WHERE 1=1");
        if visible_only {
            items_builder.push(" AND is_visible = true");
            items_builder.push(" AND (first_look_until IS NULL OR first_look_until <= NOW())");
        }
        if let Some(s) = status {
            items_builder.push(" AND status::text = ").push_bind(s);
        }
        if let Some(s) = search {
            items_builder
                .push(" AND name ILIKE '%' || ")
                .push_bind(s)
                .push(" || '%'");
        }
        items_builder.push(order_clause);
        if let Some(per_page) = q.per_page {
            let page = q.page.unwrap_or(1).max(1);
            let offset = (page - 1) * per_page;
            items_builder.push(" LIMIT ").push_bind(per_page);
            items_builder.push(" OFFSET ").push_bind(offset);
        }
        let figurines = items_builder
            .build_query_as::<Figurine>()
            .fetch_all(&self.pg_pool)
            .await?;

        Ok((figurines, total))
    }

    /// Admin-only Pinterest SEO copy, batched by id — keyed by id string so
    /// `feed_rss` can look items up straight against `FigurineListItemDto.id`.
    /// Only non-blank values are returned; feed_rss falls back to its own
    /// composed description for everything else.
    pub async fn get_pinterest_descriptions(
        &self,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<String, String>> {
        let rows: Vec<(Uuid, Option<String>)> =
            sqlx::query_as("SELECT id, pinterest_description FROM figurines WHERE id = ANY($1)")
                .bind(ids)
                .fetch_all(&self.pg_pool)
                .await?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, d)| {
                let d = d?.trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some((id.to_string(), d))
                }
            })
            .collect())
    }

    pub async fn get_figurine_by_id(&self, id: Uuid) -> Result<Option<Figurine>> {
        let figurine = sqlx::query_as::<_, Figurine>("SELECT * FROM figurines WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pg_pool)
            .await?;
        Ok(figurine)
    }

    /// Resolve a work by its transliterated slug (the pretty-URL handle).
    pub async fn get_figurine_by_slug(&self, slug: &str) -> Result<Option<Figurine>> {
        let figurine = sqlx::query_as::<_, Figurine>("SELECT * FROM figurines WHERE slug = $1")
            .bind(slug)
            .fetch_optional(&self.pg_pool)
            .await?;
        Ok(figurine)
    }

    /// Every work still missing a URL slug (NULL or blank), for slug backfill.
    /// Same ordering as the default archive so backfilled suffixes are stable.
    pub async fn get_figurines_without_slug(&self) -> Result<Vec<Figurine>> {
        Ok(sqlx::query_as::<_, Figurine>(
            "SELECT * FROM figurines
             WHERE slug IS NULL OR slug = ''
             ORDER BY sort_order, created_at DESC, id",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Overwrite a single work's URL slug and its manual/auto flag. Bumps
    /// updated_at so the sitemap reflects the new address. The caller guarantees
    /// uniqueness; the partial UNIQUE(slug) index is the backstop.
    pub async fn update_figurine_slug(
        &self,
        id: Uuid,
        slug: &str,
        slug_manual: bool,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE figurines SET slug = $1, slug_manual = $2, updated_at = NOW() WHERE id = $3",
        )
        .bind(slug)
        .bind(slug_manual)
        .bind(id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Works currently inside their "first look" window — visible, with a release
    /// time still in the future. Soonest to open first. These are deliberately
    /// excluded from the public archive (see `get_all_figurines`) and surfaced
    /// only on the book-holders' shelf.
    pub async fn get_first_look_figurines(&self) -> Result<Vec<Figurine>> {
        Ok(sqlx::query_as::<_, Figurine>(
            "SELECT * FROM figurines
             WHERE is_visible = true
               AND first_look_until IS NOT NULL
               AND first_look_until > NOW()
             ORDER BY first_look_until ASC",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Batch fetch by id, visible pieces only, in no particular order — callers
    /// that need a specific order (e.g. pinned-first) reorder client-side.
    pub async fn get_figurines_by_ids(&self, ids: &[Uuid]) -> Result<Vec<Figurine>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(sqlx::query_as::<_, Figurine>(
            "SELECT * FROM figurines WHERE id = ANY($1) AND is_visible = true",
        )
        .bind(ids)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_images_by_figurine(&self, figurine_id: Uuid) -> Result<Vec<Image>> {
        let images = sqlx::query_as::<_, Image>(
            "SELECT * FROM images WHERE figurine_id = $1 ORDER BY sort_order",
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(images)
    }

    /// Batch-load the face image for many figurines at once (avoids the N+1 of
    /// querying images per figurine when building list views). One row per
    /// figurine — the lowest-sort_order face image.
    pub async fn get_face_images_for_figurines(
        &self,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Image>> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows = sqlx::query_as::<_, Image>(
            "SELECT DISTINCT ON (figurine_id) * FROM images
             WHERE figurine_id = ANY($1) AND image_type = 'face'
             ORDER BY figurine_id, sort_order",
        )
        .bind(ids)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows.into_iter().map(|i| (i.figurine_id, i)).collect())
    }

    /// Batch-load one "detail" (second-angle) image per figurine, for the
    /// home gallery's hover reveal. Same batching rationale as
    /// get_face_images_for_figurines; a figurine with no detail image is
    /// simply absent from the map.
    pub async fn get_detail_images_for_figurines(
        &self,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Image>> {
        if ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows = sqlx::query_as::<_, Image>(
            "SELECT DISTINCT ON (figurine_id) * FROM images
             WHERE figurine_id = ANY($1) AND image_type = 'detail'
             ORDER BY figurine_id, sort_order",
        )
        .bind(ids)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows.into_iter().map(|i| (i.figurine_id, i)).collect())
    }

    pub async fn get_steps_by_figurine(&self, figurine_id: Uuid) -> Result<Vec<ProcessStep>> {
        let steps = sqlx::query_as::<_, ProcessStep>(
            "SELECT * FROM process_steps WHERE figurine_id = $1 ORDER BY sort_order",
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(steps)
    }

    pub async fn get_related_figurines(&self, current_id: Uuid) -> Result<Vec<Figurine>> {
        let current = match self.get_figurine_by_id(current_id).await? {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        // Take the first 4 *characters* (not bytes) so non-ASCII material names
        // (e.g. Cyrillic) can't panic on a mid-codepoint byte slice.
        let material_hint: String = current
            .material
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(4)
            .collect();

        let related = sqlx::query_as::<_, Figurine>(
            "SELECT * FROM figurines
             WHERE id != $1
             AND is_visible = true
             AND (
                 year = $2
                 OR ($3 != '' AND material LIKE '%' || $4 || '%')
             )
             ORDER BY RANDOM()
             LIMIT 3",
        )
        .bind(current_id)
        .bind(current.year)
        .bind(&material_hint)
        .bind(&material_hint)
        .fetch_all(&self.pg_pool)
        .await?;

        Ok(related)
    }

    pub async fn get_texts_by_category(&self, category: TextCategory) -> Result<Vec<Text>> {
        let texts = sqlx::query_as::<_, Text>(
            "SELECT * FROM texts WHERE category = $1 ORDER BY sort_order",
        )
        .bind(category)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(texts)
    }

    // === ADMIN WRITE OPERATIONS ===

    /// Upsert a figurine together with its full image and step sets, atomically.
    /// All IDs are parsed up-front so a bad UUID aborts before any write, and the
    /// delete+insert of images/steps runs in one transaction — a figurine can
    /// never be left with half its media.
    pub async fn save_figurine_full(
        &self,
        f: &crate::models::SaveFigurineRequest,
        images: &[crate::models::SaveImageRequest],
        steps: &[crate::models::SaveStepRequest],
        slug_manual: bool,
    ) -> Result<()> {
        let id = Uuid::parse_str(&f.id)
            .map_err(|_| AppError::BadRequest("Invalid figurine ID".to_string()))?;

        // Parse all child IDs first — fail before mutating anything.
        let image_rows: Vec<(Uuid, &crate::models::SaveImageRequest, i32)> = images
            .iter()
            .enumerate()
            .map(|(idx, img)| {
                let img_id = Uuid::parse_str(&img.id)
                    .map_err(|_| AppError::BadRequest("Invalid image ID".to_string()))?;
                Ok((img_id, img, img.sort_order.unwrap_or(idx as i32)))
            })
            .collect::<Result<_>>()?;
        let step_rows: Vec<(Uuid, &crate::models::SaveStepRequest, i32)> = steps
            .iter()
            .enumerate()
            .map(|(idx, step)| {
                let step_id = Uuid::parse_str(&step.id)
                    .map_err(|_| AppError::BadRequest("Invalid step ID".to_string()))?;
                Ok((step_id, step, step.sort_order.unwrap_or(idx as i32)))
            })
            .collect::<Result<_>>()?;

        // Lenient parse: a bad/empty room id frees the work (NULL → always open)
        // rather than aborting the save.
        let showing_room_uuid = f
            .showing_room_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok());

        // Strict parse: a malformed first-look date aborts the save (it gates
        // archive visibility, so a silent NULL could expose a work early).
        let first_look_until = match f
            .first_look_until
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(s) => Some(
                chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|_| {
                        AppError::BadRequest("Invalid first_look_until (expected ISO-8601)".into())
                    })?
                    .with_timezone(&chrono::Utc),
            ),
            None => None,
        };

        let mut tx = self.pg_pool.begin().await?;

        sqlx::query(
            "INSERT INTO figurines (id, name, short_text, full_description, dimensions, material, technique, series, year, passport_number, edition, created_period, care_instructions, provenance_note, authenticity_note, included_items, ambience_path, video_url, secret_text, is_visible, is_featured, status, sort_order, open_from_min, open_until_min, sealed_door_image, showing_room_id, display_layout, display_config, catalog_lists, first_look_until, slug, slug_manual, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, NOW())
             ON CONFLICT (id) DO UPDATE SET
               name=EXCLUDED.name, slug=EXCLUDED.slug, slug_manual=EXCLUDED.slug_manual, short_text=EXCLUDED.short_text, full_description=EXCLUDED.full_description,
               dimensions=EXCLUDED.dimensions, material=EXCLUDED.material, technique=EXCLUDED.technique, series=EXCLUDED.series,
               passport_number=EXCLUDED.passport_number, edition=EXCLUDED.edition, created_period=EXCLUDED.created_period,
               care_instructions=EXCLUDED.care_instructions, provenance_note=EXCLUDED.provenance_note,
               authenticity_note=EXCLUDED.authenticity_note, included_items=EXCLUDED.included_items,
               year=EXCLUDED.year, ambience_path=EXCLUDED.ambience_path, video_url=EXCLUDED.video_url,
               secret_text=EXCLUDED.secret_text, is_visible=EXCLUDED.is_visible, is_featured=EXCLUDED.is_featured,
               status=EXCLUDED.status, sort_order=EXCLUDED.sort_order,
               open_from_min=EXCLUDED.open_from_min, open_until_min=EXCLUDED.open_until_min,
               sealed_door_image=EXCLUDED.sealed_door_image,
               showing_room_id=EXCLUDED.showing_room_id,
               display_layout=EXCLUDED.display_layout, display_config=EXCLUDED.display_config,
               catalog_lists=EXCLUDED.catalog_lists,
               first_look_until=EXCLUDED.first_look_until, updated_at=NOW()"
        )
        .bind(id).bind(&f.name).bind(&f.short_text).bind(&f.full_description)
        .bind(&f.dimensions).bind(&f.material).bind(&f.technique).bind(&f.series).bind(f.year)
        .bind(&f.passport_number).bind(&f.edition).bind(&f.created_period)
        .bind(&f.care_instructions).bind(&f.provenance_note).bind(&f.authenticity_note)
        .bind(&f.included_items)
        .bind(&f.ambience_path).bind(&f.video_url).bind(&f.secret_text)
        .bind(f.is_visible).bind(f.is_featured).bind(&f.status).bind(f.sort_order)
        .bind(f.open_from_min).bind(f.open_until_min).bind(&f.sealed_door_image)
        .bind(showing_room_uuid).bind(&f.display_layout).bind(&f.display_config)
        .bind(&f.catalog_lists)
        .bind(first_look_until).bind(&f.slug).bind(slug_manual)
        .execute(&mut *tx).await?;

        sqlx::query("DELETE FROM images WHERE figurine_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for (img_id, img, sort) in &image_rows {
            sqlx::query(
                "INSERT INTO images (id, figurine_id, image_type, file_path, original_path, thumb_path, depth_path, parallax_intensity, focal_x, focal_y, reveal_radius, darkness, alt_text, sort_order) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"
            )
            .bind(img_id).bind(id).bind(&img.image_type)
            .bind(&img.url).bind(&img.original_url).bind(&img.thumb_url)
            .bind(&img.depth_url)
            .bind(img.parallax_intensity)
            .bind(img.focal_x).bind(img.focal_y).bind(img.reveal_radius)
            .bind(img.darkness)
            .bind(&img.alt_text).bind(*sort)
            .execute(&mut *tx).await?;
        }

        sqlx::query("DELETE FROM process_steps WHERE figurine_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for (step_id, step, sort) in &step_rows {
            sqlx::query(
                "INSERT INTO process_steps (id, figurine_id, step_type, description, image_path, sort_order) VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(step_id).bind(id).bind(&step.step_type)
            .bind(&step.description).bind(&step.image_url).bind(*sort)
            .execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_figurine(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM figurines WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn delete_analytics_events_by_figurine(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM figurine_analytics_events WHERE figurine_id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_showing_rooms(&self) -> Result<Vec<crate::models::ShowingRoom>> {
        let rooms = sqlx::query_as::<_, crate::models::ShowingRoom>(
            "SELECT * FROM showing_rooms ORDER BY sort_order, name",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rooms)
    }

    pub async fn upsert_showing_room(
        &self,
        r: &crate::models::SaveShowingRoomRequest,
        sort_order: i32,
    ) -> Result<()> {
        let id = Uuid::parse_str(&r.id)
            .map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?;
        sqlx::query(
            "INSERT INTO showing_rooms (id, name, open_from_min, open_until_min, open_days_mask, open_month_day, open_date_from, open_date_until, sort_order, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
             ON CONFLICT (id) DO UPDATE SET
               name=EXCLUDED.name, open_from_min=EXCLUDED.open_from_min,
               open_until_min=EXCLUDED.open_until_min, open_days_mask=EXCLUDED.open_days_mask,
               open_month_day=EXCLUDED.open_month_day, open_date_from=EXCLUDED.open_date_from,
               open_date_until=EXCLUDED.open_date_until, sort_order=EXCLUDED.sort_order, updated_at=NOW()",
        )
        .bind(id)
        .bind(&r.name)
        .bind(r.open_from_min)
        .bind(r.open_until_min)
        .bind(r.open_days_mask)
        .bind(&r.open_month_day)
        .bind(&r.open_date_from)
        .bind(&r.open_date_until)
        .bind(sort_order)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn delete_showing_room(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM showing_rooms WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_showing_room_count(&self) -> Result<i32> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM showing_rooms")
            .fetch_one(&self.pg_pool)
            .await?;
        Ok(row.0 as i32)
    }

    pub async fn upsert_text(
        &self,
        t: &crate::models::SaveTextRequest,
        category: &crate::models::TextCategory,
    ) -> Result<()> {
        let id = Uuid::parse_str(&t.id)
            .map_err(|_| AppError::BadRequest("Invalid text ID".to_string()))?;
        sqlx::query(
            "INSERT INTO texts (id, category, content, caption, image_path, sort_order, updated_at)
             VALUES ($1, $2, $3, $4, $5, COALESCE((SELECT sort_order FROM texts WHERE id = $6), (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM texts WHERE category = $7)), NOW())
             ON CONFLICT (id) DO UPDATE SET
               content=EXCLUDED.content, caption=EXCLUDED.caption,
               image_path=EXCLUDED.image_path, updated_at=NOW()"
        )
        .bind(id).bind(category).bind(&t.content).bind(&t.caption)
        .bind(&t.image_url).bind(id).bind(category)
        .execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn delete_text(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM texts WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_main_background(&self) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT file_path FROM app_resources WHERE key = 'main_background'")
                .fetch_optional(&self.pg_pool)
                .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set_main_background(&self, url: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO app_resources (key, file_path, updated_at) VALUES ('main_background', $1, NOW())
             ON CONFLICT (key) DO UPDATE SET file_path=EXCLUDED.file_path, updated_at=NOW()"
        )
        .bind(url).execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn get_home_content(&self) -> Result<Option<crate::models::HomeContent>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT file_path FROM app_resources WHERE key = 'home_content'")
                .fetch_optional(&self.pg_pool)
                .await?;
        match row {
            None => Ok(None),
            Some((json,)) => {
                let content = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Corrupt home_content JSON: {e}")))?;
                Ok(Some(content))
            }
        }
    }

    pub async fn save_home_content(&self, content: &crate::models::HomeContent) -> Result<()> {
        let json = serde_json::to_string(content).map_err(|e| AppError::Internal(e.to_string()))?;
        sqlx::query(
            "INSERT INTO app_resources (key, file_path, updated_at) VALUES ('home_content', $1, NOW())
             ON CONFLICT (key) DO UPDATE SET file_path=EXCLUDED.file_path, updated_at=NOW()"
        )
        .bind(json).execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn get_author_profile(&self) -> Result<Option<crate::models::AuthorProfile>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT file_path FROM app_resources WHERE key = 'author_profile'")
                .fetch_optional(&self.pg_pool)
                .await?;
        match row {
            None => Ok(None),
            Some((json,)) => {
                let profile = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Corrupt author_profile JSON: {e}")))?;
                Ok(Some(profile))
            }
        }
    }

    pub async fn save_author_profile(&self, profile: &crate::models::AuthorProfile) -> Result<()> {
        let json = serde_json::to_string(profile).map_err(|e| AppError::Internal(e.to_string()))?;
        sqlx::query(
            "INSERT INTO app_resources (key, file_path, updated_at) VALUES ('author_profile', $1, NOW())
             ON CONFLICT (key) DO UPDATE SET file_path=EXCLUDED.file_path, updated_at=NOW()"
        )
        .bind(json).execute(&self.pg_pool).await?;
        Ok(())
    }

    // === MEDIA ===

    pub async fn get_media_usages(&self) -> Result<Vec<MediaUsageDto>> {
        let mut usages = Vec::new();

        let image_rows: Vec<(
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT i.id::text, i.file_path, i.original_path, i.thumb_path, f.id::text, f.name
             FROM images i
             LEFT JOIN figurines f ON f.id = i.figurine_id",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        for (image_id, preview, original, thumb, fig_id, fig_name) in image_rows {
            let label = format!(
                "Image for {}",
                fig_name.unwrap_or_else(|| "Unknown figurine".to_string())
            );
            let entity_id = fig_id.unwrap_or_else(|| image_id.clone());
            usages.push(MediaUsageDto {
                path: preview,
                label: label.clone(),
                entity_type: "figurineImage".to_string(),
                entity_id: entity_id.clone(),
                field: "preview".to_string(),
            });
            if let Some(path) = original {
                usages.push(MediaUsageDto {
                    path,
                    label: label.clone(),
                    entity_type: "figurineImage".to_string(),
                    entity_id: entity_id.clone(),
                    field: "original".to_string(),
                });
            }
            if let Some(path) = thumb {
                usages.push(MediaUsageDto {
                    path,
                    label: label.clone(),
                    entity_type: "figurineImage".to_string(),
                    entity_id: entity_id.clone(),
                    field: "thumb".to_string(),
                });
            }
        }

        let step_rows: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT ps.id::text, ps.image_path, f.id::text, f.name
             FROM process_steps ps
             LEFT JOIN figurines f ON f.id = ps.figurine_id",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        for (step_id, path, fig_id, fig_name) in step_rows {
            usages.push(MediaUsageDto {
                path,
                label: format!(
                    "Process step for {}",
                    fig_name.unwrap_or_else(|| "Unknown figurine".to_string())
                ),
                entity_type: "processStep".to_string(),
                entity_id: fig_id.unwrap_or(step_id),
                field: "image".to_string(),
            });
        }

        let text_rows: Vec<(String, String, Option<String>, String)> = sqlx::query_as(
            "SELECT id::text, category::text, caption, image_path FROM texts WHERE image_path IS NOT NULL"
        ).fetch_all(&self.pg_pool).await?;
        for (id, category, caption, path) in text_rows {
            usages.push(MediaUsageDto {
                path,
                label: caption.unwrap_or_else(|| format!("{} text", category)),
                entity_type: "text".to_string(),
                entity_id: id,
                field: "image".to_string(),
            });
        }

        let figurine_rows: Vec<(String, String, Option<String>, Option<String>)> =
            sqlx::query_as("SELECT id::text, name, ambience_path, video_url FROM figurines")
                .fetch_all(&self.pg_pool)
                .await?;
        for (id, name, ambience, video) in figurine_rows {
            if let Some(path) = ambience {
                usages.push(MediaUsageDto {
                    path,
                    label: format!("Audio for {}", name),
                    entity_type: "figurine".to_string(),
                    entity_id: id.clone(),
                    field: "ambience".to_string(),
                });
            }
            if let Some(path) = video {
                usages.push(MediaUsageDto {
                    path,
                    label: format!("Video for {}", name),
                    entity_type: "figurine".to_string(),
                    entity_id: id.clone(),
                    field: "video".to_string(),
                });
            }
        }

        let resource_rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, file_path FROM app_resources WHERE key NOT IN ('author_profile', 'home_content')"
        ).fetch_all(&self.pg_pool).await?;
        for (key, path) in resource_rows {
            usages.push(MediaUsageDto {
                path,
                label: format!("App resource {}", key),
                entity_type: "appResource".to_string(),
                entity_id: key,
                field: "file".to_string(),
            });
        }

        Ok(usages)
    }

    // === SHOWINGS ===

    pub async fn get_all_showings(&self) -> Result<Vec<Showing>> {
        let rows =
            sqlx::query_as::<_, Showing>("SELECT * FROM figurine_showings ORDER BY starts_at DESC")
                .fetch_all(&self.pg_pool)
                .await?;
        Ok(rows)
    }

    pub async fn upsert_showing(&self, req: &crate::models::SaveShowingRequest) -> Result<Uuid> {
        let id = match &req.id {
            Some(s) => Uuid::parse_str(s)
                .map_err(|_| AppError::BadRequest("Invalid showing ID".to_string()))?,
            None => Uuid::new_v4(),
        };
        let figurine_id = Uuid::parse_str(&req.figurine_id)
            .map_err(|_| AppError::BadRequest("Invalid figurine ID".to_string()))?;
        let starts_at = chrono::NaiveDate::parse_from_str(&req.starts_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid starts_at date".to_string()))?;
        let ends_at = chrono::NaiveDate::parse_from_str(&req.ends_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid ends_at date".to_string()))?;

        sqlx::query(
            "INSERT INTO figurine_showings (id, figurine_id, title, showing_type, starts_at, ends_at, venue, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
               figurine_id=EXCLUDED.figurine_id, title=EXCLUDED.title,
               showing_type=EXCLUDED.showing_type, starts_at=EXCLUDED.starts_at,
               ends_at=EXCLUDED.ends_at, venue=EXCLUDED.venue, notes=EXCLUDED.notes"
        )
        .bind(id).bind(figurine_id).bind(&req.title).bind(&req.showing_type)
        .bind(starts_at).bind(ends_at).bind(&req.venue).bind(&req.notes)
        .execute(&self.pg_pool).await?;
        Ok(id)
    }

    pub async fn delete_showing(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM figurine_showings WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_figurine_schedule(
        &self,
        figurine_id: Uuid,
    ) -> Result<(Vec<Showing>, Vec<Booking>, Vec<Booking>)> {
        let today = chrono::Utc::now().date_naive();
        let showings = sqlx::query_as::<_, Showing>(
            "SELECT * FROM figurine_showings WHERE figurine_id = $1 AND ends_at >= $2 ORDER BY starts_at"
        )
        .bind(figurine_id).bind(today)
        .fetch_all(&self.pg_pool).await?;

        let confirmed = sqlx::query_as::<_, Booking>(
            "SELECT * FROM figurine_bookings WHERE figurine_id = $1 AND status = 'confirmed' AND ends_at >= $2 ORDER BY starts_at"
        )
        .bind(figurine_id).bind(today)
        .fetch_all(&self.pg_pool).await?;

        let pending = sqlx::query_as::<_, Booking>(
            "SELECT * FROM figurine_bookings WHERE figurine_id = $1 AND status = 'pending' AND ends_at >= $2 ORDER BY starts_at"
        )
        .bind(figurine_id).bind(today)
        .fetch_all(&self.pg_pool).await?;

        Ok((showings, confirmed, pending))
    }

    // === BOOKINGS ===

    /// Per-figurine transaction lock key, derived from the figurine UUID. Used to
    /// serialise conflict-check + insert/confirm so two concurrent requests can't
    /// both pass the check and double-book the same dates.
    fn booking_lock_key(figurine_id: Uuid) -> i64 {
        let b = figurine_id.as_bytes();
        i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    }

    /// Like mutations on one figurine serialize against each other, not against
    /// bookings — reverse the UUID bytes so the two lock namespaces stay apart.
    fn like_lock_key(figurine_id: Uuid) -> i64 {
        let b = figurine_id.as_bytes();
        i64::from_le_bytes([b[7], b[6], b[5], b[4], b[3], b[2], b[1], b[0]])
    }

    /// Create a pending booking atomically: acquire the per-figurine advisory
    /// lock, re-check conflicts and insert inside one transaction. Returns
    /// `Ok(None)` if the dates conflict (caller maps to 409).
    pub async fn create_booking_atomic(
        &self,
        req: &crate::models::CreateBookingRequest,
        starts_at: chrono::NaiveDate,
        ends_at: chrono::NaiveDate,
        user_id: Option<Uuid>,
    ) -> Result<Option<Booking>> {
        let figurine_id = Uuid::parse_str(&req.figurine_id)
            .map_err(|_| AppError::BadRequest("Invalid figurine ID".to_string()))?;

        let mut tx = self.pg_pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(Self::booking_lock_key(figurine_id))
            .execute(&mut *tx)
            .await?;

        let (showing_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_showings WHERE figurine_id = $1 AND starts_at <= $3 AND ends_at >= $2)"
        )
        .bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&mut *tx).await?;
        if showing_conflict {
            return Ok(None);
        }

        let (booking_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_bookings WHERE figurine_id = $1 AND status = 'confirmed' AND starts_at <= $3 AND ends_at >= $2)"
        )
        .bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&mut *tx).await?;
        if booking_conflict {
            return Ok(None);
        }

        let cancel_token = Self::generate_cancel_token();
        let rec = sqlx::query_as::<_, Booking>(
            "INSERT INTO figurine_bookings (figurine_id, figurine_name, requester_name, requester_email, requester_phone, purpose, display_type, venue, starts_at, ends_at, cancel_token, user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING *"
        )
        .bind(figurine_id).bind(&req.figurine_name).bind(&req.requester_name)
        .bind(&req.requester_email).bind(&req.requester_phone).bind(&req.purpose)
        .bind(&req.display_type).bind(&req.venue)
        .bind(starts_at).bind(ends_at).bind(&cancel_token).bind(user_id)
        .fetch_one(&mut *tx).await?;

        tx.commit().await?;
        Ok(Some(rec))
    }

    /// Receipt token for anonymous cancellation. 16 hex chars (~64 bits of
    /// entropy) grouped for readability — far beyond brute-forceable, unlike the
    /// old 8-char (~32-bit) token.
    fn generate_cancel_token() -> String {
        let raw = Uuid::new_v4().to_string().replace('-', "").to_uppercase();
        format!(
            "{}-{}-{}-{}",
            &raw[..4],
            &raw[4..8],
            &raw[8..12],
            &raw[12..16]
        )
    }

    pub async fn get_booking_by_cancel_token(&self, token: &str) -> Result<Option<Booking>> {
        Ok(
            sqlx::query_as::<_, Booking>("SELECT * FROM figurine_bookings WHERE cancel_token = $1")
                .bind(token)
                .fetch_optional(&self.pg_pool)
                .await?,
        )
    }

    pub async fn get_bookings_by_cancel_tokens(&self, tokens: &[String]) -> Result<Vec<Booking>> {
        Ok(sqlx::query_as::<_, Booking>(
            "SELECT * FROM figurine_bookings WHERE cancel_token = ANY($1)",
        )
        .bind(tokens)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn cancel_booking_by_token(&self, token: &str) -> Result<Option<Booking>> {
        Ok(sqlx::query_as::<_, Booking>(
            "UPDATE figurine_bookings SET status = 'cancelled'
             WHERE cancel_token = $1 AND status = 'pending'
             RETURNING *",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Set or clear a visitor's wax-seal mark on a figurine. `tone: None`
    /// deletes the row; `Some(t)` upserts it. Explicit-set rather than a
    /// stateful toggle — the client already tracks its own local tone, so it
    /// tells the server the target state directly, which makes a retry or a
    /// duplicate submit naturally idempotent instead of flipping state twice.
    pub async fn set_figurine_mark(
        &self,
        figurine_id: Uuid,
        visitor_token: &str,
        tone: Option<&str>,
    ) -> Result<()> {
        match tone {
            Some(t) => {
                sqlx::query(
                    "INSERT INTO figurine_marks (figurine_id, visitor_token, tone)
                     VALUES ($1, $2, $3)
                     ON CONFLICT (figurine_id, visitor_token)
                     DO UPDATE SET tone = EXCLUDED.tone, created_at = NOW()",
                )
                .bind(figurine_id)
                .bind(visitor_token)
                .bind(t)
                .execute(&self.pg_pool)
                .await?;
            }
            None => {
                sqlx::query(
                    "DELETE FROM figurine_marks WHERE figurine_id = $1 AND visitor_token = $2",
                )
                .bind(figurine_id)
                .bind(visitor_token)
                .execute(&self.pg_pool)
                .await?;
            }
        }
        Ok(())
    }

    /// Explicit-set a heart like. Returns (this visitor/account now likes, distinct like count).
    ///
    /// One transaction, one row per person: a logged-in like absorbs the guest
    /// row for this token (and any other device row for this account) so the
    /// unique visitor/user indexes cannot 500 or double-count. Unlike always
    /// clears `users.wishlist` for whoever owned the deleted row — including
    /// a logged-out tap on a token that was created while signed in.
    pub async fn set_figurine_like(
        &self,
        figurine_id: Uuid,
        visitor_token: &str,
        user_id: Option<Uuid>,
        liked: bool,
    ) -> Result<(bool, i64)> {
        let fig_key = figurine_id.to_string();
        let mut tx = self.pg_pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(Self::like_lock_key(figurine_id))
            .execute(&mut *tx)
            .await?;

        if liked {
            if let Some(uid) = user_id {
                sqlx::query(
                    "DELETE FROM figurine_likes
                     WHERE figurine_id = $1
                       AND (visitor_token = $2 OR user_id = $3)",
                )
                .bind(figurine_id)
                .bind(visitor_token)
                .bind(uid)
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "INSERT INTO figurine_likes (figurine_id, visitor_token, user_id)
                     VALUES ($1, $2, $3)",
                )
                .bind(figurine_id)
                .bind(visitor_token)
                .bind(uid)
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "UPDATE users
                     SET wishlist = (
                         SELECT ARRAY(SELECT DISTINCT x FROM unnest(array_append(wishlist, $1::text)) AS x)
                     )
                     WHERE id = $2",
                )
                .bind(&fig_key)
                .bind(uid)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    "INSERT INTO figurine_likes (figurine_id, visitor_token, user_id)
                     VALUES ($1, $2, NULL)
                     ON CONFLICT (figurine_id, visitor_token) DO NOTHING",
                )
                .bind(figurine_id)
                .bind(visitor_token)
                .execute(&mut *tx)
                .await?;
            }
        } else {
            let mut owner_ids: Vec<Uuid> = sqlx::query_scalar(
                "SELECT DISTINCT user_id FROM figurine_likes
                 WHERE figurine_id = $1
                   AND user_id IS NOT NULL
                   AND (visitor_token = $2 OR user_id = $3)",
            )
            .bind(figurine_id)
            .bind(visitor_token)
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await?;
            if let Some(uid) = user_id {
                if !owner_ids.contains(&uid) {
                    owner_ids.push(uid);
                }
            }

            sqlx::query(
                "DELETE FROM figurine_likes
                 WHERE figurine_id = $1
                   AND (visitor_token = $2 OR user_id = $3)",
            )
            .bind(figurine_id)
            .bind(visitor_token)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

            for uid in owner_ids {
                sqlx::query(
                    "UPDATE users SET wishlist = array_remove(wishlist, $1::text) WHERE id = $2",
                )
                .bind(&fig_key)
                .bind(uid)
                .execute(&mut *tx)
                .await?;
            }
        }

        let like_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM figurine_likes WHERE figurine_id = $1",
        )
        .bind(figurine_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        // Explicit-set: echo the requested state, not a re-read. A leftover row
        // must not glue the heart back on after the visitor asked to unlike.
        Ok((liked, like_count))
    }

    /// The two public "noticed" tiers, computed as a percentile rank among
    /// marked, visible figurines — not a fixed score threshold. "Deservedly
    /// in the top" means relative to the rest of the collection (Airbnb
    /// Guest Favorite works the same way: a percentile of eligible listings,
    /// not an absolute review count). Figurines with zero marks are never
    /// eligible — they're not "in the running" at all, same as an unreviewed
    /// Airbnb listing can't be a Guest Favorite. Below MIN_ELIGIBLE marked
    /// figurines, the collection is too young for "top X%" to mean anything,
    /// so both tiers come back empty rather than crowning whichever single
    /// piece happens to have 1 mark.
    pub async fn get_favorite_tiers(&self) -> Result<FavoriteTiers> {
        const MIN_ELIGIBLE: usize = 5;
        const NOTICED_PERCENTILE: f64 = 0.30;
        const HOUSE_FAVORITE_PERCENTILE: f64 = 0.10;

        let mut scored: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT f.id,
                    (COUNT(*) FILTER (WHERE m.tone = 'touched')
                     + COUNT(*) FILTER (WHERE m.tone = 'mesmerized') * 2
                     + COUNT(*) FILTER (WHERE m.tone = 'desired') * 3) AS score
             FROM figurines f
             JOIN figurine_marks m ON m.figurine_id = f.id
             WHERE f.is_visible = true
               AND (f.first_look_until IS NULL OR f.first_look_until <= NOW())
             GROUP BY f.id
             HAVING (COUNT(*) FILTER (WHERE m.tone = 'touched')
                     + COUNT(*) FILTER (WHERE m.tone = 'mesmerized') * 2
                     + COUNT(*) FILTER (WHERE m.tone = 'desired') * 3) > 0
             ORDER BY score DESC, f.id ASC",
        )
        .fetch_all(&self.pg_pool)
        .await?;

        if scored.len() < MIN_ELIGIBLE {
            return Ok(FavoriteTiers::default());
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        let noticed_cutoff = ((scored.len() as f64) * NOTICED_PERCENTILE).ceil().max(1.0) as usize;
        let favorite_cutoff = ((scored.len() as f64) * HOUSE_FAVORITE_PERCENTILE)
            .ceil()
            .max(1.0) as usize;

        Ok(FavoriteTiers {
            noticed: scored
                .iter()
                .take(noticed_cutoff)
                .map(|(id, _)| *id)
                .collect(),
            house_favorite: scored
                .iter()
                .take(favorite_cutoff)
                .map(|(id, _)| *id)
                .collect(),
        })
    }

    /// Admin-only ranking of every figurine by weighted mark score, including
    /// sold/gone pieces — this is a curation signal for the artisan, never
    /// rendered publicly. `desired` (closest to commission intent) outweighs
    /// `mesmerized`, which outweighs the base `touched`.
    pub async fn get_admin_mark_stats(&self) -> Result<Vec<AdminFigurineMarkStat>> {
        Ok(sqlx::query_as::<_, AdminFigurineMarkStat>(
            "SELECT f.id AS figurine_id, f.name AS figurine_name, f.status, f.is_visible,
                    COUNT(m.id) AS mark_count,
                    (SELECT COUNT(*)::bigint FROM figurine_likes l WHERE l.figurine_id = f.id) AS like_count,
                    COUNT(*) FILTER (WHERE m.tone = 'touched') AS touched_count,
                    COUNT(*) FILTER (WHERE m.tone = 'mesmerized') AS mesmerized_count,
                    COUNT(*) FILTER (WHERE m.tone = 'desired') AS desired_count,
                    (COUNT(*) FILTER (WHERE m.tone = 'touched')
                     + COUNT(*) FILTER (WHERE m.tone = 'mesmerized') * 2
                     + COUNT(*) FILTER (WHERE m.tone = 'desired') * 3) AS weighted_score,
                    MAX(m.created_at) AS last_marked_at
             FROM figurines f
             LEFT JOIN figurine_marks m ON m.figurine_id = f.id
             GROUP BY f.id, f.name, f.status, f.is_visible
             ORDER BY weighted_score DESC, mark_count DESC, last_marked_at DESC NULLS LAST, f.name ASC",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Auto-fill candidates for the public "noticed by guests" shelf: visible
    /// figurines with actual mark signal (score > 0), ranked by the same
    /// weighted score as the admin view, excluding anything already pinned or
    /// explicitly excluded by the admin. Never padded with zero-signal work —
    /// callers get fewer than `limit` rows if there isn't enough signal yet.
    pub async fn get_top_marked_figurine_ids(
        &self,
        exclude: &[Uuid],
        limit: i64,
    ) -> Result<Vec<Uuid>> {
        if limit <= 0 {
            return Ok(Vec::new());
        }
        Ok(sqlx::query_scalar::<_, Uuid>(
            "SELECT f.id
             FROM figurines f
             JOIN figurine_marks m ON m.figurine_id = f.id
             WHERE f.is_visible = true
               AND (f.first_look_until IS NULL OR f.first_look_until <= NOW())
               AND NOT (f.id = ANY($1))
             GROUP BY f.id
             HAVING (COUNT(*) FILTER (WHERE m.tone = 'touched')
                     + COUNT(*) FILTER (WHERE m.tone = 'mesmerized') * 2
                     + COUNT(*) FILTER (WHERE m.tone = 'desired') * 3) > 0
             ORDER BY (COUNT(*) FILTER (WHERE m.tone = 'touched')
                       + COUNT(*) FILTER (WHERE m.tone = 'mesmerized') * 2
                       + COUNT(*) FILTER (WHERE m.tone = 'desired') * 3) DESC,
                      MAX(m.created_at) DESC NULLS LAST
             LIMIT $2",
        )
        .bind(exclude)
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_bookings_page(
        &self,
        status_filter: Option<&str>,
        figurine_id_filter: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Booking>, i64)> {
        // Build WHERE clauses dynamically
        let mut conditions: Vec<String> = Vec::new();
        if status_filter.is_some() {
            conditions.push(format!(
                "status = ${}::booking_status",
                conditions.len() + 1
            ));
        }
        if figurine_id_filter.is_some() {
            conditions.push(format!("figurine_id = ${}", conditions.len() + 1));
        }
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let items_sql = format!(
            "SELECT * FROM figurine_bookings {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_clause,
            conditions.len() + 1,
            conditions.len() + 2
        );
        let count_sql = format!("SELECT COUNT(*) FROM figurine_bookings {}", where_clause);

        macro_rules! bind_filters {
            ($q:expr) => {{
                let mut q = $q;
                if let Some(s) = status_filter {
                    q = q.bind(s);
                }
                if let Some(f) = figurine_id_filter {
                    q = q.bind(f);
                }
                q
            }};
        }

        let items = bind_filters!(sqlx::query_as::<_, Booking>(&items_sql))
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pg_pool)
            .await?;

        let (total,): (i64,) = bind_filters!(sqlx::query_as::<_, (i64,)>(&count_sql))
            .fetch_one(&self.pg_pool)
            .await?;

        Ok((items, total))
    }

    pub async fn update_figurine_status(
        &self,
        figurine_id: Uuid,
        status: &crate::models::FigurineStatus,
    ) -> Result<()> {
        sqlx::query("UPDATE figurines SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status)
            .bind(figurine_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn has_future_confirmed_bookings(
        &self,
        figurine_id: Uuid,
        exclude_id: Uuid,
    ) -> Result<bool> {
        let today = chrono::Utc::now().date_naive();
        let (exists,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_bookings WHERE id != $1 AND figurine_id = $2 AND status = 'confirmed' AND ends_at >= $3)"
        )
        .bind(exclude_id).bind(figurine_id).bind(today)
        .fetch_one(&self.pg_pool).await?;
        Ok(exists)
    }

    pub async fn get_booking_by_id(&self, id: Uuid) -> Result<Option<Booking>> {
        Ok(
            sqlx::query_as::<_, Booking>("SELECT * FROM figurine_bookings WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pg_pool)
                .await?,
        )
    }

    /// Confirm a booking atomically: lock the figurine, re-check conflicts
    /// (showings + other confirmed bookings), then set the booking to confirmed
    /// and the figurine to reserved — all in one transaction. Returns
    /// `Ok(Some(reason))` if a conflict blocks confirmation (nothing written).
    pub async fn confirm_booking_atomic(
        &self,
        booking_id: Uuid,
        figurine_id: Uuid,
        starts_at: chrono::NaiveDate,
        ends_at: chrono::NaiveDate,
        admin_notes: Option<&str>,
        curator_conditions: Option<&str>,
    ) -> Result<Option<String>> {
        let mut tx = self.pg_pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(Self::booking_lock_key(figurine_id))
            .execute(&mut *tx)
            .await?;

        let (showing_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_showings WHERE figurine_id = $1 AND starts_at <= $3 AND ends_at >= $2)"
        )
        .bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&mut *tx).await?;
        if showing_conflict {
            return Ok(Some("Даты пересекаются с показом фигурки".to_string()));
        }

        let (booking_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_bookings WHERE id != $1 AND figurine_id = $2 AND status = 'confirmed' AND starts_at <= $4 AND ends_at >= $3)"
        )
        .bind(booking_id).bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&mut *tx).await?;
        if booking_conflict {
            return Ok(Some(
                "На эти даты уже есть подтверждённая бронь".to_string(),
            ));
        }

        sqlx::query(
            "UPDATE figurine_bookings SET status = 'confirmed', admin_notes = $2, curator_conditions = $3 WHERE id = $1"
        )
        .bind(booking_id).bind(admin_notes).bind(curator_conditions)
        .execute(&mut *tx).await?;

        sqlx::query("UPDATE figurines SET status = 'reserved', updated_at = NOW() WHERE id = $1")
            .bind(figurine_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(None)
    }

    pub async fn get_pending_bookings_count(&self) -> Result<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM figurine_bookings WHERE status = 'pending'")
                .fetch_one(&self.pg_pool)
                .await?;
        Ok(count)
    }

    pub async fn update_booking_status(
        &self,
        id: Uuid,
        status: &crate::models::BookingStatus,
        admin_notes: Option<&str>,
        curator_conditions: Option<&str>,
    ) -> Result<()> {
        // Direct assignment (not COALESCE) so the admin can clear notes/conditions
        // by sending null/empty, not only overwrite them.
        let affected = sqlx::query(
            "UPDATE figurine_bookings SET status = $1, admin_notes = $2, curator_conditions = $3 WHERE id = $4"
        )
        .bind(status).bind(admin_notes).bind(curator_conditions).bind(id)
        .execute(&self.pg_pool).await?.rows_affected();

        if affected == 0 {
            return Err(AppError::NotFound(format!("Booking {} not found", id)));
        }
        Ok(())
    }

    pub async fn replace_media_path_everywhere(
        &self,
        old_path: &str,
        new_preview_path: &str,
        new_original_path: Option<&str>,
        new_thumb_path: Option<&str>,
    ) -> Result<usize> {
        let mut updated = 0usize;

        if new_preview_path.starts_with("images/preview/")
            || new_preview_path.starts_with("/static/images/preview/")
        {
            let result = sqlx::query(
                "UPDATE images
                 SET file_path = $1, original_path = $2, thumb_path = $3
                 WHERE file_path = $4 OR original_path = $5 OR thumb_path = $6",
            )
            .bind(new_preview_path)
            .bind(new_original_path)
            .bind(new_thumb_path)
            .bind(old_path)
            .bind(old_path)
            .bind(old_path)
            .execute(&self.pg_pool)
            .await?;
            updated += result.rows_affected() as usize;
        } else {
            for column in ["file_path", "original_path", "thumb_path"] {
                let query = format!("UPDATE images SET {} = $1 WHERE {} = $2", column, column);
                let result = sqlx::query(&query)
                    .bind(new_preview_path)
                    .bind(old_path)
                    .execute(&self.pg_pool)
                    .await?;
                updated += result.rows_affected() as usize;
            }
        }

        for (table, column) in [
            ("process_steps", "image_path"),
            ("texts", "image_path"),
            ("figurines", "ambience_path"),
            ("figurines", "video_url"),
        ] {
            let query = format!("UPDATE {} SET {} = $1 WHERE {} = $2", table, column, column);
            let result = sqlx::query(&query)
                .bind(new_preview_path)
                .bind(old_path)
                .execute(&self.pg_pool)
                .await?;
            updated += result.rows_affected() as usize;
        }

        // app_resources — skip JSON-stored keys
        let result = sqlx::query(
            "UPDATE app_resources SET file_path = $1 WHERE file_path = $2 AND key NOT IN ('author_profile', 'home_content')"
        )
        .bind(new_preview_path)
        .bind(old_path)
        .execute(&self.pg_pool).await?;
        updated += result.rows_affected() as usize;

        Ok(updated)
    }

    // ============================================================
    // USER ACCOUNTS
    // ============================================================

    /// Завести имя. Адрес при этом НЕ подтверждён: подтверждает его переход по
    /// ссылке из письма, и до перехода имя не пускают внутрь.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_user(
        &self,
        email: &str,
        display_name: &str,
        hash: &str,
        visual_pool: &serde_json::Value,
        confirm_hash: &str,
        confirm_expires_at: chrono::DateTime<chrono::Utc>,
        ctx: &crate::models::ClientContext,
    ) -> Result<crate::models::User> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "INSERT INTO users (email, display_name, visual_password_hash, visual_pool,
                                email_confirm_hash, email_confirm_expires_at,
                                signup_ip, signup_country_code, signup_city)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *"
        )
        .bind(email)
        .bind(display_name)
        .bind(hash)
        .bind(visual_pool)
        .bind(confirm_hash)
        .bind(confirm_expires_at)
        .bind(&ctx.ip)
        .bind(&ctx.country_code)
        .bind(&ctx.city)
        .fetch_one(&self.pg_pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref dbe) = e
                && (dbe.constraint() == Some("users_email_key") || dbe.constraint() == Some("idx_users_email")) {
                    return AppError::Conflict("Email already registered".into());
                }
            e.into()
        })?;
        Ok(user)
    }

    /// Переписать имя, которое так и не открыли из ящика.
    ///
    /// Неподтверждённое имя принадлежит тому, кто докажет ящик, — и никому
    /// больше. Без этого правила занять чужой адрес можно было бы, просто
    /// набрав его: настоящий хозяин упёрся бы в уникальный индекс навсегда.
    /// Захвата здесь нет: письмо в обоих случаях уходит владельцу ящика, а
    /// прежняя ссылка гаснет вместе с прежними знаками.
    #[allow(clippy::too_many_arguments)]
    pub async fn reclaim_unconfirmed_user(
        &self,
        email: &str,
        display_name: &str,
        hash: &str,
        visual_pool: &serde_json::Value,
        confirm_hash: &str,
        confirm_expires_at: chrono::DateTime<chrono::Utc>,
        ctx: &crate::models::ClientContext,
    ) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "UPDATE users SET display_name = $2, visual_password_hash = $3, visual_pool = $4,
                    email_confirm_hash = $5, email_confirm_expires_at = $6,
                    password_reset_token = NULL, password_reset_expires_at = NULL,
                    signup_ip = $7, signup_country_code = $8, signup_city = $9
              WHERE email = $1 AND email_confirmed_at IS NULL AND telegram_id IS NULL
              RETURNING *",
        )
        .bind(email)
        .bind(display_name)
        .bind(hash)
        .bind(visual_pool)
        .bind(confirm_hash)
        .bind(confirm_expires_at)
        .bind(&ctx.ip)
        .bind(&ctx.country_code)
        .bind(&ctx.city)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(user)
    }

    /// Выдать новое письмо-приглашение уже существующему имени: и тому, кто не
    /// дождался первого, и тому, кто назвал адрес в ответ на дело.
    pub async fn set_email_confirm(
        &self,
        user_id: Uuid,
        confirm_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users SET email_confirm_hash = $2, email_confirm_expires_at = $3
              WHERE id = $1 AND email_confirmed_at IS NULL",
        )
        .bind(user_id)
        .bind(confirm_hash)
        .bind(expires_at)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Открыли дверь изнутри ящика. Условие стоит в самом `UPDATE`: два
    /// перехода по одной ссылке, пришедшие разом, иначе оба стали бы входом.
    pub async fn confirm_email(&self, confirm_hash: &str) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "UPDATE users SET email_confirmed_at = NOW(),
                    email_confirm_hash = NULL, email_confirm_expires_at = NULL
              WHERE email_confirm_hash = $1 AND email_confirm_expires_at > NOW()
              RETURNING *",
        )
        .bind(confirm_hash)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(user)
    }

    /// Пересобрать знаки на месте: новый хеш и новый набор при том же выборе.
    ///
    /// Зовётся после удачного входа по старым знакам — единственный миг, когда
    /// дом знает выбор человека в открытую и может переложить его на нынешний
    /// перец и нынешний размер набора, не спрашивая ни о чём.
    pub async fn refresh_visual_credentials(
        &self,
        user_id: Uuid,
        hash: &str,
        visual_pool: &serde_json::Value,
    ) -> Result<()> {
        sqlx::query("UPDATE users SET visual_password_hash = $2, visual_pool = $3 WHERE id = $1")
            .bind(user_id)
            .bind(hash)
            .bind(visual_pool)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // === TELEGRAM LOGIN: одноразовое слово ===

    /// Завести попытку входа. `bind_user` заполнен — это привязка из профиля,
    /// а не вход.
    pub async fn create_telegram_login_code(
        &self,
        code: &str,
        word: &str,
        bind_user: Option<Uuid>,
        browser: Option<&str>,
        ctx: &crate::models::ClientContext,
    ) -> Result<crate::models::TelegramLoginCode> {
        let row = sqlx::query_as::<_, crate::models::TelegramLoginCode>(
            "INSERT INTO telegram_login_codes
                (code, word, bind_user_id, browser, ip, country_code, city)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
        )
        .bind(code)
        .bind(word)
        .bind(bind_user)
        .bind(browser)
        .bind(&ctx.ip)
        .bind(&ctx.country_code)
        .bind(&ctx.city)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row)
    }

    /// Строка по коду — без оглядки на срок: просроченную надо УВИДЕТЬ, чтобы
    /// ответить «слово остыло», а не «такого слова нет».
    pub async fn find_telegram_login_code(
        &self,
        code: &str,
    ) -> Result<Option<crate::models::TelegramLoginCode>> {
        let row = sqlx::query_as::<_, crate::models::TelegramLoginCode>(
            "SELECT * FROM telegram_login_codes WHERE code = $1",
        )
        .bind(code)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(row)
    }

    /// Бот получил `/start` и показал записку. Переход только из `waiting`:
    /// второй `/start` по тому же коду ничего не переписывает, иначе нажатие
    /// «это я» относилось бы уже к другой записке.
    #[allow(clippy::too_many_arguments)]
    pub async fn telegram_login_asked(
        &self,
        code: &str,
        telegram_id: i64,
        username: Option<&str>,
        first_name: Option<&str>,
        chat_id: i64,
        message_id: Option<i64>,
    ) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE telegram_login_codes
                SET state = 'asked', telegram_id = $2, telegram_username = $3,
                    telegram_first_name = $4, chat_id = $5, message_id = $6
              WHERE code = $1 AND state = 'waiting' AND expires_at > NOW()",
        )
        .bind(code)
        .bind(telegram_id)
        .bind(username)
        .bind(first_name)
        .bind(chat_id)
        .bind(message_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected() == 1)
    }

    /// Нажали «это я»: сессия уже заведена, кладём её в строку.
    ///
    /// Подтверждает только ТОТ Telegram, который открывал записку: `telegram_id`
    /// в условии — защита от нажатия по чужой кнопке (её можно переслать).
    pub async fn telegram_login_confirmed(
        &self,
        code: &str,
        telegram_id: i64,
        session_token: &str,
    ) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE telegram_login_codes
                SET state = 'confirmed', session_token = $3
              WHERE code = $1 AND telegram_id = $2 AND state = 'asked' AND expires_at > NOW()",
        )
        .bind(code)
        .bind(telegram_id)
        .bind(session_token)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected() == 1)
    }

    /// Нажали «не я». Слово гаснет немедленно, не дожидаясь срока.
    pub async fn telegram_login_refused(&self, code: &str, telegram_id: i64) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE telegram_login_codes SET state = 'refused'
              WHERE code = $1 AND telegram_id = $2 AND state = 'asked'",
        )
        .bind(code)
        .bind(telegram_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected() == 1)
    }

    /// Страница забирает сессию — ровно один раз. Условие `state = 'confirmed'`
    /// стоит в самом UPDATE: два опроса, пришедшие разом, иначе унесли бы одну
    /// сессию дважды.
    pub async fn take_telegram_login_session(&self, code: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "UPDATE telegram_login_codes
                SET state = 'taken', taken_at = NOW()
              WHERE code = $1 AND state = 'confirmed' AND expires_at > NOW()
              RETURNING session_token",
        )
        .bind(code)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    /// Убрать отработавшие и остывшие слова. Срок короткий, строк мало —
    /// чистится заодно с прочей уборкой.
    pub async fn prune_telegram_login_codes(&self) -> Result<u64> {
        let res = sqlx::query(
            "DELETE FROM telegram_login_codes
              WHERE expires_at < NOW() - INTERVAL '1 hour' OR state IN ('taken', 'refused')",
        )
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    // === TELEGRAM-КАНАЛ ===

    /// Отметить вещи, замеченные на людях. Уже отмеченные не трогаются: строка
    /// журнала заводится один раз, поэтому скрыть и снова показать работу —
    /// не повод объявлять её второй раз.
    pub async fn channel_note_seen(&self, kind: &str, ids: &[Uuid], skipped: bool) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let res = sqlx::query(
            "INSERT INTO telegram_channel_posts (kind, target_id, skipped)
             SELECT $1, id, $3 FROM UNNEST($2::uuid[]) AS id
             ON CONFLICT (kind, target_id) DO NOTHING",
        )
        .bind(kind)
        .bind(ids)
        .bind(skipped)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    /// Что пора объявить: замечено не позже `grace_secs` назад, ещё не вышло и
    /// не исчерпало попыток. Старшие первыми — канал идёт в том порядке, в
    /// каком вещи появлялись на сайте.
    pub async fn channel_due(
        &self,
        grace_secs: i64,
        max_attempts: i32,
        limit: i64,
    ) -> Result<Vec<(String, Uuid)>> {
        Ok(sqlx::query_as(
            "SELECT kind, target_id FROM telegram_channel_posts
              WHERE posted_at IS NULL AND NOT skipped AND attempts < $2
                AND seen_at <= NOW() - make_interval(secs => $1)
              ORDER BY seen_at
              LIMIT $3",
        )
        .bind(grace_secs as f64)
        .bind(max_attempts)
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn channel_mark_posted(&self, kind: &str, id: Uuid, message_id: Option<i64>) -> Result<()> {
        sqlx::query(
            "UPDATE telegram_channel_posts
                SET posted_at = NOW(), message_id = $3, last_error = NULL
              WHERE kind = $1 AND target_id = $2",
        )
        .bind(kind)
        .bind(id)
        .bind(message_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn channel_mark_failed(&self, kind: &str, id: Uuid, why: &str) -> Result<()> {
        sqlx::query(
            "UPDATE telegram_channel_posts
                SET attempts = attempts + 1, last_error = $3
              WHERE kind = $1 AND target_id = $2",
        )
        .bind(kind)
        .bind(id)
        .bind(why)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Аккаунт, заведённый через Telegram: ни почты, ни значков.
    ///
    /// Почту спрашивает дело (бронь, заказ, заявка), а не дверь, — и спрашивает
    /// один раз. Значки человек назначит себе позже через `/set-password`,
    /// когда почта появится.
    pub async fn create_telegram_user(
        &self,
        telegram_id: i64,
        telegram_username: Option<&str>,
        display_name: &str,
        ctx: &crate::models::ClientContext,
    ) -> Result<crate::models::User> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "INSERT INTO users (display_name, telegram_id, telegram_username, telegram_linked_at,
                                signup_ip, signup_country_code, signup_city)
             VALUES ($1, $2, $3, NOW(), $4, $5, $6) RETURNING *",
        )
        .bind(display_name)
        .bind(telegram_id)
        .bind(telegram_username)
        .bind(&ctx.ip)
        .bind(&ctx.country_code)
        .bind(&ctx.city)
        .fetch_one(&self.pg_pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref dbe) = e
                && dbe.constraint() == Some("users_telegram_id_idx")
            {
                return AppError::Conflict("This Telegram account is already linked".into());
            }
            e.into()
        })?;
        Ok(user)
    }

    /// Записать почту аккаунту, у которого её не было.
    ///
    /// Условие `email IS NULL` стоит в самом UPDATE: замена почты — другое
    /// дело, с подтверждением по письму, и случиться молча оно не должно.
    /// Возвращает None, когда почта уже была.
    /// Назвать почту. Вместе с адресом заводится письмо-приглашение: названный
    /// адрес ещё не дверь, и станет ею только изнутри ящика.
    pub async fn set_user_email(
        &self,
        user_id: Uuid,
        email: &str,
        confirm_hash: &str,
        confirm_expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "UPDATE users SET email = $2, email_confirm_hash = $3, email_confirm_expires_at = $4
              WHERE id = $1 AND email IS NULL RETURNING *",
        )
        .bind(user_id)
        .bind(email)
        .bind(confirm_hash)
        .bind(confirm_expires_at)
        .fetch_optional(&self.pg_pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref dbe) = e
                && (dbe.constraint() == Some("users_email_key")
                    || dbe.constraint() == Some("idx_users_email"))
            {
                return AppError::Conflict("Email already registered".into());
            }
            e.into()
        })?;
        Ok(user)
    }

    /// Вернувшегося узнают по ЧИСЛУ, и только по нему. Поиск по `@username`
    /// однажды отдал бы архив тому, кто забрал освобождённое имя.
    pub async fn find_user_by_telegram_id(
        &self,
        telegram_id: i64,
    ) -> Result<Option<crate::models::User>> {
        let user =
            sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE telegram_id = $1")
                .bind(telegram_id)
                .fetch_optional(&self.pg_pool)
                .await?;
        Ok(user)
    }

    /// Привязать Telegram к существующему аккаунту. Занятое число — отказ:
    /// одно число значит одного человека.
    pub async fn link_telegram_to_user(
        &self,
        user_id: Uuid,
        telegram_id: i64,
        telegram_username: Option<&str>,
    ) -> Result<crate::models::User> {
        // `telegram_id IS NULL OR telegram_id = $2` — у имени, за которым уже
        // закреплено ДРУГОЕ число, привязка ничего не переписывает. Прежде
        // переписывала молча: дверь, о которой хозяин знал, переставала быть
        // его дверью, и узнавал он об этом, только не сумев войти. Повторная
        // привязка того же числа проходит — это не подмена, а нажали дважды.
        let user = sqlx::query_as::<_, crate::models::User>(
            "UPDATE users
                SET telegram_id = $2, telegram_username = $3, telegram_linked_at = NOW()
              WHERE id = $1 AND (telegram_id IS NULL OR telegram_id = $2) RETURNING *",
        )
        .bind(user_id)
        .bind(telegram_id)
        .bind(telegram_username)
        .fetch_optional(&self.pg_pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref dbe) = e
                && dbe.constraint() == Some("users_telegram_id_idx")
            {
                return AppError::Conflict("This Telegram account is already linked".into());
            }
            e.into()
        })?;
        user.ok_or_else(|| {
            AppError::Conflict("This account already has a different Telegram linked".into())
        })
    }

    /// Отвязать. Проверку «остаётся ли хоть одна дверь» делает слой выше:
    /// здесь бы она молча зависела от того, кто позвал.
    pub async fn unlink_telegram(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE users
                SET telegram_id = NULL, telegram_username = NULL, telegram_linked_at = NULL
              WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Освежить `@имя` при каждом входе: человек переименовался — в профиле
    /// должно стоять новое. Имя в профиле при этом НЕ трогается: его человек
    /// мог выбрать себе сам, и второй вход, вернувший телеграмовское, молча
    /// испортил бы его выбор.
    pub async fn touch_telegram_username(
        &self,
        user_id: Uuid,
        telegram_username: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE users SET telegram_username = $2 WHERE id = $1")
            .bind(user_id)
            .bind(telegram_username)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pg_pool)
            .await?;
        Ok(user)
    }

    pub async fn find_user_by_id(&self, id: Uuid) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pg_pool)
            .await?;
        Ok(user)
    }

    // ── Sessions ─────────────────────────────────────────────

    /// Завести сессию. В колонку `token` кладётся ОТПЕЧАТОК ключа: сам ключ
    /// уезжает в браузер и в базе не хранится. Чтение базы больше не является
    /// входом в каждое имя на месяц вперёд.
    pub async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        ctx: &crate::models::ClientContext,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_sessions (user_id, token, expires_at, ip, user_agent, country_code, city)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(&ctx.ip)
        .bind(&ctx.user_agent)
        .bind(&ctx.country_code)
        .bind(&ctx.city)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Чей это ключ. `is_blocked` спрашивается ЗДЕСЬ, а не только при входе:
    /// блокировка гасит сессии в свой миг, но полагаться на это значит
    /// полагаться на то, что ни одна будущая ветка не заведёт сессию в обход.
    pub async fn get_session_user(&self, token_hash: &str) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "SELECT u.* FROM users u
             JOIN user_sessions s ON s.user_id = u.id
             WHERE s.token = $1 AND s.expires_at > NOW() AND u.is_blocked = false",
        )
        .bind(token_hash)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(user)
    }

    pub async fn delete_session(&self, token_hash: &str) -> Result<()> {
        sqlx::query("DELETE FROM user_sessions WHERE token = $1")
            .bind(token_hash)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    /// Свои двери, как их видит хозяин имени. Ключей здесь нет — только
    /// отпечаток нынешней сессии, чтобы отметить ту, из которой смотрят.
    pub async fn own_sessions(
        &self,
        user_id: Uuid,
        current_hash: &str,
    ) -> Result<Vec<crate::models::OwnSessionDto>> {
        let rows: Vec<(
            Uuid,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT id, created_at, expires_at, token, user_agent, country_code, city
               FROM user_sessions
              WHERE user_id = $1 AND expires_at > NOW()
              ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(id, created_at, expires_at, token, user_agent, country_code, city)| {
                    let place: Vec<String> = [city, country_code]
                        .into_iter()
                        .flatten()
                        .filter(|s| !s.trim().is_empty())
                        .collect();
                    crate::models::OwnSessionDto {
                        id: id.to_string(),
                        created_at: created_at.to_rfc3339(),
                        expires_at: expires_at.to_rfc3339(),
                        current: token == current_hash,
                        browser: user_agent,
                        place: (!place.is_empty()).then(|| place.join(", ")),
                    }
                },
            )
            .collect())
    }

    /// Закрыть все двери, кроме той, из которой смотрят. Нынешняя остаётся
    /// намеренно: кнопка, выбрасывающая нажавшего, спрашивала бы каждый раз
    /// «а точно ли я хотел выйти сам».
    pub async fn revoke_other_sessions(&self, user_id: Uuid, keep_hash: &str) -> Result<u64> {
        let res = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1 AND token <> $2")
            .bind(user_id)
            .bind(keep_hash)
            .execute(&self.pg_pool)
            .await?;
        Ok(res.rows_affected())
    }

    // ── Wishlist ─────────────────────────────────────────────

    pub async fn get_user_wishlist(&self, user_id: Uuid) -> Result<Vec<String>> {
        let row: (Vec<String>,) = sqlx::query_as("SELECT wishlist FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.pg_pool)
            .await?;
        Ok(row.0)
    }

    pub async fn set_user_wishlist(&self, user_id: Uuid, ids: &[String]) -> Result<()> {
        sqlx::query("UPDATE users SET wishlist = $1 WHERE id = $2")
            .bind(ids)
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // ── Link guest requests by code ──────────────────────────
    //
    // bookings, waitlist entries and notify-orders all carry a `cancel_token`,
    // `requester_email`, `user_id` and `figurine_name`, so one helper covers all
    // three. The email guard mirrors link_bookings_to_user: the secret token is the
    // lookup key, but the row's email must match the account claiming it.

    async fn link_claim_row(
        &self,
        table: &str,
        user_id: Uuid,
        email: &str,
        token: &str,
    ) -> Result<Option<ClaimMatch>> {
        // `table` is a trusted, hard-coded constant (never user input); token and
        // email are bound parameters, so there is no injection surface.
        let select = format!(
            "SELECT requester_email, user_id, figurine_name FROM {table} WHERE cancel_token = $1"
        );
        let row = sqlx::query_as::<_, (String, Option<Uuid>, String)>(&select)
            .bind(token)
            .fetch_optional(&self.pg_pool)
            .await?;
        let Some((req_email, owner, name)) = row else {
            return Ok(None);
        };

        if !req_email.trim().eq_ignore_ascii_case(email.trim()) {
            return Ok(Some(ClaimMatch {
                email_ok: false,
                linked: false,
                name,
            }));
        }
        let linked = match owner {
            Some(o) if o != user_id => false, // already attached to a different account
            _ => {
                let update = format!("UPDATE {table} SET user_id = $1 WHERE cancel_token = $2");
                sqlx::query(&update)
                    .bind(user_id)
                    .bind(token)
                    .execute(&self.pg_pool)
                    .await?;
                true
            }
        };
        Ok(Some(ClaimMatch {
            email_ok: true,
            linked,
            name,
        }))
    }

    pub async fn link_booking_by_token(
        &self,
        user_id: Uuid,
        email: &str,
        token: &str,
    ) -> Result<Option<ClaimMatch>> {
        self.link_claim_row("figurine_bookings", user_id, email, token)
            .await
    }

    pub async fn link_waitlist_by_token(
        &self,
        user_id: Uuid,
        email: &str,
        token: &str,
    ) -> Result<Option<ClaimMatch>> {
        self.link_claim_row("figurine_waitlist", user_id, email, token)
            .await
    }

    pub async fn link_notify_order_by_token(
        &self,
        user_id: Uuid,
        email: &str,
        token: &str,
    ) -> Result<Option<ClaimMatch>> {
        // Only "notify" orders ever carry a cancel_token, so a match here is unambiguous.
        self.link_claim_row("orders", user_id, email, token).await
    }

    /// Whether a commission with this claim token exists and is still claimable by
    /// the given user (unclaimed, or already theirs). Commissions are token-only
    /// (no email guard), matching the existing claim_commission contract.
    pub async fn commission_claimable_by(&self, token: &str, user_id: Uuid) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM commissions WHERE claim_token = $1 AND (user_id IS NULL OR user_id = $2)"
        )
        .bind(token)
        .bind(user_id)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(row.is_some())
    }

    // ── Challenges ───────────────────────────────────────────

    pub async fn save_challenge(
        &self,
        email: &str,
        tokens_json: &serde_json::Value,
    ) -> Result<Uuid> {
        let rec: (Uuid,) = sqlx::query_as(
            "INSERT INTO login_challenges (email, tokens_json)
             VALUES ($1, $2) RETURNING id",
        )
        .bind(email)
        .bind(tokens_json)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(rec.0)
    }

    pub async fn get_challenge(&self, id: Uuid) -> Result<Option<(String, serde_json::Value)>> {
        let row: Option<(String, serde_json::Value)> = sqlx::query_as(
            "SELECT email, tokens_json FROM login_challenges
             WHERE id = $1 AND expires_at > NOW() AND used_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(row)
    }

    pub async fn mark_challenge_used(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE login_challenges SET used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // ── Lockout ──────────────────────────────────────────────

    pub async fn record_attempt(
        &self,
        email: &str,
        success: bool,
        ctx: &crate::models::ClientContext,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO login_attempts (email, success, ip, user_agent, country_code, city)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(email)
        .bind(success)
        .bind(&ctx.ip)
        .bind(&ctx.user_agent)
        .bind(&ctx.country_code)
        .bind(&ctx.city)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Delete login attempts older than `days` (retention / GDPR housekeeping).
    pub async fn prune_old_login_attempts(&self, days: i64) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM login_attempts WHERE attempted_at < NOW() - ($1 || ' days')::interval",
        )
        .bind(days)
        .execute(&self.pg_pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Неудачи, посчитанные тремя способами за один проход.
    ///
    /// Одного счёта по почте не хватает, и не хватает в обе стороны. Пять
    /// ошибок по чужому адресу запирали хозяина на пятнадцать минут — то есть
    /// запереть человека мог кто угодно, бесплатно и сколько угодно раз. А
    /// счёт по одной только паре «почта и сеть» запирал бы лишь одну сеть:
    /// перебирающий с десяти адресов не заметил бы преграды вовсе.
    ///
    /// Поэтому считается и пара (тесно, на четверть часа), и имя целиком
    /// (свободно, на час — это потолок на подбор, а не на опечатку), и сама
    /// сеть (чтобы один адрес не веерил по многим именам).
    pub async fn recent_failures(
        &self,
        email: &str,
        ip: Option<&str>,
        pair_minutes: i64,
        wide_minutes: i64,
    ) -> Result<crate::models::FailureTally> {
        let row: (i64, i64, i64) = sqlx::query_as(
            "SELECT
                COUNT(*) FILTER (
                    WHERE email = $1 AND ip IS NOT DISTINCT FROM $2
                      AND attempted_at > NOW() - ($3 || ' minutes')::interval
                )::bigint,
                COUNT(*) FILTER (WHERE email = $1)::bigint,
                COUNT(*) FILTER (WHERE ip IS NOT DISTINCT FROM $2)::bigint
             FROM login_attempts
             WHERE success = false
               AND attempted_at > NOW() - ($4 || ' minutes')::interval
               AND (email = $1 OR ip IS NOT DISTINCT FROM $2)",
        )
        .bind(email)
        .bind(ip)
        .bind(pair_minutes)
        .bind(wide_minutes)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(crate::models::FailureTally {
            pair: row.0,
            by_email: row.1,
            by_ip: row.2,
        })
    }

    // ── Profile data ─────────────────────────────────────────

    /// Attach anonymous bookings to a user account. A short cancel token alone is
    /// not proof of ownership, so we additionally require the booking's
    /// requester_email to match the account's email — preventing a guessed token
    /// from claiming someone else's booking.
    pub async fn link_bookings_to_user(
        &self,
        user_id: Uuid,
        email: &str,
        cancel_tokens: &[String],
    ) -> Result<usize> {
        if cancel_tokens.is_empty() {
            return Ok(0);
        }
        let result = sqlx::query(
            "UPDATE figurine_bookings SET user_id = $1
             WHERE cancel_token = ANY($2) AND user_id IS NULL AND lower(requester_email) = lower($3)"
        )
        .bind(user_id)
        .bind(cancel_tokens)
        .bind(email)
        .execute(&self.pg_pool)
        .await?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn prune_expired_sessions(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM user_sessions WHERE user_id = $1 AND expires_at < NOW()")
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_user_bookings(&self, user_id: Uuid) -> Result<Vec<crate::models::Booking>> {
        let bookings = sqlx::query_as::<_, crate::models::Booking>(
            "SELECT * FROM figurine_bookings WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(bookings)
    }

    pub async fn get_user_orders(&self, user_id: Uuid) -> Result<Vec<crate::models::Order>> {
        let orders = sqlx::query_as::<_, crate::models::Order>(
            "SELECT * FROM orders WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(orders)
    }

    // ── Admin user management ────────────────────────────────

    pub async fn admin_list_users(
        &self,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<crate::models::AdminUserListItem>, i64)> {
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));
        let items = if let Some(ref p) = pattern {
            sqlx::query_as::<_, crate::models::AdminUserListItem>(
                "SELECT u.id::text, u.email, u.display_name, u.admin_notes,
                        u.created_at::text,
                        COUNT(DISTINCT b.id) AS booking_count,
                        COUNT(DISTINCT o.id) AS order_count
                 FROM users u
                 LEFT JOIN figurine_bookings b ON b.user_id = u.id
                 LEFT JOIN orders o ON o.user_id = u.id
                 WHERE LOWER(u.email) LIKE $1 OR LOWER(u.display_name) LIKE $1
                 GROUP BY u.id
                 ORDER BY u.created_at DESC
                 LIMIT $2 OFFSET $3",
            )
            .bind(p)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pg_pool)
            .await?
        } else {
            sqlx::query_as::<_, crate::models::AdminUserListItem>(
                "SELECT u.id::text, u.email, u.display_name, u.admin_notes,
                        u.created_at::text,
                        COUNT(DISTINCT b.id) AS booking_count,
                        COUNT(DISTINCT o.id) AS order_count
                 FROM users u
                 LEFT JOIN figurine_bookings b ON b.user_id = u.id
                 LEFT JOIN orders o ON o.user_id = u.id
                 GROUP BY u.id
                 ORDER BY u.created_at DESC
                 LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pg_pool)
            .await?
        };

        let (total,): (i64,) = if let Some(ref p) = pattern {
            sqlx::query_as(
                "SELECT COUNT(*) FROM users WHERE LOWER(email) LIKE $1 OR LOWER(display_name) LIKE $1"
            ).bind(p).fetch_one(&self.pg_pool).await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM users")
                .fetch_one(&self.pg_pool)
                .await?
        };

        Ok((items, total))
    }

    pub async fn admin_get_user_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::AdminSessionDto>> {
        let rows: Vec<(
            Uuid,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT id, created_at, expires_at, ip, country_code, city FROM user_sessions
                 WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;

        let now = chrono::Utc::now();
        Ok(rows
            .into_iter()
            .map(|(id, created_at, expires_at, ip, country_code, city)| {
                crate::models::AdminSessionDto {
                    id: id.to_string(),
                    created_at: created_at.to_rfc3339(),
                    expires_at: expires_at.to_rfc3339(),
                    is_active: expires_at > now,
                    ip,
                    country_code,
                    city,
                }
            })
            .collect())
    }

    pub async fn admin_revoke_all_sessions(&self, user_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn admin_update_user_notes(&self, user_id: Uuid, notes: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE users SET admin_notes = $1 WHERE id = $2")
            .bind(notes)
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn admin_set_user_blocked(&self, user_id: Uuid, blocked: bool) -> Result<()> {
        sqlx::query("UPDATE users SET is_blocked = $1 WHERE id = $2")
            .bind(blocked)
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn admin_create_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users SET password_reset_token = $1, password_reset_expires_at = $2 WHERE id = $3"
        )
        .bind(token_hash)
        .bind(expires_at)
        .bind(user_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Self-service reset: store the token and record where it was requested from.
    pub async fn create_self_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        ctx: &crate::models::ClientContext,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users SET password_reset_token = $1, password_reset_expires_at = $2,
                 last_reset_request_ip = $3, last_reset_request_country_code = $4,
                 last_reset_request_city = $5, last_reset_request_at = NOW()
             WHERE id = $6",
        )
        .bind(token_hash)
        .bind(expires_at)
        .bind(&ctx.ip)
        .bind(&ctx.country_code)
        .bind(&ctx.city)
        .bind(user_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Чей это ключ сброса — по отпечатку и сроку.
    pub async fn find_user_by_reset_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "SELECT * FROM users WHERE password_reset_token = $1 AND password_reset_expires_at > NOW()"
        )
        .bind(token_hash)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(user)
    }

    pub async fn apply_password_reset(
        &self,
        user_id: Uuid,
        new_hash: &str,
        visual_pool: &serde_json::Value,
        ctx: &crate::models::ClientContext,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users SET visual_password_hash = $1, visual_pool = $2,
                 password_reset_token = NULL, password_reset_expires_at = NULL,
                 last_reset_ip = $3, last_reset_country_code = $4, last_reset_city = $5, last_reset_at = NOW()
             WHERE id = $6"
        )
        .bind(new_hash)
        .bind(visual_pool)
        .bind(&ctx.ip)
        .bind(&ctx.country_code)
        .bind(&ctx.city)
        .bind(user_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    // === COMMENTS ===

    pub async fn insert_comment(
        &self,
        figurine_id: Uuid,
        user_id: Option<Uuid>,
        author_name: &str,
        author_email: Option<&str>,
        body: &str,
    ) -> Result<crate::models::Comment> {
        let rec = sqlx::query_as::<_, crate::models::Comment>(
            "INSERT INTO figurine_comments (figurine_id, user_id, author_name, author_email, body)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(figurine_id)
        .bind(user_id)
        .bind(author_name)
        .bind(author_email)
        .bind(body)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(rec)
    }

    pub async fn get_approved_comments(
        &self,
        figurine_id: Uuid,
        newest_first: bool,
    ) -> Result<Vec<crate::models::CommentWithAvatar>> {
        let order = if newest_first { "DESC" } else { "ASC" };
        let rows = sqlx::query_as::<_, crate::models::CommentWithAvatar>(&format!(
            "SELECT c.id, c.figurine_id, c.user_id, c.author_name, c.author_email, \
                        c.body, c.is_approved, c.admin_reply, c.created_at, \
                        u.avatar_url \
                 FROM figurine_comments c \
                 LEFT JOIN users u ON u.id = c.user_id \
                 WHERE c.figurine_id = $1 AND c.is_approved = true \
                 ORDER BY c.created_at {order}"
        ))
        .bind(figurine_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_pending_comments_count(&self) -> Result<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM figurine_comments WHERE is_approved = false")
                .fetch_one(&self.pg_pool)
                .await?;
        Ok(count)
    }

    pub async fn get_comments_admin_page(
        &self,
        only_pending: bool,
        figurine_filter: Option<Uuid>,
        newest_first: bool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<(crate::models::Comment, String)>, i64)> {
        // The items query binds limit ($1) and offset ($2) first, so its
        // figurine filter is $3. The count query has no limit/offset, so its
        // figurine filter must be $1. Build a where-clause for each — sharing a
        // single clause would leave the count query referencing an unbound $3
        // (Postgres: "could not determine data type of parameter $2").
        let build_where = |figurine_placeholder: &str| {
            let mut conditions: Vec<String> = Vec::new();
            if only_pending {
                conditions.push("c.is_approved = false".to_string());
            }
            if figurine_filter.is_some() {
                conditions.push(format!("c.figurine_id = {figurine_placeholder}"));
            }
            if conditions.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", conditions.join(" AND "))
            }
        };
        let where_clause = build_where("$3");
        let count_where = build_where("$1");
        let order = if newest_first { "DESC" } else { "ASC" };

        let items: Vec<(crate::models::Comment, String)> = {
            let query_str = format!(
                "SELECT c.*, f.name AS figurine_name
                 FROM figurine_comments c
                 JOIN figurines f ON f.id = c.figurine_id
                 {where_clause}
                 ORDER BY c.created_at {order}
                 LIMIT $1 OFFSET $2"
            );
            let mut q = sqlx::query(&query_str).bind(limit).bind(offset);
            if let Some(fid) = figurine_filter {
                q = q.bind(fid);
            }

            let rows = q.fetch_all(&self.pg_pool).await?;

            use sqlx::Row;
            rows.into_iter()
                .map(|row| {
                    let c = crate::models::Comment {
                        id: row.get("id"),
                        figurine_id: row.get("figurine_id"),
                        user_id: row.get("user_id"),
                        author_name: row.get("author_name"),
                        author_email: row.get("author_email"),
                        body: row.get("body"),
                        is_approved: row.get("is_approved"),
                        admin_reply: row.get("admin_reply"),
                        created_at: row.get("created_at"),
                    };
                    let name: String = row.get("figurine_name");
                    (c, name)
                })
                .collect()
        };

        let count_str = format!(
            "SELECT COUNT(*) FROM figurine_comments c JOIN figurines f ON f.id = c.figurine_id {count_where}"
        );
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_str);
        if let Some(fid) = figurine_filter {
            count_q = count_q.bind(fid);
        }
        let (total,) = count_q.fetch_one(&self.pg_pool).await?;

        Ok((items, total))
    }

    pub async fn moderate_comment(
        &self,
        id: Uuid,
        is_approved: bool,
        admin_reply: Option<&str>,
    ) -> Result<crate::models::Comment> {
        let rec = sqlx::query_as::<_, crate::models::Comment>(
            "UPDATE figurine_comments SET is_approved = $1, admin_reply = $2 WHERE id = $3 RETURNING *"
        )
        .bind(is_approved)
        .bind(admin_reply)
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Comment {} not found", id)))?;
        Ok(rec)
    }

    pub async fn delete_comment(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM figurine_comments WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Comment {} not found", id)));
        }
        Ok(())
    }

    // === VISITOR IMPRESSIONS ===

    pub async fn insert_impression(
        &self,
        message: &str,
        author_name: Option<&str>,
        mood: Option<&str>,
        ip: Option<&str>,
    ) -> Result<crate::models::Impression> {
        let rec = sqlx::query_as::<_, crate::models::Impression>(
            "INSERT INTO visitor_impressions (message, author_name, mood, ip)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(message)
        .bind(author_name)
        .bind(mood)
        .bind(ip)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(rec)
    }

    pub async fn get_featured_impressions(&self) -> Result<Vec<crate::models::Impression>> {
        let rows = sqlx::query_as::<_, crate::models::Impression>(
            "SELECT * FROM visitor_impressions
             WHERE is_approved = true AND is_featured = true
             ORDER BY created_at DESC
             LIMIT 24",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_pending_impressions_count(&self) -> Result<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM visitor_impressions WHERE is_approved = false")
                .fetch_one(&self.pg_pool)
                .await?;
        Ok(count)
    }

    pub async fn get_impressions_admin_page(
        &self,
        only_pending: bool,
        newest_first: bool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<crate::models::Impression>, i64)> {
        let where_clause = if only_pending {
            "WHERE is_approved = false"
        } else {
            ""
        };
        let order = if newest_first { "DESC" } else { "ASC" };

        let items = sqlx::query_as::<_, crate::models::Impression>(&format!(
            "SELECT * FROM visitor_impressions {where_clause}
             ORDER BY created_at {order}
             LIMIT $1 OFFSET $2"
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pg_pool)
        .await?;

        let (total,): (i64,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM visitor_impressions {where_clause}"
        ))
        .fetch_one(&self.pg_pool)
        .await?;

        Ok((items, total))
    }

    pub async fn moderate_impression(
        &self,
        id: Uuid,
        is_approved: bool,
        is_featured: bool,
    ) -> Result<crate::models::Impression> {
        let rec = sqlx::query_as::<_, crate::models::Impression>(
            "UPDATE visitor_impressions SET is_approved = $1, is_featured = $2 WHERE id = $3 RETURNING *"
        )
        .bind(is_approved)
        .bind(is_featured)
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Impression {} not found", id)))?;
        Ok(rec)
    }

    pub async fn delete_impression(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM visitor_impressions WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Impression {} not found", id)));
        }
        Ok(())
    }

    // === SETTINGS ===

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pg_pool)
            .await?;
        Ok(row.map(|(v,)| v))
    }

    pub async fn upsert_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn update_user_display_name(
        &self,
        user_id: Uuid,
        display_name: &str,
    ) -> Result<crate::models::User> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "UPDATE users SET display_name = $1 WHERE id = $2 RETURNING *",
        )
        .bind(display_name)
        .bind(user_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(user)
    }

    pub async fn update_user_avatar(
        &self,
        user_id: Uuid,
        avatar_url: &str,
    ) -> Result<crate::models::User> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "UPDATE users SET avatar_url = $1 WHERE id = $2 RETURNING *",
        )
        .bind(avatar_url)
        .bind(user_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(user)
    }

    pub async fn delete_user(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // === CONFLICT CHECK EXCLUDING ONE BOOKING ===

    /// Same as check_booking_conflicts but excludes a specific booking ID (for reschedule).
    pub async fn check_booking_conflicts_excluding(
        &self,
        figurine_id: Uuid,
        exclude_booking_id: Uuid,
        starts_at: chrono::NaiveDate,
        ends_at: chrono::NaiveDate,
    ) -> Result<bool> {
        let (showing_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_showings WHERE figurine_id = $1 AND starts_at <= $3 AND ends_at >= $2)"
        )
        .bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&self.pg_pool).await?;

        if showing_conflict {
            return Ok(true);
        }

        let (booking_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_bookings WHERE figurine_id = $1 AND id != $2 AND status = 'confirmed' AND starts_at <= $4 AND ends_at >= $3)"
        )
        .bind(figurine_id).bind(exclude_booking_id).bind(starts_at).bind(ends_at)
        .fetch_one(&self.pg_pool).await?;

        Ok(booking_conflict)
    }

    // === RESCHEDULE BOOKING BY TOKEN ===

    /// Updates starts_at/ends_at for a pending booking identified by cancel token.
    /// Returns the updated booking, or None if not found / not pending.
    pub async fn reschedule_booking_by_token(
        &self,
        token: &str,
        starts_at: chrono::NaiveDate,
        ends_at: chrono::NaiveDate,
    ) -> Result<Option<crate::models::Booking>> {
        Ok(sqlx::query_as::<_, crate::models::Booking>(
            "UPDATE figurine_bookings SET starts_at = $1, ends_at = $2
             WHERE cancel_token = $3 AND status = 'pending'
             RETURNING *",
        )
        .bind(starts_at)
        .bind(ends_at)
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    // === WAITLIST ===

    /// Join a figurine's queue. If this email is already queued for the work,
    /// the existing row is refreshed in place (keeping its original join time,
    /// so the visitor keeps their place) rather than creating a duplicate.
    /// Returns the entry plus its 1-based position in the queue.
    pub async fn add_to_waitlist(
        &self,
        figurine_id: Uuid,
        req: &crate::models::CreateWaitlistRequest,
        user_id: Option<Uuid>,
    ) -> Result<(crate::models::WaitlistEntry, i64)> {
        // Dedupe at the DB level via a unique index on (figurine_id, lower(email)).
        // A repeat request updates the existing row in place (keeping its original
        // created_at → queue position, and its cancel token), so two concurrent
        // submissions from the same email can't create duplicate rows.
        let token = Self::generate_cancel_token();
        let entry = sqlx::query_as::<_, crate::models::WaitlistEntry>(
            "INSERT INTO figurine_waitlist (figurine_id, figurine_name, requester_name, requester_email, requester_phone, note, user_id, cancel_token)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (figurine_id, lower(requester_email)) DO UPDATE SET
               requester_name = EXCLUDED.requester_name,
               requester_phone = EXCLUDED.requester_phone,
               note = EXCLUDED.note,
               user_id = COALESCE(EXCLUDED.user_id, figurine_waitlist.user_id)
             RETURNING *"
        )
        .bind(figurine_id)
        .bind(&req.figurine_name)
        .bind(&req.requester_name)
        .bind(&req.requester_email)
        .bind(&req.requester_phone)
        .bind(&req.note)
        .bind(user_id)
        .bind(&token)
        .fetch_one(&self.pg_pool).await?;

        let position = self
            .waitlist_position(figurine_id, entry.created_at)
            .await?;
        Ok((entry, position))
    }

    /// 1-based rank of an entry in its figurine queue, ordered by join time.
    /// Derived from `created_at` ordering, so positions recompute automatically
    /// whenever anyone leaves the queue — no stored counter to maintain.
    pub async fn waitlist_position(
        &self,
        figurine_id: Uuid,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64> {
        let (pos,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM figurine_waitlist WHERE figurine_id = $1 AND created_at <= $2",
        )
        .bind(figurine_id)
        .bind(created_at)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(pos)
    }

    /// Batched queue positions for one user's waitlist entries: one query instead
    /// of a per-row `SELECT COUNT(*)` (the N+1 `get_user_waitlist` used to do in the
    /// service). Position matches `waitlist_position` — rank within the whole
    /// figurine queue (everyone created at or before this entry). Keyed by entry id.
    pub async fn waitlist_positions_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<std::collections::HashMap<Uuid, i64>> {
        let rows: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT w.id,
                    (SELECT COUNT(*) FROM figurine_waitlist w2
                      WHERE w2.figurine_id = w.figurine_id AND w2.created_at <= w.created_at)
             FROM figurine_waitlist w
             WHERE w.user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows.into_iter().collect())
    }

    pub async fn get_user_waitlist(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::WaitlistEntry>> {
        Ok(sqlx::query_as::<_, crate::models::WaitlistEntry>(
            "SELECT * FROM figurine_waitlist WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_waitlist_by_cancel_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::WaitlistEntry>> {
        Ok(sqlx::query_as::<_, crate::models::WaitlistEntry>(
            "SELECT * FROM figurine_waitlist WHERE cancel_token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Leave the queue by token. Idempotent — returns the removed row if any.
    pub async fn remove_waitlist_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::WaitlistEntry>> {
        Ok(sqlx::query_as::<_, crate::models::WaitlistEntry>(
            "DELETE FROM figurine_waitlist WHERE cancel_token = $1 RETURNING *",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn get_waitlist_admin(
        &self,
        figurine_id: Option<Uuid>,
    ) -> Result<Vec<crate::models::WaitlistEntry>> {
        if let Some(fid) = figurine_id {
            Ok(sqlx::query_as::<_, crate::models::WaitlistEntry>(
                "SELECT * FROM figurine_waitlist WHERE figurine_id = $1 ORDER BY created_at ASC",
            )
            .bind(fid)
            .fetch_all(&self.pg_pool)
            .await?)
        } else {
            Ok(sqlx::query_as::<_, crate::models::WaitlistEntry>(
                "SELECT * FROM figurine_waitlist ORDER BY created_at ASC",
            )
            .fetch_all(&self.pg_pool)
            .await?)
        }
    }

    pub async fn remove_from_waitlist(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM figurine_waitlist WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // === NEWSLETTER ("visitor book") ===

    /// Sign the book. Idempotent: a repeat sign-up of the same email updates the
    /// row in place and clears any prior unsubscribe (so the unique email index
    /// can't be violated by a race). Returns the row and whether the address was
    /// already an active subscriber — used only to vary the visitor's wording.
    pub async fn subscribe(
        &self,
        req: &crate::models::CreateSubscriptionRequest,
        ip: Option<&str>,
    ) -> Result<(crate::models::Subscriber, bool)> {
        // Cosmetic "already in the book" flag; the unique index is what actually
        // guarantees no duplicates, so a race here is harmless.
        let (existed,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM newsletter_subscribers WHERE lower(email) = lower($1) AND unsubscribed_at IS NULL)",
        )
        .bind(&req.email)
        .fetch_one(&self.pg_pool)
        .await?;

        let token = Self::generate_cancel_token();
        let source = req.source.as_deref().unwrap_or("home");
        let lang = req.lang.as_deref().unwrap_or("en");
        let sub = sqlx::query_as::<_, crate::models::Subscriber>(
            "INSERT INTO newsletter_subscribers (email, name, source, lang, unsubscribe_token, ip)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (lower(email)) DO UPDATE SET
               name = COALESCE(EXCLUDED.name, newsletter_subscribers.name),
               lang = EXCLUDED.lang,
               unsubscribed_at = NULL
             RETURNING *",
        )
        .bind(&req.email)
        .bind(&req.name)
        .bind(source)
        .bind(lang)
        .bind(&token)
        .bind(ip)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok((sub, existed))
    }

    /// Look up a subscriber by unsubscribe token (the unsubscribe page's receipt
    /// lookup). Returns the row even if already unsubscribed, so the page is
    /// idempotent.
    pub async fn get_subscriber_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::Subscriber>> {
        Ok(sqlx::query_as::<_, crate::models::Subscriber>(
            "SELECT * FROM newsletter_subscribers WHERE unsubscribe_token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Soft-unsubscribe by token. Idempotent — returns the row only on the first
    /// call that actually flips it (already-unsubscribed rows return None).
    pub async fn unsubscribe_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::Subscriber>> {
        Ok(sqlx::query_as::<_, crate::models::Subscriber>(
            "UPDATE newsletter_subscribers SET unsubscribed_at = NOW()
             WHERE unsubscribe_token = $1 AND unsubscribed_at IS NULL
             RETURNING *",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn list_subscribers_admin(&self) -> Result<Vec<crate::models::Subscriber>> {
        Ok(sqlx::query_as::<_, crate::models::Subscriber>(
            "SELECT * FROM newsletter_subscribers WHERE unsubscribed_at IS NULL ORDER BY created_at DESC",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn remove_subscriber(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM newsletter_subscribers WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // === CONTACT MESSAGES ("write to the author") ===

    pub async fn create_contact_message(
        &self,
        req: &crate::models::CreateContactMessageRequest,
        ip: Option<&str>,
    ) -> Result<crate::models::ContactMessage> {
        let source = req.source.as_deref().unwrap_or("home");
        let lang = req.lang.as_deref().unwrap_or("en");
        Ok(sqlx::query_as::<_, crate::models::ContactMessage>(
            "INSERT INTO contact_messages (email, message, source, lang, ip)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(&req.email)
        .bind(&req.message)
        .bind(source)
        .bind(lang)
        .bind(ip)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn list_contact_messages_admin(&self) -> Result<Vec<crate::models::ContactMessage>> {
        Ok(sqlx::query_as::<_, crate::models::ContactMessage>(
            "SELECT * FROM contact_messages ORDER BY created_at DESC",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn mark_contact_message_read(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE contact_messages SET is_read = true WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn remove_contact_message(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM contact_messages WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_waitlist_for_figurine(
        &self,
        figurine_id: Uuid,
    ) -> Result<Vec<crate::models::WaitlistEntry>> {
        Ok(sqlx::query_as::<_, crate::models::WaitlistEntry>(
            "SELECT * FROM figurine_waitlist WHERE figurine_id = $1 ORDER BY created_at ASC",
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn mark_waitlist_notified(&self, figurine_id: Uuid) -> Result<u64> {
        Ok(
            sqlx::query("DELETE FROM figurine_waitlist WHERE figurine_id = $1")
                .bind(figurine_id)
                .execute(&self.pg_pool)
                .await?
                .rows_affected(),
        )
    }

    // ── Message threads ────────────────────────────────────────

    pub async fn create_thread(
        &self,
        user_id: Uuid,
        category: &str,
        reference_id: Option<Uuid>,
        subject: &str,
        body: &str,
        from_admin: bool,
        attachments: &[crate::models::AttachmentInput],
    ) -> Result<(crate::models::MessageThread, crate::models::ThreadMessage)> {
        // Thread + first message + its attachments are written atomically, so a
        // failure can't leave an empty thread or a message without its references.
        let mut tx = self.pg_pool.begin().await?;

        let thread = sqlx::query_as::<_, crate::models::MessageThread>(
            "INSERT INTO message_threads (user_id, category, reference_id, subject)
             VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(user_id)
        .bind(category)
        .bind(reference_id)
        .bind(subject)
        .fetch_one(&mut *tx)
        .await?;

        let msg = sqlx::query_as::<_, crate::models::ThreadMessage>(
            "INSERT INTO thread_messages (thread_id, from_admin, body)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(thread.id)
        .bind(from_admin)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;

        for att in attachments {
            sqlx::query(
                "INSERT INTO thread_message_attachments (message_id, url, thumb_url) VALUES ($1, $2, $3)"
            )
            .bind(msg.id).bind(&att.url).bind(&att.thumb_url)
            .execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok((thread, msg))
    }

    pub async fn add_thread_reply(
        &self,
        thread_id: Uuid,
        _user_id: Uuid,
        from_admin: bool,
        body: &str,
        attachments: &[crate::models::AttachmentInput],
    ) -> Result<crate::models::ThreadMessage> {
        let mut tx = self.pg_pool.begin().await?;

        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM message_threads WHERE id = $1)")
                .bind(thread_id)
                .fetch_one(&mut *tx)
                .await?;

        if !exists {
            return Err(crate::error::AppError::NotFound(format!(
                "Thread {} not found",
                thread_id
            )));
        }

        let msg = sqlx::query_as::<_, crate::models::ThreadMessage>(
            "INSERT INTO thread_messages (thread_id, from_admin, body)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(thread_id)
        .bind(from_admin)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;

        for att in attachments {
            sqlx::query(
                "INSERT INTO thread_message_attachments (message_id, url, thumb_url) VALUES ($1, $2, $3)"
            )
            .bind(msg.id).bind(&att.url).bind(&att.thumb_url)
            .execute(&mut *tx).await?;
        }

        sqlx::query(
            "UPDATE message_threads SET last_message_at = NOW(), status = 'open' WHERE id = $1",
        )
        .bind(thread_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(msg)
    }

    pub async fn get_user_threads(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(crate::models::MessageThread, i64, Option<String>)>> {
        let rows: Vec<(Uuid, Uuid, String, Option<Uuid>, String, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, i64, Option<String>)> = sqlx::query_as(
            r#"SELECT
                t.id, t.user_id, t.category, t.reference_id, t.subject,
                t.status, t.created_at, t.last_message_at,
                COUNT(m.id) FILTER (WHERE m.read_at IS NULL AND m.from_admin = true)::bigint AS unread,
                (SELECT body FROM thread_messages WHERE thread_id = t.id ORDER BY created_at DESC LIMIT 1) AS preview
            FROM message_threads t
            LEFT JOIN thread_messages m ON m.thread_id = t.id
            WHERE t.user_id = $1
            GROUP BY t.id
            ORDER BY t.last_message_at DESC"#
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool).await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    user_id,
                    category,
                    reference_id,
                    subject,
                    status,
                    created_at,
                    last_message_at,
                    unread,
                    preview,
                )| {
                    let thread = crate::models::MessageThread {
                        id,
                        user_id,
                        category,
                        reference_id,
                        subject,
                        status,
                        created_at,
                        last_message_at,
                    };
                    (thread, unread, preview)
                },
            )
            .collect())
    }

    pub async fn get_thread_messages(
        &self,
        thread_id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<(
        crate::models::MessageThread,
        Vec<crate::models::ThreadMessage>,
    )> {
        let thread = sqlx::query_as::<_, crate::models::MessageThread>(
            "SELECT * FROM message_threads WHERE id = $1",
        )
        .bind(thread_id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| {
            crate::error::AppError::NotFound(format!("Thread {} not found", thread_id))
        })?;

        if let Some(uid) = user_id
            && thread.user_id != uid
        {
            return Err(crate::error::AppError::Unauthorized);
        }

        let messages = sqlx::query_as::<_, crate::models::ThreadMessage>(
            "SELECT * FROM thread_messages WHERE thread_id = $1 ORDER BY created_at ASC",
        )
        .bind(thread_id)
        .fetch_all(&self.pg_pool)
        .await?;

        Ok((thread, messages))
    }

    pub async fn mark_thread_read(&self, thread_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE thread_messages SET read_at = NOW()
             WHERE thread_id = $1 AND from_admin = true AND read_at IS NULL
             AND EXISTS (SELECT 1 FROM message_threads WHERE id = $1 AND user_id = $2)",
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn mark_thread_read_admin(&self, thread_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE thread_messages SET read_at = NOW()
             WHERE thread_id = $1 AND from_admin = false AND read_at IS NULL",
        )
        .bind(thread_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn resolve_thread(&self, thread_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE message_threads SET status = 'resolved' WHERE id = $1")
            .bind(thread_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn reopen_thread(&self, thread_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE message_threads SET status = 'open' WHERE id = $1")
            .bind(thread_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn count_unread_threads(&self, user_id: Uuid) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(DISTINCT t.id)
               FROM message_threads t
               JOIN thread_messages m ON m.thread_id = t.id
               WHERE t.user_id = $1 AND m.from_admin = true AND m.read_at IS NULL"#,
        )
        .bind(user_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row.0)
    }

    pub async fn admin_get_threads(
        &self,
        category: Option<&str>,
        status: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(
        Vec<(
            crate::models::MessageThread,
            crate::models::User,
            i64,
            Option<String>,
        )>,
        i64,
    )> {
        let offset = (page - 1) * per_page;

        // Build the WHERE clause with bound parameters ($1, $2, …) instead of
        // interpolating values into the SQL string.
        let mut conditions: Vec<String> = Vec::new();
        if category.is_some() {
            conditions.push(format!("t.category = ${}", conditions.len() + 1));
        }
        if status.is_some() {
            conditions.push(format!("t.status = ${}", conditions.len() + 1));
        }
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        macro_rules! bind_filters {
            ($q:expr) => {{
                let mut q = $q;
                if let Some(c) = category {
                    q = q.bind(c);
                }
                if let Some(s) = status {
                    q = q.bind(s);
                }
                q
            }};
        }

        let count_sql = format!(
            "SELECT COUNT(DISTINCT t.id) FROM message_threads t {}",
            where_clause
        );
        let (total,): (i64,) = bind_filters!(sqlx::query_as::<_, (i64,)>(&count_sql))
            .fetch_one(&self.pg_pool)
            .await?;

        let main_sql = format!(
            r#"SELECT
                t.id as thread_id, t.user_id, t.category, t.reference_id, t.subject,
                t.status, t.created_at as thread_created_at, t.last_message_at,
                u.id as u_id, u.email, u.display_name, u.visual_password_hash,
                u.admin_notes, u.is_blocked, u.password_reset_token, u.password_reset_expires_at,
                u.created_at as u_created_at, u.avatar_url,
                COUNT(m.id) FILTER (WHERE m.read_at IS NULL AND m.from_admin = false)::bigint AS unread,
                (SELECT body FROM thread_messages WHERE thread_id = t.id ORDER BY created_at DESC LIMIT 1) AS preview
            FROM message_threads t
            JOIN users u ON u.id = t.user_id
            LEFT JOIN thread_messages m ON m.thread_id = t.id
            {}
            GROUP BY t.id, u.id
            ORDER BY t.last_message_at DESC
            LIMIT ${} OFFSET ${}"#,
            where_clause,
            conditions.len() + 1,
            conditions.len() + 2
        );
        let rows = bind_filters!(sqlx::query(&main_sql))
            .bind(per_page)
            .bind(offset)
            .fetch_all(&self.pg_pool)
            .await?;

        use sqlx::Row;
        let items = rows
            .into_iter()
            .map(|r| {
                let thread = crate::models::MessageThread {
                    id: r.get("thread_id"),
                    user_id: r.get("user_id"),
                    category: r.get("category"),
                    reference_id: r.get("reference_id"),
                    subject: r.get("subject"),
                    status: r.get("status"),
                    created_at: r.get("thread_created_at"),
                    last_message_at: r.get("last_message_at"),
                };
                let user = crate::models::User {
                    id: r.get("u_id"),
                    email: r.get("email"),
                    display_name: r.get("display_name"),
                    visual_password_hash: r.get("visual_password_hash"),
                    admin_notes: r.get("admin_notes"),
                    is_blocked: r.get("is_blocked"),
                    password_reset_token: r.get("password_reset_token"),
                    password_reset_expires_at: r.get("password_reset_expires_at"),
                    created_at: r.get("u_created_at"),
                    avatar_url: r.get("avatar_url"),
                    visual_pool: None, // not needed for admin thread listing
                    signup_ip: None,
                    signup_country_code: None,
                    signup_city: None,
                    last_reset_ip: None,
                    last_reset_country_code: None,
                    last_reset_city: None,
                    last_reset_at: None,
                    last_reset_request_ip: None,
                    last_reset_request_country_code: None,
                    last_reset_request_city: None,
                    last_reset_request_at: None,
                    telegram_id: None, // not needed for admin thread listing
                    telegram_username: None,
                    telegram_linked_at: None,
                    email_confirmed_at: None,
                    email_confirm_hash: None,
                    email_confirm_expires_at: None,
                };
                let unread: i64 = r.get("unread");
                let preview: Option<String> = r.get("preview");
                (thread, user, unread, preview)
            })
            .collect();

        Ok((items, total))
    }

    pub async fn get_user_threads_for_admin(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(crate::models::MessageThread, i64, Option<String>)>> {
        let rows: Vec<(Uuid, Uuid, String, Option<Uuid>, String, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, i64, Option<String>)> = sqlx::query_as(
            r#"SELECT
                t.id, t.user_id, t.category, t.reference_id, t.subject,
                t.status, t.created_at, t.last_message_at,
                COUNT(m.id) FILTER (WHERE m.read_at IS NULL AND m.from_admin = false)::bigint AS unread,
                (SELECT body FROM thread_messages WHERE thread_id = t.id ORDER BY created_at DESC LIMIT 1) AS preview
            FROM message_threads t
            LEFT JOIN thread_messages m ON m.thread_id = t.id
            WHERE t.user_id = $1
            GROUP BY t.id
            ORDER BY t.last_message_at DESC"#
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool).await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    user_id,
                    category,
                    reference_id,
                    subject,
                    status,
                    created_at,
                    last_message_at,
                    unread,
                    preview,
                )| {
                    let thread = crate::models::MessageThread {
                        id,
                        user_id,
                        category,
                        reference_id,
                        subject,
                        status,
                        created_at,
                        last_message_at,
                    };
                    (thread, unread, preview)
                },
            )
            .collect())
    }

    // === MESSAGE ATTACHMENTS ===

    pub async fn get_message_attachments(
        &self,
        message_id: Uuid,
    ) -> Result<Vec<crate::models::Attachment>> {
        Ok(sqlx::query_as::<_, crate::models::Attachment>(
            "SELECT id, url, thumb_url FROM thread_message_attachments WHERE message_id = $1 ORDER BY created_at ASC"
        )
        .bind(message_id)
        .fetch_all(&self.pg_pool).await?)
    }

    /// Batch-load attachments for many messages at once (avoids the N+1 of
    /// querying attachments per message when building a thread's message list).
    pub async fn get_attachments_for_messages(
        &self,
        message_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<crate::models::Attachment>>> {
        if message_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>)>(
            "SELECT message_id, id, url, thumb_url FROM thread_message_attachments
             WHERE message_id = ANY($1) ORDER BY message_id, created_at ASC",
        )
        .bind(message_ids)
        .fetch_all(&self.pg_pool)
        .await?;
        let mut out: std::collections::HashMap<Uuid, Vec<crate::models::Attachment>> =
            std::collections::HashMap::new();
        for (message_id, id, url, thumb_url) in rows {
            out.entry(message_id)
                .or_default()
                .push(crate::models::Attachment { id, url, thumb_url });
        }
        Ok(out)
    }

    // === COMMISSIONS ===

    fn generate_claim_token() -> String {
        let a = Uuid::new_v4().to_string().replace('-', "");
        let b = Uuid::new_v4().to_string().replace('-', "");
        format!("{}{}", a, b)
    }

    pub async fn create_commission(
        &self,
        req: &crate::models::CommissionRequest,
    ) -> Result<crate::models::Commission> {
        let deadline = parse_optional_deadline(req.deadline.as_deref())?;
        let claim_token = Self::generate_claim_token();

        let mut tx = self.pg_pool.begin().await?;

        let lang = match req.lang.as_deref() {
            Some("en") => "en",
            _ => "ru",
        };
        let source_figurine_id = req
            .source_figurine_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        let similar_keep_note = req
            .similar_keep_note
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        let similar_change_note = req
            .similar_change_note
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        let similar_tags: Vec<String> = req
            .similar_tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
            .take(12)
            .map(ToOwned::to_owned)
            .collect();
        let commission = sqlx::query_as::<_, crate::models::Commission>(
            r#"INSERT INTO commissions
               (claim_token, requester_name, requester_email, requester_phone,
                title, description, size_note, mood, deadline, budget_note, occasion,
                source_figurine_id, similar_keep_note, similar_change_note, similar_tags, lang)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
               RETURNING *"#,
        )
        .bind(&claim_token)
        .bind(req.requester_name.clone().unwrap_or_default())
        .bind(&req.requester_email)
        .bind(&req.requester_phone)
        .bind(req.title.clone().unwrap_or_default())
        .bind(&req.description)
        .bind(&req.size_note)
        .bind(&req.mood)
        .bind(deadline)
        .bind(&req.budget_note)
        .bind(&req.occasion)
        .bind(&source_figurine_id)
        .bind(&similar_keep_note)
        .bind(&similar_change_note)
        .bind(&similar_tags)
        .bind(lang)
        .fetch_one(&mut *tx)
        .await?;

        for att in &req.attachment_urls {
            sqlx::query(
                "INSERT INTO commission_attachments (commission_id, url, thumb_url) VALUES ($1, $2, $3)"
            )
            .bind(commission.id).bind(&att.url).bind(&att.thumb_url)
            .execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(commission)
    }

    pub async fn get_commission_attachments(
        &self,
        commission_id: Uuid,
    ) -> Result<Vec<crate::models::Attachment>> {
        Ok(sqlx::query_as::<_, crate::models::Attachment>(
            "SELECT id, url, thumb_url FROM commission_attachments WHERE commission_id = $1 ORDER BY created_at ASC"
        )
        .bind(commission_id)
        .fetch_all(&self.pg_pool).await?)
    }

    pub async fn get_commission_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::Commission>> {
        Ok(sqlx::query_as::<_, crate::models::Commission>(
            "SELECT * FROM commissions WHERE claim_token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn get_commission_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::Commission>> {
        Ok(sqlx::query_as::<_, crate::models::Commission>(
            "SELECT * FROM commissions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn get_commissions_page(
        &self,
        status_filter: Option<&str>,
        similar_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<crate::models::Commission>, i64)> {
        let (items, total) = if let Some(status) = status_filter {
            if similar_only {
                let items = sqlx::query_as::<_, crate::models::Commission>(
                    "SELECT * FROM commissions WHERE status = $1::commission_status AND source_figurine_id IS NOT NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(status).bind(limit).bind(offset)
                .fetch_all(&self.pg_pool).await?;
                let (total,): (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM commissions WHERE status = $1::commission_status AND source_figurine_id IS NOT NULL"
                )
                .bind(status)
                .fetch_one(&self.pg_pool).await?;
                (items, total)
            } else {
                let items = sqlx::query_as::<_, crate::models::Commission>(
                    "SELECT * FROM commissions WHERE status = $1::commission_status ORDER BY created_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(status).bind(limit).bind(offset)
                .fetch_all(&self.pg_pool).await?;
                let (total,): (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM commissions WHERE status = $1::commission_status",
                )
                .bind(status)
                .fetch_one(&self.pg_pool)
                .await?;
                (items, total)
            }
        } else if similar_only {
            let items = sqlx::query_as::<_, crate::models::Commission>(
                "SELECT * FROM commissions WHERE source_figurine_id IS NOT NULL ORDER BY created_at DESC LIMIT $1 OFFSET $2"
            )
            .bind(limit).bind(offset)
            .fetch_all(&self.pg_pool).await?;
            let (total,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM commissions WHERE source_figurine_id IS NOT NULL",
            )
            .fetch_one(&self.pg_pool)
            .await?;
            (items, total)
        } else {
            let items = sqlx::query_as::<_, crate::models::Commission>(
                "SELECT * FROM commissions ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pg_pool)
            .await?;
            let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM commissions")
                .fetch_one(&self.pg_pool)
                .await?;
            (items, total)
        };
        Ok((items, total))
    }

    pub async fn get_new_commissions_count(&self) -> Result<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM commissions WHERE status = 'new'")
                .fetch_one(&self.pg_pool)
                .await?;
        Ok(count)
    }

    pub async fn update_commission(
        &self,
        id: Uuid,
        status: &crate::models::CommissionStatus,
        admin_notes: Option<&str>,
        figurine_id: Option<&str>,
    ) -> Result<Option<crate::models::Commission>> {
        Ok(sqlx::query_as::<_, crate::models::Commission>(
            r#"UPDATE commissions
               SET status = $1,
                   admin_notes = COALESCE($2, admin_notes),
                   figurine_id = COALESCE($3, figurine_id),
                   updated_at = NOW()
               WHERE id = $4
               RETURNING *"#,
        )
        .bind(status)
        .bind(admin_notes)
        .bind(figurine_id)
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn update_commission_content(
        &self,
        id: Uuid,
        req: &crate::models::EditCommissionRequest,
    ) -> Result<Option<crate::models::Commission>> {
        let deadline = parse_optional_deadline(req.deadline.as_deref())?;

        let mut tx = self.pg_pool.begin().await?;

        let updated = sqlx::query_as::<_, crate::models::Commission>(
            r#"UPDATE commissions
               SET title = $1, description = $2, size_note = $3, mood = $4,
                   deadline = $5, budget_note = $6, occasion = $7, updated_at = NOW()
               WHERE id = $8
               RETURNING *"#,
        )
        .bind(req.title.clone().unwrap_or_default())
        .bind(&req.description)
        .bind(&req.size_note)
        .bind(&req.mood)
        .bind(deadline)
        .bind(&req.budget_note)
        .bind(&req.occasion)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        // Replace the reference set only when the client sent one.
        if updated.is_some()
            && let Some(attachments) = &req.attachment_urls
        {
            sqlx::query("DELETE FROM commission_attachments WHERE commission_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            for att in attachments {
                sqlx::query(
                        "INSERT INTO commission_attachments (commission_id, url, thumb_url) VALUES ($1, $2, $3)"
                    )
                    .bind(id).bind(&att.url).bind(&att.thumb_url)
                    .execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;
        Ok(updated)
    }

    pub async fn delete_commission(&self, id: Uuid) -> Result<()> {
        let mut tx = self.pg_pool.begin().await?;
        // Remove the linked conversation (messages + attachments cascade via FK).
        sqlx::query(
            "DELETE FROM message_threads WHERE reference_id = $1 AND category = 'commission'",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM commissions WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Attach a guest commission to a user account. Returns the commission if the
    /// token matched and it was previously unclaimed (or already owned by this user).
    pub async fn claim_commission(
        &self,
        token: &str,
        user_id: Uuid,
    ) -> Result<Option<crate::models::Commission>> {
        Ok(sqlx::query_as::<_, crate::models::Commission>(
            r#"UPDATE commissions
               SET user_id = $1, updated_at = NOW()
               WHERE claim_token = $2 AND (user_id IS NULL OR user_id = $1)
               RETURNING *"#,
        )
        .bind(user_id)
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn get_user_commissions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::Commission>> {
        Ok(sqlx::query_as::<_, crate::models::Commission>(
            "SELECT * FROM commissions WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Claim tokens of unclaimed petitions whose email matches an account — so they
    /// can be adopted (each via claim_commission, which also seeds its thread).
    pub async fn orphan_commission_tokens_by_email(&self, email: &str) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT claim_token FROM commissions WHERE user_id IS NULL AND lower(requester_email) = lower($1)"
        )
        .bind(email)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// Find an existing thread linked to a commission (category 'commission').
    pub async fn find_thread_by_reference(
        &self,
        reference_id: Uuid,
        category: &str,
    ) -> Result<Option<crate::models::MessageThread>> {
        Ok(sqlx::query_as::<_, crate::models::MessageThread>(
            "SELECT * FROM message_threads WHERE reference_id = $1 AND category = $2 ORDER BY created_at ASC LIMIT 1"
        )
        .bind(reference_id).bind(category)
        .fetch_optional(&self.pg_pool).await?)
    }

    // === CABINET GAZETTE ===

    const GAZETTE_LEAF_SELECT: &'static str = r#"
        SELECT l.id, l.slug, l.kind, l.status,
               l.title_en, l.title_ru, l.dek_en, l.dek_ru, l.body_en, l.body_ru,
               l.figurine_id, l.href, l.source_name, l.source_url, l.author, l.image_url, l.image_urls,
               l.pinned, l.shelf_order, l.published_at, l.scheduled_at, l.expected_from, l.expected_to,
               l.created_at, l.updated_at,
               f.name AS figurine_name, f.slug AS figurine_slug,
               f.status::text AS figurine_status,
               (SELECT COUNT(*) FROM gazette_watches w WHERE w.leaf_id = l.id)::bigint AS watch_count
        FROM gazette_leaves l
        LEFT JOIN figurines f ON f.id = l.figurine_id
    "#;

    pub async fn list_gazette_leaves_public(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<crate::models::GazetteLeafListed>, i64)> {
        let items = sqlx::query_as::<_, crate::models::GazetteLeafListed>(&format!(
            "{} WHERE (l.status = 'published'
                  OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))
             ORDER BY l.pinned DESC,
                      COALESCE(l.published_at, l.scheduled_at, l.created_at) DESC
             LIMIT $1 OFFSET $2",
            Self::GAZETTE_LEAF_SELECT
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pg_pool)
        .await?;
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM gazette_leaves l
             WHERE (l.status = 'published'
                OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))",
        )
        .fetch_one(&self.pg_pool)
        .await?;
        Ok((items, total))
    }

    /// The shelf of tall tales, in the order the keeper arranged it by hand.
    ///
    /// `pinned` deliberately does not sort here the way it sorts the gazette:
    /// on a hand-arranged shelf it would fight the dragging. It only marks the
    /// one tale the room shows large, and the room picks that itself.
    pub async fn list_tales_public(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::models::GazetteLeafListed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteLeafListed>(&format!(
            "{} WHERE l.kind = 'tale'
               AND (l.status = 'published'
                    OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))
             ORDER BY l.shelf_order NULLS LAST,
                      COALESCE(l.published_at, l.scheduled_at, l.created_at) DESC,
                      l.id DESC
             LIMIT $1",
            Self::GAZETTE_LEAF_SELECT
        ))
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Rewrite the whole shelf in one statement. Ids that are not tales are
    /// ignored rather than rejected, so a stale tab cannot renumber the vestnik.
    /// `updated_at` is left alone on purpose: the writing desk compares it
    /// against its local draft, and a reorder is not a rewrite of the prose.
    pub async fn set_tale_shelf_order(&self, ids: &[Uuid]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let orders: Vec<i32> = (0..ids.len() as i32).collect();
        let res = sqlx::query(
            "UPDATE gazette_leaves AS l
                SET shelf_order = v.ord
               FROM (SELECT * FROM UNNEST($1::uuid[], $2::int[]) AS t(id, ord)) AS v
              WHERE l.id = v.id AND l.kind = 'tale'",
        )
        .bind(ids)
        .bind(&orders)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    pub async fn list_gazette_leaves_home(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::models::GazetteLeafListed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteLeafListed>(&format!(
            "{} WHERE (l.status = 'published'
                  OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))
             ORDER BY l.pinned DESC,
                      COALESCE(l.published_at, l.scheduled_at, l.created_at) DESC
             LIMIT $1",
            Self::GAZETTE_LEAF_SELECT
        ))
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_gazette_leaf_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<crate::models::GazetteLeafListed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteLeafListed>(&format!(
            "{} WHERE l.slug = $1
               AND (l.status = 'published'
                    OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))",
            Self::GAZETTE_LEAF_SELECT
        ))
        .bind(slug)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn list_gazette_leaves_admin(
        &self,
        status: Option<&str>,
        kind: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<crate::models::GazetteLeafListed>, i64)> {
        let mut where_sql = String::from("WHERE 1=1");
        if status.is_some() {
            where_sql.push_str(" AND l.status = $1");
        }
        if kind.is_some() {
            where_sql.push_str(if status.is_some() {
                " AND l.kind = $2"
            } else {
                " AND l.kind = $1"
            });
        }
        let query = format!(
            "{} {where_sql} ORDER BY l.updated_at DESC LIMIT ${} OFFSET ${}",
            Self::GAZETTE_LEAF_SELECT,
            if status.is_some() && kind.is_some() {
                3
            } else if status.is_some() || kind.is_some() {
                2
            } else {
                1
            },
            if status.is_some() && kind.is_some() {
                4
            } else if status.is_some() || kind.is_some() {
                3
            } else {
                2
            },
        );
        let mut q = sqlx::query_as::<_, crate::models::GazetteLeafListed>(&query);
        if let Some(s) = status {
            q = q.bind(s);
        }
        if let Some(k) = kind {
            q = q.bind(k);
        }
        let items = q.bind(limit).bind(offset).fetch_all(&self.pg_pool).await?;

        let count_sql = format!("SELECT COUNT(*) FROM gazette_leaves l {where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(s) = status {
            cq = cq.bind(s);
        }
        if let Some(k) = kind {
            cq = cq.bind(k);
        }
        let (total,) = cq.fetch_one(&self.pg_pool).await?;
        Ok((items, total))
    }

    pub async fn get_gazette_leaf_admin(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::GazetteLeafListed>> {
        Ok(
            sqlx::query_as::<_, crate::models::GazetteLeafListed>(&format!(
                "{} WHERE l.id = $1",
                Self::GAZETTE_LEAF_SELECT
            ))
            .bind(id)
            .fetch_optional(&self.pg_pool)
            .await?,
        )
    }

    pub async fn list_gazette_slugs_except(&self, except: Option<Uuid>) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = if let Some(id) = except {
            sqlx::query_as("SELECT slug FROM gazette_leaves WHERE id <> $1")
                .bind(id)
                .fetch_all(&self.pg_pool)
                .await?
        } else {
            sqlx::query_as("SELECT slug FROM gazette_leaves")
                .fetch_all(&self.pg_pool)
                .await?
        };
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn insert_gazette_leaf(
        &self,
        slug: &str,
        kind: &str,
        status: &str,
        title_en: &str,
        title_ru: &str,
        dek_en: Option<&str>,
        dek_ru: Option<&str>,
        body_en: Option<&str>,
        body_ru: Option<&str>,
        figurine_id: Option<Uuid>,
        href: Option<&str>,
        source_name: Option<&str>,
        source_url: Option<&str>,
        author: Option<&str>,
        image_url: Option<&str>,
        image_urls: &[String],
        pinned: bool,
        published_at: Option<DateTime<Utc>>,
        scheduled_at: Option<DateTime<Utc>>,
        expected_from: Option<NaiveDate>,
        expected_to: Option<NaiveDate>,
    ) -> Result<crate::models::GazetteLeaf> {
        Ok(sqlx::query_as::<_, crate::models::GazetteLeaf>(
            r#"INSERT INTO gazette_leaves (
                    slug, kind, status, title_en, title_ru, dek_en, dek_ru, body_en, body_ru,
                    figurine_id, href, source_name, source_url, author, image_url, image_urls,
                    pinned, published_at, scheduled_at, expected_from, expected_to
               ) VALUES (
                    $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21
               ) RETURNING *"#,
        )
        .bind(slug)
        .bind(kind)
        .bind(status)
        .bind(title_en)
        .bind(title_ru)
        .bind(dek_en)
        .bind(dek_ru)
        .bind(body_en)
        .bind(body_ru)
        .bind(figurine_id)
        .bind(href)
        .bind(source_name)
        .bind(source_url)
        .bind(author)
        .bind(image_url)
        .bind(image_urls)
        .bind(pinned)
        .bind(published_at)
        .bind(scheduled_at)
        .bind(expected_from)
        .bind(expected_to)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn update_gazette_leaf(
        &self,
        id: Uuid,
        slug: &str,
        kind: &str,
        status: &str,
        title_en: &str,
        title_ru: &str,
        dek_en: Option<&str>,
        dek_ru: Option<&str>,
        body_en: Option<&str>,
        body_ru: Option<&str>,
        figurine_id: Option<Uuid>,
        href: Option<&str>,
        source_name: Option<&str>,
        source_url: Option<&str>,
        author: Option<&str>,
        image_url: Option<&str>,
        image_urls: &[String],
        pinned: bool,
        published_at: Option<DateTime<Utc>>,
        scheduled_at: Option<DateTime<Utc>>,
        expected_from: Option<NaiveDate>,
        expected_to: Option<NaiveDate>,
    ) -> Result<crate::models::GazetteLeaf> {
        sqlx::query_as::<_, crate::models::GazetteLeaf>(
            r#"UPDATE gazette_leaves SET
                    slug = $2, kind = $3, status = $4,
                    title_en = $5, title_ru = $6, dek_en = $7, dek_ru = $8,
                    body_en = $9, body_ru = $10, figurine_id = $11, href = $12,
                    source_name = $13, source_url = $14, author = $15,
                    image_url = $16, image_urls = $17,
                    pinned = $18, published_at = $19, scheduled_at = $20,
                    expected_from = $21, expected_to = $22,
                    updated_at = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .bind(slug)
        .bind(kind)
        .bind(status)
        .bind(title_en)
        .bind(title_ru)
        .bind(dek_en)
        .bind(dek_ru)
        .bind(body_en)
        .bind(body_ru)
        .bind(figurine_id)
        .bind(href)
        .bind(source_name)
        .bind(source_url)
        .bind(author)
        .bind(image_url)
        .bind(image_urls)
        .bind(pinned)
        .bind(published_at)
        .bind(scheduled_at)
        .bind(expected_from)
        .bind(expected_to)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Gazette leaf {id} not found")))
    }

    pub async fn delete_gazette_leaf(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM gazette_leaves WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Gazette leaf {id} not found")));
        }
        Ok(())
    }

    pub async fn upsert_gazette_watch(
        &self,
        leaf_id: Uuid,
        email: &str,
        name: Option<&str>,
        lang: &str,
        user_id: Option<Uuid>,
    ) -> Result<(crate::models::GazetteWatch, bool)> {
        let existing = sqlx::query_as::<_, crate::models::GazetteWatch>(
            "SELECT * FROM gazette_watches WHERE leaf_id = $1 AND lower(email) = lower($2) LIMIT 1",
        )
        .bind(leaf_id)
        .bind(email)
        .fetch_optional(&self.pg_pool)
        .await?;
        if let Some(ex) = existing {
            let rec = sqlx::query_as::<_, crate::models::GazetteWatch>(
                "UPDATE gazette_watches SET
                        name = COALESCE($2, name),
                        lang = $3,
                        user_id = COALESCE($4, user_id)
                 WHERE id = $1
                 RETURNING *",
            )
            .bind(ex.id)
            .bind(name)
            .bind(lang)
            .bind(user_id)
            .fetch_one(&self.pg_pool)
            .await?;
            return Ok((rec, true));
        }
        let token = Self::generate_cancel_token();
        let rec = sqlx::query_as::<_, crate::models::GazetteWatch>(
            "INSERT INTO gazette_watches (leaf_id, email, name, lang, cancel_token, user_id)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *",
        )
        .bind(leaf_id)
        .bind(email)
        .bind(name)
        .bind(lang)
        .bind(&token)
        .bind(user_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok((rec, false))
    }

    pub async fn get_gazette_watch_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::GazetteWatch>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteWatch>(
            "SELECT * FROM gazette_watches WHERE cancel_token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn delete_gazette_watch_by_token(&self, token: &str) -> Result<()> {
        sqlx::query("DELETE FROM gazette_watches WHERE cancel_token = $1")
            .bind(token)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn list_gazette_watches_for_figurine(
        &self,
        figurine_id: Uuid,
    ) -> Result<Vec<crate::models::GazetteWatch>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteWatch>(
            "SELECT w.* FROM gazette_watches w
             JOIN gazette_leaves l ON l.id = w.leaf_id
             WHERE l.figurine_id = $1
             ORDER BY w.created_at ASC",
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn list_unnotified_gazette_watches_for_figurine(
        &self,
        figurine_id: Uuid,
    ) -> Result<Vec<crate::models::GazetteWatch>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteWatch>(
            "SELECT w.* FROM gazette_watches w
             JOIN gazette_leaves l ON l.id = w.leaf_id
             WHERE l.figurine_id = $1 AND w.notified_at IS NULL
             ORDER BY w.created_at ASC",
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn mark_gazette_watches_notified_for_figurine(
        &self,
        figurine_id: Uuid,
        emails: &[String],
    ) -> Result<()> {
        if emails.is_empty() {
            return Ok(());
        }
        sqlx::query(
            "UPDATE gazette_watches w
             SET notified_at = NOW()
             FROM gazette_leaves l
             WHERE w.leaf_id = l.id
               AND l.figurine_id = $1
               AND w.notified_at IS NULL
               AND lower(w.email) = ANY($2::text[])",
        )
        .bind(figurine_id)
        .bind(emails)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn list_gazette_watches_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::GazetteWatchListed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteWatchListed>(
            "SELECT w.id, w.leaf_id, l.slug AS leaf_slug, l.title_en, l.title_ru,
                    w.cancel_token, w.notified_at, w.created_at
             FROM gazette_watches w
             JOIN gazette_leaves l ON l.id = w.leaf_id
             WHERE w.user_id = $1
             ORDER BY w.created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn link_gazette_watches_to_user(&self, user_id: Uuid, email: &str) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE gazette_watches SET user_id = $1
             WHERE user_id IS NULL AND lower(email) = lower($2)",
        )
        .bind(user_id)
        .bind(email)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    pub async fn list_gazette_feeds(&self) -> Result<Vec<crate::models::GazetteFeed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteFeed>(
            "SELECT * FROM gazette_feeds ORDER BY title",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn insert_gazette_feed(
        &self,
        title: &str,
        url: &str,
        enabled: bool,
        mark_key: &str,
    ) -> Result<crate::models::GazetteFeed> {
        Ok(sqlx::query_as::<_, crate::models::GazetteFeed>(
            "INSERT INTO gazette_feeds (title, url, enabled, mark_key) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(title)
        .bind(url)
        .bind(enabled)
        .bind(mark_key)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn update_gazette_feed(
        &self,
        id: Uuid,
        title: &str,
        url: &str,
        enabled: bool,
        mark_key: &str,
        mark_url: Option<&str>,
    ) -> Result<crate::models::GazetteFeed> {
        sqlx::query_as::<_, crate::models::GazetteFeed>(
            "UPDATE gazette_feeds SET title = $2, url = $3, enabled = $4, mark_key = $5, mark_url = $6 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(title)
        .bind(url)
        .bind(enabled)
        .bind(mark_key)
        .bind(mark_url)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Gazette feed {id} not found")))
    }

    pub async fn delete_gazette_feed(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM gazette_feeds WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Gazette feed {id} not found")));
        }
        Ok(())
    }

    pub async fn mark_gazette_feed_fetched(&self, id: Uuid, error: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE gazette_feeds SET last_fetched_at = NOW(), last_error = $2 WHERE id = $1",
        )
        .bind(id)
        .bind(error)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_gazette_cuttings(
        &self,
        feed_id: Uuid,
        items: &[crate::gazette::ParsedFeedItem],
    ) -> Result<i64> {
        let mut imported: i64 = 0;
        for item in items {
            let res = sqlx::query(
                r#"INSERT INTO gazette_cuttings (feed_id, guid, title, url, summary, published_at)
                   VALUES ($1, $2, $3, $4, $5, $6)
                   ON CONFLICT (feed_id, guid) DO UPDATE SET
                     title = EXCLUDED.title,
                     url = EXCLUDED.url,
                     summary = EXCLUDED.summary,
                     published_at = COALESCE(EXCLUDED.published_at, gazette_cuttings.published_at)
                   WHERE gazette_cuttings.title IS DISTINCT FROM EXCLUDED.title
                      OR gazette_cuttings.url IS DISTINCT FROM EXCLUDED.url
                      OR gazette_cuttings.summary IS DISTINCT FROM EXCLUDED.summary"#,
            )
            .bind(feed_id)
            .bind(&item.guid)
            .bind(&item.title)
            .bind(&item.url)
            .bind(if item.summary.is_empty() {
                None
            } else {
                Some(item.summary.as_str())
            })
            .bind(item.published_at)
            .execute(&self.pg_pool)
            .await?;
            imported += res.rows_affected() as i64;
        }
        Ok(imported)
    }

    pub async fn list_gazette_cuttings_public(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::models::GazetteCuttingListed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteCuttingListed>(
            r#"SELECT c.id, c.feed_id, c.guid, c.title, c.url, c.summary,
                      c.published_at, c.dismissed, c.pinned, c.created_at,
                      f.title AS source_name,
                      COALESCE(f.mark_key, 'letter') AS mark_key,
                      f.mark_url
               FROM gazette_cuttings c
               JOIN gazette_feeds f ON f.id = c.feed_id
               WHERE NOT c.dismissed AND c.pinned AND f.enabled
               ORDER BY c.published_at DESC NULLS LAST, c.created_at DESC
               LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn list_gazette_leaves_year(
        &self,
        year: i32,
        limit: i64,
    ) -> Result<Vec<crate::models::GazetteLeafListed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteLeafListed>(&format!(
            "{} WHERE (l.status = 'published'
                  OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))
             AND EXTRACT(YEAR FROM COALESCE(l.published_at, l.scheduled_at, l.created_at))::int = $1
             ORDER BY l.pinned DESC,
                      COALESCE(l.published_at, l.scheduled_at, l.created_at) DESC
             LIMIT $2",
            Self::GAZETTE_LEAF_SELECT
        ))
        .bind(year)
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn list_gazette_cuttings_year(
        &self,
        year: i32,
        limit: i64,
    ) -> Result<Vec<crate::models::GazetteCuttingListed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteCuttingListed>(
            r#"SELECT c.id, c.feed_id, c.guid, c.title, c.url, c.summary,
                      c.published_at, c.dismissed, c.pinned, c.created_at,
                      f.title AS source_name,
                      COALESCE(f.mark_key, 'letter') AS mark_key,
                      f.mark_url
               FROM gazette_cuttings c
               JOIN gazette_feeds f ON f.id = c.feed_id
               WHERE NOT c.dismissed AND c.pinned AND f.enabled
                 AND EXTRACT(YEAR FROM COALESCE(c.published_at, c.created_at))::int = $1
               ORDER BY c.published_at DESC NULLS LAST, c.created_at DESC
               LIMIT $2"#,
        )
        .bind(year)
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn list_gazette_years(&self) -> Result<Vec<i32>> {
        Ok(sqlx::query_scalar::<_, i32>(
            r#"SELECT y FROM (
                 SELECT EXTRACT(YEAR FROM COALESCE(l.published_at, l.scheduled_at, l.created_at))::int AS y
                   FROM gazette_leaves l
                  WHERE (l.status = 'published'
                     OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))
                 UNION
                 SELECT EXTRACT(YEAR FROM COALESCE(c.published_at, c.created_at))::int AS y
                   FROM gazette_cuttings c
                   JOIN gazette_feeds f ON f.id = c.feed_id
                  WHERE NOT c.dismissed AND c.pinned AND f.enabled
               ) years
               WHERE y IS NOT NULL
               ORDER BY y DESC"#,
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn list_gazette_leaves_for_figurine(
        &self,
        figurine_id: Uuid,
        limit: i64,
    ) -> Result<Vec<crate::models::GazetteLeafListed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteLeafListed>(&format!(
            "{} WHERE (l.status = 'published'
                  OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))
             AND l.figurine_id = $1
             ORDER BY COALESCE(l.published_at, l.scheduled_at, l.created_at) DESC
             LIMIT $2",
            Self::GAZETTE_LEAF_SELECT
        ))
        .bind(figurine_id)
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Who stands on either side of a leaf.
    ///
    /// A tall tale looks along its own shelf, in the order the keeper arranged
    /// it; every other leaf looks along the gazette in time. Without the fork,
    /// "next along the shelf" would walk a reader out of the room and into an
    /// announcement of a showing.
    pub async fn gazette_leaf_neighbors(
        &self,
        slug: &str,
        kind: &str,
    ) -> Result<(
        Option<crate::models::GazetteNeighborDto>,
        Option<crate::models::GazetteNeighborDto>,
    )> {
        #[derive(sqlx::FromRow)]
        struct NeighborRow {
            newer_slug: Option<String>,
            newer_title_en: Option<String>,
            newer_title_ru: Option<String>,
            older_slug: Option<String>,
            older_title_en: Option<String>,
            older_title_ru: Option<String>,
        }

        let tale = kind == "tale";
        // Both halves are house constants, never anything a request carries.
        let scope = if tale { "AND l.kind = 'tale'" } else { "" };
        let walk = if tale {
            "shelf_order NULLS LAST, at DESC, id DESC"
        } else {
            "at DESC, id DESC"
        };

        let row = sqlx::query_as::<_, NeighborRow>(&format!(
            r#"WITH live AS (
                 SELECT l.slug, l.title_en, l.title_ru, l.id, l.shelf_order,
                        COALESCE(l.published_at, l.scheduled_at, l.created_at) AS at
                   FROM gazette_leaves l
                  WHERE (l.status = 'published'
                     OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))
                    {scope}
               ),
               ord AS (
                 SELECT slug,
                        LAG(slug) OVER (ORDER BY {walk}) AS newer_slug,
                        LAG(title_en) OVER (ORDER BY {walk}) AS newer_title_en,
                        LAG(title_ru) OVER (ORDER BY {walk}) AS newer_title_ru,
                        LEAD(slug) OVER (ORDER BY {walk}) AS older_slug,
                        LEAD(title_en) OVER (ORDER BY {walk}) AS older_title_en,
                        LEAD(title_ru) OVER (ORDER BY {walk}) AS older_title_ru
                   FROM live
               )
               SELECT newer_slug, newer_title_en, newer_title_ru,
                      older_slug, older_title_en, older_title_ru
                 FROM ord
                WHERE slug = $1"#,
        ))
        .bind(slug)
        .fetch_optional(&self.pg_pool)
        .await?;

        let Some(row) = row else {
            return Ok((None, None));
        };
        let prev = match (row.newer_slug, row.newer_title_en, row.newer_title_ru) {
            (Some(slug), Some(title_en), Some(title_ru)) => {
                Some(crate::models::GazetteNeighborDto {
                    slug,
                    title_en,
                    title_ru,
                })
            }
            _ => None,
        };
        let next = match (row.older_slug, row.older_title_en, row.older_title_ru) {
            (Some(slug), Some(title_en), Some(title_ru)) => {
                Some(crate::models::GazetteNeighborDto {
                    slug,
                    title_en,
                    title_ru,
                })
            }
            _ => None,
        };
        Ok((prev, next))
    }

    pub async fn list_gazette_cuttings_admin(
        &self,
        bucket: &str,
        feed_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<crate::models::GazetteCuttingListed>, i64)> {
        let bucket_sql = match bucket {
            "table" => "NOT c.dismissed AND c.pinned",
            "aside" => "c.dismissed",
            "all" => "TRUE",
            _ => "NOT c.dismissed AND NOT c.pinned",
        };
        let items = sqlx::query_as::<_, crate::models::GazetteCuttingListed>(&format!(
            r#"SELECT c.id, c.feed_id, c.guid, c.title, c.url, c.summary,
                      c.published_at, c.dismissed, c.pinned, c.created_at,
                      f.title AS source_name,
                      COALESCE(f.mark_key, 'letter') AS mark_key,
                      f.mark_url
               FROM gazette_cuttings c
               JOIN gazette_feeds f ON f.id = c.feed_id
               WHERE {bucket_sql}
                 AND ($1::uuid IS NULL OR c.feed_id = $1)
               ORDER BY c.published_at DESC NULLS LAST, c.created_at DESC
               LIMIT $2 OFFSET $3"#
        ))
        .bind(feed_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pg_pool)
        .await?;
        let (total,): (i64,) = sqlx::query_as(&format!(
            r#"SELECT COUNT(*) FROM gazette_cuttings c
               WHERE {bucket_sql}
                 AND ($1::uuid IS NULL OR c.feed_id = $1)"#
        ))
        .bind(feed_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok((items, total))
    }

    pub async fn get_gazette_cutting(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::GazetteCuttingListed>> {
        Ok(sqlx::query_as::<_, crate::models::GazetteCuttingListed>(
            r#"SELECT c.id, c.feed_id, c.guid, c.title, c.url, c.summary,
                      c.published_at, c.dismissed, c.pinned, c.created_at,
                      f.title AS source_name,
                      COALESCE(f.mark_key, 'letter') AS mark_key,
                      f.mark_url
               FROM gazette_cuttings c
               JOIN gazette_feeds f ON f.id = c.feed_id
               WHERE c.id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn set_gazette_cutting_dismissed(&self, id: Uuid, dismissed: bool) -> Result<()> {
        // Set aside also takes the cutting off the public blotter.
        let sql = if dismissed {
            "UPDATE gazette_cuttings SET dismissed = TRUE, pinned = FALSE WHERE id = $1"
        } else {
            "UPDATE gazette_cuttings SET dismissed = FALSE WHERE id = $1"
        };
        let affected = sqlx::query(sql)
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!(
                "Gazette cutting {id} not found"
            )));
        }
        Ok(())
    }

    pub async fn set_gazette_cutting_pinned(&self, id: Uuid, pinned: bool) -> Result<()> {
        // Pinning puts it on the blotter, so it cannot stay set-aside.
        let sql = if pinned {
            "UPDATE gazette_cuttings SET pinned = TRUE, dismissed = FALSE WHERE id = $1"
        } else {
            "UPDATE gazette_cuttings SET pinned = FALSE WHERE id = $1"
        };
        let affected = sqlx::query(sql)
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!(
                "Gazette cutting {id} not found"
            )));
        }
        Ok(())
    }

    // === БАЙКИ: ОТКЛИК ЧИТАТЕЛЯ ===

    /// Байка существует и её можно читать. Отклик принимается только на такую:
    /// черновик и лист вестника комментировать нечем.
    pub async fn tale_is_live(&self, tale_id: Uuid) -> Result<bool> {
        let (live,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                 SELECT 1 FROM gazette_leaves
                 WHERE id = $1 AND kind = 'tale'
                   AND (status = 'published'
                        OR (status = 'scheduled' AND scheduled_at IS NOT NULL AND scheduled_at <= NOW()))
             )",
        )
        .bind(tale_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(live)
    }

    pub async fn insert_tale_comment(
        &self,
        tale_id: Uuid,
        user_id: Option<Uuid>,
        author_name: &str,
        author_email: Option<&str>,
        body: &str,
    ) -> Result<crate::models::TaleComment> {
        Ok(sqlx::query_as::<_, crate::models::TaleComment>(
            "INSERT INTO tale_comments (tale_id, user_id, author_name, author_email, body)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(tale_id)
        .bind(user_id)
        .bind(author_name)
        .bind(author_email)
        .bind(body)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn get_approved_tale_comments(
        &self,
        tale_id: Uuid,
        newest_first: bool,
    ) -> Result<Vec<crate::models::TaleCommentWithAvatar>> {
        let order = if newest_first { "DESC" } else { "ASC" };
        Ok(
            sqlx::query_as::<_, crate::models::TaleCommentWithAvatar>(&format!(
                "SELECT c.id, c.author_name, c.body, c.admin_reply, c.created_at, u.avatar_url
                 FROM tale_comments c
                 LEFT JOIN users u ON u.id = c.user_id
                 WHERE c.tale_id = $1 AND c.is_approved = true
                 ORDER BY c.created_at {order}"
            ))
            .bind(tale_id)
            .fetch_all(&self.pg_pool)
            .await?,
        )
    }

    /// Очередь стола рассказов. `tale_filter` — отклик на одну байку, иначе всё.
    pub async fn get_tale_comments_admin_page(
        &self,
        only_pending: bool,
        tale_filter: Option<Uuid>,
        newest_first: bool,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<crate::models::AdminTaleCommentRow>, i64, i64)> {
        let order = if newest_first { "DESC" } else { "ASC" };
        let mut where_sql = String::from("WHERE 1=1");
        if only_pending {
            where_sql.push_str(" AND c.is_approved = false");
        }
        if tale_filter.is_some() {
            where_sql.push_str(" AND c.tale_id = $1");
        }
        let (limit_n, offset_n) = if tale_filter.is_some() { (2, 3) } else { (1, 2) };
        let sql = format!(
            "SELECT c.id, c.tale_id,
                    l.title_en AS tale_title_en, l.title_ru AS tale_title_ru, l.slug AS tale_slug,
                    c.author_name, c.author_email, c.body, c.is_approved, c.admin_reply,
                    c.created_at, c.user_id
             FROM tale_comments c
             JOIN gazette_leaves l ON l.id = c.tale_id
             {where_sql}
             ORDER BY c.created_at {order}
             LIMIT ${limit_n} OFFSET ${offset_n}"
        );
        let mut q = sqlx::query_as::<_, crate::models::AdminTaleCommentRow>(&sql);
        if let Some(id) = tale_filter {
            q = q.bind(id);
        }
        let items = q
            .bind(per_page)
            .bind((page - 1) * per_page)
            .fetch_all(&self.pg_pool)
            .await?;

        let count_sql = format!("SELECT COUNT(*) FROM tale_comments c {where_sql}");
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(id) = tale_filter {
            cq = cq.bind(id);
        }
        let (total,): (i64,) = cq.fetch_one(&self.pg_pool).await?;

        let (pending,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tale_comments WHERE is_approved = false")
                .fetch_one(&self.pg_pool)
                .await?;
        Ok((items, total, pending))
    }

    pub async fn moderate_tale_comment(
        &self,
        id: Uuid,
        is_approved: bool,
        admin_reply: Option<&str>,
    ) -> Result<crate::models::TaleComment> {
        sqlx::query_as::<_, crate::models::TaleComment>(
            "UPDATE tale_comments SET is_approved = $1, admin_reply = $2 WHERE id = $3 RETURNING *",
        )
        .bind(is_approved)
        .bind(admin_reply)
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Tale comment {id} not found")))
    }

    pub async fn delete_tale_comment(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM tale_comments WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Tale comment {id} not found")));
        }
        Ok(())
    }

    /// Назначить голос читателя. `value` — 1, −1 или 0 (снять).
    ///
    /// Одна строка на человека и одна сделка: вошедший поглощает свою же
    /// гостевую строку по этому жетону и строку с другого устройства, иначе
    /// два уникальных указателя разошлись бы на первом же входе в дом.
    pub async fn set_tale_vote(
        &self,
        tale_id: Uuid,
        visitor_token: &str,
        user_id: Option<Uuid>,
        value: i16,
    ) -> Result<(i16, i64, i64)> {
        let mut tx = self.pg_pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(Self::like_lock_key(tale_id))
            .execute(&mut *tx)
            .await?;

        if let Some(uid) = user_id {
            sqlx::query(
                "DELETE FROM tale_likes
                 WHERE tale_id = $1 AND (visitor_token = $2 OR user_id = $3)",
            )
            .bind(tale_id)
            .bind(visitor_token)
            .bind(uid)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query("DELETE FROM tale_likes WHERE tale_id = $1 AND visitor_token = $2")
                .bind(tale_id)
                .bind(visitor_token)
                .execute(&mut *tx)
                .await?;
        }

        if value != 0 {
            sqlx::query(
                "INSERT INTO tale_likes (tale_id, visitor_token, user_id, value)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(tale_id)
            .bind(visitor_token)
            .bind(user_id)
            .bind(value)
            .execute(&mut *tx)
            .await?;
        }

        let (likes, dislikes): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*) FILTER (WHERE value = 1)::bigint,
                    COUNT(*) FILTER (WHERE value = -1)::bigint
             FROM tale_likes WHERE tale_id = $1",
        )
        .bind(tale_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((value, likes, dislikes))
    }

    /// Просмотр. Один на читателя в сутки — повторное открытие ничего не пишет.
    pub async fn record_tale_view(&self, tale_id: Uuid, visitor_token: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO tale_views (tale_id, visitor_token)
             VALUES ($1, $2)
             ON CONFLICT (tale_id, visitor_token, seen_on) DO NOTHING",
        )
        .bind(tale_id)
        .bind(visitor_token)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Числа под одной байкой. `visitor_token` называет голос спрашивающего —
    /// без него страница не смогла бы показать, что читатель уже ответил.
    pub async fn tale_stats(
        &self,
        tale_id: Uuid,
        visitor_token: Option<&str>,
    ) -> Result<(i64, i64, i64, i64, Option<i16>)> {
        let (views, likes, dislikes, comments): (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT
                 (SELECT COUNT(*) FROM tale_views WHERE tale_id = $1)::bigint,
                 (SELECT COUNT(*) FROM tale_likes WHERE tale_id = $1 AND value = 1)::bigint,
                 (SELECT COUNT(*) FROM tale_likes WHERE tale_id = $1 AND value = -1)::bigint,
                 (SELECT COUNT(*) FROM tale_comments WHERE tale_id = $1 AND is_approved)::bigint",
        )
        .bind(tale_id)
        .fetch_one(&self.pg_pool)
        .await?;
        let mine = match visitor_token {
            None => None,
            Some(token) => sqlx::query_as::<_, (i16,)>(
                "SELECT value FROM tale_likes WHERE tale_id = $1 AND visitor_token = $2",
            )
            .bind(tale_id)
            .bind(token)
            .fetch_optional(&self.pg_pool)
            .await?
            .map(|r| r.0),
        };
        Ok((views, likes, dislikes, comments, mine))
    }

    /// Свод по всем байкам разом — стол рассказов печатает его строкой списка.
    pub async fn tale_stats_all(&self) -> Result<Vec<crate::models::AdminTaleStatRow>> {
        Ok(sqlx::query_as::<_, crate::models::AdminTaleStatRow>(
            "SELECT l.id::text AS tale_id,
                    (SELECT COUNT(*) FROM tale_views v WHERE v.tale_id = l.id)::bigint AS views,
                    (SELECT COUNT(*) FROM tale_likes k WHERE k.tale_id = l.id AND k.value = 1)::bigint AS likes,
                    (SELECT COUNT(*) FROM tale_likes k WHERE k.tale_id = l.id AND k.value = -1)::bigint AS dislikes,
                    (SELECT COUNT(*) FROM tale_comments c WHERE c.tale_id = l.id AND c.is_approved)::bigint AS comments,
                    (SELECT COUNT(*) FROM tale_comments c WHERE c.tale_id = l.id AND NOT c.is_approved)::bigint AS pending_comments,
                    l.sequel_of::text AS sequel_of,
                    (SELECT COUNT(*) FROM tale_sequel_wishes w WHERE w.tale_id = l.id)::bigint AS sequel_wishes,
                    (SELECT COUNT(*) FROM tale_sequel_wishes w
                      WHERE w.tale_id = l.id AND w.email IS NOT NULL)::bigint AS sequel_letters
             FROM gazette_leaves l
             WHERE l.kind = 'tale'",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    // === БАЙКИ: ЧИТАТЕЛЬ ПОСЛЕ ПОСЛЕДНЕЙ СТРОКИ ===

    /// Продолжение и начало одной байки, если они на людях, и просьба этого
    /// читателя: `None` — читатель не назван жетоном; `Some(None)` — не
    /// просил; `Some(Some((письмо, записка)))` — просил, и ждёт ли письма и
    /// записки в Telegram. Они отдаются вместе с просьбой, иначе вернувшийся
    /// читатель видел бы приглашение оставить адрес, который уже оставил.
    pub async fn tale_sequel_bits(
        &self,
        tale_id: Uuid,
        visitor_token: Option<&str>,
    ) -> Result<(
        Option<crate::models::GazetteNeighborDto>,
        Option<crate::models::GazetteNeighborDto>,
        Option<Option<(bool, bool)>>,
    )> {
        let sequel: Option<(String, String, String)> = sqlx::query_as(
            "SELECT slug, title_en, title_ru FROM gazette_leaves
              WHERE sequel_of = $1 AND kind = 'tale'
                AND (status = 'published'
                     OR (status = 'scheduled' AND scheduled_at IS NOT NULL AND scheduled_at <= NOW()))
              ORDER BY COALESCE(published_at, scheduled_at, created_at), id
              LIMIT 1",
        )
        .bind(tale_id)
        .fetch_optional(&self.pg_pool)
        .await?;
        let original: Option<(String, String, String)> = sqlx::query_as(
            "SELECT o.slug, o.title_en, o.title_ru
               FROM gazette_leaves l
               JOIN gazette_leaves o ON o.id = l.sequel_of
              WHERE l.id = $1 AND o.kind = 'tale'
                AND (o.status = 'published'
                     OR (o.status = 'scheduled' AND o.scheduled_at IS NOT NULL AND o.scheduled_at <= NOW()))",
        )
        .bind(tale_id)
        .fetch_optional(&self.pg_pool)
        .await?;
        let wants = match visitor_token {
            None => None,
            Some(token) => Some(
                sqlx::query_as::<_, (bool, bool)>(
                    "SELECT email IS NOT NULL, by_telegram FROM tale_sequel_wishes
                      WHERE tale_id = $1 AND visitor_token = $2",
                )
                .bind(tale_id)
                .bind(token)
                .fetch_optional(&self.pg_pool)
                .await?,
            ),
        };
        let neighbor = |(slug, title_en, title_ru): (String, String, String)| {
            crate::models::GazetteNeighborDto {
                slug,
                title_en,
                title_ru,
            }
        };
        Ok((sequel.map(neighbor), original.map(neighbor), wants))
    }

    /// Назначить «хочу продолжение». Возвращает, стоит ли просьба, ждёт ли она
    /// письма и ждёт ли записки в Telegram.
    ///
    /// Вошедший поглощает свои строки с других устройств — так же, как голос
    /// под байкой, — иначе один человек просил бы продолжение дважды. Почта и
    /// просьба о записке не стираются повторным нажатием без них: человек,
    /// оставивший адрес, не должен терять письмо оттого, что нажал кнопку ещё
    /// раз. `telegram` — `None` оставляет решённое как есть.
    pub async fn set_tale_sequel_wish(
        &self,
        tale_id: Uuid,
        visitor_token: &str,
        user_id: Option<Uuid>,
        want: bool,
        email: Option<&str>,
        telegram: Option<bool>,
        lang: &str,
    ) -> Result<(bool, bool, bool)> {
        let mut tx = self.pg_pool.begin().await?;
        if !want {
            sqlx::query(
                "DELETE FROM tale_sequel_wishes
                  WHERE tale_id = $1 AND (visitor_token = $2 OR ($3::uuid IS NOT NULL AND user_id = $3))",
            )
            .bind(tale_id)
            .bind(visitor_token)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok((false, false, false));
        }

        // Адрес и просьба о записке с другого устройства того же человека
        // переезжают сюда, а не пропадают вместе со строкой.
        let carried: Vec<(Option<String>, bool)> = match user_id {
            None => Vec::new(),
            Some(uid) => sqlx::query_as(
                "DELETE FROM tale_sequel_wishes
                  WHERE tale_id = $1 AND user_id = $2 AND visitor_token <> $3
                  RETURNING email, by_telegram",
            )
            .bind(tale_id)
            .bind(uid)
            .bind(visitor_token)
            .fetch_all(&mut *tx)
            .await?,
        };
        let email = email
            .map(str::to_string)
            .or_else(|| carried.iter().find_map(|(e, _)| e.clone()));
        let telegram = telegram.or_else(|| carried.iter().any(|(_, t)| *t).then_some(true));

        let (has_letter, by_telegram): (bool, bool) = sqlx::query_as(
            "INSERT INTO tale_sequel_wishes (tale_id, visitor_token, user_id, email, by_telegram, lang)
             VALUES ($1, $2, $3, $4, COALESCE($5, FALSE), $6)
             ON CONFLICT (tale_id, visitor_token) DO UPDATE SET
                 user_id = COALESCE(EXCLUDED.user_id, tale_sequel_wishes.user_id),
                 email = COALESCE(EXCLUDED.email, tale_sequel_wishes.email),
                 by_telegram = COALESCE($5, tale_sequel_wishes.by_telegram),
                 lang = EXCLUDED.lang
             RETURNING email IS NOT NULL, by_telegram",
        )
        .bind(tale_id)
        .bind(visitor_token)
        .bind(user_id)
        .bind(email)
        .bind(telegram)
        .bind(lang)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((true, has_letter, by_telegram))
    }

    /// Какую байку продолжает эта. Обе обязаны быть байками — лист вестника
    /// ничьим началом не бывает, — и цепочка не замыкается в круг: байка не
    /// продолжает ни себя, ни ту, что уже продолжает её (прямо или через
    /// другие). В круге каждая страница называла бы соседку разом и началом, и
    /// продолжением, а письма уходили бы в обе стороны.
    pub async fn set_tale_sequel_of(&self, tale_id: Uuid, of: Option<Uuid>) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE gazette_leaves SET sequel_of = $2
              WHERE id = $1 AND kind = 'tale'
                AND ($2::uuid IS NULL
                     OR ($2 <> $1
                         AND EXISTS (SELECT 1 FROM gazette_leaves o
                                      WHERE o.id = $2 AND o.kind = 'tale')
                         AND NOT EXISTS (
                             WITH RECURSIVE up AS (
                                 SELECT sequel_of AS id FROM gazette_leaves WHERE id = $2
                                 UNION
                                 SELECT g.sequel_of FROM gazette_leaves g JOIN up ON g.id = up.id
                             )
                             SELECT 1 FROM up WHERE id = $1)))",
        )
        .bind(tale_id)
        .bind(of)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    // --- Голосование «о ком записать следующую» ---

    pub async fn tale_poll_is_open(&self) -> Result<bool> {
        Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM tale_polls WHERE state = 'open')")
            .fetch_one(&self.pg_pool)
            .await?)
    }

    /// Открыть голосование. Кандидаты — в том порядке, в каком их назвали.
    pub async fn open_tale_poll(&self, figurine_ids: &[Uuid]) -> Result<Uuid> {
        let mut tx = self.pg_pool.begin().await?;
        let id: Uuid = sqlx::query_scalar("INSERT INTO tale_polls (state) VALUES ('open') RETURNING id")
            .fetch_one(&mut *tx)
            .await?;
        for (position, fid) in figurine_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO tale_poll_candidates (poll_id, figurine_id, position)
                 VALUES ($1, $2, $3)",
            )
            .bind(id)
            .bind(fid)
            .bind(position as i16)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    /// Голосование, которое сейчас показывается читателю: открытое, а если
    /// его нет — закрытое с победителем, чья байка ещё не вышла (полка тогда
    /// говорит «пишется»). Исполненное не показывается: сама байка уже на
    /// полке и говорит за себя.
    pub async fn current_tale_poll(&self) -> Result<Option<(Uuid, String, Option<Uuid>)>> {
        Ok(sqlx::query_as(
            "SELECT id, state, winner_figurine_id FROM tale_polls
              WHERE state = 'open'
                 OR (state = 'closed' AND winner_figurine_id IS NOT NULL AND fulfilled_leaf_id IS NULL)
              ORDER BY (state = 'open') DESC, closed_at DESC NULLS LAST, opened_at DESC
              LIMIT 1",
        )
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Кандидаты голосования: лицо и имя, в назначенном порядке.
    pub async fn tale_poll_works(
        &self,
        poll_id: Uuid,
    ) -> Result<Vec<(Uuid, String, Option<String>, Option<String>)>> {
        Ok(sqlx::query_as(
            "SELECT f.id, f.name, f.slug, face.url
               FROM tale_poll_candidates c
               JOIN figurines f ON f.id = c.figurine_id
               LEFT JOIN LATERAL (
                   SELECT COALESCE(i.thumb_path, i.file_path) AS url FROM images i
                    WHERE i.figurine_id = f.id AND i.image_type = 'face'
                    ORDER BY i.sort_order LIMIT 1
               ) face ON TRUE
              WHERE c.poll_id = $1
              ORDER BY c.position",
        )
        .bind(poll_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Выбор этого читателя и то, ждёт ли он письма.
    pub async fn tale_poll_choice(
        &self,
        poll_id: Uuid,
        visitor_token: &str,
    ) -> Result<Option<(Uuid, bool)>> {
        Ok(sqlx::query_as(
            "SELECT figurine_id, email IS NOT NULL FROM tale_poll_votes
              WHERE poll_id = $1 AND visitor_token = $2",
        )
        .bind(poll_id)
        .bind(visitor_token)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Отдать голос. Только пока голосование открыто и только за
    /// выставленную работу — второе держит внешний ключ, первое проверяется
    /// здесь же, до того как что-либо тронуто. `None` — голосование уже не
    /// принимает.
    ///
    /// Проверка стоит первой и держит строку голосования общим замком до
    /// конца сделки. Прежде голоса вошедшего с других устройств удалялись до
    /// неё: голос из устаревшей вкладки в закрытое голосование стирал прежний
    /// голос вместе с адресом, и закрытое голосование теряло голос уже после
    /// итога. Замок же не даёт закрыть голосование посреди записи голоса.
    pub async fn vote_tale_poll(
        &self,
        poll_id: Uuid,
        visitor_token: &str,
        user_id: Option<Uuid>,
        figurine_id: Uuid,
        email: Option<&str>,
        lang: &str,
    ) -> Result<Option<(Uuid, bool)>> {
        let mut tx = self.pg_pool.begin().await?;
        let open: bool = sqlx::query_scalar(
            "SELECT state = 'open' FROM tale_polls WHERE id = $1 FOR SHARE",
        )
        .bind(poll_id)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(false);
        if !open {
            tx.rollback().await?;
            return Ok(None);
        }
        // Вошедший голосует один раз, с какого бы устройства ни пришёл.
        let carried: Option<String> = match user_id {
            None => None,
            Some(uid) => sqlx::query_scalar::<_, Option<String>>(
                "DELETE FROM tale_poll_votes
                  WHERE poll_id = $1 AND user_id = $2 AND visitor_token <> $3
                  RETURNING email",
            )
            .bind(poll_id)
            .bind(uid)
            .bind(visitor_token)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .flatten()
            .next(),
        };
        let email = email.map(str::to_string).or(carried);
        let row: (Uuid, bool) = sqlx::query_as(
            "INSERT INTO tale_poll_votes (poll_id, visitor_token, user_id, figurine_id, email, lang)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (poll_id, visitor_token) DO UPDATE SET
                 figurine_id = EXCLUDED.figurine_id,
                 user_id = COALESCE(EXCLUDED.user_id, tale_poll_votes.user_id),
                 email = COALESCE(EXCLUDED.email, tale_poll_votes.email),
                 lang = EXCLUDED.lang
             RETURNING figurine_id, email IS NOT NULL",
        )
        .bind(poll_id)
        .bind(visitor_token)
        .bind(user_id)
        .bind(figurine_id)
        .bind(email)
        .bind(lang)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(row))
    }

    /// Голосование закрыто, а проголосовавший просит письмо: адрес ложится к
    /// его голосу, выбор не трогается. Не голосовал — `None`.
    pub async fn attach_tale_poll_letter(
        &self,
        poll_id: Uuid,
        visitor_token: &str,
        email: &str,
        lang: &str,
    ) -> Result<Option<(Uuid, bool)>> {
        Ok(sqlx::query_as(
            "UPDATE tale_poll_votes SET email = $3, lang = $4
              WHERE poll_id = $1 AND visitor_token = $2
              RETURNING figurine_id, email IS NOT NULL",
        )
        .bind(poll_id)
        .bind(visitor_token)
        .bind(email)
        .bind(lang)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn tale_poll_has_candidate(&self, poll_id: Uuid, figurine_id: Uuid) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM tale_poll_candidates
                             WHERE poll_id = $1 AND figurine_id = $2)",
        )
        .bind(poll_id)
        .bind(figurine_id)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// Все голосования для стола, новые первыми.
    pub async fn admin_tale_polls(
        &self,
    ) -> Result<
        Vec<(
            Uuid,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<Uuid>,
            Option<String>,
            Option<String>,
            Option<String>,
        )>,
    > {
        Ok(sqlx::query_as(
            "SELECT p.id, p.state, p.opened_at, p.closed_at, p.winner_figurine_id,
                    l.slug, l.title_en, l.title_ru
               FROM tale_polls p
               LEFT JOIN gazette_leaves l ON l.id = p.fulfilled_leaf_id
              ORDER BY p.opened_at DESC
              LIMIT 50",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Кандидаты сразу нескольких голосований со счётом — это видит только
    /// стол. Одним запросом на весь список, а не запросом на голосование:
    /// за год еженедельных голосований их набирается полсотни.
    pub async fn admin_tale_poll_candidates(
        &self,
        poll_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, Uuid, String, Option<String>, i64, i64)>> {
        if poll_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(sqlx::query_as(
            "SELECT c.poll_id, f.id, f.name, face.url,
                    COUNT(v.visitor_token)::bigint,
                    (COUNT(v.visitor_token) FILTER (WHERE v.email IS NOT NULL))::bigint
               FROM tale_poll_candidates c
               JOIN figurines f ON f.id = c.figurine_id
               LEFT JOIN LATERAL (
                   SELECT COALESCE(i.thumb_path, i.file_path) AS url FROM images i
                    WHERE i.figurine_id = f.id AND i.image_type = 'face'
                    ORDER BY i.sort_order LIMIT 1
               ) face ON TRUE
               LEFT JOIN tale_poll_votes v
                      ON v.poll_id = c.poll_id AND v.figurine_id = c.figurine_id
              WHERE c.poll_id = ANY($1)
              GROUP BY c.poll_id, c.position, f.id, f.name, face.url
              ORDER BY c.poll_id, c.position",
        )
        .bind(poll_ids)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Закрыть голосование с победителем. Победитель обязан быть кандидатом,
    /// а закрыть можно только открытое. Возвращает время открытия — по нему
    /// ищется уже вышедшая байка о победителе.
    pub async fn close_tale_poll(
        &self,
        poll_id: Uuid,
        winner: Uuid,
    ) -> Result<Option<DateTime<Utc>>> {
        Ok(sqlx::query_scalar(
            "UPDATE tale_polls SET state = 'closed', winner_figurine_id = $2, closed_at = NOW()
              WHERE id = $1 AND state = 'open'
                AND EXISTS (SELECT 1 FROM tale_poll_candidates
                             WHERE poll_id = $1 AND figurine_id = $2)
              RETURNING opened_at",
        )
        .bind(poll_id)
        .bind(winner)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn delete_tale_poll(&self, poll_id: Uuid) -> Result<bool> {
        let res = sqlx::query("DELETE FROM tale_polls WHERE id = $1")
            .bind(poll_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    // --- Письма о байке ---

    /// Байки, которые вышли на люди не меньше `grace_secs` назад и по которым
    /// письма ещё не разложены. Выдержка — ради того же, ради чего она у
    /// канала: вышедшую байку ещё правят, а письмо не поправишь.
    pub async fn tales_awaiting_letters(&self, grace_secs: i64) -> Result<Vec<Uuid>> {
        Ok(sqlx::query_scalar(
            "SELECT id FROM gazette_leaves
              WHERE kind = 'tale' AND letters_at IS NULL
                AND (status = 'published'
                     OR (status = 'scheduled' AND scheduled_at IS NOT NULL AND scheduled_at <= NOW()))
                AND COALESCE(published_at, scheduled_at, created_at) <= NOW() - make_interval(secs => $1)
              ORDER BY COALESCE(published_at, scheduled_at, created_at)",
        )
        .bind(grace_secs as f64)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Разложить письма по одной байке: кто просил её продолжение, кто
    /// выбирал её работу, кто вписан в книгу дома. Одна сделка и строка
    /// байки под замком, поэтому два прохода разом не разложат её дважды.
    /// Возвращает число заведённых писем.
    pub async fn lay_out_tale_letters(&self, tale_id: Uuid) -> Result<u64> {
        let mut tx = self.pg_pool.begin().await?;
        let row: Option<(Option<Uuid>, Option<Uuid>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT sequel_of, figurine_id, letters_at FROM gazette_leaves
              WHERE id = $1 AND kind = 'tale' FOR UPDATE",
        )
        .bind(tale_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((sequel_of, figurine_id, None)) = row else {
            tx.commit().await?;
            return Ok(0);
        };

        let mut laid = 0;
        if let Some(original) = sequel_of {
            laid += Self::enqueue_sequel_letters_tx(&mut tx, original, tale_id).await?;
        }
        if let Some(work) = figurine_id {
            laid += Self::enqueue_chosen_letters_tx(&mut tx, work, tale_id, None).await?;
        }
        laid += sqlx::query(
            "INSERT INTO tale_letters (tale_id, email, lang, reason)
             SELECT $1, s.email, CASE WHEN s.lang = 'ru' THEN 'ru' ELSE 'en' END, $2
               FROM newsletter_subscribers s
              WHERE s.unsubscribed_at IS NULL
             ON CONFLICT (tale_id, lower(email)) DO NOTHING",
        )
        .bind(tale_id)
        .bind(crate::tales::LetterReason::New.as_str())
        .execute(&mut *tx)
        .await?
        .rows_affected();

        sqlx::query("UPDATE gazette_leaves SET letters_at = NOW() WHERE id = $1")
            .bind(tale_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(laid)
    }

    /// Письма и записки тем, кто просил продолжение `original`, — о байке
    /// `sequel`. Записка заводится на имя, а не на номер чата: номер берётся
    /// в миг отправки. Возвращает, сколько писем и записок заведено.
    async fn enqueue_sequel_letters_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        original: Uuid,
        sequel: Uuid,
    ) -> Result<u64> {
        let letters = sqlx::query(
            "INSERT INTO tale_letters (tale_id, email, lang, reason)
             SELECT DISTINCT ON (lower(w.email)) $2, w.email, w.lang, $3
               FROM tale_sequel_wishes w
              WHERE w.tale_id = $1 AND w.email IS NOT NULL
              ORDER BY lower(w.email), w.created_at
             ON CONFLICT (tale_id, lower(email)) DO NOTHING",
        )
        .bind(original)
        .bind(sequel)
        .bind(crate::tales::LetterReason::Sequel.as_str())
        .execute(&mut **tx)
        .await?
        .rows_affected();
        let notes = sqlx::query(
            "INSERT INTO tale_notes (tale_id, user_id, lang)
             SELECT DISTINCT ON (w.user_id) $2, w.user_id, w.lang
               FROM tale_sequel_wishes w
              WHERE w.tale_id = $1 AND w.by_telegram AND w.user_id IS NOT NULL
              ORDER BY w.user_id, w.created_at
             ON CONFLICT (tale_id, user_id) DO NOTHING",
        )
        .bind(original)
        .bind(sequel)
        .execute(&mut **tx)
        .await?
        .rows_affected();
        Ok(letters + notes)
    }

    /// Письма голосовавшим — о байке `tale` про работу `work`, которую
    /// выбрали читатели. Пишется всем, кто оставил адрес, а не только тем, кто
    /// выбрал победителя: голосовал каждый. Голосование отмечается
    /// исполненным. `poll` — одно определённое голосование; `None` — всякое
    /// закрытое с этим победителем и ещё не исполненное.
    async fn enqueue_chosen_letters_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        work: Uuid,
        tale: Uuid,
        poll: Option<Uuid>,
    ) -> Result<u64> {
        let polls: Vec<Uuid> = sqlx::query_scalar(
            "UPDATE tale_polls SET fulfilled_leaf_id = $2
              WHERE state = 'closed' AND winner_figurine_id = $1 AND fulfilled_leaf_id IS NULL
                AND ($3::uuid IS NULL OR id = $3)
              RETURNING id",
        )
        .bind(work)
        .bind(tale)
        .bind(poll)
        .fetch_all(&mut **tx)
        .await?;
        if polls.is_empty() {
            return Ok(0);
        }
        Ok(sqlx::query(
            "INSERT INTO tale_letters (tale_id, email, lang, reason)
             SELECT DISTINCT ON (lower(v.email)) $2, v.email, v.lang, $3
               FROM tale_poll_votes v
              WHERE v.poll_id = ANY($1) AND v.email IS NOT NULL
              ORDER BY lower(v.email), v.created_at
             ON CONFLICT (tale_id, lower(email)) DO NOTHING",
        )
        .bind(&polls)
        .bind(tale)
        .bind(crate::tales::LetterReason::Chosen.as_str())
        .execute(&mut **tx)
        .await?
        .rows_affected())
    }

    /// Разложены ли письма об этой байке — то есть побывала ли она на людях
    /// дольше выдержки. Тогда её адрес уже стоит в письмах.
    pub async fn tale_letters_laid(&self, tale_id: Uuid) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT letters_at IS NOT NULL FROM gazette_leaves WHERE id = $1 AND kind = 'tale'",
        )
        .bind(tale_id)
        .fetch_optional(&self.pg_pool)
        .await?
        .unwrap_or(false))
    }

    /// Сколько писем и записок завела бы раскладка этой байки сейчас:
    /// `(разложены ли уже, писем, записок)`. Считается теми же выборками, что
    /// и `lay_out_tale_letters`, но без записи, — адрес, попавший в два
    /// повода, считается один раз, как его и отсеет `ON CONFLICT`.
    pub async fn tale_letters_forecast(&self, tale_id: Uuid) -> Result<(bool, i64, i64)> {
        let row: Option<(bool, i64, i64)> = sqlx::query_as(
            "WITH t AS (
                 SELECT id, sequel_of, figurine_id, letters_at IS NOT NULL AS laid
                   FROM gazette_leaves WHERE id = $1 AND kind = 'tale'
             ),
             mail AS (
                 SELECT lower(w.email) AS e FROM tale_sequel_wishes w, t
                  WHERE w.tale_id = t.sequel_of AND w.email IS NOT NULL
                 UNION
                 SELECT lower(v.email) FROM tale_poll_votes v
                   JOIN tale_polls p ON p.id = v.poll_id, t
                  WHERE p.state = 'closed' AND p.winner_figurine_id = t.figurine_id
                    AND p.fulfilled_leaf_id IS NULL AND v.email IS NOT NULL
                 UNION
                 SELECT lower(s.email) FROM newsletter_subscribers s
                  WHERE s.unsubscribed_at IS NULL
             )
             SELECT t.laid,
                    (SELECT COUNT(*) FROM mail)::bigint,
                    (SELECT COUNT(DISTINCT w.user_id) FROM tale_sequel_wishes w
                      WHERE w.tale_id = t.sequel_of AND w.by_telegram
                        AND w.user_id IS NOT NULL)::bigint
               FROM t",
        )
        .bind(tale_id)
        .fetch_optional(&self.pg_pool)
        .await?;
        row.ok_or_else(|| AppError::NotFound("Tale not found".to_string()))
    }

    /// Продолжение назначено байке, по которой письма уже разложены: те, кто
    /// просил продолжение, получают его сейчас. Если письма ещё не
    /// разложены — ничего: раскладка сама найдёт начало.
    pub async fn enqueue_sequel_letters_now(&self, original: Uuid, sequel: Uuid) -> Result<u64> {
        let mut tx = self.pg_pool.begin().await?;
        let laid_out: bool = sqlx::query_scalar(
            "SELECT letters_at IS NOT NULL FROM gazette_leaves WHERE id = $1 AND kind = 'tale'",
        )
        .bind(sequel)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(false);
        let laid = if laid_out {
            Self::enqueue_sequel_letters_tx(&mut tx, original, sequel).await?
        } else {
            0
        };
        tx.commit().await?;
        Ok(laid)
    }

    /// Голосование закрыто, а байка о победителе уже вышла после его
    /// открытия и разложена: письма голосовавшим уходят сейчас. Порядок, в
    /// котором хозяин закрыл голосование и выложил байку, не должен решать,
    /// придут ли письма.
    pub async fn fulfil_tale_poll_now(
        &self,
        poll_id: Uuid,
        work: Uuid,
        opened_at: DateTime<Utc>,
    ) -> Result<u64> {
        let mut tx = self.pg_pool.begin().await?;
        let tale: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM gazette_leaves
              WHERE kind = 'tale' AND figurine_id = $1 AND letters_at IS NOT NULL
                AND (status = 'published'
                     OR (status = 'scheduled' AND scheduled_at IS NOT NULL AND scheduled_at <= NOW()))
                AND COALESCE(published_at, scheduled_at, created_at) >= $2
              ORDER BY COALESCE(published_at, scheduled_at, created_at)
              LIMIT 1",
        )
        .bind(work)
        .bind(opened_at)
        .fetch_optional(&mut *tx)
        .await?;
        let laid = match tale {
            Some(tale) => Self::enqueue_chosen_letters_tx(&mut tx, work, tale, Some(poll_id)).await?,
            None => 0,
        };
        tx.commit().await?;
        Ok(laid)
    }

    /// Взять созревшие письма на отправку — вместе со всем, что нужно для
    /// текста.
    ///
    /// Взятие — одна инструкция: строки выбираются под `FOR UPDATE SKIP
    /// LOCKED` и тут же помечаются `claimed_at`, поэтому вторая копия сервера
    /// на той же базе не получит тех же писем. Взятое, но не ушедшее и не
    /// отказавшее (процесс упал посреди прохода), возвращается в очередь через
    /// `claim_secs`.
    ///
    /// Байка обязана быть на людях: снятую с полки не объявляют, а письмо о
    /// ней ждёт, пока её не вернут. Письмо старше недели не уходит вовсе —
    /// весть, пришедшая через месяц, уже не весть. Письмо книги дома уходит,
    /// только если адрес всё ещё в книге: оно могло ждать в очереди, пока
    /// человек отписывался. Дверь из книги берётся здесь же — у того, кто
    /// просил продолжение и состоит в книге, она тоже есть.
    pub async fn claim_tale_letters(
        &self,
        limit: i64,
        max_attempts: i32,
        claim_secs: i64,
    ) -> Result<Vec<crate::models::TaleLetterRow>> {
        Ok(sqlx::query_as::<_, crate::models::TaleLetterRow>(
            "WITH due AS (
                 SELECT t.tale_id, t.email
                   FROM tale_letters t
                   JOIN gazette_leaves l ON l.id = t.tale_id
                  WHERE t.sent_at IS NULL AND t.attempts < $2
                    AND t.created_at > NOW() - INTERVAL '7 days'
                    AND (t.claimed_at IS NULL OR t.claimed_at <= NOW() - make_interval(secs => $3))
                    AND (l.status = 'published'
                         OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))
                    AND (t.reason <> $4
                         OR EXISTS (SELECT 1 FROM newsletter_subscribers s
                                     WHERE lower(s.email) = lower(t.email) AND s.unsubscribed_at IS NULL))
                  ORDER BY t.created_at, t.email
                  LIMIT $1
                  FOR UPDATE OF t SKIP LOCKED
             ),
             claimed AS (
                 UPDATE tale_letters t SET claimed_at = NOW()
                   FROM due
                  WHERE t.tale_id = due.tale_id AND lower(t.email) = lower(due.email)
                  RETURNING t.tale_id, t.email, t.lang, t.reason, t.created_at
             )
             SELECT c.tale_id, c.email, c.lang, c.reason, s.unsubscribe_token,
                    l.slug, l.title_en, l.title_ru, l.dek_en, l.dek_ru,
                    o.title_en AS original_title_en, o.title_ru AS original_title_ru,
                    f.name AS figurine_name
               FROM claimed c
               JOIN gazette_leaves l ON l.id = c.tale_id
               LEFT JOIN gazette_leaves o ON o.id = l.sequel_of
               LEFT JOIN figurines f ON f.id = l.figurine_id
               LEFT JOIN newsletter_subscribers s
                      ON lower(s.email) = lower(c.email) AND s.unsubscribed_at IS NULL
              ORDER BY c.created_at, c.email",
        )
        .bind(limit)
        .bind(max_attempts)
        .bind(claim_secs as f64)
        .bind(crate::tales::LetterReason::New.as_str())
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn tale_letter_sent(&self, tale_id: Uuid, email: &str) -> Result<()> {
        sqlx::query(
            "UPDATE tale_letters SET sent_at = NOW(), last_error = NULL
              WHERE tale_id = $1 AND lower(email) = lower($2)",
        )
        .bind(tale_id)
        .bind(email)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Отказ почты: попытка засчитана, и письмо сразу возвращается в очередь
    /// — следующий проход попробует снова, не дожидаясь срока взятия.
    pub async fn tale_letter_failed(&self, tale_id: Uuid, email: &str, why: &str) -> Result<()> {
        sqlx::query(
            "UPDATE tale_letters SET attempts = attempts + 1, last_error = $3, claimed_at = NULL
              WHERE tale_id = $1 AND lower(email) = lower($2)",
        )
        .bind(tale_id)
        .bind(email)
        .bind(why)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Взять созревшие записки в Telegram — тем же взятием, что и письма
    /// (`claim_tale_letters`). Номер чата берётся из имени сейчас: пусто —
    /// Telegram от имени отвязали, и служба отметит записку недошедшей.
    pub async fn claim_tale_notes(
        &self,
        limit: i64,
        max_attempts: i32,
        claim_secs: i64,
    ) -> Result<Vec<crate::models::TaleNoteRow>> {
        Ok(sqlx::query_as::<_, crate::models::TaleNoteRow>(
            "WITH due AS (
                 SELECT n.tale_id, n.user_id
                   FROM tale_notes n
                   JOIN gazette_leaves l ON l.id = n.tale_id
                  WHERE n.sent_at IS NULL AND n.attempts < $2
                    AND n.created_at > NOW() - INTERVAL '7 days'
                    AND (n.claimed_at IS NULL OR n.claimed_at <= NOW() - make_interval(secs => $3))
                    AND (l.status = 'published'
                         OR (l.status = 'scheduled' AND l.scheduled_at IS NOT NULL AND l.scheduled_at <= NOW()))
                  ORDER BY n.created_at
                  LIMIT $1
                  FOR UPDATE OF n SKIP LOCKED
             ),
             claimed AS (
                 UPDATE tale_notes n SET claimed_at = NOW()
                   FROM due
                  WHERE n.tale_id = due.tale_id AND n.user_id = due.user_id
                  RETURNING n.tale_id, n.user_id, n.lang, n.created_at
             )
             SELECT c.tale_id, c.user_id, u.telegram_id, c.lang,
                    l.slug, l.title_en, l.title_ru,
                    o.title_en AS original_title_en, o.title_ru AS original_title_ru
               FROM claimed c
               JOIN gazette_leaves l ON l.id = c.tale_id
               LEFT JOIN gazette_leaves o ON o.id = l.sequel_of
               LEFT JOIN users u ON u.id = c.user_id
              ORDER BY c.created_at",
        )
        .bind(limit)
        .bind(max_attempts)
        .bind(claim_secs as f64)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn tale_note_sent(&self, tale_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE tale_notes SET sent_at = NOW(), last_error = NULL
              WHERE tale_id = $1 AND user_id = $2",
        )
        .bind(tale_id)
        .bind(user_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Отказ Telegram. `give_up` — ждать нечего (Telegram отвязан): попытки
    /// исчерпываются сразу (`max_attempts`), чтобы записка не крутилась неделю.
    pub async fn tale_note_failed(
        &self,
        tale_id: Uuid,
        user_id: Uuid,
        why: &str,
        give_up: bool,
        max_attempts: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE tale_notes
                SET attempts = CASE WHEN $4 THEN GREATEST(attempts + 1, $5) ELSE attempts + 1 END,
                    last_error = $3, claimed_at = NULL
              WHERE tale_id = $1 AND user_id = $2",
        )
        .bind(tale_id)
        .bind(user_id)
        .bind(why)
        .bind(give_up)
        .bind(max_attempts)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Кто ждёт продолжения этой байки — для стола рассказов, новые первыми.
    ///
    /// Судьба письма и записки ищется по всем вышедшим продолжениям этой байки
    /// (у начала их может быть несколько) и называется словом: `none`,
    /// `pending`, `sent`, `failed`. Письмо ищется по адресу, а не по поводу:
    /// читатель, вписанный в книгу, мог получить то же продолжение письмом
    /// «новая небылица», если продолжением его назвали после раскладки.
    pub async fn admin_sequel_wishes(
        &self,
        tale_id: Uuid,
        max_attempts: i32,
    ) -> Result<Vec<crate::models::AdminSequelWishRow>> {
        Ok(sqlx::query_as::<_, crate::models::AdminSequelWishRow>(
            "WITH seq AS (
                 SELECT id FROM gazette_leaves WHERE sequel_of = $1 AND kind = 'tale'
             )
             SELECT w.created_at, w.email, u.display_name AS name,
                    u.telegram_username, w.by_telegram, w.lang,
                    COALESCE(letter.status, 'none') AS letter,
                    COALESCE(note.status, 'none') AS note
               FROM tale_sequel_wishes w
               LEFT JOIN users u ON u.id = w.user_id
               LEFT JOIN LATERAL (
                   SELECT CASE
                              WHEN t.sent_at IS NOT NULL THEN 'sent'
                              WHEN t.attempts >= $2 OR t.created_at <= NOW() - INTERVAL '7 days' THEN 'failed'
                              ELSE 'pending'
                          END AS status
                     FROM tale_letters t
                    WHERE w.email IS NOT NULL
                      AND t.tale_id IN (SELECT id FROM seq)
                      AND lower(t.email) = lower(w.email)
                    ORDER BY (t.sent_at IS NOT NULL) DESC, t.created_at DESC
                    LIMIT 1
               ) letter ON TRUE
               LEFT JOIN LATERAL (
                   SELECT CASE
                              WHEN n.sent_at IS NOT NULL THEN 'sent'
                              WHEN n.attempts >= $2 OR n.created_at <= NOW() - INTERVAL '7 days' THEN 'failed'
                              ELSE 'pending'
                          END AS status
                     FROM tale_notes n
                    WHERE w.by_telegram
                      AND n.tale_id IN (SELECT id FROM seq)
                      AND n.user_id = w.user_id
                    ORDER BY (n.sent_at IS NOT NULL) DESC, n.created_at DESC
                    LIMIT 1
               ) note ON TRUE
              WHERE w.tale_id = $1
              ORDER BY w.created_at DESC",
        )
        .bind(tale_id)
        .bind(max_attempts)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    // === СКРОМНЫЕ ЭПИЧЕСКИЕ БИТВЫ ===

    /// A card plus what its work lends it. `file_path` is the preview-sized
    /// variant, not the 420px thumb: a card is rendered large enough that the
    /// thumb goes soft, the same reason the detail page reaches for it.
    const BATTLE_CARD_SELECT: &'static str = r#"
        SELECT c.id, c.slug, c.figurine_id, c.race_id, c.status, c.tier,
               c.type_en, c.type_ru,
               c.title_en, c.title_ru, c.effect_en, c.effect_ru, c.lore_en, c.lore_ru,
               c.cost, c.power, c.health, c.mana, c.traits,
               c.kind, c.armor, c.ward, c.attack_channel, c.reach, c.step, c.speed, c.mend,
               c.abilities, c.budget_points, c.balance_index, c.rules_version,
               c.price_dust, c.price_feed, c.level_price_dust,
               c.art_url, c.art_focal, c.frame_override, c.motion_wear,
               c.shelf_order, c.lendable, c.credit_name, c.edition_size, c.minted,
               c.created_at, c.updated_at,
               f.name AS figurine_name, f.slug AS figurine_slug,
               fi.file_path AS figurine_face_path, fi.id AS figurine_face_id,
               r.name_en AS race_name_en, r.name_ru AS race_name_ru, r.icon_url AS race_icon_url,
               r.level_frames AS race_level_frames,
               r.motion_wear AS race_motion_wear
        FROM battle_cards c
        LEFT JOIN figurines f ON f.id = c.figurine_id
        LEFT JOIN battle_races r ON r.id = c.race_id
        LEFT JOIN LATERAL (
            SELECT i.id, i.file_path FROM images i
            WHERE i.figurine_id = c.figurine_id AND i.image_type = 'face'
            ORDER BY i.sort_order LIMIT 1
        ) fi ON TRUE
    "#;

    /// The shelf as guests see it: published cards only, in the keeper's order.
    /// A card the keeper has not arranged falls to the end by rank, so a new
    /// card lands somewhere sensible before anyone drags it anywhere.
    pub async fn list_battle_cards_public(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::models::BattleCardListed>> {
        Ok(
            sqlx::query_as::<_, crate::models::BattleCardListed>(&format!(
                "{} WHERE c.status = 'published'
             ORDER BY c.shelf_order NULLS LAST, c.tier DESC, c.created_at DESC, c.id DESC
             LIMIT $1",
                Self::BATTLE_CARD_SELECT
            ))
            .bind(limit)
            .fetch_all(&self.pg_pool)
            .await?,
        )
    }

    /// The keeper's desk: drafts and retired cards too, same order.
    pub async fn list_battle_cards_admin(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::models::BattleCardListed>> {
        Ok(
            sqlx::query_as::<_, crate::models::BattleCardListed>(&format!(
                "{} ORDER BY c.shelf_order NULLS LAST, c.tier DESC, c.created_at DESC, c.id DESC
             LIMIT $1",
                Self::BATTLE_CARD_SELECT
            ))
            .bind(limit)
            .fetch_all(&self.pg_pool)
            .await?,
        )
    }

    pub async fn get_battle_card_admin(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::BattleCardListed>> {
        Ok(
            sqlx::query_as::<_, crate::models::BattleCardListed>(&format!(
                "{} WHERE c.id = $1",
                Self::BATTLE_CARD_SELECT
            ))
            .bind(id)
            .fetch_optional(&self.pg_pool)
            .await?,
        )
    }

    pub async fn list_battle_card_slugs_except(&self, except: Option<Uuid>) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = if let Some(id) = except {
            sqlx::query_as("SELECT slug FROM battle_cards WHERE id <> $1")
                .bind(id)
                .fetch_all(&self.pg_pool)
                .await?
        } else {
            sqlx::query_as("SELECT slug FROM battle_cards")
                .fetch_all(&self.pg_pool)
                .await?
        };
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// Which card already stands for this work, if any. Checked before writing
    /// so the keeper is told "that work already has a card" instead of being
    /// handed a raw unique-index violation.
    pub async fn battle_card_for_figurine(
        &self,
        figurine_id: Uuid,
        except: Option<Uuid>,
    ) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT title_ru FROM battle_cards
             WHERE figurine_id = $1 AND ($2::uuid IS NULL OR id <> $2)
             LIMIT 1",
        )
        .bind(figurine_id)
        .bind(except)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    /// One struct rather than thirty positional arguments: two neighbouring
    /// `Option<&str>` can be swapped by hand without the compiler saying a word,
    /// and a card whose lore quietly holds its effect is found by a reader
    /// months later.
    pub async fn insert_battle_card(
        &self,
        w: &crate::models::BattleCardWrite,
    ) -> Result<crate::models::BattleCard> {
        Ok(sqlx::query_as::<_, crate::models::BattleCard>(
            r#"INSERT INTO battle_cards (
                    slug, figurine_id, race_id, status, tier, type_en, type_ru,
                    title_en, title_ru, effect_en, effect_ru, lore_en, lore_ru,
                    cost, power, health, mana, traits,
                    kind, armor, ward, attack_channel, reach, step, speed, mend,
                    abilities, budget_points, balance_index,
                    price_dust, price_feed, level_price_dust,
                    art_url, art_focal, frame_override, motion_wear, lendable,
                    edition_size, credit_name
               ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,
                         $19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34,$35,$36,$37,$38,$39)
               RETURNING *"#,
        )
        .bind(&w.slug)
        .bind(w.figurine_id)
        .bind(w.race_id)
        .bind(&w.status)
        .bind(w.tier)
        .bind(w.type_en.as_deref())
        .bind(w.type_ru.as_deref())
        .bind(&w.title_en)
        .bind(&w.title_ru)
        .bind(w.effect_en.as_deref())
        .bind(w.effect_ru.as_deref())
        .bind(w.lore_en.as_deref())
        .bind(w.lore_ru.as_deref())
        .bind(w.cost)
        .bind(w.power)
        .bind(w.health)
        .bind(w.mana)
        .bind(w.traits.as_deref())
        .bind(&w.kind)
        .bind(w.armor)
        .bind(w.ward)
        .bind(&w.attack_channel)
        .bind(w.reach)
        .bind(w.step)
        .bind(w.speed)
        .bind(w.mend)
        .bind(w.abilities.as_deref())
        .bind(w.budget_points)
        .bind(w.balance_index)
        .bind(w.price_dust)
        .bind(w.price_feed)
        .bind(w.level_price_dust.as_deref())
        .bind(w.art_url.as_deref())
        .bind(w.art_focal.as_deref())
        .bind(w.frame_override.as_deref())
        .bind(w.motion_wear.as_deref())
        .bind(w.lendable)
        .bind(w.edition_size)
        .bind(w.credit_name.as_deref())
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// Editing the numbers raises `rules_version`: a match records the version
    /// it was played under, so a rebalance never rewrites a match already
    /// played.
    pub async fn update_battle_card(
        &self,
        id: Uuid,
        w: &crate::models::BattleCardWrite,
    ) -> Result<crate::models::BattleCard> {
        sqlx::query_as::<_, crate::models::BattleCard>(
            r#"UPDATE battle_cards SET
                    slug = $2, figurine_id = $3, race_id = $4, status = $5, tier = $6,
                    type_en = $7, type_ru = $8,
                    title_en = $9, title_ru = $10, effect_en = $11, effect_ru = $12,
                    lore_en = $13, lore_ru = $14, cost = $15, power = $16,
                    health = $17, mana = $18, traits = $19,
                    kind = $20, armor = $21, ward = $22, attack_channel = $23,
                    reach = $24, step = $25, speed = $26, mend = $27,
                    abilities = $28, budget_points = $29, balance_index = $30,
                    price_dust = $31, price_feed = $32, level_price_dust = $33,
                    art_url = $34, art_focal = $35, frame_override = $36,
                    motion_wear = $37, lendable = $38, edition_size = $39,
                    rules_version = rules_version + 1,
                    updated_at = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .bind(&w.slug)
        .bind(w.figurine_id)
        .bind(w.race_id)
        .bind(&w.status)
        .bind(w.tier)
        .bind(w.type_en.as_deref())
        .bind(w.type_ru.as_deref())
        .bind(&w.title_en)
        .bind(&w.title_ru)
        .bind(w.effect_en.as_deref())
        .bind(w.effect_ru.as_deref())
        .bind(w.lore_en.as_deref())
        .bind(w.lore_ru.as_deref())
        .bind(w.cost)
        .bind(w.power)
        .bind(w.health)
        .bind(w.mana)
        .bind(w.traits.as_deref())
        .bind(&w.kind)
        .bind(w.armor)
        .bind(w.ward)
        .bind(&w.attack_channel)
        .bind(w.reach)
        .bind(w.step)
        .bind(w.speed)
        .bind(w.mend)
        .bind(w.abilities.as_deref())
        .bind(w.budget_points)
        .bind(w.balance_index)
        .bind(w.price_dust)
        .bind(w.price_feed)
        .bind(w.level_price_dust.as_deref())
        .bind(w.art_url.as_deref())
        .bind(w.art_focal.as_deref())
        .bind(w.frame_override.as_deref())
        .bind(w.motion_wear.as_deref())
        .bind(w.lendable)
        .bind(w.edition_size)
        .bind(w.credit_name.as_deref())
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle card {id} not found")))
    }

    pub async fn delete_battle_card(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM battle_cards WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Battle card {id} not found")));
        }
        Ok(())
    }

    /// Lay the shelf out in one statement. `updated_at` is deliberately left
    /// alone: rearranging the shelf is not a rewrite of the card, and the desk
    /// compares that stamp against its own open draft.
    /// Подписать карту автором. Отдельным запросом, а не полем в записи карты:
    /// автограф ставит студия, а не стол хозяина, и `SaveBattleCardRequest` о
    /// нём знать не должен — иначе подпись однажды приедет из формы.
    pub async fn set_battle_card_credit(&self, id: Uuid, name: &str) -> Result<()> {
        sqlx::query("UPDATE battle_cards SET credit_name = $2 WHERE id = $1")
            .bind(id)
            .bind(name)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn set_battle_card_order(&self, ids: &[Uuid]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let orders: Vec<i32> = (0..ids.len() as i32).collect();
        let res = sqlx::query(
            "UPDATE battle_cards AS c
                SET shelf_order = v.ord
               FROM (SELECT * FROM UNNEST($1::uuid[], $2::int[]) AS t(id, ord)) AS v
              WHERE c.id = v.id",
        )
        .bind(ids)
        .bind(&orders)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    /// The race dictionary, with how many cards stand under each. The count is
    /// what tells the keeper whether a rename is a small act or a large one.
    pub async fn list_battle_races(&self) -> Result<Vec<crate::models::BattleRaceListed>> {
        Ok(sqlx::query_as::<_, crate::models::BattleRaceListed>(
            "SELECT r.id, r.slug, r.name_en, r.name_ru, r.note_en, r.note_ru, r.icon_url,
                    r.level_frames, r.motion_wear, r.sort_order,
                    (SELECT COUNT(*) FROM battle_cards c WHERE c.race_id = r.id)::bigint
                        AS card_count
               FROM battle_races r
              ORDER BY r.sort_order NULLS LAST, r.name_ru, r.id",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn list_battle_race_slugs_except(&self, except: Option<Uuid>) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = if let Some(id) = except {
            sqlx::query_as("SELECT slug FROM battle_races WHERE id <> $1")
                .bind(id)
                .fetch_all(&self.pg_pool)
                .await?
        } else {
            sqlx::query_as("SELECT slug FROM battle_races")
                .fetch_all(&self.pg_pool)
                .await?
        };
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn insert_battle_race(
        &self,
        slug: &str,
        name_en: &str,
        name_ru: &str,
        note_en: Option<&str>,
        note_ru: Option<&str>,
        icon_url: Option<&str>,
        level_frames: Option<&str>,
        motion_wear: Option<&str>,
    ) -> Result<crate::models::BattleRace> {
        Ok(sqlx::query_as::<_, crate::models::BattleRace>(
            "INSERT INTO battle_races (slug, name_en, name_ru, note_en, note_ru, icon_url,
                                       level_frames, motion_wear)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *",
        )
        .bind(slug)
        .bind(name_en)
        .bind(name_ru)
        .bind(note_en)
        .bind(note_ru)
        .bind(icon_url)
        .bind(level_frames)
        .bind(motion_wear)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn update_battle_race(
        &self,
        id: Uuid,
        slug: &str,
        name_en: &str,
        name_ru: &str,
        note_en: Option<&str>,
        note_ru: Option<&str>,
        icon_url: Option<&str>,
        level_frames: Option<&str>,
        motion_wear: Option<&str>,
    ) -> Result<crate::models::BattleRace> {
        sqlx::query_as::<_, crate::models::BattleRace>(
            "UPDATE battle_races SET slug = $2, name_en = $3, name_ru = $4,
                    note_en = $5, note_ru = $6, icon_url = $7, level_frames = $8,
                    motion_wear = $9, updated_at = NOW()
              WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(slug)
        .bind(name_en)
        .bind(name_ru)
        .bind(note_en)
        .bind(note_ru)
        .bind(icon_url)
        .bind(level_frames)
        .bind(motion_wear)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle race {id} not found")))
    }

    /// Removing a race leaves its cards standing, without one — the foreign key
    /// is ON DELETE SET NULL. A dictionary entry is not the cards under it.
    pub async fn delete_battle_race(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM battle_races WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Battle race {id} not found")));
        }
        Ok(())
    }

    // === WALLET ===
    //
    // Append-only. The balance is never stored, it is summed — a stored balance
    // is an invitation to a double spend, and the ledger is also the keeper's
    // record of where every grain of dust came from.

    /// Write one credit. Returns `false` when this exact grant was already made:
    /// `idem_key` is unique per visitor, so a double-clicked victory, a retried
    /// request and a replayed challenge all land on the same row.
    pub async fn credit_battle_wallet(
        &self,
        user_id: Uuid,
        currency: &str,
        amount: i32,
        reason: &str,
        ref_id: Option<Uuid>,
        idem_key: &str,
    ) -> Result<bool> {
        let res = sqlx::query(
            "INSERT INTO battle_wallet_entries (user_id, currency, amount, reason, ref_id, idem_key)
             VALUES ($1,$2,$3,$4,$5,$6)
             ON CONFLICT (user_id, idem_key) DO NOTHING",
        )
        .bind(user_id)
        .bind(currency)
        .bind(amount)
        .bind(reason)
        .bind(ref_id)
        .bind(idem_key)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Дано из рук.
    ///
    /// Отдельно от `credit_battle_wallet`, а не параметром к нему, потому что
    /// это другой род движения: у него есть записка, оно бывает со знаком минус
    /// (ошибка в книге исправляется обратной строкой, а не правкой строки), и
    /// ключ ему даёт не содержимое, а самый акт — иначе два состоявшихся показа
    /// одному гостю слились бы в один.
    pub async fn grant_battle_wallet(
        &self,
        user_id: Uuid,
        currency: &str,
        amount: i32,
        note: Option<&str>,
        idem_key: &str,
    ) -> Result<bool> {
        let res = sqlx::query(
            "INSERT INTO battle_wallet_entries
                 (user_id, currency, amount, reason, note, idem_key)
             VALUES ($1,$2,$3,'hand',$4,$5)
             ON CONFLICT (user_id, idem_key) DO NOTHING",
        )
        .bind(user_id)
        .bind(currency)
        .bind(amount)
        .bind(note)
        .bind(idem_key)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    // === ПОРУЧЕНИЯ ===

    /// Лист, как его видит гость: только опубликованные, в порядке хранителя.
    pub async fn list_published_battle_errands(
        &self,
    ) -> Result<Vec<crate::models::BattleErrand>> {
        Ok(sqlx::query_as::<_, crate::models::BattleErrand>(
            "SELECT * FROM battle_errands
              WHERE status = 'published'
              ORDER BY sort_order NULLS LAST, created_at",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Отметить приход.
    ///
    /// День — по часам дома, поэтому он приходит СЮДА уже посчитанным: база не
    /// знает, где живёт дом, и `CURRENT_DATE` записал бы московский вечер
    /// вчерашним днём.
    ///
    /// Ключ (гость, день) и есть вся защита от двойной записи: заходить можно
    /// хоть двадцать раз, строка ляжет одна.
    pub async fn mark_battle_visit(&self, user_id: Uuid, day: chrono::NaiveDate) -> Result<()> {
        sqlx::query(
            "INSERT INTO battle_visits (user_id, day) VALUES ($1, $2)
             ON CONFLICT (user_id, day) DO NOTHING",
        )
        .bind(user_id)
        .bind(day)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Весь справочник, включая черновики: стол хранителя видит и то, чего
    /// гость ещё не видит.
    pub async fn list_battle_errands_admin(&self) -> Result<Vec<crate::models::BattleErrand>> {
        Ok(sqlx::query_as::<_, crate::models::BattleErrand>(
            "SELECT * FROM battle_errands ORDER BY sort_order NULLS LAST, created_at",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Сколько каждое поручение уже выдало: гостей и монет.
    ///
    /// Считается по КЛЮЧУ, а не по `ref_id`: удалённое поручение уносит с собой
    /// строку справочника, но не строки книги — а книга и есть то, что было
    /// заплачено на самом деле. Ключ повторяющегося поручения несёт ещё и окно
    /// (`errand:{slug}:{день}`), поэтому slug берётся вторым куском.
    pub async fn battle_errand_payouts(&self) -> Result<Vec<(String, i64, i64)>> {
        Ok(sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT split_part(idem_key, ':', 2) AS slug,
                    COUNT(DISTINCT user_id)::bigint,
                    COALESCE(SUM(amount), 0)::bigint
               FROM battle_wallet_entries
              WHERE reason = 'errand'
              GROUP BY 1",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn upsert_battle_errand(
        &self,
        id: Option<Uuid>,
        w: &crate::models::SaveBattleErrandRequest,
    ) -> Result<crate::models::BattleErrand> {
        let sql = if id.is_some() {
            "UPDATE battle_errands SET
                 slug = $2, title_en = $3, title_ru = $4, note_en = $5, note_ru = $6,
                 rule = $7, threshold = $8, currency = $9, amount = $10,
                 period = $11, starts_at = $12, ends_at = $13, status = $14,
                 sort_order = $15, by_hand = $16, updated_at = NOW()
              WHERE id = $1
              RETURNING *"
        } else {
            "INSERT INTO battle_errands
                 (id, slug, title_en, title_ru, note_en, note_ru, rule, threshold,
                  currency, amount, period, starts_at, ends_at, status, sort_order, by_hand)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
             RETURNING *"
        };
        let row = sqlx::query_as::<_, crate::models::BattleErrand>(sql)
            .bind(id.unwrap_or_else(Uuid::new_v4))
            .bind(&w.slug)
            .bind(&w.title_en)
            .bind(&w.title_ru)
            .bind(w.note_en.as_deref())
            .bind(w.note_ru.as_deref())
            .bind(&w.rule)
            .bind(w.threshold)
            .bind(&w.currency)
            .bind(w.amount)
            .bind(&w.period)
            .bind(w.starts_at)
            .bind(w.ends_at)
            .bind(&w.status)
            .bind(w.sort_order)
            .bind(w.by_hand)
            .fetch_optional(&self.pg_pool)
            .await?;
        row.ok_or_else(|| AppError::NotFound("Battle errand not found".into()))
    }

    /// Убрать поручение из справочника.
    ///
    /// Книгу это не трогает: заплаченное остаётся заплаченным, и ключ никуда не
    /// девается. Заведённое заново под тем же slug'ом второй раз не заплатит —
    /// и это правильно, а не досадно.
    pub async fn delete_battle_errand(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM battle_errands WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Battle errand {id} not found")));
        }
        Ok(())
    }

    pub async fn reorder_battle_errands(&self, ids: &[Uuid]) -> Result<()> {
        let mut tx = self.pg_pool.begin().await?;
        for (i, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE battle_errands SET sort_order = $2, updated_at = NOW() WHERE id = $1")
                .bind(id)
                .bind(i as i32)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Что дом насчитал по гостю за окно.
    ///
    /// `since = None` — за всё время (разовое поручение). Иначе считается
    /// только то, что случилось ВНУТРИ окна: «посмотреть три работы сегодня»,
    /// засчитанное вчерашними, платило бы каждый день ни за что, и это
    /// единственная ошибка во всей механике, которую нельзя отозвать.
    ///
    /// Условия о состоянии (`cards_owned`, `card_level`, `deck_laid`) окна не
    /// знают по существу: у снимка нет времени. Стол не даёт поставить их на
    /// повторяющееся поручение — см. `battles::ERRAND_RULES_STATEFUL`.
    pub async fn battle_errand_tally(
        &self,
        user_id: Uuid,
        since: Option<DateTime<Utc>>,
    ) -> Result<crate::models::BattleErrandTally> {
        // Одним заходом по книге: три вида внимания и потраченная пыль лежат в
        // одной таблице, и четыре запроса за ними были бы четырьмя проходами.
        let wallet: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT
                 COALESCE(SUM(CASE WHEN reason = 'seen'  THEN 1 ELSE 0 END), 0)::bigint,
                 COALESCE(SUM(CASE WHEN reason = 'liked' THEN 1 ELSE 0 END), 0)::bigint,
                 COALESCE(SUM(CASE WHEN reason = 'read'  THEN 1 ELSE 0 END), 0)::bigint,
                 COALESCE(-SUM(CASE WHEN currency = 'dust' AND amount < 0
                                    THEN amount ELSE 0 END), 0)::bigint
               FROM battle_wallet_entries
              WHERE user_id = $1 AND ($2::timestamptz IS NULL OR created_at >= $2)",
        )
        .bind(user_id)
        .bind(since)
        .fetch_one(&self.pg_pool)
        .await?;

        // Собрание — это состояние, и окно ему не применяется намеренно.
        let owned: (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*)::bigint, COALESCE(MAX(level), 0)::bigint
               FROM card_copies WHERE owner_id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pg_pool)
        .await?;

        let deck: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM battle_decks WHERE user_id = $1)")
                .bind(user_id)
                .fetch_one(&self.pg_pool)
                .await?;

        let matches: (i64, i64, i64) = sqlx::query_as(
            "SELECT
                 COUNT(*) FILTER (WHERE outcome IS NOT NULL)::bigint,
                 COUNT(*) FILTER (WHERE outcome = 'player')::bigint,
                 COUNT(DISTINCT challenge_id) FILTER (WHERE outcome = 'player')::bigint
               FROM battle_matches
              WHERE user_id = $1 AND ($2::timestamptz IS NULL OR created_at >= $2)",
        )
        .bind(user_id)
        .bind(since)
        .fetch_one(&self.pg_pool)
        .await?;

        // Отмеченные хранителем, а не написанные: иначе поручение «оставьте
        // впечатление» — кран, отвечающий пылью на любой набор букв.
        let comments: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM figurine_comments
              WHERE user_id = $1 AND is_approved
                AND ($2::timestamptz IS NULL OR created_at >= $2)",
        )
        .bind(user_id)
        .bind(since)
        .fetch_one(&self.pg_pool)
        .await?;

        let bookings: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM figurine_bookings
              WHERE user_id = $1 AND status::text = 'completed'
                AND ($2::timestamptz IS NULL OR created_at >= $2)",
        )
        .bind(user_id)
        .bind(since)
        .fetch_one(&self.pg_pool)
        .await?;

        let orders: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM orders
              WHERE user_id = $1 AND ($2::timestamptz IS NULL OR created_at >= $2)",
        )
        .bind(user_id)
        .bind(since)
        .fetch_one(&self.pg_pool)
        .await?;

        // Считается по `visited_at`, а не по `day`: тогда окно здесь — то же
        // сравнение времени, что у всех прочих условий, а не особый случай с
        // приведением дат к местному календарю во второй раз.
        let visits: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM battle_visits
              WHERE user_id = $1 AND ($2::timestamptz IS NULL OR visited_at >= $2)",
        )
        .bind(user_id)
        .bind(since)
        .fetch_one(&self.pg_pool)
        .await?;

        Ok(crate::models::BattleErrandTally {
            works_seen: wallet.0,
            works_liked: wallet.1,
            tales_read: wallet.2,
            dust_spent: wallet.3,
            cards_owned: owned.0,
            card_level: owned.1,
            deck_laid: i64::from(deck.0),
            matches_finished: matches.0,
            matches_won: matches.1,
            challenges_won: matches.2,
            comments_left: comments.0,
            bookings_done: bookings.0,
            orders_made: orders.0,
            visits: visits.0,
        })
    }

    /// Дар первого входа.
    ///
    /// Отдельно от `grant_battle_wallet` по той же причине, по какой тот отделён
    /// от `credit_battle_wallet`: это движение другого рода. У него нет записки —
    /// слова подбирает страница, иначе они легли бы в книгу на одном языке и
    /// остались бы на нём навсегда, — и повод у него один на всю жизнь книги.
    ///
    /// Ключ у каждой монеты свой. Хранитель, поднявший дар с нуля до единицы
    /// через месяц, доплатит эту монету и тем, кто уже вошёл, — а не заплатит
    /// весь дар второй раз.
    pub async fn welcome_battle_wallet(
        &self,
        user_id: Uuid,
        currency: &str,
        amount: i32,
    ) -> Result<bool> {
        let res = sqlx::query(
            "INSERT INTO battle_wallet_entries
                 (user_id, currency, amount, reason, idem_key)
             VALUES ($1, $2, $3, 'welcome', $4)
             ON CONFLICT (user_id, idem_key) DO NOTHING",
        )
        .bind(user_id)
        .bind(currency)
        .bind(amount)
        .bind(format!("welcome:{currency}"))
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Проявка: засчитать разом работы, которые гость смотрел до того, как
    /// комната для него открылась.
    ///
    /// Одним запросом, а не циклом: проверка существования, проверка открытости
    /// и защита от повтора — всё это уже умеет база, и шестьдесят обращений
    /// подряд за тем же самым были бы разве что медленнее.
    ///
    /// Возвращает, сколько строк ЛЕГЛО, а не сколько прислали: работа, за
    /// которую уже платили, и работа, которой нет, отсюда неотличимы — и
    /// человеку в обоих случаях нечего сказать.
    pub async fn credit_battle_seen_bulk(
        &self,
        user_id: Uuid,
        ids: &[Uuid],
        amount: i32,
    ) -> Result<i64> {
        if ids.is_empty() || amount <= 0 {
            return Ok(0);
        }
        let res = sqlx::query(
            "INSERT INTO battle_wallet_entries
                 (user_id, currency, amount, reason, ref_id, idem_key)
             SELECT $1, 'dust', $2, 'seen', f.id, 'seen:' || f.id
               FROM figurines f
              WHERE f.id = ANY($3) AND f.is_visible
             ON CONFLICT (user_id, idem_key) DO NOTHING",
        )
        .bind(user_id)
        .bind(amount)
        .bind(ids)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected() as i64)
    }

    /// Что дому дали не за внимание — последние записки, новыми вперёд.
    ///
    /// `hand` — из рук хранителя, `welcome` — дар первого входа. Пыль, осевшая
    /// с маяков, сюда не попадает: у неё нет ни записки, ни повода её назвать,
    /// и на полях она считается иначе (сколько осело с прошлого раза).
    pub async fn list_battle_hand_grants(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<crate::models::BattleGrantRow>> {
        Ok(sqlx::query_as::<_, crate::models::BattleGrantRow>(
            "SELECT currency, amount, reason, note, created_at FROM battle_wallet_entries
              WHERE user_id = $1 AND reason IN ('hand', 'welcome') AND amount > 0
              ORDER BY created_at DESC LIMIT $2",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Существует ли вещь, за внимание к которой просят пыль, и открыта ли она.
    ///
    /// Маячок верит странице на слово: она присылает вид и id. Без этой проверки
    /// придуманный UUID — это выдуманная пыль, а проявка (`BATTLE-ERRANDS.md`),
    /// шлющая целый список разом, делает дыру ровно в пятьдесят раз шире.
    ///
    /// Неизвестный вид — не ошибка, а «нет»: маячок молчит по построению, и
    /// ошибку здесь всё равно некому показать.
    pub async fn battle_attention_target_public(&self, kind: &str, id: Uuid) -> Result<bool> {
        let sql = match kind {
            "seen" | "liked" => "SELECT EXISTS(SELECT 1 FROM figurines WHERE id = $1 AND is_visible)",
            "read" => {
                "SELECT EXISTS(SELECT 1 FROM gazette_leaves WHERE id = $1 AND status = 'published')"
            }
            _ => return Ok(false),
        };
        let row: (bool,) = sqlx::query_as(sql).bind(id).fetch_one(&self.pg_pool).await?;
        Ok(row.0)
    }

    pub async fn battle_wallet_balance(&self, user_id: Uuid, currency: &str) -> Result<i64> {
        let row: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount)::bigint FROM battle_wallet_entries
              WHERE user_id = $1 AND currency = $2",
        )
        .bind(user_id)
        .bind(currency)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row.0.unwrap_or(0))
    }

    pub async fn battle_wallet_paid(&self, user_id: Uuid, idem_key: &str) -> Result<bool> {
        let row: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM battle_wallet_entries
                            WHERE user_id = $1 AND idem_key = $2)",
        )
        .bind(user_id)
        .bind(idem_key)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row.0)
    }

    /// Every key this visitor has already been paid under, for marking a shelf
    /// of challenges in one query rather than one query per challenge.
    pub async fn battle_wallet_keys(&self, user_id: Uuid) -> Result<Vec<String>> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT idem_key FROM battle_wallet_entries WHERE user_id = $1")
                .bind(user_id)
                .fetch_all(&self.pg_pool)
                .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    // === OWNING ===

    /// Every card this visitor holds. One query for the whole shelf: asking per
    /// card would be forty round trips to draw one page.
    pub async fn list_owned_battle_cards(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::BattleOwnedCard>> {
        Ok(sqlx::query_as::<_, crate::models::BattleOwnedCard>(
            // Запертое объявлением НЕ СЧИТАЕТСЯ владением: выставленной вещью
            // нельзя играть, иначе она стоит на доске в тот самый миг, когда
            // её у тебя покупают.
            "SELECT id, owner_id AS user_id, card_id, serial, level, acquired_at, seen_at
               FROM card_copies WHERE owner_id = $1 AND locked_by IS NULL
              ORDER BY acquired_at",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    // ── Стол гостя ────────────────────────────────────────────────────────

    /// `None` — гость не раскладывал стол ни разу. Это не то же самое, что
    /// пустой стол: первый заход на встречу без разложенного стола ведёт на
    /// стол, а не в партию.
    pub async fn get_battle_deck(
        &self,
        user_id: Uuid,
    ) -> Result<Option<crate::models::BattleDeck>> {
        Ok(sqlx::query_as::<_, crate::models::BattleDeck>(
            "SELECT * FROM battle_decks WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Стол переписывается целиком. Не по одному месту: половина сохранённой
    /// расстановки — это не расстановка, а состояние, в котором её застали.
    pub async fn upsert_battle_deck(&self, user_id: Uuid, layout: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO battle_decks (user_id, layout) VALUES ($1, $2)
             ON CONFLICT (user_id) DO UPDATE SET layout = EXCLUDED.layout, updated_at = NOW()",
        )
        .bind(user_id)
        .bind(layout)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Что дом готов одолжить, всегда в одном порядке.
    ///
    /// Чин отбирается здесь, а не отметкой: хранитель волен пометить карту
    /// заёмной заранее, до того как решит её чин. Здоровье — то же условие,
    /// что и у стенда: тело без здоровья падает в тот же миг, как встанет.
    pub async fn list_lendable_battle_cards(
        &self,
        tier: i16,
        limit: i64,
    ) -> Result<Vec<crate::models::BattleCardListed>> {
        Ok(
            sqlx::query_as::<_, crate::models::BattleCardListed>(&format!(
                "{} WHERE c.lendable AND c.status = 'published' AND c.tier = $1 AND c.health > 0
             ORDER BY c.shelf_order NULLS LAST, c.created_at DESC, c.id DESC
             LIMIT $2",
                Self::BATTLE_CARD_SELECT
            ))
            .bind(tier)
            .bind(limit)
            .fetch_all(&self.pg_pool)
            .await?,
        )
    }

    /// Выдать карты в собрание напрямую, минуя покупку.
    ///
    /// Кошелька не касается: это подарок, а не покупка, и списывать за него
    /// нечего. Уровень переписывается намеренно — инструмент нужен, чтобы
    /// быстро привести собрание в нужное для проверки состояние, а не чтобы
    /// бережно дополнять его.
    pub async fn give_battle_cards(
        &self,
        user_id: Uuid,
        card_ids: &[Uuid],
        level: i16,
    ) -> Result<u64> {
        if card_ids.is_empty() {
            return Ok(0);
        }
        // Экземпляр, а не отметка. У кого карта уже есть, тому переписывается
        // уровень — второй экземпляр из рук хранителя не появляется, иначе
        // «выдать всё», нажатое дважды, удвоило бы собрание.
        //
        // Двумя запросами, а не одним ловким: тронутыми считаются И
        // переписанные, и заведённые, и хранителю сказано одно число, которое
        // не врёт.
        let mut tx = self.pg_pool.begin().await?;
        let raised = sqlx::query(
            "UPDATE card_copies SET level = $3
              WHERE owner_id = $1 AND card_id = ANY($2)",
        )
        .bind(user_id)
        .bind(card_ids)
        .bind(level)
        .execute(&mut *tx)
        .await?;

        // Номер новому экземпляру даёт счётчик отпечатанного — тот же, что и
        // при покупке: подарок печатается из того же тиража, что и покупка,
        // иначе тираж перестаёт быть числом.
        let given = sqlx::query(
            "WITH fresh AS (
                 SELECT t.id FROM UNNEST($2::uuid[]) AS t(id)
                  WHERE NOT EXISTS (
                      SELECT 1 FROM card_copies o
                       WHERE o.owner_id = $1 AND o.card_id = t.id
                  )
             ),
             minted AS (
                 UPDATE battle_cards c SET minted = c.minted + 1
                  WHERE c.id IN (SELECT id FROM fresh)
              RETURNING c.id, c.minted
             )
             INSERT INTO card_copies (card_id, owner_id, serial, level, origin)
             SELECT id, $1, minted, $3, 'gift' FROM minted",
        )
        .bind(user_id)
        .bind(card_ids)
        .bind(level)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(raised.rows_affected() + given.rows_affected())
    }

    /// Забрать карты обратно. Пустой список — забрать все: проверка пустого
    /// собрания (и заёма, который его закрывает) иначе недостижима.
    pub async fn revoke_battle_cards(&self, user_id: Uuid, card_ids: &[Uuid]) -> Result<u64> {
        let res = if card_ids.is_empty() {
            sqlx::query("DELETE FROM card_copies WHERE owner_id = $1")
                .bind(user_id)
                .execute(&self.pg_pool)
                .await?
        } else {
            sqlx::query("DELETE FROM card_copies WHERE owner_id = $1 AND card_id = ANY($2)")
                .bind(user_id)
                .bind(card_ids)
                .execute(&self.pg_pool)
                .await?
        };
        Ok(res.rows_affected())
    }

    /// Take the mark off a card that has now been looked at. Idempotent by
    /// being a no-op once `seen_at` is set: the first look is the one recorded.
    pub async fn mark_battle_card_seen(&self, user_id: Uuid, card_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE card_copies SET seen_at = NOW()
              WHERE owner_id = $1 AND card_id = $2 AND seen_at IS NULL",
        )
        .bind(user_id)
        .bind(card_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Take a card off the shelf: one row out of the wallet, one row of owning,
    /// both or neither.
    ///
    /// The whole of it is one transaction, and the order inside it matters. The
    /// balance is summed **inside** that transaction and the ledger row is
    /// written before the check can go stale — two tabs pressing "take" at once
    /// both read the same balance otherwise, and both are allowed to spend it.
    ///
    /// `idem_key` is `buy:{card_id}`, so the second press writes nothing and is
    /// answered with the same board: a card cannot be bought twice, and a
    /// double click is not a double spend.
    ///
    /// Returns `false` when nothing was written because it was already theirs.
    pub async fn buy_battle_card(
        &self,
        user_id: Uuid,
        card_id: Uuid,
        currency: &str,
        price: i32,
    ) -> Result<bool> {
        let mut tx = self.pg_pool.begin().await?;

        // Serialise the buyers of this one wallet against each other. Two
        // presses in two tabs queue here instead of both reading the same
        // balance and both being told they can afford it.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await?;

        let owned: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM card_copies WHERE owner_id = $1 AND card_id = $2)",
        )
        .bind(user_id)
        .bind(card_id)
        .fetch_one(&mut *tx)
        .await?;
        if owned.0 {
            tx.rollback().await?;
            return Ok(false);
        }

        let balance: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount)::bigint FROM battle_wallet_entries
              WHERE user_id = $1 AND currency = $2",
        )
        .bind(user_id)
        .bind(currency)
        .fetch_one(&mut *tx)
        .await?;
        if balance.0.unwrap_or(0) < i64::from(price) {
            tx.rollback().await?;
            return Err(AppError::BadRequest("Not enough in the wallet".into()));
        }

        // Номер экземпляра берётся ДО списания: у карты с тиражом он может
        // кончиться, и человек, у которого взяли пыль за ненапечатанное, —
        // это худшее, что умеет сделать лавка. Счётчик поднимается тем же
        // запросом, которым читается: два тайма между чтением и записью —
        // это два экземпляра с номером семь.
        let minted: Option<(i32,)> = sqlx::query_as(
            "UPDATE battle_cards SET minted = minted + 1
              WHERE id = $1 AND (edition_size IS NULL OR minted < edition_size)
          RETURNING minted",
        )
        .bind(card_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((serial,)) = minted else {
            tx.rollback().await?;
            return Err(AppError::BadRequest("The edition is out".into()));
        };

        let spent = sqlx::query(
            "INSERT INTO battle_wallet_entries
                 (user_id, currency, amount, reason, ref_id, idem_key)
             VALUES ($1, $2, $3, 'bought', $4, $5)
             ON CONFLICT (user_id, idem_key) DO NOTHING",
        )
        .bind(user_id)
        .bind(currency)
        .bind(-price)
        .bind(card_id)
        .bind(format!("buy:{card_id}"))
        .execute(&mut *tx)
        .await?;
        if spent.rows_affected() == 0 {
            // Paid for once already, and the owning row is missing — the only
            // way here is a crash between the two writes. Owning is what the
            // payment bought, so it is written rather than the money returned.
            tracing::warn!(%user_id, %card_id, "покупка была оплачена, но карта не записана — дописываем");
        }

        sqlx::query(
            "INSERT INTO card_copies (card_id, owner_id, serial, level, origin)
             VALUES ($1, $2, $3, 1, 'bought')",
        )
        .bind(card_id)
        .bind(user_id)
        .bind(serial)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }

    /// Raise one owned copy one rung.
    ///
    /// The same shape as a purchase, and for the same reasons: one transaction,
    /// the wallet locked against its own other tabs, the balance summed inside.
    /// Two differences.
    ///
    /// The ledger key names the rung (`level:{card}:{to}`) and not just the
    /// card, because a card is bought once but climbed four times — a key
    /// without the rung would let the second rung ride the first one's row.
    ///
    /// The level is raised by compare-and-set (`WHERE level = from`) rather
    /// than by `level + 1`: two presses that both get past the key would
    /// otherwise each add one, and the owner would arrive at three having paid
    /// for two.
    ///
    /// Returns the level after, and whether anything was written.
    pub async fn raise_battle_card_level(
        &self,
        user_id: Uuid,
        card_id: Uuid,
        from_level: i16,
        price: i32,
    ) -> Result<(i16, bool)> {
        let to_level = from_level + 1;
        let mut tx = self.pg_pool.begin().await?;

        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(user_id.to_string())
            .execute(&mut *tx)
            .await?;

        // Поднимается САМЫЙ ОТСТАЮЩИЙ экземпляр этой карты. Пока экземпляр
        // один, выбирать не из чего; когда экземпляры пойдут по рукам, поднять
        // прокачанную, пока рядом лежит непрокачанная, никто не попросит.
        let held: Option<(Uuid, i16)> = sqlx::query_as(
            "SELECT id, level FROM card_copies
              WHERE owner_id = $1 AND card_id = $2
              ORDER BY level, acquired_at LIMIT 1 FOR UPDATE",
        )
        .bind(user_id)
        .bind(card_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((copy_id, held)) = held else {
            tx.rollback().await?;
            return Err(AppError::BadRequest("This card is not yours".into()));
        };
        // Уже поднято — значит, это повтор, а не вторая ступень.
        if held >= to_level {
            tx.rollback().await?;
            return Ok((held, false));
        }
        if held != from_level {
            tx.rollback().await?;
            return Err(AppError::BadRequest("The level has changed".into()));
        }

        let balance: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount)::bigint FROM battle_wallet_entries
              WHERE user_id = $1 AND currency = 'dust'",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        if balance.0.unwrap_or(0) < i64::from(price) {
            tx.rollback().await?;
            return Err(AppError::BadRequest("Not enough dust".into()));
        }

        let spent = sqlx::query(
            "INSERT INTO battle_wallet_entries
                 (user_id, currency, amount, reason, ref_id, idem_key)
             VALUES ($1, 'dust', $2, 'level_up', $3, $4)
             ON CONFLICT (user_id, idem_key) DO NOTHING",
        )
        .bind(user_id)
        .bind(-price)
        .bind(card_id)
        .bind(format!("level:{card_id}:{to_level}"))
        .execute(&mut *tx)
        .await?;
        if spent.rows_affected() == 0 {
            // Ступень оплачена, а уровень ниже её: между двумя записями что-то
            // оборвалось. Ступень уже куплена — значит, её и дописываем.
            tracing::warn!(%user_id, %card_id, to_level, "ступень оплачена, но не поднята — дописываем");
        }

        let raised = sqlx::query(
            "UPDATE card_copies SET level = $2 WHERE id = $1 AND level = $3",
        )
        .bind(copy_id)
        .bind(to_level)
        .bind(from_level)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((to_level, raised.rows_affected() > 0))
    }

    // === CHALLENGES AND MATCHES ===

    pub async fn list_battle_challenges(
        &self,
        published_only: bool,
    ) -> Result<Vec<crate::models::BattleChallenge>> {
        let sql = if published_only {
            "SELECT * FROM battle_challenges WHERE status = 'published'
              ORDER BY sort_order NULLS LAST, created_at DESC"
        } else {
            "SELECT * FROM battle_challenges ORDER BY sort_order NULLS LAST, created_at DESC"
        };
        Ok(sqlx::query_as::<_, crate::models::BattleChallenge>(sql)
            .fetch_all(&self.pg_pool)
            .await?)
    }

    pub async fn get_battle_challenge(&self, id: Uuid) -> Result<crate::models::BattleChallenge> {
        sqlx::query_as::<_, crate::models::BattleChallenge>(
            "SELECT * FROM battle_challenges WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle challenge {id} not found")))
    }

    pub async fn list_battle_challenge_slugs_except(
        &self,
        except: Option<Uuid>,
    ) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = if let Some(id) = except {
            sqlx::query_as("SELECT slug FROM battle_challenges WHERE id <> $1")
                .bind(id)
                .fetch_all(&self.pg_pool)
                .await?
        } else {
            sqlx::query_as("SELECT slug FROM battle_challenges")
                .fetch_all(&self.pg_pool)
                .await?
        };
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    #[allow(clippy::too_many_arguments)]
    /// Переписать расстановку одного испытания.
    ///
    /// Узкий запрос, а не полный `upsert`: переезд слуга не должен уметь
    /// тронуть ни заголовок, ни награду, ни статус — иначе однажды тронет.
    /// Сыгранные партии, свежими вперёд, с именем гостя и названием испытания.
    ///
    /// `board_cache` намеренно не читается: он кэш, а для разбора нужны журнал
    /// и расстановка. Тянуть за собой доску каждой партии — это мегабайты за
    /// то, чего никто не покажет.
    pub async fn admin_list_battle_matches(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::models::BattleMatchRow>> {
        Ok(sqlx::query_as::<_, crate::models::BattleMatchRow>(
            "SELECT m.id, u.display_name AS guest, m.challenge_id,
                    c.title_ru, c.title_en, m.outcome, m.rounds,
                    m.setup, m.actions, m.created_at, m.finished_at
               FROM battle_matches m
               JOIN users u ON u.id = m.user_id
               LEFT JOIN battle_challenges c ON c.id = m.challenge_id
              ORDER BY m.created_at DESC
              LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn update_battle_challenge_setup(&self, id: Uuid, setup: &str) -> Result<()> {
        sqlx::query("UPDATE battle_challenges SET setup = $2, updated_at = now() WHERE id = $1")
            .bind(id)
            .bind(setup)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn upsert_battle_challenge(
        &self,
        id: Option<Uuid>,
        w: &crate::models::BattleChallengeWrite<'_>,
    ) -> Result<crate::models::BattleChallenge> {
        match id {
            None => Ok(sqlx::query_as::<_, crate::models::BattleChallenge>(
                "INSERT INTO battle_challenges
                     (slug, title_en, title_ru, note_en, note_ru, setup, bot_depth,
                      reward_dust, player_side, status, reward_finish_dust)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING *",
            )
            .bind(w.slug)
            .bind(w.title_en)
            .bind(w.title_ru)
            .bind(w.note_en)
            .bind(w.note_ru)
            .bind(w.setup)
            .bind(w.bot_depth)
            .bind(w.reward_dust)
            .bind(w.player_side)
            .bind(w.status)
            .bind(w.reward_finish_dust)
            .fetch_one(&self.pg_pool)
            .await?),
            Some(id) => sqlx::query_as::<_, crate::models::BattleChallenge>(
                "UPDATE battle_challenges SET slug=$2, title_en=$3, title_ru=$4,
                        note_en=$5, note_ru=$6, setup=$7, bot_depth=$8,
                        reward_dust=$9, player_side=$10, status=$11,
                        reward_finish_dust=$12, updated_at = NOW()
                  WHERE id=$1 RETURNING *",
            )
            .bind(id)
            .bind(w.slug)
            .bind(w.title_en)
            .bind(w.title_ru)
            .bind(w.note_en)
            .bind(w.note_ru)
            .bind(w.setup)
            .bind(w.bot_depth)
            .bind(w.reward_dust)
            .bind(w.player_side)
            .bind(w.status)
            .bind(w.reward_finish_dust)
            .fetch_optional(&self.pg_pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Battle challenge {id} not found"))),
        }
    }

    /// Разложить полку этюдов одним запросом. `updated_at` намеренно не
    /// трогается: переставить испытание — не переписать его.
    pub async fn set_battle_challenge_order(&self, ids: &[Uuid]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let orders: Vec<i32> = (0..ids.len() as i32).collect();
        let res = sqlx::query(
            "UPDATE battle_challenges AS c
                SET sort_order = v.ord
               FROM (SELECT * FROM UNNEST($1::uuid[], $2::int[]) AS t(id, ord)) AS v
              WHERE c.id = v.id",
        )
        .bind(ids)
        .bind(&orders)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    pub async fn delete_battle_challenge(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM battle_challenges WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!(
                "Battle challenge {id} not found"
            )));
        }
        Ok(())
    }

    /// The match this visitor already has going on this challenge, if any.
    ///
    /// A returning guest continues the one they left rather than starting a
    /// second with the same click — that is what the partial unique index on
    /// `outcome IS NULL` is for.
    pub async fn find_open_battle_match(
        &self,
        user_id: Uuid,
        challenge_id: Uuid,
    ) -> Result<Option<crate::models::BattleMatch>> {
        Ok(sqlx::query_as::<_, crate::models::BattleMatch>(
            "SELECT * FROM battle_matches
              WHERE user_id = $1 AND challenge_id = $2 AND outcome IS NULL",
        )
        .bind(user_id)
        .bind(challenge_id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Open matches for this visitor, keyed by the challenge they belong to.
    /// The shelf of studies uses it to print «continue» on the ones left going.
    pub async fn list_open_battle_match_ids(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid)>> {
        let rows: Vec<(Option<Uuid>, Uuid)> = sqlx::query_as(
            "SELECT challenge_id, id FROM battle_matches
              WHERE user_id = $1 AND outcome IS NULL AND challenge_id IS NOT NULL",
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|(challenge_id, match_id)| challenge_id.map(|cid| (cid, match_id)))
            .collect())
    }

    /// Give the field: the row finishes as a keeper win, without a move.
    /// The board cache is stamped so a later read does not resurrect the live board.
    pub async fn yield_battle_match(
        &self,
        id: Uuid,
        user_id: Uuid,
        board_cache: &str,
    ) -> Result<Option<crate::models::BattleMatch>> {
        Ok(sqlx::query_as::<_, crate::models::BattleMatch>(
            "UPDATE battle_matches
                SET outcome = 'keeper', board_cache = $3, finished_at = NOW()
              WHERE id = $1 AND user_id = $2 AND outcome IS NULL
              RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(board_cache)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Drop the unfinished match so a replay can insert a new one.
    /// Not a loss: they asked to start this over, they did not give the field.
    pub async fn drop_open_battle_match(
        &self,
        user_id: Uuid,
        challenge_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM battle_matches
              WHERE user_id = $1 AND challenge_id = $2 AND outcome IS NULL",
        )
        .bind(user_id)
        .bind(challenge_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Партия без проверки владельца. Только для стола хранителя: гостю партия
    /// выдаётся `get_battle_match`, и та проверяет, что она его.
    pub async fn admin_get_battle_match(
        &self,
        id: Uuid,
    ) -> Result<Option<crate::models::BattleMatch>> {
        Ok(sqlx::query_as::<_, crate::models::BattleMatch>(
            "SELECT * FROM battle_matches WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn get_battle_match(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<crate::models::BattleMatch> {
        sqlx::query_as::<_, crate::models::BattleMatch>(
            "SELECT * FROM battle_matches WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle match {id} not found")))
    }

    pub async fn insert_battle_match(
        &self,
        user_id: Uuid,
        challenge_id: Uuid,
        setup: &str,
        rules_version: i32,
        board_cache: &str,
        rules: &str,
    ) -> Result<crate::models::BattleMatch> {
        Ok(sqlx::query_as::<_, crate::models::BattleMatch>(
            "INSERT INTO battle_matches
                 (user_id, challenge_id, setup, rules_version, board_cache, rules)
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING *",
        )
        .bind(user_id)
        .bind(challenge_id)
        .bind(setup)
        .bind(rules_version)
        .bind(board_cache)
        .bind(rules)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// Append to the journal, under the number the caller expected.
    ///
    /// The `seq = $2` in the WHERE clause is the whole guard: a repeated or
    /// reordered request finds no row and changes nothing, instead of playing
    /// the same move twice.
    #[allow(clippy::too_many_arguments)]
    pub async fn advance_battle_match(
        &self,
        id: Uuid,
        expected_seq: i32,
        actions: &str,
        board_cache: &str,
        outcome: Option<&str>,
        rounds: i16,
        // Ставятся только вместе с исходом и только один раз: печать под
        // сыгранной партией, а не счётчик, который растёт по ходу.
        player_acts: Option<i16>,
        bodies_lost: Option<i16>,
    ) -> Result<Option<crate::models::BattleMatch>> {
        Ok(sqlx::query_as::<_, crate::models::BattleMatch>(
            "UPDATE battle_matches
                SET actions = $3, board_cache = $4, seq = seq + 1,
                    outcome = $5, rounds = $6,
                    player_acts = $7, bodies_lost = $8,
                    finished_at = CASE WHEN $5 IS NULL THEN NULL ELSE NOW() END
              WHERE id = $1 AND seq = $2
              RETURNING *",
        )
        .bind(id)
        .bind(expected_seq)
        .bind(actions)
        .bind(board_cache)
        .bind(outcome)
        .bind(rounds)
        .bind(player_acts)
        .bind(bodies_lost)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Чем помечены этюды у одного гостя: доведено, выиграно, выиграно без
    /// потерь, и за сколько дел прошло лучшее из выигранных.
    ///
    /// Один запрос на всю полку, а не по одному на этюд, — тем же правилом, что
    /// и кошельковые ключи рядом.
    ///
    /// «Доведено» — это `player_acts IS NOT NULL`, а не «есть исход». Отданное
    /// поле исход тоже ставит, но печати не заслуживает: за него и пыль не
    /// платят.
    ///
    /// Победа считается доведённой и без этого числа: выиграть, не доиграв,
    /// нельзя, а партии старше самой колонки числа не имеют — и без этой
    /// оговорки читались бы как «выиграно, но не доведено до конца».
    pub async fn battle_challenge_marks(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(Uuid, bool, bool, bool, Option<i16>)>> {
        Ok(
            sqlx::query_as::<_, (Uuid, bool, bool, bool, Option<i16>)>(
                "SELECT challenge_id,
                        COALESCE(BOOL_OR(player_acts IS NOT NULL OR outcome = 'player'), false),
                        COALESCE(BOOL_OR(outcome = 'player'), false),
                        COALESCE(BOOL_OR(outcome = 'player' AND bodies_lost = 0), false),
                        MIN(player_acts) FILTER (WHERE outcome = 'player')
                   FROM battle_matches
                  WHERE user_id = $1 AND challenge_id IS NOT NULL AND outcome IS NOT NULL
                  GROUP BY challenge_id",
            )
            .bind(user_id)
            .fetch_all(&self.pg_pool)
            .await?,
        )
    }

    /// Планка каждого этюда: лучшая линия, которую дом видел от кого бы то ни
    /// было. Поднимается живой рукой, а не перебором, — поэтому не обещает
    /// того, чего дом не знает.
    pub async fn battle_challenge_best_lines(&self) -> Result<Vec<(Uuid, i16)>> {
        Ok(sqlx::query_as::<_, (Uuid, i16)>(
            "SELECT challenge_id, MIN(player_acts)
               FROM battle_matches
              WHERE challenge_id IS NOT NULL
                AND outcome = 'player' AND player_acts IS NOT NULL
              GROUP BY challenge_id",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// То же для одного этюда. Спрашивается в тот единственный миг, когда
    /// партия кончилась победой, — чтобы сказать «за пять, а лучшее было шесть»
    /// прежде, чем эта же партия станет новым лучшим.
    pub async fn battle_challenge_best_line(&self, challenge_id: Uuid) -> Result<Option<i16>> {
        Ok(sqlx::query_scalar::<_, Option<i16>>(
            "SELECT MIN(player_acts)
               FROM battle_matches
              WHERE challenge_id = $1
                AND outcome = 'player' AND player_acts IS NOT NULL",
        )
        .bind(challenge_id)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    // === KEYWORDS ===
    //
    // No card count beside a keyword, unlike a race: an ability names a keyword
    // by slug inside its own JSON, so counting would mean reading every card.
    // When that number is wanted, it will be worth its own index.

    pub async fn list_battle_keywords(&self) -> Result<Vec<crate::models::BattleKeyword>> {
        Ok(sqlx::query_as::<_, crate::models::BattleKeyword>(
            "SELECT * FROM battle_keywords ORDER BY sort_order NULLS LAST, name_ru, id",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn list_battle_keyword_slugs_except(
        &self,
        except: Option<Uuid>,
    ) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = if let Some(id) = except {
            sqlx::query_as("SELECT slug FROM battle_keywords WHERE id <> $1")
                .bind(id)
                .fetch_all(&self.pg_pool)
                .await?
        } else {
            sqlx::query_as("SELECT slug FROM battle_keywords")
                .fetch_all(&self.pg_pool)
                .await?
        };
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn insert_battle_keyword(
        &self,
        slug: &str,
        name_en: &str,
        name_ru: &str,
        rules_en: Option<&str>,
        rules_ru: Option<&str>,
        icon_url: Option<&str>,
        point_value: Option<f64>,
    ) -> Result<crate::models::BattleKeyword> {
        Ok(sqlx::query_as::<_, crate::models::BattleKeyword>(
            "INSERT INTO battle_keywords (slug, name_en, name_ru, rules_en, rules_ru, icon_url, point_value)
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING *",
        )
        .bind(slug)
        .bind(name_en)
        .bind(name_ru)
        .bind(rules_en)
        .bind(rules_ru)
        .bind(icon_url)
        .bind(point_value)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn update_battle_keyword(
        &self,
        id: Uuid,
        slug: &str,
        name_en: &str,
        name_ru: &str,
        rules_en: Option<&str>,
        rules_ru: Option<&str>,
        icon_url: Option<&str>,
        point_value: Option<f64>,
    ) -> Result<crate::models::BattleKeyword> {
        sqlx::query_as::<_, crate::models::BattleKeyword>(
            "UPDATE battle_keywords SET slug = $2, name_en = $3, name_ru = $4,
                    rules_en = $5, rules_ru = $6, icon_url = $7, point_value = $8,
                    updated_at = NOW()
              WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(slug)
        .bind(name_en)
        .bind(name_ru)
        .bind(rules_en)
        .bind(rules_ru)
        .bind(icon_url)
        .bind(point_value)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle keyword {id} not found")))
    }

    /// A keyword removed from the dictionary leaves the cards that named it
    /// standing. Their ability still carries the slug; it simply no longer
    /// resolves to a wording — the same forgiveness a race gets.
    pub async fn delete_battle_keyword(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM battle_keywords WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Battle keyword {id} not found")));
        }
        Ok(())
    }

    pub async fn set_battle_keyword_order(&self, ids: &[Uuid]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let orders: Vec<i32> = (0..ids.len() as i32).collect();
        let res = sqlx::query(
            "UPDATE battle_keywords AS k SET sort_order = v.ord
               FROM (SELECT * FROM UNNEST($1::uuid[], $2::int[]) AS t(id, ord)) AS v
              WHERE k.id = v.id",
        )
        .bind(ids)
        .bind(&orders)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    // === BATTLE ASSETS ===

    pub async fn list_battle_asset_sheets(
        &self,
    ) -> Result<Vec<crate::models::BattleAssetSheetListed>> {
        Ok(
            sqlx::query_as::<_, crate::models::BattleAssetSheetListed>(
                "SELECT s.id, s.name, s.source_url, s.width, s.height, s.settings,
                        s.sort_order, s.created_at,
                        (SELECT COUNT(*) FROM battle_assets a WHERE a.sheet_id = s.id)::bigint
                            AS part_count
                   FROM battle_asset_sheets s
                  ORDER BY s.sort_order NULLS LAST, s.created_at DESC, s.id",
            )
            .fetch_all(&self.pg_pool)
            .await?,
        )
    }

    pub async fn get_battle_asset_sheet(
        &self,
        id: Uuid,
    ) -> Result<crate::models::BattleAssetSheet> {
        sqlx::query_as::<_, crate::models::BattleAssetSheet>(
            "SELECT * FROM battle_asset_sheets WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle asset sheet {id} not found")))
    }

    pub async fn insert_battle_asset_sheet(
        &self,
        name: &str,
        source_url: &str,
        width: i32,
        height: i32,
    ) -> Result<crate::models::BattleAssetSheet> {
        Ok(sqlx::query_as::<_, crate::models::BattleAssetSheet>(
            "INSERT INTO battle_asset_sheets (name, source_url, width, height)
             VALUES ($1,$2,$3,$4) RETURNING *",
        )
        .bind(name)
        .bind(source_url)
        .bind(width)
        .bind(height)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn rename_battle_asset_sheet(
        &self,
        id: Uuid,
        name: &str,
    ) -> Result<crate::models::BattleAssetSheet> {
        sqlx::query_as::<_, crate::models::BattleAssetSheet>(
            "UPDATE battle_asset_sheets SET name = $2, updated_at = NOW()
              WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(name)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle asset sheet {id} not found")))
    }

    /// Remembering how a sheet was last cut. Written on every cut, so the next
    /// time the keeper opens it the knobs stand where they were left.
    pub async fn save_battle_asset_sheet_settings(
        &self,
        id: Uuid,
        settings: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE battle_asset_sheets SET settings = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(settings)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Clearing a sheet away leaves its parts standing, loose — the foreign key
    /// is ON DELETE SET NULL. A frame may already be wearing one of them.
    pub async fn delete_battle_asset_sheet(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM battle_asset_sheets WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!(
                "Battle asset sheet {id} not found"
            )));
        }
        Ok(())
    }

    /// Every filter in one statement rather than a hand-built query: a NULL
    /// bind means "no opinion", so the three questions the desk actually asks
    /// — which sheet, which role, which name — compose without string surgery.
    // ── Студия ──────────────────────────────────────────────────────────────
    //
    // Каждое чтение — с `owner_id` в `WHERE`, а не с проверкой после чтения:
    // забытая проверка после чтения это чужая работа на экране.

    /// Сколько занято в ящике. Одним запросом по обеим таблицам: ящик — это
    /// правда о складе, а не число, которое кто-то не забыл обновить.
    pub async fn studio_box_used(&self, owner: Uuid) -> Result<i64> {
        let used: Option<i64> = sqlx::query_scalar(
            "SELECT COALESCE((SELECT SUM(bytes) FROM studio_assets WHERE owner_id = $1), 0)
                  + COALESCE((SELECT SUM(bytes) FROM studio_sheets WHERE owner_id = $1), 0)",
        )
        .bind(owner)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(used.unwrap_or(0))
    }

    pub async fn list_studio_assets(
        &self,
        owner: Uuid,
    ) -> Result<Vec<crate::models::StudioAssetDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioAssetDto>(
            "SELECT id, sheet_id, name, role, url, width, height, bytes, created_at
               FROM studio_assets WHERE owner_id = $1 ORDER BY created_at DESC",
        )
        .bind(owner)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn list_studio_sheets(
        &self,
        owner: Uuid,
    ) -> Result<Vec<crate::models::StudioSheetDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioSheetDto>(
            "SELECT id, name, url, width, height, bytes, harvested_at, created_at
               FROM studio_sheets WHERE owner_id = $1 ORDER BY created_at DESC",
        )
        .bind(owner)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_studio_asset(
        &self,
        owner: Uuid,
        name: &str,
        role: &str,
        url: &str,
        width: i32,
        height: i32,
        bytes: i32,
    ) -> Result<crate::models::StudioAssetDto> {
        Ok(sqlx::query_as::<_, crate::models::StudioAssetDto>(
            "INSERT INTO studio_assets (owner_id, name, role, url, width, height, bytes)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, sheet_id, name, role, url, width, height, bytes, created_at",
        )
        .bind(owner)
        .bind(name)
        .bind(role)
        .bind(url)
        .bind(width)
        .bind(height)
        .bind(bytes)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// Убрать деталь. Возвращает её адрес на складе — файл убирает служба,
    /// а не запрос: база не знает про диск.
    pub async fn delete_studio_asset(&self, owner: Uuid, id: Uuid) -> Result<Option<String>> {
        Ok(
            sqlx::query_scalar("DELETE FROM studio_assets WHERE id = $1 AND owner_id = $2 RETURNING url")
                .bind(id)
                .bind(owner)
                .fetch_optional(&self.pg_pool)
                .await?,
        )
    }

    /// Сколько весят названные детали этого человека.
    ///
    /// По адресам, а не по идентификаторам: рама носит адрес картинки, и
    /// спрашивать её об идентификаторе — значит завести второе имя одной и той
    /// же вещи. Чужие адреса в счёт не идут: `owner_id` в условии.
    pub async fn studio_assets_weight(&self, owner: Uuid, urls: &[String]) -> Result<i64> {
        if urls.is_empty() {
            return Ok(0);
        }
        let sum: Option<i64> = sqlx::query_scalar(
            "SELECT SUM(bytes) FROM studio_assets WHERE owner_id = $1 AND url = ANY($2)",
        )
        .bind(owner)
        .bind(urls)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(sum.unwrap_or(0))
    }

    pub async fn set_studio_frame_bytes(&self, id: Uuid, bytes: i32) -> Result<()> {
        sqlx::query("UPDATE studio_frames SET bytes = $2 WHERE id = $1")
            .bind(id)
            .bind(bytes)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    /// С какой редакцией соглашения человек согласился и когда.
    pub async fn studio_agreement(&self, owner: Uuid) -> Result<Option<String>> {
        Ok(sqlx::query_scalar(
            "SELECT studio_agreement FROM users WHERE id = $1 AND studio_agreed_at IS NOT NULL",
        )
        .bind(owner)
        .fetch_optional(&self.pg_pool)
        .await?
        .flatten())
    }

    pub async fn accept_studio_agreement(&self, owner: Uuid, version: &str) -> Result<()> {
        sqlx::query(
            "UPDATE users SET studio_agreed_at = NOW(), studio_agreement = $2 WHERE id = $1",
        )
        .bind(owner)
        .bind(version)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn set_studio_frame_status(
        &self,
        owner: Uuid,
        id: Uuid,
        status: &str,
    ) -> Result<Option<crate::models::StudioFrameDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioFrameDto>(
            "UPDATE studio_frames SET status = $3, updated_at = NOW()
              WHERE id = $1 AND owner_id = $2 AND status IN ('draft', 'withdrawn')
             RETURNING id, name, body, status, admitted_at, bytes, created_at, updated_at",
        )
        .bind(id)
        .bind(owner)
        .bind(status)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn insert_studio_sheet(
        &self,
        owner: Uuid,
        name: &str,
        url: &str,
        width: i32,
        height: i32,
        bytes: i32,
    ) -> Result<crate::models::StudioSheetDto> {
        Ok(sqlx::query_as::<_, crate::models::StudioSheetDto>(
            "INSERT INTO studio_sheets (owner_id, name, url, width, height, bytes)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, name, url, width, height, bytes, harvested_at, created_at",
        )
        .bind(owner)
        .bind(name)
        .bind(url)
        .bind(width)
        .bind(height)
        .bind(bytes)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn get_studio_sheet(
        &self,
        owner: Uuid,
        id: Uuid,
    ) -> Result<Option<crate::models::StudioSheetDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioSheetDto>(
            "SELECT id, name, url, width, height, bytes, harvested_at, created_at
               FROM studio_sheets WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(owner)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Лист, из которого что-то вырезали, уборщик больше не трогает.
    pub async fn mark_studio_sheet_harvested(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE studio_sheets SET harvested_at = NOW() WHERE id = $1 AND harvested_at IS NULL")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn delete_studio_sheet(&self, owner: Uuid, id: Uuid) -> Result<Option<String>> {
        Ok(
            sqlx::query_scalar("DELETE FROM studio_sheets WHERE id = $1 AND owner_id = $2 RETURNING url")
                .bind(id)
                .bind(owner)
                .fetch_optional(&self.pg_pool)
                .await?,
        )
    }

    /// Сколько листов лежит сейчас и сколько принесли за сутки. Два числа
    /// одним запросом: они всегда спрашиваются вместе — перед загрузкой.
    pub async fn count_studio_sheets(&self, owner: Uuid) -> Result<(i64, i64)> {
        let row: (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*),
                    COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '1 day')
               FROM studio_sheets WHERE owner_id = $1",
        )
        .bind(owner)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row)
    }

    pub async fn attach_studio_asset_to_sheet(&self, id: Uuid, sheet: Uuid) -> Result<()> {
        sqlx::query("UPDATE studio_assets SET sheet_id = $2 WHERE id = $1")
            .bind(id)
            .bind(sheet)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_studio_asset(
        &self,
        owner: Uuid,
        id: Uuid,
    ) -> Result<Option<crate::models::StudioAssetDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioAssetDto>(
            "SELECT id, sheet_id, name, role, url, width, height, bytes, created_at
               FROM studio_assets WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(owner)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn rename_studio_asset(&self, owner: Uuid, id: Uuid, name: &str, role: &str) -> Result<()> {
        sqlx::query("UPDATE studio_assets SET name = $3, role = $4 WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner)
            .bind(name)
            .bind(role)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // ── Сезоны ──────────────────────────────────────────────────────────────

    pub async fn current_studio_season(&self) -> Result<Option<crate::models::StudioSeasonDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioSeasonDto>(
            "SELECT id, number, opens_at, closes_at, verdict_at, state, theme, theme_note
               FROM studio_seasons ORDER BY number DESC LIMIT 1",
        )
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn studio_season_by_number(
        &self,
        number: i32,
    ) -> Result<Option<crate::models::StudioSeasonDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioSeasonDto>(
            "SELECT id, number, opens_at, closes_at, verdict_at, state, theme, theme_note
               FROM studio_seasons WHERE number = $1",
        )
        .bind(number)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn insert_studio_season(
        &self,
        number: i32,
        opens: chrono::DateTime<chrono::Utc>,
        closes: chrono::DateTime<chrono::Utc>,
        verdict: chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::models::StudioSeasonDto> {
        Ok(sqlx::query_as::<_, crate::models::StudioSeasonDto>(
            "INSERT INTO studio_seasons (number, opens_at, closes_at, verdict_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (number) DO UPDATE SET number = EXCLUDED.number
             RETURNING id, number, opens_at, closes_at, verdict_at, state, theme, theme_note",
        )
        .bind(number)
        .bind(opens)
        .bind(closes)
        .bind(verdict)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn save_studio_season_theme(
        &self,
        id: Uuid,
        theme: Option<&str>,
        note: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE studio_seasons SET theme = $2, theme_note = $3 WHERE id = $1")
            .bind(id)
            .bind(theme)
            .bind(note)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    /// Сезоны, которым пора вердикта, но которых ещё не судили.
    ///
    /// Список, а не один: сервер мог простоять выходные, и тогда неподведённых
    /// окажется несколько. Пропустить их молча — значит потерять чью-то неделю.
    pub async fn studio_seasons_due(&self) -> Result<Vec<crate::models::StudioSeasonDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioSeasonDto>(
            "SELECT id, number, opens_at, closes_at, verdict_at, state, theme, theme_note
               FROM studio_seasons
              WHERE state <> 'judged' AND verdict_at <= NOW()
              ORDER BY number",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Заявки сезона с рамой и именем автора. `viewer` — чтобы принести свою
    /// оценку, если она есть: чужие голоса поимённо не показываются.
    pub async fn list_studio_entries(
        &self,
        season: Uuid,
        viewer: Option<Uuid>,
    ) -> Result<Vec<crate::models::StudioEntryDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioEntryDto>(
            "SELECT e.id, e.frame_id, f.name, f.body, e.author_id, u.display_name AS author,
                    e.entered_at, e.score::float8 AS score,
                    (SELECT COUNT(*) FROM studio_ratings r WHERE r.entry_id = e.id) AS votes,
                    e.place, e.to_keeper,
                    (SELECT r.value FROM studio_ratings r
                      WHERE r.entry_id = e.id AND r.voter_id = $2) AS mine
               FROM studio_entries e
               JOIN studio_frames f ON f.id = e.frame_id
               JOIN users u ON u.id = e.author_id
              WHERE e.season_id = $1
              ORDER BY e.place NULLS LAST, e.entered_at",
        )
        .bind(season)
        .bind(viewer)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn insert_studio_entry(
        &self,
        season: Uuid,
        frame: Uuid,
        author: Uuid,
    ) -> Result<Uuid> {
        Ok(sqlx::query_scalar(
            "INSERT INTO studio_entries (season_id, frame_id, author_id) VALUES ($1, $2, $3)
             RETURNING id",
        )
        .bind(season)
        .bind(frame)
        .bind(author)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn set_studio_frame_lang(&self, owner: Uuid, id: Uuid, lang: &str) -> Result<()> {
        sqlx::query("UPDATE studio_frames SET lang = $3 WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner)
            .bind(lang)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn studio_entry_frame(&self, entry: Uuid) -> Result<Option<Uuid>> {
        Ok(
            sqlx::query_scalar("SELECT frame_id FROM studio_entries WHERE id = $1")
                .bind(entry)
                .fetch_optional(&self.pg_pool)
                .await?,
        )
    }

    pub async fn studio_entry_author(&self, entry: Uuid) -> Result<Option<(Uuid, Uuid)>> {
        Ok(
            sqlx::query_as("SELECT author_id, season_id FROM studio_entries WHERE id = $1")
                .bind(entry)
                .fetch_optional(&self.pg_pool)
                .await?,
        )
    }

    /// Сколько этот человек уже оценил в этом сезоне.
    pub async fn count_studio_ratings(&self, season: Uuid, voter: Uuid) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM studio_ratings r
               JOIN studio_entries e ON e.id = r.entry_id
              WHERE e.season_id = $1 AND r.voter_id = $2",
        )
        .bind(season)
        .bind(voter)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// Оценка. Переоценить можно — мнение меняется; удвоить нельзя.
    pub async fn put_studio_rating(
        &self,
        entry: Uuid,
        voter: Uuid,
        value: i16,
        weight: f64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO studio_ratings (entry_id, voter_id, value, weight)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (entry_id, voter_id) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(entry)
        .bind(voter)
        .bind(value)
        .bind(weight)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Итог каждой заявки сезона: сумма «оценка × вес», сумма весов, число
    /// голосов. Считает база — перебирать тысячу оценок в памяти незачем.
    pub async fn studio_season_tallies(
        &self,
        season: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, f64, f64, i64, chrono::DateTime<chrono::Utc>)>> {
        Ok(sqlx::query_as(
            "SELECT e.id, e.author_id,
                    COALESCE(SUM(r.value * r.weight), 0)::float8,
                    COALESCE(SUM(r.weight), 0)::float8,
                    COUNT(r.voter_id),
                    e.entered_at
               FROM studio_entries e
               LEFT JOIN studio_ratings r ON r.entry_id = e.id
              WHERE e.season_id = $1
              GROUP BY e.id",
        )
        .bind(season)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn set_studio_entry_result(
        &self,
        entry: Uuid,
        score: f64,
        votes: i64,
        weight: f64,
        place: i16,
        to_keeper: bool,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE studio_entries
                SET score = $2, votes = $3, weight = $4, place = $5, to_keeper = $6
              WHERE id = $1",
        )
        .bind(entry)
        .bind(score)
        .bind(votes as i32)
        .bind(weight)
        .bind(place)
        .bind(to_keeper)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn set_studio_season_state(&self, id: Uuid, state: &str) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_seasons SET state = $2 WHERE id = $1 AND state <> 'judged'",
        )
        .bind(id)
        .bind(state)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Допуск: очередь и решение.
    pub async fn list_studio_admissions(&self) -> Result<Vec<crate::models::StudioFrameDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioFrameDto>(
            "SELECT id, name, body, status, admitted_at, bytes, created_at, updated_at
               FROM studio_frames
              WHERE status = 'shown' AND admitted_at IS NULL
              ORDER BY updated_at",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn admit_studio_frame(&self, id: Uuid, word: Option<&str>) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_frames SET admitted_at = NOW(), admit_word = $2
              WHERE id = $1 AND status = 'shown' AND admitted_at IS NULL",
        )
        .bind(id)
        .bind(word)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Не допустить: работа возвращается автору со словом. Возвращается именно
    /// в черновик — иначе её нельзя ни поправить, ни выбросить.
    pub async fn deny_studio_frame(&self, id: Uuid, word: &str) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_frames SET status = 'draft', admit_word = $2, updated_at = NOW()
              WHERE id = $1 AND status = 'shown' AND admitted_at IS NULL",
        )
        .bind(id)
        .bind(word)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Рама, допущенная и не выставленная: то, чем можно заявиться в сезон.
    pub async fn studio_frame_ready(
        &self,
        owner: Uuid,
        id: Uuid,
    ) -> Result<Option<(String, i16, bool)>> {
        Ok(sqlx::query_as(
            "SELECT status, entries_used, admitted_at IS NOT NULL
               FROM studio_frames WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(owner)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Работа пошла в неделю. Состояние при этом НЕ трогается: участие живёт в
    /// `studio_entries`, и вторая его копия в столбце — та самая, которую никто
    /// не снимал и из-за которой работа застревала навсегда.
    pub async fn mark_studio_frame_entered(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE studio_frames SET entries_used = entries_used + 1, updated_at = NOW()
              WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn insert_studio_report(
        &self,
        frame: Uuid,
        reporter: Option<Uuid>,
        reason: &str,
        note: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO studio_reports (frame_id, reporter_id, reason, note)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(frame)
        .bind(reporter)
        .bind(reason)
        .bind(note)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Возраст человека и сыграл ли он: две трети веса голоса.
    pub async fn voter_standing(&self, user: Uuid) -> Result<(bool, bool, bool)> {
        let row: (bool, bool, bool) = sqlx::query_as(
            "SELECT (u.created_at < NOW() - ($2 || ' days')::interval),
                    EXISTS (SELECT 1 FROM card_copies o WHERE o.owner_id = u.id),
                    EXISTS (SELECT 1 FROM battle_matches m WHERE m.user_id = u.id)
               FROM users u WHERE u.id = $1",
        )
        .bind(user)
        .bind(crate::studio::WEIGHT_AGE_DAYS.to_string())
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row)
    }

    /// Тройка сезона — то, что ушло хозяину.
    pub async fn list_studio_keeper_queue(
        &self,
        season: Uuid,
    ) -> Result<Vec<crate::models::StudioEntryDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioEntryDto>(
            "SELECT e.id, e.frame_id, f.name, f.body, e.author_id, u.display_name AS author,
                    e.entered_at, e.score::float8 AS score,
                    (SELECT COUNT(*) FROM studio_ratings r WHERE r.entry_id = e.id) AS votes,
                    e.place, e.to_keeper, NULL::smallint AS mine
               FROM studio_entries e
               JOIN studio_frames f ON f.id = e.frame_id
               JOIN users u ON u.id = e.author_id
              WHERE e.season_id = $1 AND e.to_keeper AND f.approved_at IS NULL
              ORDER BY e.place",
        )
        .bind(season)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Утвердить раму и назначить ей тираж лицензий.
    pub async fn approve_studio_frame(&self, id: Uuid, edition: Option<i32>) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_frames
                SET edition_size = $2, approved_at = NOW(), updated_at = NOW()
              WHERE id = $1 AND status = 'shown' AND approved_at IS NULL",
        )
        .bind(id)
        .bind(edition)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Снять работу насовсем. Купленные лицензии при этом не отбираются — люди
    /// не виноваты; здесь только сама работа уходит с людских глаз.
    pub async fn strike_studio_frame(&self, id: Uuid, word: &str) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_frames SET status = 'withdrawn', admit_word = $2, admitted_at = NULL,
                    updated_at = NOW()
              WHERE id = $1",
        )
        .bind(id)
        .bind(word)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn list_studio_reports(&self) -> Result<Vec<crate::models::StudioReportDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioReportDto>(
            "SELECT r.id, r.frame_id, f.name AS frame_name, f.body,
                    u.display_name AS reporter, r.reason, r.note, r.state, r.created_at
               FROM studio_reports r
               JOIN studio_frames f ON f.id = r.frame_id
               LEFT JOIN users u ON u.id = r.reporter_id
              WHERE r.state = 'open'
              ORDER BY r.created_at",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn close_studio_report(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE studio_reports SET state = 'closed' WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    /// Сколько чего ждёт хозяина: допуск, тройка, жалобы. Одним запросом —
    /// значок на вкладке спрашивается на каждом заходе в админку.
    pub async fn studio_waiting(&self) -> Result<(i64, i64, i64)> {
        let row: (i64, i64, i64) = sqlx::query_as(
            "SELECT
               (SELECT COUNT(*) FROM studio_frames
                 WHERE status = 'shown' AND admitted_at IS NULL),
               (SELECT COUNT(*) FROM studio_entries e
                  JOIN studio_frames f ON f.id = e.frame_id
                 WHERE e.to_keeper AND f.approved_at IS NULL),
               (SELECT COUNT(*) FROM studio_reports WHERE state = 'open')",
        )
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row)
    }

    // ── Галерея и Зал авторов ───────────────────────────────────────────────

    /// Рамки, которые хозяин УТВЕРДИЛ. Список для его же стола — надевать на
    /// карту чужую работу можно только из уже одобренного, не из допущенного:
    /// допуск говорит «люди могут посмотреть», а не «дом ручается».
    pub async fn list_approved_studio_frames(
        &self,
    ) -> Result<Vec<crate::models::StudioShownDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioShownDto>(
            "SELECT f.id, f.name, f.body, f.status, u.display_name AS author,
                    u.studio_slug AS author_slug, f.admitted_at, f.approved_at, f.edition_size
               FROM studio_frames f
               JOIN users u ON u.id = f.owner_id
              WHERE f.approved_at IS NOT NULL
              ORDER BY f.approved_at DESC",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Всё, что допущено на люди. Страницами: полка растёт, и «все сразу» это
    /// ответ, который однажды перестанет приходить.
    pub async fn list_studio_gallery(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::StudioShownDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioShownDto>(
            "SELECT f.id, f.name, f.body, f.status, u.display_name AS author,
                    u.studio_slug AS author_slug, f.admitted_at, f.approved_at, f.edition_size
               FROM studio_frames f
               JOIN users u ON u.id = f.owner_id
              WHERE f.admitted_at IS NOT NULL AND f.status <> 'withdrawn'
              ORDER BY f.approved_at DESC NULLS LAST, f.admitted_at DESC
              LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn count_studio_gallery(&self) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM studio_frames
              WHERE admitted_at IS NOT NULL AND status <> 'withdrawn'",
        )
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// Автор и его работы. По слугу — адрес должен читаться словами.
    pub async fn studio_author(
        &self,
        slug: &str,
    ) -> Result<Option<(Uuid, String)>> {
        Ok(
            sqlx::query_as("SELECT id, display_name FROM users WHERE studio_slug = $1")
                .bind(slug)
                .fetch_optional(&self.pg_pool)
                .await?,
        )
    }

    pub async fn list_studio_author_works(
        &self,
        author: Uuid,
    ) -> Result<Vec<crate::models::StudioShownDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioShownDto>(
            "SELECT f.id, f.name, f.body, f.status, u.display_name AS author,
                    u.studio_slug AS author_slug, f.admitted_at, f.approved_at, f.edition_size
               FROM studio_frames f
               JOIN users u ON u.id = f.owner_id
              WHERE f.owner_id = $1 AND f.admitted_at IS NOT NULL AND f.status <> 'withdrawn'
              ORDER BY f.approved_at DESC NULLS LAST, f.admitted_at DESC",
        )
        .bind(author)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Карты автора, дошедшие до полки. Только взятые домом: непринятая работа
    /// — это черновик, и показывать его на людской странице нечего.
    pub async fn list_studio_author_cards(
        &self,
        author: Uuid,
    ) -> Result<Vec<crate::models::StudioAuthorCardDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioAuthorCardDto>(
            "SELECT c.id, c.body, c.card_id, c.approved_at
               FROM studio_cards c
              WHERE c.owner_id = $1 AND c.approved_at IS NOT NULL
              ORDER BY c.approved_at DESC",
        )
        .bind(author)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn studio_slug_taken(&self, slug: &str) -> Result<bool> {
        Ok(
            sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE studio_slug = $1)")
                .bind(slug)
                .fetch_one(&self.pg_pool)
                .await?,
        )
    }

    pub async fn set_studio_slug(&self, user: Uuid, slug: &str) -> Result<()> {
        sqlx::query("UPDATE users SET studio_slug = $2 WHERE id = $1 AND studio_slug IS NULL")
            .bind(user)
            .bind(slug)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    /// Кому принадлежит работа, как его звать, его адрес и на каком языке с
    /// ним говорить. Одним запросом: записка пишется сразу после решения, и
    /// четыре похода в базу за одним и тем же — это четыре места, где можно
    /// разойтись.
    pub async fn studio_frame_owner(
        &self,
        id: Uuid,
    ) -> Result<Option<(Uuid, String, Option<String>, String, String)>> {
        Ok(sqlx::query_as(
            "SELECT u.id, u.display_name, u.studio_slug, f.lang, f.name
               FROM studio_frames f JOIN users u ON u.id = f.owner_id WHERE f.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    // ── Лицензии ────────────────────────────────────────────────────────────
    //
    // Лицензия — право носить раму. Продаётся ИМЕННО ОНА, а не рама: продать
    // «раму» значило бы раздеть в момент сделки все карты автора, её носившие.

    /// Отпечатать тираж автору. Разом, одним запросом, и только если у рамы
    /// ещё нет ни одной лицензии: утверждение могут нажать дважды, а тираж —
    /// число, а не действие.
    pub async fn mint_frame_licences(
        &self,
        frame: Uuid,
        owner: Uuid,
        count: i32,
    ) -> Result<u64> {
        let done = sqlx::query(
            "INSERT INTO frame_licenses (frame_id, owner_id, serial, origin)
             SELECT $1, $2, n, 'author' FROM generate_series(1, $3) AS n
              WHERE NOT EXISTS (SELECT 1 FROM frame_licenses WHERE frame_id = $1)",
        )
        .bind(frame)
        .bind(owner)
        .bind(count)
        .execute(&self.pg_pool)
        .await?;
        Ok(done.rows_affected())
    }

    pub async fn count_frame_licences(&self, frame: Uuid) -> Result<(i64, i64)> {
        let row: (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*), COUNT(*) FILTER (WHERE owner_id IS NOT NULL)
               FROM frame_licenses WHERE frame_id = $1",
        )
        .bind(frame)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row)
    }

    /// Лицензии человека, с рамой и именем автора: так их и показывают.
    pub async fn list_my_licences(&self, owner: Uuid) -> Result<Vec<crate::models::LicenceDto>> {
        Ok(sqlx::query_as::<_, crate::models::LicenceDto>(
            "SELECT l.id, l.frame_id, f.name, f.body, l.serial, f.edition_size,
                    u.display_name AS author, u.studio_slug AS author_slug,
                    l.origin, l.locked_by IS NOT NULL AS locked
               FROM frame_licenses l
               JOIN studio_frames f ON f.id = l.frame_id
               JOIN users u ON u.id = f.owner_id
              WHERE l.owner_id = $1
              ORDER BY l.acquired_at DESC",
        )
        .bind(owner)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    // ── Лавка авторов ───────────────────────────────────────────────────────

    /// Выставить лицензию. Она при этом ЗАПИРАЕТСЯ: выставленной нельзя ни
    /// играть, ни продать второму — эскроу без слова «эскроу».
    /// Выставить СВОЮ вещь: лицензию на раму или экземпляр карты.
    ///
    /// Одна дорога на оба вида, потому что сделка у них одна: вещь запирается
    /// объявлением, деньги и владение меняются одной транзакцией. Отличается
    /// только таблица, в которой вещь лежит, и держать ради этого два разных
    /// пути значило бы однажды починить один и забыть другой.
    ///
    /// `locked_by IS NULL` в отборе — вся защита от второго объявления: вещь,
    /// уже стоящую на прилавке, нельзя ни выставить снова, ни играть ею.
    pub async fn list_thing(
        &self,
        seller: Uuid,
        kind: &str,
        subject: Uuid,
        price: i32,
        currency: &str,
    ) -> Result<Uuid> {
        let (pick, lock) = if kind == "copy" {
            (
                "INSERT INTO market_listings (kind, subject_id, seller_id, price, currency)
                 SELECT 'copy', c.id, $1, $3, $4 FROM card_copies c
                  WHERE c.id = $2 AND c.owner_id = $1 AND c.locked_by IS NULL
                 RETURNING id",
                "UPDATE card_copies SET locked_by = $2 WHERE id = $1",
            )
        } else {
            (
                "INSERT INTO market_listings (kind, subject_id, seller_id, price, currency)
                 SELECT 'license', l.id, $1, $3, $4 FROM frame_licenses l
                  WHERE l.id = $2 AND l.owner_id = $1 AND l.locked_by IS NULL
                 RETURNING id",
                "UPDATE frame_licenses SET locked_by = $2 WHERE id = $1",
            )
        };
        let mut tx = self.pg_pool.begin().await?;
        let listing: Uuid = sqlx::query_scalar(pick)
            .bind(seller)
            .bind(subject)
            .bind(price)
            .bind(currency)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::BadRequest("notYours".into()))?;
        sqlx::query(lock)
            .bind(subject)
            .bind(listing)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(listing)
    }

    /// Снять с продажи: объявление закрыто, лицензия отперта.
    pub async fn withdraw_listing(&self, seller: Uuid, listing: Uuid) -> Result<bool> {
        let mut tx = self.pg_pool.begin().await?;
        let hit = sqlx::query(
            "UPDATE market_listings SET state = 'withdrawn', closed_at = NOW()
              WHERE id = $1 AND seller_id = $2 AND state = 'open'",
        )
        .bind(listing)
        .bind(seller)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        // Отпираем обе таблицы: вещь лежит в одной из них, и спрашивать в
        // какой — значит держать вид объявления вторым знанием рядом с самим
        // объявлением.
        sqlx::query("UPDATE frame_licenses SET locked_by = NULL WHERE locked_by = $1")
            .bind(listing)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE card_copies SET locked_by = NULL WHERE locked_by = $1")
            .bind(listing)
            .execute(&mut *tx)
            .await?;
        // Мены на снятое отпадают вместе с ним: иначе предложенное осталось бы
        // запертым за объявлением, которого больше нет.
        if hit > 0 {
            self.drop_other_trades(&mut tx, listing, None).await?;
        }
        tx.commit().await?;
        Ok(hit > 0)
    }

    /// Купить. Всё одной транзакцией: списание, выплата автору, сгоревшая
    /// комиссия, смена владельца, закрытие объявления. Половина сделки — это
    /// потерянные деньги или потерянная вещь, и середины здесь быть не может.
    ///
    /// Кошелёк покупателя запирается тем же advisory-замком, что и покупка
    /// карты: два нажатия в двух вкладках встают в очередь, а не читают один и
    /// тот же баланс и оба слышат «хватает».
    pub async fn buy_listing(
        &self,
        buyer: Uuid,
        listing: Uuid,
        commission_percent: i32,
    ) -> Result<(i32, String, Uuid)> {
        let mut tx = self.pg_pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(buyer.to_string())
            .execute(&mut *tx)
            .await?;

        let row: Option<(Uuid, Uuid, i32, String, String)> = sqlx::query_as(
            "SELECT subject_id, seller_id, price, currency, kind FROM market_listings
              WHERE id = $1 AND state = 'open' FOR UPDATE",
        )
        .bind(listing)
        .fetch_optional(&mut *tx)
        .await?;
        let (thing, seller, price, currency, kind) =
            row.ok_or_else(|| AppError::BadRequest("gone".into()))?;
        if seller == buyer {
            tx.rollback().await?;
            return Err(AppError::BadRequest("ownListing".into()));
        }

        let balance = self.battle_wallet_balance(buyer, &currency).await?;
        if balance < i64::from(price) {
            tx.rollback().await?;
            return Err(AppError::BadRequest("notEnough".into()));
        }

        // Комиссия дома СГОРАЕТ: она никому не начисляется. Это единственный
        // сток валюты — без него за месяц цены перестанут что-то значить.
        let commission = (price * commission_percent) / 100;
        let to_seller = price - commission;

        sqlx::query(
            "INSERT INTO battle_wallet_entries (user_id, currency, amount, reason, ref_id, idem_key)
             VALUES ($1, $2, $3, 'market_bought', $4, $5)",
        )
        .bind(buyer)
        .bind(&currency)
        .bind(-price)
        .bind(listing)
        .bind(format!("market:{listing}:paid"))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO battle_wallet_entries (user_id, currency, amount, reason, ref_id, idem_key)
             VALUES ($1, $2, $3, 'market_sold', $4, $5)",
        )
        .bind(seller)
        .bind(&currency)
        .bind(to_seller)
        .bind(listing)
        .bind(format!("market:{listing}:got"))
        .execute(&mut *tx)
        .await?;

        // Смена владельца — ОДНА СТРОКА, и в этом весь смысл экземпляра:
        // ничего не копируется, ничего не исчезает, и правило «продал — у тебя
        // больше нет» выполняется формой таблицы, а не проверкой.
        let move_it = if kind == "copy" {
            "UPDATE card_copies SET owner_id = $2, origin = 'bought', locked_by = NULL,
                    acquired_at = NOW(), seen_at = NULL
              WHERE id = $1"
        } else {
            "UPDATE frame_licenses SET owner_id = $2, origin = 'bought', locked_by = NULL,
                    acquired_at = NOW()
              WHERE id = $1"
        };
        sqlx::query(move_it)
            .bind(thing)
            .bind(buyer)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE market_listings SET state = 'sold', closed_at = NOW() WHERE id = $1",
        )
        .bind(listing)
        .execute(&mut *tx)
        .await?;
        // Кто-то отдал за неё пыль — мены на эту вещь отпадают, и предложенное
        // возвращается своим хозяевам отпертым.
        self.drop_other_trades(&mut tx, listing, None).await?;

        tx.commit().await?;
        Ok((price, currency, thing))
    }

    /// Полка лавки: что продаётся сейчас.
    pub async fn list_market(&self) -> Result<Vec<crate::models::ListingDto>> {
        Ok(sqlx::query_as::<_, crate::models::ListingDto>(
            "SELECT m.id, l.id AS licence_id, m.price, m.currency, l.serial, f.id AS frame_id, f.name, f.body,
                    f.edition_size, u.display_name AS seller, a.display_name AS author,
                    a.studio_slug AS author_slug, m.created_at
               FROM market_listings m
               JOIN frame_licenses l ON l.id = m.subject_id
               JOIN studio_frames f ON f.id = l.frame_id
               JOIN users u ON u.id = m.seller_id
               JOIN users a ON a.id = f.owner_id
              WHERE m.state = 'open' AND m.kind = 'license'
              ORDER BY m.created_at DESC",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Экземпляры карт на прилавке.
    ///
    /// Только тех карт, что стоят на полке дома: снятую с полки карту дом не
    /// показывает и в лавке — иначе человек купил бы вещь, которой на полке
    /// нет и посмотреть которую негде.
    pub async fn list_market_copies(&self) -> Result<Vec<crate::models::CopyListingDto>> {
        Ok(sqlx::query_as::<_, crate::models::CopyListingDto>(
            "SELECT m.id, c.id AS copy_id, m.price, m.currency, c.serial, c.level,
                    c.card_id, k.title_en, k.title_ru, k.credit_name,
                    u.display_name AS seller, m.created_at
               FROM market_listings m
               JOIN card_copies c ON c.id = m.subject_id
               JOIN battle_cards k ON k.id = c.card_id
               JOIN users u ON u.id = m.seller_id
              WHERE m.state = 'open' AND m.kind = 'copy' AND k.status = 'published'
              ORDER BY m.created_at DESC",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Свои экземпляры — ВСЕ, включая запертые: продавать их человек приходит
    /// сюда же, и вещь, пропавшая из списка ровно потому, что выставлена, —
    /// это вещь, которую нельзя снять с продажи.
    pub async fn list_my_copies(&self, owner: Uuid) -> Result<Vec<crate::models::MyCopyDto>> {
        Ok(sqlx::query_as::<_, crate::models::MyCopyDto>(
            "SELECT c.id, c.card_id, c.serial, c.level, c.locked_by IS NOT NULL AS locked,
                    k.title_en, k.title_ru
               FROM card_copies c
               JOIN battle_cards k ON k.id = c.card_id
              WHERE c.owner_id = $1
              ORDER BY c.acquired_at DESC",
        )
        .bind(owner)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    // ── Мена ────────────────────────────────────────────────────────────────

    /// Предложить мену против объявления.
    ///
    /// Запираются ТОЛЬКО вещи предлагающего. Запирать заодно и чужие — значит
    /// дать любому заморозить чужое собрание одним предложением, которое он и
    /// не собирался доводить до конца.
    ///
    /// Вещи проверяются тем же отбором, что и на прилавке (`owner_id` и
    /// `locked_by IS NULL`): предложить чужое или уже занятое нельзя, и это
    /// отбор в запросе, а не проверка на странице.
    pub async fn offer_trade(
        &self,
        from: Uuid,
        listing: Uuid,
        items: &[crate::models::TradeItem],
    ) -> Result<Uuid> {
        let mut tx = self.pg_pool.begin().await?;

        // Объявление должно быть открыто и НЕ СВОИМ: мена с собой — это не
        // мена, а способ запереть собственную вещь дважды.
        let seller: Option<(Uuid,)> = sqlx::query_as(
            "SELECT seller_id FROM market_listings WHERE id = $1 AND state = 'open'",
        )
        .bind(listing)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((seller,)) = seller else {
            tx.rollback().await?;
            return Err(AppError::BadRequest("gone".into()));
        };
        if seller == from {
            tx.rollback().await?;
            return Err(AppError::BadRequest("ownListing".into()));
        }

        let trade: Uuid = sqlx::query_scalar(
            "INSERT INTO trades (listing_id, from_id) VALUES ($1, $2) RETURNING id",
        )
        .bind(listing)
        .bind(from)
        .fetch_one(&mut *tx)
        .await?;

        for item in items {
            let lock = if item.kind == "copy" {
                "UPDATE card_copies SET locked_by = $3
                  WHERE id = $1 AND owner_id = $2 AND locked_by IS NULL"
            } else {
                "UPDATE frame_licenses SET locked_by = $3
                  WHERE id = $1 AND owner_id = $2 AND locked_by IS NULL"
            };
            let hit = sqlx::query(lock)
                .bind(item.subject_id)
                .bind(from)
                .bind(trade)
                .execute(&mut *tx)
                .await?
                .rows_affected();
            if hit == 0 {
                // Ни одной половины мены: либо вся она законна, либо её нет.
                tx.rollback().await?;
                return Err(AppError::BadRequest("notYours".into()));
            }
            sqlx::query(
                "INSERT INTO trade_items (trade_id, kind, subject_id) VALUES ($1, $2, $3)",
            )
            .bind(trade)
            .bind(&item.kind)
            .bind(item.subject_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(trade)
    }

    /// Принять мену. Всё одной транзакцией: вещь с прилавка идёт предложившему,
    /// предложенное — продавцу, объявление закрывается словом `traded`.
    ///
    /// Принять может ТОЛЬКО хозяин объявления, и это условие стоит в запросе.
    pub async fn accept_trade(&self, seller: Uuid, trade: Uuid) -> Result<bool> {
        let mut tx = self.pg_pool.begin().await?;

        let row: Option<(Uuid, Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT t.from_id, m.id, m.subject_id, m.kind
               FROM trades t
               JOIN market_listings m ON m.id = t.listing_id
              WHERE t.id = $1 AND t.state = 'offered'
                AND m.state = 'open' AND m.seller_id = $2
              FOR UPDATE OF t, m",
        )
        .bind(trade)
        .bind(seller)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((buyer, listing, subject, kind)) = row else {
            tx.rollback().await?;
            return Ok(false);
        };

        // Вещь с прилавка — предложившему.
        let hand_over = if kind == "copy" {
            "UPDATE card_copies SET owner_id = $2, origin = 'traded', locked_by = NULL,
                    acquired_at = NOW(), seen_at = NULL WHERE id = $1"
        } else {
            "UPDATE frame_licenses SET owner_id = $2, origin = 'traded', locked_by = NULL,
                    acquired_at = NOW() WHERE id = $1"
        };
        sqlx::query(hand_over)
            .bind(subject)
            .bind(buyer)
            .execute(&mut *tx)
            .await?;

        // Предложенное — продавцу. Отпираем тем же движением: вещь занята была
        // ЭТОЙ меной, и держать её запертой после неё нечем.
        let items: Vec<(String, Uuid)> =
            sqlx::query_as("SELECT kind, subject_id FROM trade_items WHERE trade_id = $1")
                .bind(trade)
                .fetch_all(&mut *tx)
                .await?;
        for (item_kind, id) in items {
            let move_it = if item_kind == "copy" {
                "UPDATE card_copies SET owner_id = $2, origin = 'traded', locked_by = NULL,
                        acquired_at = NOW(), seen_at = NULL WHERE id = $1"
            } else {
                "UPDATE frame_licenses SET owner_id = $2, origin = 'traded', locked_by = NULL,
                        acquired_at = NOW() WHERE id = $1"
            };
            sqlx::query(move_it)
                .bind(id)
                .bind(seller)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query(
            "UPDATE market_listings SET state = 'traded', closed_at = NOW() WHERE id = $1",
        )
        .bind(listing)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE trades SET state = 'taken', closed_at = NOW() WHERE id = $1")
            .bind(trade)
            .execute(&mut *tx)
            .await?;
        // Остальные мены на это объявление отпадают: вещь ушла.
        self.drop_other_trades(&mut tx, listing, Some(trade)).await?;

        tx.commit().await?;
        Ok(true)
    }

    /// Отказать в мене или забрать её назад — одним движением, потому что
    /// делают эти двое одно и то же: мена кончается, вещи отпираются.
    pub async fn refuse_trade(&self, who: Uuid, trade: Uuid) -> Result<bool> {
        let mut tx = self.pg_pool.begin().await?;
        let hit = sqlx::query(
            "UPDATE trades t SET state = 'refused', closed_at = NOW()
               FROM market_listings m
              WHERE t.id = $1 AND t.state = 'offered' AND m.id = t.listing_id
                AND (t.from_id = $2 OR m.seller_id = $2)",
        )
        .bind(trade)
        .bind(who)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if hit > 0 {
            Self::unlock_trade(&mut tx, trade).await?;
        }
        tx.commit().await?;
        Ok(hit > 0)
    }

    /// Отпереть вещи, занятые меной.
    async fn unlock_trade(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        trade: Uuid,
    ) -> Result<()> {
        sqlx::query("UPDATE card_copies SET locked_by = NULL WHERE locked_by = $1")
            .bind(trade)
            .execute(&mut **tx)
            .await?;
        sqlx::query("UPDATE frame_licenses SET locked_by = NULL WHERE locked_by = $1")
            .bind(trade)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    /// Мены, оставшиеся без вещи: объявление закрыто, и держать их нельзя —
    /// предложенное осталось бы запертым навсегда.
    async fn drop_other_trades(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        listing: Uuid,
        except: Option<Uuid>,
    ) -> Result<()> {
        let others: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM trades
              WHERE listing_id = $1 AND state = 'offered' AND ($2::uuid IS NULL OR id <> $2)",
        )
        .bind(listing)
        .bind(except)
        .fetch_all(&mut **tx)
        .await?;
        for (id,) in others {
            sqlx::query("UPDATE trades SET state = 'gone', closed_at = NOW() WHERE id = $1")
                .bind(id)
                .execute(&mut **tx)
                .await?;
            Self::unlock_trade(tx, id).await?;
        }
        Ok(())
    }

    /// Мены, которые касаются этого человека: и те, что он предложил, и те, что
    /// предложили ему.
    pub async fn list_trades(&self, who: Uuid) -> Result<Vec<crate::models::TradeDto>> {
        let rows: Vec<(Uuid, Uuid, String, String, Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT t.id, t.listing_id, t.state, u.display_name, t.from_id, t.created_at
               FROM trades t
               JOIN market_listings m ON m.id = t.listing_id
               JOIN users u ON u.id = t.from_id
              WHERE t.state = 'offered' AND (t.from_id = $1 OR m.seller_id = $1)
              ORDER BY t.created_at DESC",
        )
        .bind(who)
        .fetch_all(&self.pg_pool)
        .await?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let listing_ids: Vec<Uuid> = rows.iter().map(|(_, listing, ..)| *listing).collect();
        let trade_ids: Vec<Uuid> = rows.iter().map(|(id, ..)| *id).collect();
        let names = self.listing_names_batch(&listing_ids).await?;
        let mut gives = self.trade_items_batch(&trade_ids).await?;

        let mut out = Vec::with_capacity(rows.len());
        for (id, listing, state, from, from_id, created_at) in rows {
            out.push(crate::models::TradeDto {
                id,
                listing_id: listing,
                state,
                from,
                mine: from_id == who,
                want: names.get(&listing).cloned().unwrap_or_default(),
                gives: gives.remove(&id).unwrap_or_default(),
                created_at,
            });
        }
        Ok(out)
    }

    /// Как называются вещи на прилавке, для целого списка лотов сразу — одним
    /// запросом, а не одним на каждый ряд мен. Одним запросом на оба вида: у
    /// лицензии имя рамы, у экземпляра — имя карты.
    async fn listing_names_batch(
        &self,
        listings: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>> {
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT m.id, COALESCE(f.name, k.title_ru, '')
               FROM market_listings m
               LEFT JOIN frame_licenses l ON l.id = m.subject_id AND m.kind = 'license'
               LEFT JOIN studio_frames f ON f.id = l.frame_id
               LEFT JOIN card_copies c ON c.id = m.subject_id AND m.kind = 'copy'
               LEFT JOIN battle_cards k ON k.id = c.card_id
              WHERE m.id = ANY($1)",
        )
        .bind(listings)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows.into_iter().collect())
    }

    /// То же для того, что предложено В мену, тоже одним запросом на весь
    /// список мен.
    async fn trade_items_batch(
        &self,
        trades: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<String>>> {
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT i.trade_id, COALESCE(f.name, k.title_ru, '')
               FROM trade_items i
               LEFT JOIN frame_licenses l ON l.id = i.subject_id AND i.kind = 'license'
               LEFT JOIN studio_frames f ON f.id = l.frame_id
               LEFT JOIN card_copies c ON c.id = i.subject_id AND i.kind = 'copy'
               LEFT JOIN battle_cards k ON k.id = c.card_id
              WHERE i.trade_id = ANY($1)",
        )
        .bind(trades)
        .fetch_all(&self.pg_pool)
        .await?;
        let mut out: std::collections::HashMap<Uuid, Vec<String>> =
            std::collections::HashMap::new();
        for (trade_id, name) in rows {
            out.entry(trade_id).or_default().push(name);
        }
        Ok(out)
    }

    // ── Свои роды ───────────────────────────────────────────────────────────

    const STUDIO_RACE_SELECT: &'static str =
        "id, name_en, name_ru, note_en, note_ru, icon_url, status, keeper_word,
         approved_at, race_id, created_at, updated_at";

    pub async fn list_studio_races(
        &self,
        owner: Uuid,
    ) -> Result<Vec<crate::models::StudioRaceDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioRaceDto>(&format!(
            "SELECT {} FROM studio_races WHERE owner_id = $1 ORDER BY updated_at DESC",
            Self::STUDIO_RACE_SELECT
        ))
        .bind(owner)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn count_studio_races_open(&self, owner: Uuid) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM studio_races
              WHERE owner_id = $1 AND status IN ('draft', 'withdrawn')",
        )
        .bind(owner)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn insert_studio_race(
        &self,
        owner: Uuid,
        req: &crate::models::SaveStudioRaceRequest,
    ) -> Result<crate::models::StudioRaceDto> {
        Ok(sqlx::query_as::<_, crate::models::StudioRaceDto>(&format!(
            "INSERT INTO studio_races (owner_id, name_en, name_ru, note_en, note_ru, icon_url, lang)
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING {}",
            Self::STUDIO_RACE_SELECT
        ))
        .bind(owner)
        .bind(&req.name_en)
        .bind(&req.name_ru)
        .bind(req.note_en.as_deref())
        .bind(req.note_ru.as_deref())
        .bind(req.icon_url.as_deref())
        .bind(&req.lang)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// `status` в условии — забор на стороне базы: отданное хозяину не
    /// правится, взятое домом тем более.
    pub async fn update_studio_race(
        &self,
        owner: Uuid,
        id: Uuid,
        req: &crate::models::SaveStudioRaceRequest,
    ) -> Result<Option<crate::models::StudioRaceDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioRaceDto>(&format!(
            "UPDATE studio_races
                SET name_en = $3, name_ru = $4, note_en = $5, note_ru = $6, icon_url = $7,
                    updated_at = NOW()
              WHERE id = $1 AND owner_id = $2 AND status IN ('draft', 'withdrawn')
             RETURNING {}",
            Self::STUDIO_RACE_SELECT
        ))
        .bind(id)
        .bind(owner)
        .bind(&req.name_en)
        .bind(&req.name_ru)
        .bind(req.note_en.as_deref())
        .bind(req.note_ru.as_deref())
        .bind(req.icon_url.as_deref())
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn set_studio_race_status(
        &self,
        owner: Uuid,
        id: Uuid,
        to: &str,
        from: &[&str],
    ) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_races SET status = $3, updated_at = NOW()
              WHERE id = $1 AND owner_id = $2 AND status = ANY($4)",
        )
        .bind(id)
        .bind(owner)
        .bind(to)
        .bind(from)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn delete_studio_race(&self, owner: Uuid, id: Uuid) -> Result<bool> {
        Ok(sqlx::query(
            // Взятый домом не выбрасывают: его уже носят чужие карты.
            "DELETE FROM studio_races WHERE id = $1 AND owner_id = $2 AND approved_at IS NULL",
        )
        .bind(id)
        .bind(owner)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn studio_race_queue(&self) -> Result<Vec<crate::models::StudioRaceQueueDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioRaceQueueDto>(
            "SELECT r.id, r.name_en, r.name_ru, r.note_en, r.note_ru, r.icon_url,
                    u.display_name AS author, r.updated_at
               FROM studio_races r
               JOIN users u ON u.id = r.owner_id
              WHERE r.status = 'shown'
              ORDER BY r.updated_at",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Работа со стороны хозяина: чья она и на каком языке писалась.
    pub async fn get_studio_race_any(
        &self,
        id: Uuid,
    ) -> Result<Option<(crate::models::StudioRaceDto, Uuid, String)>> {
        let row: Option<(Uuid, String)> =
            sqlx::query_as("SELECT owner_id, lang FROM studio_races WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pg_pool)
                .await?;
        let Some((owner, lang)) = row else {
            return Ok(None);
        };
        let race = sqlx::query_as::<_, crate::models::StudioRaceDto>(&format!(
            "SELECT {} FROM studio_races WHERE id = $1",
            Self::STUDIO_RACE_SELECT
        ))
        .bind(id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(Some((race, owner, lang)))
    }

    pub async fn deny_studio_race(&self, id: Uuid, word: &str) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_races SET status = 'draft', keeper_word = $2, updated_at = NOW()
              WHERE id = $1 AND status = 'shown'",
        )
        .bind(id)
        .bind(word)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn mark_studio_race_approved(&self, id: Uuid, race: Uuid) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_races SET approved_at = NOW(), race_id = $2, updated_at = NOW()
              WHERE id = $1 AND approved_at IS NULL",
        )
        .bind(id)
        .bind(race)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Подписать род автором — тем же движением, каким подписывается карта.
    pub async fn set_battle_race_credit(&self, id: Uuid, name: &str) -> Result<()> {
        sqlx::query("UPDATE battle_races SET credit_name = $2 WHERE id = $1")
            .bind(id)
            .bind(name)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // ── Свои движения ───────────────────────────────────────────────────────

    /// Ящик человека целиком — тем же видом, каким его читает стол такта.
    pub async fn list_studio_motions(&self, owner: Uuid) -> Result<Vec<crate::models::StudioMotionDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioMotionDto>(
            "SELECT id, body, status, keeper_word, approved_at, house_id, created_at, updated_at
               FROM studio_motions WHERE owner_id = $1 ORDER BY created_at",
        )
        .bind(owner)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Записать движение человека. Ящик правится целиком, поэтому запись идёт
    /// по ИМЕНИ движения: стол сохраняет весь ящик разом, и попытка угадать,
    /// какая строка какому движению отвечает, разошлась бы с ним на первой же
    /// перестановке.
    ///
    /// Отданное хозяину и взятое домом не переписываются — условие в запросе,
    /// а не проверка в службе.
    pub async fn upsert_studio_motion(
        &self,
        owner: Uuid,
        name: &str,
        body: &serde_json::Value,
        lang: &str,
    ) -> Result<()> {
        // `lang` только на вставке: как у карты, язык называет речь, на
        // которой работа заведена, а не последнюю, на которой стол открыт.
        sqlx::query(
            "INSERT INTO studio_motions (owner_id, body, lang) VALUES ($1, $2, $3)
             ON CONFLICT (owner_id, (body->>'id')) DO UPDATE
                SET body = EXCLUDED.body, updated_at = NOW()
              WHERE studio_motions.status IN ('draft', 'withdrawn')",
        )
        .bind(owner)
        .bind(body)
        .bind(lang)
        .execute(&self.pg_pool)
        .await?;
        let _ = name;
        Ok(())
    }

    /// Убрать то, чего в ящике больше нет. Отданное и взятое остаётся: человек
    /// стёр его у себя, но хозяин уже смотрит, а взятое домом носят чужие карты.
    pub async fn drop_missing_studio_motions(
        &self,
        owner: Uuid,
        keep: &[String],
    ) -> Result<u64> {
        Ok(sqlx::query(
            "DELETE FROM studio_motions
              WHERE owner_id = $1 AND status IN ('draft', 'withdrawn')
                AND NOT (body->>'id' = ANY($2))",
        )
        .bind(owner)
        .bind(keep)
        .execute(&self.pg_pool)
        .await?
        .rows_affected())
    }

    pub async fn set_studio_motion_status(
        &self,
        owner: Uuid,
        id: Uuid,
        to: &str,
        from: &[&str],
    ) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_motions SET status = $3, updated_at = NOW()
              WHERE id = $1 AND owner_id = $2 AND status = ANY($4)",
        )
        .bind(id)
        .bind(owner)
        .bind(to)
        .bind(from)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn studio_motion_queue(&self) -> Result<Vec<crate::models::StudioMotionQueueDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioMotionQueueDto>(
            "SELECT m.id, m.body, u.display_name AS author, m.updated_at
               FROM studio_motions m
               JOIN users u ON u.id = m.owner_id
              WHERE m.status = 'shown'
              ORDER BY m.updated_at",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_studio_motion_any(
        &self,
        id: Uuid,
    ) -> Result<Option<(crate::models::StudioMotionDto, Uuid)>> {
        let owner: Option<(Uuid,)> =
            sqlx::query_as("SELECT owner_id FROM studio_motions WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pg_pool)
                .await?;
        let Some((owner,)) = owner else {
            return Ok(None);
        };
        let one = sqlx::query_as::<_, crate::models::StudioMotionDto>(
            "SELECT id, body, status, keeper_word, approved_at, house_id, created_at, updated_at
               FROM studio_motions WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(Some((one, owner)))
    }

    pub async fn deny_studio_motion(&self, id: Uuid, word: &str) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_motions SET status = 'draft', keeper_word = $2, updated_at = NOW()
              WHERE id = $1 AND status = 'shown'",
        )
        .bind(id)
        .bind(word)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn mark_studio_motion_approved(&self, id: Uuid, house: &str) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_motions SET approved_at = NOW(), house_id = $2, updated_at = NOW()
              WHERE id = $1 AND approved_at IS NULL",
        )
        .bind(id)
        .bind(house)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    // ── Аукцион ─────────────────────────────────────────────────────────────

    /// Выставить вещь на торг. Вещь запирается лотом — тем же `locked_by`, что
    /// и прилавком: занятое занято, и знать, чем именно, дому незачем.
    pub async fn start_auction(
        &self,
        seller: Option<Uuid>,
        kind: &str,
        subject: Uuid,
        start_price: i32,
        currency: &str,
        ends_at: DateTime<Utc>,
        estate: bool,
    ) -> Result<Uuid> {
        let mut tx = self.pg_pool.begin().await?;
        let auction: Uuid = sqlx::query_scalar(
            "INSERT INTO auctions (kind, subject_id, seller_id, estate, start_price, currency, ends_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(kind)
        .bind(subject)
        .bind(seller)
        .bind(estate)
        .bind(start_price)
        .bind(currency)
        .bind(ends_at)
        .fetch_one(&mut *tx)
        .await?;

        // Вещь должна быть СВОЕЙ и свободной. У наследства хозяина нет вовсе —
        // тогда проверяется, что она и правда ничья.
        let lock = match (kind, seller.is_some()) {
            ("copy", true) => {
                "UPDATE card_copies SET locked_by = $3
                  WHERE id = $1 AND owner_id = $2 AND locked_by IS NULL"
            }
            ("copy", false) => {
                "UPDATE card_copies SET locked_by = $3
                  WHERE id = $1 AND owner_id IS NULL AND $2::uuid IS NULL AND locked_by IS NULL"
            }
            (_, true) => {
                "UPDATE frame_licenses SET locked_by = $3
                  WHERE id = $1 AND owner_id = $2 AND locked_by IS NULL"
            }
            (_, false) => {
                "UPDATE frame_licenses SET locked_by = $3
                  WHERE id = $1 AND owner_id IS NULL AND $2::uuid IS NULL AND locked_by IS NULL"
            }
        };
        let hit = sqlx::query(lock)
            .bind(subject)
            .bind(seller)
            .bind(auction)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if hit == 0 {
            tx.rollback().await?;
            return Err(AppError::BadRequest("notYours".into()));
        }
        tx.commit().await?;
        Ok(auction)
    }

    /// Что сейчас на молотке.
    pub async fn list_auctions(&self) -> Result<Vec<crate::models::AuctionDto>> {
        Ok(sqlx::query_as::<_, crate::models::AuctionDto>(
            "SELECT a.id, a.kind, a.subject_id, u.display_name AS seller, a.estate,
                    COALESCE(f.name, k.title_ru, '') AS name,
                    c.card_id,
                    a.start_price, a.currency,
                    (SELECT MAX(b.amount) FROM auction_bids b WHERE b.auction_id = a.id) AS top_bid,
                    (SELECT COUNT(*) FROM auction_bids b WHERE b.auction_id = a.id) AS bids,
                    a.ends_at
               FROM auctions a
               LEFT JOIN users u ON u.id = a.seller_id
               LEFT JOIN frame_licenses l ON l.id = a.subject_id AND a.kind = 'license'
               LEFT JOIN studio_frames f ON f.id = l.frame_id
               LEFT JOIN card_copies c ON c.id = a.subject_id AND a.kind = 'copy'
               LEFT JOIN battle_cards k ON k.id = c.card_id
              WHERE a.state = 'open'
              ORDER BY a.ends_at",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Поставить. Ставка НИЧЕГО не списывает: пока идёт торг, деньги у человека
    /// на руках. Проверяется только, что они у него есть, — обещание, за
    /// которое в момент закрытия спросят.
    ///
    /// Ставка в последние минуты продлевает торг на столько же: иначе всё
    /// решается в последнюю секунду, и выигрывает не тот, кто дал больше, а
    /// тот, у кого быстрее рука.
    pub async fn place_bid(
        &self,
        bidder: Uuid,
        auction: Uuid,
        amount: i32,
        step: i32,
        snipe_minutes: i64,
    ) -> Result<i32> {
        let mut tx = self.pg_pool.begin().await?;
        let lot: Option<(Option<Uuid>, i32, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT seller_id, start_price, currency, ends_at FROM auctions
              WHERE id = $1 AND state = 'open' FOR UPDATE",
        )
        .bind(auction)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((seller, start_price, currency, ends_at)) = lot else {
            tx.rollback().await?;
            return Err(AppError::BadRequest("gone".into()));
        };
        if ends_at <= Utc::now() {
            tx.rollback().await?;
            return Err(AppError::BadRequest("over".into()));
        }
        if seller == Some(bidder) {
            tx.rollback().await?;
            return Err(AppError::BadRequest("ownLot".into()));
        }

        let top: Option<i32> =
            sqlx::query_scalar("SELECT MAX(amount) FROM auction_bids WHERE auction_id = $1")
                .bind(auction)
                .fetch_one(&mut *tx)
                .await?;
        let least = match top {
            Some(top) => top + step,
            None => start_price,
        };
        if amount < least {
            tx.rollback().await?;
            return Err(AppError::BadRequest(format!("bidTooLow:{least}")));
        }

        let balance: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount)::bigint FROM battle_wallet_entries
              WHERE user_id = $1 AND currency = $2",
        )
        .bind(bidder)
        .bind(&currency)
        .fetch_one(&mut *tx)
        .await?;
        if balance.0.unwrap_or(0) < i64::from(amount) {
            tx.rollback().await?;
            return Err(AppError::BadRequest("notEnough".into()));
        }

        sqlx::query(
            "INSERT INTO auction_bids (auction_id, bidder_id, amount) VALUES ($1, $2, $3)",
        )
        .bind(auction)
        .bind(bidder)
        .bind(amount)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE auctions SET ends_at = GREATEST(ends_at, NOW() + ($2 || ' minutes')::interval)
              WHERE id = $1 AND ends_at < NOW() + ($2 || ' minutes')::interval",
        )
        .bind(auction)
        .bind(snipe_minutes.to_string())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(amount)
    }

    /// Торги, которым вышел срок.
    pub async fn auctions_due(&self) -> Result<Vec<Uuid>> {
        Ok(
            sqlx::query_scalar("SELECT id FROM auctions WHERE state = 'open' AND ends_at <= NOW()")
                .fetch_all(&self.pg_pool)
                .await?,
        )
    }

    /// Ударить молотком.
    ///
    /// Ставки перебираются сверху вниз, и лот достаётся ПЕРВОМУ, кто может
    /// заплатить прямо сейчас: ставка не списывалась, и за неделю деньги могли
    /// уйти на другое. Никто не может — лот пуст (`void`), и вещь возвращается
    /// хозяину отпертой.
    ///
    /// Вырученное у наследства СГОРАЕТ: платить некому, а дописать эту пыль
    /// кому-нибудь значило бы напечатать её из ниоткуда.
    pub async fn close_auction(
        &self,
        auction: Uuid,
        commission_percent: i32,
    ) -> Result<Option<(Uuid, i32)>> {
        let mut tx = self.pg_pool.begin().await?;
        let lot: Option<(String, Uuid, Option<Uuid>, String, bool)> = sqlx::query_as(
            "SELECT kind, subject_id, seller_id, currency, estate FROM auctions
              WHERE id = $1 AND state = 'open' FOR UPDATE",
        )
        .bind(auction)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((kind, subject, seller, currency, estate)) = lot else {
            tx.rollback().await?;
            return Ok(None);
        };

        let bids: Vec<(Uuid, i32)> = sqlx::query_as(
            "SELECT bidder_id, amount FROM auction_bids
              WHERE auction_id = $1 ORDER BY amount DESC, created_at",
        )
        .bind(auction)
        .fetch_all(&mut *tx)
        .await?;

        let mut winner: Option<(Uuid, i32)> = None;
        for (bidder, amount) in bids {
            let balance = self.battle_wallet_balance(bidder, &currency).await?;
            if balance >= i64::from(amount) {
                winner = Some((bidder, amount));
                break;
            }
        }

        match winner {
            Some((who, price)) => {
                sqlx::query(
                    "INSERT INTO battle_wallet_entries
                         (user_id, currency, amount, reason, ref_id, idem_key)
                     VALUES ($1, $2, $3, 'auction_won', $4, $5)
                     ON CONFLICT (user_id, idem_key) DO NOTHING",
                )
                .bind(who)
                .bind(&currency)
                .bind(-price)
                .bind(auction)
                .bind(format!("auction:{auction}:paid"))
                .execute(&mut *tx)
                .await?;

                // Продавцу — если он есть, и за вычетом комиссии дома: тот же
                // сток, что и у лавки, иначе аукцион был бы дырой в нём.
                // У наследства продавца нет, и вырученное сгорает целиком —
                // это тот же сток, что и доля дома.
                if let (Some(seller), false) = (seller, estate) {
                    let commission = (price * commission_percent) / 100;
                    let to_seller = price - commission;
                    sqlx::query(
                        "INSERT INTO battle_wallet_entries
                             (user_id, currency, amount, reason, ref_id, idem_key)
                         VALUES ($1, $2, $3, 'auction_sold', $4, $5)
                         ON CONFLICT (user_id, idem_key) DO NOTHING",
                    )
                    .bind(seller)
                    .bind(&currency)
                    .bind(to_seller)
                    .bind(auction)
                    .bind(format!("auction:{auction}:got"))
                    .execute(&mut *tx)
                    .await?;
                }

                let hand_over = if kind == "copy" {
                    "UPDATE card_copies SET owner_id = $2, origin = 'traded', locked_by = NULL,
                            acquired_at = NOW(), seen_at = NULL WHERE id = $1"
                } else {
                    "UPDATE frame_licenses SET owner_id = $2, origin = 'traded', locked_by = NULL,
                            acquired_at = NOW() WHERE id = $1"
                };
                sqlx::query(hand_over)
                    .bind(subject)
                    .bind(who)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query(
                    "UPDATE auctions SET state = 'done', winner_id = $2, final_price = $3,
                            closed_at = NOW() WHERE id = $1",
                )
                .bind(auction)
                .bind(who)
                .bind(price)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                Ok(Some((who, price)))
            }
            None => {
                // Никто не взял — вещь возвращается отпертой, а не остаётся
                // висеть за торгом, которого больше нет.
                let free = if kind == "copy" {
                    "UPDATE card_copies SET locked_by = NULL WHERE id = $1"
                } else {
                    "UPDATE frame_licenses SET locked_by = NULL WHERE id = $1"
                };
                sqlx::query(free)
                    .bind(subject)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query(
                    "UPDATE auctions SET state = 'void', closed_at = NOW() WHERE id = $1",
                )
                .bind(auction)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                Ok(None)
            }
        }
    }

    /// Вещи, оставшиеся без хозяина: учётную запись удалили, а вещи стоят.
    /// Ровно они и уходят с молотка.
    pub async fn ownerless_things(&self) -> Result<Vec<(String, Uuid)>> {
        let copies: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT c.id FROM card_copies c
              WHERE c.owner_id IS NULL AND c.locked_by IS NULL",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        let licences: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT l.id FROM frame_licenses l
              WHERE l.owner_id IS NULL AND l.locked_by IS NULL",
        )
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(copies
            .into_iter()
            .map(|(id,)| ("copy".to_string(), id))
            .chain(licences.into_iter().map(|(id,)| ("license".to_string(), id)))
            .collect())
    }

    // ── Уборка ──────────────────────────────────────────────────────────────

    /// Листы, из которых ничего не вырезали и которые пролежали дольше срока.
    ///
    /// Лист весит 2.6 МБ против 12 КБ у детали — это самый тяжёлый мусор в
    /// доме и единственный, который копится сам собой: человек принёс лист,
    /// посмотрел разрез, передумал и ушёл.
    pub async fn studio_sheets_to_sweep(&self, days: i32) -> Result<Vec<(Uuid, String)>> {
        Ok(sqlx::query_as(
            "SELECT id, url FROM studio_sheets
              WHERE harvested_at IS NULL
                AND created_at < NOW() - ($1 || ' days')::interval",
        )
        .bind(days.to_string())
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Детали, на которые не ссылается ни одна рама этого человека и которые
    /// пролежали дольше срока.
    ///
    /// Ссылка ищется в ТЕЛЕ рамы, а не по столбцу: деталь стоит в раме адресом,
    /// и второго учёта — «эта деталь занята» — заводить нельзя, он разойдётся с
    /// правдой на первой же правке.
    pub async fn studio_assets_to_sweep(&self, days: i32) -> Result<Vec<(Uuid, String)>> {
        Ok(sqlx::query_as(
            "SELECT a.id, a.url FROM studio_assets a
              WHERE a.created_at < NOW() - ($1 || ' days')::interval
                AND NOT EXISTS (
                      SELECT 1 FROM studio_frames f
                       WHERE f.owner_id = a.owner_id
                         AND f.body::text LIKE '%' || a.url || '%')",
        )
        .bind(days.to_string())
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn drop_studio_sheet(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM studio_sheets WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn drop_studio_asset(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM studio_assets WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn list_studio_frames(
        &self,
        owner: Uuid,
    ) -> Result<Vec<crate::models::StudioFrameDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioFrameDto>(
            "SELECT id, name, body, status, admitted_at, bytes, created_at, updated_at
               FROM studio_frames WHERE owner_id = $1 ORDER BY updated_at DESC",
        )
        .bind(owner)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_studio_frame(
        &self,
        owner: Uuid,
        id: Uuid,
    ) -> Result<Option<crate::models::StudioFrameDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioFrameDto>(
            "SELECT id, name, body, status, admitted_at, bytes, created_at, updated_at
               FROM studio_frames WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(owner)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn count_studio_frames_open(&self, owner: Uuid) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM studio_frames
              WHERE owner_id = $1 AND status IN ('draft', 'withdrawn')",
        )
        .bind(owner)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn insert_studio_frame(
        &self,
        owner: Uuid,
        name: &str,
        body: &serde_json::Value,
    ) -> Result<crate::models::StudioFrameDto> {
        Ok(sqlx::query_as::<_, crate::models::StudioFrameDto>(
            "INSERT INTO studio_frames (owner_id, name, body) VALUES ($1, $2, $3)
             RETURNING id, name, body, status, admitted_at, bytes, created_at, updated_at",
        )
        .bind(owner)
        .bind(name)
        .bind(body)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// Записать рамку. `status` в условии — забор на стороне базы: выложенное
    /// не правится, и это не должно держаться на одной проверке в службе.
    pub async fn update_studio_frame(
        &self,
        owner: Uuid,
        id: Uuid,
        name: &str,
        body: &serde_json::Value,
    ) -> Result<Option<crate::models::StudioFrameDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioFrameDto>(
            "UPDATE studio_frames SET name = $3, body = $4, updated_at = NOW()
              WHERE id = $1 AND owner_id = $2 AND status IN ('draft', 'withdrawn')
             RETURNING id, name, body, status, admitted_at, bytes, created_at, updated_at",
        )
        .bind(id)
        .bind(owner)
        .bind(name)
        .bind(body)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    pub async fn delete_studio_frame(&self, owner: Uuid, id: Uuid) -> Result<bool> {
        Ok(sqlx::query(
            // Утверждённую не выбрасывают: её уже носят чужие карты.
            "DELETE FROM studio_frames WHERE id = $1 AND owner_id = $2 AND approved_at IS NULL",
        )
        .bind(id)
        .bind(owner)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    // ── Свои карты ──────────────────────────────────────────────────────

    const STUDIO_CARD_SELECT: &'static str =
        "id, body, status, keeper_word, approved_at, card_id, created_at, updated_at";

    pub async fn list_studio_cards(
        &self,
        owner: Uuid,
    ) -> Result<Vec<crate::models::StudioCardDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioCardDto>(&format!(
            "SELECT {} FROM studio_cards WHERE owner_id = $1 ORDER BY updated_at DESC",
            Self::STUDIO_CARD_SELECT
        ))
        .bind(owner)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_studio_card(
        &self,
        owner: Uuid,
        id: Uuid,
    ) -> Result<Option<crate::models::StudioCardDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioCardDto>(&format!(
            "SELECT {} FROM studio_cards WHERE id = $1 AND owner_id = $2",
            Self::STUDIO_CARD_SELECT
        ))
        .bind(id)
        .bind(owner)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Работа со стороны хозяина: без владельца в условии, потому что смотрит
    /// он чужое. Возвращает ещё и чья она и на каком языке её писали — записка
    /// автору пишется на ЕГО языке, а не на том, что открыт у хозяина.
    pub async fn get_studio_card_any(&self, id: Uuid) -> Result<Option<StudioCardRow>> {
        Ok(sqlx::query_as::<_, StudioCardRow>(&format!(
            "SELECT {}, owner_id, lang FROM studio_cards WHERE id = $1",
            Self::STUDIO_CARD_SELECT
        ))
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Сколько работ у человека в руках — незаконченных. Взятые домом и
    /// отданные не в счёт: потолок про то, сколько можно НАЧАТЬ.
    pub async fn count_studio_cards_open(&self, owner: Uuid) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM studio_cards
              WHERE owner_id = $1 AND status IN ('draft', 'withdrawn')",
        )
        .bind(owner)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn insert_studio_card(
        &self,
        owner: Uuid,
        body: &serde_json::Value,
        lang: &str,
    ) -> Result<crate::models::StudioCardDto> {
        Ok(sqlx::query_as::<_, crate::models::StudioCardDto>(&format!(
            "INSERT INTO studio_cards (owner_id, body, lang) VALUES ($1, $2, $3)
             RETURNING {}",
            Self::STUDIO_CARD_SELECT
        ))
        .bind(owner)
        .bind(body)
        .bind(lang)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// `status` в условии — забор на стороне базы: отданное хозяину не
    /// правится, и это не должно держаться на одной проверке в службе.
    pub async fn update_studio_card(
        &self,
        owner: Uuid,
        id: Uuid,
        body: &serde_json::Value,
    ) -> Result<Option<crate::models::StudioCardDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioCardDto>(&format!(
            "UPDATE studio_cards SET body = $3, updated_at = NOW()
              WHERE id = $1 AND owner_id = $2 AND status IN ('draft', 'withdrawn')
             RETURNING {}",
            Self::STUDIO_CARD_SELECT
        ))
        .bind(id)
        .bind(owner)
        .bind(body)
        .fetch_optional(&self.pg_pool)
        .await?)
    }

    /// Перевести работу в другое состояние. `from` — те слова, из которых
    /// переход законен: без них «отдать» дважды сбросило бы отметку хозяина.
    pub async fn set_studio_card_status(
        &self,
        owner: Uuid,
        id: Uuid,
        to: &str,
        from: &[&str],
    ) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_cards SET status = $3, updated_at = NOW()
              WHERE id = $1 AND owner_id = $2 AND status = ANY($4)",
        )
        .bind(id)
        .bind(owner)
        .bind(to)
        .bind(from)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn delete_studio_card(&self, owner: Uuid, id: Uuid) -> Result<bool> {
        Ok(sqlx::query(
            // Утверждённую не выбрасывают: она уже стоит на полке дома, и
            // память о принесённом — часть её истории.
            "DELETE FROM studio_cards WHERE id = $1 AND owner_id = $2 AND approved_at IS NULL",
        )
        .bind(id)
        .bind(owner)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Очередь хозяина: отданное, самое старое сверху.
    ///
    /// ОДНА, а не два ящика: допуск у карт снят (`STUDIO-CARDS-REVIEW.md` П3).
    /// Он не значил ничего, кроме того, в каком ящике стола лежит работа, —
    /// допущенной карты не видел никто, и утвердить можно было мимо него.
    pub async fn studio_card_queue(&self) -> Result<Vec<crate::models::StudioCardQueueDto>> {
        Ok(sqlx::query_as::<_, crate::models::StudioCardQueueDto>(
            "SELECT c.id, c.body, c.status, c.approved_at,
                    u.display_name AS author, u.id AS author_id,
                    u.studio_slug AS author_slug, c.updated_at
               FROM studio_cards c
               JOIN users u ON u.id = c.owner_id
              WHERE c.status = 'shown'
              ORDER BY c.updated_at",
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Отказ. Работа возвращается автору ПРАВИМОЙ: отказ, который нельзя
    /// починить, — это не отказ, а тупик.
    pub async fn deny_studio_card(&self, id: Uuid, word: &str) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_cards
                SET status = 'draft', keeper_word = $2, updated_at = NOW()
              WHERE id = $1 AND status = 'shown'",
        )
        .bind(id)
        .bind(word)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Отметить утверждённой. Условие `approved_at IS NULL` — вся защита от
    /// второго нажатия: два «утвердить» не заведут двух карт.
    pub async fn mark_studio_card_approved(&self, id: Uuid, card: Uuid) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE studio_cards SET approved_at = NOW(), card_id = $2, updated_at = NOW()
              WHERE id = $1 AND approved_at IS NULL",
        )
        .bind(id)
        .bind(card)
        .execute(&self.pg_pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Первый экземпляр — автору, даром. Номер из того же тиража, что и у
    /// покупателей: второго счётчика у карты нет.
    pub async fn give_first_copy(&self, card: Uuid, owner: Uuid) -> Result<Option<i32>> {
        let minted: Option<(i32,)> = sqlx::query_as(
            "UPDATE battle_cards SET minted = minted + 1
              WHERE id = $1 AND (edition_size IS NULL OR minted < edition_size)
          RETURNING minted",
        )
        .bind(card)
        .fetch_optional(&self.pg_pool)
        .await?;
        let Some((serial,)) = minted else {
            return Ok(None);
        };
        sqlx::query(
            "INSERT INTO card_copies (card_id, owner_id, serial, level, origin)
             VALUES ($1, $2, $3, 1, 'prize')",
        )
        .bind(card)
        .bind(owner)
        .bind(serial)
        .execute(&self.pg_pool)
        .await?;
        Ok(Some(serial))
    }

    /// Библиотека дома: детали склада, открытые людям. Отдельной таблицы нет —
    /// это те же детали, просто с поднятым флагом.
    pub async fn list_public_battle_assets(
        &self,
        role: Option<&str>,
    ) -> Result<Vec<crate::models::BattleAssetListed>> {
        Ok(sqlx::query_as::<_, crate::models::BattleAssetListed>(
            "SELECT a.id, a.sheet_id, a.name, a.role, a.url, a.width, a.height,
                    a.sort_order, a.created_at, a.updated_at, a.public, NULL::text AS sheet_name
               FROM battle_assets a
              WHERE a.public AND ($1::text IS NULL OR a.role = $1)
              ORDER BY a.role, a.sort_order NULLS LAST, a.created_at DESC",
        )
        .bind(role)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn list_battle_assets(
        &self,
        sheet: crate::models::AssetSheetFilter,
        role: Option<&str>,
        query: Option<&str>,
    ) -> Result<Vec<crate::models::BattleAssetListed>> {
        let (one, loose) = match sheet {
            crate::models::AssetSheetFilter::Any => (None, false),
            crate::models::AssetSheetFilter::Loose => (None, true),
            crate::models::AssetSheetFilter::One(id) => (Some(id), false),
        };
        Ok(sqlx::query_as::<_, crate::models::BattleAssetListed>(
            "SELECT a.id, a.sheet_id, a.name, a.role, a.url, a.width, a.height,
                    a.sort_order, a.created_at, a.updated_at, a.public, s.name AS sheet_name
               FROM battle_assets a
               LEFT JOIN battle_asset_sheets s ON s.id = a.sheet_id
              WHERE ($1::uuid IS NULL OR a.sheet_id = $1)
                AND ($2::bool IS NOT TRUE OR a.sheet_id IS NULL)
                AND ($3::text IS NULL OR a.role = $3)
                AND ($4::text IS NULL OR a.name ILIKE '%' || $4 || '%')
              ORDER BY a.sort_order NULLS LAST, a.created_at DESC, a.id",
        )
        .bind(one)
        .bind(loose)
        .bind(role)
        .bind(query)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_battle_asset(&self, id: Uuid) -> Result<crate::models::BattleAsset> {
        sqlx::query_as::<_, crate::models::BattleAsset>(
            "SELECT * FROM battle_assets WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle asset {id} not found")))
    }

    /// Where the next part lands when it joins a sheet: after everything
    /// already there, so a second cut appends instead of shuffling.
    pub async fn next_battle_asset_order(&self, sheet_id: Option<Uuid>) -> Result<i32> {
        let row: (Option<i32>,) = sqlx::query_as(
            "SELECT MAX(sort_order) FROM battle_assets
              WHERE ($1::uuid IS NULL AND sheet_id IS NULL) OR sheet_id = $1",
        )
        .bind(sheet_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row.0.unwrap_or(-1) + 1)
    }

    pub async fn insert_battle_asset(
        &self,
        sheet_id: Option<Uuid>,
        name: &str,
        role: &str,
        url: &str,
        width: i32,
        height: i32,
        sort_order: i32,
    ) -> Result<crate::models::BattleAsset> {
        Ok(sqlx::query_as::<_, crate::models::BattleAsset>(
            "INSERT INTO battle_assets (sheet_id, name, role, url, width, height, sort_order)
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING *",
        )
        .bind(sheet_id)
        .bind(name)
        .bind(role)
        .bind(url)
        .bind(width)
        .bind(height)
        .bind(sort_order)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    /// COALESCE on every field: the desk edits one thing at a time, and a
    /// rename must not silently re-file the part under another role.
    pub async fn update_battle_asset(
        &self,
        id: Uuid,
        name: Option<&str>,
        role: Option<&str>,
        move_sheet: bool,
        sheet_id: Option<Uuid>,
        public: Option<bool>,
    ) -> Result<crate::models::BattleAsset> {
        sqlx::query_as::<_, crate::models::BattleAsset>(
            "UPDATE battle_assets
                SET name = COALESCE($2, name),
                    role = COALESCE($3, role),
                    sheet_id = CASE WHEN $4 THEN $5 ELSE sheet_id END,
                    public = COALESCE($6, public),
                    updated_at = NOW()
              WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(name)
        .bind(role)
        .bind(move_sheet)
        .bind(sheet_id)
        .bind(public)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle asset {id} not found")))
    }

    pub async fn delete_battle_asset(&self, id: Uuid) -> Result<crate::models::BattleAsset> {
        sqlx::query_as::<_, crate::models::BattleAsset>(
            "DELETE FROM battle_assets WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Battle asset {id} not found")))
    }

    pub async fn set_battle_asset_order(&self, ids: &[Uuid]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let orders: Vec<i32> = (0..ids.len() as i32).collect();
        let res = sqlx::query(
            "UPDATE battle_assets AS a SET sort_order = v.ord
               FROM (SELECT * FROM UNNEST($1::uuid[], $2::int[]) AS t(id, ord)) AS v
              WHERE a.id = v.id",
        )
        .bind(ids)
        .bind(&orders)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    pub async fn set_battle_race_order(&self, ids: &[Uuid]) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let orders: Vec<i32> = (0..ids.len() as i32).collect();
        let res = sqlx::query(
            "UPDATE battle_races AS r SET sort_order = v.ord
               FROM (SELECT * FROM UNNEST($1::uuid[], $2::int[]) AS t(id, ord)) AS v
              WHERE r.id = v.id",
        )
        .bind(ids)
        .bind(&orders)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }
}
