use crate::config::Config;
use crate::db::Repository;
use crate::error::{AppError, Result};
use crate::models::*;
use crate::observability::ObservabilityState;
use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use reqwest::Client;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

type RateLimiter = Arc<Mutex<HashMap<String, Vec<Instant>>>>;

#[derive(Clone)]
pub struct AppService {
    repo: Repository,
    config: Config,
    comment_rate_limiter: RateLimiter,
    commission_rate_limiter: RateLimiter,
    /// Shared limiter for assorted public endpoints (auth, bookings, orders,
    /// waitlist, token lookups), keyed by "bucket|ip".
    general_rate_limiter: RateLimiter,
    /// Offline IP → country/city resolver (best-effort; may be a no-op).
    geoip: Arc<crate::geo::GeoIp>,
    observability: ObservabilityState,
    analytics: crate::analytics::AnalyticsRuntime,
    /// Cached "house favorite" / "noticed" tiers. Computing them is an aggregating JOIN
    /// over every mark in the collection plus a percentile cut — and every figurine LIST
    /// (the home page, the archive, every filter change) needs them, so it ran on
    /// essentially every public read. The value is a percentile over the WHOLE collection:
    /// it moves slowly by construction, and a minute of staleness is invisible.
    favorite_tiers_cache: Arc<Mutex<Option<(Instant, crate::db::FavoriteTiers)>>>,
    /// One reused HTTP client for all outbound notifications (Telegram). Reqwest's
    /// Client holds a connection pool and is meant to be created once and cloned —
    /// building a fresh one per notification threw away connection/TLS reuse. It
    /// also carries a request timeout so a hung endpoint can't stall a caller.
    http_client: Client,
}

/// Widest rate-limit window in use anywhere; the GC keeps timestamps at least this
/// fresh, so a swept key can never wrongly reject (its data was already expired for
/// every bucket).
const RATE_LIMIT_WIDEST_WINDOW: Duration = Duration::from_secs(3600);

/// How long a computed set of favorite tiers stays good.
const FAVORITE_TIERS_TTL: Duration = Duration::from_secs(60);

impl AppService {
    pub fn new(repo: Repository, config: Config) -> Self {
        let geoip = Arc::new(crate::geo::GeoIp::open(config.geoip_db_path.as_deref()));
        let analytics = crate::analytics::AnalyticsRuntime::new(repo.clone());
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            repo,
            config,
            comment_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            commission_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            general_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            geoip,
            observability: ObservabilityState::default(),
            analytics,
            favorite_tiers_cache: Arc::new(Mutex::new(None)),
            http_client,
        }
    }

    pub fn observability(&self) -> ObservabilityState {
        self.observability.clone()
    }

    pub fn analytics(&self) -> crate::analytics::AnalyticsRuntime {
        self.analytics.clone()
    }

    pub async fn shutdown_analytics(&self) {
        self.analytics.shutdown().await;
    }

    fn log_domain_event(
        event: &'static str,
        entity: &'static str,
        entity_id: impl std::fmt::Display,
        outcome: &'static str,
    ) {
        tracing::info!(
            target: "gotiga_server::domain",
            event,
            entity,
            entity_id = %entity_id,
            outcome,
            "domain event"
        );
    }

    /// Delete login attempts older than the retention window (privacy / GDPR).
    pub async fn prune_login_history(&self, retention_days: i64) -> Result<u64> {
        self.repo.prune_old_login_attempts(retention_days).await
    }

    /// Assemble request metadata for storage, resolving geolocation from the IP.
    fn client_context(
        &self,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> crate::models::ClientContext {
        let (country_code, city) = ip
            .as_deref()
            .map(|i| self.geoip.lookup(i))
            .unwrap_or((None, None));
        crate::models::ClientContext {
            ip,
            user_agent,
            country_code,
            city,
        }
    }

    /// Generic in-memory rate limit: at most `max` requests per `window_secs`
    /// for a given (bucket, ip). Best-effort (in-process, resets on restart) —
    /// a coarse abuse brake, not a hard quota.
    pub async fn check_rate_limit(
        &self,
        bucket: &str,
        ip: &str,
        max: usize,
        window_secs: u64,
    ) -> Result<()> {
        let now = Instant::now();
        let window = Duration::from_secs(window_secs);
        let key = format!("{bucket}|{ip}");
        let mut map = self.general_rate_limiter.lock().await;
        let entry = map.entry(key).or_default();
        entry.retain(|t: &Instant| now.duration_since(*t) < window);
        if entry.len() >= max {
            return Err(AppError::BadRequest(
                "Too many requests from this address. Please wait and try again.".into(),
            ));
        }
        entry.push(now);
        Ok(())
    }

    pub async fn check_commission_rate_limit(&self, ip: &str) -> Result<()> {
        const MAX_PER_HOUR: usize = 6;
        let now = Instant::now();
        let cutoff_secs = Duration::from_secs(3600);
        let mut map = self.commission_rate_limiter.lock().await;
        let entry = map.entry(ip.to_string()).or_default();
        entry.retain(|t: &Instant| now.duration_since(*t) < cutoff_secs);
        if entry.len() >= MAX_PER_HOUR {
            return Err(AppError::BadRequest(
                "Too many requests from this address. Please wait before submitting again.".into(),
            ));
        }
        entry.push(now);
        Ok(())
    }

    /// Periodic sweep so the in-memory limiters can't grow without bound. The
    /// per-request `check_*` paths only prune the single key they touch, so a key
    /// for an IP that never returns would otherwise live forever. This drops stale
    /// timestamps across every key and removes the now-empty ones. Safe to run on
    /// any interval — it only ever removes already-expired data. Spawned on a timer
    /// from main.rs.
    pub async fn prune_rate_limiters(&self) -> usize {
        let now = Instant::now();
        let mut removed = 0usize;
        for limiter in [
            &self.comment_rate_limiter,
            &self.commission_rate_limiter,
            &self.general_rate_limiter,
        ] {
            let mut map = limiter.lock().await;
            let before = map.len();
            map.retain(|_, times| {
                times.retain(|t| now.duration_since(*t) < RATE_LIMIT_WIDEST_WINDOW);
                !times.is_empty()
            });
            removed += before - map.len();
        }
        removed
    }

    pub async fn initialize(&self) -> Result<()> {
        Ok(()) // Postgres is always ready, no pool to load
    }

    pub async fn health_check(&self) -> Result<()> {
        self.repo.health_check().await
    }

    // === FIGURINE ANALYTICS ===

    pub async fn enqueue_analytics_event(
        &self,
        mut req: AnalyticsEventRequest,
        headers: &axum::http::HeaderMap,
    ) -> Result<()> {
        // A detail-page view fires with the URL handle, which may be a slug. Resolve it
        // to the canonical UUID so slug- and UUID-visits of the same work aggregate onto
        // one figurine_id. Only non-empty, non-UUID handles hit the DB; UUIDs, absent
        // figurineId (site-wide page_view events), and empty strings pass through
        // untouched (build_event_record/validate_event keep their existing checks).
        if let Some(handle) = req.figurine_id.as_deref().filter(|s| !s.is_empty())
            && Uuid::parse_str(handle).is_err()
        {
            req.figurine_id = Some(self.resolve_figurine_uuid(handle).await?.to_string());
        }

        let request_context = self.client_context(
            crate::api::handlers::client_ip_from_headers(headers),
            crate::api::handlers::extract_user_agent_from_headers(headers),
        );
        let ctx = crate::analytics::AnalyticsRequestContext {
            headers,
            admin_api_key: &self.config.admin_api_key,
            hash_secret: &self.config.analytics_hash_secret,
            country_code: request_context.country_code,
            site_host: crate::analytics::host_of(&self.config.public_url),
        };
        let Some(event) = crate::analytics::build_event_record(req, ctx)? else {
            return Ok(());
        };
        if !self.analytics.try_enqueue(event).await {
            tracing::warn!(
                dropped_total = self.analytics.dropped_total(),
                "analytics event dropped"
            );
        }
        Ok(())
    }

    /// Delegates to AnalyticsRuntime's shared gate — the background
    /// aggregate_loop tick and every admin HTTP request calling this all
    /// serialize through the same lock, so they can't race each other into a
    /// concurrent DELETE+re-INSERT on figurine_analytics_daily (see the
    /// comment on AnalyticsRuntime::refresh_gate for the incident this fixes).
    pub async fn refresh_analytics_hot_window_if_due(&self) -> Result<()> {
        self.analytics.refresh_hot_window_if_due(&self.repo).await
    }

    pub async fn prune_analytics_events_chunked(
        &self,
        retention_days: i64,
        batch_size: i64,
    ) -> Result<u64> {
        self.repo
            .prune_old_analytics_events_chunked(retention_days, batch_size)
            .await
    }

    pub async fn admin_list_figurine_analytics(
        &self,
        query: AdminAnalyticsQuery,
    ) -> Result<AdminFigurineAnalyticsListPage> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let (prev_from, prev_to) = Self::previous_period(from, to);
        let sort = query.sort.as_deref().unwrap_or("views");
        let dir = query.dir.as_deref().unwrap_or("desc");
        let mut items = self
            .repo
            .get_admin_figurine_analytics_list(from, to, sort, dir)
            .await?;

        let growth = self.repo.get_admin_growth_window(to).await?;
        let sparklines = self.repo.get_figurine_sparklines(to).await?;

        for item in &mut items {
            if let Some(face) = item.face_url.clone() {
                item.face_url = Some(self.resolve_url(&face, "images_thumb", &item.figurine_id));
            }
            let Ok(id) = Self::parse_uuid(&item.figurine_id) else {
                continue;
            };
            let (last7, prior7) = growth.get(&id).copied().unwrap_or((0, 0));
            item.is_growing = Self::is_growing(last7, prior7);
            item.signal = Self::analytics_signal(
                item.views,
                item.engaged_views,
                item.cta_clicks,
                item.submissions,
                item.conversion_rate,
                item.is_growing,
            );
            item.sparkline = Self::zero_filled_sparkline(sparklines.get(&id), to);
        }

        let summary = Self::analytics_summary(items.iter().map(|i| {
            (
                i.views,
                i.unique_visitors,
                i.engaged_views,
                i.cta_clicks,
                i.submissions,
            )
        }));

        // Previous-period totals for the delta comparison. Re-uses the same
        // list query (unsorted/unfiltered doesn't matter — only the sum is
        // kept) rather than a bespoke aggregate query, since figurine counts
        // on this site are small.
        let prev_items = self
            .repo
            .get_admin_figurine_analytics_list(prev_from, prev_to, "views", "desc")
            .await?;
        let previous_summary = Self::analytics_summary(prev_items.iter().map(|i| {
            (
                i.views,
                i.unique_visitors,
                i.engaged_views,
                i.cta_clicks,
                i.submissions,
            )
        }));

        Ok(AdminFigurineAnalyticsListPage {
            total: items.len() as i64,
            items,
            summary,
            previous_summary,
            previous_from: prev_from,
            previous_to: prev_to,
        })
    }

    pub async fn admin_get_figurine_analytics(
        &self,
        id: String,
        query: AdminAnalyticsQuery,
    ) -> Result<AdminFigurineAnalyticsDetail> {
        let figurine_id = Self::parse_uuid(&id)?;
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let (prev_from, prev_to) = Self::previous_period(from, to);
        let raw_data_from = Self::raw_data_floor().max(from);
        let figurine = self
            .repo
            .get_figurine_by_id(figurine_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Figurine {} not found", id)))?;
        let face = self
            .repo
            .get_face_images_for_figurines(&[figurine_id])
            .await?
            .get(&figurine_id)
            .cloned();
        let detail = self
            .repo
            .get_detail_images_for_figurines(&[figurine_id])
            .await?
            .get(&figurine_id)
            .cloned();
        let daily = self
            .repo
            .get_admin_figurine_analytics_daily(figurine_id, from, to)
            .await?;
        let prev_daily = self
            .repo
            .get_admin_figurine_analytics_daily(figurine_id, prev_from, prev_to)
            .await?;
        let sources = self
            .repo
            .get_admin_figurine_analytics_sources(figurine_id, from, to)
            .await?;
        // Breakdowns and medians below read raw events, which are pruned after
        // `analytics::RETENTION_DAYS` — clamp their query floor to
        // `raw_data_from` so we never silently report "no data" as if it were a
        // real empty period when it's actually just pruned.
        // Country is read from the permanent geo rollup (not raw events —
        // see get_admin_figurine_geo_breakdown), so unlike the other
        // breakdowns below it isn't clamped to raw_data_from.
        let countries = self
            .repo
            .get_admin_figurine_geo_breakdown(figurine_id, from, to, 12)
            .await?;
        let devices = self
            .repo
            .get_admin_figurine_analytics_breakdown(figurine_id, raw_data_from, to, "device", 8)
            .await?;
        let browsers = self
            .repo
            .get_admin_figurine_analytics_breakdown(figurine_id, raw_data_from, to, "browser", 8)
            .await?;
        let referrers = self
            .repo
            .get_admin_figurine_analytics_breakdown(
                figurine_id,
                raw_data_from,
                to,
                "referrer",
                12,
            )
            .await?;
        let utm_sources = self
            .repo
            .get_admin_figurine_analytics_breakdown(
                figurine_id,
                raw_data_from,
                to,
                "utm_source",
                12,
            )
            .await?;
        let visitor_cohorts = self
            .repo
            .get_admin_figurine_analytics_breakdown(figurine_id, raw_data_from, to, "visitor", 12)
            .await?;
        let languages = self
            .repo
            .get_admin_figurine_analytics_breakdown(figurine_id, raw_data_from, to, "lang", 4)
            .await?;
        let internal_sources = self
            .repo
            .get_admin_figurine_analytics_breakdown(
                figurine_id,
                raw_data_from,
                to,
                "internal_source",
                8,
            )
            .await?;
        let (median_duration_ms, median_scroll_depth) = self
            .repo
            .get_admin_figurine_engagement_medians(figurine_id, raw_data_from, to)
            .await?;
        let cta_funnel = self
            .repo
            .get_admin_figurine_cta_funnel(figurine_id, from, to)
            .await?;
        let (last7, prior7) = self
            .repo
            .get_figurine_growth_window(figurine_id, to)
            .await?;

        let summary = Self::analytics_summary(daily.iter().map(|d| {
            (
                d.views,
                d.unique_visitors,
                d.engaged_views,
                d.cta_clicks,
                d.submissions,
            )
        }));
        let previous_summary = Self::analytics_summary(prev_daily.iter().map(|d| {
            (
                d.views,
                d.unique_visitors,
                d.engaged_views,
                d.cta_clicks,
                d.submissions,
            )
        }));
        let signal = Self::analytics_signal(
            summary.views,
            summary.engaged_views,
            summary.cta_clicks,
            summary.submissions,
            summary.conversion_rate,
            Self::is_growing(last7, prior7),
        );
        let funnel = AnalyticsFunnel {
            views: summary.views,
            engaged_views: summary.engaged_views,
            cta_clicks: summary.cta_clicks,
            submissions: summary.submissions,
        };
        Ok(AdminFigurineAnalyticsDetail {
            figurine: self.to_list_item(figurine, face.as_ref(), detail.as_ref(), false),
            signal,
            summary,
            previous_summary,
            previous_from: prev_from,
            previous_to: prev_to,
            daily,
            sources,
            countries,
            devices,
            browsers,
            referrers,
            utm_sources,
            visitor_cohorts,
            languages,
            internal_sources,
            funnel,
            cta_funnel,
            median_duration_ms,
            median_scroll_depth,
            raw_data_from,
        })
    }

    /// Full (day, country) breakdown for one figurine — the geography map's
    /// "one figurine" mode. See `admin_get_figurine_geo_daily` on the repo
    /// for why this is a separate, lighter query than the full detail above.
    pub async fn admin_get_figurine_geo_daily(
        &self,
        id: String,
        query: AdminAnalyticsQuery,
    ) -> Result<Vec<FigurineGeoDailyPoint>> {
        let figurine_id = Self::parse_uuid(&id)?;
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        self.repo.get_admin_figurine_geo_daily(figurine_id, from, to).await
    }

    /// Site-wide traffic overview (all figurines summed) — J1, "is interest in
    /// the house growing overall". Built from the same pre-aggregated daily
    /// table figurine pages already fill; once Phase 3's site-wide `page_view`
    /// lands, non-figurine pages (home/archive/author/workshop/commission) will
    /// add into the same rollup.
    pub async fn admin_get_analytics_overview(
        &self,
        query: AdminAnalyticsQuery,
    ) -> Result<AdminAnalyticsOverview> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let (prev_from, prev_to) = Self::previous_period(from, to);
        let daily = self.merged_site_daily(from, to).await?;
        let prev_daily = self.merged_site_daily(prev_from, prev_to).await?;
        let figurine_sources = self.repo.get_admin_site_analytics_sources(from, to).await?;
        let geo = self.repo.get_admin_site_geo(from, to).await?;
        let summary = Self::analytics_summary(daily.iter().map(|d| {
            (
                d.views,
                d.unique_visitors,
                d.engaged_views,
                d.cta_clicks,
                d.submissions,
            )
        }));
        let previous_summary = Self::analytics_summary(prev_daily.iter().map(|d| {
            (
                d.views,
                d.unique_visitors,
                d.engaged_views,
                d.cta_clicks,
                d.submissions,
            )
        }));
        Ok(AdminAnalyticsOverview {
            from,
            to,
            previous_from: prev_from,
            previous_to: prev_to,
            summary,
            previous_summary,
            daily,
            sources: figurine_sources,
            geo,
        })
    }

    /// Site-wide daily trend: figurine-page views/engagement (from
    /// figurine_analytics_daily) plus generic-page views (from
    /// site_page_views_daily, figurine_id-IS-NULL page_view events — no
    /// overlap with the former), with `submissions` replaced day-by-day by the
    /// authoritative site-wide count (see get_admin_site_submissions_daily) so
    /// commissions with no figurine attribution aren't silently dropped from
    /// the headline total.
    async fn merged_site_daily(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<AnalyticsDailyPoint>> {
        let figurine_daily = self.repo.get_admin_site_overview_daily(from, to).await?;
        let page_views = self.repo.get_admin_site_page_views_daily(from, to).await?;
        let submissions = self.repo.get_admin_site_submissions_daily(from, to).await?;

        let mut by_day: std::collections::BTreeMap<chrono::NaiveDate, AnalyticsDailyPoint> =
            std::collections::BTreeMap::new();
        for d in figurine_daily {
            by_day.insert(d.day, d);
        }
        let blank = |day: chrono::NaiveDate| AnalyticsDailyPoint {
            day,
            views: 0,
            unique_visitors: 0,
            engaged_views: 0,
            cta_clicks: 0,
            submissions: 0,
        };
        for (day, views, uniques) in page_views {
            let entry = by_day.entry(day).or_insert_with(|| blank(day));
            entry.views += views;
            entry.unique_visitors += uniques;
        }
        for (day, subs) in submissions {
            let entry = by_day.entry(day).or_insert_with(|| blank(day));
            entry.submissions = subs;
        }
        Ok(by_day.into_values().collect())
    }

    /// Site → works → /commission → started form → submitted. Steps 1-4 are
    /// distinct-visitor counts from raw events, so they're clamped to
    /// analytics::RETENTION_DAYS just like medians/breakdowns.
    pub async fn admin_get_commission_funnel(
        &self,
        query: AdminAnalyticsQuery,
    ) -> Result<CommissionFunnel> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let raw_data_from = Self::raw_data_floor().max(from);
        let (visited, viewed_works, opened_commission_page, started_form, submitted) = self
            .repo
            .get_admin_commission_funnel(raw_data_from, to)
            .await?;
        Ok(CommissionFunnel {
            from,
            to,
            raw_data_from,
            visited,
            viewed_works,
            opened_commission_page,
            started_form,
            submitted,
        })
    }

    /// Per-page engagement for the generic pages: visits, quick-exit and
    /// reached-works rates, and time/scroll/works medians — plus the same for
    /// the previous period so the panel can show deltas. Visits come from the
    /// permanent page-views rollup (full range); everything else is derived from
    /// raw `page_engaged` events, so — like the commission funnel — that part is
    /// clamped to the raw-event floor and `raw_data_from` says how far it reaches.
    pub async fn admin_get_site_page_engagement(
        &self,
        query: AdminAnalyticsQuery,
    ) -> Result<SitePageEngagementResponse> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let (previous_from, previous_to) = Self::previous_period(from, to);
        let raw_data_from = Self::raw_data_floor().max(from);
        let pages = self.assemble_site_page_engagement(from, to).await?;
        let previous_pages = self
            .assemble_site_page_engagement(previous_from, previous_to)
            .await?;
        Ok(SitePageEngagementResponse {
            from,
            to,
            previous_from,
            previous_to,
            raw_data_from,
            pages,
            previous_pages,
        })
    }

    /// Engagement (retention-clamped) merged with views (permanent rollup),
    /// keyed by path_group — a page with visits but no engagement events still
    /// appears, with its rates/medians left empty.
    async fn assemble_site_page_engagement(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<Vec<SitePageEngagement>> {
        let raw_from = Self::raw_data_floor().max(from);
        let engagement = self.repo.get_admin_site_page_engagement(raw_from, to).await?;
        let views = self.repo.get_admin_site_page_views_by_group(from, to).await?;

        let mut by_group: std::collections::BTreeMap<String, SitePageEngagement> = engagement
            .into_iter()
            .map(|e| (e.path_group.clone(), e))
            .collect();
        for (path_group, v, u) in views {
            let entry = by_group
                .entry(path_group.clone())
                .or_insert_with(|| SitePageEngagement {
                    path_group,
                    views: 0,
                    unique_visitors: 0,
                    engaged_events: 0,
                    quick_exit_events: 0,
                    reached_works_events: 0,
                    median_duration_ms: None,
                    median_scroll_depth: None,
                    median_works_seen: None,
                });
            entry.views = v;
            entry.unique_visitors = u;
        }
        Ok(by_group.into_values().collect())
    }

    /// Paged list of anonymous visitor sessions (daily `visitor_hash`) active in
    /// range, newest first. Raw-event derived, so clamped to the retention floor;
    /// `raw_data_from`/`total` let the panel show coverage and paginate.
    pub async fn admin_get_visitor_sessions(
        &self,
        query: AdminVisitorsQuery,
    ) -> Result<AdminVisitorSessionsPage> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let raw_data_from = Self::raw_data_floor().max(from);
        let limit = query.limit.unwrap_or(100).clamp(1, 500);
        let offset = query.offset.unwrap_or(0).max(0);
        let only_actions = query.only_actions.unwrap_or(false);
        let sessions = self
            .repo
            .get_admin_visitor_sessions(raw_data_from, to, limit, offset, only_actions)
            .await?;
        let total = self
            .repo
            .count_admin_visitor_sessions(raw_data_from, to, only_actions)
            .await?;
        Ok(AdminVisitorSessionsPage {
            sessions,
            total,
            from,
            to,
            raw_data_from,
        })
    }

    /// One anonymous visitor's event timeline. The hash is validated as hex
    /// (it's an HMAC digest) before it reaches SQL — a cheap guard against a
    /// malformed or probing id.
    pub async fn admin_get_visitor_timeline(
        &self,
        visitor_hash: String,
        query: AdminVisitorsQuery,
    ) -> Result<Vec<AdminVisitorEvent>> {
        let hash = visitor_hash.trim();
        if hash.is_empty() || hash.len() > 128 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(AppError::BadRequest("Invalid visitor id".into()));
        }
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let raw_data_from = Self::raw_data_floor().max(from);
        self.repo
            .get_admin_visitor_timeline(hash, raw_data_from, to, 500)
            .await
    }

    pub async fn admin_create_analytics_annotation(
        &self,
        req: CreateAnnotationRequest,
    ) -> Result<AnalyticsAnnotation> {
        let label = req.label.trim();
        if label.is_empty() || label.chars().count() > 200 {
            return Err(AppError::BadRequest(
                "Annotation label must be 1-200 characters".into(),
            ));
        }
        self.repo
            .create_analytics_annotation(&CreateAnnotationRequest {
                day: req.day,
                label: label.to_string(),
            })
            .await
    }

    pub async fn admin_list_analytics_annotations(
        &self,
        query: AdminAnalyticsQuery,
    ) -> Result<Vec<AnalyticsAnnotation>> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        self.repo.list_analytics_annotations(from, to).await
    }

    pub async fn admin_delete_analytics_annotation(&self, id: Uuid) -> Result<()> {
        self.repo.delete_analytics_annotation(id).await
    }

    pub async fn admin_get_life_of_house_trend(
        &self,
        query: AdminAnalyticsQuery,
    ) -> Result<LifeOfHouseTrend> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let (prev_from, prev_to) = Self::previous_period(from, to);
        let daily = self.repo.get_admin_life_of_house_daily(from, to).await?;
        let prev_daily = self
            .repo
            .get_admin_life_of_house_daily(prev_from, prev_to)
            .await?;
        let sum = |points: &[LifeOfHouseDailyPoint]| -> (i64, i64, i64) {
            points.iter().fold((0, 0, 0), |(m, s, c), p| {
                (m + p.marks, s + p.subscribers, c + p.comments)
            })
        };
        let (marks_total, subscribers_total, comments_total) = sum(&daily);
        let (previous_marks_total, previous_subscribers_total, previous_comments_total) =
            sum(&prev_daily);
        Ok(LifeOfHouseTrend {
            from,
            to,
            daily,
            marks_total,
            subscribers_total,
            comments_total,
            previous_marks_total,
            previous_subscribers_total,
            previous_comments_total,
        })
    }

    /// One-off admin action: re-run the daily aggregation over a historical
    /// range. Needed after a fix to `refresh_analytics_aggregates` itself
    /// (e.g. the commissions attribution join) — the automatic hot-window job
    /// only ever touches yesterday+today, so older days stay wrong until this
    /// runs. Idempotent; safe to call more than once over overlapping ranges.
    pub async fn admin_backfill_analytics(
        &self,
        req: BackfillAnalyticsRequest,
    ) -> Result<BackfillAnalyticsResponse> {
        const MAX_BACKFILL_DAYS: i64 = 800;
        let to = req.to.unwrap_or_else(|| chrono::Utc::now().date_naive());
        let from = match req.from {
            Some(f) => f,
            None => self
                .repo
                .get_earliest_analytics_day()
                .await?
                .unwrap_or(to),
        };
        if from > to {
            return Err(AppError::BadRequest("Invalid backfill range".into()));
        }
        if (to - from).num_days() > MAX_BACKFILL_DAYS {
            return Err(AppError::BadRequest(
                "Backfill range is too large (max 800 days)".into(),
            ));
        }
        self.repo.refresh_analytics_aggregates(from, to).await?;
        Ok(BackfillAnalyticsResponse { from, to })
    }

    fn analytics_range(
        from: Option<chrono::NaiveDate>,
        to: Option<chrono::NaiveDate>,
    ) -> Result<(chrono::NaiveDate, chrono::NaiveDate)> {
        let today = chrono::Utc::now().date_naive();
        let to = to.unwrap_or(today);
        let from = from.unwrap_or(to - chrono::Duration::days(29));
        if from > to {
            return Err(AppError::BadRequest("Invalid analytics date range".into()));
        }
        if (to - from).num_days() > 366 {
            return Err(AppError::BadRequest(
                "Analytics date range is too large".into(),
            ));
        }
        Ok((from, to))
    }

    /// The immediately-preceding period of identical length, for delta
    /// comparisons: for `[from, to]` (inclusive, `len` days), the previous
    /// period is `[from - len, from - 1]` — contiguous, same length, no overlap
    /// and no gap.
    fn previous_period(
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> (chrono::NaiveDate, chrono::NaiveDate) {
        let len_days = (to - from).num_days() + 1;
        let prev_to = from - chrono::Duration::days(1);
        let prev_from = prev_to - chrono::Duration::days(len_days - 1);
        (prev_from, prev_to)
    }

    /// Earliest date raw `figurine_analytics_events` rows can still exist for.
    /// The retention-prune job runs off real "now", not whatever range the
    /// admin happens to be browsing, so this floor is anchored to today.
    fn raw_data_floor() -> chrono::NaiveDate {
        chrono::Utc::now().date_naive()
            - chrono::Duration::days(crate::analytics::RETENTION_DAYS - 1)
    }

    /// `growing_interest` requires both a minimum-volume gate (so 1 -> 3 views
    /// doesn't read as "+200%") and real week-over-week growth. A `prior7` of
    /// zero can't produce a ratio, so it's treated as growth as long as the
    /// current week alone clears the volume gate.
    fn is_growing(last7: i64, prior7: i64) -> bool {
        const MIN_VOLUME: i64 = 10;
        const GROWTH_RATIO: f64 = 1.3;
        if last7 < MIN_VOLUME {
            return false;
        }
        if prior7 == 0 {
            return true;
        }
        (last7 as f64) >= (prior7 as f64) * GROWTH_RATIO
    }

    /// Turn a sparse (day, views) list into a dense 14-point array ending at
    /// `to`, zero-filling any day with no aggregate row.
    fn zero_filled_sparkline(
        points: Option<&Vec<(chrono::NaiveDate, i64)>>,
        to: chrono::NaiveDate,
    ) -> Vec<i64> {
        let by_day: HashMap<chrono::NaiveDate, i64> = points
            .map(|p| p.iter().cloned().collect())
            .unwrap_or_default();
        (0..14)
            .rev()
            .map(|offset| {
                let day = to - chrono::Duration::days(offset);
                by_day.get(&day).copied().unwrap_or(0)
            })
            .collect()
    }

    fn analytics_summary(
        rows: impl Iterator<Item = (i64, i64, i64, i64, i64)>,
    ) -> AnalyticsSummary {
        let mut summary = AnalyticsSummary {
            views: 0,
            unique_visitors: 0,
            engaged_views: 0,
            cta_clicks: 0,
            submissions: 0,
            conversion_rate: 0.0,
        };
        for (views, unique_visitors, engaged_views, cta_clicks, submissions) in rows {
            summary.views += views;
            summary.unique_visitors += unique_visitors;
            summary.engaged_views += engaged_views;
            summary.cta_clicks += cta_clicks;
            summary.submissions += submissions;
        }
        summary.conversion_rate = if summary.engaged_views > 0 {
            ((summary.submissions as f64 / summary.engaged_views as f64) * 10000.0).round() / 100.0
        } else {
            0.0
        };
        summary
    }

    fn analytics_signal(
        views: i64,
        engaged_views: i64,
        cta_clicks: i64,
        submissions: i64,
        conversion_rate: f64,
        is_growing: bool,
    ) -> AnalyticsSignal {
        if views < 10 {
            AnalyticsSignal::LowData
        } else if conversion_rate >= 12.0 && submissions >= 2 {
            AnalyticsSignal::HighConversion
        } else if submissions == 0 && (engaged_views >= 8 || cta_clicks >= 3) {
            AnalyticsSignal::AttentionNoSubmissions
        } else if is_growing {
            AnalyticsSignal::GrowingInterest
        } else if views < 25 {
            AnalyticsSignal::LowVisibility
        } else {
            AnalyticsSignal::Normal
        }
    }

    // === CONTENT API ===

    // Resolve a stored path/URL to a full URL for the frontend:
    // - "http..." → use as-is (external URL or legacy full URL)
    // - "/static/..." → prepend public_url (web-uploaded file, stored as relative path)
    // - anything else → treat as a relative path under /static (legacy stored paths)
    fn resolve_url(&self, file_path: &str, _table: &str, _id: &str) -> String {
        let base = self.config.public_url.trim_end_matches('/');
        if file_path.starts_with("http") {
            file_path.to_string()
        } else if file_path.starts_with("/static/") {
            format!("{}{}", base, file_path)
        } else {
            format!("{}/static/{}", base, file_path.trim_start_matches('/'))
        }
    }

    fn parse_uuid(s: &str) -> Result<Uuid> {
        Uuid::parse_str(s)
            .map_err(|_| crate::error::AppError::BadRequest(format!("Invalid ID: {}", s)))
    }

    /// Resolve a public figurine handle to its canonical UUID. The detail page (and
    /// therefore every sub-resource call the client makes from it — comments, schedule,
    /// marks, bookings, waitlist, analytics) can be reached by either a transliterated
    /// slug or the raw UUID. A UUID passes straight through (no DB hit); anything else is
    /// looked up as a slug. Keeps every entity keyed on the same UUID regardless of which
    /// URL the visitor arrived by.
    pub async fn resolve_figurine_uuid(&self, handle: &str) -> Result<Uuid> {
        match Uuid::parse_str(handle) {
            Ok(uuid) => Ok(uuid),
            Err(_) => self
                .repo
                .get_figurine_by_slug(handle)
                .await?
                .map(|f| f.id)
                .ok_or_else(|| AppError::NotFound(format!("Figurine {} not found", handle))),
        }
    }

    /// Resolve the public URL for a figurine's face thumbnail (thumb if present,
    /// else the full image).
    fn face_image_url(&self, img: &Image) -> String {
        let i_id_str = img.id.to_string();
        img.thumb_path
            .as_ref()
            .map(|p| self.resolve_url(p, "images_thumb", &i_id_str))
            .unwrap_or_else(|| self.resolve_url(&img.file_path, "images", &i_id_str))
    }

    /// The preview-sized (1800px) URL for the same image — what the detail page
    /// shows. `file_path` is the preview variant; the thumb lives in thumb_path.
    fn face_image_large_url(&self, img: &Image) -> String {
        self.resolve_url(&img.file_path, "images", &img.id.to_string())
    }

    fn to_list_item(
        &self,
        f: Figurine,
        face: Option<&Image>,
        detail: Option<&Image>,
        house_favorite: bool,
    ) -> FigurineListItemDto {
        FigurineListItemDto {
            id: f.id.to_string(),
            name: f.name,
            slug: f.slug,
            slug_manual: f.slug_manual,
            status: f.status,
            short_text: f.short_text,
            face_image_url: face.map(|i| self.face_image_url(i)),
            detail_image_url: detail.map(|i| self.face_image_url(i)),
            face_image_large_url: face.map(|i| self.face_image_large_url(i)),
            detail_image_large_url: detail.map(|i| self.face_image_large_url(i)),
            year: f.year,
            sort_order: f.sort_order,
            series: None,
            technique: f.technique,
            material: f.material,
            is_featured: f.is_featured,
            created_at: f.created_at,
            updated_at: f.updated_at,
            focal_x: face.and_then(|i| i.focal_x),
            focal_y: face.and_then(|i| i.focal_y),
            reveal_radius: face.and_then(|i| i.reveal_radius),
            darkness: face.and_then(|i| i.darkness),
            open_from_min: f.open_from_min,
            open_until_min: f.open_until_min,
            sealed_door_image: f.sealed_door_image,
            showing_room_id: f.showing_room_id.map(|u| u.to_string()),
            first_look_until: f.first_look_until,
            house_favorite,
        }
    }

    /// Batch "House Favorite" lookup for a list of figurines about to be
    /// rendered as cards — computed once (percentile over the whole
    /// collection) and intersected with the ids actually being rendered.
    async fn house_favorite_ids(&self, ids: &[Uuid]) -> Result<std::collections::HashSet<Uuid>> {
        let tiers = self.favorite_tiers().await?;
        Ok(ids.iter().filter(|id| tiers.house_favorite.contains(id)).copied().collect())
    }

    /// The favorite tiers, recomputed at most once a minute (see favorite_tiers_cache).
    async fn favorite_tiers(&self) -> Result<crate::db::FavoriteTiers> {
        let mut cache = self.favorite_tiers_cache.lock().await;
        if let Some((computed_at, tiers)) = cache.as_ref() {
            if computed_at.elapsed() < FAVORITE_TIERS_TTL {
                return Ok(tiers.clone());
            }
        }
        let tiers = self.repo.get_favorite_tiers().await?;
        *cache = Some((Instant::now(), tiers.clone()));
        Ok(tiers)
    }

    pub async fn list_figurines(&self, visible_only: bool, mut query: crate::models::FigurineQuery) -> Result<crate::models::FigurinesPage> {
        // A bounded default so an omitted page size can never become an unbounded
        // scan-and-return of the whole table: get_all_figurines applies no LIMIT at
        // all when per_page is None. 1000 sits far above the real catalogue size yet
        // caps the worst case for sitemap/feed and any client that forgets perPage.
        const DEFAULT_PAGE_CAP: i64 = 1000;
        let page = query.page.unwrap_or(1).max(1);
        let per_page = query
            .per_page
            .unwrap_or(DEFAULT_PAGE_CAP)
            .clamp(1, DEFAULT_PAGE_CAP);
        query.per_page = Some(per_page);
        let (figurines, total) = self.repo.get_all_figurines(visible_only, &query).await?;
        let ids: Vec<Uuid> = figurines.iter().map(|f| f.id).collect();
        let faces = self.repo.get_face_images_for_figurines(&ids).await?;
        let details = self.repo.get_detail_images_for_figurines(&ids).await?;
        let favorites = self.house_favorite_ids(&ids).await?;
        let items = figurines
            .into_iter()
            .map(|f| {
                let face = faces.get(&f.id);
                let detail = details.get(&f.id);
                let favorite = favorites.contains(&f.id);
                self.to_list_item(f, face, detail, favorite)
            })
            .collect();
        Ok(crate::models::FigurinesPage { items, total, page, per_page })
    }

    /// Works inside their "first look" window — the book-holders' shelf. Public
    /// endpoint (membership is device-local, not an auth boundary); the home page
    /// renders these only for a signed visitor.
    pub async fn list_first_look_figurines(&self) -> Result<Vec<FigurineListItemDto>> {
        let figurines = self.repo.get_first_look_figurines().await?;
        let ids: Vec<Uuid> = figurines.iter().map(|f| f.id).collect();
        let faces = self.repo.get_face_images_for_figurines(&ids).await?;
        let details = self.repo.get_detail_images_for_figurines(&ids).await?;
        let favorites = self.house_favorite_ids(&ids).await?;
        Ok(figurines
            .into_iter()
            .map(|f| {
                let face = faces.get(&f.id);
                let detail = details.get(&f.id);
                let favorite = favorites.contains(&f.id);
                self.to_list_item(f, face, detail, favorite)
            })
            .collect())
    }

    pub async fn list_in_progress_figurines(&self) -> Result<Vec<FigurineListItemDto>> {
        let q = crate::models::FigurineQuery {
            status: Some("in_progress".into()),
            ..Default::default()
        };
        let (figurines, _) = self.repo.get_all_figurines(true, &q).await?;
        let ids: Vec<Uuid> = figurines.iter().map(|f| f.id).collect();
        let faces = self.repo.get_face_images_for_figurines(&ids).await?;
        let details = self.repo.get_detail_images_for_figurines(&ids).await?;
        let favorites = self.house_favorite_ids(&ids).await?;
        Ok(figurines
            .into_iter()
            .map(|f| {
                let face = faces.get(&f.id);
                let detail = details.get(&f.id);
                let favorite = favorites.contains(&f.id);
                self.to_list_item(f, face, detail, favorite)
            })
            .collect())
    }

    pub async fn get_figurine_details(&self, id: String) -> Result<FigurineDto> {
        // `id` is a handle: a UUID for legacy/canonical links, or a transliterated
        // slug for the pretty URL. Try UUID first (cheap parse); anything else is a
        // slug lookup. Either resolves to the same work.
        let figurine = match Uuid::parse_str(&id) {
            Ok(uuid) => self.repo.get_figurine_by_id(uuid).await?,
            Err(_) => self.repo.get_figurine_by_slug(&id).await?,
        }
        .ok_or_else(|| {
            crate::error::AppError::NotFound(format!("Figurine {} not found", id))
        })?;

        let uuid = figurine.id;

        // Percentile rank among marked figurines (see get_favorite_tiers) — a
        // relative "deservedly in the top" signal, not an arbitrary fixed
        // score. Below a minimum eligible pool size both tiers come back
        // empty, and we say nothing at all (see project decision on negative
        // social proof) rather than crown whichever piece happens to lead an
        // almost-empty field.
        let tiers = self.favorite_tiers().await?;
        let noticed_by_others = tiers.noticed.contains(&uuid);
        let house_favorite = tiers.house_favorite.contains(&uuid);

        let images = self.repo.get_images_by_figurine(uuid).await?;
        let steps = self.repo.get_steps_by_figurine(uuid).await?;
        let related_entities = self.repo.get_related_figurines(uuid).await?;

        let fig_id_str = figurine.id.to_string();

        let related_ids: Vec<Uuid> = related_entities.iter().map(|r| r.id).collect();
        let related_faces = self
            .repo
            .get_face_images_for_figurines(&related_ids)
            .await?;
        let related_details = self
            .repo
            .get_detail_images_for_figurines(&related_ids)
            .await?;
        let related_favorites = self.house_favorite_ids(&related_ids).await?;
        let related_items: Vec<FigurineListItemDto> = related_entities
            .into_iter()
            .map(|r| {
                let face = related_faces.get(&r.id);
                let detail = related_details.get(&r.id);
                let favorite = related_favorites.contains(&r.id);
                self.to_list_item(r, face, detail, favorite)
            })
            .collect();

        let image_dtos = images
            .into_iter()
            .map(|i| {
                let i_id_str = i.id.to_string();
                ImageDto {
                    id: i_id_str.clone(),
                    image_type: i.image_type,
                    url: self.resolve_url(&i.file_path, "images", &i_id_str),
                    original_url: i
                        .original_path
                        .as_ref()
                        .map(|p| self.resolve_url(p, "images_original", &i_id_str)),
                    thumb_url: i
                        .thumb_path
                        .as_ref()
                        .map(|p| self.resolve_url(p, "images_thumb", &i_id_str)),
                    depth_url: i
                        .depth_path
                        .as_ref()
                        .map(|p| self.resolve_url(p, "images_depth", &i_id_str)),
                    parallax_intensity: i.parallax_intensity,
                    focal_x: i.focal_x,
                    focal_y: i.focal_y,
                    reveal_radius: i.reveal_radius,
                    darkness: i.darkness,
                    alt_text: i.alt_text,
                }
            })
            .collect();

        let step_dtos = steps
            .into_iter()
            .map(|s| {
                let s_id_str = s.id.to_string();
                ProcessStepDto {
                    id: s_id_str.clone(),
                    step_type: s.step_type,
                    description: s.description,
                    image_url: self.resolve_url(&s.image_path, "process_steps", &s_id_str),
                }
            })
            .collect();

        Ok(FigurineDto {
            id: fig_id_str.clone(),
            name: figurine.name,
            slug: figurine.slug,
            short_text: figurine.short_text,
            full_description: figurine.full_description,
            dimensions: figurine.dimensions,
            material: figurine.material,
            technique: figurine.technique,
            year: figurine.year,
            passport_number: figurine.passport_number,
            edition: figurine.edition,
            created_period: figurine.created_period,
            care_instructions: figurine.care_instructions,
            provenance_note: figurine.provenance_note,
            authenticity_note: figurine.authenticity_note,
            included_items: figurine.included_items,
            ambience_path: figurine
                .ambience_path
                .as_ref()
                .map(|p| self.resolve_url(p, "figurines_audio", &fig_id_str)),
            video_url: figurine
                .video_url
                .as_ref()
                .map(|p| self.resolve_url(p, "figurines_video", &fig_id_str)),
            secret_text: figurine.secret_text,
            status: figurine.status,
            sort_order: figurine.sort_order,
            is_visible: figurine.is_visible,
            is_featured: figurine.is_featured,
            open_from_min: figurine.open_from_min,
            open_until_min: figurine.open_until_min,
            sealed_door_image: figurine.sealed_door_image,
            showing_room_id: figurine.showing_room_id.map(|u| u.to_string()),
            display_layout: figurine.display_layout,
            display_config: figurine.display_config,
            first_look_until: figurine.first_look_until,
            images: image_dtos,
            process_steps: step_dtos,
            related_items,
            noticed_by_others,
            house_favorite,
        })
    }

    /// On-demand depth-map generation for one figurine (admin button). Runs
    /// Depth-Anything in-process (candle, CPU) for each image that still lacks a
    /// depth map, writes the grayscale PNG into the uploads tree and records
    /// `depth_path`. Inference is serialised inside the depth module.
    pub async fn generate_figurine_depth(&self, id: String) -> Result<DepthGenSummary> {
        if !crate::depth::is_available() {
            return Err(AppError::Internal(
                "Depth model unavailable (weights not bundled in this build)".into(),
            ));
        }
        let uuid = Self::parse_uuid(&id)?;
        self.repo
            .get_figurine_by_id(uuid)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Figurine {} not found", id)))?;

        let images = self.repo.get_images_by_figurine(uuid).await?;
        let upload_dir = self.config.upload_dir.clone();
        let mut results = Vec::new();
        let (mut generated, mut skipped, mut failed) = (0usize, 0usize, 0usize);

        for img in images {
            let image_id = img.id.to_string();

            if img.depth_path.is_some() {
                skipped += 1;
                results.push(DepthGenItem {
                    image_id,
                    status: "skip".into(),
                    detail: Some("already has a depth map".into()),
                });
                continue;
            }

            let stored = img
                .original_path
                .clone()
                .unwrap_or_else(|| img.file_path.clone());
            let src = crate::depth::local_source(&stored, &upload_dir);
            if !src.exists() {
                skipped += 1;
                results.push(DepthGenItem {
                    image_id,
                    status: "skip".into(),
                    detail: Some(format!("source missing: {}", src.display())),
                });
                continue;
            }

            let rel_depth = format!("images/depth/{}.png", image_id);
            let out = std::path::Path::new(&upload_dir).join(&rel_depth);
            let (src2, out2) = (src.clone(), out.clone());
            let res = tokio::task::spawn_blocking(move || crate::depth::generate(&src2, &out2))
                .await
                .map_err(|e| AppError::Internal(format!("depth task join error: {e}")))?;

            match res {
                Ok(()) => {
                    sqlx::query("UPDATE images SET depth_path = $1 WHERE id = $2")
                        .bind(&rel_depth)
                        .bind(img.id)
                        .execute(self.repo.pg_pool())
                        .await?;
                    generated += 1;
                    results.push(DepthGenItem {
                        image_id,
                        status: "done".into(),
                        detail: Some(rel_depth),
                    });
                }
                Err(e) => {
                    failed += 1;
                    results.push(DepthGenItem {
                        image_id,
                        status: "fail".into(),
                        detail: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(DepthGenSummary {
            generated,
            skipped,
            failed,
            results,
        })
    }

    /// Bulk admin action: regenerate depth maps for every image across the
    /// whole collection that doesn't have one yet. Thin wrapper around
    /// `generate_figurine_depth`, run figurine-by-figurine.
    pub async fn bulk_recalculate_parallax(&self) -> Result<DepthGenSummary> {
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM figurines")
            .fetch_all(self.repo.pg_pool())
            .await?;
        let (mut generated, mut skipped, mut failed) = (0usize, 0usize, 0usize);
        let mut results = Vec::new();
        for id in ids {
            let summary = self.generate_figurine_depth(id.to_string()).await?;
            generated += summary.generated;
            skipped += summary.skipped;
            failed += summary.failed;
            results.extend(summary.results);
        }
        Self::log_domain_event("bulk_parallax_recalculated", "figurine", generated, "ok");
        Ok(DepthGenSummary {
            generated,
            skipped,
            failed,
            results,
        })
    }

    // ── Semantic search ("Хранитель") ──────────────────────────────────────
    //
    // One dense text embedding per figurine (embed.rs, on-device candle). A
    // guest's natural-language query is embedded the same way and ranked by
    // cosine similarity. Corpus is tiny, so ranking is a brute-force in-memory
    // dot product — no vector index needed.

    /// Noise gate on cosine similarity: results below this are dropped so a
    /// nonsense query returns nothing rather than the "least irrelevant" works.
    /// Tuning knob — multilingual-e5 similarities sit in a compressed, high band,
    /// so this is deliberately gentle; raise it if weak matches leak through.
    const SEMANTIC_FLOOR: f32 = 0.70;

    /// Concatenate a figurine's curatorial fields into the text that gets
    /// embedded — ordered loosely by descriptive weight; blank/NULL fields drop.
    fn curatorial_text(f: &Figurine) -> String {
        let opt = |s: &Option<String>| -> Option<String> {
            s.as_ref()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let mut parts: Vec<String> = vec![f.name.trim().to_string()];
        for field in [
            opt(&f.short_text),
            opt(&f.full_description),
            // Backstage AI description of what the photo shows — lets a visual
            // query ("монах со свечой") match through the text encoder.
            opt(&f.visual_caption),
            opt(&f.material),
            opt(&f.technique),
            opt(&f.dimensions),
            opt(&f.edition),
            opt(&f.created_period),
            opt(&f.included_items),
            opt(&f.provenance_note),
            opt(&f.authenticity_note),
            opt(&f.care_instructions),
        ] {
            if let Some(v) = field {
                parts.push(v);
            }
        }
        if let Some(y) = f.year {
            parts.push(y.to_string());
        }
        parts.retain(|p| !p.is_empty());
        parts.join(". ")
    }

    fn text_hash(model: &str, text: &str) -> String {
        use sha2::Digest;
        let mut h = sha2::Sha256::new();
        h.update(model.as_bytes());
        h.update(b"\n");
        h.update(text.as_bytes());
        hex::encode(h.finalize())
    }

    /// (Re)embed one figurine if its text or the model changed. Returns whether a
    /// new vector was written. No-ops (Ok(false)) when weights aren't bundled, so
    /// callers can fire this best-effort after a save without guarding.
    pub async fn reindex_figurine_embedding(&self, id: Uuid) -> Result<bool> {
        if !crate::embed::is_available() {
            return Ok(false);
        }
        let Some(fig) = self.repo.get_figurine_by_id(id).await? else {
            return Ok(false);
        };
        let model = crate::embed::model_name();
        let text = Self::curatorial_text(&fig);
        let hash = Self::text_hash(&model, &text);

        let existing: Option<String> = sqlx::query_scalar(
            "SELECT source_hash FROM figurine_embeddings WHERE figurine_id = $1 AND model = $2",
        )
        .bind(id)
        .bind(&model)
        .fetch_optional(self.repo.pg_pool())
        .await?;
        if existing.as_deref() == Some(hash.as_str()) {
            return Ok(false);
        }

        let text_owned = text.clone();
        let vec = tokio::task::spawn_blocking(move || crate::embed::embed_passage(&text_owned))
            .await
            .map_err(|e| AppError::Internal(format!("embed task join error: {e}")))?
            .map_err(|e| AppError::Internal(format!("embed failed: {e}")))?;
        let dim = vec.len() as i32;
        let bytes = crate::embed::to_bytes(&vec);

        sqlx::query(
            "INSERT INTO figurine_embeddings (figurine_id, model, dim, vec, source_hash, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW())
             ON CONFLICT (figurine_id) DO UPDATE
               SET model = EXCLUDED.model, dim = EXCLUDED.dim, vec = EXCLUDED.vec,
                   source_hash = EXCLUDED.source_hash, updated_at = NOW()",
        )
        .bind(id)
        .bind(&model)
        .bind(dim)
        .bind(&bytes)
        .bind(&hash)
        .execute(self.repo.pg_pool())
        .await?;
        Ok(true)
    }

    /// Admin action: (re)index every publicly-visible work, and drop vectors for
    /// works no longer public so they can't surface in search.
    pub async fn reindex_all_embeddings(&self) -> Result<EmbedIndexSummary> {
        if !crate::embed::is_available() {
            return Err(AppError::Internal(
                "Embedding model unavailable (weights not bundled in this build)".into(),
            ));
        }
        let q = FigurineQuery {
            status: None,
            search: None,
            sort: None,
            page: None,
            per_page: None,
        };
        let (figs, _total) = self.repo.get_all_figurines(true, &q).await?;
        let total = figs.len();
        let (mut indexed, mut skipped, mut failed) = (0usize, 0usize, 0usize);
        for f in &figs {
            match self.reindex_figurine_embedding(f.id).await {
                Ok(true) => indexed += 1,
                Ok(false) => skipped += 1,
                Err(_) => failed += 1,
            }
        }
        // Prune vectors for works that are no longer public.
        let visible_ids: Vec<Uuid> = figs.iter().map(|f| f.id).collect();
        let _ = sqlx::query("DELETE FROM figurine_embeddings WHERE figurine_id <> ALL($1)")
            .bind(&visible_ids)
            .execute(self.repo.pg_pool())
            .await;
        Self::log_domain_event("embeddings_reindexed", "figurine", indexed, "ok");
        Ok(EmbedIndexSummary {
            total,
            indexed,
            skipped,
            failed,
        })
    }

    /// Rank publicly-visible works by cosine similarity to a natural-language
    /// query. Returns id + score, closest first (client already holds the archive
    /// and reorders). Empty when the query is blank or weights aren't bundled.
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<SemanticHit>> {
        let query = query.trim();
        if query.is_empty() || !crate::embed::is_available() {
            return Ok(Vec::new());
        }
        let model = crate::embed::model_name();

        let q_owned = query.to_string();
        let qv = tokio::task::spawn_blocking(move || crate::embed::embed_query(&q_owned))
            .await
            .map_err(|e| AppError::Internal(format!("embed task join error: {e}")))?
            .map_err(|e| AppError::Internal(format!("embed failed: {e}")))?;

        let rows: Vec<(Uuid, Vec<u8>)> =
            sqlx::query_as("SELECT figurine_id, vec FROM figurine_embeddings WHERE model = $1")
                .bind(&model)
                .fetch_all(self.repo.pg_pool())
                .await?;

        // Restrict to works that are currently public (respects hide/first-look
        // even if the vector is momentarily stale).
        let fq = FigurineQuery {
            status: None,
            search: None,
            sort: None,
            page: None,
            per_page: None,
        };
        let (figs, _) = self.repo.get_all_figurines(true, &fq).await?;
        let visible: HashSet<Uuid> = figs.iter().map(|f| f.id).collect();

        let mut hits: Vec<SemanticHit> = rows
            .into_iter()
            .filter(|(id, _)| visible.contains(id))
            .filter_map(|(id, bytes)| {
                crate::embed::from_bytes(&bytes).map(|v| SemanticHit {
                    id: id.to_string(),
                    score: crate::embed::dot(&qv, &v),
                })
            })
            .filter(|h| h.score >= Self::SEMANTIC_FLOOR)
            .collect();

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(limit.clamp(1, 200));
        Ok(hits)
    }

    /// Read a work's backstage visual caption (search-only; never public).
    pub async fn get_figurine_caption(&self, id: String) -> Result<Option<String>> {
        let uuid = Self::parse_uuid(&id)?;
        Ok(self
            .repo
            .get_figurine_by_id(uuid)
            .await?
            .and_then(|f| f.visual_caption))
    }

    /// Set (or clear, when blank) a work's backstage visual caption, then refresh
    /// its search embedding so the new text is immediately searchable. Deliberately
    /// separate from save_figurine so ordinary edits never touch the caption, and
    /// so the offline captioner can write it without the full figurine payload.
    pub async fn set_figurine_caption(&self, id: String, caption: Option<String>) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        let cap = caption
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        sqlx::query("UPDATE figurines SET visual_caption = $1 WHERE id = $2")
            .bind(&cap)
            .bind(uuid)
            .execute(self.repo.pg_pool())
            .await?;
        // Best-effort: re-embed so the caption is reflected in search right away.
        if let Err(e) = self.reindex_figurine_embedding(uuid).await {
            tracing::warn!(target: "gotiga_server::embed", error = %e, "reindex after caption set failed");
        }
        Ok(())
    }

    /// Bulk admin action: set every image's darkness to 0, fully dissolving
    /// the keyhole shadow (the CSS gradient collapses to zero alpha at 0, so
    /// this switches the veil off entirely rather than reverting to the
    /// global default, which is still dark).
    pub async fn bulk_clear_darkness(&self) -> Result<crate::models::BulkOpSummary> {
        let affected = sqlx::query("UPDATE images SET darkness = 0 WHERE darkness IS DISTINCT FROM 0")
            .execute(self.repo.pg_pool())
            .await?
            .rows_affected();
        Self::log_domain_event("bulk_darkness_cleared", "image", affected, "ok");
        Ok(crate::models::BulkOpSummary { affected })
    }

    /// Bulk admin action: reset the manual parallax intensity override for
    /// every image back to the default.
    pub async fn bulk_reset_parallax_intensity(&self) -> Result<crate::models::BulkOpSummary> {
        let affected = sqlx::query(
            "UPDATE images SET parallax_intensity = NULL WHERE parallax_intensity IS NOT NULL",
        )
        .execute(self.repo.pg_pool())
        .await?
        .rows_affected();
        Self::log_domain_event("bulk_parallax_reset", "image", affected, "ok");
        Ok(crate::models::BulkOpSummary { affected })
    }

    /// Bulk admin action: set the same parallax intensity on every image.
    pub async fn bulk_set_parallax_intensity(
        &self,
        intensity: f32,
    ) -> Result<crate::models::BulkOpSummary> {
        if !(0.0..=1.0).contains(&intensity) {
            return Err(AppError::BadRequest(
                "Parallax intensity must be between 0 and 1".into(),
            ));
        }
        let affected = sqlx::query("UPDATE images SET parallax_intensity = $1")
            .bind(intensity)
            .execute(self.repo.pg_pool())
            .await?
            .rows_affected();
        Self::log_domain_event("bulk_parallax_set", "image", affected, "ok");
        Ok(crate::models::BulkOpSummary { affected })
    }

    /// Bulk admin action: for every figurine that has at least two images,
    /// mark the second image (by display order) as the "detail" (second
    /// angle) image, clearing any previous detail mark on that figurine.
    /// The face image is never overwritten this way, so a figurine whose
    /// second image is its face image is left untouched.
    pub async fn bulk_set_second_angle(&self) -> Result<crate::models::BulkOpSummary> {
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM figurines")
            .fetch_all(self.repo.pg_pool())
            .await?;
        let mut affected = 0u64;
        for figurine_id in ids {
            let images = self.repo.get_images_by_figurine(figurine_id).await?;
            let Some(second) = images.get(1) else {
                continue;
            };
            if second.image_type == crate::models::ImageType::Face
                || second.image_type == crate::models::ImageType::Detail
            {
                continue;
            }
            sqlx::query(
                "UPDATE images SET image_type = 'full' WHERE figurine_id = $1 AND image_type = 'detail'",
            )
            .bind(figurine_id)
            .execute(self.repo.pg_pool())
            .await?;
            sqlx::query("UPDATE images SET image_type = 'detail' WHERE id = $1")
                .bind(second.id)
                .execute(self.repo.pg_pool())
                .await?;
            affected += 1;
        }
        Self::log_domain_event("bulk_second_angle_set", "figurine", affected, "ok");
        Ok(crate::models::BulkOpSummary { affected })
    }

    pub async fn get_author_texts(&self) -> Result<Vec<TextDto>> {
        let texts = self
            .repo
            .get_texts_by_category(TextCategory::Author)
            .await?;
        Ok(texts
            .into_iter()
            .map(|t| TextDto {
                id: t.id.to_string(),
                content: t.content,
            })
            .collect())
    }

    pub async fn get_workshop_items(&self) -> Result<Vec<WorkshopItemDto>> {
        let texts = self
            .repo
            .get_texts_by_category(TextCategory::Workshop)
            .await?;
        Ok(texts
            .into_iter()
            .map(|t| {
                let t_id_str = t.id.to_string();
                WorkshopItemDto {
                    id: t_id_str.clone(),
                    content: t.content,
                    caption: t.caption,
                    image_url: t
                        .image_path
                        .as_ref()
                        .map(|p| self.resolve_url(p, "texts", &t_id_str)),
                }
            })
            .collect())
    }

    pub async fn get_cabinet_zones(&self) -> Result<Vec<CabinetZoneDto>> {
        let zones = self.repo.get_zones().await?;
        Ok(zones
            .into_iter()
            .map(|z| CabinetZoneDto {
                id: z.id.to_string(),
                zone_type: z.zone_type,
                x: z.x_percent,
                y: z.y_percent,
                width: z.width_percent,
                height: z.height_percent,
                target_route: z.target_route,
            })
            .collect())
    }

    // === ADMIN WRITE ===

    pub async fn save_figurine(&self, mut req: crate::models::SaveFigurineRequest) -> Result<()> {
        let figurine_id = Self::parse_uuid(&req.id)?;
        validate_text("Name", &req.name, 200)?;
        if req.images.len() > 50 {
            return Err(AppError::BadRequest("Too many images (max 50)".into()));
        }
        if req.process_steps.len() > 50 {
            return Err(AppError::BadRequest(
                "Too many process steps (max 50)".into(),
            ));
        }
        for image in &req.images {
            if let Some(value) = image.parallax_intensity
                && !(0.0..=1.0).contains(&value)
            {
                return Err(AppError::BadRequest(
                    "Image parallax intensity must be between 0 and 1".into(),
                ));
            }
            for value in [
                image.focal_x,
                image.focal_y,
                image.reveal_radius,
                image.darkness,
            ]
            .into_iter()
            .flatten()
            {
                if !(0.0..=1.0).contains(&value) {
                    return Err(AppError::BadRequest(
                        "Image focal point / reveal radius / darkness must be between 0 and 1".into(),
                    ));
                }
            }
        }
        for value in [req.open_from_min, req.open_until_min].into_iter().flatten() {
            if !(0..=1439).contains(&value) {
                return Err(AppError::BadRequest(
                    "Showing window minutes must be between 0 and 1439".into(),
                ));
            }
        }
        // Resolve the URL slug: honour a non-blank admin override, otherwise
        // transliterate the name (see slug_base).
        let base = Self::slug_base(req.slug.as_deref(), &req.name, &req.id);
        let prev = self.repo.get_figurine_by_id(figurine_id).await?;
        let prev_status = prev.as_ref().map(|f| f.status.clone());
        // Manual vs auto: the figurine form round-trips the stored slug on every
        // save, so "override present" alone can't mean "hand-typed". A blank field
        // is an explicit auto (regenerate from name); a value equal to what is
        // already stored preserves the prior flag (the form just echoed it back);
        // anything else is a fresh hand-typed slug → manual.
        let incoming = req.slug.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let slug_manual = match (incoming, prev.as_ref()) {
            (None, _) => false,
            (Some(s), Some(p)) if Some(s) == p.slug.as_deref() => p.slug_manual,
            (Some(_), _) => true,
        };

        // Enforce uniqueness — a slug must resolve to exactly one work. Append
        // -2, -3, … past any collision with a *different* work.
        //
        // The check-then-insert is a TOCTOU window: two admins saving at once can
        // both pick the same free candidate, and the partial UNIQUE(slug) index
        // then rejects the loser's write with a raw unique-violation. Rather than
        // surfacing that as a 500, catch it and resume the suffix search from where
        // we left off, retrying the save with the next free candidate.
        let mut n = 2;
        loop {
            let mut candidate = base.clone();
            while let Some(existing) = self.repo.get_figurine_by_slug(&candidate).await? {
                if existing.id == figurine_id {
                    break; // already ours
                }
                candidate = format!("{base}-{n}");
                n += 1;
            }
            req.slug = Some(candidate);

            match self
                .repo
                .save_figurine_full(&req, &req.images, &req.process_steps, slug_manual)
                .await
            {
                Ok(()) => break,
                Err(AppError::Database(sqlx::Error::Database(db_err)))
                    if db_err.is_unique_violation() && n <= 1000 =>
                {
                    // A concurrent save claimed our slug between check and insert.
                    // Bump past it and try the next candidate.
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        // Just flipped to available → alert the author to everyone who was waiting
        // (queue + notify-me), so they can reach out personally.
        let became_available = req.status == crate::models::FigurineStatus::Available
            && prev_status
                .as_ref()
                .is_some_and(|s| *s != crate::models::FigurineStatus::Available);
        if became_available {
            let svc = self.clone();
            let name = req.name.clone();
            tokio::spawn(async move {
                let _ = svc.send_availability_digest(figurine_id, &name).await;
            });
        }
        Self::log_domain_event("figurine_saved", "figurine", figurine_id, "ok");
        Ok(())
    }

    /// Compute the slug *base* for a work (before uniqueness suffixing): a
    /// sanitised non-blank admin override, else the transliterated name, else the
    /// id's first block when the name has no usable characters. slugify() also
    /// sanitises a hand-typed override down to `[a-z0-9-]`.
    fn slug_base(override_opt: Option<&str>, name: &str, id: &str) -> String {
        let base = override_opt
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(crate::slug::slugify)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| crate::slug::slugify(name));
        if base.is_empty() {
            id.split('-').next().unwrap_or(id).to_string()
        } else {
            base
        }
    }

    /// Resolve a collision-free slug from `base` and persist it on the work (with
    /// its manual/auto flag), retrying past the check-then-write TOCTOU window if a
    /// concurrent write claims the candidate first. Returns the slug actually stored.
    async fn assign_unique_slug(
        &self,
        figurine_id: Uuid,
        base: &str,
        slug_manual: bool,
    ) -> Result<String> {
        let mut n = 2;
        loop {
            let mut candidate = base.to_string();
            while let Some(existing) = self.repo.get_figurine_by_slug(&candidate).await? {
                if existing.id == figurine_id {
                    break; // already ours
                }
                candidate = format!("{base}-{n}");
                n += 1;
            }
            match self
                .repo
                .update_figurine_slug(figurine_id, &candidate, slug_manual)
                .await
            {
                Ok(()) => return Ok(candidate),
                Err(AppError::Database(sqlx::Error::Database(db_err)))
                    if db_err.is_unique_violation() && n <= 1000 =>
                {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Backfill: give every work still missing a URL slug a transliterated one.
    /// Idempotent — works that already have a slug are left untouched. Backfilled
    /// slugs are auto (name-derived). Returns the number of works that got one.
    pub async fn backfill_figurine_slugs(&self) -> Result<crate::models::BulkOpSummary> {
        let works = self.repo.get_figurines_without_slug().await?;
        let mut affected = 0u64;
        for work in works {
            let base = Self::slug_base(None, &work.name, &work.id.to_string());
            self.assign_unique_slug(work.id, &base, false).await?;
            affected += 1;
        }
        Self::log_domain_event("slugs_backfilled", "figurine", affected, "ok");
        Ok(crate::models::BulkOpSummary { affected })
    }

    /// Set or regenerate a single work's URL slug. A non-blank `slug` is honoured
    /// as a hand-typed override (sanitised) → flagged manual; a blank/None one
    /// regenerates from the work's name → flagged auto. Uniqueness is enforced.
    /// Returns the slug actually stored.
    pub async fn set_figurine_slug(&self, id: String, slug: Option<String>) -> Result<String> {
        let figurine_id = Self::parse_uuid(&id)?;
        let work = self
            .repo
            .get_figurine_by_id(figurine_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Figurine {} not found", id)))?;
        let manual = slug.as_deref().map(str::trim).is_some_and(|s| !s.is_empty());
        let base = Self::slug_base(slug.as_deref(), &work.name, &id);
        let stored = self.assign_unique_slug(figurine_id, &base, manual).await?;
        Self::log_domain_event("figurine_slug_set", "figurine", figurine_id, "ok");
        Ok(stored)
    }

    pub async fn delete_figurine(&self, id: String) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;

        // Collect image paths before CASCADE removes the rows.
        let images = self.repo.get_images_by_figurine(uuid).await?;

        // figurine_analytics_events has no FK (by design), so delete manually.
        self.repo.delete_analytics_events_by_figurine(uuid).await?;

        // DELETE cascades to: images, process_steps, figurine_showings,
        // figurine_bookings, figurine_comments, figurine_waitlist,
        // figurine_analytics_daily, figurine_analytics_sources_daily.
        self.repo.delete_figurine(uuid).await?;

        // Remove physical files from disk (skip remote http URLs), including the
        // derived renditions that are NOT stored on the image row. The client
        // derives those URLs by rewriting the path, so save_image_variants writes
        // seven files per upload (original + preview/medium/thumb, each JPEG+WebP)
        // while only the preview/original/thumb JPEGs live in the DB. Removing only
        // the stored three leaked preview.webp / medium.* / thumb.webp on every
        // delete — collect the full set here. Uses tokio::fs so the (possibly many)
        // unlinks never block a runtime worker.
        let webp_sibling = |p: &str| -> Option<String> {
            p.strip_suffix(".jpg")
                .or_else(|| p.strip_suffix(".jpeg"))
                .map(|stem| format!("{stem}.webp"))
        };
        let mut rels: Vec<String> = Vec::new();
        for img in &images {
            for p in [
                Some(img.file_path.clone()),
                img.original_path.clone(),
                img.thumb_path.clone(),
                img.depth_path.clone(),
            ]
            .into_iter()
            .flatten()
            {
                rels.push(p);
            }
            // Preview path → its medium rendition and every WebP sibling.
            let preview = img.file_path.clone();
            let medium = preview.replace("/preview/", "/medium/");
            for jpg in [preview.clone(), medium.clone()] {
                if let Some(w) = webp_sibling(&jpg) {
                    rels.push(w);
                }
            }
            if medium != preview {
                rels.push(medium);
            }
            if let Some(thumb) = &img.thumb_path
                && let Some(w) = webp_sibling(thumb)
            {
                rels.push(w);
            }
        }
        rels.sort();
        rels.dedup();
        for rel in rels {
            if rel.starts_with("http") {
                continue;
            }
            let path = Path::new(&self.config.upload_dir).join(rel.trim_start_matches('/'));
            let _ = tokio::fs::remove_file(&path).await;
        }

        Self::log_domain_event("figurine_deleted", "figurine", uuid, "ok");
        Ok(())
    }

    pub async fn save_zone(&self, req: crate::models::SaveZoneRequest) -> Result<()> {
        let zone_id = req.id.clone();
        let count = self.repo.get_zone_count().await?;
        self.repo.upsert_zone(&req, count).await?;
        Self::log_domain_event("zone_saved", "zone", zone_id, "ok");
        Ok(())
    }

    pub async fn delete_zone(&self, id: String) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        self.repo.delete_zone(uuid).await?;
        Self::log_domain_event("zone_deleted", "zone", uuid, "ok");
        Ok(())
    }

    pub async fn get_showing_rooms(&self) -> Result<Vec<crate::models::ShowingRoomDto>> {
        let rooms = self.repo.get_showing_rooms().await?;
        Ok(rooms
            .into_iter()
            .map(|r| crate::models::ShowingRoomDto {
                id: r.id.to_string(),
                name: r.name,
                open_from_min: r.open_from_min,
                open_until_min: r.open_until_min,
                open_days_mask: r.open_days_mask,
                open_month_day: r.open_month_day,
                open_date_from: r.open_date_from,
                open_date_until: r.open_date_until,
            })
            .collect())
    }

    pub async fn save_showing_room(
        &self,
        req: crate::models::SaveShowingRoomRequest,
    ) -> Result<()> {
        let room_id = req.id.clone();
        let count = self.repo.get_showing_room_count().await?;
        self.repo.upsert_showing_room(&req, count).await?;
        Self::log_domain_event("showing_room_saved", "showing_room", room_id, "ok");
        Ok(())
    }

    pub async fn delete_showing_room(&self, id: String) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        self.repo.delete_showing_room(uuid).await?;
        Self::log_domain_event("showing_room_deleted", "showing_room", uuid, "ok");
        Ok(())
    }

    pub async fn save_text(
        &self,
        category: crate::models::TextCategory,
        req: crate::models::SaveTextRequest,
    ) -> Result<()> {
        let text_id = req.id.clone();
        self.repo.upsert_text(&req, &category).await?;
        Self::log_domain_event("text_saved", "text", text_id, "ok");
        Ok(())
    }

    pub async fn delete_text_item(&self, id: String) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        self.repo.delete_text(uuid).await?;
        Self::log_domain_event("text_deleted", "text", uuid, "ok");
        Ok(())
    }

    pub async fn get_background(&self) -> Result<Option<String>> {
        let path = self.repo.get_main_background().await?;
        Ok(path.map(|p| {
            let base = self.config.public_url.trim_end_matches('/');
            if p.starts_with("http") {
                p
            } else if p.starts_with("/static/") {
                format!("{}{}", base, p)
            } else {
                format!("{}/static/{}", base, p.trim_start_matches('/'))
            }
        }))
    }

    pub async fn set_background(&self, url: String) -> Result<()> {
        self.repo.set_main_background(&url).await?;
        Self::log_domain_event("background_saved", "setting", "main_background", "ok");
        Ok(())
    }

    pub async fn get_home_content(&self) -> Result<HomeContent> {
        Ok(self.repo.get_home_content().await?.unwrap_or_default())
    }

    pub async fn save_home_content(&self, content: HomeContent) -> Result<()> {
        self.repo.save_home_content(&content).await?;
        Self::log_domain_event("home_content_saved", "setting", "home_content", "ok");
        Ok(())
    }

    // === AUTHOR PROFILE ===

    pub async fn get_author_profile(&self) -> Result<AuthorProfile> {
        Ok(self.repo.get_author_profile().await?.unwrap_or_default())
    }

    pub async fn save_author_profile(&self, profile: AuthorProfile) -> Result<()> {
        self.repo.save_author_profile(&profile).await?;
        Self::log_domain_event("author_profile_saved", "setting", "author_profile", "ok");
        Ok(())
    }

    // === ORDERS / NOTIFICATIONS ===

    pub async fn create_order(&self, order: &OrderRequest, user_id: Option<Uuid>) -> Result<Order> {
        if order.mode == OrderMode::Reserve {
            let figurine_id = uuid::Uuid::parse_str(&order.figurine_id)
                .map_err(|_| AppError::BadRequest("Invalid figurine ID".to_string()))?;
            let figurine = self
                .repo
                .get_figurine_by_id(figurine_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Figurine not found".to_string()))?;
            if figurine.status != FigurineStatus::Available {
                return Err(AppError::BadRequest(
                    "Reserve requests are available only for available works".to_string(),
                ));
            }
        }

        // Notify-me subscriptions are deduplicated and carry a cancel token;
        // request/question are plain one-off messages.
        let saved = if order.mode == OrderMode::Notify {
            self.repo.upsert_notify_order(order, user_id).await?
        } else {
            self.repo.save_order(order, user_id).await?
        };
        self.observability
            .record_business_event("order_created", "ok");
        Self::log_domain_event("order_created", "order", saved.id, "ok");
        {
            let svc = self.clone();
            let saved = saved.clone();
            tokio::spawn(async move {
                let _ = svc.send_order_notification(&saved).await;
            });
        }
        Ok(saved)
    }

    pub async fn link_orders_to_user(&self, user_id: Uuid, email: &str) -> Result<u64> {
        self.repo.link_orders_to_user(user_id, email).await
    }

    /// View a notify subscription by its token (visitor's receipt lookup).
    pub async fn get_notify_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::NotifyInfo>> {
        let Some(o) = self.repo.get_order_by_cancel_token(token).await? else {
            return Ok(None);
        };
        Ok(Some(crate::models::NotifyInfo {
            figurine_id: o.figurine_id,
            figurine_name: o.figurine_name,
        }))
    }

    /// Stop a notify subscription by token. Idempotent.
    pub async fn cancel_notify_by_token(&self, token: &str) -> Result<()> {
        self.repo.delete_order_by_cancel_token(token).await
    }

    pub async fn list_orders(
        &self,
        status_filter: Option<&str>,
        mode_filter: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<OrdersPage> {
        let offset = (page - 1) * per_page;
        let (items, total) = self
            .repo
            .get_orders_page(status_filter, mode_filter, per_page, offset)
            .await?;
        let new_count = self.repo.get_new_orders_count().await?;
        Ok(OrdersPage {
            items,
            total,
            new_count,
            page,
            per_page,
        })
    }

    pub async fn update_order_status(
        &self,
        id: uuid::Uuid,
        body: &UpdateOrderStatusRequest,
    ) -> Result<()> {
        self.repo
            .update_order_status(
                id,
                &body.status,
                body.admin_notes.as_deref(),
                body.reserve_status.as_ref(),
                body.reserve_expires_at.as_deref(),
                body.admin_terms_note.as_deref(),
                body.invoice_note.as_deref(),
            )
            .await?;
        Self::log_domain_event("order_status_updated", "order", id, "ok");
        if body.status == OrderStatus::Replied
            && let Ok(Some(order)) = self.repo.get_order_by_id(id).await
            && let Some(user_id) = order.user_id
        {
            let subject = format!("Ответ на ваш запрос — {}", order.figurine_name);
            let body = match order.admin_notes.as_deref() {
                Some(n) if !n.is_empty() => format!(
                    "Ваш запрос по «{}» получил ответ.\n\n{}",
                    order.figurine_name, n
                ),
                _ => format!("Ваш запрос по «{}» получил ответ.", order.figurine_name),
            };
            let _ = self
                .repo
                .create_thread(user_id, "order", Some(order.id), &subject, &body, true, &[])
                .await;
        }
        Ok(())
    }

    fn certificate_dto(order: &Order) -> Option<CollectorCertificateDto> {
        let token = order.certificate_token.clone()?;
        let certificate_number = order.certificate_number.clone()?;
        let issued_at = order.certificate_issued_at?;
        Some(CollectorCertificateDto {
            token,
            certificate_number,
            figurine_id: order.figurine_id.clone(),
            figurine_name: order.figurine_name.clone(),
            order_id: order.id.to_string(),
            issued_at: issued_at.to_rfc3339(),
            revoked_at: order.certificate_revoked_at.map(|d| d.to_rfc3339()),
        })
    }

    pub async fn issue_order_certificate(&self, id: uuid::Uuid) -> Result<CollectorCertificateDto> {
        let token = format!("cert_{}", uuid::Uuid::new_v4().simple());
        let certificate_number = format!(
            "CERT-{}",
            uuid::Uuid::new_v4().simple().to_string()[..8].to_uppercase()
        );
        let order = self
            .repo
            .issue_order_certificate(id, &token, &certificate_number)
            .await?;
        Self::log_domain_event("order_certificate_issued", "order", id, "ok");
        Self::certificate_dto(&order)
            .ok_or_else(|| AppError::Internal("Issued certificate is incomplete".to_string()))
    }

    pub async fn revoke_order_certificate(
        &self,
        id: uuid::Uuid,
    ) -> Result<CollectorCertificateDto> {
        let order = self.repo.revoke_order_certificate(id).await?;
        Self::log_domain_event("order_certificate_revoked", "order", id, "ok");
        Self::certificate_dto(&order)
            .ok_or_else(|| AppError::Internal("Revoked certificate is incomplete".to_string()))
    }

    fn commission_certificate_dto(c: &Commission) -> Option<CollectorCertificateDto> {
        let token = c.certificate_token.clone()?;
        let certificate_number = c.certificate_number.clone()?;
        let issued_at = c.certificate_issued_at?;
        Some(CollectorCertificateDto {
            token,
            certificate_number,
            figurine_id: c.figurine_id.clone().unwrap_or_default(),
            figurine_name: c.title.clone(),
            order_id: c.id.to_string(),
            issued_at: issued_at.to_rfc3339(),
            revoked_at: c.certificate_revoked_at.map(|d| d.to_rfc3339()),
        })
    }

    pub async fn issue_commission_certificate(
        &self,
        id: uuid::Uuid,
    ) -> Result<CollectorCertificateDto> {
        let token = format!("cert_{}", uuid::Uuid::new_v4().simple());
        let certificate_number = format!(
            "CERT-{}",
            uuid::Uuid::new_v4().simple().to_string()[..8].to_uppercase()
        );
        let commission = self
            .repo
            .issue_commission_certificate(id, &token, &certificate_number)
            .await?;
        Self::log_domain_event("commission_certificate_issued", "commission", id, "ok");
        Self::commission_certificate_dto(&commission)
            .ok_or_else(|| AppError::Internal("Issued certificate is incomplete".to_string()))
    }

    pub async fn revoke_commission_certificate(
        &self,
        id: uuid::Uuid,
    ) -> Result<CollectorCertificateDto> {
        let commission = self.repo.revoke_commission_certificate(id).await?;
        Self::log_domain_event("commission_certificate_revoked", "commission", id, "ok");
        Self::commission_certificate_dto(&commission)
            .ok_or_else(|| AppError::Internal("Revoked certificate is incomplete".to_string()))
    }

    pub async fn get_public_certificate(
        &self,
        token: &str,
    ) -> Result<Option<PublicCertificateDto>> {
        // A public token may belong to an order (reserve) or a commission.
        let cert = if let Some(order) = self.repo.get_order_by_certificate_token(token).await? {
            Self::certificate_dto(&order)
        } else if let Some(commission) =
            self.repo.get_commission_by_certificate_token(token).await?
        {
            Self::commission_certificate_dto(&commission)
        } else {
            None
        };
        let Some(c) = cert else {
            return Ok(None);
        };
        Ok(Some(PublicCertificateDto {
            token: c.token,
            certificate_number: c.certificate_number,
            figurine_id: c.figurine_id,
            figurine_name: c.figurine_name,
            issued_at: c.issued_at,
            revoked: c.revoked_at.is_some(),
        }))
    }

    pub async fn get_user_certificates(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<CollectorCertificateDto>> {
        let orders = self.repo.get_user_certificate_orders(user_id).await?;
        let commissions = self.repo.get_user_certificate_commissions(user_id).await?;
        let mut certs: Vec<CollectorCertificateDto> =
            orders.iter().filter_map(Self::certificate_dto).collect();
        certs.extend(
            commissions
                .iter()
                .filter_map(Self::commission_certificate_dto),
        );
        Ok(certs)
    }

    async fn send_order_notification(&self, order: &Order) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let mode_label = match order.mode {
            OrderMode::Request => "🛒 Запрос на покупку",
            OrderMode::Question => "❓ Вопрос",
            OrderMode::Notify => "🔔 Уведомить о наличии",
            OrderMode::Reserve => "🔒 Запрос резерва",
        };

        let admin_link = format!(
            "{}/admin#orders",
            self.config.public_url.trim_end_matches('/')
        );

        let text = format!(
            "{}\n\n\
            🏺 Фигурка: {}\n\
            👤 Имя: {}\n\
            📧 Email: {}\n\
            💬 Сообщение: {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(mode_label),
            escape_markdown(&order.figurine_name),
            escape_markdown(&order.requester_name),
            escape_markdown(&order.requester_email),
            escape_markdown(order.message.as_deref().unwrap_or("—")),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let client = self.http_client.clone();
        let _ = client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "MarkdownV2"
            }))
            .send()
            .await;

        Ok(())
    }

    // === COMMISSIONS ===

    async fn commission_to_dto(&self, c: &Commission) -> Result<CommissionDto> {
        let attachments = self.repo.get_commission_attachments(c.id).await?;
        let thread = self
            .repo
            .find_thread_by_reference(c.id, "commission")
            .await?;
        Ok(CommissionDto {
            id: c.id.to_string(),
            claim_token: c.claim_token.clone(),
            requester_name: c.requester_name.clone(),
            requester_email: c.requester_email.clone(),
            requester_phone: c.requester_phone.clone(),
            title: c.title.clone(),
            description: c.description.clone(),
            size_note: c.size_note.clone(),
            mood: c.mood.clone(),
            deadline: c.deadline.map(|d| d.to_string()),
            budget_note: c.budget_note.clone(),
            occasion: c.occasion.clone(),
            source_figurine_id: c.source_figurine_id.clone(),
            similar_keep_note: c.similar_keep_note.clone(),
            similar_change_note: c.similar_change_note.clone(),
            similar_tags: c.similar_tags.clone(),
            figurine_id: c.figurine_id.clone(),
            status: c.status.clone(),
            admin_notes: c.admin_notes.clone(),
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
            attachments: attachments.iter().map(AttachmentDto::from).collect(),
            thread_id: thread.map(|t| t.id.to_string()),
            started: c.status.is_started(),
            certificate: Self::commission_certificate_dto(c),
        })
    }

    pub async fn create_commission(
        &self,
        req: &CommissionRequest,
    ) -> Result<CommissionCreatedResponse> {
        validate_attachments(&req.attachment_urls)?;
        if let Some(source_id) = req
            .source_figurine_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            let uuid = Self::parse_uuid(source_id)?;
            if self.repo.get_figurine_by_id(uuid).await?.is_none() {
                return Err(crate::error::AppError::BadRequest(
                    "Source figurine not found.".into(),
                ));
            }
        }
        let saved = self.repo.create_commission(req).await?;
        self.observability
            .record_business_event("commission_created", "ok");
        Self::log_domain_event("commission_created", "commission", saved.id, "ok");
        {
            let svc = self.clone();
            let saved = saved.clone();
            tokio::spawn(async move {
                let _ = svc.send_commission_notification(&saved).await;
            });
        }
        Ok(CommissionCreatedResponse {
            id: saved.id.to_string(),
            claim_token: saved.claim_token,
        })
    }

    pub async fn get_commission_by_token(&self, token: &str) -> Result<Option<CommissionDto>> {
        match self.repo.get_commission_by_token(token).await? {
            Some(c) => Ok(Some(self.commission_to_dto(&c).await?)),
            None => Ok(None),
        }
    }

    pub async fn claim_commission(&self, token: &str, user_id: Uuid) -> Result<CommissionDto> {
        let commission = self
            .repo
            .claim_commission(token, user_id)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound("Commission not found".to_string()))?;

        // Seed a conversation thread with the original request, once.
        if self
            .repo
            .find_thread_by_reference(commission.id, "commission")
            .await?
            .is_none()
        {
            let subject = if !commission.title.trim().is_empty() {
                commission.title.clone()
            } else if commission.lang == "en" {
                "A petition for a new figurine".to_string()
            } else {
                "Прошение о новой фигурке".to_string()
            };
            let _ = self
                .repo
                .create_thread(
                    user_id,
                    "commission",
                    Some(commission.id),
                    &subject,
                    &commission.description,
                    false,
                    &[],
                )
                .await;
        }

        Self::log_domain_event("commission_claimed", "commission", commission.id, "ok");
        self.commission_to_dto(&commission).await
    }

    /// Edit a petition's content. `owner` limits the action to the petition's
    /// author (None ⇒ admin). Refused once work has started.
    pub async fn edit_commission(
        &self,
        id: Uuid,
        owner: Option<Uuid>,
        req: &EditCommissionRequest,
    ) -> Result<CommissionDto> {
        let existing =
            self.repo.get_commission_by_id(id).await?.ok_or_else(|| {
                crate::error::AppError::NotFound("Commission not found".to_string())
            })?;
        if let Some(uid) = owner
            && existing.user_id != Some(uid)
        {
            return Err(crate::error::AppError::Unauthorized);
        }
        if existing.status.is_started() {
            return Err(crate::error::AppError::BadRequest(
                "Work has already begun on this petition — it can no longer be edited.".into(),
            ));
        }
        if let Some(atts) = &req.attachment_urls {
            validate_attachments(atts)?;
        }
        let updated = self
            .repo
            .update_commission_content(id, req)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound("Commission not found".to_string()))?;
        Self::log_domain_event("commission_edited", "commission", id, "ok");
        self.commission_to_dto(&updated).await
    }

    /// Delete a petition. `owner` limits the action to its author (None ⇒ admin).
    /// Refused once work has started.
    pub async fn delete_commission(&self, id: Uuid, owner: Option<Uuid>) -> Result<()> {
        let existing =
            self.repo.get_commission_by_id(id).await?.ok_or_else(|| {
                crate::error::AppError::NotFound("Commission not found".to_string())
            })?;
        if let Some(uid) = owner
            && existing.user_id != Some(uid)
        {
            return Err(crate::error::AppError::Unauthorized);
        }
        if existing.status.is_started() {
            return Err(crate::error::AppError::BadRequest(
                "Work has already begun on this petition — it can no longer be deleted.".into(),
            ));
        }

        // When the master removes a claimed petition, the petitioner must be told.
        // The commission's own conversation is cascade-deleted with it, so the
        // notice goes into a separate, persistent system thread (owner = None ⇒ admin).
        if owner.is_none()
            && let Some(user_id) = existing.user_id
        {
            let en = existing.lang == "en";
            let titled = !existing.title.trim().is_empty();
            let subject = if en {
                "Petition removed"
            } else {
                "Прошение снято"
            };
            let body = if en {
                if titled {
                    format!(
                        "Your petition “{}” has been removed by the archive keeper. You are welcome to send a new one.",
                        existing.title
                    )
                } else {
                    "Your petition has been removed by the archive keeper. You are welcome to send a new one.".to_string()
                }
            } else if titled {
                format!(
                    "Ваше прошение «{}» снято хранителем архива. Вы можете отправить новое.",
                    existing.title
                )
            } else {
                "Ваше прошение снято хранителем архива. Вы можете отправить новое.".to_string()
            };
            let _ = self
                .repo
                .create_thread(user_id, "system", None, subject, &body, true, &[])
                .await;
        }

        self.repo.delete_commission(id).await?;
        Self::log_domain_event("commission_deleted", "commission", id, "ok");
        Ok(())
    }

    pub async fn list_commissions(
        &self,
        status_filter: Option<&str>,
        similar_only: bool,
        page: i64,
        per_page: i64,
    ) -> Result<CommissionsPage> {
        let offset = (page - 1) * per_page;
        let (items, total) = self
            .repo
            .get_commissions_page(status_filter, similar_only, per_page, offset)
            .await?;
        let new_count = self.repo.get_new_commissions_count().await?;
        let mut dtos = Vec::with_capacity(items.len());
        for c in &items {
            dtos.push(self.commission_to_dto(c).await?);
        }
        Ok(CommissionsPage {
            items: dtos,
            total,
            new_count,
            page,
            per_page,
        })
    }

    pub async fn get_user_commissions(&self, user_id: Uuid) -> Result<Vec<CommissionDto>> {
        let items = self.repo.get_user_commissions(user_id).await?;
        let mut dtos = Vec::with_capacity(items.len());
        for c in &items {
            dtos.push(self.commission_to_dto(c).await?);
        }
        Ok(dtos)
    }

    /// Adopt unclaimed petitions sent from this account's email. Each goes through
    /// claim_commission so its conversation thread is seeded exactly as on manual
    /// claim. Covers guest petitions and the case where only the last claim token
    /// survived in the browser's localStorage.
    pub async fn adopt_commissions_by_email(&self, user_id: Uuid, email: &str) -> Result<()> {
        let tokens = self.repo.orphan_commission_tokens_by_email(email).await?;
        for token in tokens {
            let _ = self.claim_commission(&token, user_id).await;
        }
        Ok(())
    }

    pub async fn update_commission(
        &self,
        id: Uuid,
        status: &CommissionStatus,
        admin_notes: Option<&str>,
        figurine_id: Option<&str>,
    ) -> Result<Option<CommissionDto>> {
        let updated = self
            .repo
            .update_commission(id, status, admin_notes, figurine_id)
            .await?;
        if let Some(ref c) = updated {
            Self::log_domain_event("commission_status_updated", "commission", c.id, "ok");
            // If the petitioner has an account, drop a note into their conversation,
            // in the language they wrote the petition in.
            if let Some(user_id) = c.user_id {
                let en = c.lang == "en";
                let label = match status {
                    CommissionStatus::Accepted => Some(if en {
                        "Your petition is accepted — the master takes up the work."
                    } else {
                        "Ваше прошение принято — мастер берётся за работу."
                    }),
                    CommissionStatus::InProgress => Some(if en {
                        "The master has begun your figurine."
                    } else {
                        "Мастер приступил к вашей фигурке."
                    }),
                    CommissionStatus::Completed => Some(if en {
                        "Your figurine is finished."
                    } else {
                        "Ваша фигурка завершена."
                    }),
                    CommissionStatus::Declined => Some(if en {
                        "Regrettably, the master will not take up this petition."
                    } else {
                        "К сожалению, мастер не возьмётся за это прошение."
                    }),
                    _ => None,
                };
                if let Some(text) = label {
                    let body = match admin_notes {
                        Some(n) if !n.trim().is_empty() => format!("{}\n\n{}", text, n),
                        _ => text.to_string(),
                    };
                    if let Some(thread) = self
                        .repo
                        .find_thread_by_reference(c.id, "commission")
                        .await?
                    {
                        let _ = self
                            .repo
                            .add_thread_reply(thread.id, uuid::Uuid::nil(), true, &body, &[])
                            .await;
                    } else {
                        let subject = if !c.title.trim().is_empty() {
                            c.title.clone()
                        } else if en {
                            "Your petition".to_string()
                        } else {
                            "Ваше прошение".to_string()
                        };
                        let _ = self
                            .repo
                            .create_thread(
                                user_id,
                                "commission",
                                Some(c.id),
                                &subject,
                                &body,
                                true,
                                &[],
                            )
                            .await;
                    }
                }
            }
            Ok(Some(self.commission_to_dto(c).await?))
        } else {
            Ok(None)
        }
    }

    async fn send_commission_notification(&self, c: &Commission) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let admin_link = format!(
            "{}/admin#commissions",
            self.config.public_url.trim_end_matches('/')
        );
        let title = if c.title.trim().is_empty() {
            "—"
        } else {
            c.title.as_str()
        };
        let text = format!(
            "🗝 Новое прошение о фигурке\n\n\
            ✒️ Идея: {}\n\
            📝 Описание: {}\n\
            👤 Имя: {}\n\
            📧 Email: {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(title),
            escape_markdown(&c.description),
            escape_markdown(&c.requester_name),
            escape_markdown(&c.requester_email),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let client = self.http_client.clone();
        let _ = client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "MarkdownV2",
            }))
            .send()
            .await;
        Ok(())
    }

    // === SHOWINGS & BOOKINGS (PUBLIC) ===

    pub async fn get_figurine_schedule(&self, figurine_id: String) -> Result<FigurineScheduleDto> {
        let uuid = self.resolve_figurine_uuid(&figurine_id).await?;
        let (showings, confirmed, pending) = self.repo.get_figurine_schedule(uuid).await?;

        let mut entries: Vec<ScheduleEntryDto> = Vec::new();

        for s in showings {
            entries.push(ScheduleEntryDto {
                entry_type: "showing".to_string(),
                title: Some(s.title),
                showing_type: Some(s.showing_type),
                venue: s.venue,
                starts_at: s.starts_at.to_string(),
                ends_at: s.ends_at.to_string(),
            });
        }

        for b in confirmed {
            entries.push(ScheduleEntryDto {
                entry_type: "booking".to_string(),
                title: None,
                showing_type: None,
                venue: None,
                starts_at: b.starts_at.to_string(),
                ends_at: b.ends_at.to_string(),
            });
        }

        for b in pending {
            entries.push(ScheduleEntryDto {
                entry_type: "pending".to_string(),
                title: None,
                showing_type: None,
                venue: None,
                starts_at: b.starts_at.to_string(),
                ends_at: b.ends_at.to_string(),
            });
        }

        entries.sort_by(|a, b| a.starts_at.cmp(&b.starts_at));
        Ok(FigurineScheduleDto { entries })
    }

    pub async fn get_booking_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::BookingCancelInfo>> {
        Ok(self
            .repo
            .get_booking_by_cancel_token(token)
            .await?
            .map(|b| crate::models::BookingCancelInfo {
                figurine_name: b.figurine_name,
                figurine_id: b.figurine_id.to_string(),
                starts_at: b.starts_at.to_string(),
                ends_at: b.ends_at.to_string(),
                status: b.status,
                admin_notes: b.admin_notes,
                curator_conditions: b.curator_conditions,
            }))
    }

    /// Batch variant — returns a map keyed by cancel token. Missing/invalid tokens are
    /// simply absent from the result (same "not found = omitted" semantics as the single GET).
    pub async fn get_bookings_by_tokens(
        &self,
        tokens: &[String],
    ) -> Result<std::collections::HashMap<String, crate::models::BookingCancelInfo>> {
        let bookings = self.repo.get_bookings_by_cancel_tokens(tokens).await?;
        Ok(bookings
            .into_iter()
            .map(|b| {
                (
                    b.cancel_token.clone(),
                    crate::models::BookingCancelInfo {
                        figurine_name: b.figurine_name,
                        figurine_id: b.figurine_id.to_string(),
                        starts_at: b.starts_at.to_string(),
                        ends_at: b.ends_at.to_string(),
                        status: b.status,
                        admin_notes: b.admin_notes,
                        curator_conditions: b.curator_conditions,
                    },
                )
            })
            .collect())
    }

    pub async fn cancel_booking_by_token(&self, token: &str) -> Result<()> {
        let booking = self.repo.cancel_booking_by_token(token).await?;
        if let Some(b) = booking {
            // If this was the only confirmed booking, revert figurine to available.
            // (token cancellation only works on 'pending' rows, so figurine status stays unchanged here.)
            let _ = b; // booking was pending — no status revert needed
        }
        // If None → booking not found or already not pending — treat as no-op (idempotent)
        Ok(())
    }

    /// Booking-rule validation (duration + advance notice), shared between the
    /// initial create and reschedule so a booking can't be created that could
    /// never be rescheduled.
    fn validate_booking_rules(
        rules: &BookingRules,
        starts_at: chrono::NaiveDate,
        ends_at: chrono::NaiveDate,
    ) -> Result<()> {
        let duration = (ends_at - starts_at).num_days() + 1;
        if duration < rules.min_days {
            return Err(AppError::BadRequest(format!(
                "Minimum booking duration is {} day(s)",
                rules.min_days
            )));
        }
        if duration > rules.max_days {
            return Err(AppError::BadRequest(format!(
                "Maximum booking duration is {} day(s)",
                rules.max_days
            )));
        }
        if rules.advance_days > 0 {
            let earliest =
                chrono::Utc::now().date_naive() + chrono::Duration::days(rules.advance_days);
            if starts_at < earliest {
                return Err(AppError::BadRequest(format!(
                    "Booking must start at least {} day(s) in advance",
                    rules.advance_days
                )));
            }
        }
        Ok(())
    }

    /// Set or clear a visitor's wax-seal mark. The token is opaque client
    /// state, not a credential, but we still bound its length to keep the
    /// column and any future index sane against a malicious/broken client.
    /// Returns the tone that ended up applied (`None` if cleared).
    pub async fn set_figurine_mark(
        &self,
        figurine_id: Uuid,
        visitor_token: &str,
        tone: Option<&str>,
    ) -> Result<Option<String>> {
        let token = visitor_token.trim();
        if token.is_empty() || token.len() > 64 {
            return Err(crate::error::AppError::BadRequest(
                "Invalid visitor token".to_string(),
            ));
        }
        let tone = match tone {
            Some(t) if MARK_TONES.contains(&t) => Some(t),
            Some(_) => {
                return Err(crate::error::AppError::BadRequest(
                    "Invalid mark tone".to_string(),
                ));
            }
            None => None,
        };
        self.repo.set_figurine_mark(figurine_id, token, tone).await?;
        Ok(tone.map(str::to_string))
    }

    pub async fn get_admin_mark_stats(&self) -> Result<Vec<AdminFigurineMarkStat>> {
        self.repo.get_admin_mark_stats().await
    }

    pub async fn get_noticed_by_guests_settings(&self) -> Result<NoticedByGuestsSettings> {
        parse_json_setting(
            "noticed_by_guests",
            self.repo.get_setting("noticed_by_guests").await?,
        )
    }

    pub async fn save_noticed_by_guests_settings(
        &self,
        settings: NoticedByGuestsSettings,
    ) -> Result<()> {
        let json = serde_json::to_string(&settings)
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
        if json.len() > 64 * 1024 {
            return Err(crate::error::AppError::BadRequest(
                "Noticed-by-guests settings are too large".to_string(),
            ));
        }
        self.repo.upsert_setting("noticed_by_guests", &json).await
    }

    /// Hybrid public shelf: admin pins go first, in their chosen order; any
    /// remaining slots fill from the private weighted mark ranking, excluding
    /// pinned + explicitly-excluded pieces. Never exposes counts or tones —
    /// only the resolved figurine list, same DTO as every other public listing.
    pub async fn list_noticed_by_guests(&self) -> Result<Vec<FigurineListItemDto>> {
        const MAX_SLOTS: usize = 8;
        let settings = self.get_noticed_by_guests_settings().await?;

        // An explicit pin always wins over a prior exclusion — exclusion only
        // blocks the *automatic* fill below (see NoticedByGuestsSettings doc).
        let mut ordered_ids: Vec<Uuid> = Vec::new();
        for id in &settings.pinned_ids {
            if !ordered_ids.contains(id) {
                ordered_ids.push(*id);
            }
        }

        if ordered_ids.len() < MAX_SLOTS {
            let mut exclude = settings.excluded_ids.clone();
            exclude.extend(ordered_ids.iter().copied());
            let remaining = (MAX_SLOTS - ordered_ids.len()) as i64;
            let auto = self
                .repo
                .get_top_marked_figurine_ids(&exclude, remaining)
                .await?;
            ordered_ids.extend(auto);
        }

        if ordered_ids.is_empty() {
            return Ok(Vec::new());
        }

        let figurines = self.repo.get_figurines_by_ids(&ordered_ids).await?;
        let mut by_id: std::collections::HashMap<Uuid, Figurine> =
            figurines.into_iter().map(|f| (f.id, f)).collect();
        let present_ids: Vec<Uuid> = ordered_ids
            .iter()
            .filter(|id| by_id.contains_key(id))
            .copied()
            .collect();
        let faces = self.repo.get_face_images_for_figurines(&present_ids).await?;
        let details = self.repo.get_detail_images_for_figurines(&present_ids).await?;
        let favorites = self.house_favorite_ids(&present_ids).await?;
        Ok(present_ids
            .into_iter()
            .filter_map(|id| by_id.remove(&id))
            .map(|f| {
                let face = faces.get(&f.id);
                let detail = details.get(&f.id);
                let favorite = favorites.contains(&f.id);
                self.to_list_item(f, face, detail, favorite)
            })
            .collect())
    }

    pub async fn create_booking(
        &self,
        mut req: CreateBookingRequest,
        user_id: Option<Uuid>,
    ) -> Result<Booking> {
        // The booking form on the detail page carries whatever handle is in the URL
        // (slug or UUID). Normalise to the canonical UUID before create_booking_atomic,
        // which binds figurine_id straight into SQL and cannot resolve a slug itself.
        req.figurine_id = self.resolve_figurine_uuid(&req.figurine_id).await?.to_string();

        let starts_at =
            chrono::NaiveDate::parse_from_str(&req.starts_at, "%Y-%m-%d").map_err(|_| {
                crate::error::AppError::BadRequest("Invalid starts_at date".to_string())
            })?;
        let ends_at = chrono::NaiveDate::parse_from_str(&req.ends_at, "%Y-%m-%d")
            .map_err(|_| crate::error::AppError::BadRequest("Invalid ends_at date".to_string()))?;

        if starts_at > ends_at {
            return Err(crate::error::AppError::BadRequest(
                "starts_at must be before or equal to ends_at".to_string(),
            ));
        }

        // Apply the same booking rules as reschedule (was previously skipped on create).
        let rules = self.get_booking_rules().await?;
        Self::validate_booking_rules(&rules, starts_at, ends_at)?;

        let booking = self
            .repo
            .create_booking_atomic(&req, starts_at, ends_at, user_id)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::Conflict(
                    "These dates conflict with existing showings or confirmed bookings".to_string(),
                )
            })?;
        self.observability
            .record_business_event("booking_created", "ok");
        Self::log_domain_event("booking_created", "booking", booking.id, "ok");
        {
            let svc = self.clone();
            let booking = booking.clone();
            tokio::spawn(async move {
                let _ = svc.send_booking_notification(&booking).await;
            });
        }
        Ok(booking)
    }

    async fn send_booking_notification(&self, booking: &Booking) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let admin_link = format!(
            "{}/admin#bookings",
            self.config.public_url.trim_end_matches('/')
        );

        let text = format!(
            "📅 Запрос на бронирование\n\n\
            🏺 Фигурка: {}\n\
            📅 Период: {} — {}\n\
            👤 Имя: {}\n\
            📧 Email: {}\n\
            💬 Цель: {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(&booking.figurine_name),
            escape_markdown(&booking.starts_at.to_string()),
            escape_markdown(&booking.ends_at.to_string()),
            escape_markdown(&booking.requester_name),
            escape_markdown(&booking.requester_email),
            escape_markdown(booking.purpose.as_deref().unwrap_or("—")),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let client = self.http_client.clone();
        let _ = client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "MarkdownV2"
            }))
            .send()
            .await;
        Ok(())
    }

    // === SHOWINGS (ADMIN) ===

    pub async fn list_showings(&self) -> Result<Vec<ShowingDto>> {
        let showings = self.repo.get_all_showings().await?;
        Ok(showings
            .into_iter()
            .map(|s| ShowingDto {
                id: s.id.to_string(),
                figurine_id: s.figurine_id.to_string(),
                title: s.title,
                showing_type: s.showing_type,
                starts_at: s.starts_at.to_string(),
                ends_at: s.ends_at.to_string(),
                venue: s.venue,
                notes: s.notes,
            })
            .collect())
    }

    pub async fn save_showing(&self, req: SaveShowingRequest) -> Result<ShowingDto> {
        let id = self.repo.upsert_showing(&req).await?;
        Self::log_domain_event("showing_saved", "showing", id, "ok");
        Ok(ShowingDto {
            id: id.to_string(),
            figurine_id: req.figurine_id,
            title: req.title,
            showing_type: req.showing_type,
            starts_at: req.starts_at,
            ends_at: req.ends_at,
            venue: req.venue,
            notes: req.notes,
        })
    }

    pub async fn delete_showing(&self, id: String) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        self.repo.delete_showing(uuid).await?;
        Self::log_domain_event("showing_deleted", "showing", uuid, "ok");
        Ok(())
    }

    /// Bulk admin action: un-feature every figurine on the home page and
    /// wipe every scheduled showing entry across the whole collection.
    pub async fn bulk_clear_showings(&self) -> Result<crate::models::BulkOpSummary> {
        let unfeatured = sqlx::query(
            "UPDATE figurines SET is_featured = false WHERE is_featured = true",
        )
        .execute(self.repo.pg_pool())
        .await?
        .rows_affected();
        let deleted = sqlx::query("DELETE FROM figurine_showings")
            .execute(self.repo.pg_pool())
            .await?
            .rows_affected();
        let affected = unfeatured + deleted;
        Self::log_domain_event("bulk_showings_cleared", "figurine", affected, "ok");
        Ok(crate::models::BulkOpSummary { affected })
    }

    // === BOOKINGS (ADMIN) ===

    pub async fn list_bookings(
        &self,
        status_filter: Option<&str>,
        figurine_id: Option<uuid::Uuid>,
        page: i64,
        per_page: i64,
    ) -> Result<BookingsPage> {
        let offset = (page - 1) * per_page;
        let (items, total) = self
            .repo
            .get_bookings_page(status_filter, figurine_id, per_page, offset)
            .await?;
        let pending_count = self.repo.get_pending_bookings_count().await?;
        let dtos = items
            .into_iter()
            .map(|b| BookingDto {
                id: b.id.to_string(),
                figurine_id: b.figurine_id.to_string(),
                figurine_name: b.figurine_name,
                requester_name: b.requester_name,
                requester_email: b.requester_email,
                requester_phone: b.requester_phone,
                purpose: b.purpose,
                display_type: b.display_type,
                venue: b.venue,
                curator_conditions: b.curator_conditions,
                starts_at: b.starts_at.to_string(),
                ends_at: b.ends_at.to_string(),
                status: b.status,
                admin_notes: b.admin_notes,
                created_at: b.created_at.to_rfc3339(),
            })
            .collect();
        Ok(BookingsPage {
            items: dtos,
            total,
            pending_count,
            page,
            per_page,
        })
    }

    pub async fn update_booking_status(
        &self,
        id: uuid::Uuid,
        status: BookingStatus,
        admin_notes: Option<String>,
        curator_conditions: Option<String>,
    ) -> Result<()> {
        let booking =
            self.repo.get_booking_by_id(id).await?.ok_or_else(|| {
                crate::error::AppError::NotFound(format!("Booking {} not found", id))
            })?;

        if status == BookingStatus::Confirmed {
            // Conflict re-check + booking/figurine update happen atomically under a
            // per-figurine lock, so two confirmations can't race into a double-book.
            if let Some(reason) = self
                .repo
                .confirm_booking_atomic(
                    id,
                    booking.figurine_id,
                    booking.starts_at,
                    booking.ends_at,
                    admin_notes.as_deref(),
                    curator_conditions.as_deref(),
                )
                .await?
            {
                return Err(crate::error::AppError::Conflict(reason));
            }
            self.send_booking_status_message(
                &booking,
                &status,
                admin_notes.as_deref(),
                curator_conditions.as_deref(),
            )
            .await;
            self.observability
                .record_business_event("booking_status_changed", "ok");
            Self::log_domain_event("booking_status_updated", "booking", id, "ok");
            return Ok(());
        }

        if (status == BookingStatus::Completed
            || status == BookingStatus::Cancelled
            || status == BookingStatus::Rejected)
            && booking.status == BookingStatus::Confirmed
        {
            self.repo
                .update_booking_status(
                    id,
                    &status,
                    admin_notes.as_deref(),
                    curator_conditions.as_deref(),
                )
                .await?;
            let has_others = self
                .repo
                .has_future_confirmed_bookings(booking.figurine_id, id)
                .await?;
            if !has_others {
                self.repo
                    .update_figurine_status(booking.figurine_id, &FigurineStatus::Available)
                    .await?;
            }
            self.send_booking_status_message(
                &booking,
                &status,
                admin_notes.as_deref(),
                curator_conditions.as_deref(),
            )
            .await;
            self.observability
                .record_business_event("booking_status_changed", "ok");
            Self::log_domain_event("booking_status_updated", "booking", id, "ok");
            return Ok(());
        }

        self.repo
            .update_booking_status(
                id,
                &status,
                admin_notes.as_deref(),
                curator_conditions.as_deref(),
            )
            .await?;
        self.send_booking_status_message(
            &booking,
            &status,
            admin_notes.as_deref(),
            curator_conditions.as_deref(),
        )
        .await;
        self.observability
            .record_business_event("booking_status_changed", "ok");
        Self::log_domain_event("booking_status_updated", "booking", id, "ok");
        Ok(())
    }

    async fn send_booking_status_message(
        &self,
        booking: &Booking,
        status: &BookingStatus,
        admin_notes: Option<&str>,
        curator_conditions: Option<&str>,
    ) {
        let Some(user_id) = booking.user_id else {
            return;
        };
        let (subject, body) = match status {
            BookingStatus::Confirmed => (
                format!("Бронирование подтверждено — {}", booking.figurine_name),
                {
                    let base = format!(
                        "Ваш запрос на бронирование «{}» ({} — {}) подтверждён.",
                        booking.figurine_name, booking.starts_at, booking.ends_at,
                    );
                    match curator_conditions {
                        Some(c) if !c.is_empty() => format!("{}\n\nУсловия куратора: {}", base, c),
                        _ => base,
                    }
                },
            ),
            BookingStatus::Rejected => (
                format!("Бронирование отклонено — {}", booking.figurine_name),
                {
                    let base = format!(
                        "Ваш запрос на бронирование «{}» ({} — {}) отклонён.",
                        booking.figurine_name, booking.starts_at, booking.ends_at,
                    );
                    match admin_notes {
                        Some(n) if !n.is_empty() => format!("{}\n\nПримечание: {}", base, n),
                        _ => base,
                    }
                },
            ),
            BookingStatus::Cancelled => (
                format!("Бронирование отменено — {}", booking.figurine_name),
                format!(
                    "Бронирование «{}» ({} — {}) отменено.",
                    booking.figurine_name, booking.starts_at, booking.ends_at,
                ),
            ),
            BookingStatus::Completed => (
                format!("Бронирование завершено — {}", booking.figurine_name),
                format!(
                    "Бронирование «{}» ({} — {}) завершено. Спасибо!",
                    booking.figurine_name, booking.starts_at, booking.ends_at,
                ),
            ),
            _ => return,
        };
        let _ = self
            .repo
            .create_thread(
                user_id,
                "booking",
                Some(booking.id),
                &subject,
                &body,
                true,
                &[],
            )
            .await;
    }

    fn clean_media_path(&self, path: &str) -> String {
        let base = self.config.public_url.trim_end_matches('/');
        path.strip_prefix(base)
            .unwrap_or(path)
            .trim_start_matches("/static/")
            .trim_start_matches('/')
            .replace('\\', "/")
    }

    fn public_media_url(&self, path: &str) -> String {
        let base = self.config.public_url.trim_end_matches('/');
        format!("{}/static/{}", base, path.trim_start_matches('/'))
    }

    fn is_managed_media_path(path: &str) -> bool {
        path.starts_with("images/")
            || path.starts_with("videos/")
            || path.starts_with("audio/")
            || path.starts_with("backgrounds/")
    }

    fn media_type_for_path(path: &str) -> String {
        if path.starts_with("images/") || path.starts_with("backgrounds/") {
            "image".to_string()
        } else if path.starts_with("videos/") {
            "video".to_string()
        } else if path.starts_with("audio/") {
            "audio".to_string()
        } else {
            "other".to_string()
        }
    }

    fn variant_for_path(path: &str) -> Option<String> {
        if path.starts_with("images/original/") {
            Some("original".to_string())
        } else if path.starts_with("images/preview/") {
            Some("preview".to_string())
        } else if path.starts_with("images/thumb/") {
            Some("thumb".to_string())
        } else {
            None
        }
    }

    /// Synchronous recursive walk of the uploads tree. Takes the dir by value (not
    /// &self) so it can be handed to `spawn_blocking` — the blocking std::fs calls
    /// must never run on an async runtime worker (see `media_inventory`).
    fn collect_upload_files(upload_dir: &str) -> Result<Vec<(String, u64)>> {
        let mut files = Vec::new();
        for folder in ["images", "videos", "audio", "backgrounds"] {
            let dir = Path::new(upload_dir).join(folder);
            if dir.exists() {
                Self::collect_files_recursive(Path::new(upload_dir), &dir, &mut files)?;
            }
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(files)
    }

    fn collect_files_recursive(
        base: &Path,
        dir: &Path,
        files: &mut Vec<(String, u64)>,
    ) -> Result<()> {
        for entry in fs::read_dir(dir).map_err(crate::error::AppError::Io)? {
            let entry = entry.map_err(crate::error::AppError::Io)?;
            let path = entry.path();
            if path.is_dir() {
                Self::collect_files_recursive(base, &path, files)?;
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let rel = path
                .strip_prefix(base)
                .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            let size = fs::metadata(&path)
                .map_err(crate::error::AppError::Io)?
                .len();
            files.push((rel, size));
        }
        Ok(())
    }

    pub async fn media_inventory(&self) -> Result<MediaInventoryDto> {
        let mut usage_map: HashMap<String, Vec<MediaUsageDto>> = HashMap::new();
        for mut usage in self.repo.get_media_usages().await? {
            let cleaned = self.clean_media_path(&usage.path);
            if !Self::is_managed_media_path(&cleaned) {
                continue;
            }
            usage.path = cleaned;
            usage_map.entry(usage.path.clone()).or_default().push(usage);
        }

        // The uploads-tree walk is synchronous std::fs — run it off the async
        // runtime so a large media library can't stall every concurrent request.
        let upload_dir = self.config.upload_dir.clone();
        let files_on_disk =
            tokio::task::spawn_blocking(move || Self::collect_upload_files(&upload_dir))
                .await
                .map_err(|e| AppError::Internal(format!("media scan task failed: {e}")))??;
        let file_size_map: HashMap<String, u64> = files_on_disk.into_iter().collect();
        let mut known_paths: HashSet<String> = usage_map.keys().cloned().collect();
        known_paths.extend(file_size_map.keys().cloned());

        let mut files = known_paths
            .into_iter()
            .map(|path| {
                let size_bytes = file_size_map.get(&path).copied().unwrap_or(0);
                let exists = file_size_map.contains_key(&path);
                let usages = usage_map.remove(&path).unwrap_or_default();
                MediaFileDto {
                    url: self.public_media_url(&path),
                    media_type: Self::media_type_for_path(&path),
                    variant: Self::variant_for_path(&path),
                    size_bytes,
                    exists,
                    path,
                    usages,
                }
            })
            .collect::<Vec<_>>();

        files.sort_by(|a, b| {
            b.usages
                .len()
                .cmp(&a.usages.len())
                .then_with(|| a.path.cmp(&b.path))
        });

        let orphan_count = files.iter().filter(|file| file.usages.is_empty()).count();
        let used_count = files.len().saturating_sub(orphan_count);
        let total_size_bytes = files.iter().map(|file| file.size_bytes).sum();
        Ok(MediaInventoryDto {
            files,
            orphan_count,
            used_count,
            total_size_bytes,
        })
    }

    pub async fn unused_media_report(&self) -> Result<MediaCleanupReportDto> {
        let inventory = self.media_inventory().await?;
        let files = inventory
            .files
            .into_iter()
            .filter(|file| file.exists && file.usages.is_empty())
            .collect::<Vec<_>>();
        let total_size_bytes = files.iter().map(|file| file.size_bytes).sum();
        Ok(MediaCleanupReportDto {
            files,
            total_size_bytes,
        })
    }

    pub async fn cleanup_unused_media(&self) -> Result<Vec<String>> {
        let report = self.unused_media_report().await?;
        let mut removed = Vec::new();
        for file in report.files {
            let path = Path::new(&self.config.upload_dir).join(&file.path);
            if path.exists() && path.is_file() {
                fs::remove_file(&path).map_err(crate::error::AppError::Io)?;
                removed.push(file.path);
            }
        }
        Self::log_domain_event("media_cleanup_completed", "media", removed.len(), "ok");
        Ok(removed)
    }

    pub async fn replace_media_everywhere(
        &self,
        old_path: &str,
        new_preview_path: &str,
        new_original_path: Option<&str>,
        new_thumb_path: Option<&str>,
    ) -> Result<MediaReplaceResultDto> {
        let old_path = self.clean_media_path(old_path);
        let base = self.config.public_url.trim_end_matches('/');
        let old_aliases = [
            old_path.clone(),
            format!("/static/{}", old_path),
            format!("{}/static/{}", base, old_path),
        ];
        let mut updated_references = 0usize;
        for alias in old_aliases {
            updated_references += self
                .repo
                .replace_media_path_everywhere(
                    &alias,
                    new_preview_path,
                    new_original_path,
                    new_thumb_path,
                )
                .await?;
        }
        let mut imported_paths = vec![new_preview_path.to_string()];
        if let Some(path) = new_original_path {
            imported_paths.push(path.to_string());
        }
        if let Some(path) = new_thumb_path {
            imported_paths.push(path.to_string());
        }
        Self::log_domain_event("media_replaced", "media", &old_path, "ok");
        Ok(MediaReplaceResultDto {
            old_path,
            new_path: new_preview_path.to_string(),
            updated_references,
            imported_paths,
        })
    }
}

// ── Input validation helpers ────────────────────────────────────────────────

const MAX_ATTACHMENTS: usize = 10;
const MAX_URL_LEN: usize = 2048;

/// Validate uploaded-reference URLs: bounded count, bounded length, and only
/// our own `/static/` paths or absolute http(s) URLs (no `javascript:` / `data:`
/// / `file:` smuggling).
fn validate_attachments(atts: &[AttachmentInput]) -> Result<()> {
    if atts.len() > MAX_ATTACHMENTS {
        return Err(AppError::BadRequest(format!(
            "Too many attachments (max {MAX_ATTACHMENTS})"
        )));
    }
    let ok_url = |u: &str| {
        u.len() <= MAX_URL_LEN
            && (u.starts_with("/static/") || u.starts_with("http://") || u.starts_with("https://"))
    };
    for a in atts {
        if !ok_url(&a.url) {
            return Err(AppError::BadRequest("Invalid attachment URL".into()));
        }
        if let Some(t) = &a.thumb_url
            && !ok_url(t)
        {
            return Err(AppError::BadRequest(
                "Invalid attachment thumbnail URL".into(),
            ));
        }
    }
    Ok(())
}

/// Require a trimmed string within `[1, max]` characters.
fn validate_text(field: &str, value: &str, max: usize) -> Result<()> {
    let len = value.trim().chars().count();
    if len == 0 {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    if len > max {
        return Err(AppError::BadRequest(format!(
            "{field} is too long (max {max} characters)"
        )));
    }
    Ok(())
}

/// Decode a JSON-stored setting. Missing → default, but a *corrupt* value is a
/// hard error instead of being silently reset to defaults (which would quietly
/// wipe SMTP creds / theme / booking rules).
fn parse_json_setting<T>(key: &str, json: Option<String>) -> Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    match json {
        Some(j) => serde_json::from_str(&j)
            .map_err(|e| AppError::Internal(format!("Corrupt setting '{key}': {e}"))),
        None => Ok(T::default()),
    }
}

fn escape_markdown(s: &str) -> String {
    // Telegram MarkdownV2 special chars
    s.chars().fold(String::new(), |mut acc, c| {
        if matches!(
            c,
            '_' | '*'
                | '['
                | ']'
                | '('
                | ')'
                | '~'
                | '`'
                | '>'
                | '#'
                | '+'
                | '-'
                | '='
                | '|'
                | '{'
                | '}'
                | '.'
                | '!'
        ) {
            acc.push('\\');
        }
        acc.push(c);
        acc
    })
}

// ============================================================
// AUTH CONSTANTS
// ============================================================

const CATEGORIES: [&str; 4] = ["animals", "dishes", "seasons", "symbols"];

/// How many icons each user is shown per category (their personal subset).
const POOL_PER_CATEGORY: usize = 8;

/// Master icon pool. Each user is shown a random subset of POOL_PER_CATEGORY of
/// these per category (chosen at registration, persisted, replayed at login).
/// IDs must match `src/lib/data/visualIcons.ts` exactly on the frontend.
const ICONS: &[(&str, &[&str])] = &[
    (
        "animals",
        &[
            "wolf",
            "raven",
            "fox",
            "owl",
            "snake",
            "deer",
            "bat",
            "cat",
            "bear",
            "hare",
            "boar",
            "lynx",
            "crow",
            "moth",
            "spider",
            "frog",
            "hound",
            "horse",
            "goat",
            "ram",
            "hawk",
            "mouse",
            "beetle",
            "stag_beetle",
        ],
    ),
    (
        "dishes",
        &[
            "mushroom", "apple", "bread", "cup", "fish", "berry", "honey", "herb", "pear", "plum",
            "egg", "cheese", "grapes", "carrot", "onion", "pumpkin", "walnut", "pie", "soup",
            "wine", "milk", "salt", "pepper", "garlic",
        ],
    ),
    (
        "seasons",
        &[
            "snowflake",
            "bare_tree",
            "sprout",
            "rain",
            "sun",
            "wheat",
            "leaf",
            "acorn",
            "icicle",
            "frost_pane",
            "bud",
            "blossom",
            "cloud",
            "lightning",
            "mist",
            "pinecone",
            "fern",
            "sheaf",
            "crescent",
            "dewdrop",
            "hail",
            "gust",
            "ember",
            "catkin",
        ],
    ),
    (
        "symbols",
        &[
            "key",
            "candle",
            "hourglass",
            "skull",
            "moon",
            "star",
            "cross",
            "anchor",
            "bell",
            "clock",
            "feather",
            "inkpot",
            "scroll",
            "dagger",
            "crown",
            "eye",
            "lantern",
            "mask",
            "book",
            "chalice",
            "compass",
            "keyhole",
            "ring",
            "coin",
        ],
    ),
];

fn valid_icon_ids(category: &str) -> Option<&'static [&'static str]> {
    ICONS
        .iter()
        .find(|(c, _)| *c == category)
        .map(|(_, ids)| *ids)
}

/// Deterministically derive a per-email decoy subset for emails that have no
/// account, so the login grid is stable and statistically indistinguishable
/// from a real stored pool (prevents account enumeration via the icon set).
/// Keyed by the admin secret so the subset can't be recomputed by an attacker.
fn decoy_pool(secret: &str, email: &str) -> Vec<Vec<String>> {
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    use sha2::{Digest, Sha256};

    CATEGORIES
        .iter()
        .map(|category| {
            let mut hasher = Sha256::new();
            hasher.update(secret.as_bytes());
            hasher.update(b"\x00");
            hasher.update(email.as_bytes());
            hasher.update(b"\x00");
            hasher.update(category.as_bytes());
            let seed: [u8; 32] = hasher.finalize().into();
            let mut rng = rand::rngs::StdRng::from_seed(seed);

            let mut ids: Vec<&'static str> = valid_icon_ids(category).unwrap_or(&[]).to_vec();
            ids.shuffle(&mut rng);
            ids.into_iter()
                .take(POOL_PER_CATEGORY)
                .map(str::to_string)
                .collect()
        })
        .collect()
}

/// Validate a client-proposed personal pool against the master pool and the
/// chosen selections, returning the `{category: [icon_id]}` JSON to persist.
/// Each category must contain exactly POOL_PER_CATEGORY distinct valid icon_ids,
/// and the selection for that category must be one of them.
fn validate_pool(pool: &[Vec<String>; 4], selections: &[String; 4]) -> Result<serde_json::Value> {
    let mut pool_obj = serde_json::Map::new();
    for (i, category) in CATEGORIES.iter().enumerate() {
        let master = valid_icon_ids(category)
            .ok_or_else(|| AppError::Internal("Unknown category".into()))?;
        let entries = &pool[i];

        if entries.len() != POOL_PER_CATEGORY {
            return Err(AppError::BadRequest(format!(
                "Invalid pool size for {category}"
            )));
        }
        let mut seen = std::collections::HashSet::new();
        for id in entries {
            if !master.contains(&id.as_str()) {
                return Err(AppError::BadRequest(format!(
                    "Invalid pool icon for {category}"
                )));
            }
            if !seen.insert(id.as_str()) {
                return Err(AppError::BadRequest(format!(
                    "Duplicate pool icon for {category}"
                )));
            }
        }
        if !entries.contains(&selections[i]) {
            return Err(AppError::BadRequest(format!(
                "Selection not in pool for {category}"
            )));
        }
        pool_obj.insert(category.to_string(), serde_json::json!(entries));
    }
    Ok(serde_json::Value::Object(pool_obj))
}

/// Parse a stored `visual_pool` JSON value ({category: [icon_id]}) into the
/// fixed category order. Returns None if the shape is unusable.
fn parse_stored_pool(value: &serde_json::Value) -> Option<Vec<Vec<String>>> {
    let obj = value.as_object()?;
    let mut out = Vec::with_capacity(CATEGORIES.len());
    for category in CATEGORIES {
        let arr = obj.get(category)?.as_array()?;
        let ids: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        if ids.is_empty() {
            return None;
        }
        out.push(ids);
    }
    Some(out)
}

fn build_hash_input(selections: &[String; 4]) -> String {
    selections
        .iter()
        .enumerate()
        .map(|(i, id)| format!("{}:{}", CATEGORIES[i], id))
        .collect::<Vec<_>>()
        .join("|")
}

fn hash_password(input: &str) -> std::result::Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(input.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

fn verify_password(input: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(input.as_bytes(), &parsed)
        .is_ok()
}

// ============================================================
// AppService — AUTH METHODS
// ============================================================

impl AppService {
    pub async fn register_user(
        &self,
        req: &RegisterRequest,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<LoginVerifyResponse> {
        if !req.email.contains('@') {
            return Err(AppError::BadRequest("Invalid email".into()));
        }
        if req.display_name.trim().is_empty() {
            return Err(AppError::BadRequest("Display name required".into()));
        }

        // Validate the personal pool (alphabet shown at login) and that each
        // selection is drawn from it.
        let pool_json = validate_pool(&req.pool, &req.selections)?;

        let hash_input = build_hash_input(&req.selections);
        let hash = hash_password(&hash_input)
            .map_err(|e| AppError::Internal(format!("Hash error: {e}")))?;

        let ctx = self.client_context(ip.clone(), user_agent.clone());
        let user = self
            .repo
            .create_user(
                &req.email.to_lowercase(),
                &req.display_name,
                &hash,
                &pool_json,
                &ctx,
            )
            .await?;

        let session_token = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
        let session_ctx = self.client_context(ip, user_agent);
        self.repo
            .create_session(user.id, &session_token, expires_at, &session_ctx)
            .await?;

        Self::log_domain_event("user_registered", "user", user.id, "ok");
        Ok(LoginVerifyResponse {
            session_token,
            user: UserDto::from(&user),
        })
    }

    pub async fn login_challenge(&self, email: &str) -> Result<LoginChallengeResponse> {
        use rand::seq::SliceRandom;
        let email_lower = email.to_lowercase();

        // A challenge is always issued — for unknown, valid, AND blocked emails —
        // so the response never reveals whether an account exists. The actual
        // user/password/blocked checks happen in login_verify, which returns the
        // same generic Unauthorized for every failure. (Avoids account enumeration.)

        // Check lockout before issuing challenge
        let failures = self.repo.count_recent_failures(&email_lower, 15).await?;
        if failures >= 5 {
            return Err(AppError::BadRequest(
                "Too many failed attempts. Try again in 15 minutes.".into(),
            ));
        }

        // Determine the icon set to show. For a real account it's the personal
        // pool stored at registration; for an unknown email (or a legacy account
        // with no pool) it's a deterministic decoy keyed by the admin secret, so
        // the grid is stable per email and reveals nothing about account
        // existence. (.await before building tokens — ThreadRng is !Send.)
        let user = self.repo.find_user_by_email(&email_lower).await?;
        let pool: Vec<Vec<String>> = user
            .as_ref()
            .and_then(|u| u.visual_pool.as_ref())
            .and_then(parse_stored_pool)
            .unwrap_or_else(|| decoy_pool(&self.config.admin_api_key, &email_lower));

        // Build tokens synchronously in a block so ThreadRng (!Send) is dropped before any .await
        let (all_tokens, steps) = {
            let mut rng = rand::thread_rng();
            let mut all_tokens: Vec<ChallengeToken> = Vec::new();
            let mut steps: Vec<ChallengeStepDto> = Vec::new();

            for (i, category) in CATEGORIES.iter().enumerate() {
                let mut icons_shuffled: Vec<&String> = pool[i].iter().collect();
                icons_shuffled.shuffle(&mut rng);

                let mut step_icons: Vec<ChallengeIconDto> = Vec::new();
                for icon_id in icons_shuffled {
                    let token = Uuid::new_v4().to_string();
                    all_tokens.push(ChallengeToken {
                        token: token.clone(),
                        category: category.to_string(),
                        icon_id: icon_id.clone(),
                    });
                    step_icons.push(ChallengeIconDto {
                        token,
                        icon_id: icon_id.clone(),
                    });
                }
                steps.push(ChallengeStepDto {
                    category: category.to_string(),
                    icons: step_icons,
                });
            }
            (all_tokens, steps)
        }; // rng dropped here, before any .await

        let tokens_json = serde_json::to_value(&all_tokens)
            .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;

        let challenge_id = self.repo.save_challenge(&email_lower, &tokens_json).await?;

        Ok(LoginChallengeResponse {
            challenge_id: challenge_id.to_string(),
            steps,
        })
    }

    pub async fn login_verify(
        &self,
        req: &LoginVerifyRequest,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<LoginVerifyResponse> {
        // Resolve geo once up front; the same context is stored on every attempt
        // (success or failure) and on the created session.
        let ctx = self.client_context(ip, user_agent);

        let challenge_id = Uuid::parse_str(&req.challenge_id)
            .map_err(|_| AppError::BadRequest("Invalid challenge ID".into()))?;

        let (email, tokens_json) = self
            .repo
            .get_challenge(challenge_id)
            .await?
            .ok_or_else(|| AppError::BadRequest("Challenge expired or not found".into()))?;

        // Check lockout
        let failures = self.repo.count_recent_failures(&email, 15).await?;
        if failures >= 5 {
            return Err(AppError::BadRequest(
                "Too many failed attempts. Try again in 15 minutes.".into(),
            ));
        }

        let token_map: Vec<ChallengeToken> = serde_json::from_value(tokens_json)
            .map_err(|e| AppError::Internal(format!("Deserialize error: {e}")))?;

        // Resolve each submitted token to icon_id, in category order
        let mut resolved_selections: [String; 4] = Default::default();
        for (i, submitted_token) in req.tokens.iter().enumerate() {
            let expected_category = CATEGORIES[i];
            let entry = token_map
                .iter()
                .find(|t| &t.token == submitted_token && t.category == expected_category)
                .ok_or_else(|| AppError::BadRequest("Invalid selection".into()))?;
            resolved_selections[i] = entry.icon_id.clone();
        }

        // Mark challenge as used before verifying (prevent replay regardless of outcome)
        self.repo.mark_challenge_used(challenge_id).await?;

        let user = match self.repo.find_user_by_email(&email).await? {
            Some(u) => u,
            None => {
                self.repo.record_attempt(&email, false, &ctx).await?;
                return Err(AppError::Unauthorized);
            }
        };

        let hash_input = build_hash_input(&resolved_selections);
        if !verify_password(&hash_input, &user.visual_password_hash) {
            self.repo.record_attempt(&email, false, &ctx).await?;
            return Err(AppError::Unauthorized);
        }

        // Blocked accounts fail with the same generic Unauthorized (no enumeration).
        if user.is_blocked {
            self.repo.record_attempt(&email, false, &ctx).await?;
            return Err(AppError::Unauthorized);
        }

        // record_attempt failure must not abort a successful login — use .ok()
        self.repo.record_attempt(&email, true, &ctx).await.ok();

        // Create 30-day session; prune expired sessions for this user at the same time
        let session_token = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
        self.repo
            .create_session(user.id, &session_token, expires_at, &ctx)
            .await?;
        self.repo.prune_expired_sessions(user.id).await.ok();

        Self::log_domain_event("user_login", "user", user.id, "ok");
        Ok(LoginVerifyResponse {
            session_token,
            user: UserDto::from(&user),
        })
    }

    pub async fn get_user_from_session(&self, token: &str) -> Result<User> {
        self.repo
            .get_session_user(token)
            .await?
            .ok_or(AppError::Unauthorized)
    }

    pub async fn logout(&self, token: &str) -> Result<()> {
        self.repo.delete_session(token).await?;
        Self::log_domain_event("user_logout", "session", "current", "ok");
        Ok(())
    }

    pub async fn link_bookings(&self, user_id: Uuid, cancel_tokens: &[String]) -> Result<usize> {
        let user = self
            .repo
            .find_user_by_id(user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;
        self.repo
            .link_bookings_to_user(user_id, &user.email, cancel_tokens)
            .await
    }

    pub async fn get_user_bookings(&self, user_id: Uuid) -> Result<Vec<UserBookingDto>> {
        let bookings = self.repo.get_user_bookings(user_id).await?;
        Ok(bookings
            .into_iter()
            .map(|b| UserBookingDto {
                id: b.id.to_string(),
                figurine_id: b.figurine_id.to_string(),
                figurine_name: b.figurine_name,
                starts_at: b.starts_at.to_string(),
                ends_at: b.ends_at.to_string(),
                status: b.status,
                created_at: b.created_at.to_rfc3339(),
                cancel_token: b.cancel_token,
                display_type: b.display_type,
                venue: b.venue,
                curator_conditions: b.curator_conditions,
            })
            .collect())
    }

    pub async fn get_user_wishlist(&self, user_id: Uuid) -> Result<Vec<String>> {
        self.repo.get_user_wishlist(user_id).await
    }

    pub async fn set_user_wishlist(&self, user_id: Uuid, ids: Vec<String>) -> Result<Vec<String>> {
        // Keep only valid figurine IDs, dedupe and cap so the column stays
        // bounded even if a client misbehaves.
        let mut seen = std::collections::HashSet::new();
        let cleaned: Vec<String> = ids
            .into_iter()
            .filter_map(|s| {
                let id = s.trim();
                Uuid::parse_str(id).ok()?;
                if seen.insert(id.to_string()) {
                    Some(id.to_string())
                } else {
                    None
                }
            })
            .take(500)
            .collect();
        self.repo.set_user_wishlist(user_id, &cleaned).await?;
        Self::log_domain_event("wishlist_updated", "user", user_id, "ok");
        Ok(cleaned)
    }

    /// Attach a guest request to the account by its secret code. Tries each request
    /// type in turn (booking → waitlist → notify → commission); the first table that
    /// recognises the token decides the outcome.
    pub async fn link_claim_by_token(&self, user: &User, token: &str) -> Result<LinkClaimResponse> {
        let token = token.trim();
        if token.is_empty() {
            return Ok(LinkClaimResponse {
                result: "not_found".into(),
                kind: None,
                name: None,
            });
        }
        if let Some(m) = self
            .repo
            .link_booking_by_token(user.id, &user.email, token)
            .await?
        {
            return Ok(Self::claim_response("booking", m));
        }
        if let Some(m) = self
            .repo
            .link_waitlist_by_token(user.id, &user.email, token)
            .await?
        {
            return Ok(Self::claim_response("waitlist", m));
        }
        if let Some(m) = self
            .repo
            .link_notify_order_by_token(user.id, &user.email, token)
            .await?
        {
            return Ok(Self::claim_response("notify", m));
        }
        if self.repo.commission_claimable_by(token, user.id).await? {
            // Token-only (no email guard). Reuse claim_commission so the conversation
            // thread is seeded exactly as it is for the in-app claim flow.
            let dto = self.claim_commission(token, user.id).await?;
            let name = if dto.title.trim().is_empty() {
                dto.requester_name.clone()
            } else {
                dto.title.clone()
            };
            return Ok(LinkClaimResponse {
                result: "linked".into(),
                kind: Some("commission".into()),
                name: Some(name),
            });
        }
        Self::log_domain_event("claim_link_checked", "user", user.id, "not_found");
        Ok(LinkClaimResponse {
            result: "not_found".into(),
            kind: None,
            name: None,
        })
    }

    fn claim_response(kind: &str, m: crate::db::ClaimMatch) -> LinkClaimResponse {
        let result = if m.linked {
            "linked"
        } else if !m.email_ok {
            "email_mismatch"
        } else {
            "already_linked"
        };
        LinkClaimResponse {
            result: result.to_string(),
            kind: Some(kind.to_string()),
            name: Some(m.name),
        }
    }

    pub async fn get_user_waitlist(&self, user_id: Uuid) -> Result<Vec<WaitlistEntryDto>> {
        let entries = self.repo.get_user_waitlist(user_id).await?;
        // All positions in one query instead of a SELECT COUNT(*) per entry (N+1).
        let positions = self.repo.waitlist_positions_for_user(user_id).await?;
        let mut dtos = Vec::with_capacity(entries.len());
        for e in entries {
            let position = positions.get(&e.id).copied().unwrap_or(0);
            dtos.push(WaitlistEntryDto {
                id: e.id.to_string(),
                figurine_id: e.figurine_id.to_string(),
                figurine_name: e.figurine_name,
                requester_name: e.requester_name,
                requester_email: e.requester_email,
                requester_phone: e.requester_phone,
                note: e.note,
                created_at: e.created_at.to_rfc3339(),
                user_id: e.user_id.map(|u| u.to_string()),
                position,
            });
        }
        Ok(dtos)
    }

    pub async fn get_user_orders(&self, user_id: Uuid) -> Result<Vec<UserOrderDto>> {
        let orders = self.repo.get_user_orders(user_id).await?;
        Ok(orders
            .into_iter()
            .map(|o| {
                let certificate = Self::certificate_dto(&o);
                UserOrderDto {
                    id: o.id.to_string(),
                    figurine_id: o.figurine_id,
                    figurine_name: o.figurine_name,
                    mode: o.mode,
                    status: o.status,
                    created_at: o.created_at.to_rfc3339(),
                    admin_notes: o.admin_notes,
                    reserve_status: o.reserve_status,
                    reserve_expires_at: o.reserve_expires_at.map(|d| d.to_string()),
                    admin_terms_note: o.admin_terms_note,
                    invoice_note: o.invoice_note,
                    certificate,
                }
            })
            .collect())
    }

    // === ADMIN USER MANAGEMENT ===

    pub async fn admin_list_users(
        &self,
        search: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<AdminUserListItem>, i64)> {
        let offset = (page - 1) * per_page;
        self.repo.admin_list_users(search, per_page, offset).await
    }

    pub async fn admin_get_user_detail(&self, user_id: Uuid) -> Result<AdminUserDetail> {
        let user = self
            .repo
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        let bookings = self.get_user_bookings(user_id).await?;
        let orders = self.get_user_orders(user_id).await?;
        let sessions = self.repo.admin_get_user_sessions(user_id).await?;
        let recent_failures = self
            .repo
            .count_recent_failures(&user.email, 24 * 60)
            .await?;
        let messages = self.admin_get_user_threads(user_id).await?;

        Ok(AdminUserDetail {
            id: user.id.to_string(),
            email: user.email,
            display_name: user.display_name,
            admin_notes: user.admin_notes,
            created_at: user.created_at.to_rfc3339(),
            signup_ip: user.signup_ip,
            signup_country_code: user.signup_country_code,
            signup_city: user.signup_city,
            last_reset_ip: user.last_reset_ip,
            last_reset_country_code: user.last_reset_country_code,
            last_reset_city: user.last_reset_city,
            last_reset_at: user.last_reset_at.map(|t| t.to_rfc3339()),
            last_reset_request_ip: user.last_reset_request_ip,
            last_reset_request_country_code: user.last_reset_request_country_code,
            last_reset_request_city: user.last_reset_request_city,
            last_reset_request_at: user.last_reset_request_at.map(|t| t.to_rfc3339()),
            bookings,
            orders,
            sessions,
            recent_failures,
            messages,
        })
    }

    pub async fn admin_revoke_user_sessions(&self, user_id: Uuid) -> Result<u64> {
        let revoked = self.repo.admin_revoke_all_sessions(user_id).await?;
        Self::log_domain_event("user_sessions_revoked", "user", user_id, "ok");
        Ok(revoked)
    }

    pub async fn admin_update_user_notes(&self, user_id: Uuid, notes: Option<&str>) -> Result<()> {
        self.repo.admin_update_user_notes(user_id, notes).await?;
        Self::log_domain_event("user_notes_updated", "user", user_id, "ok");
        Ok(())
    }

    pub async fn admin_set_user_blocked(&self, user_id: Uuid, blocked: bool) -> Result<()> {
        // Revoke all active sessions when blocking so the user is immediately logged out
        if blocked {
            self.repo.admin_revoke_all_sessions(user_id).await?;
        }
        self.repo.admin_set_user_blocked(user_id, blocked).await?;
        Self::log_domain_event("user_blocked_updated", "user", user_id, "ok");
        Ok(())
    }

    pub async fn admin_generate_reset_token(&self, user_id: Uuid) -> Result<ResetTokenResponse> {
        // Verify user exists
        self.repo
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        let token = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(48);
        self.repo
            .admin_create_reset_token(user_id, &token, expires_at)
            .await?;
        Self::log_domain_event("admin_reset_token_created", "user", user_id, "ok");
        Ok(ResetTokenResponse {
            token,
            expires_at: expires_at.to_rfc3339(),
        })
    }

    pub async fn validate_reset_token(&self, token: &str) -> Result<UserDto> {
        let user = self
            .repo
            .find_user_by_reset_token(token)
            .await?
            .ok_or_else(|| AppError::BadRequest("Reset link is invalid or has expired.".into()))?;
        Ok(UserDto::from(&user))
    }

    pub async fn apply_password_reset(
        &self,
        req: &ApplyPasswordResetRequest,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<()> {
        let user = self
            .repo
            .find_user_by_reset_token(&req.token)
            .await?
            .ok_or_else(|| AppError::BadRequest("Reset link is invalid or has expired.".into()))?;

        // Reset regenerates the personal pool, so validate it the same way as
        // registration and persist it alongside the new hash.
        let pool_json = validate_pool(&req.pool, &req.selections)?;

        let hash_input = build_hash_input(&req.selections);
        let new_hash = hash_password(&hash_input)
            .map_err(|e| AppError::Internal(format!("Hash error: {e}")))?;

        // Record where the reset was applied from (audit), then persist.
        let ctx = self.client_context(ip, user_agent);

        // Invalidate all existing sessions so old password can't be used
        self.repo.admin_revoke_all_sessions(user.id).await?;
        self.repo
            .apply_password_reset(user.id, &new_hash, &pool_json, &ctx)
            .await?;
        Self::log_domain_event("password_reset_applied", "user", user.id, "ok");
        Ok(())
    }

    /// Self-service "forgot password": issue a reset token and email the link to
    /// the account owner. Always succeeds from the caller's view — whether or not
    /// the email maps to an account is never revealed (anti-enumeration), and the
    /// email is sent out of band so response latency doesn't leak existence.
    pub async fn request_password_reset(
        &self,
        email: &str,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<()> {
        let email_lower = email.trim().to_lowercase();

        // Unknown or blocked accounts: silently do nothing, same response.
        let Some(user) = self.repo.find_user_by_email(&email_lower).await? else {
            return Ok(());
        };
        if user.is_blocked {
            return Ok(());
        }

        let token = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(48);
        let ctx = self.client_context(ip, user_agent);
        self.repo
            .create_self_reset_token(user.id, &token, expires_at, &ctx)
            .await?;
        Self::log_domain_event("password_reset_requested", "user", user.id, "ok");

        // Fire-and-forget the email so the slow SMTP path can't be timed to infer
        // whether the account exists.
        let svc = self.clone();
        let to = user.email.clone();
        tokio::spawn(async move {
            if let Err(e) = svc.send_password_reset_email(&to, &token).await {
                tracing::warn!("Password reset email failed: {e}");
            }
        });

        Ok(())
    }

    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<()> {
        use lettre::message::header::ContentType;
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

        // DB settings take precedence over env config (same resolution as replies).
        let db = self.get_smtp_settings().await.unwrap_or_default();
        let host = db.host.as_deref().or(self.config.smtp_host.as_deref());
        let user = db.user.as_deref().or(self.config.smtp_user.as_deref());
        let pass = db.pass.as_deref().or(self.config.smtp_pass.as_deref());
        let from = db.from.as_deref().or(self.config.smtp_from.as_deref());
        let port = db.port.or(self.config.smtp_port).unwrap_or(587);

        let (Some(host), Some(user), Some(pass), Some(from)) = (host, user, pass, from) else {
            tracing::warn!("SMTP not configured — password reset link not sent to {to}");
            return Ok(());
        };

        let link = format!(
            "{}/set-password?token={}",
            self.config.public_url.trim_end_matches('/'),
            token
        );
        let body_text = format!(
            "Someone asked to restore the way into the archive for this address.\n\n\
            If it was you, follow this passage within 48 hours:\n\n\
            {link}\n\n\
            You will be asked to choose your signs anew.\n\n\
            If it was not you, no harm is done — ignore this letter and nothing changes.",
        );

        let email = Message::builder()
            .from(
                from.parse()
                    .map_err(|_| AppError::Internal("Invalid SMTP from address".into()))?,
            )
            .to(to
                .parse()
                .map_err(|_| AppError::Internal("Invalid recipient address".into()))?)
            .subject("A key to the archive")
            .header(ContentType::TEXT_PLAIN)
            .body(body_text)
            .map_err(|e| AppError::Internal(format!("Email build error: {e}")))?;

        let creds = Credentials::new(user.to_string(), pass.to_string());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(host)
            .map_err(|e| AppError::Internal(format!("SMTP relay error: {e}")))?
            .port(port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("SMTP send error: {e}")))?;
        Ok(())
    }

    // === COMMENTS ===

    async fn check_comment_rate_limit(&self, ip: &str) -> Result<()> {
        const MAX_PER_HOUR: usize = 5;
        let now = Instant::now();
        let cutoff_secs = Duration::from_secs(3600);
        let mut map = self.comment_rate_limiter.lock().await;
        let entry = map.entry(ip.to_string()).or_default();
        entry.retain(|t: &Instant| now.duration_since(*t) < cutoff_secs);
        if entry.len() >= MAX_PER_HOUR {
            return Err(AppError::BadRequest(
                "Too many comments from this address. Please wait before submitting again.".into(),
            ));
        }
        entry.push(now);
        Ok(())
    }

    pub async fn get_smtp_settings(&self) -> Result<SmtpSettings> {
        parse_json_setting("smtp", self.repo.get_setting("smtp").await?)
    }

    pub async fn save_smtp_settings(&self, s: SmtpSettings) -> Result<()> {
        let json = serde_json::to_string(&s).map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("smtp", &json).await
    }

    pub async fn get_contact_settings(&self) -> Result<ContactSettings> {
        parse_json_setting("contact", self.repo.get_setting("contact").await?)
    }

    pub async fn save_contact_settings(&self, s: ContactSettings) -> Result<()> {
        let json = serde_json::to_string(&s).map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("contact", &json).await
    }

    // === WORKSHOP FEATURE (home page) ===

    pub async fn get_programme_settings(&self) -> Result<ProgrammeSettings> {
        parse_json_setting(
            "programme_settings",
            self.repo.get_setting("programme_settings").await?,
        )
    }

    pub async fn save_programme_settings(&self, settings: ProgrammeSettings) -> Result<()> {
        let json =
            serde_json::to_string(&settings).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 8 * 1024 {
            return Err(AppError::BadRequest("Programme settings too large".into()));
        }
        self.repo.upsert_setting("programme_settings", &json).await
    }

    pub async fn get_workshop_feature(&self) -> Result<WorkshopFeature> {
        parse_json_setting(
            "workshop_feature",
            self.repo.get_setting("workshop_feature").await?,
        )
    }

    pub async fn save_workshop_feature(&self, feature: WorkshopFeature) -> Result<()> {
        let json =
            serde_json::to_string(&feature).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 64 * 1024 {
            return Err(AppError::BadRequest("Workshop feature is too large".into()));
        }
        self.repo.upsert_setting("workshop_feature", &json).await
    }

    // === BOOKING RULES ===

    pub async fn get_booking_rules(&self) -> Result<BookingRules> {
        parse_json_setting(
            "booking_rules",
            self.repo.get_setting("booking_rules").await?,
        )
    }

    pub async fn save_booking_rules(&self, rules: BookingRules) -> Result<()> {
        let json = serde_json::to_string(&rules).map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("booking_rules", &json).await
    }

    // === THEME CONFIG ===

    pub async fn get_theme_config(&self) -> Result<ThemeConfig> {
        parse_json_setting("theme_config", self.repo.get_setting("theme_config").await?)
    }

    pub async fn save_theme_config(&self, config: ThemeConfig) -> Result<()> {
        let json = serde_json::to_string(&config).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 100 * 1024 {
            return Err(AppError::BadRequest("Theme config is too large".into()));
        }
        self.repo.upsert_setting("theme_config", &json).await
    }

    // === HOME LAYOUT CONFIG ===

    pub async fn get_home_layout(&self) -> Result<HomeLayoutConfig> {
        parse_json_setting("home_layout", self.repo.get_setting("home_layout").await?)
    }

    pub async fn save_home_layout(&self, config: HomeLayoutConfig) -> Result<()> {
        let json = serde_json::to_string(&config).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 100 * 1024 {
            return Err(AppError::BadRequest("Home layout config is too large".into()));
        }
        self.repo.upsert_setting("home_layout", &json).await
    }

    pub async fn get_home_layout_presets(&self) -> Result<Vec<crate::models::HomeLayoutPreset>> {
        match self.repo.get_setting("home_layout_presets").await? {
            Some(j) => serde_json::from_str(&j)
                .map_err(|e| AppError::Internal(format!("Corrupt home layout presets: {e}"))),
            None => Ok(vec![]),
        }
    }

    pub async fn save_home_layout_presets(
        &self,
        presets: Vec<crate::models::HomeLayoutPreset>,
    ) -> Result<()> {
        let json =
            serde_json::to_string(&presets).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 200 * 1024 {
            return Err(AppError::BadRequest("Home layout presets payload is too large".into()));
        }
        self.repo.upsert_setting("home_layout_presets", &json).await
    }

    // === REEL THEME ===

    pub async fn get_reel_theme(&self) -> Result<crate::models::ReelTheme> {
        parse_json_setting("reel_theme", self.repo.get_setting("reel_theme").await?)
    }

    pub async fn save_reel_theme(&self, config: crate::models::ReelTheme) -> Result<()> {
        let json = serde_json::to_string(&config).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 64 * 1024 {
            return Err(AppError::BadRequest("Reel theme is too large".into()));
        }
        self.repo.upsert_setting("reel_theme", &json).await
    }

    pub async fn get_reel_theme_presets(&self) -> Result<Vec<crate::models::ReelThemePreset>> {
        match self.repo.get_setting("reel_theme_presets").await? {
            Some(j) => serde_json::from_str(&j)
                .map_err(|e| AppError::Internal(format!("Corrupt reel theme presets: {e}"))),
            None => Ok(vec![]),
        }
    }

    pub async fn save_reel_theme_presets(
        &self,
        presets: Vec<crate::models::ReelThemePreset>,
    ) -> Result<()> {
        let json =
            serde_json::to_string(&presets).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 200 * 1024 {
            return Err(AppError::BadRequest("Reel theme presets payload is too large".into()));
        }
        self.repo.upsert_setting("reel_theme_presets", &json).await
    }

    // === DISPLAY CONFIG PRESETS ===

    pub async fn get_display_presets(&self) -> Result<Vec<crate::models::DisplayConfigPreset>> {
        match self.repo.get_setting("display_config_presets").await? {
            Some(j) => serde_json::from_str(&j)
                .map_err(|e| AppError::Internal(format!("Corrupt display presets: {e}"))),
            None => Ok(vec![]),
        }
    }

    pub async fn save_display_presets(
        &self,
        presets: Vec<crate::models::DisplayConfigPreset>,
    ) -> Result<()> {
        let json =
            serde_json::to_string(&presets).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 200 * 1024 {
            return Err(AppError::BadRequest("Display presets payload is too large".into()));
        }
        self.repo.upsert_setting("display_config_presets", &json).await
    }

    // === COPY OVERRIDES ===

    pub async fn get_copy_overrides(&self) -> Result<CopyOverrides> {
        parse_json_setting(
            "copy_overrides",
            self.repo.get_setting("copy_overrides").await?,
        )
    }

    pub async fn save_copy_overrides(&self, overrides: CopyOverrides) -> Result<()> {
        let json =
            serde_json::to_string(&overrides).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 500 * 1024 {
            return Err(AppError::BadRequest("Copy overrides are too large".into()));
        }
        self.repo.upsert_setting("copy_overrides", &json).await
    }

    // === RESCHEDULE ===

    pub async fn reschedule_booking_by_token(
        &self,
        token: &str,
        req: RescheduleBookingRequest,
    ) -> Result<BookingCancelInfo> {
        let rules = self.get_booking_rules().await?;

        let starts_at = chrono::NaiveDate::parse_from_str(&req.starts_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid starts_at date".to_string()))?;
        let ends_at = chrono::NaiveDate::parse_from_str(&req.ends_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid ends_at date".to_string()))?;

        if starts_at > ends_at {
            return Err(AppError::BadRequest(
                "starts_at must be ≤ ends_at".to_string(),
            ));
        }

        Self::validate_booking_rules(&rules, starts_at, ends_at)?;

        // Fetch the current booking to get figurine_id for conflict check
        let current = self
            .repo
            .get_booking_by_cancel_token(token)
            .await?
            .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

        if current.status != BookingStatus::Pending {
            return Err(AppError::BadRequest(
                "Only pending bookings can be rescheduled".to_string(),
            ));
        }

        // Check for conflicts, excluding this booking itself
        if self
            .repo
            .check_booking_conflicts_excluding(current.figurine_id, current.id, starts_at, ends_at)
            .await?
        {
            return Err(AppError::Conflict(
                "These dates conflict with an existing showing or confirmed booking".to_string(),
            ));
        }

        let updated = self
            .repo
            .reschedule_booking_by_token(token, starts_at, ends_at)
            .await?
            .ok_or_else(|| {
                AppError::NotFound("Booking not found or already processed".to_string())
            })?;

        Ok(BookingCancelInfo {
            figurine_name: updated.figurine_name,
            figurine_id: updated.figurine_id.to_string(),
            starts_at: updated.starts_at.to_string(),
            ends_at: updated.ends_at.to_string(),
            status: updated.status,
            admin_notes: updated.admin_notes,
            curator_conditions: updated.curator_conditions,
        })
    }

    // === WAITLIST ===

    pub async fn join_waitlist(
        &self,
        figurine_id: String,
        req: CreateWaitlistRequest,
        user_id: Option<Uuid>,
    ) -> Result<crate::models::WaitlistCreatedResponse> {
        let uuid = self.resolve_figurine_uuid(&figurine_id).await?;
        validate_text("Name", &req.requester_name, 100)?;
        if !req.requester_email.contains('@') || req.requester_email.len() > 200 {
            return Err(AppError::BadRequest("Valid email is required".to_string()));
        }
        if let Some(note) = &req.note
            && note.chars().count() > 1000
        {
            return Err(AppError::BadRequest(
                "Note is too long (max 1000 characters)".to_string(),
            ));
        }
        let (entry, position) = self.repo.add_to_waitlist(uuid, &req, user_id).await?;
        self.observability
            .record_business_event("waitlist_joined", "ok");
        Self::log_domain_event("waitlist_joined", "waitlist", entry.id, "ok");
        {
            let svc = self.clone();
            let entry = entry.clone();
            tokio::spawn(async move {
                let _ = svc.send_waitlist_notification(&entry).await;
            });
        }
        Ok(crate::models::WaitlistCreatedResponse {
            cancel_token: entry.cancel_token,
            position,
        })
    }

    /// View a queue place by its cancel token (visitor's receipt lookup).
    pub async fn get_waitlist_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::WaitlistCancelInfo>> {
        let Some(e) = self.repo.get_waitlist_by_cancel_token(token).await? else {
            return Ok(None);
        };
        let position = self
            .repo
            .waitlist_position(e.figurine_id, e.created_at)
            .await?;
        Ok(Some(crate::models::WaitlistCancelInfo {
            figurine_id: e.figurine_id.to_string(),
            figurine_name: e.figurine_name,
            position,
            created_at: e.created_at.to_rfc3339(),
        }))
    }

    /// Leave the queue by token. Idempotent.
    pub async fn leave_waitlist_by_token(&self, token: &str) -> Result<()> {
        let removed = self.repo.remove_waitlist_by_token(token).await?;
        match removed {
            Some(entry) => Self::log_domain_event("waitlist_left", "waitlist", entry.id, "ok"),
            None => Self::log_domain_event("waitlist_left", "waitlist", "unknown", "not_found"),
        }
        Ok(())
    }

    pub async fn admin_notify_waitlist(&self, figurine_id: String) -> Result<serde_json::Value> {
        let uuid = Self::parse_uuid(&figurine_id)?;
        let entries = self.repo.get_waitlist_for_figurine(uuid).await?;
        if entries.is_empty() {
            return Ok(serde_json::json!({ "notified": 0 }));
        }
        let figurine_name = entries[0].figurine_name.clone();
        let subject = format!("Фигурина «{}» снова доступна", figurine_name);
        let body = format!(
            "Хорошие новости — фигурина «{}», которую вы ждали, снова доступна.\n\nПосетите архив, чтобы узнать подробности.",
            figurine_name
        );
        let mut notified = 0u64;
        for entry in &entries {
            if let Some(uid) = entry.user_id {
                let _ = self
                    .repo
                    .create_thread(uid, "waitlist", None, &subject, &body, true, &[])
                    .await;
                notified += 1;
            }
        }
        // Registered users get an in-app message; anonymous visitors have no
        // account, so also send the author a digest with everyone's contacts
        // to reach out personally. Runs before entries are cleared.
        {
            let svc = self.clone();
            let figurine_name = figurine_name.clone();
            tokio::spawn(async move {
                let _ = svc.send_availability_digest(uuid, &figurine_name).await;
            });
        }
        // Remove all entries for this figurine after notification
        self.repo.mark_waitlist_notified(uuid).await?;
        Self::log_domain_event("waitlist_notified", "figurine", uuid, "ok");
        Ok(serde_json::json!({ "notified": notified, "total": entries.len() }))
    }

    pub async fn list_waitlist_admin(
        &self,
        figurine_id: Option<String>,
    ) -> Result<Vec<WaitlistEntryDto>> {
        let fid = match figurine_id {
            Some(s) => Some(Self::parse_uuid(&s)?),
            None => None,
        };
        let entries = self.repo.get_waitlist_admin(fid).await?;
        // Entries arrive ordered by join time, so a per-figurine running counter
        // yields each row's 1-based position in its queue.
        let mut counters: std::collections::HashMap<uuid::Uuid, i64> =
            std::collections::HashMap::new();
        Ok(entries
            .into_iter()
            .map(|e| {
                let counter = counters.entry(e.figurine_id).or_insert(0);
                *counter += 1;
                WaitlistEntryDto {
                    id: e.id.to_string(),
                    figurine_id: e.figurine_id.to_string(),
                    figurine_name: e.figurine_name,
                    requester_name: e.requester_name,
                    requester_email: e.requester_email,
                    requester_phone: e.requester_phone,
                    note: e.note,
                    created_at: e.created_at.to_rfc3339(),
                    user_id: e.user_id.map(|id| id.to_string()),
                    position: *counter,
                }
            })
            .collect())
    }

    // === NEWSLETTER ("visitor book") ===

    /// Sign the visitor book (single opt-in — active immediately). Sends a
    /// welcome letter only for a genuinely new signature, so a re-sign doesn't
    /// re-mail an existing subscriber.
    pub async fn subscribe(
        &self,
        req: crate::models::CreateSubscriptionRequest,
        ip: Option<String>,
    ) -> Result<crate::models::SubscriptionCreatedResponse> {
        let email = req.email.trim().to_string();
        if !email.contains('@') || email.len() > 200 {
            return Err(AppError::BadRequest("Valid email is required".to_string()));
        }
        if let Some(name) = &req.name
            && name.chars().count() > 100
        {
            return Err(AppError::BadRequest(
                "Name is too long (max 100 characters)".to_string(),
            ));
        }
        let normalized = crate::models::CreateSubscriptionRequest {
            email,
            name: req
                .name
                .as_ref()
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty()),
            source: req.source.clone(),
            lang: req.lang.clone(),
        };
        let (sub, already) = self.repo.subscribe(&normalized, ip.as_deref()).await?;
        self.observability
            .record_business_event("newsletter_subscribed", "ok");
        Self::log_domain_event("newsletter_subscribed", "subscriber", sub.id, "ok");
        if !already {
            let svc = self.clone();
            let sub = sub.clone();
            tokio::spawn(async move {
                let _ = svc.send_welcome_email(&sub).await;
            });
        }
        Ok(crate::models::SubscriptionCreatedResponse {
            unsubscribe_token: sub.unsubscribe_token,
            already_subscribed: already,
        })
    }

    /// View a subscription by its unsubscribe token (the unsubscribe page).
    pub async fn get_subscriber_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::SubscriberInfo>> {
        Ok(self
            .repo
            .get_subscriber_by_token(token)
            .await?
            .map(|s| crate::models::SubscriberInfo { email: s.email }))
    }

    /// Leave the book by token. Idempotent.
    pub async fn unsubscribe_by_token(&self, token: &str) -> Result<()> {
        match self.repo.unsubscribe_by_token(token).await? {
            Some(s) => Self::log_domain_event("newsletter_unsubscribed", "subscriber", s.id, "ok"),
            None => {
                Self::log_domain_event("newsletter_unsubscribed", "subscriber", "unknown", "noop")
            }
        }
        Ok(())
    }

    pub async fn list_subscribers_admin(&self) -> Result<Vec<crate::models::SubscriberDto>> {
        let subs = self.repo.list_subscribers_admin().await?;
        Ok(subs
            .into_iter()
            .map(|s| crate::models::SubscriberDto {
                id: s.id.to_string(),
                email: s.email,
                name: s.name,
                source: s.source,
                lang: s.lang,
                created_at: s.created_at.to_rfc3339(),
            })
            .collect())
    }

    pub async fn remove_subscriber(&self, id: Uuid) -> Result<()> {
        self.repo.remove_subscriber(id).await?;
        Self::log_domain_event("newsletter_removed", "subscriber", id, "ok");
        Ok(())
    }

    // === CONTACT MESSAGES ("write to the author") ===

    /// Send a letter — anonymous, not tied to a figurine or an account (see
    /// [`crate::models::ContactMessage`]). Best-effort Telegram ping to the
    /// admin; the letter is saved either way.
    pub async fn submit_contact_message(
        &self,
        req: crate::models::CreateContactMessageRequest,
        ip: Option<String>,
    ) -> Result<()> {
        let email = req.email.trim().to_string();
        if !email.contains('@') || email.len() > 200 {
            return Err(AppError::BadRequest("Valid email is required".to_string()));
        }
        let message = req.message.trim().to_string();
        if message.is_empty() || message.chars().count() > 4000 {
            return Err(AppError::BadRequest(
                "Message must be between 1 and 4000 characters".to_string(),
            ));
        }
        let normalized = crate::models::CreateContactMessageRequest {
            email,
            message,
            source: req.source.clone(),
            lang: req.lang.clone(),
        };
        let msg = self.repo.create_contact_message(&normalized, ip.as_deref()).await?;
        self.observability
            .record_business_event("contact_message_received", "ok");
        Self::log_domain_event("contact_message_received", "contact_message", msg.id, "ok");
        {
            let svc = self.clone();
            let msg = msg.clone();
            tokio::spawn(async move {
                let _ = svc.send_contact_message_notification(&msg).await;
            });
        }
        Ok(())
    }

    pub async fn list_contact_messages_admin(&self) -> Result<Vec<crate::models::ContactMessageDto>> {
        let msgs = self.repo.list_contact_messages_admin().await?;
        Ok(msgs
            .into_iter()
            .map(|m| crate::models::ContactMessageDto {
                id: m.id.to_string(),
                email: m.email,
                message: m.message,
                source: m.source,
                lang: m.lang,
                is_read: m.is_read,
                created_at: m.created_at.to_rfc3339(),
            })
            .collect())
    }

    pub async fn mark_contact_message_read(&self, id: Uuid) -> Result<()> {
        self.repo.mark_contact_message_read(id).await?;
        Self::log_domain_event("contact_message_read", "contact_message", id, "ok");
        Ok(())
    }

    pub async fn remove_contact_message(&self, id: Uuid) -> Result<()> {
        self.repo.remove_contact_message(id).await?;
        Self::log_domain_event("contact_message_removed", "contact_message", id, "ok");
        Ok(())
    }

    async fn send_contact_message_notification(
        &self,
        msg: &crate::models::ContactMessage,
    ) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let admin_link = format!(
            "{}/admin#contactMessages",
            self.config.public_url.trim_end_matches('/')
        );

        let text = format!(
            "✉ Письмо с сайта\n\n\
            📧 Email: {}\n\
            💬 Сообщение: {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(&msg.email),
            escape_markdown(&msg.message),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let client = self.http_client.clone();
        let _ = client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "MarkdownV2"
            }))
            .send()
            .await;

        Ok(())
    }

    /// A quiet welcome letter — confirms the signature and carries the
    /// unsubscribe door. Fire-and-forget: no SMTP configured → logged and skipped
    /// (the subscription is already active either way).
    async fn send_welcome_email(&self, sub: &crate::models::Subscriber) -> Result<()> {
        use lettre::message::header::ContentType;
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

        let db = self.get_smtp_settings().await.unwrap_or_default();
        let host = db.host.as_deref().or(self.config.smtp_host.as_deref());
        let user = db.user.as_deref().or(self.config.smtp_user.as_deref());
        let pass = db.pass.as_deref().or(self.config.smtp_pass.as_deref());
        let from = db.from.as_deref().or(self.config.smtp_from.as_deref());
        let port = db.port.or(self.config.smtp_port).unwrap_or(587);

        let (Some(host), Some(user), Some(pass), Some(from)) = (host, user, pass, from) else {
            tracing::warn!(
                "SMTP not configured — welcome letter not sent to {}",
                sub.email
            );
            return Ok(());
        };

        let base = self.config.public_url.trim_end_matches('/');
        let unsub = format!("{}/unsubscribe/{}", base, sub.unsubscribe_token);
        let ru = sub.lang == "ru";
        let subject = if ru {
            "Ваше имя вписано в книгу дома"
        } else {
            "Your name is in the house book"
        };
        let body_text = if ru {
            format!(
                "Дом запомнил вас.\n\n\
                 Теперь вести из мастерской — новые работы, открытие показов — будут находить \
                 вас первыми. Без шума и спешки.\n\n\
                 Если захотите, чтобы дом забыл ваше имя, эта дверь всегда открыта:\n{unsub}",
            )
        } else {
            format!(
                "The house has remembered you.\n\n\
                 Letters from the workshop — new works, the opening of showings — will now \
                 reach you first. No noise, no haste.\n\n\
                 Should you ever wish the house to forget your name, this door stays open:\n{unsub}",
            )
        };

        let email = Message::builder()
            .from(
                from.parse()
                    .map_err(|_| AppError::Internal("Invalid SMTP from address".into()))?,
            )
            .to(sub
                .email
                .parse()
                .map_err(|_| AppError::Internal("Invalid recipient address".into()))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body_text)
            .map_err(|e| AppError::Internal(format!("Email build error: {e}")))?;

        let creds = Credentials::new(user.to_string(), pass.to_string());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(host)
            .map_err(|e| AppError::Internal(format!("SMTP relay error: {e}")))?
            .port(port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("SMTP send error: {e}")))?;
        Ok(())
    }

    pub async fn remove_waitlist_entry(&self, id: uuid::Uuid) -> Result<()> {
        self.repo.remove_from_waitlist(id).await?;
        Self::log_domain_event("waitlist_removed", "waitlist", id, "ok");
        Ok(())
    }

    async fn send_waitlist_notification(&self, entry: &WaitlistEntry) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let admin_link = format!(
            "{}/admin#waitlist",
            self.config.public_url.trim_end_matches('/')
        );
        let text = format!(
            "👁 Лист ожидания\n\n\
            🏺 {}\n\
            👤 {}\n\
            📧 {}\n\
            📝 {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(&entry.figurine_name),
            escape_markdown(&entry.requester_name),
            escape_markdown(&entry.requester_email),
            escape_markdown(entry.note.as_deref().unwrap_or("—")),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let _ = self.http_client.post(&url)
            .json(&serde_json::json!({ "chat_id": chat_id, "text": text, "parse_mode": "MarkdownV2" }))
            .send().await;
        Ok(())
    }

    /// When a work becomes reachable again, send the author one Telegram digest
    /// listing everyone waiting — queue + "notify me" requests, including
    /// anonymous visitors who have no account — so the author can reach out
    /// personally. This is the handmade/author-model alternative to an
    /// automated e-commerce mailshot.
    async fn send_availability_digest(&self, figurine_id: Uuid, figurine_name: &str) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let queue = self
            .repo
            .get_waitlist_for_figurine(figurine_id)
            .await
            .unwrap_or_default();
        let notify = self
            .repo
            .get_notify_orders_for_figurine(figurine_id)
            .await
            .unwrap_or_default();
        if queue.is_empty() && notify.is_empty() {
            return Ok(());
        }

        let fmt_contact = |name: &str, email: &str, phone: Option<&str>, note: Option<&str>| {
            let phone = phone.filter(|p| !p.trim().is_empty()).unwrap_or("—");
            let note = note.filter(|n| !n.trim().is_empty()).unwrap_or("—");
            format!(
                "• {} — {} — {} — {}",
                escape_markdown(name),
                escape_markdown(email),
                escape_markdown(phone),
                escape_markdown(note),
            )
        };

        let mut sections: Vec<String> = Vec::new();
        if !queue.is_empty() {
            let lines: Vec<String> = queue
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    format!(
                        "{}\\. {}",
                        i + 1,
                        fmt_contact(
                            &e.requester_name,
                            &e.requester_email,
                            e.requester_phone.as_deref(),
                            e.note.as_deref()
                        )
                    )
                })
                .collect();
            sections.push(format!(
                "🪑 *Очередь* \\({}\\)\n{}",
                queue.len(),
                lines.join("\n")
            ));
        }
        if !notify.is_empty() {
            let lines: Vec<String> = notify
                .iter()
                .map(|o| {
                    fmt_contact(
                        &o.requester_name,
                        &o.requester_email,
                        o.requester_phone.as_deref(),
                        o.message.as_deref(),
                    )
                })
                .collect();
            sections.push(format!(
                "🔔 *Просили уведомить* \\({}\\)\n{}",
                notify.len(),
                lines.join("\n")
            ));
        }

        let admin_link = format!(
            "{}/admin#waitlist",
            self.config.public_url.trim_end_matches('/')
        );
        let text = format!(
            "✨ Работа «{}» снова доступна\n\nСвяжитесь лично с теми, кто ждал:\n\n{}\n\n🔗 [Открыть в админке]({})",
            escape_markdown(figurine_name),
            sections.join("\n\n"),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let _ = self.http_client.post(&url)
            .json(&serde_json::json!({ "chat_id": chat_id, "text": text, "parse_mode": "MarkdownV2" }))
            .send().await;
        Ok(())
    }

    async fn send_reply_email(
        &self,
        to: &str,
        figurine_name: &str,
        figurine_id: &str,
        comment_body: &str,
        reply: &str,
    ) -> Result<()> {
        use lettre::message::header::ContentType;
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

        // DB settings take precedence over env config
        let db = self.get_smtp_settings().await.unwrap_or_default();
        let host = db.host.as_deref().or(self.config.smtp_host.as_deref());
        let user = db.user.as_deref().or(self.config.smtp_user.as_deref());
        let pass = db.pass.as_deref().or(self.config.smtp_pass.as_deref());
        let from = db.from.as_deref().or(self.config.smtp_from.as_deref());
        let port = db.port.or(self.config.smtp_port).unwrap_or(587);

        let (Some(host), Some(user), Some(pass), Some(from)) = (host, user, pass, from) else {
            return Ok(());
        };

        let figurine_url = format!(
            "{}/figurines/{}",
            self.config.public_url.trim_end_matches('/'),
            figurine_id
        );
        let body_text = format!(
            "Your impression of «{figurine_name}»:\n\n\
            {comment_body}\n\n\
            — — —\n\n\
            Author's reply:\n\n\
            {reply}\n\n\
            View the figurine: {figurine_url}",
        );

        let email = Message::builder()
            .from(
                from.parse()
                    .map_err(|_| AppError::Internal("Invalid SMTP from address".into()))?,
            )
            .to(to
                .parse()
                .map_err(|_| AppError::Internal("Invalid recipient address".into()))?)
            .subject(format!("Re: Your impression of «{figurine_name}»"))
            .header(ContentType::TEXT_PLAIN)
            .body(body_text)
            .map_err(|e| AppError::Internal(format!("Email build error: {e}")))?;

        let creds = Credentials::new(user.to_string(), pass.to_string());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(host)
            .map_err(|e| AppError::Internal(format!("SMTP relay error: {e}")))?
            .port(port)
            .credentials(creds)
            .build();

        let _ = mailer.send(email).await;
        Ok(())
    }

    async fn send_comment_telegram_notification(
        &self,
        figurine_name: &str,
        author_name: &str,
        body: &str,
    ) {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return;
        };

        let admin_link = format!(
            "{}/admin#comments",
            self.config.public_url.trim_end_matches('/')
        );
        let text = format!(
            "💬 Новый комментарий\n\n\
            🏺 {}\n\
            👤 {}\n\
            📝 {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(figurine_name),
            escape_markdown(author_name),
            escape_markdown(&body.chars().take(200).collect::<String>()),
            admin_link,
        );
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let _ = self.http_client.post(&url)
            .json(&serde_json::json!({ "chat_id": chat_id, "text": text, "parse_mode": "MarkdownV2" }))
            .send().await;
    }

    pub async fn submit_comment(
        &self,
        figurine_id: Uuid,
        user: Option<&User>,
        req: &SubmitCommentRequest,
        ip: &str,
    ) -> Result<()> {
        if user.is_none() {
            self.check_comment_rate_limit(ip).await?;
        }

        let body = req.body.trim();
        if body.is_empty() {
            return Err(AppError::BadRequest("Comment body cannot be empty".into()));
        }
        if body.chars().count() > 1000 {
            return Err(AppError::BadRequest(
                "Comment is too long (max 1000 characters)".into(),
            ));
        }

        let (author_name, author_email, user_id) = if let Some(u) = user {
            (u.display_name.clone(), None::<String>, Some(u.id))
        } else {
            let name = req
                .author_name
                .as_deref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    AppError::BadRequest("Author name is required for anonymous comments".into())
                })?;
            if name.chars().count() > 100 {
                return Err(AppError::BadRequest(
                    "Name is too long (max 100 characters)".into(),
                ));
            }
            (name, req.author_email.clone(), None)
        };

        self.repo
            .insert_comment(
                figurine_id,
                user_id,
                &author_name,
                author_email.as_deref(),
                body,
            )
            .await?;
        Self::log_domain_event("comment_submitted", "figurine", figurine_id, "ok");

        let figurine_name = self
            .repo
            .get_figurine_by_id(figurine_id)
            .await?
            .map(|f| f.name)
            .unwrap_or_default();
        {
            let svc = self.clone();
            let author_name = author_name.clone();
            let body = body.to_string();
            tokio::spawn(async move {
                svc.send_comment_telegram_notification(&figurine_name, &author_name, &body)
                    .await;
            });
        }

        Ok(())
    }

    pub async fn get_figurine_comments(
        &self,
        figurine_id: Uuid,
        newest_first: bool,
    ) -> Result<Vec<CommentDto>> {
        let rows = self
            .repo
            .get_approved_comments(figurine_id, newest_first)
            .await?;
        Ok(rows
            .into_iter()
            .map(|c| CommentDto {
                id: c.id.to_string(),
                author_name: c.author_name,
                author_avatar_url: c.avatar_url,
                body: c.body,
                admin_reply: c.admin_reply,
                created_at: c.created_at.to_rfc3339(),
            })
            .collect())
    }

    pub async fn admin_list_comments(
        &self,
        only_pending: bool,
        figurine_filter: Option<Uuid>,
        newest_first: bool,
        page: i64,
        per_page: i64,
    ) -> Result<AdminCommentsPage> {
        let offset = (page - 1) * per_page;
        let (rows, total) = self
            .repo
            .get_comments_admin_page(
                only_pending,
                figurine_filter,
                newest_first,
                per_page,
                offset,
            )
            .await?;
        let pending_count = self.repo.get_pending_comments_count().await?;

        let items = rows
            .into_iter()
            .map(|(c, figurine_name)| AdminCommentDto {
                id: c.id.to_string(),
                figurine_id: c.figurine_id.to_string(),
                figurine_name,
                author_name: c.author_name,
                author_email: c.author_email,
                body: c.body,
                is_approved: c.is_approved,
                admin_reply: c.admin_reply,
                created_at: c.created_at.to_rfc3339(),
                user_id: c.user_id.map(|id| id.to_string()),
            })
            .collect();

        Ok(AdminCommentsPage {
            items,
            total,
            pending_count,
            page,
            per_page,
        })
    }

    pub async fn admin_moderate_comment(
        &self,
        id: Uuid,
        is_approved: bool,
        admin_reply: Option<&str>,
    ) -> Result<AdminCommentDto> {
        let prev = self
            .repo
            .moderate_comment(id, is_approved, admin_reply)
            .await?;
        let figurine = self.repo.get_figurine_by_id(prev.figurine_id).await?;
        let figurine_name = figurine
            .as_ref()
            .map(|f| f.name.clone())
            .unwrap_or_default();

        // Send email to commenter if reply was just set and they have an email
        let reply_is_new = admin_reply.map(|r| !r.trim().is_empty()).unwrap_or(false);
        if reply_is_new && let Some(email) = prev.author_email.as_deref() {
            let fid = prev.figurine_id.to_string();
            let _ = self
                .send_reply_email(
                    email,
                    &figurine_name,
                    &fid,
                    &prev.body,
                    admin_reply.unwrap_or(""),
                )
                .await;
        }

        Self::log_domain_event("comment_moderated", "comment", id, "ok");
        Ok(AdminCommentDto {
            id: prev.id.to_string(),
            figurine_id: prev.figurine_id.to_string(),
            figurine_name,
            author_name: prev.author_name,
            author_email: prev.author_email,
            body: prev.body,
            is_approved: prev.is_approved,
            admin_reply: prev.admin_reply,
            created_at: prev.created_at.to_rfc3339(),
            user_id: prev.user_id.map(|id| id.to_string()),
        })
    }

    pub async fn admin_delete_comment(&self, id: Uuid) -> Result<()> {
        self.repo.delete_comment(id).await?;
        Self::log_domain_event("comment_deleted", "comment", id, "ok");
        Ok(())
    }

    // === VISITOR IMPRESSIONS ===

    pub async fn submit_impression(&self, req: &SubmitImpressionRequest, ip: &str) -> Result<()> {
        // Honeypot: bots fill this hidden field, real visitors never do. Drop
        // silently — no error, so a bot can't tell it "worked" or not.
        if req.hp.as_deref().is_some_and(|v| !v.trim().is_empty()) {
            return Ok(());
        }

        self.check_rate_limit("impression", ip, 10, 3600).await?;

        let message = req.message.trim();
        if message.is_empty() {
            return Err(AppError::BadRequest("Impression cannot be empty".into()));
        }
        if message.chars().count() > 400 {
            return Err(AppError::BadRequest(
                "Impression is too long (max 400 characters)".into(),
            ));
        }
        let author_name = req
            .author_name
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        if author_name.is_some_and(|s| s.chars().count() > 100) {
            return Err(AppError::BadRequest(
                "Name is too long (max 100 characters)".into(),
            ));
        }
        let mood = req
            .mood
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());

        let rec = self
            .repo
            .insert_impression(message, author_name, mood, Some(ip))
            .await?;
        Self::log_domain_event("impression_submitted", "impression", rec.id, "ok");
        Ok(())
    }

    pub async fn get_featured_impressions(&self) -> Result<Vec<ImpressionDto>> {
        let rows = self.repo.get_featured_impressions().await?;
        Ok(rows
            .into_iter()
            .map(|i| ImpressionDto {
                id: i.id.to_string(),
                message: i.message,
                author_name: i.author_name,
                mood: i.mood,
                created_at: i.created_at.to_rfc3339(),
            })
            .collect())
    }

    pub async fn admin_list_impressions(
        &self,
        only_pending: bool,
        newest_first: bool,
        page: i64,
        per_page: i64,
    ) -> Result<AdminImpressionsPage> {
        let offset = (page - 1) * per_page;
        let (rows, total) = self
            .repo
            .get_impressions_admin_page(only_pending, newest_first, per_page, offset)
            .await?;
        let pending_count = self.repo.get_pending_impressions_count().await?;

        let items = rows
            .into_iter()
            .map(|i| AdminImpressionDto {
                id: i.id.to_string(),
                message: i.message,
                author_name: i.author_name,
                mood: i.mood,
                is_approved: i.is_approved,
                is_featured: i.is_featured,
                created_at: i.created_at.to_rfc3339(),
            })
            .collect();

        Ok(AdminImpressionsPage {
            items,
            total,
            pending_count,
            page,
            per_page,
        })
    }

    pub async fn admin_moderate_impression(
        &self,
        id: Uuid,
        is_approved: bool,
        is_featured: bool,
    ) -> Result<AdminImpressionDto> {
        let rec = self
            .repo
            .moderate_impression(id, is_approved, is_featured)
            .await?;
        Self::log_domain_event("impression_moderated", "impression", id, "ok");
        Ok(AdminImpressionDto {
            id: rec.id.to_string(),
            message: rec.message,
            author_name: rec.author_name,
            mood: rec.mood,
            is_approved: rec.is_approved,
            is_featured: rec.is_featured,
            created_at: rec.created_at.to_rfc3339(),
        })
    }

    pub async fn admin_delete_impression(&self, id: Uuid) -> Result<()> {
        self.repo.delete_impression(id).await?;
        Self::log_domain_event("impression_deleted", "impression", id, "ok");
        Ok(())
    }

    pub async fn update_profile(&self, user_id: Uuid, display_name: &str) -> Result<UserDto> {
        if display_name.trim().is_empty() {
            return Err(AppError::BadRequest("Display name required".into()));
        }
        let user = self
            .repo
            .update_user_display_name(user_id, display_name)
            .await?;
        Self::log_domain_event("profile_updated", "user", user_id, "ok");
        Ok(UserDto::from(&user))
    }

    pub async fn set_user_avatar(&self, user_id: Uuid, avatar_url: &str) -> Result<UserDto> {
        let user = self.repo.update_user_avatar(user_id, avatar_url).await?;
        Self::log_domain_event("avatar_updated", "user", user_id, "ok");
        Ok(UserDto::from(&user))
    }

    pub async fn delete_account(&self, user_id: Uuid) -> Result<()> {
        self.repo.delete_user(user_id).await?;
        Self::log_domain_event("account_deleted", "user", user_id, "ok");
        Ok(())
    }

    // ── Message threads ─────────────────────────────────────────

    fn thread_dto(
        thread: &MessageThread,
        unread: i64,
        preview: Option<String>,
    ) -> MessageThreadDto {
        MessageThreadDto {
            id: thread.id.to_string(),
            category: thread.category.clone(),
            reference_id: thread.reference_id.map(|id| id.to_string()),
            subject: thread.subject.clone(),
            status: thread.status.clone(),
            unread,
            last_message_at: thread.last_message_at.to_rfc3339(),
            created_at: thread.created_at.to_rfc3339(),
            preview: preview.map(|p| {
                if p.chars().count() > 80 {
                    format!("{}…", &p.chars().take(80).collect::<String>())
                } else {
                    p
                }
            }),
        }
    }

    pub async fn get_user_threads(&self, user_id: Uuid) -> Result<Vec<MessageThreadDto>> {
        let rows = self.repo.get_user_threads(user_id).await?;
        Ok(rows
            .iter()
            .map(|(t, unread, preview)| Self::thread_dto(t, *unread, preview.clone()))
            .collect())
    }

    pub async fn count_unread_threads(&self, user_id: Uuid) -> Result<i64> {
        self.repo.count_unread_threads(user_id).await
    }

    /// Build message DTOs, batch-loading attachments for all messages at once.
    async fn messages_with_attachments(
        &self,
        messages: &[ThreadMessage],
    ) -> Result<Vec<ThreadMessageDto>> {
        let ids: Vec<Uuid> = messages.iter().map(|m| m.id).collect();
        let mut atts_by_message = self.repo.get_attachments_for_messages(&ids).await?;
        Ok(messages
            .iter()
            .map(|m| {
                let atts = atts_by_message.remove(&m.id).unwrap_or_default();
                ThreadMessageDto::from_with_attachments(
                    m,
                    atts.iter().map(AttachmentDto::from).collect(),
                )
            })
            .collect())
    }

    async fn message_dto_with_attachments(&self, msg: &ThreadMessage) -> Result<ThreadMessageDto> {
        let atts = self.repo.get_message_attachments(msg.id).await?;
        Ok(ThreadMessageDto::from_with_attachments(
            msg,
            atts.iter().map(AttachmentDto::from).collect(),
        ))
    }

    pub async fn get_thread_detail(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
    ) -> Result<ThreadDetailDto> {
        let (thread, messages) = self
            .repo
            .get_thread_messages(thread_id, Some(user_id))
            .await?;
        self.repo.mark_thread_read(thread_id, user_id).await?;
        let preview = messages.last().map(|m| m.body.clone());
        Ok(ThreadDetailDto {
            thread: Self::thread_dto(&thread, 0, preview),
            messages: self.messages_with_attachments(&messages).await?,
            user: None,
        })
    }

    pub async fn user_create_thread(
        &self,
        user_id: Uuid,
        subject: String,
        body: String,
        category: Option<String>,
        attachments: Vec<AttachmentInput>,
    ) -> Result<ThreadDetailDto> {
        validate_text("Subject", &subject, 200)?;
        validate_text("Message", &body, 5000)?;
        validate_attachments(&attachments)?;
        let category = category.unwrap_or_else(|| "general".to_string());
        let (thread, msg) = self
            .repo
            .create_thread(
                user_id,
                &category,
                None,
                &subject,
                &body,
                false,
                &attachments,
            )
            .await?;
        Self::log_domain_event("thread_created", "thread", thread.id, "ok");
        Ok(ThreadDetailDto {
            thread: Self::thread_dto(&thread, 0, Some(msg.body.clone())),
            messages: vec![self.message_dto_with_attachments(&msg).await?],
            user: None,
        })
    }

    pub async fn user_reply_to_thread(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
        body: String,
        attachments: Vec<AttachmentInput>,
    ) -> Result<ThreadMessageDto> {
        validate_text("Message", &body, 5000)?;
        validate_attachments(&attachments)?;
        let (thread, _) = self
            .repo
            .get_thread_messages(thread_id, Some(user_id))
            .await?;
        if thread.status == "resolved" {
            self.repo.reopen_thread(thread_id).await?;
        }
        let msg = self
            .repo
            .add_thread_reply(thread_id, user_id, false, &body, &attachments)
            .await?;
        Self::log_domain_event("thread_replied", "thread", thread_id, "ok");
        self.message_dto_with_attachments(&msg).await
    }

    pub async fn admin_create_thread(
        &self,
        user_id: Uuid,
        subject: String,
        body: String,
        category: Option<String>,
        reference_id: Option<Uuid>,
        attachments: Vec<AttachmentInput>,
    ) -> Result<ThreadDetailDto> {
        validate_attachments(&attachments)?;
        let category = category.unwrap_or_else(|| "general".to_string());
        let (thread, msg) = self
            .repo
            .create_thread(
                user_id,
                &category,
                reference_id,
                &subject,
                &body,
                true,
                &attachments,
            )
            .await?;
        let user = self.repo.find_user_by_id(user_id).await?;
        Self::log_domain_event("admin_thread_created", "thread", thread.id, "ok");
        Ok(ThreadDetailDto {
            thread: Self::thread_dto(&thread, 0, Some(msg.body.clone())),
            messages: vec![self.message_dto_with_attachments(&msg).await?],
            user: user.map(|u| ThreadUserDto {
                id: u.id.to_string(),
                display_name: u.display_name,
                email: u.email,
            }),
        })
    }

    pub async fn admin_reply_to_thread(
        &self,
        thread_id: Uuid,
        body: String,
        attachments: Vec<AttachmentInput>,
    ) -> Result<ThreadMessageDto> {
        validate_attachments(&attachments)?;
        let msg = self
            .repo
            .add_thread_reply(thread_id, uuid::Uuid::nil(), true, &body, &attachments)
            .await?;
        Self::log_domain_event("admin_thread_replied", "thread", thread_id, "ok");
        self.message_dto_with_attachments(&msg).await
    }

    pub async fn admin_get_thread_detail(&self, thread_id: Uuid) -> Result<ThreadDetailDto> {
        let (thread, messages) = self.repo.get_thread_messages(thread_id, None).await?;
        self.repo.mark_thread_read_admin(thread_id).await?;
        let user = self.repo.find_user_by_id(thread.user_id).await?;
        let preview = messages.last().map(|m| m.body.clone());
        Ok(ThreadDetailDto {
            thread: Self::thread_dto(&thread, 0, preview),
            messages: self.messages_with_attachments(&messages).await?,
            user: user.map(|u| ThreadUserDto {
                id: u.id.to_string(),
                display_name: u.display_name,
                email: u.email,
            }),
        })
    }

    pub async fn admin_list_threads(
        &self,
        category: Option<String>,
        status: Option<String>,
        page: i64,
        per_page: i64,
    ) -> Result<serde_json::Value> {
        let (rows, total) = self
            .repo
            .admin_get_threads(category.as_deref(), status.as_deref(), page, per_page)
            .await?;
        let items: Vec<serde_json::Value> = rows.iter().map(|(thread, user, unread, preview)| {
            let dto = Self::thread_dto(thread, *unread, preview.clone());
            serde_json::json!({
                "thread": dto,
                "user": { "id": user.id.to_string(), "displayName": user.display_name, "email": user.email }
            })
        }).collect();
        Ok(serde_json::json!({ "items": items, "total": total, "page": page, "perPage": per_page }))
    }

    pub async fn admin_resolve_thread(&self, thread_id: Uuid) -> Result<()> {
        self.repo.resolve_thread(thread_id).await?;
        Self::log_domain_event("thread_resolved", "thread", thread_id, "ok");
        Ok(())
    }

    pub async fn admin_reopen_thread(&self, thread_id: Uuid) -> Result<()> {
        self.repo.reopen_thread(thread_id).await?;
        Self::log_domain_event("thread_reopened", "thread", thread_id, "ok");
        Ok(())
    }

    pub async fn admin_get_user_threads(&self, user_id: Uuid) -> Result<Vec<MessageThreadDto>> {
        let rows = self.repo.get_user_threads_for_admin(user_id).await?;
        Ok(rows
            .iter()
            .map(|(t, unread, preview)| Self::thread_dto(t, *unread, preview.clone()))
            .collect())
    }
}

#[cfg(test)]
mod tests;
