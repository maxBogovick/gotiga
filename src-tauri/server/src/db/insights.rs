//! Queries behind the "Behaviour" tab of the analytics panel and the daily
//! rollups that keep its numbers past the 30-day raw-event window.
//!
//! A child module of `db` on purpose: `Repository::pg_pool` and
//! `PATH_GROUP_SQL` are private to `db`, and the rollups must name rooms with
//! the very same `CASE` as the page-view rollup — written twice, the two would
//! drift apart on the first new room.

use super::{PATH_GROUP_SQL, Repository};
use crate::error::Result;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::{Postgres, Transaction};

/// `PATH_GROUP_SQL` reads the column `path`; the same room-naming applied to
/// `exit_to` (where the visitor went next) is that text with the column
/// swapped, so "went to the archive" and "was in the archive" are one word.
fn exit_group_sql() -> String {
    PATH_GROUP_SQL.replace("split_part(path,", "split_part(exit_to,")
}

// ── Rollups ──────────────────────────────────────────────────────────────

/// Recompute the signal rollups for `from..=to` inside the aggregate
/// transaction (delete + reinsert, like every other rollup there).
pub(super) async fn refresh_signal_rollups(
    tx: &mut Transaction<'_, Postgres>,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<()> {
    for table in [
        "site_engagement_daily",
        "site_exits_daily",
        "site_interactions_daily",
        "site_errors_daily",
        "site_vitals_daily",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE day BETWEEN $1 AND $2"))
            .bind(from)
            .bind(to)
            .execute(&mut **tx)
            .await?;
    }

    // Distribution of time and scroll per room. Buckets, not an average: an
    // average hides the two humps ("left at once" and "read it all").
    sqlx::query(&format!(
        r#"
        INSERT INTO site_engagement_daily (
            day, path_group, visits,
            dur_lt10, dur_10_30, dur_30_60, dur_60_180, dur_ge180,
            scroll_lt25, scroll_25_49, scroll_50_74, scroll_ge75,
            new_visits, returning_visits, new_long, returning_long, updated_at
        )
        SELECT
            event_date,
            {pg},
            COUNT(*)::int,
            COUNT(*) FILTER (WHERE duration_ms < 10000)::int,
            COUNT(*) FILTER (WHERE duration_ms >= 10000 AND duration_ms < 30000)::int,
            COUNT(*) FILTER (WHERE duration_ms >= 30000 AND duration_ms < 60000)::int,
            COUNT(*) FILTER (WHERE duration_ms >= 60000 AND duration_ms < 180000)::int,
            COUNT(*) FILTER (WHERE duration_ms >= 180000)::int,
            COUNT(*) FILTER (WHERE scroll_depth < 25)::int,
            COUNT(*) FILTER (WHERE scroll_depth >= 25 AND scroll_depth < 50)::int,
            COUNT(*) FILTER (WHERE scroll_depth >= 50 AND scroll_depth < 75)::int,
            COUNT(*) FILTER (WHERE scroll_depth >= 75)::int,
            COUNT(*) FILTER (WHERE is_returning IS FALSE)::int,
            COUNT(*) FILTER (WHERE is_returning IS TRUE)::int,
            COUNT(*) FILTER (WHERE is_returning IS FALSE AND duration_ms >= 30000)::int,
            COUNT(*) FILTER (WHERE is_returning IS TRUE AND duration_ms >= 30000)::int,
            NOW()
        FROM figurine_analytics_events
        WHERE event_date BETWEEN $1 AND $2
          AND event_type = 'page_engaged'
          AND figurine_id IS NULL
        GROUP BY event_date, 2
        "#,
        pg = PATH_GROUP_SQL,
    ))
    .bind(from)
    .bind(to)
    .execute(&mut **tx)
    .await?;

    // Where visits end: the next room, another site, or the tab going away.
    sqlx::query(&format!(
        r#"
        INSERT INTO site_exits_daily (day, path_group, exit_to, visits, updated_at)
        SELECT
            event_date,
            {pg},
            CASE WHEN exit_to LIKE '/%' THEN 'to:' || ({xg}) ELSE exit_to END,
            COUNT(*)::int,
            NOW()
        FROM figurine_analytics_events
        WHERE event_date BETWEEN $1 AND $2
          AND event_type = 'page_engaged'
          AND figurine_id IS NULL
          AND exit_to IS NOT NULL
        GROUP BY event_date, 2, 3
        "#,
        pg = PATH_GROUP_SQL,
        xg = exit_group_sql(),
    ))
    .bind(from)
    .bind(to)
    .execute(&mut **tx)
    .await?;

    // Acts. Capped per day (the biggest groups win): `detail` is free text from
    // a browser, and the table must not grow a row per typed letter.
    sqlx::query(
        r#"
        INSERT INTO site_interactions_daily (
            day, name, detail, events, visitors, zero_events, value_sum, updated_at
        )
        SELECT day, name, detail, events, visitors, zero_events, value_sum, NOW()
        FROM (
            SELECT
                event_date AS day,
                name,
                COALESCE(detail, '') AS detail,
                COUNT(*)::int AS events,
                COUNT(DISTINCT visitor_hash) FILTER (WHERE visitor_hash IS NOT NULL)::int AS visitors,
                COUNT(*) FILTER (WHERE value = 0)::int AS zero_events,
                COALESCE(SUM(value), 0)::bigint AS value_sum,
                ROW_NUMBER() OVER (
                    PARTITION BY event_date
                    ORDER BY COUNT(*) DESC, name, COALESCE(detail, '')
                ) AS rn
            FROM figurine_analytics_events
            WHERE event_date BETWEEN $1 AND $2
              AND event_type = 'interaction'
              AND name IS NOT NULL
            GROUP BY event_date, name, COALESCE(detail, '')
        ) t
        WHERE rn <= 2000
        "#,
    )
    .bind(from)
    .bind(to)
    .execute(&mut **tx)
    .await?;

    // Breakage: script errors, failed images and API calls, and dead links.
    // A dead link's detail is the address that was asked for.
    sqlx::query(&format!(
        r#"
        INSERT INTO site_errors_daily (
            day, kind, path_group, detail, events, visitors, updated_at
        )
        SELECT day, kind, path_group, detail, events, visitors, NOW()
        FROM (
            SELECT
                event_date AS day,
                CASE WHEN event_type = 'not_found' THEN 'not_found' ELSE name END AS kind,
                {pg} AS path_group,
                CASE WHEN event_type = 'not_found'
                     THEN LEFT(split_part(path, '?', 1), 160)
                     ELSE COALESCE(detail, '') END AS detail,
                COUNT(*)::int AS events,
                COUNT(DISTINCT visitor_hash) FILTER (WHERE visitor_hash IS NOT NULL)::int AS visitors,
                ROW_NUMBER() OVER (PARTITION BY event_date ORDER BY COUNT(*) DESC) AS rn
            FROM figurine_analytics_events
            WHERE event_date BETWEEN $1 AND $2
              AND event_type IN ('client_error', 'not_found')
            GROUP BY 1, 2, 3, 4
        ) t
        WHERE kind IS NOT NULL AND rn <= 1000
        "#,
        pg = PATH_GROUP_SQL,
    ))
    .bind(from)
    .bind(to)
    .execute(&mut **tx)
    .await?;

    // Speed, as the visitor's device saw it. 75th percentile because that is
    // the figure Core Web Vitals are judged by; "good" counts use the same
    // published thresholds (LCP 2.5 s, INP 200 ms, CLS 0.1).
    sqlx::query(&format!(
        r#"
        INSERT INTO site_vitals_daily (
            day, path_group, device_class, samples,
            lcp_n, lcp_p75, lcp_good,
            inp_n, inp_p75, inp_good,
            cls_n, cls_p75, cls_good,
            ttfb_p75, updated_at
        )
        SELECT
            event_date,
            {pg},
            COALESCE(device_class, 'unknown'),
            COUNT(*)::int,
            COUNT(vital_lcp)::int,
            (percentile_cont(0.75) WITHIN GROUP (ORDER BY vital_lcp)
                FILTER (WHERE vital_lcp IS NOT NULL))::int,
            COUNT(*) FILTER (WHERE vital_lcp <= 2500)::int,
            COUNT(vital_inp)::int,
            (percentile_cont(0.75) WITHIN GROUP (ORDER BY vital_inp)
                FILTER (WHERE vital_inp IS NOT NULL))::int,
            COUNT(*) FILTER (WHERE vital_inp <= 200)::int,
            COUNT(vital_cls)::int,
            (percentile_cont(0.75) WITHIN GROUP (ORDER BY vital_cls)
                FILTER (WHERE vital_cls IS NOT NULL))::int,
            COUNT(*) FILTER (WHERE vital_cls <= 100)::int,
            (percentile_cont(0.75) WITHIN GROUP (ORDER BY vital_ttfb)
                FILTER (WHERE vital_ttfb IS NOT NULL))::int,
            NOW()
        FROM figurine_analytics_events
        WHERE event_date BETWEEN $1 AND $2
          AND event_type = 'web_vitals'
        GROUP BY event_date, 2, 3
        "#,
        pg = PATH_GROUP_SQL,
    ))
    .bind(from)
    .bind(to)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

// ── Reads ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngagementShape {
    pub path_group: String,
    pub visits: i64,
    pub dur_lt10: i64,
    pub dur_10_30: i64,
    pub dur_30_60: i64,
    pub dur_60_180: i64,
    pub dur_ge180: i64,
    pub scroll_lt25: i64,
    pub scroll_25_49: i64,
    pub scroll_50_74: i64,
    pub scroll_ge75: i64,
    pub new_visits: i64,
    pub returning_visits: i64,
    pub new_long: i64,
    pub returning_long: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitRow {
    pub path_group: String,
    pub exit_to: String,
    pub visits: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionRow {
    pub path_group: String,
    pub section: String,
    pub visits: i64,
    pub median_active_ms: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VitalsRow {
    pub path_group: String,
    pub device_class: String,
    pub samples: i64,
    pub lcp_n: i64,
    pub lcp_p75: Option<f64>,
    pub lcp_good: i64,
    pub inp_n: i64,
    pub inp_p75: Option<f64>,
    pub inp_good: i64,
    pub cls_n: i64,
    pub cls_p75: Option<f64>,
    pub cls_good: i64,
    pub ttfb_p75: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorRow {
    pub kind: String,
    pub path_group: String,
    pub detail: String,
    pub events: i64,
    pub visitors: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadLinkSource {
    pub path: String,
    /// The room the visitor came from, or the external host when they arrived
    /// from outside; `direct` when neither is known.
    pub came_from: String,
    pub events: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionRow {
    pub name: String,
    pub detail: String,
    pub events: i64,
    pub visitors: i64,
    pub zero_events: i64,
    pub value_sum: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaleReadRow {
    pub slug: String,
    /// `scene:on` / `scene:off` / empty — the arm of the running experiment.
    pub variant: String,
    pub opened: i64,
    pub engaged: i64,
    pub quick_exits: i64,
    pub finished: i64,
    pub median_active_ms: Option<f64>,
    pub median_scroll: Option<f64>,
    /// Visits by a browser that had been here on an earlier day.
    pub came_back: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleFunnelRow {
    pub slug: Option<String>,
    pub title: Option<String>,
    pub started: i64,
    pub players: i64,
    pub finished: i64,
    pub won: i64,
    /// Started more than a day ago and never finished.
    pub abandoned: i64,
    /// How many actions the abandoned games had reached when they stopped.
    pub avg_actions_when_abandoned: Option<f64>,
    pub avg_rounds: Option<f64>,
    pub avg_minutes: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataTrust {
    pub page_views: i64,
    pub page_engaged: i64,
    pub figurine_views: i64,
    pub figurine_engaged: i64,
    pub web_vitals: i64,
    pub returning_known: i64,
    pub returning_total: i64,
    pub errors: i64,
}

impl Repository {
    /// Run the signal rollups on their own (the backfill path reuses the
    /// aggregate transaction; this is for a standalone re-run).
    pub async fn refresh_signal_rollups_only(&self, from: NaiveDate, to: NaiveDate) -> Result<()> {
        let mut tx = self.pg_pool.begin().await?;
        refresh_signal_rollups(&mut tx, from, to).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_engagement_shape(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<EngagementShape>> {
        Ok(sqlx::query_as::<_, EngagementShape>(
            r#"
            SELECT
                path_group,
                SUM(visits)::bigint AS visits,
                SUM(dur_lt10)::bigint AS dur_lt10,
                SUM(dur_10_30)::bigint AS dur_10_30,
                SUM(dur_30_60)::bigint AS dur_30_60,
                SUM(dur_60_180)::bigint AS dur_60_180,
                SUM(dur_ge180)::bigint AS dur_ge180,
                SUM(scroll_lt25)::bigint AS scroll_lt25,
                SUM(scroll_25_49)::bigint AS scroll_25_49,
                SUM(scroll_50_74)::bigint AS scroll_50_74,
                SUM(scroll_ge75)::bigint AS scroll_ge75,
                SUM(new_visits)::bigint AS new_visits,
                SUM(returning_visits)::bigint AS returning_visits,
                SUM(new_long)::bigint AS new_long,
                SUM(returning_long)::bigint AS returning_long
            FROM site_engagement_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY path_group
            ORDER BY visits DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// The five commonest ways out of each room.
    pub async fn get_exits(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<ExitRow>> {
        Ok(sqlx::query_as::<_, ExitRow>(
            r#"
            SELECT path_group, exit_to, visits FROM (
                SELECT
                    path_group,
                    exit_to,
                    SUM(visits)::bigint AS visits,
                    ROW_NUMBER() OVER (
                        PARTITION BY path_group ORDER BY SUM(visits) DESC, exit_to
                    ) AS rn
                FROM site_exits_daily
                WHERE day BETWEEN $1 AND $2
                GROUP BY path_group, exit_to
            ) t
            WHERE rn <= 5
            ORDER BY path_group, visits DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// The last heading reached before the visit ended. Raw events only (the
    /// headings are free text), so the caller clamps `from` to the retention
    /// floor.
    pub async fn get_last_sections(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<SectionRow>> {
        Ok(sqlx::query_as::<_, SectionRow>(&format!(
            r#"
            SELECT path_group, section, visits, median_active_ms FROM (
                SELECT
                    path_group,
                    last_section AS section,
                    COUNT(*)::bigint AS visits,
                    percentile_cont(0.5) WITHIN GROUP (ORDER BY duration_ms)
                        FILTER (WHERE duration_ms IS NOT NULL) AS median_active_ms,
                    ROW_NUMBER() OVER (
                        PARTITION BY path_group ORDER BY COUNT(*) DESC, last_section
                    ) AS rn
                FROM (
                    SELECT {pg} AS path_group, last_section, duration_ms
                    FROM figurine_analytics_events
                    WHERE event_date BETWEEN $1 AND $2
                      AND event_type = 'page_engaged'
                      AND figurine_id IS NULL
                      AND last_section IS NOT NULL
                ) e
                GROUP BY path_group, last_section
            ) t
            WHERE rn <= 6
            ORDER BY path_group, visits DESC
            "#,
            pg = PATH_GROUP_SQL,
        ))
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_vitals(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<VitalsRow>> {
        // Daily 75th percentiles are combined as a sample-weighted mean: not the
        // exact percentile of the whole range, but close, and it keeps the
        // figures past the raw-event window.
        Ok(sqlx::query_as::<_, VitalsRow>(
            r#"
            SELECT
                path_group,
                device_class,
                SUM(samples)::bigint AS samples,
                SUM(lcp_n)::bigint AS lcp_n,
                SUM(lcp_p75::float8 * lcp_n)
                    / NULLIF(SUM(lcp_n) FILTER (WHERE lcp_p75 IS NOT NULL), 0) AS lcp_p75,
                SUM(lcp_good)::bigint AS lcp_good,
                SUM(inp_n)::bigint AS inp_n,
                SUM(inp_p75::float8 * inp_n)
                    / NULLIF(SUM(inp_n) FILTER (WHERE inp_p75 IS NOT NULL), 0) AS inp_p75,
                SUM(inp_good)::bigint AS inp_good,
                SUM(cls_n)::bigint AS cls_n,
                SUM(cls_p75::float8 * cls_n)
                    / NULLIF(SUM(cls_n) FILTER (WHERE cls_p75 IS NOT NULL), 0) AS cls_p75,
                SUM(cls_good)::bigint AS cls_good,
                SUM(ttfb_p75::float8 * samples)
                    / NULLIF(SUM(samples) FILTER (WHERE ttfb_p75 IS NOT NULL), 0) AS ttfb_p75
            FROM site_vitals_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY path_group, device_class
            ORDER BY samples DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn get_errors(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<ErrorRow>> {
        Ok(sqlx::query_as::<_, ErrorRow>(
            r#"
            SELECT kind, path_group, detail,
                   SUM(events)::bigint AS events,
                   SUM(visitors)::bigint AS visitors
            FROM site_errors_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY kind, path_group, detail
            ORDER BY events DESC
            LIMIT 60
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Where the visitors who hit a dead address came from. Raw events: the
    /// referrer is the whole point, and it is not kept in the rollup.
    pub async fn get_dead_link_sources(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DeadLinkSource>> {
        Ok(sqlx::query_as::<_, DeadLinkSource>(
            r#"
            SELECT
                LEFT(split_part(path, '?', 1), 160) AS path,
                COALESCE(referrer_host, detail, 'direct') AS came_from,
                COUNT(*)::bigint AS events
            FROM figurine_analytics_events
            WHERE event_date BETWEEN $1 AND $2
              AND event_type = 'not_found'
            GROUP BY 1, 2
            ORDER BY events DESC
            LIMIT 30
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Per name, the forty biggest details. Past the cap the long tail of
    /// free-text queries says nothing and only inflates the payload.
    pub async fn get_interactions(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<InteractionRow>> {
        Ok(sqlx::query_as::<_, InteractionRow>(
            r#"
            SELECT name, detail, events, visitors, zero_events, value_sum FROM (
                SELECT
                    name,
                    detail,
                    SUM(events)::bigint AS events,
                    SUM(visitors)::bigint AS visitors,
                    SUM(zero_events)::bigint AS zero_events,
                    SUM(value_sum)::bigint AS value_sum,
                    ROW_NUMBER() OVER (
                        PARTITION BY name ORDER BY SUM(events) DESC, detail
                    ) AS rn
                FROM site_interactions_daily
                WHERE day BETWEEN $1 AND $2
                GROUP BY name, detail
            ) t
            WHERE rn <= 40
            ORDER BY name, events DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Reading, tale by tale and arm by arm, from raw events. `finished` is the
    /// `tale_finished` act (the last line reached the screen); `opened` is the
    /// page view; time is foreground time.
    pub async fn get_tale_reading(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<TaleReadRow>> {
        Ok(sqlx::query_as::<_, TaleReadRow>(
            r#"
            WITH visits AS (
                SELECT
                    page_view_id,
                    regexp_replace(split_part(path, '?', 1), '^/tales/', '') AS slug,
                    COALESCE(MAX(variant), '') AS variant,
                    MAX(duration_ms) FILTER (WHERE event_type = 'page_engaged') AS active_ms,
                    MAX(scroll_depth) FILTER (WHERE event_type = 'page_engaged') AS scroll,
                    BOOL_OR(event_type = 'page_engaged') AS engaged,
                    BOOL_OR(event_type = 'interaction' AND name = 'tale_finished') AS finished,
                    BOOL_OR(is_returning IS TRUE) AS came_back
                FROM figurine_analytics_events
                WHERE event_date BETWEEN $1 AND $2
                  AND page_view_id IS NOT NULL
                  AND split_part(path, '?', 1) LIKE '/tales/%'
                  AND event_type IN ('page_view', 'page_engaged', 'interaction')
                GROUP BY page_view_id, 2
            )
            SELECT
                slug,
                variant,
                COUNT(*)::bigint AS opened,
                COUNT(*) FILTER (WHERE engaged)::bigint AS engaged,
                COUNT(*) FILTER (WHERE engaged AND active_ms < 10000)::bigint AS quick_exits,
                COUNT(*) FILTER (WHERE finished)::bigint AS finished,
                percentile_cont(0.5) WITHIN GROUP (ORDER BY active_ms)
                    FILTER (WHERE active_ms IS NOT NULL) AS median_active_ms,
                percentile_cont(0.5) WITHIN GROUP (ORDER BY scroll)
                    FILTER (WHERE scroll IS NOT NULL) AS median_scroll,
                COUNT(*) FILTER (WHERE came_back)::bigint AS came_back
            FROM visits
            WHERE slug <> ''
            GROUP BY slug, variant
            ORDER BY opened DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// The games, from the games themselves: the action log already is the
    /// record of where people stop, so nothing new is tracked for this.
    pub async fn get_battle_funnel(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<BattleFunnelRow>> {
        Ok(sqlx::query_as::<_, BattleFunnelRow>(
            r#"
            SELECT
                c.slug,
                c.title_en AS title,
                COUNT(m.id)::bigint AS started,
                COUNT(DISTINCT m.user_id)::bigint AS players,
                COUNT(m.id) FILTER (WHERE m.outcome IS NOT NULL)::bigint AS finished,
                COUNT(m.id) FILTER (WHERE m.outcome = 'player')::bigint AS won,
                COUNT(m.id) FILTER (
                    WHERE m.outcome IS NULL AND m.created_at < NOW() - INTERVAL '1 day'
                )::bigint AS abandoned,
                (AVG(m.seq) FILTER (
                    WHERE m.outcome IS NULL AND m.created_at < NOW() - INTERVAL '1 day'
                ))::float8 AS avg_actions_when_abandoned,
                (AVG(m.rounds) FILTER (WHERE m.outcome IS NOT NULL))::float8 AS avg_rounds,
                (AVG(EXTRACT(EPOCH FROM (m.finished_at - m.created_at)) / 60.0)
                    FILTER (WHERE m.finished_at IS NOT NULL))::float8 AS avg_minutes
            FROM battle_matches m
            LEFT JOIN battle_challenges c ON c.id = m.challenge_id
            WHERE m.created_at::date BETWEEN $1 AND $2
            GROUP BY c.id, c.slug, c.title_en
            ORDER BY started DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Distinct signed-in people who came through the battles door in range.
    pub async fn count_battle_visitors(&self, from: NaiveDate, to: NaiveDate) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT user_id)::bigint FROM battle_visits WHERE day BETWEEN $1 AND $2",
        )
        .bind(from)
        .bind(to)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(row.0)
    }

    pub async fn get_data_trust(&self, from: NaiveDate, to: NaiveDate) -> Result<DataTrust> {
        Ok(sqlx::query_as::<_, DataTrust>(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE event_type = 'page_view')::bigint AS page_views,
                COUNT(*) FILTER (WHERE event_type = 'page_engaged')::bigint AS page_engaged,
                COUNT(*) FILTER (WHERE event_type = 'figurine_view')::bigint AS figurine_views,
                COUNT(*) FILTER (WHERE event_type = 'figurine_engaged')::bigint AS figurine_engaged,
                COUNT(*) FILTER (WHERE event_type = 'web_vitals')::bigint AS web_vitals,
                COUNT(*) FILTER (
                    WHERE event_type IN ('page_view', 'figurine_view') AND is_returning IS NOT NULL
                )::bigint AS returning_known,
                COUNT(*) FILTER (
                    WHERE event_type IN ('page_view', 'figurine_view')
                )::bigint AS returning_total,
                COUNT(*) FILTER (WHERE event_type IN ('client_error', 'not_found'))::bigint AS errors
            FROM figurine_analytics_events
            WHERE event_date BETWEEN $1 AND $2
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_one(&self.pg_pool)
        .await?)
    }

    pub async fn leaf_title(&self, id: uuid::Uuid) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT title_en FROM gazette_leaves WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pg_pool)
                .await?;
        Ok(row.map(|r| r.0))
    }

    /// Add a note to the trend chart on its own. Idempotent per (day, label):
    /// a restart that re-announces the same release must not stack markers.
    pub async fn add_auto_annotation(&self, day: NaiveDate, label: &str) -> Result<bool> {
        let res = sqlx::query(
            "INSERT INTO analytics_annotations (day, label, auto) VALUES ($1, $2, TRUE)
             ON CONFLICT (day, label) WHERE auto DO NOTHING",
        )
        .bind(day)
        .bind(label)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }
}
