use crate::config::Config;
use crate::db::Repository;
use crate::error::{AppError, Result};
use crate::models::*;
use crate::observability::ObservabilityState;
use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Datelike, Utc};
use reqwest::Client;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use base64::Engine as _;
use uuid::Uuid;

mod insights;
mod oauth_login;
mod readiness;
pub use insights::{BattleReport, BehaviourReport, HealthReport, PipelineHealth};

#[derive(Clone)]
pub struct AppService {
    repo: Repository,
    config: Config,
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
    /// Пределы для горячих ручек (замер, поиск) — в памяти, окном в минуту. Базе
    /// их доверять нельзя: это самые частые запросы сайта, и счёт в базе на каждый
    /// из них при наплыве давал бы наплыву ещё и нагрузку на базу. Перезапуск
    /// обнуляет — для защиты от потока этого достаточно.
    hot_limits: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
    /// Ответы поиска на минуту: один и тот же вопрос (его задают подряд при
    /// каждом нажатии клавиши и при каждой перезагрузке) не должен каждый раз
    /// гонять кодировщик.
    search_cache: Arc<Mutex<HashMap<String, (Instant, Vec<SemanticHit>)>>>,
    /// One reused HTTP client for all outbound notifications (Telegram). Reqwest's
    /// Client holds a connection pool and is meant to be created once and cloned —
    /// building a fresh one per notification threw away connection/TLS reuse. It
    /// also carries a request timeout so a hung endpoint can't stall a caller.
    http_client: Client,
    /// Протокол входа через Google и Apple (`crate::oauth`). Ключи провайдеров
    /// кэшируются внутри, поэтому он один на сервис, а не строится на запрос.
    oauth: crate::oauth::Oauth,
}

/// Widest rate-limit window in use anywhere (the daily upload cap); the GC keeps
/// hits at least this fresh, so a swept key can never wrongly reject (its data
/// was already expired for every bucket).
const RATE_LIMIT_WIDEST_WINDOW: Duration = Duration::from_secs(86_400);

/// Сколько живёт ответ поиска и сколько ответов держим.
const SEARCH_CACHE_TTL: Duration = Duration::from_secs(60);
const SEARCH_CACHE_MAX: usize = 200;

/// Начало кода в ссылке подписки на книгу дома в Telegram (`?start=sub_…`):
/// по нему бот отличает подписку от входа.
const TG_SUBSCRIBE_PREFIX: &str = "sub_";

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
        // Настройки проверены при запуске (`Config::validate`), поэтому здесь
        // ошибка означает только одно: в код попало то, чего проверка не знает.
        let oauth = crate::oauth::Oauth::new(
            http_client.clone(),
            crate::oauth::Settings {
                public_url: &config.public_url,
                google: config
                    .google_client_id
                    .as_deref()
                    .zip(config.google_client_secret.as_deref()),
                // Apple принимает только https на настоящем домене: на своей
                // машине кнопка остаётся выключенной (об этом уже сказано при
                // запуске), а не ведёт к отказу провайдера.
                apple: match (
                    config.apple_client_id.as_deref(),
                    config.apple_team_id.as_deref(),
                    config.apple_key_id.as_deref(),
                    config.apple_private_key.as_deref(),
                ) {
                    (Some(client_id), Some(team_id), Some(key_id), Some(private_key))
                        if !config.public_url_is_local() =>
                    {
                        Some(crate::oauth::AppleSettings { client_id, team_id, key_id, private_key })
                    }
                    _ => None,
                },
            },
        )
        .expect("OAuth settings were validated at startup");
        Self {
            repo,
            config,
            geoip,
            observability: ObservabilityState::default(),
            analytics,
            favorite_tiers_cache: Arc::new(Mutex::new(None)),
            hot_limits: Arc::new(Mutex::new(HashMap::new())),
            search_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client,
            oauth,
        }
    }

    /// Только для проверок: сервис с другим протоколом входа (поддельный
    /// провайдер на своём порту). В работе протокол собирается из окружения.
    pub fn with_oauth(mut self, oauth: crate::oauth::Oauth) -> Self {
        self.oauth = oauth;
        self
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

    /// Предел «не более `max` обращений за `window_secs`» под ключом
    /// «корзина|адрес». Счёт лежит в базе (`rate_limit_hits`), а не в памяти:
    /// перезапуск сервера его не обнуляет.
    pub async fn check_rate_limit(
        &self,
        bucket: &str,
        ip: &str,
        max: usize,
        window_secs: u64,
    ) -> Result<()> {
        if self
            .repo
            .rate_limit_take(&format!("{bucket}|{ip}"), max as i64, window_secs as i64)
            .await?
        {
            Ok(())
        } else {
            Err(AppError::BadRequest(
                "Too many requests from this address. Please wait and try again.".into(),
            ))
        }
    }

    /// Предел для горячих ручек: не более `max` обращений за окно в `window_secs`
    /// секунд (окно фиксированное, счёт в памяти). Для ручек, которые зовут
    /// чаще всего: счёт в базе на каждое такое обращение сам был бы нагрузкой.
    pub async fn check_hot_limit(
        &self,
        bucket: &str,
        ip: &str,
        max: u32,
        window_secs: u64,
    ) -> Result<()> {
        let now = Instant::now();
        let window = Duration::from_secs(window_secs);
        let mut map = self.hot_limits.lock().await;
        let entry = map
            .entry(format!("{bucket}|{ip}"))
            .or_insert((now, 0));
        if now.duration_since(entry.0) >= window {
            *entry = (now, 0);
        }
        if entry.1 >= max {
            return Err(AppError::BadRequest(
                "Too many requests from this address. Please wait and try again.".into(),
            ));
        }
        entry.1 += 1;
        Ok(())
    }

    pub async fn check_commission_rate_limit(&self, ip: &str) -> Result<()> {
        if self
            .repo
            .rate_limit_take(&format!("commission|{ip}"), 6, 3600)
            .await?
        {
            Ok(())
        } else {
            Err(AppError::BadRequest(
                "Too many requests from this address. Please wait before submitting again.".into(),
            ))
        }
    }

    /// Периодическая уборка: стирает обращения старше самого широкого окна.
    /// Безопасна на любом интервале — трогает только уже истёкшее. Запускается
    /// по таймеру из main.rs.
    pub async fn prune_rate_limiters(&self) -> usize {
        let now = Instant::now();
        {
            let mut hot = self.hot_limits.lock().await;
            // Самое длинное окно горячих ручек — минута.
            hot.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(120));
            let mut cache = self.search_cache.lock().await;
            cache.retain(|_, (at, _)| now.duration_since(*at) < SEARCH_CACHE_TTL);
        }
        match self
            .repo
            .prune_rate_limit_hits(RATE_LIMIT_WIDEST_WINDOW.as_secs() as i64)
            .await
        {
            Ok(n) => n as usize,
            Err(e) => {
                tracing::warn!("rate-limit prune failed: {e}");
                0
            }
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        Ok(()) // Postgres is always ready, no pool to load
    }

    pub async fn health_check(&self) -> Result<()> {
        self.repo.health_check().await
    }

    // === FIGURINE ANALYTICS ===

    /// Отпечатки посетителя, под которыми замер мог его записать: по ним пыль за
    /// внимание сверяет, видел ли сервер человека на странице.
    pub fn attention_hashes(&self, headers: &axum::http::HeaderMap) -> Vec<String> {
        crate::analytics::visitor_hashes_for_attention(
            headers,
            &self.config.analytics_hash_secret,
            &self.config.admin_api_key,
        )
    }

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
            .get_admin_figurine_analytics_breakdown(figurine_id, raw_data_from, to, "referrer", 12)
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
        self.repo
            .get_admin_figurine_geo_daily(figurine_id, from, to)
            .await
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

    /// Tale pages with and without their scene: counts per tale and arm.
    /// Clamped to the raw-event floor like the commission funnel, and for the
    /// same reason — the variant lives only on raw events.
    pub async fn admin_get_scene_experiment(
        &self,
        query: AdminAnalyticsQuery,
    ) -> Result<SceneExperimentResponse> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let raw_data_from = Self::raw_data_floor().max(from);
        let arms = self
            .repo
            .get_admin_scene_experiment(raw_data_from, to)
            .await?;
        Ok(SceneExperimentResponse {
            from,
            to,
            raw_data_from,
            arms,
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
        let engagement = self
            .repo
            .get_admin_site_page_engagement(raw_from, to)
            .await?;
        let views = self
            .repo
            .get_admin_site_page_views_by_group(from, to)
            .await?;

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
            None => self.repo.get_earliest_analytics_day().await?.unwrap_or(to),
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
    /// То же, но только для работ на людях. Всё, что посетитель делает с
    /// работой по адресу (расписание, отклик, бронь, очередь), отвечает на
    /// скрытую работу так же, как на несуществующую: она не вышла, и знать о
    /// ней посетителю нечего.
    pub async fn resolve_public_figurine_uuid(&self, handle: &str) -> Result<Uuid> {
        let uuid = self.resolve_figurine_uuid(handle).await?;
        if !self.repo.figurine_is_visible(uuid).await? {
            return Err(AppError::NotFound(format!("Figurine {} not found", handle)));
        }
        Ok(uuid)
    }

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
            series: f.series,
            technique: f.technique,
            material: f.material,
            dimensions: f.dimensions,
            is_featured: f.is_featured,
            is_visible: f.is_visible,
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
            similar_ids: Vec::new(),
            translations: std::collections::HashMap::new(),
        }
    }

    /// Запомнить язык записи и человека. Незнакомый код молча пропускается:
    /// язык — удобство, а не условие, и запись из-за него не падает.
    async fn note_lang(&self, table: &'static str, id: Uuid, user: Option<Uuid>, raw: Option<&str>) {
        let Some(lang) = raw.and_then(crate::lang::Lang::parse) else {
            return;
        };
        if let Err(e) = self.repo.remember_lang(table, id, user, lang.code()).await {
            tracing::warn!("could not remember the language of {table} {id}: {e}");
        }
    }

    /// На каком языке писать о записи: её язык, язык человека, английский.
    async fn lang_for(&self, table: &'static str, id: Uuid) -> crate::lang::Lang {
        self.repo
            .lang_of_record(table, id)
            .await
            .ok()
            .flatten()
            .and_then(|l| crate::lang::Lang::parse(&l))
            .unwrap_or_default()
    }

    /// Тексты работ на других языках для готового списка карточек — только
    /// поля списка. Один запрос на весь список.
    async fn add_list_translations(&self, items: &mut [FigurineListItemDto]) -> Result<()> {
        let ids: Vec<Uuid> = items.iter().filter_map(|i| Uuid::parse_str(&i.id).ok()).collect();
        let mut all = self.repo.get_figurine_translations(&ids).await?;
        for item in items.iter_mut() {
            let Ok(id) = Uuid::parse_str(&item.id) else { continue };
            item.translations = all
                .remove(&id)
                .unwrap_or_default()
                .into_iter()
                .map(|(lang, t)| (lang, t.for_list()))
                .filter(|(_, t)| !t.is_empty())
                .collect();
        }
        Ok(())
    }

    /// Названия одной работы на других языках, по строковому номеру. Номер не
    /// разобрался — названий нет: ответ из-за этого не падает.
    async fn name_translations_of(&self, id: &str) -> std::collections::HashMap<String, String> {
        let Ok(uuid) = Uuid::parse_str(id.trim()) else {
            return Default::default();
        };
        self.repo
            .get_figurine_name_translations(&[uuid])
            .await
            .ok()
            .and_then(|mut m| m.remove(&uuid))
            .unwrap_or_default()
    }

    /// Название работы на языке человека: перевод, а без него — английское.
    async fn figurine_name_in(&self, id: Uuid, english: &str, lang: crate::lang::Lang) -> String {
        if lang == crate::lang::Lang::En {
            return english.to_string();
        }
        self.name_translations_of(&id.to_string())
            .await
            .remove(lang.code())
            .unwrap_or_else(|| english.to_string())
    }

    /// Batch "House Favorite" lookup for a list of figurines about to be
    /// rendered as cards — computed once (percentile over the whole
    /// collection) and intersected with the ids actually being rendered.
    async fn house_favorite_ids(&self, ids: &[Uuid]) -> Result<std::collections::HashSet<Uuid>> {
        let tiers = self.favorite_tiers().await?;
        Ok(ids
            .iter()
            .filter(|id| tiers.house_favorite.contains(id))
            .copied()
            .collect())
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

    pub async fn list_figurines(
        &self,
        visible_only: bool,
        mut query: crate::models::FigurineQuery,
    ) -> Result<crate::models::FigurinesPage> {
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
        // Публично показываются только похожие, которые сами на людях.
        let mut similar = self.repo.get_similar_for_figurines(&ids, !visible_only).await?;
        let mut translations = self.repo.get_figurine_translations(&ids).await?;
        let items = figurines
            .into_iter()
            .map(|f| {
                let face = faces.get(&f.id);
                let detail = details.get(&f.id);
                let favorite = favorites.contains(&f.id);
                let similar_ids = similar
                    .remove(&f.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|u| u.to_string())
                    .collect();
                let texts = translations.remove(&f.id).unwrap_or_default();
                let mut item = self.to_list_item(f, face, detail, favorite);
                item.similar_ids = similar_ids;
                item.translations = texts
                    .into_iter()
                    .map(|(lang, t)| (lang, t.for_list()))
                    .filter(|(_, t)| !t.is_empty())
                    .collect();
                item
            })
            .collect();
        Ok(crate::models::FigurinesPage {
            items,
            total,
            page,
            per_page,
        })
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
        let mut items: Vec<FigurineListItemDto> = figurines
            .into_iter()
            .map(|f| {
                let face = faces.get(&f.id);
                let detail = details.get(&f.id);
                let favorite = favorites.contains(&f.id);
                self.to_list_item(f, face, detail, favorite)
            })
            .collect();
        self.add_list_translations(&mut items).await?;
        Ok(items)
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
        let mut items: Vec<FigurineListItemDto> = figurines
            .into_iter()
            .map(|f| {
                let face = faces.get(&f.id);
                let detail = details.get(&f.id);
                let favorite = favorites.contains(&f.id);
                self.to_list_item(f, face, detail, favorite)
            })
            .collect();
        self.add_list_translations(&mut items).await?;
        Ok(items)
    }

    pub async fn get_figurine_details(&self, id: String, is_admin: bool) -> Result<FigurineDto> {
        // `id` is a handle: a UUID for legacy/canonical links, or a transliterated
        // slug for the pretty URL. Try UUID first (cheap parse); anything else is a
        // slug lookup. Either resolves to the same work.
        let figurine = match Uuid::parse_str(&id) {
            Ok(uuid) => self.repo.get_figurine_by_id(uuid).await?,
            Err(_) => self.repo.get_figurine_by_slug(&id).await?,
        }
        .ok_or_else(|| crate::error::AppError::NotFound(format!("Figurine {} not found", id)))?;

        // Не на людях — нет и для того, кто знает адрес. Ответ тот же, что и на
        // несуществующую: по нему нельзя узнать, что такая работа заведена.
        if !figurine.is_visible && !is_admin {
            return Err(crate::error::AppError::NotFound(format!("Figurine {} not found", id)));
        }

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
        let mut related_items = related_items;
        self.add_list_translations(&mut related_items).await?;

        let mut alt_translations = self.repo.get_image_alt_translations(uuid).await?;
        let image_dtos = images
            .into_iter()
            .map(|i| {
                let i_id_str = i.id.to_string();
                let alt_translations = alt_translations.remove(&i.id).unwrap_or_default();
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
                    alt_translations,
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
            series: figurine.series,
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
            catalog_lists: figurine.catalog_lists,
            first_look_until: figurine.first_look_until,
            images: image_dtos,
            process_steps: step_dtos,
            related_items,
            noticed_by_others,
            house_favorite,
            similar_ids: self
                .repo
                .get_similar_for_figurines(&[uuid], is_admin)
                .await?
                .remove(&uuid)
                .unwrap_or_default()
                .into_iter()
                .map(|u| u.to_string())
                .collect(),
            translations: self
                .repo
                .get_figurine_translations(&[uuid])
                .await?
                .remove(&uuid)
                .unwrap_or_default(),
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
    ///
    /// Translated name and short text are appended in a fixed language order
    /// (`FigurineTextDto::LANGS`), so a query in Spanish or German finds the
    /// work by meaning too, and the fingerprint changes when a translation does.
    pub(crate) fn curatorial_text(
        f: &Figurine,
        texts: Option<&std::collections::HashMap<String, crate::models::FigurineTextDto>>,
    ) -> String {
        let opt = |s: &Option<String>| -> Option<String> {
            s.as_ref()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let mut parts: Vec<String> = vec![f.name.trim().to_string()];
        for field in [
            opt(&f.series),
            opt(&f.short_text),
            opt(&f.full_description),
            // Backstage AI description of what the photo shows — lets a visual
            // query ("монах со свечой") match through the text encoder.
            opt(&f.visual_caption),
            // Pinterest copy is visitor-invisible but keyword-rich; search-only.
            opt(&f.pinterest_description),
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
        if let Some(texts) = texts {
            for lang in crate::models::FigurineTextDto::LANGS {
                let Some(t) = texts.get(lang) else { continue };
                for field in [opt(&t.name), opt(&t.short_text)] {
                    if let Some(v) = field {
                        parts.push(v);
                    }
                }
            }
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
        let texts = self.repo.get_figurine_translations(&[id]).await?.remove(&id);
        let text = Self::curatorial_text(&fig, texts.as_ref());
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

    /// Rank publicly-visible works for a natural-language query. Hybrid: e5
    /// cosine (when weights are bundled), word BM25 (AUTO-fuzzed against the
    /// cabinet vocabulary), and pg_trgm-style trigram word-similarity, fused
    /// with RRF. Empty when the query is blank. Degrades to BM25+trigrams
    /// when embeddings are absent.
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<SemanticHit>> {
        let key = format!("{limit}|{}", query.trim().to_lowercase());
        if let Some((at, hits)) = self.search_cache.lock().await.get(&key)
            && at.elapsed() < SEARCH_CACHE_TTL
        {
            return Ok(hits.clone());
        }
        let hits = self.semantic_search_uncached(query, limit).await?;
        let mut cache = self.search_cache.lock().await;
        if cache.len() >= SEARCH_CACHE_MAX {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), hits.clone()));
        Ok(hits)
    }

    async fn semantic_search_uncached(&self, query: &str, limit: usize) -> Result<Vec<SemanticHit>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        // Restrict to works that are currently public (respects hide/first-look
        // even if a vector is momentarily stale).
        let fq = FigurineQuery {
            status: None,
            search: None,
            sort: None,
            page: None,
            per_page: None,
        };
        let (figs, _) = self.repo.get_all_figurines(true, &fq).await?;
        let visible: HashSet<Uuid> = figs.iter().map(|f| f.id).collect();
        let records = crate::search::records_from_figurines(&figs);

        let mut vector_scores: Vec<(String, f32)> = Vec::new();
        if crate::embed::is_available() {
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

            vector_scores = rows
                .into_iter()
                .filter(|(id, _)| visible.contains(id))
                .filter_map(|(id, bytes)| {
                    crate::embed::from_bytes(&bytes)
                        .map(|v| (id.to_string(), crate::embed::dot(&qv, &v)))
                })
                .filter(|(_, score)| *score >= Self::SEMANTIC_FLOOR)
                .collect();
        }

        Ok(crate::search::hybrid_search(
            query,
            &records,
            &vector_scores,
            limit,
        ))
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

    /// Batched Pinterest SEO description lookup for feed_rss — see
    /// `get_figurine_pinterest_description` for the single-item admin editor version.
    pub async fn get_pinterest_descriptions(
        &self,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<String, String>> {
        self.repo.get_pinterest_descriptions(ids).await
    }

    /// Read a work's admin-only Pinterest SEO description (never public).
    pub async fn get_figurine_pinterest_description(&self, id: String) -> Result<Option<String>> {
        let uuid = Self::parse_uuid(&id)?;
        Ok(self
            .repo
            .get_figurine_by_id(uuid)
            .await?
            .and_then(|f| f.pinterest_description))
    }

    /// Set (or clear, when blank) a work's Pinterest SEO description. Deliberately
    /// separate from save_figurine, like the search caption above, so ordinary
    /// edits never touch it and the field never round-trips through the shared
    /// figurine DTO.
    pub async fn set_figurine_pinterest_description(
        &self,
        id: String,
        description: Option<String>,
    ) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        let desc = description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        sqlx::query("UPDATE figurines SET pinterest_description = $1 WHERE id = $2")
            .bind(&desc)
            .bind(uuid)
            .execute(self.repo.pg_pool())
            .await?;
        Ok(())
    }

    /// Bulk admin action: set every image's darkness to 0, fully dissolving
    /// the keyhole shadow (the CSS gradient collapses to zero alpha at 0, so
    /// this switches the veil off entirely rather than reverting to the
    /// global default, which is still dark).
    pub async fn bulk_clear_darkness(&self) -> Result<crate::models::BulkOpSummary> {
        let affected =
            sqlx::query("UPDATE images SET darkness = 0 WHERE darkness IS DISTINCT FROM 0")
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

    /// Set the registry order from a full list of ids: 1, 2, 3, … in the order given.
    /// Renumbering the whole list (rather than nudging one value by ±1) is what makes the
    /// order total — the old scheme left ties, and a tie is resolved by creation date.
    pub async fn reorder_figurines(&self, ids: Vec<String>) -> Result<crate::models::BulkOpSummary> {
        if ids.is_empty() || ids.len() > 5000 {
            return Err(AppError::BadRequest("Order must list 1..5000 works".into()));
        }
        let mut seen = std::collections::HashSet::new();
        let mut parsed = Vec::with_capacity(ids.len());
        for raw in &ids {
            let id = Self::parse_uuid(raw)?;
            if !seen.insert(id) {
                return Err(AppError::BadRequest("Duplicate work in order".into()));
            }
            parsed.push(id);
        }
        let affected = self.repo.reorder_figurines(&parsed).await?;
        Self::log_domain_event("figurines_reordered", "figurine", affected, "ok");
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
                        "Image focal point / reveal radius / darkness must be between 0 and 1"
                            .into(),
                    ));
                }
            }
        }
        for value in [req.open_from_min, req.open_until_min]
            .into_iter()
            .flatten()
        {
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
                let _ = svc.send_sketch_watch_letters(figurine_id, &name).await;
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
        let manual = slug
            .as_deref()
            .map(str::trim)
            .is_some_and(|s| !s.is_empty());
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

    pub async fn get_showing_rooms(&self) -> Result<Vec<crate::models::ShowingRoomDto>> {
        let rooms = self.repo.get_showing_rooms().await?;
        Ok(rooms
            .into_iter()
            .map(|r| crate::models::ShowingRoomDto {
                id: r.id.to_string(),
                name: r.name,
                name_ru: r.name_ru.filter(|s| !s.trim().is_empty()),
                name_uk: r.name_uk.filter(|s| !s.trim().is_empty()),
                name_es: r.name_es.filter(|s| !s.trim().is_empty()),
                name_de: r.name_de.filter(|s| !s.trim().is_empty()),
                name_it: r.name_it.filter(|s| !s.trim().is_empty()),
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
        for name in [&req.name_ru, &req.name_uk, &req.name_es, &req.name_de, &req.name_it]
            .into_iter()
            .flatten()
        {
            if name.trim().chars().count() > 200 {
                return Err(AppError::BadRequest("Room name is longer than 200 characters".into()));
            }
        }
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
        let before = self.repo.get_home_content().await?.unwrap_or_default();
        let content = content.keep_translations(&before);
        self.repo.save_home_content(&content).await?;
        Self::log_domain_event("home_content_saved", "setting", "home_content", "ok");
        Ok(())
    }

    // === AUTHOR PROFILE ===

    pub async fn get_author_profile(&self) -> Result<AuthorProfile> {
        Ok(self.repo.get_author_profile().await?.unwrap_or_default())
    }

    pub async fn save_author_profile(&self, profile: AuthorProfile) -> Result<()> {
        let before = self.repo.get_author_profile().await?.unwrap_or_default();
        let profile = profile.keep_translations(&before);
        self.repo.save_author_profile(&profile).await?;
        Self::log_domain_event("author_profile_saved", "setting", "author_profile", "ok");
        Ok(())
    }

    // === ORDERS / NOTIFICATIONS ===

    pub async fn create_order(&self, order: &OrderRequest, user_id: Option<Uuid>) -> Result<Order> {
        if !order.age_confirmed {
            return Err(AppError::BadRequest(
                "Please confirm you are 16 or older".to_string(),
            ));
        }
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
        self.note_lang("orders", saved.id, user_id, order.lang.as_deref()).await;
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
            let lang = self.lang_for("orders", order.id).await;
            let name = match Uuid::parse_str(&order.figurine_id) {
                Ok(fid) => self.figurine_name_in(fid, &order.figurine_name, lang).await,
                Err(_) => order.figurine_name.clone(),
            };
            let (subject, said) = order_reply_words(lang, &name);
            let body = match order.admin_notes.as_deref() {
                Some(n) if !n.is_empty() => format!("{said}\n\n{n}"),
                _ => said,
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
        let figurine_name_translations = self.name_translations_of(&c.figurine_id).await;
        Ok(Some(PublicCertificateDto {
            token: c.token,
            certificate_number: c.certificate_number,
            figurine_id: c.figurine_id,
            figurine_name: c.figurine_name,
            figurine_name_translations,
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
        if !req.age_confirmed {
            return Err(crate::error::AppError::BadRequest(
                "Please confirm you are 16 or older".to_string(),
            ));
        }
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
            } else {
                crate::lang::Lang::parse(&commission.lang)
                    .unwrap_or_default()
                    .pick(
                        "A petition for a new figurine",
                        "Прошение о новой фигурке",
                        "Прохання про нову фігурку",
                        "Solicitud de una figura nueva",
                        "Anfrage für eine neue Figur",
                        "Richiesta di una nuova figura",
                    )
                    .to_string()
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
            let lang = crate::lang::Lang::parse(&existing.lang).unwrap_or_default();
            let titled = !existing.title.trim().is_empty();
            let subject = lang.pick(
                "Petition removed",
                "Прошение снято",
                "Прохання знято",
                "Solicitud retirada",
                "Anfrage entfernt",
                "Richiesta ritirata",
            );
            let body = match (lang, titled) {
                (crate::lang::Lang::En, true) => format!(
                    "Your petition “{}” has been removed by the archive keeper. You are welcome to send a new one.",
                    existing.title
                ),
                (crate::lang::Lang::En, false) => "Your petition has been removed by the archive keeper. You are welcome to send a new one.".to_string(),
                (crate::lang::Lang::Ru, true) => format!(
                    "Ваше прошение «{}» снято хранителем архива. Вы можете отправить новое.",
                    existing.title
                ),
                (crate::lang::Lang::Ru, false) => "Ваше прошение снято хранителем архива. Вы можете отправить новое.".to_string(),
                (crate::lang::Lang::Uk, true) => format!(
                    "Ваше прохання «{}» знято зберігачем архіву. Ви можете надіслати нове.",
                    existing.title
                ),
                (crate::lang::Lang::Uk, false) => "Ваше прохання знято зберігачем архіву. Ви можете надіслати нове.".to_string(),
                (crate::lang::Lang::Es, true) => format!(
                    "Su solicitud «{}» ha sido retirada del archivo. Puede enviar una nueva.",
                    existing.title
                ),
                (crate::lang::Lang::Es, false) => "Su solicitud ha sido retirada del archivo. Puede enviar una nueva.".to_string(),
                (crate::lang::Lang::De, true) => format!(
                    "Ihre Anfrage „{}“ wurde aus dem Archiv entfernt. Sie können gern eine neue senden.",
                    existing.title
                ),
                (crate::lang::Lang::De, false) => "Ihre Anfrage wurde aus dem Archiv entfernt. Sie können gern eine neue senden.".to_string(),
                (crate::lang::Lang::It, true) => format!(
                    "La sua richiesta «{}» è stata ritirata dall’archivio. Può inviarne una nuova.",
                    existing.title
                ),
                (crate::lang::Lang::It, false) => "La sua richiesta è stata ritirata dall’archivio. Può inviarne una nuova.".to_string(),
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
                let lang = crate::lang::Lang::parse(&c.lang).unwrap_or_default();
                let label = match status {
                    CommissionStatus::Accepted => Some(lang.pick(
                        "Your petition is accepted — the master takes up the work.",
                        "Ваше прошение принято — мастер берётся за работу.",
                        "Ваше прохання прийнято — майстер береться за роботу.",
                        "Su solicitud ha sido aceptada: la autora empieza el trabajo.",
                        "Ihre Anfrage ist angenommen: die Künstlerin beginnt mit der Arbeit.",
                        "La sua richiesta è stata accettata: l’autrice inizia il lavoro.",
                    )),
                    CommissionStatus::InProgress => Some(lang.pick(
                        "The master has begun your figurine.",
                        "Мастер приступил к вашей фигурке.",
                        "Майстер приступив до вашої фігурки.",
                        "La autora ha empezado su figura.",
                        "Die Künstlerin hat mit Ihrer Figur begonnen.",
                        "L’autrice ha iniziato la sua figura.",
                    )),
                    CommissionStatus::Completed => Some(lang.pick(
                        "Your figurine is finished.",
                        "Ваша фигурка завершена.",
                        "Вашу фігурку завершено.",
                        "Su figura está terminada.",
                        "Ihre Figur ist fertig.",
                        "La sua figura è finita.",
                    )),
                    CommissionStatus::Declined => Some(lang.pick(
                        "Regrettably, the master will not take up this petition.",
                        "К сожалению, мастер не возьмётся за это прошение.",
                        "На жаль, майстер не візьметься за це прохання.",
                        "Lamentablemente, la autora no aceptará esta solicitud.",
                        "Leider wird die Künstlerin diese Anfrage nicht annehmen.",
                        "Purtroppo l’autrice non accetterà questa richiesta.",
                    )),
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
                        } else {
                            lang.pick(
                                "Your petition",
                                "Ваше прошение",
                                "Ваше прохання",
                                "Su solicitud",
                                "Ihre Anfrage",
                                "La sua richiesta",
                            )
                                .to_string()
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
        let uuid = self.resolve_public_figurine_uuid(&figurine_id).await?;
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
        self.repo
            .set_figurine_mark(figurine_id, token, tone)
            .await?;
        Ok(tone.map(str::to_string))
    }

    pub async fn set_figurine_like(
        &self,
        figurine_id: Uuid,
        visitor_token: &str,
        user_id: Option<Uuid>,
        liked: bool,
        visitor_hashes: &[String],
    ) -> Result<(bool, i64)> {
        let token = visitor_token.trim();
        if token.is_empty() || token.len() > 64 {
            return Err(crate::error::AppError::BadRequest(
                "Invalid visitor token".to_string(),
            ));
        }
        let result = self
            .repo
            .set_figurine_like(figurine_id, token, user_id, liked)
            .await?;
        Self::log_domain_event(
            if liked {
                "figurine_liked"
            } else {
                "figurine_unliked"
            },
            "figurine",
            figurine_id,
            "ok",
        );

        // Пыль за сердечко: однажды за работу, и только тому, кто вошёл под
        // именем. Снятое сердечко ничего не возвращает — иначе появился бы
        // насос из одного щелчка туда и обратно; ключ книги (`liked:{id}`) и
        // так не даст заплатить за ту же работу дважды.
        //
        // Ошибка кошелька не отменяет сердечка: человек пришёл отметить работу,
        // а не пополнить книгу, и споткнувшаяся пыль не должна выглядеть как
        // «не удалось поставить отметку».
        if liked && let Some(uid) = user_id {
            if let Err(e) = self.grant_attention_dust(uid, "liked", figurine_id, visitor_hashes).await {
                tracing::warn!(%figurine_id, "пыль за сердечко не осела: {e}");
            }
        }
        Ok(result)
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
        let faces = self
            .repo
            .get_face_images_for_figurines(&present_ids)
            .await?;
        let details = self
            .repo
            .get_detail_images_for_figurines(&present_ids)
            .await?;
        let favorites = self.house_favorite_ids(&present_ids).await?;
        let mut items: Vec<FigurineListItemDto> = present_ids
            .into_iter()
            .filter_map(|id| by_id.remove(&id))
            .map(|f| {
                let face = faces.get(&f.id);
                let detail = details.get(&f.id);
                let favorite = favorites.contains(&f.id);
                self.to_list_item(f, face, detail, favorite)
            })
            .collect();
        self.add_list_translations(&mut items).await?;
        Ok(items)
    }

    pub async fn create_booking(
        &self,
        mut req: CreateBookingRequest,
        user_id: Option<Uuid>,
    ) -> Result<Booking> {
        if !req.age_confirmed {
            return Err(crate::error::AppError::BadRequest(
                "Please confirm you are 16 or older".to_string(),
            ));
        }

        // The booking form on the detail page carries whatever handle is in the URL
        // (slug or UUID). Normalise to the canonical UUID before create_booking_atomic,
        // which binds figurine_id straight into SQL and cannot resolve a slug itself.
        req.figurine_id = self
            .resolve_public_figurine_uuid(&req.figurine_id)
            .await?
            .to_string();

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
        self.note_lang("figurine_bookings", booking.id, user_id, req.lang.as_deref()).await;
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
        let unfeatured =
            sqlx::query("UPDATE figurines SET is_featured = false WHERE is_featured = true")
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
        let lang = self.lang_for("figurine_bookings", booking.id).await;
        let name = self.figurine_name_in(booking.figurine_id, &booking.figurine_name, lang).await;
        let Some((subject, base)) =
            booking_status_words(lang, status, &name, &booking.starts_at.to_string(), &booking.ends_at.to_string())
        else {
            return;
        };
        let (extra_label, extra) = match status {
            BookingStatus::Confirmed => (
                lang.pick(
                    "The curator's conditions",
                    "Условия куратора",
                    "Умови куратора",
                    "Condiciones del comisario",
                    "Bedingungen der Kuratorin",
                    "Condizioni della curatrice",
                ),
                curator_conditions,
            ),
            BookingStatus::Rejected => (
                lang.pick("Note", "Примечание", "Примітка", "Nota", "Anmerkung", "Nota"),
                admin_notes,
            ),
            _ => ("", None),
        };
        let body = match extra {
            Some(x) if !x.is_empty() => format!("{base}\n\n{extra_label}: {x}"),
            _ => base,
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
///
/// Шестнадцать, а не восемь. Знаков в пароле четыре, и весь пароль — это
/// `POOL_PER_CATEGORY⁴`: восемь давали 4096 сочетаний, то есть двенадцать бит.
/// Пять попыток в четверть часа — это 480 попыток в сутки, и полный перебор
/// одного имени занимал неделю; по утёкшей базе он занимал минуты, потому что
/// рядом с хешем лежит и набор, из которого выбирали. Шестнадцать дают 65 536,
/// то есть в шестнадцать раз дольше и то и другое, а набор при этом остаётся
/// подмножеством мастер-набора из двадцати четырёх — декой-набор неизвестного
/// адреса по-прежнему от настоящего не отличить.
///
/// Дальше этого размер набора не растёт: шестнадцать значков — предел того,
/// что человек окидывает взглядом и по чему находит свой. Оставшееся расстояние
/// закрывает не размер набора, а перец (`hash_password`): пространство в 65 536
/// перебирается по дампу за часы при любой стоимости Argon2, а без перца не
/// перебирается вовсе.
const POOL_PER_CATEGORY: usize = 16;

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
/// Половина HMAC-SHA-256: ключ, дополненный до блока и сложенный с прокладкой
/// (`0x36` внутри, `0x5c` снаружи). Отдельного крейта ради двух строк не
/// заводится, а сама конструкция записана здесь целиком, чтобы её было видно.
fn hmac_block(secret: &str, pad: u8) -> [u8; 64] {
    use sha2::{Digest, Sha256};
    let mut key = [0u8; 64];
    let bytes = secret.as_bytes();
    if bytes.len() > 64 {
        let digest = Sha256::digest(bytes);
        key[..32].copy_from_slice(&digest);
    } else {
        key[..bytes.len()].copy_from_slice(bytes);
    }
    for b in key.iter_mut() {
        *b ^= pad;
    }
    key
}

/// Ключом здесь стоит перец знаков, а не `ADMIN_API_KEY`: ключ админа меняют,
/// и в день замены все декой-наборы поменялись бы вместе с ним — то есть
/// адрес, который вчера отвечал одной сеткой, сегодня ответил бы другой, и
/// этим отличался бы от настоящего имени, чья сетка не меняется никогда.
fn decoy_pool(secret: &str, email: &str) -> Vec<Vec<String>> {
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    use sha2::{Digest, Sha256};

    CATEGORIES
        .iter()
        .map(|category| {
            // HMAC, а не `SHA256(секрет‖…)`: у второй записи есть известное
            // слабое место (дополнение длины), и хотя здесь оно не достаёт до
            // вывода, ключевой хеш пишут одним способом, чтобы не выяснять
            // этого заново на каждой правке.
            let mut hasher = Sha256::new();
            hasher.update(hmac_block(secret, 0x5c));
            hasher.update({
                let mut inner = Sha256::new();
                inner.update(hmac_block(secret, 0x36));
                inner.update(email.as_bytes());
                inner.update(b"\x00");
                inner.update(category.as_bytes());
                inner.finalize()
            });
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

/// Собрать набор нынешнего размера вокруг уже сделанного выбора.
///
/// Нужно там, где набор у имени старого размера (восемь знаков вместо
/// шестнадцати), а выбор человека только что стал известен — то есть сразу
/// после удачного входа. Выбранный знак остаётся на месте, прежние соседи
/// остаются соседями, недостающее добирается из мастер-набора.
///
/// Растить набор молча можно именно потому, что он не секрет: секрет — это
/// какой из знаков выбран, а набор рисуется на экране всякому, кто назовёт
/// адрес.
fn grown_pool(previous: &[Vec<String>], selections: &[String; 4]) -> serde_json::Value {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let mut pool_obj = serde_json::Map::new();

    for (i, category) in CATEGORIES.iter().enumerate() {
        let master = valid_icon_ids(category).unwrap_or(&[]);
        let mut kept: Vec<String> = previous
            .get(i)
            .map(|ids| {
                ids.iter()
                    .filter(|id| master.contains(&id.as_str()))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if !kept.contains(&selections[i]) {
            kept.push(selections[i].clone());
        }
        kept.truncate(POOL_PER_CATEGORY);

        let mut spare: Vec<&str> = master
            .iter()
            .copied()
            .filter(|id| !kept.iter().any(|k| k == id))
            .collect();
        spare.shuffle(&mut rng);
        for id in spare {
            if kept.len() >= POOL_PER_CATEGORY {
                break;
            }
            kept.push(id.to_string());
        }
        kept.shuffle(&mut rng);
        pool_obj.insert(category.to_string(), serde_json::json!(kept));
    }
    serde_json::Value::Object(pool_obj)
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

/// Argon2id с перцем и с большей стоимостью, чем значение по умолчанию.
///
/// Перец (`AUTH_PEPPER`) лежит вне базы, и в этом весь смысл: знаков всего
/// четыре из шестнадцати, то есть 65 536 сочетаний, а столько перебирается по
/// дампу за часы при любой стоимости — пространство слишком мало, чтобы его
/// спасла цена одной попытки. Дамп без перца не перебирается вовсе.
///
/// Стоимость при этом всё равно поднята (64 МиБ, три прохода против 19 МиБ и
/// двух): перец защищает от чтения базы, но не от того, кто прочёл и базу, и
/// окружение, — а против него работает только цена.
fn argon(pepper: &str) -> std::result::Result<Argon2<'_>, argon2::password_hash::Error> {
    use argon2::{Algorithm, Params, Version};
    let params = Params::new(64 * 1024, 3, 1, None)?;
    Argon2::new_with_secret(pepper.as_bytes(), Algorithm::Argon2id, Version::V0x13, params)
        .map_err(|_| argon2::password_hash::Error::Crypto)
}

fn hash_password(
    input: &str,
    pepper: &str,
) -> std::result::Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon(pepper)?.hash_password(input.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Чем знаки сошлись: нынешним способом или каким-то из прежних.
///
/// Перец нельзя досыпать в уже лежащий хеш — пересобрать его можно только в тот
/// единственный миг, когда выбор человека известен в открытую, то есть сразу
/// после удачного входа. Поэтому проверка знает про все записи, какие бывают, и
/// говорит, какая сработала: `Legacy` — это поручение пересобрать.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignsMatch {
    No,
    Now,
    Legacy,
}

/// Лестница из трёх ступеней, и у каждой своя причина:
///
/// 1. нынешний перец — обычный случай;
/// 2. прежний перец (`AUTH_PEPPER_OLD`) — идёт смена перца. Без этой ступени
///    перец был бы несменяем вовсе: утечка означала бы «поменять нельзя, не
///    заперев всех»;
/// 3. вовсе без перца — записи, сделанные до того, как перец появился.
///
/// Вторая и третья отвечают `Legacy`, то есть «сошлось, но перепиши»: разницы
/// между ними для входа нет, а работа после них одна и та же.
fn verify_password(input: &str, hash: &str, pepper: &str, was: Option<&str>) -> SignsMatch {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return SignsMatch::No;
    };
    let opens = |pepper: &str| {
        argon(pepper).is_ok_and(|a| a.verify_password(input.as_bytes(), &parsed).is_ok())
    };
    if opens(pepper) {
        return SignsMatch::Now;
    }
    if was.is_some_and(opens) {
        return SignsMatch::Legacy;
    }
    // Стоимость записана в самом хеше, поэтому `Argon2::default()` разбирает и
    // прежнюю, более дешёвую.
    if Argon2::default()
        .verify_password(input.as_bytes(), &parsed)
        .is_ok()
    {
        return SignsMatch::Legacy;
    }
    SignsMatch::No
}

/// Ключ настройки: когда канал подключили. Нет его — следующий тик
/// отмечает всё, что уже на сайте, как не подлежащее объявлению.
const CHANNEL_SINCE_KEY: &str = "telegram_channel_since";
/// Когда канал начал объявлять игру (карты и этюды). Свой ключ, а не прежний:
/// канал подключён давно, и без второго ключа первый проход после выкладки
/// объявил бы всю полку карт разом.
const CHANNEL_PLAY_SINCE_KEY: &str = "telegram_channel_since_play";
/// Сколько вещь стоит на людях, прежде чем выйти в канал. Работу часто
/// показывают, а потом ещё минуту правят подпись и меняют фотографию; пост в
/// канале так не поправишь.
const CHANNEL_GRACE_SECS: i64 = 600;
/// Постов за тик. Дюжина работ, открытых разом, выходит в канал за полчаса, а
/// не стеной подряд — и не упирается в пределы Telegram на отправку в канал.
const CHANNEL_PER_TICK: usize = 2;
/// После стольких отказов вещь больше не пытаются объявить; причина лежит в
/// `last_error`.
const CHANNEL_MAX_ATTEMPTS: i32 = 5;

/// Сколько байка стоит на людях, прежде чем о ней уходят письма. Та же
/// выдержка, что у канала, и по той же причине: письмо не поправишь.
const TALE_LETTER_GRACE_SECS: i64 = 600;
/// Писем за проход. Книга дома невелика, и тридцать за пять минут не упрутся
/// ни в какой предел почтового ящика отправителя.
const TALE_LETTERS_PER_TICK: i64 = 30;
/// После стольких отказов письмо больше не отправляют; причина — в
/// `last_error`.
const TALE_LETTER_MAX_ATTEMPTS: i32 = 5;
/// Сколько письмо считается взятым на отправку. Проход отправляет тридцать
/// писем за минуту-другую; взятое и не ушедшее дольше этого — значит, процесс
/// упал посреди прохода, и письмо возвращается в очередь.
const TALE_LETTER_CLAIM_SECS: i64 = 600;
/// Кандидатов в голосовании «о ком следующая». Из одного не выбирают, а пять
/// лиц в ряд на телефоне уже не лица, а полоска.
const TALE_POLL_MIN: usize = 2;
const TALE_POLL_MAX: usize = 4;

/// Готовый пост: что напечатать, куда вести и что показать.
struct ChannelPost {
    title: String,
    lead: Option<String>,
    url: String,
    image: Option<String>,
    button: &'static str,
    /// Род вещи словами («Новая байка») и метка для поиска по каналу («байка»).
    label: &'static str,
    tag: &'static str,
}

/// Знаки слова для сверки: без 0/O, 1/I/L и прочих пар, которые глаз путает.
/// Слово сравнивают взглядом, и «кажется, то же самое» защитой не является.
const WORD_ALPHABET: &[u8] = b"ACDEFGHJKMNPQRTUVWXY34679";

/// Слово, которое человек сверяет на экране и в боте. Шесть знаков через
/// дефис: короче — подбирается за пять минут перебором записок, длиннее —
/// не сверяется взглядом.
fn login_word() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let pick = |rng: &mut rand::rngs::ThreadRng| {
        WORD_ALPHABET[rng.gen_range(0..WORD_ALPHABET.len())] as char
    };
    let left: String = (0..3).map(|_| pick(&mut rng)).collect();
    let right: String = (0..3).map(|_| pick(&mut rng)).collect();
    format!("{left}-{right}")
}

/// Секрет в ссылке. Его не сверяют глазами, поэтому длинный: по нему находят
/// строку, и подобрать его должно быть нельзя.
fn login_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.r#gen();
    hex::encode(bytes)
}

/// Чем человек смотрит — одним словом, для записки бота. Полная строка
/// `User-Agent` в переписке нечитаема, а «Chrome» отвечает на тот вопрос,
/// который человек себе задаёт.
fn browser_name(ua: &str) -> String {
    let lower = ua.to_ascii_lowercase();
    // Порядок важен: Edge и Opera называют себя Chrome, Chrome называет себя
    // Safari. Спрашиваем от частного к общему.
    for (needle, name) in [
        ("edg/", "Edge"),
        ("opr/", "Opera"),
        ("yabrowser", "Yandex"),
        ("firefox", "Firefox"),
        ("chrome", "Chrome"),
        ("safari", "Safari"),
    ] {
        if lower.contains(needle) {
            return name.to_string();
        }
    }
    String::new()
}

/// Чем дом может дозвониться до хозяина имени, доказанно.
struct Channels {
    email: Option<String>,
    telegram: Option<i64>,
}

impl Channels {
    fn any(&self) -> bool {
        self.email.is_some() || self.telegram.is_some()
    }
}

/// Соединение с почтовым сервером — одно место на все письма дома.
///
/// **Способ шифрования выбирает порт, и это не украшение.** У почтовых серверов
/// их два, и они несовместимы:
///
/// * **465** — шифрование с первого байта (SMTPS). Это `relay()`.
/// * **587** — сначала открытый разговор, потом команда `STARTTLS`. Это
///   `starttls_relay()`.
///
/// Прежде здесь стоял `relay()` с любым портом из настроек, а по умолчанию
/// порт был 587. То есть в настройках по умолчанию дом стучался шифрованием в
/// дверь, которая ждёт открытого приветствия, — и письмо не уходило никуда.
/// Увидеть это было можно только по строке в журнале, а с тех пор как письмо
/// стало частью входа, это означало бы «никто не может завести имя».
/// Почта, собранная один раз: транспорт и обратный адрес (`AppService::mailer`).
struct Mailer {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    from: lettre::message::Mailbox,
}

fn smtp_transport(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
) -> Result<lettre::AsyncSmtpTransport<lettre::Tokio1Executor>> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, Tokio1Executor};

    let builder = if port == 587 {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::relay(host)
    }
    .map_err(|e| AppError::Internal(format!("SMTP relay error: {e}")))?;

    Ok(builder
        .port(port)
        .credentials(Credentials::new(user.to_string(), pass.to_string()))
        .build())
}

/// Отпечаток выданного ключа: в базе лежит он, ключ уезжает человеку.
///
/// Один на все три ключа — сессию, сброс знаков и письмо-приглашение. Простой
/// SHA-256, без соли и без стоимости, и это правильно: ключ здесь не пароль, а
/// сто двадцать два случайных бита, перебирать которые нечем и незачем. Соль
/// же сделала бы поиск по отпечатку невозможным.
fn fingerprint(token: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Ключ, который уезжает в письмо или в браузер.
fn secret_key() -> String {
    Uuid::new_v4().to_string()
}

/// Сколько живёт письмо-приглашение. Двое суток: человек заводит имя вечером,
/// а до ящика добирается на следующий день.
const CONFIRM_HOURS: i64 = 48;

/// Сколько живёт ключ сброса. Два часа, а не двое суток: письмо о сбросе
/// читают сразу — его для того и просят, — а лежит эта ссылка в ящике, в
/// истории браузера и в журналах прокси, и каждый лишний час она там лежит
/// зря.
const RESET_HOURS: i64 = 2;

/// Одна проверка адреса на все три двери: заведение имени, названная почта и
/// сброс. Прежде их было столько же, сколько мест, и они расходились: одна
/// требовала `@` и ничего больше (то есть пропускала адрес из одного знака
/// `@`), другая ещё и обрезала пробелы. Разошедшиеся проверки значат, что
/// адрес, которым завели имя, может не совпасть с адресом, которым в него
/// входят.
fn normalize_email(raw: &str) -> Result<String> {
    let email = raw.trim().to_lowercase();
    let bad = || AppError::BadRequest("Invalid email".into());
    if email.len() < 6 || email.len() > 200 {
        return Err(bad());
    }
    let mut halves = email.split('@');
    let (local, domain) = (halves.next().unwrap_or(""), halves.next().unwrap_or(""));
    if halves.next().is_some() || local.is_empty() {
        return Err(bad());
    }
    let dotted = domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.');
    if !dotted || domain.len() < 4 || email.contains(char::is_whitespace) {
        return Err(bad());
    }
    Ok(email)
}

/// Имя, как его печатают. Потолок есть, потому что колонка `TEXT` его не
/// имеет, а поле формы — тем более.
fn normalize_display_name(raw: &str) -> Result<String> {
    let name: String = raw.trim().chars().take(80).collect();
    if name.is_empty() {
        return Err(AppError::BadRequest("Display name required".into()));
    }
    Ok(name)
}

// ============================================================
// AppService — AUTH METHODS
// ============================================================

impl AppService {
    /// Есть ли у дома чем отправить письмо.
    ///
    /// Спрашивается там же, откуда берёт настройки `send_mail`: у базы, а чего
    /// нет в базе — у окружения. По этому ответу дверь выбирает порядок, и
    /// поэтому он не может разойтись с тем, дойдёт письмо или нет.
    pub async fn mail_works(&self) -> bool {
        let db = self.get_smtp_settings().await.unwrap_or_default();
        let named = |from_db: Option<&str>, from_env: Option<&str>| {
            from_db
                .or(from_env)
                .is_some_and(|v| !v.trim().is_empty())
        };
        named(db.host.as_deref(), self.config.smtp_host.as_deref())
            && named(db.user.as_deref(), self.config.smtp_user.as_deref())
            && named(db.pass.as_deref(), self.config.smtp_pass.as_deref())
            && named(db.from.as_deref(), self.config.smtp_from.as_deref())
    }

    /// Завести имя. Ответ — «письмо ушло», и ничего кроме.
    ///
    /// Сессии здесь больше нет, и это не строгость, а единственный способ
    /// закрыть перечисление жильцов. Пока имя заводилось сразу, занятый адрес
    /// отвечал «такой уже есть», а свободный — сессией, и узнать, кто живёт в
    /// доме, можно было одним запросом на адрес — мимо всей работы,
    /// проделанной на входе ради того, чтобы этого нельзя было узнать
    /// (декой-наборы, общий `Unauthorized`, молчание «забытых знаков»).
    ///
    /// Теперь оба случая отвечают одинаково, а разницу узнаёт только тот, у
    /// кого ключ от ящика: на свободный адрес уходит приглашение, на занятый —
    /// записка «имя у вас уже есть, вот вход».
    ///
    /// Заодно это и есть подтверждение почты: дом проверяет, что до ящика
    /// можно дозвониться, ровно один раз — и именно тогда, когда это ещё
    /// ничего не стоит человеку.
    pub async fn register_user(
        &self,
        req: &RegisterRequest,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<crate::models::RegisterResponse> {
        if !req.age_confirmed {
            return Err(AppError::BadRequest(
                "Please confirm you are 16 or older".into(),
            ));
        }
        let email = normalize_email(&req.email)?;
        let display_name = normalize_display_name(&req.display_name)?;

        // Validate the personal pool (alphabet shown at login) and that each
        // selection is drawn from it.
        let pool_json = validate_pool(&req.pool, &req.selections)?;

        let hash_input = build_hash_input(&req.selections);
        let hash = hash_password(&hash_input, &self.config.auth_pepper)
            .map_err(|e| AppError::Internal(format!("Hash error: {e}")))?;

        let ctx = self.client_context(ip, user_agent);
        let key = secret_key();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(CONFIRM_HOURS);

        // Открытый порядок: отправить письмо нечем, значит подтверждать нечем,
        // и держать человека у двери ради письма, которое никогда не придёт, —
        // это просто закрытый дом. Отметка о письме всё равно заводится: появится
        // почта — приглашение уйдёт по «выслать заново», ничего не переделывая.
        //
        // Перезахват неоткрытого имени здесь НЕ делается, хотя неподтверждёнными
        // тут будут все: он безопасен ровно потому, что письмо доказывает ящик.
        // Без письма он означал бы «набери чужой адрес — и имя твоё».
        if !self.mail_works().await {
            let user = self
                .repo
                .create_user(
                    &email,
                    &display_name,
                    &hash,
                    &pool_json,
                    &fingerprint(&key),
                    expires_at,
                    &ctx,
                )
                .await?;
            let session_token = self.open_session(user.id, &ctx).await?;
            Self::log_domain_event("user_registered", "user", user.id, "open");
            return Ok(crate::models::RegisterResponse {
                pending: false,
                email,
                session_token: Some(session_token),
                user: Some(UserDto::from(&user)),
            });
        }

        let made = self
            .repo
            .create_user(
                &email,
                &display_name,
                &hash,
                &pool_json,
                &fingerprint(&key),
                expires_at,
                &ctx,
            )
            .await;

        let invited = match made {
            Ok(user) => {
                Self::log_domain_event("user_registered", "user", user.id, "pending");
                true
            }
            // Адрес занят. Имя, до которого так и не дозвонились, принадлежит
            // тому, кто докажет ящик, — иначе занять чужой адрес можно было бы,
            // просто набрав его, и настоящий хозяин упёрся бы в уникальный
            // индекс навсегда. Захвата тут нет: письмо в обоих случаях уходит
            // владельцу ящика.
            Err(AppError::Conflict(_)) => {
                let reclaimed = self
                    .repo
                    .reclaim_unconfirmed_user(
                        &email,
                        &display_name,
                        &hash,
                        &pool_json,
                        &fingerprint(&key),
                        expires_at,
                        &ctx,
                    )
                    .await?;
                match reclaimed {
                    Some(user) => {
                        Self::log_domain_event("user_reclaimed", "user", user.id, "pending");
                        true
                    }
                    None => false,
                }
            }
            Err(e) => return Err(e),
        };

        // Письмо уходит вдогонку: медленный SMTP не должен держать ответ, а по
        // времени ответа не должно быть видно, какая из двух веток сработала.
        let svc = self.clone();
        let to = email.clone();
        tokio::spawn(async move {
            let sent = if invited {
                svc.send_confirm_email(&to, &key).await
            } else {
                svc.send_already_have_name_email(&to).await
            };
            if let Err(e) = sent {
                tracing::error!("Sign-up letter to {to} failed: {e}");
            }
        });

        Ok(crate::models::RegisterResponse {
            pending: true,
            email,
            session_token: None,
            user: None,
        })
    }

    /// Перешли по ссылке из письма. Дверь открыта, и человек сразу внутри:
    /// заставлять его после этого набирать знаки заново значило бы спросить
    /// дважды одно и то же.
    pub async fn confirm_email(
        &self,
        token: &str,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<LoginVerifyResponse> {
        let user = self
            .repo
            .confirm_email(&fingerprint(token))
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("This letter has gone cold — ask for a new one.".into())
            })?;
        if user.is_blocked {
            return Err(AppError::Unauthorized);
        }

        let ctx = self.client_context(ip, user_agent);
        let session_token = self.open_session(user.id, &ctx).await?;
        Self::log_domain_event("email_confirmed", "user", user.id, "ok");
        Ok(LoginVerifyResponse {
            session_token,
            user: UserDto::from(&user),
        })
    }

    /// Выслать приглашение заново — тому, кто не дождался первого, и тому, кто
    /// назвал адрес в ответ на дело. Подтверждённому имени отвечает молчанием:
    /// просить подтвердить дважды не о чем.
    pub async fn resend_confirm_email(&self, user: &User) -> Result<()> {
        let Some(to) = user.email.clone() else {
            return Err(AppError::BadRequest("No email on this account".into()));
        };
        if user.email_confirmed_at.is_some() {
            return Ok(());
        }
        let key = secret_key();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(CONFIRM_HOURS);
        self.repo
            .set_email_confirm(user.id, &fingerprint(&key), expires_at)
            .await?;
        let svc = self.clone();
        tokio::spawn(async move {
            if let Err(e) = svc.send_confirm_email(&to, &key).await {
                tracing::error!("Confirmation letter to {to} failed: {e}");
            }
        });
        Ok(())
    }

    /// Завести сессию: человеку — ключ, базе — отпечаток.
    ///
    /// Одно место на все четыре двери (знаки, письмо, Telegram, сброс): пока
    /// ключ клали в базу как есть, чтение базы было входом в каждое имя на
    /// месяц вперёд, и четыре записи этого «клали» разошлись бы на первой же
    /// правке.
    async fn open_session(
        &self,
        user_id: Uuid,
        ctx: &crate::models::ClientContext,
    ) -> Result<String> {
        let token = secret_key();
        let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
        self.repo
            .create_session(user_id, &fingerprint(&token), expires_at, ctx)
            .await?;
        self.repo.prune_expired_sessions(user_id).await.ok();
        Ok(token)
    }

    /// Сколько неудач терпит дом.
    ///
    /// `PAIR` — тесный счёт по паре «этот адрес из этой сети». Он и ловит
    /// подбирающего, и НЕ запирает хозяина: пять ошибок постороннего гасят
    /// посторонего, а не человека, который сейчас войдёт из дома.
    ///
    /// `WIDE_EMAIL` — потолок на само имя, широкий: он не про опечатку, а про
    /// того, кто перебирает с десяти адресов сразу, и без него тесный счёт
    /// обходится сменой сети. Тридцать в час — это девяносто дней на полный
    /// перебор нынешних 65 536 сочетаний.
    ///
    /// `WIDE_IP` — чтобы один адрес не веерил по многим именам.
    const LOCK_PAIR: i64 = 5;
    const LOCK_PAIR_MINUTES: i64 = 15;
    const LOCK_WIDE: i64 = 30;
    const LOCK_WIDE_MINUTES: i64 = 60;

    async fn check_lockout(&self, email: &str, ip: Option<&str>) -> Result<()> {
        let tally = self
            .repo
            .recent_failures(email, ip, Self::LOCK_PAIR_MINUTES, Self::LOCK_WIDE_MINUTES)
            .await?;
        // Все три преграды отвечают одним и тем же: который из счётов
        // сработал, знать снаружи незачем, а разные ответы сообщали бы
        // перебирающему, в какую сторону двигаться.
        if tally.pair >= Self::LOCK_PAIR
            || tally.by_email >= Self::LOCK_WIDE
            || tally.by_ip >= Self::LOCK_WIDE
        {
            return Err(AppError::BadRequest(
                "Too many failed attempts. Try again later.".into(),
            ));
        }
        Ok(())
    }

    pub async fn login_challenge(
        &self,
        email: &str,
        ip: Option<&str>,
    ) -> Result<LoginChallengeResponse> {
        use rand::seq::SliceRandom;
        let email_lower = normalize_email(email)?;

        // A challenge is always issued — for unknown, valid, AND blocked emails —
        // so the response never reveals whether an account exists. The actual
        // user/password/blocked checks happen in login_verify, which returns the
        // same generic Unauthorized for every failure. (Avoids account enumeration.)

        // Check lockout before issuing challenge
        self.check_lockout(&email_lower, ip).await?;

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
            .unwrap_or_else(|| decoy_pool(&self.config.auth_pepper, &email_lower));

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

        self.check_lockout(&email, ctx.ip.as_deref()).await?;

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

        // Аккаунт без визуального пароля (вошедший через Telegram) этой дверью
        // не входит — и отвечает ему тот же общий Unauthorized, что и неизвестной
        // почте: по ответу нельзя узнать, есть ли такой аккаунт и какие у него
        // двери.
        let Some(stored_hash) = user.visual_password_hash.as_deref() else {
            self.repo.record_attempt(&email, false, &ctx).await?;
            return Err(AppError::Unauthorized);
        };
        let hash_input = build_hash_input(&resolved_selections);
        let matched = verify_password(
            &hash_input,
            stored_hash,
            &self.config.auth_pepper,
            self.config.auth_pepper_old.as_deref(),
        );
        if matched == SignsMatch::No {
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

        // Единственный миг, когда выбор человека известен в открытую. Здесь и
        // только здесь его можно переложить на нынешний перец и нынешний
        // размер набора — не спросив ни о чём и ничего не сломав. Человек
        // входит теми же знаками; меняется то, как они записаны.
        let pool = user.visual_pool.as_ref().and_then(parse_stored_pool);
        let old_size = pool
            .as_ref()
            .map(|p| p.iter().any(|ids| ids.len() != POOL_PER_CATEGORY))
            .unwrap_or(true);
        if matched == SignsMatch::Legacy || old_size {
            self.refresh_signs(&user, &resolved_selections, pool.as_deref().unwrap_or(&[]))
                .await;
        }

        let session_token = self.open_session(user.id, &ctx).await?;

        Self::log_domain_event("user_login", "user", user.id, "ok");
        self.tell_of_new_door(&user, &ctx);
        Ok(LoginVerifyResponse {
            session_token,
            user: UserDto::from(&user),
        })
    }

    /// Переписать знаки на нынешний лад. Неудача здесь ничего не отменяет:
    /// человек уже вошёл, а запись подтянется при следующем входе.
    async fn refresh_signs(&self, user: &User, selections: &[String; 4], previous: &[Vec<String>]) {
        let Ok(hash) = hash_password(&build_hash_input(selections), &self.config.auth_pepper) else {
            return;
        };
        let pool = grown_pool(previous, selections);
        if let Err(e) = self
            .repo
            .refresh_visual_credentials(user.id, &hash, &pool)
            .await
        {
            tracing::warn!("Could not refresh signs for {}: {e}", user.id);
        }
    }

    // ========================================================
    // ВХОД ЧЕРЕЗ TELEGRAM: одноразовое слово
    // ========================================================

    /// Есть ли эта дверь на этом сервере. Клиент спрашивает, чтобы не рисовать
    /// кнопку, ведущую в никуда, а имя бота приходит отсюда, чтобы сменить
    /// бота можно было без пересборки фронта.
    pub fn telegram_login_config(&self) -> crate::models::TelegramLoginConfig {
        crate::models::TelegramLoginConfig {
            enabled: self.config.telegram_login_bot_token.is_some(),
            bot_username: self.config.telegram_login_bot_username.clone(),
            local: self.config.public_url_is_local(),
        }
    }

    /// Назвать почту — тогда, когда её впервые спросило дело.
    ///
    /// Вход через Telegram почты не даёт, и спрашивать её у двери значит терять
    /// человека там, ради чего вход и облегчали. Спрашивает бронь, заказ или
    /// заявка — то есть ровно там, где без обратного адреса дом не сможет
    /// ответить, — и спрашивает ОДИН раз: дальше она лежит в имени.
    ///
    /// Замена почты сюда не входит. Она меняет личность, и делаться молча, по
    /// дороге к заказу, не должна.
    pub async fn name_email(&self, user: &User, email: &str) -> Result<UserDto> {
        let email = normalize_email(email)?;
        if let Some(existing) = user.email.as_deref() {
            // Та же самая — спрашивать было не о чем, и отказывать не за что.
            // Приглашение при этом уходит заново: человек, назвавший адрес
            // второй раз, обычно ищет именно то письмо.
            if existing == email {
                if user.email_confirmed_at.is_none() {
                    self.resend_confirm_email(user).await?;
                }
                return Ok(UserDto::from(user));
            }
            return Err(AppError::BadRequest("This account already has an email".into()));
        }
        let key = secret_key();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(CONFIRM_HOURS);
        let fresh = self
            .repo
            .set_user_email(user.id, &email, &fingerprint(&key), expires_at)
            .await?
            .ok_or_else(|| AppError::BadRequest("This account already has an email".into()))?;
        Self::log_domain_event("user_named_email", "user", fresh.id, "ok");

        // Названный адрес — ещё не дверь. Дом им ОТВЕТИТ на заказ (для этого
        // его и спросили), но ВПУСТИТ по нему только после письма: иначе
        // опечатка в адресе отдавала бы имя постороннему, а «забыли знаки»
        // делает почту каналом полного восстановления.
        let to = email.clone();
        let svc = self.clone();
        tokio::spawn(async move {
            if let Err(e) = svc.send_confirm_email(&to, &key).await {
                tracing::error!("Confirmation letter to {to} failed: {e}");
            }
        });
        Ok(UserDto::from(&fresh))
    }

    /// Отвязать Telegram.
    ///
    /// Отказ, когда другой двери нет: первая дверь — это почта И знаки вместе
    /// (вход по знакам начинается с почты), и без любой из половин человек,
    /// отвязавший Telegram, запер бы себя снаружи собственной дверью. Проверка
    /// и снятие — одна операция (`unlink_door`), а не «прочитал, потом записал».
    pub async fn telegram_unlink(&self, user: &User) -> Result<UserDto> {
        if user.telegram_id.is_none() {
            return Ok(UserDto::from(user));
        }
        // Дверью считается ПОДТВЕРЖДЁННАЯ почта со знаками (по названному, но
        // не открытому адресу войти нельзя), привязанный Google или привязанный
        // Apple. Список один на все отвязки (`User::doors_besides`), а подсчёт и
        // снятие идут под одной блокировкой строки: две отвязки разом иначе
        // видели бы друг у друга вторую дверь и обе проходили.
        if !self.repo.unlink_door(user.id, Door::Telegram).await? {
            return Err(AppError::BadRequest(
                "Telegram is the only way in — add another way in first".into(),
            ));
        }
        Self::log_domain_event("telegram_unlinked", "user", user.id, "ok");
        let fresh = self
            .repo
            .find_user_by_id(user.id)
            .await?
            .ok_or(AppError::Unauthorized)?;
        Ok(UserDto::from(&fresh))
    }

    /// Суточная уборка: все просроченные сессии.
    pub async fn prune_expired_sessions(&self) -> Result<u64> {
        self.repo.prune_all_expired_sessions().await
    }

    /// Уборка за входом: отработавшие и остывшие слова.
    pub async fn prune_telegram_login_codes(&self) -> Result<u64> {
        self.repo.prune_telegram_login_codes().await
    }

    /// Адрес, по которому этот дом принимает обновления Telegram.
    ///
    /// Считается в одном месте: тот же адрес называют и саморегистрация при
    /// запуске, и подпись в панели выкладки. Два способа собрать один адрес
    /// однажды разошлись бы на косой черте.
    pub fn telegram_webhook_url(&self) -> Option<String> {
        let secret = self.config.telegram_webhook_secret.as_deref()?;
        Some(crate::telegram::webhook_url(&self.config.public_url, secret))
    }

    /// Сказать Telegram, куда стучаться. Вызывается при каждом запуске.
    ///
    /// Руками это делалось один раз и забывалось навсегда — ровно до переезда
    /// на другой домен, после которого вход молча переставал работать, а
    /// ошибка лежала не у нас, а в `getWebhookInfo`, куда никто не смотрит.
    /// Сервер знает и домен, и токен, и секрет, поэтому говорит сам.
    ///
    /// Ошибка не роняет запуск: не дозвонились до Telegram — сайт всё равно
    /// должен открыться, а вход подождёт следующего запуска.
    pub async fn announce_telegram_webhook(&self) {
        let (Some(bot), Some(url)) = (self.login_bot(), self.telegram_webhook_url()) else {
            return; // двери нет — и объявлять нечего
        };
        let secret = match self.config.telegram_webhook_secret.as_deref() {
            Some(s) => s,
            None => return,
        };
        // В журнал идёт адрес БЕЗ секрета: секрет стоит в самом пути, а журнал
        // читают и в терминале, и в админском просмотрщике.
        let said = crate::telegram::webhook_url_for_the_journal(&self.config.public_url);
        if self.config.public_url_is_local() {
            tracing::info!(
                "Telegram login is on, but PUBLIC_URL is local ({}): Telegram cannot reach {}. \
                 The webhook is NOT registered — the door is played here instead: each word \
                 issued prints its own link (see telegram_play_locally).",
                self.config.public_url,
                said
            );
            return;
        }
        match bot.announce(&url, secret).await {
            Ok(()) => tracing::info!("Telegram webhook registered at {said}"),
            // Уровень warn, а не info: вход не работает, и это должно быть
            // видно в журнале запуска, а не найдено потом по жалобе.
            Err(e) => tracing::warn!("Telegram webhook NOT registered ({said}): {e}"),
        }
    }

    // ============================================================
    // TELEGRAM-КАНАЛ
    // ============================================================

    /// Объявить в канале то, что появилось на сайте. Зовётся фоном раз в
    /// несколько минут; возвращает число вышедших постов.
    ///
    /// Событий «опубликовано» здесь не ловят вовсе: запланированный лист
    /// выходит сам, когда наступило его время, а работа — когда с неё сняли
    /// «в работе» или истёк первый взгляд. Поэтому канал сверяет то, что на
    /// людях **сейчас**, с журналом `telegram_channel_posts` — и это
    /// единственный способ не пропустить ни один из путей на сайт.
    pub async fn announce_to_channel(&self) -> Result<usize> {
        let Some((bot, chat)) = self.channel_bot() else {
            return Ok(0); // канала нет — объявлять некуда
        };
        let posts = self.channel_on_display().await?;
        let ids = |kind: &str| -> Vec<Uuid> {
            posts.keys().filter(|(k, _)| k == kind).map(|(_, id)| *id).collect()
        };

        // Первый тик после подключения канала: всё, что уже стоит на сайте,
        // отмечается как пропущенное. Иначе канал начался бы с архива целиком.
        if self.repo.get_setting(CHANNEL_SINCE_KEY).await?.is_none() {
            let works = self.repo.channel_note_seen("work", &ids("work"), true).await?;
            let leaves = self.repo.channel_note_seen("leaf", &ids("leaf"), true).await?;
            self.repo
                .upsert_setting(CHANNEL_SINCE_KEY, &Utc::now().to_rfc3339())
                .await?;
            tracing::info!(
                "Telegram channel {chat} connected: {works} works and {leaves} leaves already on \
                 the site are left unannounced; everything that appears from now on goes out"
            );
            return Ok(0);
        }

        // Игра пришла в канал позже работ и листов: всё, что уже стоит на полке
        // и на карте этюдов к первому проходу, отмечается пропущенным.
        if self.repo.get_setting(CHANNEL_PLAY_SINCE_KEY).await?.is_none() {
            let cards = self.repo.channel_note_seen("card", &ids("card"), true).await?;
            let etudes = self.repo.channel_note_seen("etude", &ids("etude"), true).await?;
            self.repo
                .upsert_setting(CHANNEL_PLAY_SINCE_KEY, &Utc::now().to_rfc3339())
                .await?;
            tracing::info!(
                "Telegram channel {chat}: {cards} cards and {etudes} etudes already on the site \
                 are left unannounced; new ones go out from now on"
            );
        }

        for kind in ["work", "leaf", "card", "etude"] {
            self.repo.channel_note_seen(kind, &ids(kind), false).await?;
        }

        // Ждущее, но снова снятое с сайта, не выходит и не занимает очередь:
        // вернут на сайт — выйдет тогда.
        let due = self
            .repo
            .channel_due(CHANNEL_GRACE_SECS, CHANNEL_MAX_ATTEMPTS, 200)
            .await?;
        let mut sent = 0;
        for (kind, id) in due {
            if sent >= CHANNEL_PER_TICK {
                break;
            }
            let Some(post) = posts.get(&(kind.clone(), id)) else {
                continue;
            };
            let caption = crate::telegram::announcement_caption(
                post.label,
                post.tag,
                &post.title,
                post.lead.as_deref(),
            );

            // Локально пост не уходит: база здесь своя, а канал настоящий, и
            // стенд объявил бы на людях то, что завёл для пробы. Закон тот же,
            // что у письма (`say_or_send`): вместо действия — строка в журнале.
            if self.config.public_url_is_local() {
                tracing::info!(
                    "Telegram channel (local, not sent): {} → {}\n{caption}",
                    post.url,
                    post.image.as_deref().unwrap_or("no photo")
                );
                self.repo.channel_mark_posted(&kind, id, None).await?;
                sent += 1;
                continue;
            }

            // Фотографию Telegram забирает по адресу сам и не всякую берёт
            // (формат, размер). Не взял — выходит текст со ссылкой: пост без
            // фотографии лучше, чем никакого.
            let photo = match post.image.as_deref() {
                Some(img) => bot.post_photo(&chat, img, &caption, post.button, &post.url).await,
                None => Err("no photo".to_string()),
            };
            let result = match photo {
                Ok(message_id) => Ok(message_id),
                Err(photo_why) => bot
                    .post_text(&chat, &caption, post.button, &post.url)
                    .await
                    .map_err(|text_why| format!("photo: {photo_why}; text: {text_why}")),
            };
            match result {
                Ok(message_id) => {
                    self.repo.channel_mark_posted(&kind, id, Some(message_id)).await?;
                    self.note_on_chart(&format!("Channel post: {}", post.title)).await;
                    sent += 1;
                }
                Err(why) => {
                    tracing::warn!("Telegram channel post for {kind} {id} failed: {why}");
                    self.repo.channel_mark_failed(&kind, id, &why).await?;
                }
            }
        }
        Ok(sent)
    }

    fn channel_bot(&self) -> Option<(crate::telegram::Bot, String)> {
        let token = self.config.telegram_bot_token.clone().filter(|t| !t.trim().is_empty())?;
        let chat = self.config.telegram_channel_id.clone()?;
        Some((crate::telegram::Bot::new(token, self.http_client.clone()), chat))
    }

    /// Что сейчас на людях и годится в канал, с готовым текстом поста.
    ///
    /// Работа — та же выборка, что у ленты для Pinterest: видимая, не «в
    /// работе» и не под первым взглядом. Лист — всякий живой, кроме двух
    /// родов: `world` пересказывает чужие новости, а `arrival` о работе
    /// объявил бы её второй раз — работа выходит в канал сама.
    async fn channel_on_display(&self) -> Result<HashMap<(String, Uuid), ChannelPost>> {
        let base = self.config.public_url.trim_end_matches('/').to_string();
        let absolute = |url: &str| -> String {
            if url.starts_with("http://") || url.starts_with("https://") {
                url.to_string()
            } else {
                format!("{base}/{}", url.trim_start_matches('/'))
            }
        };
        // `src` остаётся в замере посещений (§ 18): по нему видно, сколько
        // людей привёл канал.
        let link = |path: &str| format!("{base}{path}?src=telegram");
        // Адрес, у которого уже есть строка запроса (`?card=`, `?play=`).
        let link_with = |path: &str| format!("{base}{path}&src=telegram");

        let now = Utc::now();
        let works = self
            .list_figurines(true, FigurineQuery::default())
            .await?
            .items;
        let mut posts = HashMap::new();
        let mut faces: HashMap<Uuid, String> = HashMap::new();
        for f in works {
            if f.status == FigurineStatus::InProgress || f.first_look_until.is_some_and(|t| t > now) {
                continue;
            }
            let Ok(id) = Uuid::parse_str(&f.id) else { continue };
            let image = f
                .face_image_large_url
                .as_deref()
                .or(f.face_image_url.as_deref())
                .filter(|u| !u.trim().is_empty())
                .map(absolute);
            if let Some(img) = &image {
                faces.insert(id, img.clone());
            }
            let handle = f.slug.clone().unwrap_or_else(|| f.id.clone());
            posts.insert(
                ("work".to_string(), id),
                ChannelPost {
                    title: f.name.clone(),
                    lead: f.short_text.clone(),
                    url: link(&format!("/figurines/{handle}")),
                    image,
                    button: "Смотреть на сайте",
                    label: "Новая работа",
                    tag: "работа",
                },
            );
        }

        let leaves = self.list_gazette_public(1, 500).await?.items;
        for leaf in leaves {
            if leaf.kind == "world" || (leaf.kind == "arrival" && leaf.figurine_id.is_some()) {
                continue;
            }
            let Ok(id) = Uuid::parse_str(&leaf.id) else { continue };
            let pick = |ru: &Option<String>, en: &Option<String>| -> Option<String> {
                [ru, en]
                    .into_iter()
                    .flatten()
                    .map(|s| s.trim())
                    .find(|s| !s.is_empty())
                    .map(str::to_string)
            };
            let title = pick(&Some(leaf.title_ru.clone()), &Some(leaf.title_en.clone()))
                .unwrap_or_default();
            if title.is_empty() {
                continue;
            }
            let lead = pick(&leaf.dek_ru, &leaf.dek_en).or_else(|| {
                pick(&leaf.body_ru, &leaf.body_en)
                    .map(|body| crate::gazette::excerpt(&body, crate::gazette::EXCERPT_MAX))
            });
            let image = leaf
                .image_url
                .as_deref()
                .or(leaf.image_urls.first().map(String::as_str))
                .filter(|u| !u.trim().is_empty())
                .map(absolute)
                .or_else(|| {
                    let work = Uuid::parse_str(leaf.figurine_id.as_deref()?).ok()?;
                    faces.get(&work).cloned()
                });
            let button = match leaf.kind.as_str() {
                "tale" | "guest_story" => "Читать",
                _ => "Открыть на сайте",
            };
            // Род называет то, что человек увидит, открыв пост: байку читают,
            // эскиз смотрят. Прочие листы — просто листы вестника.
            let (label, tag) = match leaf.kind.as_str() {
                "tale" => ("Новая байка", "байка"),
                "guest_story" => ("Гостевая история", "история"),
                "sketch" => ("Эскиз из мастерской", "эскиз"),
                "showing" => ("Показ", "показ"),
                "collage" => ("Коллаж", "коллаж"),
                "arrival" => ("Новое поступление", "поступление"),
                _ => ("Лист вестника", "вестник"),
            };
            posts.insert(
                ("leaf".to_string(), id),
                ChannelPost {
                    title,
                    lead,
                    url: link(&crate::gazette::leaf_path(&leaf.kind, &leaf.slug)),
                    image,
                    button,
                    label,
                    tag,
                },
            );
        }

        // Карты полки. Строения (стены, башни) — снаряжение этюдов, не карта,
        // и в канал не идут.
        let cards = self.list_battle_cards_public().await?;
        let mut card_art: HashMap<String, String> = HashMap::new();
        for card in cards {
            if card.kind == "structure" {
                continue;
            }
            let Ok(id) = Uuid::parse_str(&card.id) else { continue };
            let title = [&card.title_ru, &card.title_en]
                .into_iter()
                .map(|s| s.trim())
                .find(|s| !s.is_empty())
                .unwrap_or_default()
                .to_string();
            if title.is_empty() {
                continue;
            }
            let image = card
                .art_url
                .as_deref()
                .filter(|u| !u.trim().is_empty())
                .map(absolute)
                .or_else(|| {
                    let work = Uuid::parse_str(card.figurine_id.as_deref()?).ok()?;
                    faces.get(&work).cloned()
                });
            if let Some(img) = &image {
                card_art.insert(card.slug.clone(), img.clone());
            }
            let lead = [&card.lore_ru, &card.effect_ru, &card.lore_en, &card.effect_en]
                .into_iter()
                .flatten()
                .map(|s| s.trim())
                .find(|s| !s.is_empty())
                .map(str::to_string);
            posts.insert(
                ("card".to_string(), id),
                ChannelPost {
                    title,
                    lead,
                    url: link_with(&format!("/battles?card={}", card.id)),
                    image,
                    button: "Открыть карту",
                    label: "Новая карта в игре",
                    tag: "карта",
                },
            );
        }

        // Этюды. Картинка — фон поля, а без него — первая карта противника.
        for etude in self.repo.list_battle_challenges(true).await? {
            let title = [&etude.title_ru, &etude.title_en]
                .into_iter()
                .map(|s| s.trim())
                .find(|s| !s.is_empty())
                .unwrap_or_default()
                .to_string();
            if title.is_empty() {
                continue;
            }
            let setup: serde_json::Value =
                serde_json::from_str(&etude.setup).unwrap_or(serde_json::Value::Null);
            let image = setup
                .get("background")
                .and_then(|v| v.as_str())
                .filter(|u| !u.trim().is_empty())
                .map(absolute)
                .or_else(|| {
                    let board = setup.get("keeperBoard")?.as_array()?;
                    board
                        .iter()
                        .filter_map(|p| p.get("card")?.as_str())
                        .find_map(|slug| card_art.get(slug).cloned())
                });
            let lead = [&etude.note_ru, &etude.note_en]
                .into_iter()
                .flatten()
                .map(|s| s.trim())
                .find(|s| !s.is_empty())
                .map(str::to_string);
            let (label, tag) = if etude.player_side == "deck" {
                ("Новая встреча в игре", "этюд")
            } else {
                ("Новый этюд в игре", "этюд")
            };
            posts.insert(
                ("etude".to_string(), etude.id),
                ChannelPost {
                    title,
                    lead,
                    url: link_with(&format!("/battles/etude?play={}", etude.id)),
                    image,
                    button: "Сыграть",
                    label,
                    tag,
                },
            );
        }
        Ok(posts)
    }

    /// Секрет webhook — читает только обработчик, сверяя два доказательства.
    pub fn telegram_webhook_secret(&self) -> Option<String> {
        self.config.telegram_webhook_secret.clone()
    }

    fn login_bot(&self) -> Option<crate::telegram::Bot> {
        let token = self.config.telegram_login_bot_token.clone()?;
        Some(crate::telegram::Bot::new(token, self.http_client.clone()))
    }

    /// Начать вход: завести слово и отдать странице ссылку на бота.
    ///
    /// `bind_user` — привязка Telegram к уже вошедшему аккаунту из профиля:
    /// тот же обряд, но в конце не новый человек, а вторая дверь у прежнего.
    pub async fn telegram_start_login(
        &self,
        bind_user: Option<Uuid>,
        age_confirmed: bool,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<crate::models::TelegramCodeResponse> {
        let bot_name = match (
            self.config.telegram_login_bot_token.as_deref(),
            self.config.telegram_login_bot_username.as_deref(),
        ) {
            (Some(_), Some(name)) => name.to_string(),
            _ => return Err(AppError::BadRequest("Telegram login is not set up".into())),
        };

        let code = login_code();
        let word = login_word();
        let ctx = self.client_context(ip, user_agent.clone());
        let row = self
            .repo
            .create_telegram_login_code(&code, &word, age_confirmed, bind_user, user_agent.as_deref(), &ctx)
            .await?;

        // На своей машине бот до нас не достучится — webhook принадлежит
        // рабочему серверу. Чтобы не гадать, почему бот отвечает «слово
        // остыло», дом сразу называет ссылку, которой обряд доигрывается
        // здесь же. То же самое он делает с ссылкой из письма.
        if self.config.public_url_is_local() {
            let site = self.config.public_url.trim_end_matches('/');
            tracing::info!(
                "[local] Telegram word {word}: the bot cannot reach this host, so play it here — {site}/api/v1/auth/telegram/local/{code}  (add ?answer=no for the refusal, ?id=<number> for a second person)"
            );
        }

        Ok(crate::models::TelegramCodeResponse {
            link: format!("https://t.me/{bot_name}?start={code}"),
            code,
            word,
            expires_at: row.expires_at.to_rfc3339(),
        })
    }

    /// Сыграть бота за Telegram — только на своей машине.
    ///
    /// Бот входа один, и webhook у него один: он принадлежит тому серверу,
    /// который зарегистрировал его последним, то есть рабочему. Локальный
    /// сервер его не регистрирует вовсе — до localhost Telegram не дотянется
    /// (`announce_telegram_webhook`). Поэтому слово, взятое здесь, лежит в
    /// здешней базе, а `/start <слово>` уходит в рабочий сервер, который
    /// такого слова не знает и честно отвечает «Слово остыло». Снаружи это
    /// неотличимо от настоящего остывшего слова — и не должно быть отличимо
    /// (§ 16.2), поэтому разработчику ответ бота не сообщает ничего.
    ///
    /// Закон тот же, что у письма (`say_or_send`): локально дом делает сам то,
    /// чего снаружи не дождаться, и называет это в журнале.
    ///
    /// Играется обряд **тем же путём**, каким приходит Telegram —
    /// `telegram_update`, а не в обход: обряд, проверяемый другим кодом,
    /// проверяет не тот обряд. Отсюда же и `who`: число, по которому узнают
    /// вернувшегося, здесь выдумано, но постоянно, поэтому второй заход
    /// приводит в тот же аккаунт, а не заводит новый. Разными числами
    /// разыгрываются двое.
    ///
    /// Исходящие вызовы к api.telegram.org при этом не проходят — чата с таким
    /// номером нет, — и сервер пишет о них `warn`, как о всяком неудачном
    /// разговоре с ботом. Записка нужна человеку, а не обряду.
    pub async fn telegram_play_locally(&self, code: &str, yes: bool, who: i64) -> Result<()> {
        // Замок второй. Первый — сам маршрут: снаружи локальной машины его
        // не существует (`api::router`), а страница, которой нет, ничего не
        // сообщает о том, что на ней бывает.
        if !self.config.public_url_is_local() {
            return Err(AppError::NotFound("no such page".into()));
        }
        let man = || crate::telegram::TgUser {
            id: who,
            username: Some("local".into()),
            first_name: Some("Местный".into()),
            last_name: None,
            language_code: Some("ru".into()),
        };

        self.telegram_update(crate::telegram::Update {
            message: Some(crate::telegram::Message {
                message_id: 1,
                chat: crate::telegram::Chat { id: who },
                from: Some(man()),
                text: Some(format!("/start {code}")),
            }),
            callback_query: None,
        })
        .await?;

        self.telegram_update(crate::telegram::Update {
            message: None,
            callback_query: Some(crate::telegram::CallbackQuery {
                id: "1".into(),
                from: man(),
                message: Some(crate::telegram::Message {
                    message_id: 1,
                    chat: crate::telegram::Chat { id: who },
                    from: None,
                    text: None,
                }),
                data: Some(format!("{}:{code}", if yes { "ok" } else { "no" })),
            }),
        })
        .await?;

        // Сессию здесь НЕ спрашиваем: она отдаётся ровно один раз (условие
        // `state = 'confirmed'` стоит внутри самого UPDATE), и спросив её, дом
        // унёс бы её у той вкладки, ради которой всё и затевалось.
        Ok(())
    }

    /// Опрос со страницы. Сессия отдаётся ОДИН раз — дальше слово ничего не
    /// значит. Неизвестное и остывшее слово отвечают одинаково: разница между
    /// «такого не было» и «уже не годится» странице не нужна, а гадающему по
    /// ответу — тем более.
    pub async fn telegram_login_status(&self, code: &str) -> Result<crate::models::TelegramCodeStatus> {
        let cold = || crate::models::TelegramCodeStatus {
            state: "expired".into(),
            session_token: None,
            user: None,
        };
        let Some(row) = self.repo.find_telegram_login_code(code).await? else {
            return Ok(cold());
        };
        if row.expires_at < chrono::Utc::now() {
            return Ok(cold());
        }
        match row.state.as_str() {
            "waiting" | "asked" => Ok(crate::models::TelegramCodeStatus {
                state: row.state,
                session_token: None,
                user: None,
            }),
            "refused" => Ok(crate::models::TelegramCodeStatus {
                state: "refused".into(),
                session_token: None,
                user: None,
            }),
            // Имени ещё нет, а возраст не подтверждён.
            "age" => Ok(crate::models::TelegramCodeStatus {
                state: "age".into(),
                session_token: None,
                user: None,
            }),
            "confirmed" => {
                // Забирается самим UPDATE: два опроса, пришедшие разом, иначе
                // унесли бы одну сессию дважды.
                let Some(token) = self.repo.take_telegram_login_session(code).await? else {
                    return Ok(cold());
                };
                let user = self.repo.get_session_user(&fingerprint(&token)).await?;
                Ok(crate::models::TelegramCodeStatus {
                    state: "ready".into(),
                    session_token: Some(token),
                    user: user.as_ref().map(UserDto::from),
                })
            }
            _ => Ok(cold()),
        }
    }

    /// Всё, что бот получает от Telegram. Ошибки здесь не поднимаются наружу:
    /// Telegram на неуспешный ответ повторяет обновление, и повтор нажатия
    /// «это я» не должен заводить вторую сессию.
    pub async fn telegram_update(&self, update: crate::telegram::Update) -> Result<()> {
        let Some(bot) = self.login_bot() else {
            return Ok(());
        };
        if let Some(msg) = update.message {
            self.telegram_started(&bot, msg).await;
        } else if let Some(cb) = update.callback_query {
            self.telegram_answered(&bot, cb).await;
        }
        Ok(())
    }

    /// `/start sub_…` — человек нажал «Запустить» по ссылке подписки. Чат
    /// берётся из самого сообщения Telegram: это и есть доказательство, что
    /// канал его.
    async fn telegram_subscribed(
        &self,
        bot: &crate::telegram::Bot,
        chat: i64,
        from: &crate::telegram::TgUser,
        code: &str,
    ) {
        let lang = from.lang();
        let joined = self
            .repo
            .activate_telegram_subscriber(&fingerprint(code), chat, from.username.as_deref(), lang.code())
            .await;
        match joined {
            Ok(crate::db::TelegramJoin::Joined(sub)) => {
                self.observability
                    .record_business_event("newsletter_subscribed_telegram", "ok");
                Self::log_domain_event("newsletter_subscribed_telegram", "subscriber", sub.id, "ok");
                bot.say(chat, &self.telegram_welcome(&sub, lang)).await;
            }
            Ok(crate::db::TelegramJoin::Already(sub)) => {
                bot.say(chat, &self.telegram_welcome(&sub, lang)).await;
            }
            Ok(crate::db::TelegramJoin::Cold) => {
                bot.say(
                    chat,
                    lang.pick(
                        "That link has gone cold. Sign up again on the site.",
                        "Ссылка остыла. Оформите подписку на сайте заново.",
                        "Посилання охололо. Оформіть підписку на сайті заново.",
                        "El enlace ha caducado. Vuelva a suscribirse en el sitio.",
                        "Der Link ist abgelaufen. Bitte melden Sie sich auf der Website erneut an.",
                        "Il link è scaduto. Si iscriva di nuovo sul sito.",
                    ),
                )
                .await;
            }
            Err(e) => {
                tracing::warn!("Telegram subscription failed: {e}");
            }
        }
    }

    /// Приветствие подписчика: что будет приходить и как выйти. Выход есть в
    /// двух видах — командой и ссылкой, — чтобы не зависеть ни от сайта, ни от
    /// бота.
    fn telegram_welcome(
        &self,
        sub: &crate::models::TelegramSubscriber,
        lang: crate::lang::Lang,
    ) -> String {
        use crate::lang::Lang;
        let site = self.config.public_url.trim_end_matches('/');
        let door = format!("{site}/unsubscribe/{}", sub.unsubscribe_token);
        match lang {
            Lang::Ru => format!(
                "<b>Вы в книге дома</b>\nКогда на полке появится новая небылица, я напишу сюда — без шума и спешки.\n\nЧтобы больше не получать: /stop или {door}"
            ),
            Lang::En => format!(
                "<b>You are in the house book</b>\nWhen a new tall tale is set on the shelf, I will write here — no noise, no haste.\n\nTo stop these: /stop or {door}"
            ),
            Lang::Uk => format!(
                "<b>Ви в книзі дому</b>\nКоли на полиці з’явиться нова небилиця, я напишу сюди — без галасу й поспіху.\n\nЩоб більше не отримувати: /stop або {door}"
            ),
            Lang::Es => format!(
                "<b>Está suscrito</b>\nCuando aparezca un cuento nuevo en el sitio, le escribiré aquí. Nada más.\n\nPara no recibir más: /stop o {door}"
            ),
            Lang::De => format!(
                "<b>Sie sind angemeldet</b>\nWenn auf der Website eine neue Geschichte erscheint, schreibe ich Ihnen hier. Sonst nichts.\n\nKeine weiteren Nachrichten: /stop oder {door}"
            ),
            Lang::It => format!(
                "<b>È iscritto</b>\nQuando sul sito comparirà un nuovo racconto, le scriverò qui. Nient’altro.\n\nPer non riceverne più: /stop o {door}"
            ),
        }
    }

    /// `/stop`.
    async fn telegram_stopped(&self, bot: &crate::telegram::Bot, chat: i64, lang: crate::lang::Lang) {
        let was = self.repo.unsubscribe_telegram_chat(chat).await.unwrap_or(false);
        let text = if was {
            lang.pick(
                "Done — I will not write again. You can return from the site.",
                "Готово: больше писать не буду. Вернуться можно на сайте.",
                "Готово: більше писати не буду. Повернутися можна на сайті.",
                "Hecho: no volveré a escribirle. Puede volver a suscribirse en el sitio.",
                "Erledigt: Ich schreibe Ihnen nicht mehr. Wieder anmelden können Sie sich auf der Website.",
                "Fatto: non le scriverò più. Può iscriversi di nuovo sul sito.",
            )
        } else {
            lang.pick(
                "You are not in the house book.",
                "Вы и так не в книге дома.",
                "Ви й так не в книзі дому.",
                "No está suscrito.",
                "Sie sind nicht angemeldet.",
                "Non è iscritto.",
            )
        };
        bot.say(chat, text).await;
    }

    /// Подписаться на книгу дома в Telegram. Вошедшему через Telegram бот уже
    /// знаком — подписка оформляется сразу; остальным даётся ссылка с кодом, и
    /// подписка начинается, когда они нажмут «Запустить».
    pub async fn subscribe_via_telegram(
        &self,
        req: crate::models::TelegramSubscribeRequest,
        user: Option<&User>,
        ip: Option<String>,
    ) -> Result<crate::models::TelegramSubscribeResponse> {
        if !req.age_confirmed {
            return Err(AppError::BadRequest(
                "Please confirm you are 16 or older".to_string(),
            ));
        }
        let (Some(bot), Some(bot_name)) = (
            self.login_bot(),
            self.config.telegram_login_bot_username.as_deref(),
        ) else {
            return Err(AppError::BadRequest("Telegram is not set up".into()));
        };
        let lang = req
            .lang
            .as_deref()
            .and_then(crate::lang::Lang::parse)
            .unwrap_or_default()
            .code();
        let source = req
            .source
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= 40)
            .unwrap_or("home");

        // Вошедший через Telegram: бот ему знаком, и написать ему можно сразу.
        if let Some(u) = user
            && let Some(chat) = u.telegram_id
        {
            let (sub, already) = self
                .repo
                .subscribe_telegram_chat(chat, u.telegram_username.as_deref(), lang, source, Some(u.id), ip.as_deref())
                .await?;
            if already {
                return Ok(crate::models::TelegramSubscribeResponse {
                    link: None,
                    confirmed: true,
                    unsubscribe_token: sub.unsubscribe_token,
                });
            }
            if bot.say_why(chat, &self.telegram_welcome(&sub, crate::lang::Lang::parse(lang).unwrap_or_default())).await.is_ok() {
                self.observability
                    .record_business_event("newsletter_subscribed_telegram", "ok");
                Self::log_domain_event("newsletter_subscribed_telegram", "subscriber", sub.id, "ok");
                return Ok(crate::models::TelegramSubscribeResponse {
                    link: None,
                    confirmed: true,
                    unsubscribe_token: sub.unsubscribe_token,
                });
            }
            // Бот не смог написать (заблокирован): подписка без канала не
            // подписка. Снимаем и просим нажать «Запустить» заново.
            let _ = self
                .repo
                .telegram_subscriber_unreachable(sub.id, "the bot could not write at sign-up")
                .await;
        }

        // Код ссылки: `sub_` и 32 знака — Telegram берёт в `start` до 64.
        let code = format!("{TG_SUBSCRIBE_PREFIX}{}", Uuid::new_v4().simple());
        let sub = self
            .repo
            .create_pending_telegram_subscriber(&fingerprint(&code), lang, source, user.map(|u| u.id), ip.as_deref())
            .await?;
        if self.config.public_url_is_local() {
            tracing::info!(
                "[local] Telegram book subscription: the bot cannot reach this host — send /start {code} to the bot from a test webhook"
            );
        }
        Ok(crate::models::TelegramSubscribeResponse {
            link: Some(format!("https://t.me/{bot_name}?start={code}")),
            confirmed: false,
            unsubscribe_token: sub.unsubscribe_token,
        })
    }

    /// Записки о новой небылице подписчикам книги в Telegram. Раскладывает их
    /// тот же проход, что и письма; здесь — отправка.
    pub async fn send_tale_book_notes(&self) -> Result<usize> {
        let local = self.config.public_url_is_local();
        let bot = self.login_bot();
        if !local && bot.is_none() {
            return Ok(0);
        }
        let site = self.config.public_url.trim_end_matches('/').to_string();
        let due = self
            .repo
            .claim_book_notes(TALE_LETTERS_PER_TICK, TALE_LETTER_MAX_ATTEMPTS, TALE_LETTER_CLAIM_SECS)
            .await?;
        let mut sent = 0;
        for note in due {
            let Some(chat) = note.chat_id else { continue };
            let lang = crate::lang::Lang::parse(&note.lang).unwrap_or_default();
            let title = crate::tales::in_lang(
                lang,
                Some(&note.title_en),
                Some(&note.title_ru),
                Some(&note.title_uk),
                Some(&note.title_es),
                Some(&note.title_de),
                Some(&note.title_it),
            )
            .unwrap_or_default();
            let dek = crate::tales::in_lang(
                lang,
                note.dek_en.as_deref(),
                note.dek_ru.as_deref(),
                note.dek_uk.as_deref(),
                note.dek_es.as_deref(),
                note.dek_de.as_deref(),
                note.dek_it.as_deref(),
            );
            let link = format!("{site}/tales/{}?src=telegram_book", note.slug);
            let (text, button) = crate::tales::compose_book_note(lang, &title, dek.as_deref());
            let Some(bot) = bot.as_ref().filter(|_| !local) else {
                tracing::info!("[local] book note to Telegram chat {chat} → {link}");
                self.repo.book_note_sent(note.tale_id, note.subscriber_id).await?;
                sent += 1;
                continue;
            };
            match bot.post_text(&chat.to_string(), &text, button, &link).await {
                Ok(_) => {
                    self.repo.book_note_sent(note.tale_id, note.subscriber_id).await?;
                    sent += 1;
                }
                Err(why) => {
                    tracing::warn!("Book note to Telegram chat {chat} failed: {why}");
                    // Заблокировал бота или удалил чат: ждать нечего, и
                    // подписка снимается, чтобы не стучаться в закрытую дверь.
                    let gone = ["blocked", "chat not found", "deactivated", "kicked"]
                        .iter()
                        .any(|m| why.contains(m));
                    if gone {
                        self.repo.telegram_subscriber_unreachable(note.subscriber_id, &why).await?;
                    }
                    self.repo.book_note_failed(note.tale_id, note.subscriber_id, &why).await?;
                }
            }
        }
        Ok(sent)
    }

    /// `/start <слово>` — показать записку с двумя кнопками.
    async fn telegram_started(&self, bot: &crate::telegram::Bot, msg: crate::telegram::Message) {
        let Some(from) = msg.from else { return };
        let lang = from.lang();
        let text = msg.text.unwrap_or_default();

        // `/stop` — выйти из книги дома в Telegram, не заходя на сайт.
        if crate::telegram::is_stop(&text) {
            self.telegram_stopped(bot, msg.chat.id, lang).await;
            return;
        }

        let Some(code) = crate::telegram::start_code(&text) else {
            // Открыли бота просто так. Вход начинают на сайте, и сказать об
            // этом — единственное, что здесь уместно.
            bot.say(
                msg.chat.id,
                lang.pick(
                    "This bot signs you in. Start on the site: it shows a word, and I will ask for it here.",
                    "Это бот входа. Начните вход на сайте — он покажет слово, и я спрошу его здесь.",
                    "Це бот входу. Почніть вхід на сайті — він покаже слово, і я спитаю його тут.",
                    "Este bot sirve para entrar. Empiece en el sitio: le mostrará una palabra, y yo se la preguntaré aquí.",
                    "Dieser Bot meldet Sie an. Beginnen Sie auf der Website: Sie zeigt ein Wort, und ich frage hier danach.",
                    "Questo bot serve per accedere. Inizi sul sito: le mostrerà una parola, e io gliela chiederò qui.",
                ),
            )
            .await;
            return;
        };

        // Код подписки на книгу дома, а не вход.
        if code.starts_with(TG_SUBSCRIBE_PREFIX) {
            self.telegram_subscribed(bot, msg.chat.id, &from, &code).await;
            return;
        }

        let cold = lang.pick(
            "That word has gone cold. Start again on the site.",
            "Слово остыло. Начните вход на сайте заново.",
            "Слово охололо. Почніть вхід на сайті заново.",
            "La palabra ha caducado. Vuelva a empezar en el sitio.",
            "Das Wort ist abgelaufen. Beginnen Sie auf der Website noch einmal.",
            "La parola è scaduta. Ricominci sul sito.",
        );
        let Ok(Some(row)) = self.repo.find_telegram_login_code(&code).await else {
            bot.say(msg.chat.id, cold).await;
            return;
        };
        if row.state != "waiting" || row.expires_at < chrono::Utc::now() {
            bot.say(msg.chat.id, cold).await;
            return;
        }

        // Записка называет слово и откуда пришёл запрос: «Chrome, Москва»
        // человеку в Вологде говорит достаточно, чтобы не нажимать.
        let esc = crate::telegram::esc;
        let where_from = {
            let mut parts: Vec<String> = Vec::new();
            if let Some(b) = row.browser.as_deref().map(browser_name).filter(|b| !b.is_empty()) {
                parts.push(b);
            }
            let place: Vec<&str> = [row.city.as_deref(), row.country_code.as_deref()]
                .into_iter()
                .flatten()
                .filter(|s| !s.trim().is_empty())
                .collect();
            if !place.is_empty() {
                parts.push(place.join(", "));
            }
            parts.join(" · ")
        };
        let from_line = |label: &str| {
            if where_from.is_empty() {
                String::new()
            } else {
                format!("\n{label}: {}", esc(&where_from))
            }
        };
        let note = match lang {
            crate::lang::Lang::Ru => format!(
                "<b>Вход на сайт</b>\nСлово: <b>{}</b>{}\n\nЕсли на сайте другое слово или вы не начинали вход — нажмите «Не я».",
                esc(&row.word),
                from_line("Откуда"),
            ),
            crate::lang::Lang::En => format!(
                "<b>Signing in</b>\nWord: <b>{}</b>{}\n\nIf the site shows a different word, or you did not start this, press “Not me”.",
                esc(&row.word),
                from_line("From"),
            ),
            crate::lang::Lang::Uk => format!(
                "<b>Вхід на сайт</b>\nСлово: <b>{}</b>{}\n\nЯкщо на сайті інше слово або ви не починали вхід — натисніть «Не я».",
                esc(&row.word),
                from_line("Звідки"),
            ),
            crate::lang::Lang::Es => format!(
                "<b>Entrar en el sitio</b>\nPalabra: <b>{}</b>{}\n\nSi el sitio muestra otra palabra o usted no ha empezado a entrar, pulse «No soy yo».",
                esc(&row.word),
                from_line("Desde"),
            ),
            crate::lang::Lang::De => format!(
                "<b>Anmeldung auf der Website</b>\nWort: <b>{}</b>{}\n\nWenn die Website ein anderes Wort zeigt oder Sie die Anmeldung nicht begonnen haben, drücken Sie „Nicht ich“.",
                esc(&row.word),
                from_line("Von"),
            ),
            crate::lang::Lang::It => format!(
                "<b>Accesso al sito</b>\nParola: <b>{}</b>{}\n\nSe il sito mostra un’altra parola o non ha iniziato lei l’accesso, prema «Non sono io».",
                esc(&row.word),
                from_line("Da"),
            ),
        };
        let (yes, no) = match lang {
            crate::lang::Lang::Ru => ("Это я, войти", "Не я"),
            crate::lang::Lang::En => ("It's me, sign in", "Not me"),
            crate::lang::Lang::Uk => ("Це я, увійти", "Не я"),
            crate::lang::Lang::Es => ("Soy yo, entrar", "No soy yo"),
            crate::lang::Lang::De => ("Ich bin es, anmelden", "Nicht ich"),
            crate::lang::Lang::It => ("Sono io, accedi", "Non sono io"),
        };

        let message_id = bot.ask(msg.chat.id, &note, yes, no, &code).await;
        // Записка отправлена — теперь закрепляем за словом того, кто её видит.
        // Условие `state = 'waiting'` внутри: два `/start` подряд не должны
        // переписывать друг друга, иначе кнопка относилась бы к чужой записке.
        let taken = self
            .repo
            .telegram_login_asked(
                &code,
                from.id,
                from.username.as_deref(),
                from.first_name.as_deref(),
                msg.chat.id,
                message_id,
            )
            .await
            .unwrap_or(false);
        if !taken {
            bot.say(msg.chat.id, cold).await;
        }
    }

    /// Нажали кнопку под запиской.
    async fn telegram_answered(&self, bot: &crate::telegram::Bot, cb: crate::telegram::CallbackQuery) {
        let lang = cb.from.lang();
        let Some(answer) = cb.data.as_deref().and_then(crate::telegram::Answer::parse) else {
            return;
        };
        let chat_id = cb.message.as_ref().map(|m| m.chat.id);
        let message_id = cb.message.as_ref().map(|m| m.message_id);
        let settle = |text: String| async move {
            if let (Some(chat), Some(id)) = (chat_id, message_id) {
                bot.settle(chat, id, &text).await;
            }
        };

        let code = match &answer {
            crate::telegram::Answer::Yes(c) | crate::telegram::Answer::No(c) => c.clone(),
        };

        if let crate::telegram::Answer::No(_) = answer {
            let _ = self.repo.telegram_login_refused(&code, cb.from.id).await;
            bot.ack(&cb.id, lang.pick(
                "Cancelled",
                "Вход отменён",
                "Вхід скасовано",
                "Cancelado",
                "Abgebrochen",
                "Annullato",
            )).await;
            settle(
                lang.pick(
                    "Sign-in cancelled.",
                    "Вход отменён.",
                    "Вхід скасовано.",
                    "Entrada cancelada.",
                    "Anmeldung abgebrochen.",
                    "Accesso annullato.",
                )
                    .to_string(),
            )
            .await;
            return;
        }

        let cold = lang.pick(
            "That word has gone cold",
            "Слово остыло",
            "Слово охололо",
            "La palabra ha caducado",
            "Das Wort ist abgelaufen",
            "La parola è scaduta",
        );
        let Ok(Some(row)) = self.repo.find_telegram_login_code(&code).await else {
            bot.ack(&cb.id, cold).await;
            return;
        };
        if row.state != "asked" || row.telegram_id != Some(cb.from.id) || row.expires_at < chrono::Utc::now() {
            bot.ack(&cb.id, cold).await;
            return;
        }

        match self.telegram_admit(&row, &cb.from).await {
            Ok(user_id) => {
                // Сессия достаётся ТОМУ браузеру, который просил вход, поэтому
                // её обстоятельства взяты из строки слова, а не из запроса
                // Telegram: телефон бота — не то устройство, что входит.
                let ctx = crate::models::ClientContext {
                    ip: row.ip.clone(),
                    user_agent: row.browser.clone(),
                    country_code: row.country_code.clone(),
                    city: row.city.clone(),
                };
                let Ok(token) = self.open_session(user_id, &ctx).await else {
                    bot.ack(
                        &cb.id,
                        lang.pick(
                            "Something went wrong",
                            "Не получилось",
                            "Не вдалося",
                            "Algo ha fallado",
                            "Das hat nicht geklappt",
                            "Qualcosa non ha funzionato",
                        ),
                    )
                    .await;
                    return;
                };
                let stored = self
                    .repo
                    .telegram_login_confirmed(&code, cb.from.id, &token)
                    .await
                    .unwrap_or(false);
                if !stored {
                    // Гонка: слово уже отвечено или остыло. Сессию не оставляем
                    // висеть — её никто не заберёт.
                    let _ = self.repo.delete_session(&fingerprint(&token)).await;
                    bot.ack(&cb.id, cold).await;
                    return;
                }
                Self::log_domain_event("telegram_login", "user", user_id, "ok");

                bot.ack(&cb.id, lang.pick("Done", "Готово", "Готово", "Hecho", "Erledigt", "Fatto")).await;
                // Возвращать надо ВО ВКЛАДКУ, а не «на сайт»: сессия лежит
                // для того браузера, который просил вход, и открытая по ссылке
                // новая вкладка показала бы главную не вошедшему — то есть
                // сказала бы, что вход не удался. Адрес назван вторым, на
                // случай закрытой вкладки.
                let back = &self.config.public_url;
                settle(format!(
                    "{}\n{}: {}",
                    lang.pick(
                        "Done. Go back to the browser tab where you started — it has already let you in.",
                        "Готово. Вернитесь во вкладку браузера, где начинали вход, — она уже впустила вас.",
                        "Готово. Поверніться до вкладки браузера, де починали вхід, — вона вже впустила вас.",
                        "Hecho. Vuelva a la pestaña del navegador donde empezó: ya le ha dejado entrar.",
                        "Erledigt. Kehren Sie zum Browser-Tab zurück, in dem Sie begonnen haben: Dort sind Sie bereits angemeldet.",
                        "Fatto. Torni alla scheda del browser in cui ha iniziato: l’accesso è già avvenuto.",
                    ),
                    lang.pick(
                        "If that tab is closed",
                        "Если вкладка закрыта",
                        "Якщо вкладку закрито",
                        "Si cerró esa pestaña",
                        "Falls der Tab geschlossen ist",
                        "Se la scheda è chiusa",
                    ),
                    crate::telegram::esc(back)
                ))
                .await;
            }
            Err(AppError::BadRequest(m)) if m == "age" => {
                // Слово погашено с причиной: странице скажут про регистрацию, а
                // не «вход отменён» — человек ничего не отменял.
                let _ = self.repo.telegram_login_aged(&code, cb.from.id).await;
                let said = lang.pick(
                    "There is no name for this Telegram yet. A new name is made on the sign-up page — tick your age there, then sign in again.",
                    "Для этого Telegram у нас ещё нет имени. Новое имя заводится на странице регистрации: отметьте там возраст и войдите снова.",
                    "Для цього Telegram у нас іще немає імені. Нове ім’я заводиться на сторінці реєстрації: позначте там вік і увійдіть знову.",
                    "Todavía no hay una cuenta para este Telegram. Las cuentas nuevas se crean en la página de registro: confirme allí su edad y vuelva a entrar.",
                    "Für dieses Telegram gibt es noch kein Konto. Neue Konten entstehen auf der Registrierungsseite: Bestätigen Sie dort Ihr Alter und melden Sie sich erneut an.",
                    "Per questo Telegram non c’è ancora un account. Gli account nuovi si creano nella pagina di registrazione: confermi lì la sua età e acceda di nuovo.",
                );
                bot.ack(
                    &cb.id,
                    lang.pick(
                        "Age needs confirming",
                        "Нужно подтвердить возраст",
                        "Потрібно підтвердити вік",
                        "Hay que confirmar la edad",
                        "Das Alter muss bestätigt werden",
                        "Bisogna confermare l’età",
                    ),
                )
                .await;
                settle(said.to_string()).await;
            }
            Err(e) => {
                let said = match &e {
                    AppError::Conflict(_) => lang.pick(
                        "This Telegram is already linked to another account",
                        "Этот Telegram уже привязан к другому имени",
                        "Цей Telegram уже прив’язано до іншого імені",
                        "Este Telegram ya está vinculado a otra cuenta",
                        "Dieses Telegram ist bereits mit einem anderen Konto verbunden",
                        "Questo Telegram è già collegato a un altro account",
                    ),
                    _ => lang.pick(
                        "Could not sign in",
                        "Войти не удалось",
                        "Увійти не вдалося",
                        "No se ha podido entrar",
                        "Anmeldung fehlgeschlagen",
                        "Accesso non riuscito",
                    ),
                };
                bot.ack(&cb.id, said).await;
                settle(said.to_string()).await;
            }
        }
    }

    /// Кого впускать: привязка к названному аккаунту, знакомое число или
    /// первый приход. Возвращает того, кому заводить сессию.
    async fn telegram_admit(
        &self,
        row: &crate::models::TelegramLoginCode,
        from: &crate::telegram::TgUser,
    ) -> Result<Uuid> {
        let username = from.username.as_deref();

        // Привязка из профиля: аккаунт уже назван, дверь к нему добавляется.
        if let Some(user_id) = row.bind_user_id {
            let user = self
                .repo
                .link_telegram_to_user(user_id, from.id, username)
                .await?;
            Self::log_domain_event("telegram_linked", "user", user.id, "ok");
            return Ok(user.id);
        }

        // Вернувшийся. Узнаём по числу — и только по нему.
        if let Some(user) = self.repo.find_user_by_telegram_id(from.id).await? {
            // Заблокированному эта дверь закрыта так же, как и первая: иначе
            // блокировка означала бы «нельзя войти значками».
            if user.is_blocked {
                return Err(AppError::Unauthorized);
            }
            // `@имя` освежается, имя в профиле — никогда: его человек мог
            // выбрать себе сам.
            let _ = self.repo.touch_telegram_username(user.id, username).await;
            return Ok(user.id);
        }

        // Первый приход — заводится НОВОЕ имя, а значит, нужно самоподтверждение
        // возраста (политика: на каждой форме, где собираются контакты). Так же,
        // как у Google и Apple (`oauth_admit`): вернувшийся и привязка из
        // профиля выше уже вернулись.
        if !row.age_confirmed {
            return Err(AppError::BadRequest("age".into()));
        }
        // Ни почты, ни значков: почту спросит первое дело, которому она нужна.
        let ctx = crate::models::ClientContext {
            ip: row.ip.clone(),
            user_agent: row.browser.clone(),
            country_code: row.country_code.clone(),
            city: row.city.clone(),
        };
        let user = self
            .repo
            .create_telegram_user(from.id, username, &from.display_name(), &ctx)
            .await?;
        Self::log_domain_event("user_registered_telegram", "user", user.id, "ok");
        Ok(user.id)
    }

    pub async fn get_user_from_session(&self, token: &str) -> Result<User> {
        self.repo
            .get_session_user(&fingerprint(token))
            .await?
            .ok_or(AppError::Unauthorized)
    }

    /// Игрок боя: вошедший или гость. Только для ручек боя — все прочие
    /// спрашивают `get_user_from_session`, а он гостей не пускает.
    pub async fn get_player_from_session(&self, token: &str) -> Result<User> {
        self.repo
            .get_player_session_user(&fingerprint(token))
            .await?
            .ok_or(AppError::Unauthorized)
    }

    /// Гость боя: имя без почты и дверей и сессия на месяц. Заводится нажатием
    /// «Начать бой» в обучении, чтобы первый бой доигрывался без формы входа.
    ///
    /// Имён можно было бы завести сколько угодно, поэтому три замка: предел на
    /// адрес, общий предел в час и то, что имя ничем, кроме боя с заданной
    /// расстановкой, не владеет (ни пыли, ни писем, ни других ручек).
    pub async fn begin_guest_player(
        &self,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<String> {
        self.check_rate_limit("guest", ip.as_deref().unwrap_or("unknown"), 20, 3600)
            .await?;
        self.check_rate_limit("guest_all", "house", 2000, 3600).await?;
        let ctx = self.client_context(ip, user_agent);
        let user = self.repo.create_guest_user(&ctx).await?;
        let token = self.open_session(user.id, &ctx).await?;
        Self::log_domain_event("guest_player_started", "user", user.id, "ok");
        Ok(token)
    }

    /// Суточная уборка: гости старше месяца.
    pub async fn prune_stale_guests(&self) -> Result<u64> {
        self.repo.prune_stale_guests(35).await
    }

    /// Свои двери и как закрыть все, кроме этой.
    pub async fn own_sessions(&self, user: &User, token: &str) -> Result<Vec<crate::models::OwnSessionDto>> {
        let mut doors = self.repo.own_sessions(user.id, &fingerprint(token)).await?;
        for door in doors.iter_mut() {
            door.browser = door
                .browser
                .as_deref()
                .map(browser_name)
                .filter(|b| !b.is_empty());
        }
        Ok(doors)
    }

    pub async fn close_other_sessions(&self, user: &User, token: &str) -> Result<u64> {
        let closed = self
            .repo
            .revoke_other_sessions(user.id, &fingerprint(token))
            .await?;
        Self::log_domain_event("user_sessions_closed", "user", user.id, "ok");
        Ok(closed)
    }

    pub async fn logout(&self, token: &str) -> Result<()> {
        self.repo.delete_session(&fingerprint(token)).await?;
        Self::log_domain_event("user_logout", "session", "current", "ok");
        Ok(())
    }

    pub async fn link_bookings(&self, user_id: Uuid, cancel_tokens: &[String]) -> Result<usize> {
        let user = self
            .repo
            .find_user_by_id(user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;
        // Гостевые брони привязываются по совпадению почты — это и есть защита
        // от чужой расписки. Почта при этом должна быть ПОДТВЕРЖДЁННОЙ: иначе
        // достаточно было бы набрать чужой адрес, чтобы чужие расписки стали
        // совпадать с твоим именем. У аккаунта без подтверждённой почты
        // совпадать нечему; откроется ящик — привяжутся тогда.
        if !user.email_confirmed() {
            return Ok(0);
        }
        let Some(email) = user.email.as_deref() else {
            return Ok(0);
        };
        self.repo
            .link_bookings_to_user(user_id, email, cancel_tokens)
            .await
    }

    pub async fn get_user_bookings(&self, user_id: Uuid) -> Result<Vec<UserBookingDto>> {
        let bookings = self.repo.get_user_bookings(user_id).await?;
        let ids: Vec<Uuid> = bookings.iter().map(|b| b.figurine_id).collect();
        let names = self.repo.get_figurine_name_translations(&ids).await?;
        Ok(bookings
            .into_iter()
            .map(|b| UserBookingDto {
                id: b.id.to_string(),
                figurine_id: b.figurine_id.to_string(),
                figurine_name: b.figurine_name,
                figurine_name_translations: names.get(&b.figurine_id).cloned().unwrap_or_default(),
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
        // Бронь, лист ожидания и «сообщить о наличии» сверяют почту — одного
        // токена мало. У аккаунта без почты эти три пути закрыты, а заказная
        // работа ниже привязывается только по токену и остаётся доступной.
        if let Some(email) = user.email.as_deref() {
            if let Some(m) = self.repo.link_booking_by_token(user.id, email, token).await? {
                return Ok(Self::claim_response("booking", m));
            }
            if let Some(m) = self.repo.link_waitlist_by_token(user.id, email, token).await? {
                return Ok(Self::claim_response("waitlist", m));
            }
            if let Some(m) = self
                .repo
                .link_notify_order_by_token(user.id, email, token)
                .await?
            {
                return Ok(Self::claim_response("notify", m));
            }
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
        let ids: Vec<Uuid> = orders
            .iter()
            .filter_map(|o| Uuid::parse_str(&o.figurine_id.to_string()).ok())
            .collect();
        let names = self.repo.get_figurine_name_translations(&ids).await?;
        Ok(orders
            .into_iter()
            .map(|o| {
                let certificate = Self::certificate_dto(&o);
                let figurine_name_translations = Uuid::parse_str(&o.figurine_id.to_string())
                    .ok()
                    .and_then(|id| names.get(&id).cloned())
                    .unwrap_or_default();
                UserOrderDto {
                    id: o.id.to_string(),
                    figurine_id: o.figurine_id,
                    figurine_name: o.figurine_name,
                    figurine_name_translations,
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
        // Неудачные попытки записаны на почту. У аккаунта без почты этой
        // двери нет вовсе, значит и неудачных попыток по ней не бывает.
        let recent_failures = match user.email.as_deref() {
            Some(email) => {
                self.repo
                    .recent_failures(email, None, 24 * 60, 24 * 60)
                    .await?
                    .by_email
            }
            None => 0,
        };
        let messages = self.admin_get_user_threads(user_id).await?;

        Ok(AdminUserDetail {
            id: user.id.to_string(),
            email: user.email,
            display_name: user.display_name,
            telegram_username: user.telegram_username.clone(),
            telegram_linked: user.telegram_id.is_some(),
            google_linked: user.google_sub.is_some(),
            google_email: user.google_email.clone(),
            apple_linked: user.apple_sub.is_some(),
            apple_email: user.apple_email.clone(),
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

        let token = secret_key();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(RESET_HOURS);
        self.repo
            .admin_create_reset_token(user_id, &fingerprint(&token), expires_at)
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
            .find_user_by_reset_hash(&fingerprint(token))
            .await?
            .ok_or_else(|| AppError::BadRequest("Reset link is invalid or has expired.".into()))?;
        Ok(UserDto::from(&user))
    }

    pub async fn apply_password_reset(
        &self,
        req: &ApplyPasswordResetRequest,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<ApplyPasswordResetResponse> {
        let user = self
            .repo
            .find_user_by_reset_hash(&fingerprint(&req.token))
            .await?
            .ok_or_else(|| AppError::BadRequest("Reset link is invalid or has expired.".into()))?;

        // Reset regenerates the personal pool, so validate it the same way as
        // registration and persist it alongside the new hash.
        let pool_json = validate_pool(&req.pool, &req.selections)?;

        let hash_input = build_hash_input(&req.selections);
        let new_hash = hash_password(&hash_input, &self.config.auth_pepper)
            .map_err(|e| AppError::Internal(format!("Hash error: {e}")))?;

        // Record where the reset was applied from (audit), then persist.
        let ctx = self.client_context(ip, user_agent);

        // Invalidate all existing sessions so old password can't be used
        self.repo.admin_revoke_all_sessions(user.id).await?;
        self.repo
            .apply_password_reset(user.id, &new_hash, &pool_json, &ctx)
            .await?;
        Self::log_domain_event("password_reset_applied", "user", user.id, "ok");

        // Старые сессии погашены выше; взамен — одна новая, для этого браузера.
        // Заблокированному её не выдают: знаки ему назначены, но дверь закрыта.
        if user.is_blocked {
            return Ok(ApplyPasswordResetResponse { session_token: None, user: None });
        }
        let session = self.open_session(user.id, &ctx).await?;
        let fresh = self.repo.find_user_by_id(user.id).await?;
        Ok(ApplyPasswordResetResponse {
            session_token: Some(session),
            user: fresh.as_ref().map(UserDto::from),
        })
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
        self.send_the_way_back(&user, self.client_context(ip, user_agent))
            .await
    }

    /// Прислать ключ к знакам — по своей просьбе изнутри дома.
    ///
    /// Та же дорога, что и «забыли знаки», только спрашивать адрес не нужно:
    /// вошедший уже назван. Это единственный путь к знакам для пришедшего через
    /// Telegram — у него почты может не быть вовсе, а страница входа, где
    /// живёт «забыли знаки», вошедшему не показывается.
    pub async fn ask_for_signs(
        &self,
        user: &User,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<()> {
        if !self.proven_channels(user).await.any() {
            return Err(AppError::BadRequest(
                "No proven way to reach you — confirm an email or link Telegram first".into(),
            ));
        }
        self.send_the_way_back(user, self.client_context(ip, user_agent))
            .await
    }

    /// Каналы, которые дом считает доказанными.
    ///
    /// Доказан тот, о котором известно, что он принадлежит хозяину имени:
    /// почта — переходом по ссылке из письма, Telegram — нажатием кнопки в
    /// боте. Названный, но не открытый адрес каналом не является: по нему не
    /// проверено даже то, что ящик его.
    async fn proven_channels(&self, user: &User) -> Channels {
        Channels {
            email: user.email_confirmed().then(|| user.email.clone()).flatten(),
            telegram: self.login_bot().and(user.telegram_id),
        }
    }

    /// Отправить дорогу назад во все доказанные каналы сразу.
    ///
    /// Во ВСЕ, а не в лучший: какой из них человек сейчас читает, дом не знает,
    /// а ключ один и тот же — вторая копия ничего не добавляет к тому, что уже
    /// может первая.
    ///
    /// Ни одного доказанного канала — дом молчит, но если адрес назван и
    /// отправить письмо есть чем, уходит приглашение открыть ящик: человеку
    /// нужен именно этот следующий шаг. Ответ снаружи в любом случае один и тот
    /// же, потому что разница между «нет такого» и «есть, но недоказан»
    /// перечисляет жильцов ровно так же, как разница между «нет» и «есть».
    async fn send_the_way_back(
        &self,
        user: &User,
        ctx: crate::models::ClientContext,
    ) -> Result<()> {
        let ways = self.proven_channels(user).await;
        if !ways.any() {
            if user.email.is_some() && self.mail_works().await {
                let svc = self.clone();
                let user = user.clone();
                tokio::spawn(async move {
                    if let Err(e) = svc.resend_confirm_email(&user).await {
                        tracing::warn!("Confirmation letter failed: {e}");
                    }
                });
            }
            return Ok(());
        }

        let token = secret_key();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(RESET_HOURS);
        self.repo
            .create_self_reset_token(user.id, &fingerprint(&token), expires_at, &ctx)
            .await?;
        Self::log_domain_event("password_reset_requested", "user", user.id, "ok");

        // Вдогонку: медленный SMTP не должен держать ответ, а по времени ответа
        // не должно быть видно, есть ли такое имя.
        let svc = self.clone();
        tokio::spawn(async move {
            if let Some(to) = ways.email {
                if let Err(e) = svc.send_password_reset_email(&to, &token).await {
                    tracing::warn!("Password reset email failed: {e}");
                }
            }
            if let Some(chat) = ways.telegram {
                svc.send_password_reset_note(chat, &token).await;
            }
        });

        Ok(())
    }

    /// Ключ к знакам — запиской в бот.
    ///
    /// Заведено затем, что почта есть не у всех и настроена не у всех, а бот
    /// уже стоит и ничего не стоит. Для пришедшего через Telegram это
    /// единственная дорога назад.
    async fn send_password_reset_note(&self, chat: i64, token: &str) {
        let Some(bot) = self.login_bot() else { return };
        let link = format!(
            "{}/set-password?token={}",
            self.config.public_url.trim_end_matches('/'),
            token
        );
        let note = format!(
            "<b>Новые знаки</b>\nКто-то попросил новые знаки для этого имени. \
             Если это вы — перейдите по ссылке в ближайшие два часа:\n{}\n\n\
             Если это не вы, ничего не делайте: знаки не менялись.",
            crate::telegram::esc(&link)
        );
        bot.say(chat, &note).await;
    }

    /// Письмо об утверждении. ОДНО письмо на весь путь работы.
    ///
    /// Записки в профиле бесплатны и никого не тревожат, поэтому почтой дом
    /// говорит только там, где иначе человек не узнает вовремя: работу взяли в
    /// игру. На допуск, отказ и место в тройке письма нет — это разговор внутри
    /// комнаты, и превращать его в поток писем значит потерять и это одно.
    ///
    /// Отправляется вдогонку (`spawn`): медленный SMTP не должен держать стол
    /// хозяина, а неудача письма не отменяет утверждения.
    async fn send_studio_approved_email(
        &self,
        to: &str,
        work: &str,
        lang: crate::lang::Lang,
    ) -> Result<()> {
        use crate::lang::Lang;
        let site = self.config.public_url.trim_end_matches('/');
        let (subject, body_text) = match lang {
            Lang::En => (
                "Your frame was taken into the game",
                format!(
                    "“{work}” is approved.\n\nIt gets a run of licences with your name on them — \
                     for good. Your works: {site}/studio/gallery\n\n\
                     This letter goes out once, when a work of yours is taken in. \
                     Nothing else from the studio reaches your inbox — the rest waits \
                     for you in the house."
                ),
            ),
            Lang::Ru => (
                "Вашу раму взяли в игру",
                format!(
                    "«{work}» утверждена.\n\nОна получает тираж лицензий и ваше имя на них — \
                     навсегда. Работы дома: {site}/studio/gallery\n\n\
                     Это письмо приходит один раз — когда вашу работу берут в игру. \
                     Больше студия почтой не пишет: остальное ждёт вас в доме."
                ),
            ),
            Lang::Uk => (
                "Вашу раму взяли в гру",
                format!(
                    "«{work}» затверджена.\n\nВона отримує наклад ліцензій і ваше ім’я на них — \
                     назавжди. Роботи дому: {site}/studio/gallery\n\n\
                     Цей лист приходить один раз — коли вашу роботу беруть у гру. \
                     Більше студія поштою не пише: решта чекає на вас у домі."
                ),
            ),
            Lang::Es => (
                "Su marco ha entrado en el juego",
                format!(
                    "«{work}» ha sido aprobado.\n\nRecibe una tirada de licencias con su nombre, \
                     para siempre. Sus trabajos: {site}/studio/gallery\n\n\
                     Esta carta se envía una sola vez, cuando uno de sus trabajos entra en el juego. \
                     El estudio no le escribe nada más por correo: el resto le espera en el sitio."
                ),
            ),
            Lang::De => (
                "Ihr Rahmen wurde ins Spiel aufgenommen",
                format!(
                    "„{work}“ ist bestätigt.\n\nEr erhält eine Auflage von Lizenzen mit Ihrem Namen, \
                     dauerhaft. Ihre Arbeiten: {site}/studio/gallery\n\n\
                     Dieser Brief kommt nur einmal, wenn eine Ihrer Arbeiten ins Spiel aufgenommen wird. \
                     Sonst schreibt Ihnen das Studio keine E-Mails: Alles Weitere finden Sie auf der Website."
                ),
            ),
            Lang::It => (
                "La sua cornice è entrata nel gioco",
                format!(
                    "«{work}» è stata approvata.\n\nRiceve una tiratura di licenze con il suo nome, \
                     per sempre. I suoi lavori: {site}/studio/gallery\n\n\
                     Questa lettera arriva una sola volta, quando un suo lavoro entra nel gioco. \
                     Lo studio non le scrive altro per posta: il resto la aspetta sul sito."
                ),
            ),
        };

        self.send_mail(to, subject, body_text).await
    }

    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<()> {
        let link = format!(
            "{}/set-password?token={}",
            self.config.public_url.trim_end_matches('/'),
            token
        );
        let body_text = format!(
            "Someone asked to restore the way into the archive for this address.\n\n\
            If it was you, follow this passage within two hours:\n\n\
            {link}\n\n\
            You will be asked to choose your signs anew.\n\n\
            If it was not you, no harm is done — ignore this letter and nothing changes.",
        );

        self.say_or_send(to, "A key to the archive", body_text, &link)
            .await
    }

    /// Письмо-приглашение: по нему адрес становится дверью, а имя — живым.
    async fn send_confirm_email(&self, to: &str, token: &str) -> Result<()> {
        let link = format!(
            "{}/confirm?token={}",
            self.config.public_url.trim_end_matches('/'),
            token
        );
        let body_text = format!(
            "A name in the house was started with this address.\n\n\
            Follow this passage within two days and you are inside:\n\n\
            {link}\n\n\
            If it was not you, no harm is done — ignore this letter, \
            and the name stays unopened."
        );
        self.say_or_send(to, "The door to the house", body_text, &link)
            .await
    }

    /// Письмо, без которого локально не войти.
    ///
    /// На своей машине SMTP не настроен, а заведение имени теперь кончается
    /// письмом — то есть без этого разработчик не может войти в дом, который
    /// сам и поднял. Ссылка печатается в журнал, и только на локальном адресе:
    /// снаружи `public_url_is_local` ложен, и ветка не существует.
    async fn say_or_send(&self, to: &str, subject: &str, body: String, link: &str) -> Result<()> {
        if self.config.public_url_is_local() {
            tracing::info!("[local] letter to {to} — follow: {link}");
            if let Err(e) = self.send_mail(to, subject, body).await {
                tracing::info!("[local] the letter itself did not go out ({e}) — use the link above");
            }
            return Ok(());
        }
        self.send_mail(to, subject, body).await
    }

    /// Ответ на попытку завести имя на уже занятый адрес.
    ///
    /// Письмо уходит ВЛАДЕЛЬЦУ ящика, а не тому, кто нажал: если нажал
    /// посторонний, хозяин узнает об этом и ничего не потеряет, а посторонний
    /// не узнает ничего вовсе — страница отвечает ему то же самое, что и на
    /// свободный адрес.
    async fn send_already_have_name_email(&self, to: &str) -> Result<()> {
        let site = self.config.public_url.trim_end_matches('/');
        let body_text = format!(
            "Someone started a new name with this address — and a name here \
            already stands on it.\n\nIf that was you: the way in is {site}/login. \
            Forgotten your signs? The same page asks for a new key.\n\n\
            If it was not you, nothing happened and nothing changed."
        );
        self.send_mail(to, "You already have a name here", body_text)
            .await
    }

    /// Одно место, где дом берётся за почту.
    ///
    /// Настройки берутся из базы, а из окружения — только то, чего в базе нет.
    /// Раньше эти двенадцать строк стояли в каждом письме отдельно, и
    /// разойтись им было делом одной правки.
    ///
    /// Сама отправка разделена надвое: `mailer` собирает транспорт и обратный
    /// адрес, `deliver` отправляет одно письмо. Проход писем о байках берёт
    /// почту один раз на все письма прохода: с пулом соединений lettre это
    /// одно соединение с почтовым сервером, а не рукопожатие на каждое письмо.
    async fn send_mail(&self, to: &str, subject: &str, body: String) -> Result<()> {
        let mailer = self.mailer().await?;
        Self::deliver(&mailer, to, subject, body).await
    }

    async fn mailer(&self) -> Result<Mailer> {
        let db = self.get_smtp_settings().await.unwrap_or_default();
        let host = db.host.as_deref().or(self.config.smtp_host.as_deref());
        let user = db.user.as_deref().or(self.config.smtp_user.as_deref());
        let pass = db.pass.as_deref().or(self.config.smtp_pass.as_deref());
        let from = db.from.as_deref().or(self.config.smtp_from.as_deref());
        let port = db.port.or(self.config.smtp_port).unwrap_or(587);

        // Не `warn` и не `Ok(())`: письмо стало частью входа, и «почта не
        // настроена» — это «в дом не войти», а не мелкая неурядица.
        let (Some(host), Some(user), Some(pass), Some(from)) = (host, user, pass, from) else {
            return Err(AppError::Internal("SMTP is not configured".into()));
        };
        let from = from
            .parse()
            .map_err(|_| AppError::Internal("Invalid SMTP from address".into()))?;
        Ok(Mailer {
            transport: smtp_transport(host, port, user, pass)?,
            from,
        })
    }

    async fn deliver(mailer: &Mailer, to: &str, subject: &str, body: String) -> Result<()> {
        use lettre::message::header::ContentType;
        use lettre::{AsyncTransport, Message};

        let email = Message::builder()
            .from(mailer.from.clone())
            .to(to
                .parse()
                .map_err(|_| AppError::Internal("Invalid recipient address".into()))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .map_err(|e| AppError::Internal(format!("Email build error: {e}")))?;

        mailer
            .transport
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("SMTP send error: {e}")))?;
        Ok(())
    }

    /// Записка в Telegram о новой двери.
    ///
    /// Сессия живёт месяц, и узнать о чужом входе иначе неоткуда: журнал
    /// попыток видит только хозяин дома. Пишется она лишь тому, у кого бот уже
    /// есть, — заводить ради этого почтовую рассылку дом не станет, — и
    /// вдогонку: ни одна дверь не должна ждать, пока Telegram ответит.
    fn tell_of_new_door(&self, user: &User, ctx: &crate::models::ClientContext) {
        let (Some(chat), Some(bot)) = (user.telegram_id, self.login_bot()) else {
            return;
        };
        let where_from = {
            let mut parts: Vec<String> = Vec::new();
            if let Some(b) = ctx
                .user_agent
                .as_deref()
                .map(browser_name)
                .filter(|b| !b.is_empty())
            {
                parts.push(b);
            }
            let place: Vec<&str> = [ctx.city.as_deref(), ctx.country_code.as_deref()]
                .into_iter()
                .flatten()
                .filter(|s| !s.trim().is_empty())
                .collect();
            if !place.is_empty() {
                parts.push(place.join(", "));
            }
            parts.join(" · ")
        };
        let svc = self.clone();
        let user_id = user.id;
        tokio::spawn(async move {
            let lang = svc
                .repo
                .user_lang(user_id)
                .await
                .ok()
                .flatten()
                .and_then(|l| crate::lang::Lang::parse(&l))
                .unwrap_or_default();
            let note = new_door_note(lang, &where_from);
            bot.say(chat, &note).await;
        });
    }

    // === COMMENTS ===

    /// Гость ограничен по адресу (5 в час), вошедший — по имени (30 в час):
    /// имя заводится бесплатно, и без предела вошедший заливал бы очередь модерации.
    async fn check_comment_rate_limit(&self, user: Option<&User>, ip: &str) -> Result<()> {
        let (key, max_per_hour) = match user {
            Some(u) => (format!("comment|user:{}", u.id), 30),
            None => (format!("comment|{ip}"), 5),
        };
        if self.repo.rate_limit_take(&key, max_per_hour, 3600).await? {
            Ok(())
        } else {
            Err(AppError::BadRequest(
                "Too many comments from this address. Please wait before submitting again.".into(),
            ))
        }
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
            return Err(AppError::BadRequest(
                "Home layout config is too large".into(),
            ));
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
            return Err(AppError::BadRequest(
                "Home layout presets payload is too large".into(),
            ));
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

    // === HEADER DECOR ===

    pub async fn get_header_decor(&self) -> Result<crate::models::HeaderDecor> {
        parse_json_setting("header_decor", self.repo.get_setting("header_decor").await?)
    }

    pub async fn save_header_decor(
        &self,
        decor: crate::models::HeaderDecor,
    ) -> Result<crate::models::HeaderDecor> {
        fn side(
            side: crate::models::HeaderDecorSide,
        ) -> Result<crate::models::HeaderDecorSide> {
            let image = crate::battles::check_background(&side.image)
                .map_err(|e| AppError::BadRequest(e.replace("The field background", "The header picture")))?;
            Ok(crate::models::HeaderDecorSide {
                image,
                width: side.width.map(|w| w.clamp(120, 640)),
                fade: side.fade.map(|f| f.min(100)),
            })
        }
        let clean = crate::models::HeaderDecor {
            left: side(decor.left)?,
            right: side(decor.right)?,
        };
        let json = serde_json::to_string(&clean).map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("header_decor", &json).await?;
        Ok(clean)
    }

    // === HEADER PORTRAIT ===

    pub async fn get_header_portrait(&self) -> Result<crate::models::HeaderPortrait> {
        parse_json_setting(
            "header_portrait",
            self.repo.get_setting("header_portrait").await?,
        )
    }

    /// Public view: only the finished picture the header shows.
    pub async fn get_header_portrait_shown(&self) -> Result<crate::models::HeaderPortraitShown> {
        let all = self.get_header_portrait().await?;
        let shown = all
            .active_id
            .as_deref()
            .and_then(|id| all.assets.iter().find(|a| a.id == id));
        Ok(match shown {
            Some(a) => crate::models::HeaderPortraitShown {
                image: Some(a.image.clone()),
                width: a.width,
                height: a.height,
                edge: all.edge.clone(),
            },
            None => crate::models::HeaderPortraitShown::default(),
        })
    }

    pub async fn save_header_portrait(
        &self,
        body: crate::models::HeaderPortrait,
    ) -> Result<crate::models::HeaderPortrait> {
        use crate::models::*;
        const PICTURES_MAX: usize = 60;

        fn id_ok(id: &str) -> bool {
            !id.is_empty()
                && id.len() <= 64
                && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        }
        fn name(raw: &str) -> String {
            raw.trim().chars().take(80).collect()
        }
        fn path(raw: &str) -> Result<String> {
            crate::battles::check_background(&Some(raw.to_string()))
                .map_err(|e| {
                    AppError::BadRequest(e.replace("The field background", "The header picture"))
                })?
                .ok_or_else(|| AppError::BadRequest("The header picture has no file".into()))
        }
        fn px(v: u32) -> u32 {
            v.clamp(1, 4000)
        }
        fn frac(v: f32, lo: f32, hi: f32) -> f32 {
            if v.is_finite() { v.clamp(lo, hi) } else { lo }
        }
        fn pictures(list: Vec<HeaderPortraitPicture>) -> Result<Vec<HeaderPortraitPicture>> {
            let mut seen = std::collections::HashSet::new();
            let mut out = Vec::new();
            for p in list.into_iter().take(PICTURES_MAX) {
                if !id_ok(&p.id) || !seen.insert(p.id.clone()) {
                    continue;
                }
                out.push(HeaderPortraitPicture {
                    name: name(&p.name),
                    image: path(&p.image)?,
                    width: px(p.width),
                    height: px(p.height),
                    id: p.id,
                });
            }
            Ok(out)
        }

        let compositions = pictures(body.compositions)?;
        let photos = pictures(body.photos)?;

        let mut seen = std::collections::HashSet::new();
        let mut assets = Vec::new();
        for a in body.assets.into_iter().take(PICTURES_MAX) {
            if !id_ok(&a.id) || !seen.insert(a.id.clone()) {
                continue;
            }
            // A link to a picture that is no longer in the library is dropped:
            // the finished picture still stands, it just cannot be reopened.
            let composition_id = a
                .composition_id
                .filter(|id| compositions.iter().any(|c| &c.id == id));
            let photo_id = a.photo_id.filter(|id| photos.iter().any(|p| &p.id == id));
            let place = a.place.map(|p| HeaderPortraitPlace {
                x: frac(p.x, -1.0, 2.0),
                y: frac(p.y, -1.0, 2.0),
                size: frac(p.size, 0.02, 4.0),
                flip: p.flip,
                fade_sides: p.fade_sides.min(50),
                fade_bottom: p.fade_bottom.min(100),
                shadow: p.shadow.min(100),
            });
            let crop = a.crop.map(|c| {
                let x = frac(c.x, 0.0, 0.99);
                let y = frac(c.y, 0.0, 0.99);
                HeaderPortraitCrop {
                    x,
                    y,
                    w: frac(c.w, 0.01, 1.0 - x),
                    h: frac(c.h, 0.01, 1.0 - y),
                }
            });
            assets.push(HeaderPortraitAsset {
                name: name(&a.name),
                image: path(&a.image)?,
                width: px(a.width),
                height: px(a.height),
                composition_id,
                photo_id,
                place,
                crop,
                id: a.id,
            });
        }
        let active_id = body
            .active_id
            .filter(|id| assets.iter().any(|a| &a.id == id));

        let clean = HeaderPortrait {
            compositions,
            photos,
            assets,
            active_id,
            edge: HeaderPortraitEdge {
                gap_left: body.edge.gap_left.min(200),
                gap_right: body.edge.gap_right.min(200),
                fade_left: body.edge.fade_left.min(50),
                fade_right: body.edge.fade_right.min(50),
            },
        };
        let json = serde_json::to_string(&clean).map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("header_portrait", &json).await?;
        Ok(clean)
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
            return Err(AppError::BadRequest(
                "Reel theme presets payload is too large".into(),
            ));
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
            return Err(AppError::BadRequest(
                "Display presets payload is too large".into(),
            ));
        }
        self.repo
            .upsert_setting("display_config_presets", &json)
            .await
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
        if !req.age_confirmed {
            return Err(AppError::BadRequest(
                "Please confirm you are 16 or older".to_string(),
            ));
        }
        let uuid = self.resolve_public_figurine_uuid(&figurine_id).await?;
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
        self.note_lang("figurine_waitlist", entry.id, user_id, req.lang.as_deref()).await;
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
        let mut notified = 0u64;
        for entry in &entries {
            if let Some(uid) = entry.user_id {
                let lang = self.lang_for("figurine_waitlist", entry.id).await;
                let name = self.figurine_name_in(uuid, &figurine_name, lang).await;
                let (subject, body) = waitlist_words(lang, &name);
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
                let _ = svc.send_sketch_watch_letters(uuid, &figurine_name).await;
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

    /// Ask to sign the visitor book. Double opt-in: the address becomes a
    /// subscriber only when its owner follows the link from the confirmation
    /// letter (`confirm_subscription`); the welcome letter goes after that.
    pub async fn subscribe(
        &self,
        req: crate::models::CreateSubscriptionRequest,
        ip: Option<String>,
    ) -> Result<crate::models::SubscriptionCreatedResponse> {
        if !req.age_confirmed {
            return Err(AppError::BadRequest(
                "Please confirm you are 16 or older".to_string(),
            ));
        }
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
            age_confirmed: req.age_confirmed,
        };
        // Ключ уходит в письмо, в базе лежит только его отпечаток.
        let key = secret_key();
        let outcome = self
            .repo
            .request_subscription(&normalized, ip.as_deref(), &fingerprint(&key))
            .await?;
        // Ответ один и тот же для нового адреса, для действующего подписчика
        // и для адреса, которому письмо уже уходило: иначе форма отвечала бы на
        // вопрос «состоит ли такой-то в книге».
        let (token, to_send) = match outcome {
            // Настоящий ключ отказа получает только тот, кто завёл строку.
            crate::db::SubscribeOutcome::Created(sub) => (sub.unsubscribe_token.clone(), Some(sub)),
            // Строка старая: письмо уходит владельцу ящика, а запросивший
            // может быть кем угодно и ключа не получает.
            crate::db::SubscribeOutcome::Sent(sub) => (Self::generate_decoy_token(), Some(sub)),
            crate::db::SubscribeOutcome::Throttled(_)
            | crate::db::SubscribeOutcome::AlreadyActive(_) => (Self::generate_decoy_token(), None),
        };
        if let Some(sub) = to_send {
            self.observability
                .record_business_event("newsletter_subscribe_requested", "ok");
            Self::log_domain_event("newsletter_subscribe_requested", "subscriber", sub.id, "ok");
            let svc = self.clone();
            tokio::spawn(async move {
                if let Err(e) = svc.send_subscription_confirmation(&sub, &key).await {
                    tracing::warn!("subscription confirmation letter not sent: {e}");
                }
            });
        }
        Ok(crate::models::SubscriptionCreatedResponse {
            unsubscribe_token: token,
            already_subscribed: false,
        })
    }

    /// Ключ-пустышка того же вида, что и настоящий отказ, который ни к чему не
    /// ведёт.
    fn generate_decoy_token() -> String {
        let raw = Uuid::new_v4().simple().to_string().to_uppercase();
        format!("{}-{}-{}-{}", &raw[..4], &raw[4..8], &raw[8..12], &raw[12..16])
    }

    /// Ссылка из письма. Подтверждает, что ящиком владеет тот, кто его вписал;
    /// первое письмо дома приходит только после этого.
    pub async fn confirm_subscription(&self, key: &str) -> Result<bool> {
        match self.repo.confirm_subscription(&fingerprint(key.trim())).await? {
            crate::db::ConfirmOutcome::Confirmed(sub) => {
                self.observability
                    .record_business_event("newsletter_subscribed", "ok");
                Self::log_domain_event("newsletter_subscribed", "subscriber", sub.id, "ok");
                let svc = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = svc.send_welcome_email(&sub).await {
                        tracing::warn!("welcome letter not sent: {e}");
                    }
                });
                Ok(true)
            }
            crate::db::ConfirmOutcome::AlreadyActive(_) => Ok(true),
            crate::db::ConfirmOutcome::Unknown => Ok(false),
        }
    }

    /// Письмо со ссылкой подтверждения. Ничего, кроме просьбы: ни новостей, ни
    /// имён — кто вписал адрес, дом не знает и не говорит.
    async fn send_subscription_confirmation(
        &self,
        sub: &crate::models::Subscriber,
        key: &str,
    ) -> Result<()> {
        let base = self.config.public_url.trim_end_matches('/');
        let link = format!("{base}/subscribe/confirm/{key}");
        use crate::lang::Lang;
        let (subject, body) = match Lang::parse(&sub.lang).unwrap_or_default() {
            Lang::Ru => (
                "Подтвердите адрес для книги дома",
                format!(
                    "Этот адрес вписали в книгу дома. Если это были вы, откройте ссылку — \
                     и письма из мастерской начнут приходить сюда:\n{link}\n\n\
                     Если вы ничего не вписывали, просто не отвечайте на это письмо: \
                     без вашего перехода по ссылке не придёт ничего. Ссылка живёт неделю."
                ),
            ),
            Lang::En => (
                "Confirm your address for the house book",
                format!(
                    "This address was written into the house book. If that was you, open \
                     the link and letters from the workshop will begin to arrive here:\n{link}\n\n\
                     If you wrote nothing, simply ignore this letter: without your follow of \
                     the link nothing will be sent. The link lives a week."
                ),
            ),
            Lang::Uk => (
                "Підтвердьте адресу для книги дому",
                format!(
                    "Цю адресу вписали до книги дому. Якщо це були ви, відкрийте посилання — \
                     і листи з майстерні почнуть надходити сюди:\n{link}\n\n\
                     Якщо ви нічого не вписували, просто не відповідайте на цей лист: \
                     без вашого переходу за посиланням не прийде нічого. Посилання діє тиждень."
                ),
            ),
            Lang::Es => (
                "Confirme su dirección para la lista de correo",
                format!(
                    "Alguien ha escrito esta dirección en la lista de correo del sitio. Si fue usted, \
                     abra el enlace y las cartas del taller empezarán a llegar aquí:\n{link}\n\n\
                     Si no escribió nada, ignore esta carta: sin abrir el enlace no se enviará nada. \
                     El enlace vale una semana."
                ),
            ),
            Lang::De => (
                "Bestätigen Sie Ihre Adresse für die Mailingliste",
                format!(
                    "Diese Adresse wurde in die Mailingliste der Website eingetragen. Wenn Sie das waren, \
                     öffnen Sie den Link, und Briefe aus der Werkstatt kommen künftig hierher:\n{link}\n\n\
                     Wenn Sie nichts eingetragen haben, ignorieren Sie diesen Brief einfach: Ohne den Link \
                     zu öffnen, wird nichts gesendet. Der Link gilt eine Woche."
                ),
            ),
            Lang::It => (
                "Confermi il suo indirizzo per la lista di posta",
                format!(
                    "Questo indirizzo è stato inserito nella lista di posta del sito. Se è stato lei, \
                     apra il link e le lettere del laboratorio cominceranno ad arrivare qui:\n{link}\n\n\
                     Se non ha inserito nulla, ignori questa lettera: senza aprire il link non verrà \
                     inviato niente. Il link vale una settimana."
                ),
            ),
        };
        self.say_or_send(&sub.email, subject, body, &link).await
    }

    /// View a subscription by its unsubscribe token (the unsubscribe page).
    pub async fn get_subscriber_by_token(
        &self,
        token: &str,
    ) -> Result<Option<crate::models::SubscriberInfo>> {
        if let Some(s) = self.repo.get_subscriber_by_token(token).await? {
            return Ok(Some(crate::models::SubscriberInfo { email: s.email }));
        }
        // Подписчик книги в Telegram: вместо адреса — его имя в Telegram.
        Ok(self
            .repo
            .get_telegram_subscriber_by_token(token)
            .await?
            .map(|s| crate::models::SubscriberInfo {
                email: match s.username {
                    Some(name) => format!("Telegram @{name}"),
                    None => "Telegram".to_string(),
                },
            }))
    }

    /// Leave the book by token. Idempotent.
    pub async fn unsubscribe_by_token(&self, token: &str) -> Result<()> {
        match self.repo.unsubscribe_by_token(token).await? {
            Some(s) => Self::log_domain_event("newsletter_unsubscribed", "subscriber", s.id, "ok"),
            None => {
                if self.repo.unsubscribe_telegram_by_token(token).await? {
                    Self::log_domain_event("newsletter_unsubscribed", "subscriber", "telegram", "ok");
                } else {
                    Self::log_domain_event("newsletter_unsubscribed", "subscriber", "unknown", "noop")
                }
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
        let msg = self
            .repo
            .create_contact_message(&normalized, ip.as_deref())
            .await?;
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

    pub async fn list_contact_messages_admin(
        &self,
    ) -> Result<Vec<crate::models::ContactMessageDto>> {
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
        let base = self.config.public_url.trim_end_matches('/');
        let unsub = format!("{}/unsubscribe/{}", base, sub.unsubscribe_token);
        use crate::lang::Lang;
        let lang = Lang::parse(&sub.lang).unwrap_or_default();
        // Вписавшийся после последней строки байки просил одного — следующую
        // байку, и первое письмо говорит об этом, а не о показах.
        let after_tale = sub.source == "tale";
        let subject = match (lang, after_tale) {
            (Lang::Ru, true) => "Следующая небылица придёт письмом",
            (Lang::En, true) => "The next tall tale will come as a letter",
            (Lang::Uk, true) => "Наступна небилиця прийде листом",
            (Lang::Ru, false) => "Ваше имя вписано в книгу дома",
            (Lang::En, false) => "Your name is in the house book",
            (Lang::Uk, false) => "Ваше ім’я вписане до книги дому",
            (Lang::Es, true) => "El próximo cuento le llegará por correo",
            (Lang::De, true) => "Die nächste Geschichte kommt per E-Mail",
            (Lang::It, true) => "Il prossimo racconto le arriverà per posta",
            (Lang::Es, false) => "Está en la lista de correo",
            (Lang::De, false) => "Sie stehen in der Mailingliste",
            (Lang::It, false) => "È nella lista di posta",
        };
        let body_text = match (lang, after_tale) {
            (Lang::Ru, true) => format!(
                "Дом запомнил вас.\n\n\
                 Когда на полке появится новая небылица, она придёт сюда — письмом, без шума \
                 и спешки. Изредка вместе с ней — вести из мастерской.\n\n\
                 Если захотите, чтобы дом забыл ваше имя, эта дверь всегда открыта:\n{unsub}",
            ),
            (Lang::En, true) => format!(
                "The house has remembered you.\n\n\
                 When a new tall tale is set on the shelf, it will come here as a letter — no \
                 noise, no haste. Now and then, word from the workshop comes with it.\n\n\
                 Should you ever wish the house to forget your name, this door stays open:\n{unsub}",
            ),
            (Lang::Uk, true) => format!(
                "Дім запам’ятав вас.\n\n\
                 Коли на полиці з’явиться нова небилиця, вона прийде сюди — листом, без галасу \
                 й поспіху. Зрідка разом із нею — вісті з майстерні.\n\n\
                 Якщо забажаєте, щоб дім забув ваше ім’я, ці двері завжди відчинені:\n{unsub}",
            ),
            (Lang::Ru, false) => format!(
                "Дом запомнил вас.\n\n\
                 Теперь вести из мастерской — новые работы, новые небылицы, открытие показов — \
                 будут находить вас первыми. Без шума и спешки.\n\n\
                 Если захотите, чтобы дом забыл ваше имя, эта дверь всегда открыта:\n{unsub}",
            ),
            (Lang::En, false) => format!(
                "The house has remembered you.\n\n\
                 Letters from the workshop — new works, new tall tales, the opening of \
                 showings — will now reach you first. No noise, no haste.\n\n\
                 Should you ever wish the house to forget your name, this door stays open:\n{unsub}",
            ),
            (Lang::Uk, false) => format!(
                "Дім запам’ятав вас.\n\n\
                 Тепер вісті з майстерні — нові роботи, нові небилиці, відчинення показів — \
                 знаходитимуть вас першими. Без галасу й поспіху.\n\n\
                 Якщо забажаєте, щоб дім забув ваше ім’я, ці двері завжди відчинені:\n{unsub}",
            ),
            (Lang::Es, true) => format!(
                "Su dirección está guardada.\n\n\
                 Cuando aparezca un cuento nuevo en el sitio, le llegará aquí por correo. \
                 De vez en cuando, con él, noticias del taller.\n\n\
                 Para salir de la lista en cualquier momento:\n{unsub}",
            ),
            (Lang::De, true) => format!(
                "Ihre Adresse ist gespeichert.\n\n\
                 Wenn auf der Website eine neue Geschichte erscheint, kommt sie per E-Mail hierher. \
                 Ab und zu, zusammen mit ihr, Neuigkeiten aus der Werkstatt.\n\n\
                 Austragen können Sie sich jederzeit hier:\n{unsub}",
            ),
            (Lang::It, true) => format!(
                "Il suo indirizzo è salvato.\n\n\
                 Quando sul sito comparirà un nuovo racconto, le arriverà qui per posta. \
                 Ogni tanto, insieme a esso, notizie dal laboratorio.\n\n\
                 Per uscire dalla lista in qualsiasi momento:\n{unsub}",
            ),
            (Lang::Es, false) => format!(
                "Su dirección está guardada.\n\n\
                 Las noticias del taller (obras nuevas, cuentos nuevos, nuevas exposiciones) \
                 le llegarán a usted primero. Sin prisas y sin ruido.\n\n\
                 Para salir de la lista en cualquier momento:\n{unsub}",
            ),
            (Lang::De, false) => format!(
                "Ihre Adresse ist gespeichert.\n\n\
                 Neuigkeiten aus der Werkstatt (neue Arbeiten, neue Geschichten, neue Vorführungen) \
                 erreichen Sie jetzt zuerst. Ohne Eile und ohne Lärm.\n\n\
                 Austragen können Sie sich jederzeit hier:\n{unsub}",
            ),
            (Lang::It, false) => format!(
                "Il suo indirizzo è salvato.\n\n\
                 Le notizie dal laboratorio (opere nuove, racconti nuovi, nuove esposizioni) \
                 arriveranno prima a lei. Senza fretta e senza rumore.\n\n\
                 Per uscire dalla lista in qualsiasi momento:\n{unsub}",
            ),
        };

        self.send_mail(&sub.email, subject, body_text).await
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
        let watches = self
            .repo
            .list_gazette_watches_for_figurine(figurine_id)
            .await
            .unwrap_or_default();
        if queue.is_empty() && notify.is_empty() && watches.is_empty() {
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
        if !watches.is_empty() {
            let seen: HashSet<String> = notify
                .iter()
                .map(|o| o.requester_email.to_ascii_lowercase())
                .collect();
            let extra: Vec<&crate::models::GazetteWatch> = watches
                .iter()
                .filter(|w| !seen.contains(&w.email.to_ascii_lowercase()))
                .collect();
            if !extra.is_empty() {
                let lines: Vec<String> = extra
                    .iter()
                    .map(|w| {
                        fmt_contact(
                            w.name.as_deref().unwrap_or("—"),
                            &w.email,
                            None,
                            Some("вестник"),
                        )
                    })
                    .collect();
                sections.push(format!(
                    "📜 *Сторожка вестника* \\({}\\)\n{}",
                    extra.len(),
                    lines.join("\n")
                ));
            }
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

    async fn send_sketch_watch_letters(
        &self,
        figurine_id: Uuid,
        figurine_name: &str,
    ) -> Result<()> {
        let pending = self
            .repo
            .list_unnotified_gazette_watches_for_figurine(figurine_id)
            .await?;
        if pending.is_empty() {
            return Ok(());
        }
        let handle = match self.repo.get_figurine_by_id(figurine_id).await? {
            Some(f) => f
                .slug
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| figurine_id.to_string()),
            None => figurine_id.to_string(),
        };
        let mut sent: Vec<String> = Vec::new();
        let mut seen = HashSet::new();
        for watch in pending {
            let key = watch.email.to_ascii_lowercase();
            if !seen.insert(key) {
                continue;
            }
            if self
                .send_sketch_laid_email(&watch, figurine_id, figurine_name, &handle)
                .await
                .is_ok()
            {
                sent.push(watch.email);
            }
        }
        self.repo
            .mark_gazette_watches_notified_for_figurine(figurine_id, &sent)
            .await?;
        Ok(())
    }

    async fn send_sketch_laid_email(
        &self,
        watch: &crate::models::GazetteWatch,
        figurine_id: Uuid,
        figurine_name: &str,
        figurine_handle: &str,
    ) -> Result<()> {
        // Имя работы — на языке письма, без перевода — английское.
        let lang = crate::lang::Lang::parse(&watch.lang).unwrap_or_default();
        let figurine_name = self.figurine_name_in(figurine_id, figurine_name, lang).await;
        let figurine_name = figurine_name.as_str();
        use lettre::message::header::ContentType;
        use lettre::{AsyncTransport, Message};

        let db = self.get_smtp_settings().await.unwrap_or_default();
        let host = db.host.as_deref().or(self.config.smtp_host.as_deref());
        let user = db.user.as_deref().or(self.config.smtp_user.as_deref());
        let pass = db.pass.as_deref().or(self.config.smtp_pass.as_deref());
        let from = db.from.as_deref().or(self.config.smtp_from.as_deref());
        let port = db.port.or(self.config.smtp_port).unwrap_or(587);

        let (Some(host), Some(user), Some(pass), Some(from)) = (host, user, pass, from) else {
            tracing::warn!(
                "SMTP not configured — sketch letter not sent to {}",
                watch.email
            );
            return Ok(());
        };

        let base = self.config.public_url.trim_end_matches('/');
        let work = format!("{base}/figurines/{figurine_handle}");
        let leave = format!("{base}/gazette/watch/{}", watch.cancel_token);
        use crate::lang::Lang;
        let (subject, body_text) = match Lang::parse(&watch.lang).unwrap_or_default() {
            Lang::Ru => (
                format!("«{figurine_name}» уже на сайте"),
                format!(
                    "Работа готова и лежит на сайте.\n\n\
                     «{figurine_name}»:\n{work}\n\n\
                     Если больше не хотите писем об этой работе:\n{leave}"
                ),
            ),
            Lang::En => (
                format!("{figurine_name} is ready"),
                format!(
                    "This work is ready and on the site.\n\n\
                     {figurine_name}:\n{work}\n\n\
                     If you no longer want letters about this work:\n{leave}"
                ),
            ),
            Lang::Uk => (
                format!("«{figurine_name}» уже на сайті"),
                format!(
                    "Робота готова й лежить на сайті.\n\n\
                     «{figurine_name}»:\n{work}\n\n\
                     Якщо більше не хочете листів про цю роботу:\n{leave}"
                ),
            ),
            Lang::Es => (
                format!("«{figurine_name}» ya está en el sitio"),
                format!(
                    "La obra está terminada y ya está en el sitio.\n\n\
                     «{figurine_name}»:\n{work}\n\n\
                     Si ya no quiere cartas sobre esta obra:\n{leave}"
                ),
            ),
            Lang::De => (
                format!("„{figurine_name}“ ist jetzt auf der Website"),
                format!(
                    "Die Arbeit ist fertig und auf der Website zu sehen.\n\n\
                     „{figurine_name}“:\n{work}\n\n\
                     Wenn Sie keine Briefe mehr zu dieser Arbeit möchten:\n{leave}"
                ),
            ),
            Lang::It => (
                format!("«{figurine_name}» è ora sul sito"),
                format!(
                    "L’opera è finita ed è sul sito.\n\n\
                     «{figurine_name}»:\n{work}\n\n\
                     Se non desidera più lettere su quest’opera:\n{leave}"
                ),
            ),
        };

        let email = Message::builder()
            .from(
                from.parse()
                    .map_err(|_| AppError::Internal("Invalid SMTP from address".into()))?,
            )
            .to(watch
                .email
                .parse()
                .map_err(|_| AppError::Internal("Invalid recipient address".into()))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body_text)
            .map_err(|e| AppError::Internal(format!("Email build error: {e}")))?;

        let mailer = smtp_transport(host, port, user, pass)?;

        mailer
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("SMTP send error: {e}")))?;
        Ok(())
    }

    async fn send_reply_email(
        &self,
        to: &str,
        figurine_name: &str,
        figurine_id: &str,
        comment_body: &str,
        reply: &str,
        lang: crate::lang::Lang,
    ) -> Result<()> {
        use lettre::message::header::ContentType;
        use lettre::{AsyncTransport, Message};

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
        let (subject, yours, answer, view) = impression_reply_words(lang, figurine_name);
        let body_text = format!(
            "{yours}\n\n\
            {comment_body}\n\n\
            — — —\n\n\
            {answer}\n\n\
            {reply}\n\n\
            {view}: {figurine_url}",
        );

        let email = Message::builder()
            .from(
                from.parse()
                    .map_err(|_| AppError::Internal("Invalid SMTP from address".into()))?,
            )
            .to(to
                .parse()
                .map_err(|_| AppError::Internal("Invalid recipient address".into()))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body_text)
            .map_err(|e| AppError::Internal(format!("Email build error: {e}")))?;

        let mailer = smtp_transport(host, port, user, pass)?;

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
        self.check_comment_rate_limit(user, ip).await?;

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

        let comment = self
            .repo
            .insert_comment(
                figurine_id,
                user_id,
                &author_name,
                author_email.as_deref(),
                body,
            )
            .await?;
        self.note_lang("figurine_comments", comment.id, user_id, req.lang.as_deref()).await;
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
            let lang = self.lang_for("figurine_comments", prev.id).await;
            let name = self.figurine_name_in(prev.figurine_id, &figurine_name, lang).await;
            let _ = self
                .send_reply_email(
                    email,
                    &name,
                    &fid,
                    &prev.body,
                    admin_reply.unwrap_or(""),
                    lang,
                )
                .await;
        }

        // Одобренный отзыв закрывает задание «Оставить отзыв о работе», и
        // закрыть его надо ЗДЕСЬ. Задания досчитываются на действиях самого
        // игрока, а одобрение — действие автора: без этого награда ждала бы
        // следующего захода игрока на страницу игры, то есть выглядела бы как
        // «отзыв оставил, а ничего не изменилось».
        if is_approved && let Some(author) = prev.user_id {
            self.settle_errands_quietly(author).await;
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

    // === БАЙКИ: ОТКЛИК ЧИТАТЕЛЯ ===

    /// Жетон посетителя — то же, чем читатель назван у работ: строка из
    /// хранилища браузера, не адрес и не имя.
    fn tale_visitor_token(token: &str) -> Result<&str> {
        let token = token.trim();
        if token.is_empty() || token.len() > 64 {
            return Err(AppError::BadRequest("Invalid visitor token".into()));
        }
        Ok(token)
    }

    async fn tale_or_404(&self, tale_id: Uuid) -> Result<()> {
        if !self.repo.tale_is_live(tale_id).await? {
            return Err(AppError::NotFound(format!("Tale {tale_id} not found")));
        }
        Ok(())
    }

    /// Правила приёма те же, что у отклика на работу: без имени гость не
    /// пишет, тысяча знаков потолок, пять штук в час с адреса. Второй набор
    /// правил разошёлся бы с первым на первой же правке.
    pub async fn submit_tale_comment(
        &self,
        tale_id: Uuid,
        user: Option<&User>,
        req: &SubmitCommentRequest,
        ip: &str,
    ) -> Result<()> {
        self.tale_or_404(tale_id).await?;
        self.check_comment_rate_limit(user, ip).await?;

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
            .insert_tale_comment(
                tale_id,
                user_id,
                &author_name,
                author_email.as_deref(),
                body,
            )
            .await?;
        Self::log_domain_event("tale_comment_submitted", "tale", tale_id, "ok");

        let tale_title = self
            .repo
            .get_gazette_leaf_admin(tale_id)
            .await?
            .map(|l| l.title_ru)
            .unwrap_or_default();
        {
            let svc = self.clone();
            let author_name = author_name.clone();
            let body = body.to_string();
            tokio::spawn(async move {
                svc.send_comment_telegram_notification(&tale_title, &author_name, &body)
                    .await;
            });
        }
        Ok(())
    }

    pub async fn get_tale_comments(
        &self,
        tale_id: Uuid,
        newest_first: bool,
    ) -> Result<Vec<CommentDto>> {
        Ok(self
            .repo
            .get_approved_tale_comments(tale_id, newest_first)
            .await?
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

    pub async fn admin_list_tale_comments(
        &self,
        only_pending: bool,
        tale_filter: Option<Uuid>,
        newest_first: bool,
        page: i64,
        per_page: i64,
    ) -> Result<crate::models::AdminTaleCommentsPage> {
        let page = page.max(1);
        let per_page = per_page.clamp(1, 100);
        let (rows, total, pending_count) = self
            .repo
            .get_tale_comments_admin_page(only_pending, tale_filter, newest_first, page, per_page)
            .await?;
        Ok(crate::models::AdminTaleCommentsPage {
            items: rows
                .into_iter()
                .map(|c| crate::models::AdminTaleCommentDto {
                    id: c.id.to_string(),
                    tale_id: c.tale_id.to_string(),
                    tale_title_en: c.tale_title_en,
                    tale_title_ru: c.tale_title_ru,
                    tale_title_uk: c.tale_title_uk,
                    tale_title_es: c.tale_title_es,
                    tale_title_de: c.tale_title_de,
                    tale_title_it: c.tale_title_it,
                    tale_slug: c.tale_slug,
                    author_name: c.author_name,
                    author_email: c.author_email,
                    body: c.body,
                    is_approved: c.is_approved,
                    admin_reply: c.admin_reply,
                    created_at: c.created_at.to_rfc3339(),
                    user_id: c.user_id.map(|u| u.to_string()),
                })
                .collect(),
            total,
            pending_count,
            page,
            per_page,
        })
    }

    pub async fn admin_moderate_tale_comment(
        &self,
        id: Uuid,
        is_approved: bool,
        admin_reply: Option<&str>,
    ) -> Result<()> {
        let reply = admin_reply.map(str::trim).filter(|s| !s.is_empty());
        self.repo.moderate_tale_comment(id, is_approved, reply).await?;
        Self::log_domain_event("tale_comment_moderated", "tale_comment", id, "ok");
        Ok(())
    }

    pub async fn admin_delete_tale_comment(&self, id: Uuid) -> Result<()> {
        self.repo.delete_tale_comment(id).await?;
        Self::log_domain_event("tale_comment_deleted", "tale_comment", id, "ok");
        Ok(())
    }

    /// Голос читателя. Значение назначается, а не переключается: удвоенный
    /// запрос не снимает то, что сам же поставил.
    pub async fn set_tale_vote(
        &self,
        tale_id: Uuid,
        visitor_token: &str,
        user_id: Option<Uuid>,
        value: i16,
    ) -> Result<(i16, i64, i64)> {
        self.tale_or_404(tale_id).await?;
        let token = Self::tale_visitor_token(visitor_token)?;
        if !(-1..=1).contains(&value) {
            return Err(AppError::BadRequest("Vote must be -1, 0 or 1".into()));
        }
        let out = self.repo.set_tale_vote(tale_id, token, user_id, value).await?;
        Self::log_domain_event("tale_voted", "tale", tale_id, "ok");
        Ok(out)
    }

    pub async fn record_tale_view(&self, tale_id: Uuid, visitor_token: &str) -> Result<()> {
        self.tale_or_404(tale_id).await?;
        let token = Self::tale_visitor_token(visitor_token)?;
        self.repo.record_tale_view(tale_id, token).await
    }

    /// Числа под байкой. Минус читателю не отдаётся — его печатает только стол.
    pub async fn get_tale_stats(
        &self,
        tale_id: Uuid,
        visitor_token: Option<&str>,
        with_dislikes: bool,
    ) -> Result<crate::models::TaleStatsDto> {
        let token = visitor_token
            .map(str::trim)
            .filter(|t| !t.is_empty() && t.len() <= 64);
        let (views, likes, dislikes, comments, mine) =
            self.repo.tale_stats(tale_id, token).await?;
        let (sequel, sequel_of, wish) = self.repo.tale_sequel_bits(tale_id, token).await?;
        Ok(crate::models::TaleStatsDto {
            views,
            likes,
            dislikes: with_dislikes.then_some(dislikes),
            comments,
            my_vote: mine,
            wants_sequel: wish.map(|w| w.is_some()),
            sequel_letter: wish.map(|w| w.is_some_and(|(letter, _)| letter)),
            sequel_telegram: wish.map(|w| w.is_some_and(|(_, note)| note)),
            sequel,
            sequel_of,
        })
    }

    pub async fn admin_tale_stats(&self) -> Result<Vec<crate::models::AdminTaleStatRow>> {
        self.repo.tale_stats_all().await
    }

    // === БАЙКИ: ЧИТАТЕЛЬ ПОСЛЕ ПОСЛЕДНЕЙ СТРОКИ ===

    /// Двери, через которые следующая байка приходит сама.
    pub async fn tale_doors(&self) -> crate::models::TaleDoorsDto {
        crate::models::TaleDoorsDto {
            // Локально письмо печатается в журнал (закон `say_or_send`), поэтому
            // стенду есть чем «отправить» и без SMTP.
            letters: self.config.public_url_is_local() || self.mail_works().await,
            telegram: self
                .config
                .telegram_channel_id
                .as_deref()
                .and_then(crate::tales::channel_link),
            // Записку пишет бот входа; локально она, как и письмо, печатается
            // в журнал.
            telegram_notes: self.config.public_url_is_local() || self.login_bot().is_some(),
        }
    }

    /// Адрес для письма по просьбе читателя. Названный адрес годится только
    /// вместе с подтверждением возраста — то же правило, что у книги дома и у
    /// слежки за эскизом. Не названный — просьба без письма, и это законно.
    fn reader_letter_address(given: Option<&str>, age_confirmed: bool) -> Result<Option<String>> {
        let Some(raw) = given.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        let Some(email) = crate::tales::usable_email(Some(raw)) else {
            return Err(AppError::BadRequest("Valid email is required".into()));
        };
        if !age_confirmed {
            return Err(AppError::BadRequest(
                "Please confirm you are 16 or older".into(),
            ));
        }
        Ok(Some(email))
    }

    /// Письмо о байке уходит только на адрес, чей хозяин доказал ящик: либо это
    /// подтверждённая почта вошедшего, либо подтверждённый подписчик книги дома.
    /// Иначе форма была бы способом отправить письмо от имени дома любому
    /// постороннему адресу.
    async fn require_verified_letter_address(
        &self,
        email: &str,
        user: Option<&User>,
    ) -> Result<()> {
        if let Some(u) = user
            && u.email_confirmed()
            && u.email.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(email))
        {
            return Ok(());
        }
        if self.repo.is_confirmed_subscriber(email).await? {
            return Ok(());
        }
        Err(AppError::BadRequest("letter_address_unconfirmed".into()))
    }

    /// «Хочу продолжение». Просьба без адреса тоже просьба: стол рассказов
    /// видит, чего ждут, даже если письмо никому не нужно.
    pub async fn set_tale_sequel_wish(
        &self,
        tale_id: Uuid,
        req: &crate::models::TaleSequelWishRequest,
        user: Option<&User>,
    ) -> Result<crate::models::TaleSequelWishResponse> {
        self.tale_or_404(tale_id).await?;
        let token = Self::tale_visitor_token(&req.visitor_token)?;
        let email = if req.want {
            Self::reader_letter_address(req.email.as_deref(), req.age_confirmed)?
        } else {
            None
        };
        if let Some(addr) = email.as_deref() {
            self.require_verified_letter_address(addr, user).await?;
        }
        // Записка в Telegram — только вошедшему, у чьего имени Telegram
        // привязан: номер чата знает только имя, а гостю писать некуда.
        if req.want
            && req.telegram == Some(true)
            && user.and_then(|u| u.telegram_id).is_none()
        {
            return Err(AppError::BadRequest(
                "Telegram is not linked to this name".into(),
            ));
        }
        let lang = crate::tales::letter_lang(req.lang.as_deref());
        let (wants, letter, telegram) = self
            .repo
            .set_tale_sequel_wish(
                tale_id,
                token,
                user.map(|u| u.id),
                req.want,
                email.as_deref(),
                req.telegram,
                lang,
            )
            .await?;
        Self::log_domain_event("tale_sequel_wished", "tale", tale_id, "ok");
        Ok(crate::models::TaleSequelWishResponse {
            wants,
            letter,
            telegram,
        })
    }

    /// Кто ждёт продолжения этой байки — и дошло ли до них известие.
    pub async fn admin_sequel_wishes(
        &self,
        tale_id: Uuid,
    ) -> Result<Vec<crate::models::AdminSequelWishRow>> {
        self.repo
            .admin_sequel_wishes(tale_id, TALE_LETTER_MAX_ATTEMPTS)
            .await
    }

    /// Что уйдёт людям, если выложить байку сейчас.
    pub async fn admin_tale_letters_forecast(
        &self,
        tale_id: Uuid,
    ) -> Result<crate::models::TaleLettersForecast> {
        let (laid, letters, notes) = self.repo.tale_letters_forecast(tale_id).await?;
        Ok(crate::models::TaleLettersForecast {
            laid,
            letters,
            notes,
            mail: self.mail_works().await,
            grace_secs: TALE_LETTER_GRACE_SECS,
        })
    }

    /// Голосование, которое сейчас стоит на полке, глазами одного читателя.
    pub async fn current_tale_poll(
        &self,
        visitor_token: Option<&str>,
    ) -> Result<Option<crate::models::TalePollDto>> {
        let Some((id, state, winner)) = self.repo.current_tale_poll().await? else {
            return Ok(None);
        };
        let works: Vec<crate::models::TalePollWorkDto> = self
            .repo
            .tale_poll_works(id)
            .await?
            .into_iter()
            .map(|(fid, name, slug, image_url)| crate::models::TalePollWorkDto {
                figurine_id: fid.to_string(),
                name,
                slug,
                image_url,
            })
            .collect();
        let winner = winner.and_then(|w| {
            let w = w.to_string();
            works.iter().find(|work| work.figurine_id == w).cloned()
        });
        let token = visitor_token
            .map(str::trim)
            .filter(|t| !t.is_empty() && t.len() <= 64);
        let mine = match token {
            Some(t) => self.repo.tale_poll_choice(id, t).await?,
            None => None,
        };
        Ok(Some(crate::models::TalePollDto {
            id: id.to_string(),
            state,
            candidates: works,
            my_choice: mine.map(|(fid, _)| fid.to_string()),
            my_letter: mine.is_some_and(|(_, letter)| letter),
            winner,
        }))
    }

    /// Голос «о ком записать следующую». Пока голосование открыто, выбор
    /// меняется; когда закрыто — проголосовавший ещё может оставить адрес
    /// (выбор его уже не меняется), а новый голос не принимается.
    pub async fn vote_tale_poll(
        &self,
        poll_id: Uuid,
        req: &crate::models::TalePollVoteRequest,
        user: &User,
    ) -> Result<crate::models::TalePollVoteResponse> {
        let user_id = Some(user.id);
        let token = Self::tale_visitor_token(&req.visitor_token)?;
        if !self
            .repo
            .tale_poll_has_candidate(poll_id, req.figurine_id)
            .await?
        {
            return Err(AppError::NotFound("No such choice in this poll".into()));
        }
        let email = Self::reader_letter_address(req.email.as_deref(), req.age_confirmed)?;
        if let Some(addr) = email.as_deref() {
            self.require_verified_letter_address(addr, Some(user)).await?;
        }
        let lang = crate::tales::letter_lang(req.lang.as_deref());
        let voted = self
            .repo
            .vote_tale_poll(poll_id, token, user_id, req.figurine_id, email.as_deref(), lang)
            .await?;
        let (choice, letter) = match voted {
            Some(row) => row,
            None => match email.as_deref() {
                Some(addr) => self
                    .repo
                    .attach_tale_poll_letter(poll_id, token, addr, lang)
                    .await?
                    .ok_or_else(|| AppError::BadRequest("This poll is closed".into()))?,
                None => return Err(AppError::BadRequest("This poll is closed".into())),
            },
        };
        Self::log_domain_event("tale_poll_voted", "tale_poll", poll_id, "ok");
        Ok(crate::models::TalePollVoteResponse {
            my_choice: choice.to_string(),
            my_letter: letter,
        })
    }

    pub async fn admin_tale_polls(&self) -> Result<Vec<crate::models::AdminTalePollDto>> {
        let polls = self.repo.admin_tale_polls().await?;
        let ids: Vec<Uuid> = polls.iter().map(|p| p.0).collect();
        let mut by_poll: HashMap<Uuid, Vec<crate::models::AdminTalePollCandidateDto>> =
            HashMap::new();
        for (poll_id, fid, name, image_url, votes, letters) in
            self.repo.admin_tale_poll_candidates(&ids).await?
        {
            by_poll
                .entry(poll_id)
                .or_default()
                .push(crate::models::AdminTalePollCandidateDto {
                    figurine_id: fid.to_string(),
                    name,
                    image_url,
                    votes,
                    letters,
                });
        }
        let mut out = Vec::with_capacity(polls.len());
        for (id, state, opened_at, closed_at, winner, slug, title_en, title_ru, title_uk, title_es, title_de, title_it) in polls {
            let candidates = by_poll.remove(&id).unwrap_or_default();
            let letters = candidates.iter().map(|c| c.letters).sum();
            out.push(crate::models::AdminTalePollDto {
                id: id.to_string(),
                state,
                opened_at: opened_at.to_rfc3339(),
                closed_at: closed_at.map(|t| t.to_rfc3339()),
                winner_figurine_id: winner.map(|w| w.to_string()),
                fulfilled: match (slug, title_en, title_ru) {
                    (Some(slug), Some(title_en), Some(title_ru)) => {
                        Some(crate::models::GazetteNeighborDto {
                            slug,
                            title_en,
                            title_ru,
                            title_uk: title_uk.unwrap_or_default(),
                            title_es: title_es.unwrap_or_default(),
                            title_de: title_de.unwrap_or_default(),
                            title_it: title_it.unwrap_or_default(),
                        })
                    }
                    _ => None,
                },
                candidates,
                letters,
            });
        }
        Ok(out)
    }

    /// Открыть голосование. Кандидатов от двух до четырёх: из одного не
    /// выбирают, а пять лиц в ряд на телефоне уже не лица, а полоска.
    pub async fn admin_open_tale_poll(&self, figurine_ids: Vec<Uuid>) -> Result<()> {
        let mut seen = HashSet::new();
        let ids: Vec<Uuid> = figurine_ids.into_iter().filter(|id| seen.insert(*id)).collect();
        if !(TALE_POLL_MIN..=TALE_POLL_MAX).contains(&ids.len()) {
            return Err(AppError::BadRequest(format!(
                "A poll needs {TALE_POLL_MIN} to {TALE_POLL_MAX} works"
            )));
        }
        for id in &ids {
            if self.repo.get_figurine_by_id(*id).await?.is_none() {
                return Err(AppError::NotFound(format!("Work {id} not found")));
            }
        }
        if self.repo.tale_poll_is_open().await? {
            return Err(AppError::BadRequest(
                "A poll is already open — close it first".into(),
            ));
        }
        let id = self.repo.open_tale_poll(&ids).await?;
        Self::log_domain_event("tale_poll_opened", "tale_poll", id, "ok");
        Ok(())
    }

    /// Закрыть голосование и назвать, о ком будет байка. Если она уже вышла
    /// после открытия голосования, голосовавшие получают письма сейчас же.
    pub async fn admin_close_tale_poll(&self, poll_id: Uuid, winner: Uuid) -> Result<()> {
        let opened_at = self
            .repo
            .close_tale_poll(poll_id, winner)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("Only an open poll closes, and only on one of its works".into())
            })?;
        let laid = self.repo.fulfil_tale_poll_now(poll_id, winner, opened_at).await?;
        if laid > 0 {
            tracing::info!("Tale poll {poll_id}: the tale was already out, {laid} letters laid out");
        }
        Self::log_domain_event("tale_poll_closed", "tale_poll", poll_id, "ok");
        Ok(())
    }

    pub async fn admin_delete_tale_poll(&self, poll_id: Uuid) -> Result<()> {
        if !self.repo.delete_tale_poll(poll_id).await? {
            return Err(AppError::NotFound("Poll not found".into()));
        }
        Self::log_domain_event("tale_poll_deleted", "tale_poll", poll_id, "ok");
        Ok(())
    }

    /// Сцена байки для читателя: выключенная не отдаётся вовсе, как будто её
    /// нет, — хозяин снял её, чтобы не показывать, а не чтобы страница знала.
    pub async fn get_tale_scene(&self, tale_id: Uuid) -> Result<Option<TaleScene>> {
        Ok(self.repo.get_tale_scene(tale_id).await?.filter(|s| s.enabled))
    }

    /// Сцена байки для стола: и выключенная тоже.
    pub async fn admin_get_tale_scene(&self, tale_id: Uuid) -> Result<Option<TaleScene>> {
        self.repo.get_tale_scene(tale_id).await
    }

    pub async fn admin_put_tale_scene(&self, tale_id: Uuid, scene: TaleScene) -> Result<TaleScene> {
        let scene = normalize_tale_scene(scene)?;
        if !self.repo.put_tale_scene(tale_id, &scene).await? {
            return Err(AppError::BadRequest("Only a tale can carry a scene".into()));
        }
        Ok(scene)
    }

    pub async fn admin_delete_tale_scene(&self, tale_id: Uuid) -> Result<()> {
        self.repo.delete_tale_scene(tale_id).await
    }

    /// Назначить, какую байку продолжает эта. Если продолжение уже вышло и
    /// письма по нему разложены, просившие получают своё сейчас: порядок, в
    /// котором хозяин выложил байку и назвал её продолжением, не должен
    /// решать, дойдёт ли письмо.
    pub async fn admin_set_tale_sequel(&self, tale_id: Uuid, of: Option<Uuid>) -> Result<()> {
        if !self.repo.set_tale_sequel_of(tale_id, of).await? {
            return Err(AppError::BadRequest(
                "Both must be tales, and a tale cannot continue itself or a tale that already continues it"
                    .into(),
            ));
        }
        if let Some(original) = of {
            let laid = self.repo.enqueue_sequel_letters_now(original, tale_id).await?;
            if laid > 0 {
                tracing::info!("Tale {tale_id} named a sequel: {laid} letters laid out");
            }
        }
        Self::log_domain_event("tale_sequel_set", "tale", tale_id, "ok");
        Ok(())
    }

    /// Письма о вышедших байках. Зовётся фоном; возвращает число ушедших.
    ///
    /// Событий «опубликовано» здесь не ловят — по той же причине, что и у
    /// канала: запланированная байка выходит сама, когда пришло её время.
    /// Проход сверяет, что на людях сейчас, с отметкой `letters_at`, и
    /// раскладывает письма по каждой вышедшей байке один раз.
    pub async fn send_tale_letters(&self) -> Result<usize> {
        for tale in self.repo.tales_awaiting_letters(TALE_LETTER_GRACE_SECS).await? {
            let laid = self.repo.lay_out_tale_letters(tale).await?;
            if laid > 0 {
                tracing::info!("Tale {tale}: {laid} letters laid out");
                let title = self
                    .repo
                    .leaf_title(tale)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "a new tale".into());
                self.note_on_chart(&format!("Tale letters: {title}")).await;
            }
        }

        let local = self.config.public_url_is_local();
        // Нечем отправить — письма ждут в очереди (неделю), а не тратят
        // попытки на заведомый отказ. Почта собирается один раз на проход.
        let mailer = if local {
            None
        } else {
            match self.mailer().await {
                Ok(mailer) => Some(mailer),
                Err(e) => {
                    // Почты нет вовсе — молча: так задумано. Почта названа, но
                    // не собирается (кривой адрес отправителя) — это слышно.
                    if self.mail_works().await {
                        tracing::warn!("Tale letters are waiting: mail is set up but does not assemble: {e}");
                    }
                    return Ok(0);
                }
            }
        };

        let site = self.config.public_url.trim_end_matches('/').to_string();
        let due = self
            .repo
            .claim_tale_letters(
                TALE_LETTERS_PER_TICK,
                TALE_LETTER_MAX_ATTEMPTS,
                TALE_LETTER_CLAIM_SECS,
            )
            .await?;
        let mut sent = 0;
        for letter in due {
            let Some(reason) = crate::tales::LetterReason::parse(&letter.reason) else {
                continue;
            };
            let lang = crate::lang::Lang::parse(&letter.lang).unwrap_or_default();
            let pick = |en: Option<&str>,
                        ru: Option<&str>,
                        uk: Option<&str>,
                        es: Option<&str>,
                        de: Option<&str>,
                        it: Option<&str>| {
                crate::tales::in_lang(lang, en, ru, uk, es, de, it)
            };
            let title = pick(
                Some(&letter.title_en),
                Some(&letter.title_ru),
                Some(&letter.title_uk),
                Some(&letter.title_es),
                Some(&letter.title_de),
                Some(&letter.title_it),
            )
            .unwrap_or_default();
            let dek = pick(
                letter.dek_en.as_deref(),
                letter.dek_ru.as_deref(),
                letter.dek_uk.as_deref(),
                letter.dek_es.as_deref(),
                letter.dek_de.as_deref(),
                letter.dek_it.as_deref(),
            );
            let original = pick(
                letter.original_title_en.as_deref(),
                letter.original_title_ru.as_deref(),
                letter.original_title_uk.as_deref(),
                letter.original_title_es.as_deref(),
                letter.original_title_de.as_deref(),
                letter.original_title_it.as_deref(),
            );
            // `src` остаётся в замере посещений (§ 18): по нему видно, сколько
            // людей привело письмо.
            let link = format!("{site}/tales/{}?src=letter", letter.slug);
            let door = letter
                .unsubscribe_token
                .as_deref()
                .map(|t| format!("{site}/unsubscribe/{t}"));
            let (subject, body) = crate::tales::compose_letter(&crate::tales::TaleLetter {
                reason,
                lang,
                title: &title,
                dek: dek.as_deref(),
                link: &link,
                original: original.as_deref(),
                chosen_work: letter.figurine_name.as_deref(),
                unsubscribe: door.as_deref(),
            });

            // Локально письмо не уходит: база здесь своя, адреса в ней —
            // пробные, а ящики настоящие. Вместо письма — строка в журнале.
            let Some(mailer) = mailer.as_ref() else {
                tracing::info!("[local] tale letter to {} — {subject} → {link}", letter.email);
                self.repo.tale_letter_sent(letter.tale_id, &letter.email).await?;
                sent += 1;
                continue;
            };
            match Self::deliver(mailer, &letter.email, &subject, body).await {
                Ok(()) => {
                    self.repo.tale_letter_sent(letter.tale_id, &letter.email).await?;
                    sent += 1;
                }
                Err(e) => {
                    tracing::warn!("Tale letter to {} failed: {e}", letter.email);
                    self.repo
                        .tale_letter_failed(letter.tale_id, &letter.email, &e.to_string())
                        .await?;
                }
            }
        }
        Ok(sent)
    }

    /// Записки в Telegram о вышедших продолжениях. Раскладывает их тот же
    /// проход, что и письма (`lay_out_tale_letters`); здесь — только
    /// отправка. Пишет бот входа: с ним человек сам начинал разговор, когда
    /// входил, а бот уведомлений ему незнаком и написать не может.
    pub async fn send_tale_notes(&self) -> Result<usize> {
        let local = self.config.public_url_is_local();
        let bot = self.login_bot();
        // Бота нет — записки ждут в очереди (неделю), а не тратят попытки.
        if !local && bot.is_none() {
            return Ok(0);
        }
        let site = self.config.public_url.trim_end_matches('/').to_string();
        let due = self
            .repo
            .claim_tale_notes(
                TALE_LETTERS_PER_TICK,
                TALE_LETTER_MAX_ATTEMPTS,
                TALE_LETTER_CLAIM_SECS,
            )
            .await?;
        let mut sent = 0;
        for note in due {
            let Some(chat) = note.telegram_id else {
                // Telegram от имени отвязали после просьбы — писать некуда, и
                // ждать нечего.
                self.repo
                    .tale_note_failed(
                        note.tale_id,
                        note.user_id,
                        "Telegram is no longer linked to this name",
                        true,
                        TALE_LETTER_MAX_ATTEMPTS,
                    )
                    .await?;
                continue;
            };
            let lang = crate::lang::Lang::parse(&note.lang).unwrap_or_default();
            let title = crate::tales::in_lang(
                lang,
                Some(&note.title_en),
                Some(&note.title_ru),
                Some(&note.title_uk),
                Some(&note.title_es),
                Some(&note.title_de),
                Some(&note.title_it),
            )
            .unwrap_or_default();
            let original = crate::tales::in_lang(
                lang,
                note.original_title_en.as_deref(),
                note.original_title_ru.as_deref(),
                note.original_title_uk.as_deref(),
                note.original_title_es.as_deref(),
                note.original_title_de.as_deref(),
                note.original_title_it.as_deref(),
            );
            // Свой `src`, отдельный от канала: по нему видно, сколько людей
            // привели записки (§ 18).
            let link = format!("{site}/tales/{}?src=telegram_note", note.slug);
            let (text, button) = crate::tales::compose_note(lang, &title, original.as_deref());

            // Локально записка не уходит: база своя, а чаты настоящие.
            let Some(bot) = bot.as_ref().filter(|_| !local) else {
                tracing::info!("[local] tale note to Telegram chat {chat} → {link}");
                self.repo.tale_note_sent(note.tale_id, note.user_id).await?;
                sent += 1;
                continue;
            };
            match bot.post_text(&chat.to_string(), &text, button, &link).await {
                Ok(_) => {
                    self.repo.tale_note_sent(note.tale_id, note.user_id).await?;
                    sent += 1;
                }
                Err(why) => {
                    tracing::warn!("Tale note to Telegram chat {chat} failed: {why}");
                    self.repo
                        .tale_note_failed(
                            note.tale_id,
                            note.user_id,
                            &why,
                            false,
                            TALE_LETTER_MAX_ATTEMPTS,
                        )
                        .await?;
                }
            }
        }
        Ok(sent)
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

    // === CABINET GAZETTE ===

    fn gazette_leaf_dto(row: GazetteLeafListed) -> GazetteLeafDto {
        Self::map_gazette_leaf(row, false)
    }

    fn gazette_leaf_dto_admin(row: GazetteLeafListed) -> GazetteLeafDto {
        Self::map_gazette_leaf(row, true)
    }

    fn map_gazette_leaf(row: GazetteLeafListed, admin: bool) -> GazetteLeafDto {
        GazetteLeafDto {
            id: row.id.to_string(),
            slug: row.slug,
            kind: row.kind,
            status: row.status,
            title_en: row.title_en,
            title_ru: row.title_ru,
            title_uk: row.title_uk,
            title_es: row.title_es,
            title_de: row.title_de,
            title_it: row.title_it,
            dek_en: row.dek_en,
            dek_ru: row.dek_ru,
            dek_uk: row.dek_uk,
            dek_es: row.dek_es,
            dek_de: row.dek_de,
            dek_it: row.dek_it,
            body_en: row.body_en,
            body_ru: row.body_ru,
            body_uk: row.body_uk,
            body_es: row.body_es,
            body_de: row.body_de,
            body_it: row.body_it,
            figurine_id: row.figurine_id.map(|u| u.to_string()),
            figurine_name: row.figurine_name,
            figurine_name_translations: crate::models::parse_name_translations(
                row.figurine_name_translations.as_deref(),
            ),
            figurine_slug: row.figurine_slug,
            href: row.href,
            source_name: row.source_name,
            source_url: row.source_url,
            author: row.author,
            image_url: row.image_url.clone(),
            image_urls: if row.image_urls.is_empty() {
                row.image_url.clone().into_iter().collect()
            } else {
                row.image_urls
            },
            pinned: row.pinned,
            published_at: row.published_at.map(|t| t.to_rfc3339()),
            scheduled_at: row.scheduled_at.map(|t| t.to_rfc3339()),
            expected_from: row.expected_from.map(|d| d.to_string()),
            expected_to: row.expected_to.map(|d| d.to_string()),
            figurine_status: row.figurine_status,
            watch_count: if admin { Some(row.watch_count) } else { None },
            shelf_order: row.shelf_order,
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
            prev: None,
            next: None,
        }
    }

    fn gazette_cutting_dto(row: GazetteCuttingListed, public: bool) -> GazetteCuttingDto {
        GazetteCuttingDto {
            id: row.id.to_string(),
            feed_id: row.feed_id.to_string(),
            title: row.title,
            url: row.url,
            // Public cards are a pointer, not a reprint. RSS excerpts stay in admin.
            summary: if public { None } else { row.summary },
            source_name: row.source_name,
            published_at: row.published_at.map(|t| t.to_rfc3339()),
            dismissed: row.dismissed,
            pinned: row.pinned,
            created_at: row.created_at.to_rfc3339(),
            mark_key: if crate::gazette::valid_mark_key(&row.mark_key) {
                row.mark_key
            } else {
                "letter".into()
            },
            mark_url: row.mark_url,
        }
    }

    fn gazette_feed_dto(row: GazetteFeed) -> GazetteFeedDto {
        GazetteFeedDto {
            id: row.id.to_string(),
            title: row.title,
            url: row.url,
            enabled: row.enabled,
            last_fetched_at: row.last_fetched_at.map(|t| t.to_rfc3339()),
            last_error: row.last_error,
            created_at: row.created_at.to_rfc3339(),
            mark_key: if crate::gazette::valid_mark_key(&row.mark_key) {
                row.mark_key
            } else {
                "letter".into()
            },
            mark_url: row.mark_url,
        }
    }

    async fn gazette_leaf_listed_or_fetch(&self, id: Uuid) -> Result<GazetteLeafDto> {
        let row = self
            .repo
            .get_gazette_leaf_admin(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Gazette leaf {id} not found")))?;
        Ok(Self::gazette_leaf_dto_admin(row))
    }

    fn parse_gazette_schedule(raw: Option<&str>) -> Result<Option<DateTime<Utc>>> {
        match raw.map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) => DateTime::parse_from_rfc3339(s)
                .map(|d| Some(d.with_timezone(&Utc)))
                .map_err(|_| AppError::BadRequest("Invalid scheduledAt (expected RFC3339)".into())),
        }
    }

    pub async fn get_gazette_home(&self) -> Result<GazetteHomeDto> {
        let leaves = self
            .repo
            .list_gazette_leaves_home(crate::gazette::HOME_LEAVES)
            .await?;
        let cuttings = self
            .repo
            .list_gazette_cuttings_public(crate::gazette::HOME_CUTTINGS)
            .await?;
        Ok(GazetteHomeDto {
            leaves: leaves.into_iter().map(Self::gazette_leaf_dto).collect(),
            cuttings: cuttings
                .into_iter()
                .map(|row| Self::gazette_cutting_dto(row, true))
                .collect(),
        })
    }

    pub async fn list_gazette_public(&self, page: i64, per_page: i64) -> Result<GazetteLeavesPage> {
        let per_page = per_page.clamp(1, 500);
        let page = page.max(1);
        let offset = (page - 1) * per_page;
        let (items, total) = self
            .repo
            .list_gazette_leaves_public(per_page, offset)
            .await?;
        Ok(GazetteLeavesPage {
            items: items.into_iter().map(Self::gazette_leaf_dto).collect(),
            total,
            page,
            per_page,
        })
    }

    pub async fn get_gazette_room(&self, year: Option<i32>) -> Result<GazetteRoomDto> {
        let years = self.repo.list_gazette_years().await?;
        let current = Utc::now().year();
        let year = match year.filter(|y| (1990..=2100).contains(y)) {
            Some(y) => y,
            None => {
                if years.contains(&current) {
                    current
                } else {
                    years.first().copied().unwrap_or(current)
                }
            }
        };
        let leaves = self
            .repo
            .list_gazette_leaves_year(year, crate::gazette::ROOM_LEAVES)
            .await?;
        let cuttings = self
            .repo
            .list_gazette_cuttings_year(year, crate::gazette::ROOM_CUTTINGS)
            .await?;
        Ok(GazetteRoomDto {
            year,
            years,
            leaves: leaves.into_iter().map(Self::gazette_leaf_dto).collect(),
            cuttings: cuttings
                .into_iter()
                .map(|row| Self::gazette_cutting_dto(row, true))
                .collect(),
        })
    }

    pub async fn get_gazette_leaf_public(&self, slug: &str) -> Result<GazetteLeafDto> {
        let row = self
            .repo
            .get_gazette_leaf_by_slug(slug)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Gazette leaf {slug} not found")))?;
        let (prev, next) = self.repo.gazette_leaf_neighbors(slug, &row.kind).await?;
        let mut dto = Self::gazette_leaf_dto(row);
        dto.prev = prev;
        dto.next = next;
        Ok(dto)
    }

    /// The whole shelf of tall tales at once — the room has no pagination,
    /// because counting pages is exactly the shop reflex it is built against.
    pub async fn list_tales_public(&self) -> Result<Vec<GazetteLeafDto>> {
        let rows = self
            .repo
            .list_tales_public(crate::gazette::SHELF_TALES)
            .await?;
        Ok(rows.into_iter().map(Self::gazette_leaf_dto).collect())
    }

    /// Числа для шапки: сколько работ, листов вестника и небылиц и сколько из
    /// них новее меток посетителя.
    pub async fn section_counts(&self, q: SectionCountsQuery) -> Result<SectionCounts> {
        self.repo
            .section_counts(q.works_since, q.gazette_since, q.tales_since)
            .await
    }

    /// Lay the shelf out in the given order; position is the index in the list.
    pub async fn admin_reorder_tales(&self, ids: Vec<Uuid>) -> Result<()> {
        if ids.len() > crate::gazette::SHELF_TALES as usize {
            return Err(AppError::BadRequest("Shelf is longer than the room".into()));
        }
        let mut seen = std::collections::HashSet::with_capacity(ids.len());
        if !ids.iter().all(|id| seen.insert(*id)) {
            return Err(AppError::BadRequest(
                "A tale cannot stand in two places on the shelf".into(),
            ));
        }
        self.repo.set_tale_shelf_order(&ids).await?;
        Ok(())
    }

    pub async fn list_gazette_for_work(&self, figurine_id: Uuid) -> Result<Vec<GazetteLeafDto>> {
        let rows = self
            .repo
            .list_gazette_leaves_for_figurine(figurine_id, crate::gazette::WORK_LEAVES)
            .await?;
        Ok(rows.into_iter().map(Self::gazette_leaf_dto).collect())
    }

    pub async fn admin_list_gazette_leaves(
        &self,
        status: Option<&str>,
        kind: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<GazetteLeavesPage> {
        let per_page = per_page.clamp(1, 100);
        let page = page.max(1);
        let offset = (page - 1) * per_page;
        let (items, total) = self
            .repo
            .list_gazette_leaves_admin(status, kind, per_page, offset)
            .await?;
        Ok(GazetteLeavesPage {
            items: items
                .into_iter()
                .map(Self::gazette_leaf_dto_admin)
                .collect(),
            total,
            page,
            per_page,
        })
    }

    pub async fn admin_get_gazette_leaf(&self, id: Uuid) -> Result<GazetteLeafDto> {
        self.gazette_leaf_listed_or_fetch(id).await
    }

    pub async fn admin_create_gazette_leaf(
        &self,
        req: SaveGazetteLeafRequest,
    ) -> Result<GazetteLeafDto> {
        let taken = self.repo.list_gazette_slugs_except(None).await?;
        let p = Self::prepare_gazette_save(&req, &taken, None)?;
        let rec = self
            .repo
            .insert_gazette_leaf(
                &p.slug,
                &p.kind,
                &p.status,
                &p.title_en,
                &p.title_ru,
                p.dek_en.as_deref(),
                p.dek_ru.as_deref(),
                p.body_en.as_deref(),
                p.body_ru.as_deref(),
                p.figurine_id,
                p.href.as_deref(),
                p.source_name.as_deref(),
                p.source_url.as_deref(),
                p.author.as_deref(),
                p.image_url.as_deref(),
                &p.image_urls,
                p.pinned,
                p.published_at,
                p.scheduled_at,
                p.expected_from,
                p.expected_to,
                &p.title_uk,
                &p.title_es,
                &p.title_de,
                &p.title_it,
                p.dek_uk.as_deref(),
                p.dek_es.as_deref(),
                p.dek_de.as_deref(),
                p.dek_it.as_deref(),
                p.body_uk.as_deref(),
                p.body_es.as_deref(),
                p.body_de.as_deref(),
                p.body_it.as_deref(),
            )
            .await?;
        Self::log_domain_event("gazette_leaf_created", "gazette_leaf", rec.id, "ok");
        let _ = self.sync_leaf_watches_to_work(rec.id).await;
        self.gazette_leaf_listed_or_fetch(rec.id).await
    }

    pub async fn admin_update_gazette_leaf(
        &self,
        id: Uuid,
        mut req: SaveGazetteLeafRequest,
    ) -> Result<GazetteLeafDto> {
        let existing = self
            .repo
            .get_gazette_leaf_admin(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Gazette leaf {id} not found")))?;
        // Адрес байки, о которой письма уже разложены, стоит в этих письмах:
        // новый адрес оставил бы каждое из них ссылкой в «не найдено». Молча
        // остаётся прежний — стол и так не даёт его править.
        if existing.kind == "tale" && self.repo.tale_letters_laid(id).await? {
            req.slug = Some(existing.slug.clone());
        }
        let taken = self.repo.list_gazette_slugs_except(Some(id)).await?;
        let p = Self::prepare_gazette_save(&req, &taken, existing.published_at)?;
        let rec = self
            .repo
            .update_gazette_leaf(
                id,
                &p.slug,
                &p.kind,
                &p.status,
                &p.title_en,
                &p.title_ru,
                p.dek_en.as_deref(),
                p.dek_ru.as_deref(),
                p.body_en.as_deref(),
                p.body_ru.as_deref(),
                p.figurine_id,
                p.href.as_deref(),
                p.source_name.as_deref(),
                p.source_url.as_deref(),
                p.author.as_deref(),
                p.image_url.as_deref(),
                &p.image_urls,
                p.pinned,
                p.published_at,
                p.scheduled_at,
                p.expected_from,
                p.expected_to,
                &p.title_uk,
                &p.title_es,
                &p.title_de,
                &p.title_it,
                p.dek_uk.as_deref(),
                p.dek_es.as_deref(),
                p.dek_de.as_deref(),
                p.dek_it.as_deref(),
                p.body_uk.as_deref(),
                p.body_es.as_deref(),
                p.body_de.as_deref(),
                p.body_it.as_deref(),
            )
            .await?;
        Self::log_domain_event("gazette_leaf_updated", "gazette_leaf", rec.id, "ok");
        let _ = self.sync_leaf_watches_to_work(rec.id).await;
        self.gazette_leaf_listed_or_fetch(rec.id).await
    }

    pub async fn admin_delete_gazette_leaf(&self, id: Uuid) -> Result<()> {
        self.repo.delete_gazette_leaf(id).await?;
        Self::log_domain_event("gazette_leaf_deleted", "gazette_leaf", id, "ok");
        Ok(())
    }

    pub async fn watch_gazette_leaf(
        &self,
        slug: &str,
        req: WatchGazetteLeafRequest,
        user_id: Option<Uuid>,
        user_email: Option<&str>,
        user_email_confirmed: bool,
        user_name: Option<&str>,
    ) -> Result<GazetteWatchCreatedResponse> {
        if !req.age_confirmed {
            return Err(AppError::BadRequest(
                "Please confirm you are 16 or older".into(),
            ));
        }
        let leaf = self
            .repo
            .get_gazette_leaf_by_slug(slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Gazette leaf not found".into()))?;
        if leaf.kind != "sketch" {
            return Err(AppError::BadRequest(
                "You can only ask to be told about a work still in the making".into(),
            ));
        }
        if leaf.figurine_status.as_deref() == Some("available") {
            return Err(AppError::BadRequest(
                "This work is already on the site".into(),
            ));
        }
        let mut email = req
            .email
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        if email.is_empty() {
            email = user_email.unwrap_or("").trim().to_string();
        }
        if !email.contains('@') || email.len() > 200 {
            return Err(AppError::BadRequest("Valid email is required".into()));
        }
        let name = req
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| {
                user_name
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            });
        let lang = req
            .lang
            .as_deref()
            .and_then(crate::lang::Lang::parse)
            .unwrap_or_default()
            .code();
        // Письмо «работа готова» уходит позже и само, поэтому слежение
        // начинает действовать, когда адрес доказан: это подтверждённая почта
        // просящего, подтверждённый подписчик книги — или переход по ссылке из
        // письма, которое уходит на адрес сейчас.
        let owner = user_id.is_some()
            && user_email_confirmed
            && user_email.is_some_and(|e| e.trim().eq_ignore_ascii_case(&email));
        let address_proved = owner || self.repo.is_confirmed_subscriber(&email).await?;
        let key = secret_key();
        let outcome = self
            .repo
            .request_gazette_watch(
                leaf.id,
                &email,
                name.as_deref(),
                lang,
                user_id,
                owner,
                address_proved,
                &fingerprint(&key),
            )
            .await?;
        let watch = outcome.watch;
        if outcome.send_confirmation {
            let svc = self.clone();
            let w = watch.clone();
            // Имя работы — на языке письма, без перевода — английское.
            let work = crate::models::parse_name_translations(
                leaf.figurine_name_translations.as_deref(),
            )
            .remove(lang)
            .or_else(|| leaf.figurine_name.clone())
            .unwrap_or_default();
            tokio::spawn(async move {
                if let Err(e) = svc.send_watch_confirmation(&w, &work, &key).await {
                    tracing::warn!("watch confirmation letter not sent: {e}");
                }
            });
        }
        if outcome.own_key
            && let Some(fid) = leaf.figurine_id
        {
            let fname = leaf.figurine_name.clone().unwrap_or_default();
            let _ = self.mirror_watch_as_notify(&watch, fid, &fname).await;
        }
        Self::log_domain_event("gazette_watch_left", "gazette_watch", watch.id, "ok");
        // Ключ отказа получает тот, кто завёл строку или владеет адресом.
        // Просящий на чужую строку мог набрать чужой адрес: ему пустышка.
        Ok(GazetteWatchCreatedResponse {
            cancel_token: if outcome.own_key {
                watch.cancel_token
            } else {
                Self::generate_decoy_token()
            },
            already_watching: outcome.existed,
            confirmed: watch.confirmed_at.is_some(),
        })
    }

    /// Ссылка из письма слежения.
    pub async fn confirm_gazette_watch(&self, key: &str) -> Result<bool> {
        self.repo.confirm_gazette_watch(&fingerprint(key.trim())).await
    }

    async fn send_watch_confirmation(
        &self,
        watch: &GazetteWatch,
        work: &str,
        key: &str,
    ) -> Result<()> {
        let base = self.config.public_url.trim_end_matches('/');
        let link = format!("{base}/gazette/watch/confirm/{key}");
        use crate::lang::Lang;
        let (subject, body) = match Lang::parse(&watch.lang).unwrap_or_default() {
            Lang::Ru => (
                "Подтвердите адрес: напишем, когда работа будет готова".to_string(),
                format!(
                    "Этот адрес попросил сообщить, когда будет готова работа «{work}». \
                     Если это были вы, откройте ссылку:\n{link}\n\n\
                     Если вы ничего не просили, не отвечайте на это письмо: без вашего \
                     перехода по ссылке не придёт ничего. Ссылка живёт неделю."
                ),
            ),
            Lang::En => (
                "Confirm your address: we will write when the work is ready".to_string(),
                format!(
                    "This address asked to be told when “{work}” is ready. If that was you, \
                     open the link:\n{link}\n\n\
                     If you asked for nothing, ignore this letter: without your follow of the \
                     link nothing will be sent. The link lives a week."
                ),
            ),
            Lang::Uk => (
                "Підтвердьте адресу: напишемо, коли робота буде готова".to_string(),
                format!(
                    "Ця адреса попросила повідомити, коли буде готова робота «{work}». \
                     Якщо це були ви, відкрийте посилання:\n{link}\n\n\
                     Якщо ви нічого не просили, не відповідайте на цей лист: без вашого \
                     переходу за посиланням не прийде нічого. Посилання діє тиждень."
                ),
            ),
            Lang::Es => (
                "Confirme su dirección: le escribiremos cuando la obra esté terminada".to_string(),
                format!(
                    "Esta dirección pidió un aviso cuando «{work}» esté terminada. Si fue usted, \
                     abra el enlace:\n{link}\n\n\
                     Si no pidió nada, ignore esta carta: sin abrir el enlace no se enviará nada. \
                     El enlace vale una semana."
                ),
            ),
            Lang::De => (
                "Bestätigen Sie Ihre Adresse: Wir schreiben, wenn die Arbeit fertig ist".to_string(),
                format!(
                    "Für diese Adresse wurde eine Nachricht erbeten, sobald „{work}“ fertig ist. \
                     Wenn Sie das waren, öffnen Sie den Link:\n{link}\n\n\
                     Wenn Sie nichts erbeten haben, ignorieren Sie diesen Brief: Ohne den Link zu \
                     öffnen, wird nichts gesendet. Der Link gilt eine Woche."
                ),
            ),
            Lang::It => (
                "Confermi il suo indirizzo: le scriveremo quando l’opera sarà finita".to_string(),
                format!(
                    "Questo indirizzo ha chiesto un avviso quando «{work}» sarà finita. Se è stato lei, \
                     apra il link:\n{link}\n\n\
                     Se non ha chiesto nulla, ignori questa lettera: senza aprire il link non verrà \
                     inviato niente. Il link vale una settimana."
                ),
            ),
        };
        self.say_or_send(&watch.email, &subject, body, &link).await
    }

    pub async fn get_gazette_watch_by_token(
        &self,
        token: &str,
    ) -> Result<Option<GazetteWatchInfo>> {
        let Some(w) = self.repo.get_gazette_watch_by_token(token).await? else {
            return Ok(None);
        };
        let Some(leaf) = self.repo.get_gazette_leaf_admin(w.leaf_id).await? else {
            return Ok(None);
        };
        Ok(Some(GazetteWatchInfo {
            leaf_slug: leaf.slug,
            title_en: leaf.title_en,
            title_ru: leaf.title_ru,
            title_uk: leaf.title_uk,
            title_es: leaf.title_es,
            title_de: leaf.title_de,
            title_it: leaf.title_it,
            notified: w.notified_at.is_some(),
            confirmed: w.confirmed_at.is_some(),
        }))
    }

    pub async fn leave_gazette_watch(&self, token: &str) -> Result<()> {
        self.repo.delete_gazette_watch_by_token(token).await?;
        Ok(())
    }

    pub async fn list_user_gazette_watches(
        &self,
        user_id: Uuid,
        email: Option<&str>,
    ) -> Result<Vec<GazetteWatchDto>> {
        // Слежения, оставленные гостем, подбираются по почте. Без почты
        // подбирать нечего — список остаётся тем, что заведено под именем.
        if let Some(email) = email {
            let _ = self.repo.link_gazette_watches_to_user(user_id, email).await;
        }
        let rows = self.repo.list_gazette_watches_for_user(user_id).await?;
        Ok(rows
            .into_iter()
            .map(|w| GazetteWatchDto {
                id: w.id.to_string(),
                leaf_id: w.leaf_id.to_string(),
                leaf_slug: w.leaf_slug,
                title_en: w.title_en,
                title_ru: w.title_ru,
                title_uk: w.title_uk,
                title_es: w.title_es,
                title_de: w.title_de,
                title_it: w.title_it,
                cancel_token: w.cancel_token,
                notified_at: w.notified_at.map(|t| t.to_rfc3339()),
                created_at: w.created_at.to_rfc3339(),
            })
            .collect())
    }

    async fn mirror_watch_as_notify(
        &self,
        watch: &GazetteWatch,
        figurine_id: Uuid,
        figurine_name: &str,
    ) -> Result<()> {
        let req = OrderRequest {
            figurine_id: figurine_id.to_string(),
            figurine_name: figurine_name.to_string(),
            requester_name: watch
                .name
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "—".into()),
            requester_email: watch.email.clone(),
            requester_phone: None,
            message: None,
            mode: OrderMode::Notify,
            lang: Some(watch.lang.clone()),
            age_confirmed: true,
        };
        let saved = self.repo.upsert_notify_order(&req, watch.user_id).await?;
        self.note_lang("orders", saved.id, watch.user_id, req.lang.as_deref()).await;
        Ok(())
    }

    async fn sync_leaf_watches_to_work(&self, leaf_id: Uuid) -> Result<()> {
        let Some(leaf) = self.repo.get_gazette_leaf_admin(leaf_id).await? else {
            return Ok(());
        };
        let Some(fid) = leaf.figurine_id else {
            return Ok(());
        };
        let name = leaf.figurine_name.clone().unwrap_or_default();
        let watches = self.repo.list_gazette_watches_for_figurine(fid).await?;
        for w in &watches {
            let _ = self.mirror_watch_as_notify(w, fid, &name).await;
        }
        if leaf.kind == "sketch" && leaf.figurine_status.as_deref() == Some("available") {
            let pending = self
                .repo
                .list_unnotified_gazette_watches_for_figurine(fid)
                .await
                .unwrap_or_default();
            if !pending.is_empty() {
                let svc = self.clone();
                let n = name.clone();
                tokio::spawn(async move {
                    let _ = svc.send_availability_digest(fid, &n).await;
                    let _ = svc.send_sketch_watch_letters(fid, &n).await;
                });
            }
        }
        Ok(())
    }

    pub async fn admin_list_gazette_feeds(&self) -> Result<Vec<GazetteFeedDto>> {
        Ok(self
            .repo
            .list_gazette_feeds()
            .await?
            .into_iter()
            .map(Self::gazette_feed_dto)
            .collect())
    }

    pub async fn admin_create_gazette_feed(
        &self,
        req: SaveGazetteFeedRequest,
    ) -> Result<GazetteFeedDto> {
        let title = req.title.trim();
        let url = req.url.trim();
        if title.is_empty() || url.is_empty() {
            return Err(AppError::BadRequest(
                "Feed title and url are required".into(),
            ));
        }
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(AppError::BadRequest("Feed url must be http(s)".into()));
        }
        let mark_key = req
            .mark_key
            .as_deref()
            .map(str::trim)
            .filter(|s| crate::gazette::valid_mark_key(s))
            .map(|s| s.to_string())
            .unwrap_or_else(|| crate::gazette::guess_mark_key(&title, &url));
        let rec = self
            .repo
            .insert_gazette_feed(title, url, req.enabled, &mark_key)
            .await
            .map_err(|e| match e {
                AppError::Database(_) => {
                    AppError::Conflict("A feed with this url already sits on the desk".into())
                }
                other => other,
            })?;
        Self::log_domain_event("gazette_feed_created", "gazette_feed", rec.id, "ok");
        Ok(Self::gazette_feed_dto(rec))
    }

    pub async fn admin_update_gazette_feed(
        &self,
        id: Uuid,
        req: SaveGazetteFeedRequest,
    ) -> Result<GazetteFeedDto> {
        let title = req.title.trim();
        let url = req.url.trim();
        if title.is_empty() || url.is_empty() {
            return Err(AppError::BadRequest(
                "Feed title and url are required".into(),
            ));
        }
        let existing = self
            .repo
            .list_gazette_feeds()
            .await?
            .into_iter()
            .find(|f| f.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Gazette feed {id} not found")))?;
        let mark_key = req
            .mark_key
            .as_deref()
            .map(str::trim)
            .filter(|s| crate::gazette::valid_mark_key(s))
            .map(|s| s.to_string())
            .unwrap_or(existing.mark_key);
        let mark_url = match &req.mark_url {
            None => existing.mark_url,
            Some(s) => {
                let t = s.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            }
        };
        let rec = self
            .repo
            .update_gazette_feed(id, title, url, req.enabled, &mark_key, mark_url.as_deref())
            .await?;
        Ok(Self::gazette_feed_dto(rec))
    }

    pub async fn admin_delete_gazette_feed(&self, id: Uuid) -> Result<()> {
        self.repo.delete_gazette_feed(id).await?;
        Self::log_domain_event("gazette_feed_deleted", "gazette_feed", id, "ok");
        Ok(())
    }

    pub async fn refresh_gazette_desk(&self) -> Result<GazetteRefreshReport> {
        let feeds = self.repo.list_gazette_feeds().await?;
        let mut imported: i64 = 0;
        let mut errors = Vec::new();
        let mut fetched: i64 = 0;
        let origin = self.config.public_url.trim_end_matches('/');
        for feed in feeds.into_iter().filter(|f| f.enabled) {
            fetched += 1;
            let result = self
                .http_client
                .get(&feed.url)
                .header(
                    "User-Agent",
                    format!("GotigaCabinet/1.0 (gazette desk; +{origin})"),
                )
                .header(
                    "Accept",
                    "application/rss+xml, application/atom+xml, application/xml, text/xml",
                )
                .send()
                .await;
            match result {
                Err(e) => {
                    let msg = format!("{}: {e}", feed.title);
                    let _ = self
                        .repo
                        .mark_gazette_feed_fetched(feed.id, Some(&msg))
                        .await;
                    errors.push(msg);
                }
                Ok(res) if !res.status().is_success() => {
                    let msg = format!("{}: HTTP {}", feed.title, res.status());
                    let _ = self
                        .repo
                        .mark_gazette_feed_fetched(feed.id, Some(&msg))
                        .await;
                    errors.push(msg);
                }
                Ok(res) => match res.text().await {
                    Err(e) => {
                        let msg = format!("{}: {e}", feed.title);
                        let _ = self
                            .repo
                            .mark_gazette_feed_fetched(feed.id, Some(&msg))
                            .await;
                        errors.push(msg);
                    }
                    Ok(xml) => {
                        let items = crate::gazette::parse_feed(&xml);
                        match self.repo.upsert_gazette_cuttings(feed.id, &items).await {
                            Ok(n) => {
                                imported += n;
                                let _ = self.repo.mark_gazette_feed_fetched(feed.id, None).await;
                            }
                            Err(e) => {
                                let msg = format!("{}: {e}", feed.title);
                                let _ = self
                                    .repo
                                    .mark_gazette_feed_fetched(feed.id, Some(&msg))
                                    .await;
                                errors.push(msg);
                            }
                        }
                    }
                },
            }
        }
        Self::log_domain_event("gazette_desk_refreshed", "gazette", fetched, "ok");
        Ok(GazetteRefreshReport {
            feeds: fetched,
            imported,
            errors,
        })
    }

    pub async fn admin_list_gazette_cuttings(
        &self,
        bucket: &str,
        feed_id: Option<Uuid>,
        page: i64,
        per_page: i64,
    ) -> Result<GazetteCuttingsPage> {
        let bucket = match bucket {
            "table" | "aside" | "all" | "inbox" => bucket,
            _ => "inbox",
        };
        let per_page = per_page.clamp(1, 100);
        let page = page.max(1);
        let offset = (page - 1) * per_page;
        let (items, total) = self
            .repo
            .list_gazette_cuttings_admin(bucket, feed_id, per_page, offset)
            .await?;
        Ok(GazetteCuttingsPage {
            items: items
                .into_iter()
                .map(|row| Self::gazette_cutting_dto(row, false))
                .collect(),
            total,
            page,
            per_page,
        })
    }

    pub async fn admin_dismiss_gazette_cutting(&self, id: Uuid, dismissed: bool) -> Result<()> {
        self.repo.set_gazette_cutting_dismissed(id, dismissed).await
    }

    pub async fn admin_pin_gazette_cutting(&self, id: Uuid, pinned: bool) -> Result<()> {
        self.repo.set_gazette_cutting_pinned(id, pinned).await
    }

    pub async fn admin_promote_gazette_cutting(&self, id: Uuid) -> Result<GazetteLeafDto> {
        let cutting = self
            .repo
            .get_gazette_cutting(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Gazette cutting {id} not found")))?;
        let title = crate::gazette::clamp_title(&cutting.title);
        let req = SaveGazetteLeafRequest {
            slug: None,
            kind: "world".into(),
            status: "draft".into(),
            title_en: title.clone(),
            title_ru: title,
            title_uk: String::new(),
            title_es: String::new(),
            title_de: String::new(),
            title_it: String::new(),
            dek_en: None,
            dek_ru: None,
            dek_uk: None,
            dek_es: None,
            dek_de: None,
            dek_it: None,
            body_en: None,
            body_ru: None,
            body_uk: None,
            body_es: None,
            body_de: None,
            body_it: None,
            figurine_id: None,
            href: Some(cutting.url.clone()),
            source_name: Some(cutting.source_name.clone()),
            source_url: Some(cutting.url),
            author: None,
            image_url: None,
            image_urls: vec![],
            pinned: false,
            scheduled_at: None,
            expected_from: None,
            expected_to: None,
        };
        // Leave the cutting where it is. Taking it off the desk while the
        // rewrite is still a draft would punch a hole in the blotter.
        self.admin_create_gazette_leaf(req).await
    }

    fn prepare_gazette_save(
        req: &SaveGazetteLeafRequest,
        taken: &[String],
        existing_published_at: Option<DateTime<Utc>>,
    ) -> Result<PreparedGazetteLeaf> {
        if !crate::gazette::valid_kind(&req.kind) {
            return Err(AppError::BadRequest("Unknown gazette kind".into()));
        }
        if !crate::gazette::valid_status(&req.status) {
            return Err(AppError::BadRequest("Unknown gazette status".into()));
        }
        let mut title_en = crate::gazette::clamp_title(&req.title_en);
        let mut title_ru = crate::gazette::clamp_title(&req.title_ru);
        let title_uk = crate::gazette::clamp_title(&req.title_uk);
        let title_es = crate::gazette::clamp_title(&req.title_es);
        let title_de = crate::gazette::clamp_title(&req.title_de);
        let title_it = crate::gazette::clamp_title(&req.title_it);
        if title_en.is_empty() && title_ru.is_empty() {
            // A leaf written only in Ukrainian (or Spanish, German, Italian)
            // still needs a title in the two columns that cannot be empty (they
            // key the address and the neighbours); the first written one stands
            // in for both.
            let Some(stand_in) = [&title_uk, &title_es, &title_de, &title_it]
                .into_iter()
                .find(|t| !t.is_empty())
                .cloned()
            else {
                return Err(AppError::BadRequest("A title is required".into()));
            };
            title_en = stand_in.clone();
            title_ru = stand_in;
        }
        if title_en.is_empty() {
            title_en = title_ru.clone();
        }
        if title_ru.is_empty() {
            title_ru = title_en.clone();
        }
        let slug = crate::gazette::unique_slug(req.slug.as_deref(), &title_en, taken);
        let figurine_id = match req
            .figurine_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            None => None,
            Some(s) => Some(
                Uuid::parse_str(s)
                    .map_err(|_| AppError::BadRequest("Invalid figurineId".into()))?,
            ),
        };
        let scheduled_at = Self::parse_gazette_schedule(req.scheduled_at.as_deref())?;
        if req.status == "scheduled" && scheduled_at.is_none() {
            return Err(AppError::BadRequest(
                "A scheduled leaf needs scheduledAt".into(),
            ));
        }
        let now = Utc::now();
        let published_at = match req.status.as_str() {
            "published" => Some(existing_published_at.unwrap_or(now)),
            "scheduled" if scheduled_at.map(|t| t <= now).unwrap_or(false) => {
                Some(existing_published_at.unwrap_or(now))
            }
            _ => None,
        };
        let image_urls =
            crate::gazette::normalize_image_urls(req.image_url.as_deref(), &req.image_urls);
        let cover = image_urls.first().cloned();
        let (expected_from, expected_to) = crate::gazette::normalize_expected(
            &req.kind,
            req.expected_from.as_deref(),
            req.expected_to.as_deref(),
        )
        .map_err(AppError::BadRequest)?;
        Ok(PreparedGazetteLeaf {
            slug,
            kind: req.kind.clone(),
            status: req.status.clone(),
            title_en,
            title_ru,
            dek_en: crate::gazette::clamp_dek(req.dek_en.as_deref()),
            dek_ru: crate::gazette::clamp_dek(req.dek_ru.as_deref()),
            body_en: crate::gazette::clamp_body(req.body_en.as_deref()),
            body_ru: crate::gazette::clamp_body(req.body_ru.as_deref()),
            title_uk,
            title_es,
            title_de,
            title_it,
            dek_uk: crate::gazette::clamp_dek(req.dek_uk.as_deref()),
            dek_es: crate::gazette::clamp_dek(req.dek_es.as_deref()),
            dek_de: crate::gazette::clamp_dek(req.dek_de.as_deref()),
            dek_it: crate::gazette::clamp_dek(req.dek_it.as_deref()),
            body_uk: crate::gazette::clamp_body(req.body_uk.as_deref()),
            body_es: crate::gazette::clamp_body(req.body_es.as_deref()),
            body_de: crate::gazette::clamp_body(req.body_de.as_deref()),
            body_it: crate::gazette::clamp_body(req.body_it.as_deref()),
            figurine_id,
            href: req
                .href
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            source_name: req
                .source_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            source_url: req
                .source_url
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            author: crate::gazette::clamp_author(req.author.as_deref()),
            image_url: cover.clone(),
            image_urls,
            pinned: req.pinned,
            published_at,
            scheduled_at,
            expected_from,
            expected_to,
        })
    }

    // === СКРОМНЫЕ ЭПИЧЕСКИЕ БИТВЫ ===

    /// A card wears its own picture when the keeper gave it one, and the work's
    /// face otherwise. The choice is made here, once, so neither the shelf nor
    /// the keeper's preview can disagree about what a card looks like.
    fn battle_card_dto(&self, row: BattleCardListed, admin: bool) -> BattleCardDto {
        let own = row
            .art_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let art_url = match (&own, &row.figurine_face_path, &row.figurine_face_id) {
            (Some(url), _, _) => Some(self.resolve_url(url, "images", "")),
            (None, Some(path), Some(img_id)) => {
                Some(self.resolve_url(path, "images", &img_id.to_string()))
            }
            _ => None,
        };
        BattleCardDto {
            id: row.id.to_string(),
            slug: row.slug,
            status: row.status,
            tier: row.tier,
            race_id: row.race_id.map(|id| id.to_string()),
            race_name_en: row.race_name_en,
            race_name_ru: row.race_name_ru,
            race_name_uk: row.race_name_uk,
            race_name_es: row.race_name_es,
            race_name_de: row.race_name_de,
            race_name_it: row.race_name_it,
            race_icon_url: row.race_icon_url,
            race_level_frames: row.race_level_frames,
            race_motion_wear: row.race_motion_wear,
            race_damage: crate::battles::parse_race_damage(row.race_damage.as_deref()),
            type_en: row.type_en,
            type_ru: row.type_ru,
            type_uk: row.type_uk,
            type_es: row.type_es,
            type_de: row.type_de,
            type_it: row.type_it,
            title_en: row.title_en,
            title_ru: row.title_ru,
            title_uk: row.title_uk,
            title_es: row.title_es,
            title_de: row.title_de,
            title_it: row.title_it,
            effect_en: row.effect_en,
            effect_ru: row.effect_ru,
            effect_uk: row.effect_uk,
            effect_es: row.effect_es,
            effect_de: row.effect_de,
            effect_it: row.effect_it,
            lore_en: row.lore_en,
            lore_ru: row.lore_ru,
            lore_uk: row.lore_uk,
            lore_es: row.lore_es,
            lore_de: row.lore_de,
            lore_it: row.lore_it,
            cost: row.cost,
            power: row.power,
            health: row.health,
            mana: row.mana,
            traits: crate::battles::read_traits(row.traits.as_deref()),
            kind: row.kind,
            armor: row.armor,
            ward: row.ward,
            attack_channel: row.attack_channel,
            reach: row.reach,
            step: row.step,
            speed: row.speed,
            mend: row.mend,
            abilities: crate::battles::read_abilities(row.abilities.as_deref()),
            // The verdict is for the desk. The shelf shows a card, not a ledger.
            budget_points: if admin { row.budget_points } else { None },
            balance_index: if admin { row.balance_index } else { None },
            rules_version: row.rules_version,
            price_dust: row.price_dust,
            price_feed: row.price_feed,
            level_price_dust: row.level_price_dust.clone(),
            art_url,
            art_url_override: if admin { own } else { None },
            art_focal: row.art_focal,
            frame_override: row.frame_override,
            motion_wear: row.motion_wear,
            shelf_order: row.shelf_order,
            lendable: row.lendable,
            shoots: row.shoots,
            credit_name: row.credit_name,
            edition_size: row.edition_size,
            minted: row.minted,
            figurine_id: row.figurine_id.map(|id| id.to_string()),
            figurine_name: row.figurine_name,
            figurine_name_translations: crate::models::parse_name_translations(
                row.figurine_name_translations.as_deref(),
            ),
            figurine_slug: row.figurine_slug,
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
        }
    }

    /// The whole shelf at once. No pagination, for the same reason the tales
    /// room refuses it: counting pages is the shop reflex this house is built
    /// against.
    pub async fn list_battle_cards_public(&self) -> Result<Vec<BattleCardDto>> {
        let rows = self
            .repo
            .list_battle_cards_public(crate::battles::SHELF_CARDS)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| self.battle_card_dto(r, false))
            .collect())
    }

    /// Карты «только для этюдов» — для сцены этюда, которой нужны их картинки.
    pub async fn list_battle_cards_etude_only(&self) -> Result<Vec<BattleCardDto>> {
        let rows = self
            .repo
            .list_battle_cards_etude_only(crate::battles::SHELF_CARDS)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| self.battle_card_dto(r, false))
            .collect())
    }

    pub async fn admin_list_battle_cards(&self) -> Result<Vec<BattleCardDto>> {
        let rows = self
            .repo
            .list_battle_cards_admin(crate::battles::SHELF_CARDS)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| self.battle_card_dto(r, true))
            .collect())
    }

    async fn battle_card_or_fetch(&self, id: Uuid) -> Result<BattleCardDto> {
        let row = self
            .repo
            .get_battle_card_admin(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Battle card {id} not found")))?;
        Ok(self.battle_card_dto(row, true))
    }

    pub async fn admin_get_battle_card(&self, id: Uuid) -> Result<BattleCardDto> {
        self.battle_card_or_fetch(id).await
    }

    pub async fn admin_create_battle_card(
        &self,
        req: SaveBattleCardRequest,
    ) -> Result<BattleCardDto> {
        Self::assert_card_complete(&req)?;
        self.admin_create_battle_card_signed(req, None).await
    }

    /// Стол хозяина не сохраняет карту без обязательного — при любом статусе.
    ///
    /// Не в `prepare_battle_card_save`: туда же приходит утверждение из
    /// студии, а гость пишет карту на своём языке, и черновиком она обязана
    /// лечь как есть — иначе перевести её было бы негде. Отказ называет поле
    /// тем же словом, что весы (`readiness.missing`), поэтому форма и сервер
    /// не расходятся.
    fn assert_card_complete(req: &SaveBattleCardRequest) -> Result<()> {
        match Self::card_readiness(req).missing.first() {
            Some(field) => Err(AppError::BadRequest(format!("card:missing:{field}"))),
            None => Ok(()),
        }
    }

    /// То же, и с подписью автора, когда карта пришла из студии.
    ///
    /// Подпись едет в ТОЙ ЖЕ записи, которой карта заводится, а не дописывается
    /// следующим запросом: обрыв между двумя запросами оставлял на полке чужую
    /// работу без имени автора (`STUDIO-CARDS-REVIEW.md` П7). В
    /// `SaveBattleCardRequest` её по-прежнему нет — подпись ставит студия, и
    /// приехать из формы стола хозяина она не должна.
    pub async fn admin_create_battle_card_signed(
        &self,
        req: SaveBattleCardRequest,
        credit: Option<&str>,
    ) -> Result<BattleCardDto> {
        let taken = self.repo.list_battle_card_slugs_except(None).await?;
        let mut p = Self::prepare_battle_card_save(&req, &taken)?;
        p.credit_name = credit.map(str::to_string);
        self.assert_work_is_free(p.figurine_id, None).await?;
        let rec = self.repo.insert_battle_card(&p).await?;
        Self::log_domain_event("battle_card_created", "battle_card", rec.id, "ok");
        self.battle_card_or_fetch(rec.id).await
    }

    pub async fn admin_update_battle_card(
        &self,
        id: Uuid,
        req: SaveBattleCardRequest,
    ) -> Result<BattleCardDto> {
        Self::assert_card_complete(&req)?;
        let taken = self.repo.list_battle_card_slugs_except(Some(id)).await?;
        let p = Self::prepare_battle_card_save(&req, &taken)?;
        self.assert_work_is_free(p.figurine_id, Some(id)).await?;
        // Как карта звалась ДО правки. Испытания ссылаются на карты по слугу —
        // намеренно, испытание это шаблон, — и переименование без этого просто
        // осиротило бы каждое из них: карта пропала бы с доски молча, а узнал
        // бы об этом гость.
        let before = self.repo.get_battle_card_admin(id).await?;
        // Тираж не опускается ниже уже отпечатанного: у людей на руках
        // экземпляры с номерами, и «из 50», под которым лежит семидесятый, —
        // это число, которое врёт молча.
        if let (Some(asked), Some(card)) = (p.edition_size, before.as_ref()) {
            if asked < card.minted {
                return Err(AppError::BadRequest(format!(
                    "Отпечатано уже {}: тираж меньше назначить нельзя",
                    card.minted
                )));
            }
        }
        let was = before.map(|c| c.slug);
        let rec = self.repo.update_battle_card(id, &p).await?;
        if let Some(old) = was.filter(|old| *old != rec.slug) {
            let moved = self.carry_slug_into_challenges(&old, &rec.slug).await?;
            if moved > 0 {
                Self::log_domain_event("battle_slug_carried", "battle_card", rec.id, "ok");
            }
        }
        // Rank and price are what an owner's collection is worth, so a change
        // to them leaves a trace even before anything can be bought.
        Self::log_domain_event("battle_card_updated", "battle_card", rec.id, "ok");
        self.battle_card_or_fetch(rec.id).await
    }

    /// Перенести переименование карты во все испытания, которые её называют.
    ///
    /// Разбором JSON, а не подстановкой в тексте: слуг «ved» встречается внутри
    /// «vedma», и текстовая замена однажды переписала бы половину чужого имени.
    /// Возвращает, сколько испытаний тронуто.
    async fn carry_slug_into_challenges(&self, from: &str, to: &str) -> Result<usize> {
        let mut moved = 0usize;
        for challenge in self.repo.list_battle_challenges(false).await? {
            let Ok(mut setup) =
                serde_json::from_str::<crate::models::ChallengeSetup>(&challenge.setup)
            else {
                // Испытание с нечитаемой расстановкой не чинится переименованием
                // и не должно ронять сохранение карты.
                continue;
            };
            let mut touched = false;
            for place in setup
                .player_board
                .iter_mut()
                .chain(setup.keeper_board.iter_mut())
            {
                if place.card == from {
                    place.card = to.to_string();
                    touched = true;
                }
            }
            for slug in setup
                .player_hand
                .iter_mut()
                .chain(setup.keeper_hand.iter_mut())
            {
                if slug == from {
                    *slug = to.to_string();
                    touched = true;
                }
            }
            if !touched {
                continue;
            }
            let json = serde_json::to_string(&setup)
                .map_err(|_| AppError::Internal("Arrangement will not serialise".into()))?;
            self.repo
                .update_battle_challenge_setup(challenge.id, &json)
                .await?;
            moved += 1;
        }
        Ok(moved)
    }

    pub async fn admin_delete_battle_card(&self, id: Uuid) -> Result<()> {
        self.repo.delete_battle_card(id).await?;
        Self::log_domain_event("battle_card_deleted", "battle_card", id, "ok");
        Ok(())
    }

    pub async fn admin_reorder_battle_cards(&self, ids: Vec<Uuid>) -> Result<()> {
        if ids.len() > crate::battles::SHELF_CARDS as usize {
            return Err(AppError::BadRequest("Shelf is longer than the room".into()));
        }
        let mut seen = std::collections::HashSet::with_capacity(ids.len());
        if !ids.iter().all(|id| seen.insert(*id)) {
            return Err(AppError::BadRequest(
                "A card cannot stand in two places on the shelf".into(),
            ));
        }
        self.repo.set_battle_card_order(&ids).await?;
        Ok(())
    }

    /// One card per work. Told plainly, because the alternative is a raw
    /// unique-index violation and a keeper who cannot tell what went wrong.
    async fn assert_work_is_free(
        &self,
        figurine_id: Option<Uuid>,
        except: Option<Uuid>,
    ) -> Result<()> {
        let Some(work) = figurine_id else {
            return Ok(());
        };
        if let Some(title) = self.repo.battle_card_for_figurine(work, except).await? {
            return Err(AppError::BadRequest(format!(
                "This work already has a card: «{title}»"
            )));
        }
        Ok(())
    }

    pub async fn get_battle_frames(&self) -> Result<crate::battles::BattleFrames> {
        let saved: crate::battles::BattleFrames = parse_json_setting(
            "battle_frames",
            self.repo.get_setting("battle_frames").await?,
        )?;
        Ok(crate::battles::BattleFrames {
            frames: crate::battles::normalize_frames(saved.frames),
        })
    }

    pub async fn save_battle_frames(
        &self,
        config: crate::battles::BattleFrames,
    ) -> Result<crate::battles::BattleFrames> {
        let normalized = crate::battles::BattleFrames {
            frames: crate::battles::normalize_frames(config.frames),
        };
        let json =
            serde_json::to_string(&normalized).map_err(|e| AppError::Internal(e.to_string()))?;
        // A backstop against a runaway save, not a real constraint — Postgres
        // couldn't care less about 64 KiB of text. Widened alongside the
        // presets cap below: every field five ranks carry (`frame_scale_x/y`
        // among the newest) grows this the same way it grows a saved dress,
        // and the old ceiling left barely any headroom for the next one.
        if json.len() > 256 * 1024 {
            return Err(AppError::BadRequest("Battle frames are too large".into()));
        }
        self.repo.upsert_setting("battle_frames", &json).await?;
        Ok(normalized)
    }

    /// The keeper's drawer of saved dresses. Admin-only on purpose: a preset
    /// is a tool of the desk, and no visitor's page is made better by carrying
    /// two dozen frames nothing on it wears.
    pub async fn get_battle_frame_presets(&self) -> Result<crate::battles::BattleFramePresets> {
        let saved: crate::battles::BattleFramePresets = parse_json_setting(
            "battle_frame_presets",
            self.repo.get_setting("battle_frame_presets").await?,
        )?;
        Ok(crate::battles::BattleFramePresets {
            presets: crate::battles::normalize_presets(saved.presets),
        })
    }

    pub async fn save_battle_frame_presets(
        &self,
        config: crate::battles::BattleFramePresets,
    ) -> Result<crate::battles::BattleFramePresets> {
        // Полный ящик — отказ, а не обрезка: `normalize_presets` отрезает
        // хвост, а новый наряд стоит именно в хвосте, и стол печатал
        // «отложено» над рамкой, которой в ящике не было.
        if config.presets.len() > crate::battles::PRESETS_MAX {
            return Err(AppError::BadRequest(format!(
                "Battle frame presets are full ({} max)",
                crate::battles::PRESETS_MAX
            )));
        }
        let normalized = crate::battles::BattleFramePresets {
            presets: crate::battles::normalize_presets(config.presets),
        };
        let json =
            serde_json::to_string(&normalized).map_err(|e| AppError::Internal(e.to_string()))?;
        // Same backstop as `save_battle_frames`, four times over: the drawer
        // is a whole frame PER PRESET, and a keeper who has been saving dresses
        // for a while already sits close to the old ceiling before adding a
        // single new one — 31 presets alone ran to 126 KiB under the 128 KiB
        // cap, so any field ever added to `BattleFrame` again (this one
        // included) was one save away from locking the drawer for everybody
        // who already filled it.
        if json.len() > 512 * 1024 {
            return Err(AppError::BadRequest(
                "Battle frame presets are too large".into(),
            ));
        }
        self.repo
            .upsert_setting("battle_frame_presets", &json)
            .await?;
        Ok(normalized)
    }

    // === ДВИЖЕНИЯ ===
    //
    // Свод живёт настройкой, а не таблицей, ровно по той же причине, что и
    // рамки чинов: это словарь дома, один на всю комнату, и читается он ЦЕЛИКОМ
    // при каждом входе в этюд. Таблица дала бы страницу, сортировку и запрос на
    // строку — три вещи, которых у словаря из полусотни записей быть не должно.

    /// Что играет комната. Публично: без свода сцена не покажет ни удара, ни
    /// лечения, а второй круг за ним посреди хода — это круг, который однажды
    /// не успеет.
    ///
    /// Пустой свод — не поломка: сцена тогда играет умолчания дома, то есть
    /// ровно то, что делала до движка (`BATTLE-MOTION.md` §4).
    pub async fn get_battle_motions(&self) -> Result<crate::battles::BattleMotions> {
        let saved: crate::battles::BattleMotions = parse_json_setting(
            "battle_motions",
            self.repo.get_setting("battle_motions").await?,
        )?;
        Ok(crate::battles::BattleMotions {
            motions: crate::battles::normalize_motions(saved.motions),
        })
    }

    pub async fn save_battle_motions(
        &self,
        config: crate::battles::BattleMotions,
    ) -> Result<crate::battles::BattleMotions> {
        let normalized = crate::battles::BattleMotions {
            motions: crate::battles::normalize_motions(config.motions),
        };
        let json =
            serde_json::to_string(&normalized).map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 128 * 1024 {
            return Err(AppError::BadRequest("Battle motions are too large".into()));
        }
        self.repo.upsert_setting("battle_motions", &json).await?;
        Ok(normalized)
    }

    // === КОШЕЛЁК, ВЛАДЕНИЕ, ЦЕРЕМОНИЯ ===

    pub async fn get_battle_dust_rates(&self) -> Result<crate::models::BattleDustRates> {
        parse_json_setting(
            "battle_dust_rates",
            self.repo.get_setting("battle_dust_rates").await?,
        )
    }

    pub async fn save_battle_dust_rates(
        &self,
        rates: crate::models::BattleDustRates,
    ) -> Result<crate::models::BattleDustRates> {
        // Отрицательная ставка — это не штраф, а испорченная книга: строка со
        // знаком минус за прочитанную небылицу. Обрезается молча.
        let clean = crate::models::BattleDustRates {
            liked: rates.liked.clamp(0, 1000),
            seen: rates.seen.clamp(0, 1000),
            read: rates.read.clamp(0, 1000),
        };
        let json = serde_json::to_string(&clean).map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("battle_dust_rates", &json).await?;
        Ok(clean)
    }

    // === ПОРУЧЕНИЯ ===
    //
    // Поручение не создаёт дело — оно называет заранее то, что дом и так считал
    // молча. Платит из той же дописываемой книги, ключом `errand:{slug}`, так
    // что «ровно однажды» достаётся даром и никакой второй машины выплат нет.

    /// Заплатить за всё, что гость успел закрыть.
    ///
    /// Зовётся из тех POST'ов, которые и так случаются: вход в комнату, маячок
    /// внимания, покупка, ступень, сохранение стола, ход в партии. Внутри
    /// чтения не зовётся никогда — страница, чтение которой печатает деньги,
    /// рано или поздно будет прочитана в цикле.
    ///
    /// Возвращает то, что заплачено ИМЕННО СЕЙЧАС: из этого складывается окно
    /// встречи, и оно должно молчать, когда сказать нечего.
    pub async fn settle_errands(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::BattleErrandDto>> {
        let errands = self.repo.list_published_battle_errands().await?;
        if errands.is_empty() {
            return Ok(Vec::new());
        }
        let windows = self.errand_windows(&errands).await?;
        let tallies = self.errand_tallies(user_id, &windows).await?;

        let mut paid = Vec::new();
        for (errand, window) in errands.iter().zip(&windows) {
            // Закрытое окно — не «ноль», а «нечего»: срок, который ещё не начался
            // или уже кончился, не должен ни платить, ни считаться.
            let Some((since, tail)) = window else { continue };
            // Дело дом не платит никогда. Корм за поступок даёт хранитель рукой,
            // и автоматическая выплата стёрла бы всю разницу между двумя
            // монетами: одна за время, другая за поступок.
            if errand.by_hand {
                continue;
            }
            let Some(tally) = tallies.get(since) else { continue };
            if tally.by_rule(&errand.rule) < i64::from(errand.threshold) {
                continue;
            }
            if self
                .repo
                .credit_battle_wallet(
                    user_id,
                    &errand.currency,
                    errand.amount,
                    "errand",
                    Some(errand.id),
                    &crate::battles::errand_key(&errand.slug, tail),
                )
                .await?
            {
                paid.push(Self::errand_dto(errand, tally, true));
            }
        }
        Ok(paid)
    }

    /// Окно каждого поручения, по порядку списка.
    ///
    /// Считается один раз на весь проход: часы дома читаются из настроек, а
    /// «сейчас» берётся ОДНО на все поручения — иначе полночь, наступившая между
    /// двумя строками цикла, развела бы соседние поручения по разным суткам.
    async fn errand_windows(
        &self,
        errands: &[crate::models::BattleErrand],
    ) -> Result<Vec<Option<(Option<DateTime<Utc>>, String)>>> {
        let clock = self.get_battle_clock().await?;
        let now = Utc::now();
        Ok(errands
            .iter()
            .map(|e| {
                crate::battles::errand_window(
                    &e.period,
                    now,
                    clock.offset_min,
                    e.starts_at,
                    e.ends_at,
                )
            })
            .collect())
    }

    /// Счёт на каждое РАЗЛИЧНОЕ окно, а не на каждое поручение.
    ///
    /// Девять разовых поручений спрашивают одни и те же семь чисел; ежедневное
    /// спрашивает их же, но с полуночи. Различных окон обычно два-три, и запросов
    /// столько же.
    async fn errand_tallies(
        &self,
        user_id: Uuid,
        windows: &[Option<(Option<DateTime<Utc>>, String)>],
    ) -> Result<std::collections::HashMap<Option<DateTime<Utc>>, crate::models::BattleErrandTally>>
    {
        let mut wanted: Vec<Option<DateTime<Utc>>> =
            windows.iter().flatten().map(|(since, _)| *since).collect();
        wanted.sort();
        wanted.dedup();
        let mut out = std::collections::HashMap::new();
        for since in wanted {
            out.insert(since, self.repo.battle_errand_tally(user_id, since).await?);
        }
        Ok(out)
    }

    // === ПОРУЧЕНИЯ: СТОЛ ХРАНИТЕЛЯ ===

    pub async fn admin_list_battle_errands(
        &self,
    ) -> Result<Vec<crate::models::AdminBattleErrandDto>> {
        let (rows, payouts) = tokio::try_join!(
            self.repo.list_battle_errands_admin(),
            self.repo.battle_errand_payouts(),
        )?;
        let paid: std::collections::HashMap<String, (i64, i64)> = payouts
            .into_iter()
            .map(|(slug, guests, coins)| (slug, (guests, coins)))
            .collect();
        Ok(rows
            .iter()
            .map(|e| Self::admin_errand_dto(e, &paid))
            .collect())
    }

    fn admin_errand_dto(
        e: &crate::models::BattleErrand,
        paid: &std::collections::HashMap<String, (i64, i64)>,
    ) -> crate::models::AdminBattleErrandDto {
        let (guests, coins) = paid.get(&e.slug).copied().unwrap_or((0, 0));
        crate::models::AdminBattleErrandDto {
            id: e.id.to_string(),
            slug: e.slug.clone(),
            title_en: e.title_en.clone(),
            title_ru: e.title_ru.clone(),
            title_uk: e.title_uk.clone(),
            title_es: e.title_es.clone(),
            title_de: e.title_de.clone(),
            title_it: e.title_it.clone(),
            note_en: e.note_en.clone(),
            note_ru: e.note_ru.clone(),
            note_uk: e.note_uk.clone(),
            note_es: e.note_es.clone(),
            note_de: e.note_de.clone(),
            note_it: e.note_it.clone(),
            rule: e.rule.clone(),
            threshold: e.threshold,
            currency: e.currency.clone(),
            amount: e.amount,
            period: e.period.clone(),
            starts_at: e.starts_at.map(|t| t.to_rfc3339()),
            ends_at: e.ends_at.map(|t| t.to_rfc3339()),
            status: e.status.clone(),
            by_hand: e.by_hand,
            sort_order: e.sort_order,
            paid_guests: guests,
            paid_coins: coins,
            slug_locked: guests > 0,
        }
    }

    /// Завести поручение или поправить существующее.
    ///
    /// Отказ называет причину СЛОВОМ, а не текстом: текст живёт в `i18n`, и
    /// сервер, который его сочиняет, сочиняет его на одном языке.
    ///
    /// Правка суммы прошлое не переписывает — книга только дописывается, и тот,
    /// кому уже заплатили десять, останется с десятью. Это сказано на самом
    /// столе словами, а не оставлено догадываться.
    pub async fn admin_save_battle_errand(
        &self,
        req: crate::models::SaveBattleErrandRequest,
    ) -> Result<crate::models::AdminBattleErrandDto> {
        let (existing, payouts) = tokio::try_join!(
            self.repo.list_battle_errands_admin(),
            self.repo.battle_errand_payouts(),
        )?;
        let paid: std::collections::HashMap<String, (i64, i64)> = payouts
            .into_iter()
            .map(|(slug, guests, coins)| (slug, (guests, coins)))
            .collect();

        let mine = req
            .id
            .and_then(|id| existing.iter().find(|e| e.id == id));
        if req.id.is_some() && mine.is_none() {
            return Err(AppError::NotFound("Battle errand not found".into()));
        }

        let clean = Self::prepare_errand(&req, mine, &existing, &paid)?;
        let saved = self.repo.upsert_battle_errand(req.id, &clean).await?;
        Ok(Self::admin_errand_dto(&saved, &paid))
    }

    /// Всё, что хранитель напечатал, приведённое к тому, что комната выдержит.
    fn prepare_errand(
        req: &crate::models::SaveBattleErrandRequest,
        mine: Option<&crate::models::BattleErrand>,
        all: &[crate::models::BattleErrand],
        paid: &std::collections::HashMap<String, (i64, i64)>,
    ) -> Result<crate::models::SaveBattleErrandRequest> {
        let slug = crate::battles::clean_errand_slug(&req.slug);
        if slug.is_empty() {
            return Err(AppError::BadRequest("errand:slug".into()));
        }
        // Ключ книги собран из slug'а. Переименование после первой выплаты —
        // это НОВОЕ поручение, за которое заплатят второй раз всем, кто его уже
        // прошёл, и заметит это не тот, кто переименовывал.
        if let Some(old) = mine {
            let locked = paid.get(&old.slug).map(|p| p.0 > 0).unwrap_or(false);
            if locked && slug != old.slug {
                return Err(AppError::BadRequest("errand:slugLocked".into()));
            }
        }
        if all
            .iter()
            .any(|e| e.slug == slug && Some(e.id) != req.id)
        {
            return Err(AppError::BadRequest("errand:slugTaken".into()));
        }

        if !crate::battles::errand_rule_known(&req.rule) {
            return Err(AppError::BadRequest("errand:rule".into()));
        }

        let period = if req.period.is_empty() {
            "once".to_string()
        } else {
            req.period.clone()
        };
        if !crate::battles::valid_errand_period(&period) {
            return Err(AppError::BadRequest("errand:period".into()));
        }
        // Условие о СОСТОЯНИИ не знает окна по существу: у снимка нет времени.
        // Повторяющееся поручение с таким условием платило бы каждый период за
        // то, что человек сделал однажды, — и это единственное место, где
        // повторение молча опустошает книгу.
        if period != "once" && crate::battles::errand_rule_stateful(&req.rule) {
            return Err(AppError::BadRequest("errand:statefulRepeat".into()));
        }
        // Дело не повторяется: повторять нечего, машина его не платит, и период
        // у него был бы состоянием, которое ничего не значит и однажды собьёт
        // с толку.
        let period = if req.by_hand { "once".to_string() } else { period };
        if period == "window" {
            let Some(from) = req.starts_at else {
                return Err(AppError::BadRequest("errand:window".into()));
            };
            if let Some(to) = req.ends_at {
                if to <= from {
                    return Err(AppError::BadRequest("errand:window".into()));
                }
            }
        }

        // Состояний у поручения два, и `valid_status` тут не годится: он про
        // карты, у которых есть ещё и «в отставке».
        let status = match req.status.as_str() {
            "draft" | "published" => req.status.clone(),
            _ => return Err(AppError::BadRequest("errand:status".into())),
        };

        // Потолок повторяющихся — часть кода, а не совет (`BATTLE-ERRANDS.md` §5).
        if period != "once" && status == "published" {
            let others = all
                .iter()
                .filter(|e| {
                    Some(e.id) != req.id && e.status == "published" && e.period != "once"
                })
                .count();
            if others >= crate::battles::ERRANDS_REPEATING_MAX {
                return Err(AppError::BadRequest("errand:repeatingMax".into()));
            }
        }

        if !crate::battles::CURRENCIES.contains(&req.currency.as_str()) {
            return Err(AppError::BadRequest("errand:currency".into()));
        }
        let amount = req.amount;
        if amount < 1 || amount > crate::battles::ERRAND_AMOUNT_MAX {
            return Err(AppError::BadRequest("errand:amount".into()));
        }
        let threshold = req.threshold;
        if threshold < 1 || threshold > crate::battles::ERRAND_THRESHOLD_MAX {
            return Err(AppError::BadRequest("errand:threshold".into()));
        }

        let title_en = Self::clamp_text(&req.title_en, crate::battles::ERRAND_TITLE_MAX);
        let title_ru = Self::clamp_text(&req.title_ru, crate::battles::ERRAND_TITLE_MAX);
        let title_uk = Self::clamp_text(&req.title_uk, crate::battles::ERRAND_TITLE_MAX);
        let title_es = Self::clamp_text(&req.title_es, crate::battles::ERRAND_TITLE_MAX);
        let title_de = Self::clamp_text(&req.title_de, crate::battles::ERRAND_TITLE_MAX);
        let title_it = Self::clamp_text(&req.title_it, crate::battles::ERRAND_TITLE_MAX);
        // Оба языка, а не «хотя бы один»: пустой английский когда-то заполняли
        // русским, и английская полка печатала кириллицу как английский.
        if title_en.is_empty() || title_ru.is_empty() {
            return Err(AppError::BadRequest("errand:title".into()));
        }
        let note = |v: &Option<String>| -> Option<String> {
            v.as_deref()
                .map(|t| Self::clamp_text(t, crate::battles::ERRAND_NOTE_MAX))
                .filter(|t| !t.is_empty())
        };

        Ok(crate::models::SaveBattleErrandRequest {
            id: req.id,
            slug,
            title_en,
            title_ru,
            title_uk,
            title_es,
            title_de,
            title_it,
            note_en: note(&req.note_en),
            note_ru: note(&req.note_ru),
            note_uk: note(&req.note_uk),
            note_es: note(&req.note_es),
            note_de: note(&req.note_de),
            note_it: note(&req.note_it),
            rule: req.rule.clone(),
            threshold,
            currency: req.currency.clone(),
            amount,
            period,
            starts_at: req.starts_at,
            ends_at: req.ends_at,
            status,
            by_hand: req.by_hand,
            sort_order: req.sort_order,
        })
    }

    /// Обрезает, а не отказывает: заголовок на символ длиннее не стоит потерянного
    /// черновика.
    fn clamp_text(raw: &str, max: usize) -> String {
        let t = raw.trim();
        if t.chars().count() <= max {
            return t.to_string();
        }
        t.chars().take(max).collect::<String>().trim_end().to_string()
    }

    pub async fn admin_delete_battle_errand(&self, id: Uuid) -> Result<()> {
        self.repo.delete_battle_errand(id).await
    }

    pub async fn admin_reorder_battle_errands(&self, ids: &[Uuid]) -> Result<()> {
        self.repo.reorder_battle_errands(ids).await
    }

    /// Лист поручений: только отчёт, ни одной строки в книгу.
    pub async fn list_battle_errands(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::BattleErrandDto>> {
        let (errands, keys) = tokio::try_join!(
            self.repo.list_published_battle_errands(),
            self.repo.battle_wallet_keys(user_id),
        )?;
        let paid: std::collections::HashSet<String> = keys.into_iter().collect();
        let windows = self.errand_windows(&errands).await?;
        let tallies = self.errand_tallies(user_id, &windows).await?;

        // Окна те же, что и при выплате, и это обязательно: список, считающий
        // прогресс иначе, чем платит книга, показывал бы «4 из 5» рядом с уже
        // закрытым поручением.
        Ok(errands
            .iter()
            .zip(&windows)
            .filter_map(|(e, window)| {
                let (since, tail) = window.as_ref()?;
                let tally = tallies.get(since)?;
                let done = paid.contains(&crate::battles::errand_key(&e.slug, tail));
                Some(Self::errand_dto(e, tally, done))
            })
            .collect())
    }

    fn errand_dto(
        errand: &crate::models::BattleErrand,
        tally: &crate::models::BattleErrandTally,
        done: bool,
    ) -> crate::models::BattleErrandDto {
        // У дела прогресса нет намеренно. Показать «1 из 1» рядом со строкой,
        // которую дом не оплатит, значит пообещать выплату и не сделать её —
        // а «сделано» без монеты хуже, чем ничего.
        let done = done && !errand.by_hand;
        // Прогресс обрезан порогом: «7 из 5» — не отчёт, а недоразумение, и
        // закрытое поручение должно показывать свой порог, а не то, сколько
        // человек нагулял сверх него потом.
        let have = tally
            .by_rule(&errand.rule)
            .min(i64::from(errand.threshold))
            .max(0);
        crate::models::BattleErrandDto {
            id: errand.id.to_string(),
            slug: errand.slug.clone(),
            by_hand: errand.by_hand,
            title_en: errand.title_en.clone(),
            title_ru: errand.title_ru.clone(),
            title_uk: errand.title_uk.clone(),
            title_es: errand.title_es.clone(),
            title_de: errand.title_de.clone(),
            title_it: errand.title_it.clone(),
            note_en: errand.note_en.clone(),
            note_ru: errand.note_ru.clone(),
            note_uk: errand.note_uk.clone(),
            note_es: errand.note_es.clone(),
            note_de: errand.note_de.clone(),
            note_it: errand.note_it.clone(),
            rule: errand.rule.clone(),
            threshold: errand.threshold,
            currency: errand.currency.clone(),
            amount: errand.amount,
            period: errand.period.clone(),
            have: if errand.by_hand {
                0
            } else {
                i32::try_from(have).unwrap_or(errand.threshold)
            },
            done,
        }
    }

    /// Отметить, что человек заходил сегодня.
    ///
    /// День — по часам дома: московский вечер и есть сегодняшний день, а не
    /// вчерашний, каким его считает UTC.
    ///
    /// Не роняет вход: комната, не открывшаяся оттого, что не записалась
    /// отметка о приходе, — это ровно наоборот.
    async fn mark_visit_quietly(&self, user_id: Uuid) {
        let offset = match self.get_battle_clock().await {
            Ok(c) => c.offset_min,
            Err(_) => crate::models::BattleClock::default().offset_min,
        };
        let Some(off) = chrono::FixedOffset::east_opt(offset * 60) else {
            return;
        };
        let day = Utc::now().with_timezone(&off).date_naive();
        if let Err(e) = self.repo.mark_battle_visit(user_id, day).await {
            tracing::warn!(%user_id, "приход не записался: {e}");
        }
    }

    /// Расплатиться по поручениям, не роняя того, кто позвал.
    ///
    /// Покупка карты не должна отказать оттого, что справочник поручений не
    /// прочитался: человек заплатил за карту, а не за лист. Ошибка уходит в
    /// журнал, и следующий же POST досчитает — книга дописываемая, терять
    /// нечего.
    async fn settle_errands_quietly(&self, user_id: Uuid) -> Vec<crate::models::BattleErrandDto> {
        match self.settle_errands(user_id).await {
            Ok(paid) => paid,
            Err(e) => {
                tracing::warn!(%user_id, "поручения не досчитались: {e}");
                Vec::new()
            }
        }
    }

    pub async fn get_battle_clock(&self) -> Result<crate::models::BattleClock> {
        parse_json_setting("battle_clock", self.repo.get_setting("battle_clock").await?)
    }

    pub async fn save_battle_clock(
        &self,
        clock: crate::models::BattleClock,
    ) -> Result<crate::models::BattleClock> {
        // Смещение больше суток — не часовой пояс, а испорченная настройка, от
        // которой «сегодня» уезжает в позавчера.
        let clean = crate::models::BattleClock {
            offset_min: clock.offset_min.clamp(-14 * 60, 14 * 60),
        };
        let json = serde_json::to_string(&clean)
            .map_err(|_| AppError::Internal("Часы не сериализуются".into()))?;
        self.repo.upsert_setting("battle_clock", &json).await?;
        Ok(clean)
    }

    // ── Студия ──────────────────────────────────────────────────────────────

    /// Пускают ли этого человека за стол.
    ///
    /// `owners` спрашивает владение картами, а не число сыгранных партий:
    /// карта на руках — самый простой честный признак того, что человек в игре,
    /// и он же не отсекает того, кто пришёл именно творить.
    async fn studio_gate_open(&self, user_id: Uuid, gate: &str) -> Result<bool> {
        Ok(match gate {
            "all" => true,
            "closed" => false,
            _ => !self.repo.list_owned_battle_cards(user_id).await?.is_empty(),
        })
    }

    pub async fn studio_state(&self, user_id: Uuid) -> Result<crate::models::StudioStateDto> {
        let settings = self.get_studio_settings().await?;
        let open = self.studio_gate_open(user_id, &settings.gate).await?;
        Ok(crate::models::StudioStateDto {
            open,
            gate: settings.gate.clone(),
            // Согласие СТАРОЙ редакции — не согласие: текст меняется, и человек,
            // принявший прошлый, нового не принимал.
            agreed: self
                .repo
                .studio_agreement(user_id)
                .await?
                .filter(|v| v == crate::studio::AGREEMENT),
            agreement: crate::studio::AGREEMENT.to_string(),
            frames: if open {
                self.repo.list_studio_frames(user_id).await?
            } else {
                Vec::new()
            },
            r#box: self.studio_box(user_id, &settings).await?,
            settings,
        })
    }

    pub async fn studio_box(
        &self,
        user_id: Uuid,
        settings: &crate::studio::StudioSettings,
    ) -> Result<crate::models::StudioBoxDto> {
        Ok(crate::models::StudioBoxDto {
            used: self.repo.studio_box_used(user_id).await?,
            limit: settings.box_bytes,
            assets: self.repo.list_studio_assets(user_id).await?,
            sheets: self.repo.list_studio_sheets(user_id).await?,
        })
    }

    /// Общая дверь всех действий студии: заперта — дальше не идём.
    async fn studio_admit(&self, user_id: Uuid) -> Result<crate::studio::StudioSettings> {
        let settings = self.get_studio_settings().await?;
        if !self.studio_gate_open(user_id, &settings.gate).await? {
            return Err(AppError::BadRequest("studioClosed".into()));
        }
        Ok(settings)
    }

    /// Положить деталь в ящик. Место проверяется ДО записи файла — отказ после
    /// того, как байты уже легли на диск, оставил бы мусор, которого никто не
    /// считает.
    pub async fn studio_room_for(&self, user_id: Uuid, bytes: usize) -> Result<()> {
        let settings = self.studio_admit(user_id).await?;
        if bytes as i64 > settings.asset_bytes {
            return Err(AppError::BadRequest("assetTooBig".into()));
        }
        self.studio_box_room(user_id, bytes, &settings).await
    }

    /// Влезет ли это в ящик — и только это.
    ///
    /// Отдельно от `studio_room_for`, потому что ЛИСТ НЕ ДЕТАЛЬ: потолок детали
    /// два мегабайта (настоящий максимум на складе — 438 КБ), а лист по замеру
    /// весит 0.6–3 МБ, и мерить его меркой детали — значит не принять ни одного
    /// листа вообще. Листу свой забор ставят число пикселей и потолок листов
    /// одновременно.
    async fn studio_box_room(
        &self,
        user_id: Uuid,
        bytes: usize,
        settings: &crate::studio::StudioSettings,
    ) -> Result<()> {
        let used = self.repo.studio_box_used(user_id).await?;
        if used + bytes as i64 > settings.box_bytes {
            return Err(AppError::BadRequest("boxFull".into()));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_studio_asset(
        &self,
        user_id: Uuid,
        name: &str,
        role: &str,
        url: &str,
        width: i32,
        height: i32,
        bytes: i32,
    ) -> Result<crate::models::StudioAssetDto> {
        let name = name.trim();
        let name = if name.is_empty() { "деталь" } else { name };
        let name: String = name.chars().take(80).collect();
        self.repo
            .insert_studio_asset(
                user_id,
                &name,
                &crate::studio::clamp_asset_role(role),
                url,
                width,
                height,
                bytes,
            )
            .await
    }

    /// Запомнить язык работы. Не свойство рамы, а свойство разговора о ней.
    pub async fn set_studio_frame_lang(&self, user: Uuid, id: Uuid, lang: &str) -> Result<()> {
        let lang = crate::lang::Lang::parse(lang).unwrap_or_default().code();
        self.repo.set_studio_frame_lang(user, id, lang).await
    }

    pub async fn remove_studio_asset(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        match self.repo.delete_studio_asset(user_id, id).await? {
            Some(url) => {
                self.remove_upload(&url).await;
                Ok(())
            }
            // Чужая деталь для этого человека не существует — не «запрещено».
            None => Err(AppError::NotFound("studio asset".into())),
        }
    }

    /// Куда класть пакет: в эту запись, в новую — или никуда.
    ///
    /// Три разных случая, и раньше все три отвечали одинаковым 404:
    ///
    /// — записи нет вовсе (её убрали, или браузер помнит ссылку с прошлой
    ///   жизни базы) — ССЫЛКА ПРОТУХЛА, и работу надо завести заново: терять
    ///   чужой труд из-за нашей уборки нельзя;
    /// — запись есть, но уже на людях — это не ошибка адреса, а отказ по делу,
    ///   и сказать надо словом, а не «не найдено»;
    /// — запись есть и в работе — писать в неё.
    pub async fn studio_package_target(
        &self,
        user_id: Uuid,
        id: Option<Uuid>,
    ) -> Result<Option<Uuid>> {
        let Some(id) = id else { return Ok(None) };
        match self.repo.studio_frame_ready(user_id, id).await? {
            None => Ok(None),
            Some((status, _, _)) if crate::studio::frame_may_edit(&status) => Ok(Some(id)),
            Some(_) => Err(AppError::BadRequest("alreadyOut".into())),
        }
    }

    /// Завести рамку. Тело нормализуется теми же правилами, что и рама дома:
    /// иначе гость назначил бы врезку в тысячу процентов, и её пришлось бы
    /// чинить на карте, которую уже кто-то увидел.
    pub async fn create_studio_frame(
        &self,
        user_id: Uuid,
        req: crate::models::SaveStudioFrameRequest,
    ) -> Result<crate::models::StudioFrameDto> {
        let settings = self.studio_admit(user_id).await?;
        let open = self.repo.count_studio_frames_open(user_id).await?;
        if open >= settings.frames_open as i64 {
            return Err(AppError::BadRequest("tooManyFrames".into()));
        }
        let (name, body) = Self::clean_studio_frame(req)?;
        let weight = self.studio_frame_weight(user_id, &body, &settings).await?;
        let frame = self.repo.insert_studio_frame(user_id, &name, &body).await?;
        self.repo.set_studio_frame_bytes(frame.id, weight).await?;
        Ok(crate::models::StudioFrameDto { bytes: weight, ..frame })
    }

    /// Сколько весит рама — суммой её деталей.
    ///
    /// Считается на записи, а не при чтении: потолок на одну работу нужен ровно
    /// в тот миг, когда её кладут. И считается по СВОИМ деталям: рама может
    /// носить картинку из библиотеки дома, и её вес — не вес этого человека.
    async fn studio_frame_weight(
        &self,
        user_id: Uuid,
        body: &serde_json::Value,
        settings: &crate::studio::StudioSettings,
    ) -> Result<i32> {
        let mut urls: Vec<String> = Vec::new();
        Self::collect_studio_urls(body, &mut urls);
        urls.sort();
        urls.dedup();
        let weight = self.repo.studio_assets_weight(user_id, &urls).await?;
        if weight > settings.frame_bytes {
            return Err(AppError::BadRequest("frameTooHeavy".into()));
        }
        Ok(weight as i32)
    }

    /// Адреса картинок студии, какие ни попадись в теле рамы. Обходом, а не
    /// перечислением полей: полей у рамы двадцать, и список, который забыли
    /// дописать, — это деталь, которая не весит ничего.
    fn collect_studio_urls(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => {
                if s.contains("/static/studio/") {
                    out.push(s.clone());
                }
            }
            serde_json::Value::Array(list) => {
                for one in list {
                    Self::collect_studio_urls(one, out);
                }
            }
            serde_json::Value::Object(map) => {
                for one in map.values() {
                    Self::collect_studio_urls(one, out);
                }
            }
            _ => {}
        }
    }

    pub async fn save_studio_frame(
        &self,
        user_id: Uuid,
        id: Uuid,
        req: crate::models::SaveStudioFrameRequest,
    ) -> Result<crate::models::StudioFrameDto> {
        let settings = self.studio_admit(user_id).await?;
        let (name, body) = Self::clean_studio_frame(req)?;
        let weight = self.studio_frame_weight(user_id, &body, &settings).await?;
        match self
            .repo
            .update_studio_frame(user_id, id, &name, &body)
            .await?
        {
            Some(frame) => {
                self.repo.set_studio_frame_bytes(frame.id, weight).await?;
                Ok(crate::models::StudioFrameDto { bytes: weight, ..frame })
            }
            // Либо чужая, либо уже выложена: и то и другое для этого человека
            // выглядит одинаково — править нечего.
            None => Err(AppError::NotFound("studio frame".into())),
        }
    }

    pub async fn delete_studio_frame(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        if self.repo.delete_studio_frame(user_id, id).await? {
            Ok(())
        } else {
            Err(AppError::NotFound("studio frame".into()))
        }
    }

    /// Имя и тело рамки, приведённые в чувство одним местом — им пользуются и
    /// заведение, и запись.
    fn clean_studio_frame(
        req: crate::models::SaveStudioFrameRequest,
    ) -> Result<(String, serde_json::Value)> {
        let name: String = req.name.trim().chars().take(60).collect();
        if name.is_empty() {
            return Err(AppError::BadRequest("nameEmpty".into()));
        }
        // Присланное КЛАДЁТСЯ ПОВЕРХ домашней рамы, а не читается само по себе.
        //
        // Иначе рамка без одного поля — отказ, и старый клиент, не знающий про
        // поле, заведённое вчера, теряет работу целиком со словами «missing
        // field». Дом уже так говорит везде: пустое значит «как в доме».
        let tier = crate::battles::clamp_tier(
            req.body
                .get("tier")
                .and_then(|t| t.as_i64())
                .unwrap_or(1) as i16,
        );
        let fallback = crate::battles::default_frames()[(tier - 1) as usize].clone();
        let mut merged = serde_json::to_value(&fallback)
            .map_err(|_| AppError::Internal("Рамка дома не сериализуется".into()))?;
        if let (Some(base), Some(sent)) = (merged.as_object_mut(), req.body.as_object()) {
            for (key, value) in sent {
                base.insert(key.clone(), value.clone());
            }
        }
        let frame: crate::battles::BattleFrame = serde_json::from_value(merged)
            .map_err(|e| AppError::BadRequest(format!("frameUnreadable: {e}")))?;
        let clean = crate::battles::normalize_frame(frame, fallback);
        let body = serde_json::to_value(&clean)
            .map_err(|_| AppError::Internal("Рамка не сериализуется".into()))?;
        Ok((name, body))
    }

    /// Принять соглашение автора. Записывается редакцией: «согласен» без
    /// указания, с чем именно, через год не значит ничего.
    pub async fn accept_studio_agreement(&self, user_id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        self.repo
            .accept_studio_agreement(user_id, crate::studio::AGREEMENT)
            .await
    }

    /// Выложить рамку на допуск хозяина.
    ///
    /// Требует согласия НЫНЕШНЕЙ редакции: выкладка — это первый раз, когда
    /// работу увидит кто-то кроме автора, и спрашивать позже уже поздно.
    pub async fn publish_studio_frame(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<crate::models::StudioFrameDto> {
        self.studio_admit(user_id).await?;
        let agreed = self.repo.studio_agreement(user_id).await?;
        if agreed.as_deref() != Some(crate::studio::AGREEMENT) {
            return Err(AppError::BadRequest("needsAgreement".into()));
        }
        match self
            .repo
            .set_studio_frame_status(user_id, id, "shown")
            .await?
        {
            Some(frame) => {
                Self::log_domain_event("studio_frame_published", "studio_frame", frame.id, "ok");
                Ok(frame)
            }
            None => Err(AppError::NotFound("studio frame".into())),
        }
    }

    // ── Листы студии ────────────────────────────────────────────────────────
    //
    // Режет их тот же `sheet.rs`, что и листы дома: один алгоритм, одни числа,
    // и деталь, вырезанная гостем, ничем не отличается от вырезанной хозяином.
    // Разное здесь только владение и квоты.

    /// Принести лист. Хранится ЦЕЛИКОМ, в тех байтах, в которых пришёл: разрез
    /// можно повторить с другими настройками, не спрашивая файл заново.
    pub async fn add_studio_sheet(
        &self,
        user_id: Uuid,
        name: &str,
        bytes: Vec<u8>,
    ) -> Result<crate::models::StudioSheetDto> {
        let settings = self.studio_admit(user_id).await?;
        let (now, today) = self.repo.count_studio_sheets(user_id).await?;
        if now >= settings.sheets_at_once as i64 {
            return Err(AppError::BadRequest("tooManySheets".into()));
        }
        if today >= settings.sheets_per_day as i64 {
            return Err(AppError::BadRequest("sheetsToday".into()));
        }
        self.studio_box_room(user_id, bytes.len(), &settings).await?;

        // Формат читается из БАЙТОВ, а не из имени файла: расширение — это
        // утверждение, а решает оно, куда файл ляжет.
        let format = image::guess_format(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Unrecognised image: {e}")))?;
        let extension = format.extensions_str().first().copied().unwrap_or("bin");
        let probe = image::load_from_memory(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid image file: {e}")))?;
        let (width, height) = (probe.width(), probe.height());
        if width as u64 * height as u64 > crate::sheet::SHEET_MAX_PIXELS {
            return Err(AppError::BadRequest("sheetTooLarge".into()));
        }
        drop(probe);

        let relative = format!("studio/{}.{}", Uuid::new_v4(), extension);
        self.write_upload(&relative, &bytes).await?;
        self.repo
            .insert_studio_sheet(
                user_id,
                &Self::clean_asset_name(name, "Лист"),
                &format!("/static/{}", relative),
                width as i32,
                height as i32,
                bytes.len() as i32,
            )
            .await
    }

    pub async fn remove_studio_sheet(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        match self.repo.delete_studio_sheet(user_id, id).await? {
            Some(url) => {
                self.remove_upload(&url).await;
                Ok(())
            }
            None => Err(AppError::NotFound("studio sheet".into())),
        }
    }

    /// Байты листа этого человека. Отдельной функцией, потому что их просят
    /// трижды: предложить разрез, вырезать отобранное и показать один кусок
    /// крупно.
    async fn studio_sheet_bytes(&self, user_id: Uuid, id: Uuid) -> Result<Vec<u8>> {
        let sheet = self
            .repo
            .get_studio_sheet(user_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound("studio sheet".into()))?;
        let path = Path::new(&self.config.upload_dir)
            .join(sheet.url.trim_start_matches("/static/"));
        tokio::fs::read(&path)
            .await
            .map_err(|e| AppError::NotFound(format!("The sheet's file is gone: {e}")))
    }

    async fn cut_studio_sheet(
        &self,
        user_id: Uuid,
        id: Uuid,
        settings: crate::sheet::SliceSettings,
    ) -> Result<crate::sheet::Sliced> {
        let bytes = self.studio_sheet_bytes(user_id, id).await?;
        tokio::task::spawn_blocking(move || crate::sheet::slice(&bytes, &settings))
            .await
            .map_err(|e| AppError::Internal(format!("Sheet cutting task failed: {e}")))?
            .map_err(|e| AppError::BadRequest(e.to_string()))
    }

    /// Предложить разрез. Ничего не пишет: перечитывать можно сколько угодно.
    pub async fn slice_studio_sheet(
        &self,
        user_id: Uuid,
        id: Uuid,
        settings: crate::sheet::SliceSettings,
    ) -> Result<BattleSheetCutDto> {
        self.studio_admit(user_id).await?;
        let cut = self.cut_studio_sheet(user_id, id, settings.clone()).await?;
        Ok(Self::sliced_dto(cut, settings))
    }

    /// Сохранить отобранное. Разрез повторяется, а не берётся из предложения:
    /// сохранённое предложение было бы вторым источником правды о том, что
    /// человек видел. Каждый отбор несёт размер, при котором на него смотрели,
    /// и несовпадение ловится, а не сохраняется молча под чужим именем.
    pub async fn keep_studio_cut(
        &self,
        user_id: Uuid,
        id: Uuid,
        settings: crate::sheet::SliceSettings,
        picks: Vec<BattleAssetPick>,
    ) -> Result<Vec<crate::models::StudioAssetDto>> {
        self.studio_admit(user_id).await?;
        if picks.is_empty() {
            return Err(AppError::BadRequest("nothingChosen".into()));
        }
        let cut = self.cut_studio_sheet(user_id, id, settings).await?;
        let mut saved = Vec::with_capacity(picks.len());
        for pick in picks.iter() {
            let part = cut
                .parts
                .iter()
                .find(|p| p.index == pick.index)
                .ok_or_else(|| AppError::BadRequest(format!("noPart:{}", pick.index)))?;
            if part.width != pick.width || part.height != pick.height {
                return Err(AppError::BadRequest("cutMoved".into()));
            }
            let name = Self::clean_asset_name(
                pick.name.as_deref().unwrap_or(""),
                &format!("{:02}", pick.index),
            );
            let role = crate::studio::clamp_asset_role(
                pick.role.as_deref().unwrap_or(Self::role_name(part.role)),
            );
            let rec = self
                .keep_studio_image(user_id, &name, &role, &part.image)
                .await?;
            self.repo.attach_studio_asset_to_sheet(rec.id, id).await?;
            saved.push(rec);

            // И то, что человек обвёл на куске сам. Кусок при этом остаётся:
            // рамки — добавление, а не замена.
            if pick.rects.len() > Self::SPLIT_RECTS_MAX {
                return Err(AppError::BadRequest("tooManyRects".into()));
            }
            for (n, rect) in pick.rects.iter().enumerate() {
                let cropped = crate::sheet::crop_to_content(
                    &part.image,
                    &rect.frame(),
                    Self::SPLIT_ALPHA_FLOOR,
                )
                .ok_or_else(|| AppError::BadRequest("emptyRect".into()))?;
                let piece = self
                    .keep_studio_image(user_id, &format!("{name}·{}", n + 1), &role, &cropped)
                    .await?;
                self.repo.attach_studio_asset_to_sheet(piece.id, id).await?;
                saved.push(piece);
            }
        }
        self.repo.mark_studio_sheet_harvested(id).await?;
        Ok(saved)
    }

    /// Один кусок предложения в полный рост — для разделочной доски. Превью в
    /// сетке нарочно мелкие, их восемьдесят в одном ответе, и обводить по ним
    /// нельзя.
    pub async fn studio_sheet_part(
        &self,
        user_id: Uuid,
        id: Uuid,
        settings: crate::sheet::SliceSettings,
        index: u32,
    ) -> Result<BattleSheetPartFullDto> {
        self.studio_admit(user_id).await?;
        let cut = self.cut_studio_sheet(user_id, id, settings).await?;
        let part = cut
            .parts
            .iter()
            .find(|p| p.index == index)
            .ok_or_else(|| AppError::BadRequest(format!("noPart:{index}")))?;
        let webp = crate::sheet::to_webp(&part.image, 88.0);
        Ok(BattleSheetPartFullDto {
            index: part.index,
            width: part.width,
            height: part.height,
            image: format!(
                "data:image/webp;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&webp)
            ),
        })
    }

    /// Разрезать уже сохранённую деталь по обведённым рамкам. Исходная
    /// остаётся: склеенная деталь иногда нужна целиком.
    pub async fn split_studio_asset(
        &self,
        user_id: Uuid,
        id: Uuid,
        rects: Vec<crate::models::BattleAssetSplitRect>,
    ) -> Result<Vec<crate::models::StudioAssetDto>> {
        self.studio_admit(user_id).await?;
        if rects.is_empty() {
            return Err(AppError::BadRequest("nothingChosen".into()));
        }
        if rects.len() > Self::SPLIT_RECTS_MAX {
            return Err(AppError::BadRequest("tooManyRects".into()));
        }
        let asset = self
            .repo
            .get_studio_asset(user_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound("studio asset".into()))?;
        let path = Path::new(&self.config.upload_dir)
            .join(asset.url.trim_start_matches("/static/"));
        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|e| AppError::NotFound(format!("The piece's file is gone: {e}")))?;
        let whole = image::load_from_memory(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid image: {e}")))?
            .to_rgba8();
        let mut saved = Vec::with_capacity(rects.len());
        for (n, rect) in rects.iter().enumerate() {
            let cropped =
                crate::sheet::crop_to_content(&whole, &rect.frame(), Self::SPLIT_ALPHA_FLOOR)
                    .ok_or_else(|| AppError::BadRequest("emptyRect".into()))?;
            saved.push(
                self.keep_studio_image(
                    user_id,
                    &format!("{}·{}", asset.name, n + 1),
                    &asset.role,
                    &cropped,
                )
                .await?,
            );
        }
        Ok(saved)
    }

    /// Положить вырезанную картинку в ящик. Место проверяется перед записью,
    /// как и у обычной загрузки.
    async fn keep_studio_image(
        &self,
        user_id: Uuid,
        name: &str,
        role: &str,
        image: &image::RgbaImage,
    ) -> Result<crate::models::StudioAssetDto> {
        let webp = crate::sheet::to_webp(image, 88.0);
        self.studio_room_for(user_id, webp.len()).await?;
        let relative = format!("studio/{}.webp", Uuid::new_v4());
        self.write_upload(&relative, &webp).await?;
        self.add_studio_asset(
            user_id,
            name,
            role,
            &format!("/static/{}", relative),
            image.width() as i32,
            image.height() as i32,
            webp.len() as i32,
        )
        .await
    }

    pub async fn rename_studio_asset(
        &self,
        user_id: Uuid,
        id: Uuid,
        name: &str,
        role: &str,
    ) -> Result<()> {
        self.studio_admit(user_id).await?;
        self.repo
            .rename_studio_asset(
                user_id,
                id,
                &Self::clean_asset_name(name, "деталь"),
                &crate::studio::clamp_asset_role(role),
            )
            .await
    }

    // ── Сезон ───────────────────────────────────────────────────────────────
    //
    // Неделя по часам дома: понедельник открывает, суббота закрывает, в ночь на
    // воскресенье — вердикт. Границы считаются от часов битв (`BattleClock`),
    // а не от UTC: «сегодня» у дома уже определено, и второе определение
    // однажды разошлось бы с первым на час.

    /// Идущий сезон. Заводится сам при первом обращении — руками открывать
    /// неделю значит однажды забыть.
    pub async fn studio_season_now(&self) -> Result<crate::models::StudioSeasonDto> {
        let offset = chrono::Duration::minutes(self.get_battle_clock().await?.offset_min as i64);
        let now_here = chrono::Utc::now() + offset;
        let monday = (now_here.date_naive()
            - chrono::Duration::days(
                now_here.date_naive().weekday().num_days_from_monday() as i64
            ))
        .and_hms_opt(0, 0, 0)
        .unwrap_or_default();
        let opens = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(monday, chrono::Utc)
            - offset;
        let closes = opens + chrono::Duration::days(6);
        let verdict = opens + chrono::Duration::days(6) + chrono::Duration::minutes(5);

        if let Some(found) = self.repo.current_studio_season().await? {
            if found.opens_at == opens {
                return Ok(found);
            }
            // Новая неделя: следующий номер по порядку.
            return self
                .repo
                .insert_studio_season(found.number + 1, opens, closes, verdict)
                .await;
        }
        self.repo.insert_studio_season(1, opens, closes, verdict).await
    }

    /// Страница сезона. Без имени тоже: смотреть можно всякому, оценивать — нет.
    pub async fn studio_season_page(
        &self,
        number: Option<i32>,
        viewer: Option<Uuid>,
    ) -> Result<crate::models::StudioSeasonPageDto> {
        let season = match number {
            Some(n) => self
                .repo
                .studio_season_by_number(n)
                .await?
                .ok_or_else(|| AppError::NotFound("season".into()))?,
            None => self.studio_season_now().await?,
        };
        let entries = self.repo.list_studio_entries(season.id, viewer).await?;
        let (ratings_left, entered) = match viewer {
            Some(who) => {
                let used = self.repo.count_studio_ratings(season.id, who).await?;
                let rules = self.get_studio_settings().await?;
                (
                    (rules.ratings_per_season - used).max(0),
                    entries.iter().any(|e| e.author_id == who),
                )
            }
            None => (0, false),
        };
        Ok(crate::models::StudioSeasonPageDto {
            season,
            // Автор, выставившийся сам, соперников не оценивает: иначе первое,
            // что сделает каждый, — поставит всем по единице.
            may_rate: viewer.is_some() && !entered && ratings_left > 0,
            entries,
            ratings_left,
            entered,
        })
    }

    /// Выставить раму в сезон. Только допущенную: до слова хозяина работу не
    /// видит никто, кроме автора.
    pub async fn enter_studio_season(&self, user_id: Uuid, frame_id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        let season = self.studio_season_now().await?;
        if season.state != "open" {
            return Err(AppError::BadRequest("seasonClosed".into()));
        }
        let (status, used, admitted) = self
            .repo
            .studio_frame_ready(user_id, frame_id)
            .await?
            .ok_or_else(|| AppError::NotFound("studio frame".into()))?;
        // Само правило — в `studio::why_not_in_season`, и там же оно
        // проверяется. Пока оно жило здесь тремя `if`, проверить его было
        // нечем: любая проверка требовала базы, сессии и открытого сезона —
        // и тупик, из-за которого работа не выставлялась второй раз, сидел
        // ровно в них.
        if let Some(why) = crate::studio::why_not_in_season(
            &status,
            admitted,
            used,
            crate::studio::ENTRIES_PER_FRAME,
        ) {
            return Err(AppError::BadRequest(why.into()));
        }
        self.repo
            .insert_studio_entry(season.id, frame_id, user_id)
            .await
            .map_err(|e| match e {
                // Одна заявка от человека в сезон — правило схемы, и отказ
                // приходит оттуда же.
                AppError::Database(ref db) if db.to_string().contains("studio_entries_one_per") => {
                    AppError::BadRequest("alreadyEntered".into())
                }
                other => other,
            })?;
        self.repo.mark_studio_frame_entered(frame_id).await?;
        Ok(())
    }

    /// Оценить работу. Вес голоса пишется В МОМЕНТ ГОЛОСА и больше не меняется.
    pub async fn rate_studio_entry(
        &self,
        user_id: Uuid,
        entry: Uuid,
        value: i16,
    ) -> Result<()> {
        self.studio_admit(user_id).await?;
        if !(1..=5).contains(&value) {
            return Err(AppError::BadRequest("badValue".into()));
        }
        let (author, season_id) = self
            .repo
            .studio_entry_author(entry)
            .await?
            .ok_or_else(|| AppError::NotFound("entry".into()))?;
        if author == user_id {
            return Err(AppError::BadRequest("ownWork".into()));
        }
        let season = self.studio_season_now().await?;
        if season.id != season_id || season.state != "open" {
            return Err(AppError::BadRequest("seasonClosed".into()));
        }
        // Автор, выставившийся в этом сезоне, соперников не оценивает.
        let entries = self
            .repo
            .list_studio_entries(season.id, Some(user_id))
            .await?;
        if entries.iter().any(|e| e.author_id == user_id) {
            return Err(AppError::BadRequest("authorMayNotRate".into()));
        }
        // Переоценка своей же оценки потолка не тратит: потолок про то,
        // сколько работ человек посмотрел, а не сколько раз передумал.
        let used = self.repo.count_studio_ratings(season.id, user_id).await?;
        let fresh = !entries
            .iter()
            .any(|e| e.id == entry && e.mine.is_some());
        let rules = self.get_studio_settings().await?;
        if fresh && used >= rules.ratings_per_season {
            return Err(AppError::BadRequest("noRatingsLeft".into()));
        }
        let (aged, owns, played) = self.repo.voter_standing(user_id).await?;
        let weight = crate::studio::voter_weight(aged, owns, played);
        self.repo.put_studio_rating(entry, user_id, value, weight).await
    }

    /// Пожаловаться. Без имени тоже: тот, кто увидел чужой арт, может быть и
    /// не здешним.
    pub async fn report_studio_frame(
        &self,
        who: Option<Uuid>,
        frame: Uuid,
        reason: &str,
        note: Option<&str>,
    ) -> Result<()> {
        let reason: String = reason.trim().chars().take(40).collect();
        if reason.is_empty() {
            return Err(AppError::BadRequest("reasonEmpty".into()));
        }
        let note: Option<String> = note
            .map(|n| n.trim().chars().take(600).collect::<String>())
            .filter(|n| !n.is_empty());
        self.repo
            .insert_studio_report(frame, who, &reason, note.as_deref())
            .await
    }

    /// Вердикт: посчитать сезон и раздать плату.
    ///
    /// ИДЕМПОТЕНТЕН по построению. Состояние `judged` ставится ПЕРВЫМ, одним
    /// запросом с условием, и если его поставил кто-то другой — второй заход
    /// просто уходит. Перезапуск сервера в воскресенье не подводит вердикт
    /// дважды и не платит дважды; плата вдобавок идёт по `idem_key`.
    pub async fn judge_studio_season(&self, season: Uuid) -> Result<usize> {
        if !self.repo.set_studio_season_state(season, "judged").await? {
            return Ok(0);
        }
        // Пороги берутся из настроек, а не из констант: первые сезоны идут при
        // десятке участников, и порог живого дома тогда не пропустит никого.
        let rules = self.get_studio_settings().await?;
        let mut rows = self.repo.studio_season_tallies(season).await?;
        // Ничья — вперёд тот, кто выложил раньше.
        rows.sort_by(|a, b| {
            let sa = crate::studio::bayes_score(a.2, a.3);
            let sb = crate::studio::bayes_score(b.2, b.3);
            sb.partial_cmp(&sa)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.5.cmp(&b.5))
        });
        let mut to_keeper = 0usize;
        let mut sum_scores = 0.0;
        let mut counted = 0usize;
        for (_, _, sum, weight, votes, _) in rows.iter() {
            if *votes > 0 {
                sum_scores += crate::studio::bayes_score(*sum, *weight);
                counted += 1;
            }
        }
        let average = if counted > 0 { sum_scores / counted as f64 } else { 0.0 };

        for (place, (entry, author, sum, weight, votes, _)) in rows.iter().enumerate() {
            let score = crate::studio::bayes_score(*sum, *weight);
            // В тройку — только с порогом доверия: иначе её займут те, кого
            // никто не смотрел.
            let trusted = *votes >= rules.ratings_trusted;
            let chosen = trusted && (to_keeper as i64) < rules.to_keeper;
            if chosen {
                to_keeper += 1;
            }
            self.repo
                .set_studio_entry_result(
                    *entry,
                    (score * 1000.0).round() / 1000.0,
                    *votes,
                    *weight,
                    (place + 1) as i16,
                    chosen,
                )
                .await?;
            // Добавка — тем, чей итог выше среднего ПО ОЦЕНЁННЫМ. Работа, на
            // которую никто не посмотрел, среднего не превосходит: её баллом
            // Байес поставил ровную середину, а среднее считалось по тем, кого
            // смотрели, — и без этого условия добавку получали все, включая
            // неделю, где не голосовал никто.
            let liked = *votes > 0 && score > average;
            self.pay_for_season(season, *author, liked).await?;
            if chosen {
                // Про тройку человек узнаёт от дома, а не подсчётом мест на
                // странице сезона.
                let frame = self.repo.studio_entry_frame(*entry).await?;
                if let Some(frame) = frame {
                    self.note_about_frame(frame, "chosen", None).await?;
                }
            }
        }
        Self::log_domain_event("studio_season_judged", "studio_season", season, "ok");
        Ok(to_keeper)
    }

    /// Плата за сезон. Платят НЕ ТОЛЬКО тройке: из двадцати авторов побеждают
    /// трое, и без платы остальным семнадцать второго сезона не будет.
    ///
    /// `idem_key` держит это в узде: пересчёт вердикта — если он когда-нибудь
    /// случится руками — не заплатит второй раз.
    async fn pay_for_season(&self, season: Uuid, author: Uuid, above: bool) -> Result<()> {
        self.repo
            .credit_battle_wallet(
                author,
                "dust",
                crate::studio::PAY_ENTERED,
                "studio_entered",
                Some(season),
                &format!("season:{season}:entered"),
            )
            .await?;
        if above {
            self.repo
                .credit_battle_wallet(
                    author,
                    "dust",
                    crate::studio::PAY_LIKED,
                    "studio_liked",
                    Some(season),
                    &format!("season:{season}:liked"),
                )
                .await?;
        }
        Ok(())
    }

    /// Подвести все сезоны, которым пора. Список, а не один: сервер мог
    /// простоять выходные, и неподведённых окажется несколько.
    pub async fn judge_due_studio_seasons(&self) -> Result<usize> {
        let mut done = 0;
        for season in self.repo.studio_seasons_due().await? {
            done += self.judge_studio_season(season.id).await?;
        }
        Ok(done)
    }

    // ── Допуск ──────────────────────────────────────────────────────────────

    pub async fn list_studio_admissions(&self) -> Result<Vec<crate::models::StudioFrameDto>> {
        self.repo.list_studio_admissions().await
    }

    pub async fn admit_studio_frame(&self, id: Uuid, word: Option<&str>) -> Result<()> {
        let word: Option<String> = word
            .map(|w| w.trim().chars().take(600).collect::<String>())
            .filter(|w| !w.is_empty());
        if self.repo.admit_studio_frame(id, word.as_deref()).await? {
            // Адрес автора заводится здесь: с этой минуты его работу видят люди.
            self.give_studio_slug(id).await?;
            // И работа СРАЗУ встаёт в идущий сезон.
            //
            // Раньше это было вторым действием автора, и получалась дурная
            // развилка: хозяин допустил — и ничего не произошло, потому что
            // человек об этом ещё не знает. Показать работу хозяину и значит
            // «хочу на люди»; спрашивать второй раз то же самое незачем.
            //
            // Отказ здесь — обычное дело, а не поломка: у автора уже может быть
            // заявка на этой неделе, или неделя подведена. Тогда работа просто
            // остаётся в галерее, и записка говорит об этом словом.
            let entered = self.enter_studio_season(self.frame_owner(id).await?, id).await.is_ok();
            self.note_about_frame(id, if entered { "admitted" } else { "admittedOnly" }, None)
                .await?;
            Self::log_domain_event("studio_frame_admitted", "studio_frame", id, "ok");
            Ok(())
        } else {
            Err(AppError::NotFound("studio frame".into()))
        }
    }

    /// Не допустить — со словом. Пустое слово не принимается: отказ, которого
    /// нельзя исправить, вернётся той же работой на следующей неделе.
    pub async fn deny_studio_frame(&self, id: Uuid, word: &str) -> Result<()> {
        let word: String = word.trim().chars().take(600).collect();
        if word.is_empty() {
            return Err(AppError::BadRequest("wordEmpty".into()));
        }
        if self.repo.deny_studio_frame(id, &word).await? {
            self.note_about_frame(id, "denied", Some(&word)).await?;
            Self::log_domain_event("studio_frame_denied", "studio_frame", id, "ok");
            Ok(())
        } else {
            Err(AppError::NotFound("studio frame".into()))
        }
    }

    /// Уборка склада студии.
    ///
    /// Два вида мусора, и они разные. ЛИСТ, из которого ничего не вырезали, —
    /// самый тяжёлый (2.6 МБ против 12 КБ у детали) и копится сам: принесли,
    /// посмотрели, ушли. ДЕТАЛЬ-СИРОТА — та, что не стоит ни в одной раме:
    /// её заменили другой, а прежняя осталась лежать.
    ///
    /// Сроки разные и не от вкуса: лист нужен, пока идёт разрез, — это часы;
    /// деталь может ждать своей рамы неделями, и трогать её раньше значит
    /// выбрасывать работу.
    pub async fn sweep_studio_store(&self) -> Result<(usize, usize)> {
        let settings = self.get_studio_settings().await?;
        let mut sheets = 0;
        for (id, url) in self.repo.studio_sheets_to_sweep(settings.sheet_days).await? {
            self.remove_upload(&url).await;
            self.repo.drop_studio_sheet(id).await?;
            sheets += 1;
        }
        let mut assets = 0;
        for (id, url) in self.repo.studio_assets_to_sweep(crate::studio::ORPHAN_DAYS).await? {
            self.remove_upload(&url).await;
            self.repo.drop_studio_asset(id).await?;
            assets += 1;
        }
        Ok((sheets, assets))
    }

    // ── Свои карты ──────────────────────────────────────────────────────────
    //
    // `STUDIO-CARDS.md`. Комната одна, поэтому и ворота одни: своего замка
    // карты не получают.

    pub async fn studio_cards(&self, user_id: Uuid) -> Result<Vec<crate::models::StudioCardDto>> {
        self.studio_admit(user_id).await?;
        self.repo.list_studio_cards(user_id).await
    }

    /// Тело работы, приведённое в чувство: домовое вырезано, содержимое
    /// нормализовано ТЕМИ ЖЕ зажимами, какими его нормализует сохранение карты
    /// хозяином. Без этого человек хранил бы у себя 99 силы, весы считали бы по
    /// зажатому, и два числа на одной странице говорили бы разное.
    fn clean_studio_card(body: &serde_json::Value) -> Result<serde_json::Value> {
        let kept = crate::studio::guest_card_json(body);
        // Читается КАК ЗАПРОС КАРТЫ, а не как свободный JSON: то же чтение, тот
        // же отказ, те же умолчания — иначе у работы завёлся бы второй разбор.
        let req: SaveBattleCardRequest = serde_json::from_value(kept)
            .map_err(|e| AppError::BadRequest(format!("cardUnreadable: {e}")))?;
        serde_json::to_value(&req)
            .map_err(|_| AppError::Internal("Карта не сериализуется".into()))
    }

    pub async fn save_studio_card(
        &self,
        user_id: Uuid,
        id: Option<Uuid>,
        body: &serde_json::Value,
        lang: &str,
    ) -> Result<crate::models::StudioCardDto> {
        let settings = self.studio_admit(user_id).await?;
        let clean = Self::clean_studio_card(body)?;
        let lang = crate::lang::Lang::parse(lang).unwrap_or_default().code();
        match id {
            None => {
                // Потолок незаконченных — СВОИМ числом, а не рамочным: карта
                // весит не то, что рама, и общая ручка однажды подвинула бы
                // обе разом.
                let open = self.repo.count_studio_cards_open(user_id).await?;
                if open >= settings.cards_open as i64 {
                    return Err(AppError::BadRequest("tooManyCards".into()));
                }
                self.repo.insert_studio_card(user_id, &clean, lang).await
            }
            Some(id) => self
                .repo
                .update_studio_card(user_id, id, &clean)
                .await?
                // Либо чужая, либо уже отдана хозяину: для этого человека и то
                // и другое выглядит одинаково — править нечего.
                .ok_or_else(|| AppError::NotFound("studio card".into())),
        }
    }

    /// Отдать работу хозяину.
    ///
    /// Только с пустым `blocking`, и разбор берётся ТОТ ЖЕ, которым отказывает
    /// публикация карты (`card_readiness`): иначе очередь хозяина заполнялась
    /// бы заведомо негодным, а объясняться словами пришлось бы ему.
    pub async fn show_studio_card(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        // Соглашение автора — то же, что у рамы, и по той же причине: это
        // единственное, чем человек разрешает дому пользоваться его работой.
        // У карт его не спрашивали вовсе, и карта уходила на полку, в продажу
        // и в чужие руки без единого слова о правах.
        let agreed = self.repo.studio_agreement(user_id).await?;
        if agreed.as_deref() != Some(crate::studio::AGREEMENT) {
            return Err(AppError::BadRequest("needsAgreement".into()));
        }
        let card = self
            .repo
            .get_studio_card(user_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound("studio card".into()))?;
        // Годность спрашивается у ОПУБЛИКОВАННОЙ карты: черновику дом прощает
        // почти всё, а на полку встанет именно опубликованная.
        let mut req: SaveBattleCardRequest = serde_json::from_value(card.body.clone())
            .map_err(|e| AppError::BadRequest(format!("cardUnreadable: {e}")))?;
        req.status = "published".into();
        let weighed = Self::weigh_studio_card(&req);
        if !weighed.readiness.blocking.is_empty() {
            return Err(AppError::BadRequest(format!(
                "cardNotReady: {}",
                weighed.readiness.blocking.join(",")
            )));
        }
        if self
            .repo
            .set_studio_card_status(user_id, id, "shown", crate::studio::WORK_MAY_SHOW_FROM)
            .await?
        {
            Self::log_domain_event("studio_card_shown", "studio_card", id, "ok");
            Ok(())
        } else {
            Err(AppError::NotFound("studio card".into()))
        }
    }

    /// Снять с людских глаз. Работа возвращается правимой — это и есть способ
    /// починить и показать снова.
    pub async fn withdraw_studio_card(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        if self
            .repo
            .set_studio_card_status(user_id, id, "withdrawn", crate::studio::WORK_MAY_WITHDRAW_FROM)
            .await?
        {
            Ok(())
        } else {
            Err(AppError::NotFound("studio card".into()))
        }
    }

    pub async fn delete_studio_card(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        if self.repo.delete_studio_card(user_id, id).await? {
            Ok(())
        } else {
            Err(AppError::NotFound("studio card".into()))
        }
    }

    // ── Свои движения ───────────────────────────────────────────────────────
    //
    // Движение человека — сочетание готовых жестов, и это то же самое, что
    // способность из готовых глаголов: движок не умеет ничего сверх списка, и
    // принести новое ПРАВИЛО через движение нельзя.

    pub async fn studio_motions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::StudioMotionDto>> {
        self.studio_admit(user_id).await?;
        self.repo.list_studio_motions(user_id).await
    }

    /// Сохранить ящик человека целиком.
    ///
    /// Тела проходят ТЕ ЖЕ зажимы, что и свод дома (`normalize_motions`): второй
    /// разбор движения разошёлся бы с первым на первом же новом поле жеста, и
    /// разошёлся бы молча — на столе сложилось, в комнате не сыграло.
    pub async fn save_studio_motions(
        &self,
        user_id: Uuid,
        req: &crate::models::SaveStudioMotionsRequest,
    ) -> Result<Vec<crate::models::StudioMotionDto>> {
        self.studio_admit(user_id).await?;
        let sent: Vec<crate::battles::Motion> = req
            .motions
            .iter()
            .cloned()
            .map(serde_json::from_value)
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| AppError::BadRequest(format!("motionUnreadable: {e}")))?;
        let clean = crate::battles::normalize_motions(sent);
        if clean.len() > crate::studio::MOTIONS_OPEN {
            return Err(AppError::BadRequest("tooManyMotions".into()));
        }
        let mut keep: Vec<String> = Vec::with_capacity(clean.len());
        for one in &clean {
            // Автограф ставит студия при утверждении, а не человек у себя: имя,
            // приехавшее из тела, было бы чужой подписью на своей работе.
            let mut body = one.clone();
            body.credit = String::new();
            let json = serde_json::to_value(&body)
                .map_err(|_| AppError::Internal("Движение не сериализуется".into()))?;
            self.repo
                .upsert_studio_motion(user_id, &one.id, &json, &req.lang)
                .await?;
            keep.push(one.id.clone());
        }
        self.repo.drop_missing_studio_motions(user_id, &keep).await?;
        self.repo.list_studio_motions(user_id).await
    }

    pub async fn show_studio_motion(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        let agreed = self.repo.studio_agreement(user_id).await?;
        if agreed.as_deref() != Some(crate::studio::AGREEMENT) {
            return Err(AppError::BadRequest("needsAgreement".into()));
        }
        if self
            .repo
            .set_studio_motion_status(user_id, id, "shown", crate::studio::WORK_MAY_SHOW_FROM)
            .await?
        {
            Ok(())
        } else {
            Err(AppError::NotFound("studio motion".into()))
        }
    }

    pub async fn withdraw_studio_motion(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        if self
            .repo
            .set_studio_motion_status(user_id, id, "withdrawn", crate::studio::WORK_MAY_WITHDRAW_FROM)
            .await?
        {
            Ok(())
        } else {
            Err(AppError::NotFound("studio motion".into()))
        }
    }

    pub async fn studio_motion_queue(
        &self,
    ) -> Result<Vec<crate::models::StudioMotionQueueDto>> {
        self.repo.studio_motion_queue().await
    }

    pub async fn deny_studio_motion(&self, id: Uuid, word: &str) -> Result<()> {
        let word: String = word.trim().chars().take(600).collect();
        if word.is_empty() {
            return Err(AppError::BadRequest("wordEmpty".into()));
        }
        if self.repo.deny_studio_motion(id, &word).await? {
            Ok(())
        } else {
            Err(AppError::NotFound("studio motion".into()))
        }
    }

    /// Утвердить движение: оно встаёт в СВОД ДОМА с именем автора.
    ///
    /// Свод — одна строка настроек, и пишется он единственным писателем
    /// (`save_battle_motions`), тем же, каким его пишет стол хозяина: второй
    /// писатель однажды разошёлся бы с первым в зажимах, и в своде появилось бы
    /// движение, которое стол сохранить бы отказался.
    ///
    /// Имя движения в своде дома может быть занято чужим: тогда оно получает
    /// своё, а не затирает соседа — на имя показывают карты и расы.
    pub async fn approve_studio_motion(&self, id: Uuid) -> Result<String> {
        let (work, owner) = self
            .repo
            .get_studio_motion_any(id)
            .await?
            .ok_or_else(|| AppError::NotFound("studio motion".into()))?;
        if work.status == "taken" || work.approved_at.is_some() {
            return Err(AppError::BadRequest("motionAlreadyTaken".into()));
        }
        if work.status != "shown" {
            return Err(AppError::BadRequest("motionNotShown".into()));
        }
        let mut body: crate::battles::Motion = serde_json::from_value(work.body.clone())
            .map_err(|e| AppError::BadRequest(format!("motionUnreadable: {e}")))?;
        body.credit = self
            .repo
            .find_user_by_id(owner)
            .await?
            .map(|u| u.display_name)
            .unwrap_or_default();

        let house = self.get_battle_motions().await?;
        if house.motions.iter().any(|m| m.id == body.id) {
            body.id = format!("{}-{}", body.id, &id.to_string()[..8]);
        }
        let name = body.id.clone();

        if !self
            .repo
            .set_studio_motion_status(owner, id, "taken", crate::studio::WORK_MAY_TAKE_FROM)
            .await?
        {
            return Err(AppError::BadRequest("motionAlreadyTaken".into()));
        }
        let mut motions = house.motions;
        motions.push(body);
        if let Err(e) = self
            .save_battle_motions(crate::battles::BattleMotions { motions })
            .await
        {
            let _ = self
                .repo
                .set_studio_motion_status(owner, id, "shown", crate::studio::WORK_MAY_RETURN_FROM)
                .await;
            return Err(e);
        }
        self.repo.mark_studio_motion_approved(id, &name).await?;
        Self::log_domain_event("studio_motion_approved", "studio_motion", id, "ok");
        Ok(name)
    }

    // ── Свои роды ───────────────────────────────────────────────────────────
    //
    // Род — словарная строка, которую однажды наденут чужие карты. Взвешивать
    // в нём нечего: есть имя, слово о нём и значок, и решает хозяин глазами.

    pub async fn studio_races(&self, user_id: Uuid) -> Result<Vec<crate::models::StudioRaceDto>> {
        self.studio_admit(user_id).await?;
        self.repo.list_studio_races(user_id).await
    }

    pub async fn save_studio_race(
        &self,
        user_id: Uuid,
        id: Option<Uuid>,
        req: &crate::models::SaveStudioRaceRequest,
    ) -> Result<crate::models::StudioRaceDto> {
        let settings = self.studio_admit(user_id).await?;
        if req.name_en.trim().is_empty() || req.name_ru.trim().is_empty() {
            // Род зовут на обоих языках, как и карту: пустое английское имя
            // ставило бы кириллицу в шапку английской полки.
            return Err(AppError::BadRequest("nameEmpty".into()));
        }
        match id {
            None => {
                let open = self.repo.count_studio_races_open(user_id).await?;
                if open >= settings.cards_open as i64 {
                    return Err(AppError::BadRequest("tooManyRaces".into()));
                }
                self.repo.insert_studio_race(user_id, req).await
            }
            Some(id) => self
                .repo
                .update_studio_race(user_id, id, req)
                .await?
                .ok_or_else(|| AppError::NotFound("studio race".into())),
        }
    }

    pub async fn show_studio_race(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        let agreed = self.repo.studio_agreement(user_id).await?;
        if agreed.as_deref() != Some(crate::studio::AGREEMENT) {
            return Err(AppError::BadRequest("needsAgreement".into()));
        }
        if self
            .repo
            .set_studio_race_status(user_id, id, "shown", crate::studio::WORK_MAY_SHOW_FROM)
            .await?
        {
            Ok(())
        } else {
            Err(AppError::NotFound("studio race".into()))
        }
    }

    pub async fn withdraw_studio_race(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        if self
            .repo
            .set_studio_race_status(user_id, id, "withdrawn", crate::studio::WORK_MAY_WITHDRAW_FROM)
            .await?
        {
            Ok(())
        } else {
            Err(AppError::NotFound("studio race".into()))
        }
    }

    pub async fn delete_studio_race(&self, user_id: Uuid, id: Uuid) -> Result<()> {
        self.studio_admit(user_id).await?;
        if self.repo.delete_studio_race(user_id, id).await? {
            Ok(())
        } else {
            Err(AppError::NotFound("studio race".into()))
        }
    }

    pub async fn studio_race_queue(&self) -> Result<Vec<crate::models::StudioRaceQueueDto>> {
        self.repo.studio_race_queue().await
    }

    pub async fn deny_studio_race(&self, id: Uuid, word: &str) -> Result<()> {
        let word: String = word.trim().chars().take(600).collect();
        if word.is_empty() {
            return Err(AppError::BadRequest("wordEmpty".into()));
        }
        if self.repo.deny_studio_race(id, &word).await? {
            Ok(())
        } else {
            Err(AppError::NotFound("studio race".into()))
        }
    }

    /// Утвердить род: он становится строкой словаря дома.
    ///
    /// Замок и порядок — те же, что у карты: работа сперва запирается своим
    /// словом `taken`, и только потом пишется словарная строка единственным
    /// писателем `battle_races`.
    pub async fn approve_studio_race(
        &self,
        id: Uuid,
        add: crate::models::ApproveStudioRaceRequest,
    ) -> Result<crate::models::BattleRaceDto> {
        let (race, owner, _) = self
            .repo
            .get_studio_race_any(id)
            .await?
            .ok_or_else(|| AppError::NotFound("studio race".into()))?;
        if race.status == "taken" || race.approved_at.is_some() {
            return Err(AppError::BadRequest("raceAlreadyTaken".into()));
        }
        if race.status != "shown" {
            return Err(AppError::BadRequest("raceNotShown".into()));
        }
        let credit = match add.credit_name.as_deref().map(str::trim) {
            Some(word) if !word.is_empty() => word.to_string(),
            _ => self
                .repo
                .find_user_by_id(owner)
                .await?
                .map(|u| u.display_name)
                .unwrap_or_default(),
        };
        if !self
            .repo
            .set_studio_race_status(owner, id, "taken", crate::studio::WORK_MAY_TAKE_FROM)
            .await?
        {
            return Err(AppError::BadRequest("raceAlreadyTaken".into()));
        }

        let made = match self
            .admin_create_battle_race(SaveBattleRaceRequest {
                slug: add.slug.clone(),
                name_en: race.name_en.clone(),
                name_ru: race.name_ru.clone(),
                name_uk: race.name_uk.clone(),
                name_es: race.name_es.clone(),
                name_de: race.name_de.clone(),
                name_it: race.name_it.clone(),
                note_en: race.note_en.clone(),
                note_ru: race.note_ru.clone(),
                note_uk: race.note_uk.clone(),
                note_es: race.note_es.clone(),
                note_de: race.note_de.clone(),
                note_it: race.note_it.clone(),
                icon_url: race.icon_url.clone(),
                level_frames: None,
                motion_wear: None,
                race_damage: None,
            })
            .await
        {
            Ok(made) => made,
            Err(e) => {
                let _ = self
                    .repo
                    .set_studio_race_status(owner, id, "shown", crate::studio::WORK_MAY_RETURN_FROM)
                    .await;
                return Err(e);
            }
        };
        let race_id = Uuid::parse_str(&made.id)
            .map_err(|_| AppError::Internal("Род без имени".into()))?;
        self.repo.set_battle_race_credit(race_id, &credit).await?;
        self.repo.mark_studio_race_approved(id, race_id).await?;
        Self::log_domain_event("studio_race_approved", "studio_race", id, "ok");
        Ok(made)
    }

    // ── Стол хозяина: очередь карт ──────────────────────────────────────────

    pub async fn studio_card_queue(&self) -> Result<Vec<crate::models::StudioCardQueueDto>> {
        self.repo.studio_card_queue().await
    }

    /// Отказ — со словом. Пустое слово не принимается: отказ, которого нельзя
    /// исправить, вернётся той же работой.
    pub async fn deny_studio_card(&self, id: Uuid, word: &str) -> Result<()> {
        let word: String = word.trim().chars().take(600).collect();
        if word.is_empty() {
            return Err(AppError::BadRequest("wordEmpty".into()));
        }
        if self.repo.deny_studio_card(id, &word).await? {
            self.note_about_card(id, "denied", Some(&word)).await?;
            Self::log_domain_event("studio_card_denied", "studio_card", id, "ok");
            Ok(())
        } else {
            Err(AppError::NotFound("studio card".into()))
        }
    }

    /// Утвердить: работа человека становится картой дома.
    ///
    /// Порядок здесь — не удобство, а условие. Сначала ЗАМОК: отметка
    /// `approved_at` ставится по `approved_at IS NULL`, и второе нажатие
    /// уходит ни с чем ДО того, как будет записана вторая карта. Карта при
    /// этом пишется единственным писателем `battle_cards` — тем же
    /// `admin_create_battle_card`, каким её пишет стол хозяина: второй
    /// писатель однажды разошёлся бы с первым в нормализации, и в базе
    /// появилась бы карта, которую стол сохранить бы отказался.
    pub async fn approve_studio_card(
        &self,
        id: Uuid,
        add: crate::models::ApproveStudioCardRequest,
    ) -> Result<BattleCardDto> {
        let row = self
            .repo
            .get_studio_card_any(id)
            .await?
            .ok_or_else(|| AppError::NotFound("studio card".into()))?;
        // Взятое домом названо СВОИМ словом, и отказы поэтому честные: одна
        // причина — «уже взята», другая — «автор снял её со стола».
        if row.card.status == "taken" || row.card.approved_at.is_some() {
            return Err(AppError::BadRequest("cardAlreadyTaken".into()));
        }
        if row.card.status != "shown" {
            return Err(AppError::BadRequest("cardNotShown".into()));
        }

        // Второй раз вырезаем домовое: запись могла лечь до последней правки
        // списка, а утверждение — последний рубеж.
        let kept = crate::studio::guest_card_json(&row.card.body);
        let mut req: SaveBattleCardRequest = serde_json::from_value(kept)
            .map_err(|e| AppError::BadRequest(format!("cardUnreadable: {e}")))?;

        // Дописка хозяина — поверх, и только домовое.
        req.slug = add.slug.clone();
        req.status = add.status.clone();
        req.price_dust = add.price_dust;
        req.price_feed = add.price_feed;
        req.level_price_dust = add.level_price_dust.clone();
        req.edition_size = add.edition_size;
        req.lendable = add.lendable;
        req.frame_override = add.frame_override.clone();
        req.motion_wear = add.motion_wear.clone();
        if let Some(art) = add.art_url.clone() {
            req.art_url = Some(art);
        }
        if add.figurine_id.is_some() {
            req.figurine_id = add.figurine_id.clone();
        }
        if let Some(tier) = add.tier {
            req.tier = tier;
        }

        let author = row.owner_id;
        // Подпись: имя автора по умолчанию — это чужой труд, и молчать о нём
        // дом не станет.
        let credit = match add.credit_name.as_deref().map(str::trim) {
            Some(word) if !word.is_empty() => word.to_string(),
            _ => self
                .repo
                .find_user_by_id(author)
                .await?
                .map(|u| u.display_name)
                .unwrap_or_default(),
        };

        // ЗАМОК прежде записи: два нажатия на медленной сети не должны завести
        // двух карт. Отметка `approved_at` ставится позже (нужен `card_id`),
        // поэтому работа сперва запирается переводом в СВОЁ состояние —
        // `taken`, терминальное: из него нельзя ни править, ни показать снова.
        // Раньше замком служило чужое слово `withdrawn`, и утверждённая работа
        // возвращалась к автору правимой, а потом вечно висела в очереди
        // (`STUDIO-CARDS-REVIEW.md` П1).
        if !self
            .repo
            .set_studio_card_status(author, id, "taken", crate::studio::WORK_MAY_TAKE_FROM)
            .await?
        {
            return Err(AppError::BadRequest("cardAlreadyTaken".into()));
        }

        let made = match self.admin_create_battle_card_signed(req, Some(&credit)).await {
            Ok(made) => made,
            Err(e) => {
                // Карта не записалась — работа возвращается в очередь, а не
                // повисает взятой: хозяин ничего не решал, он получил отказ.
                let _ = self
                    .repo
                    .set_studio_card_status(author, id, "shown", crate::studio::WORK_MAY_RETURN_FROM)
                    .await;
                return Err(e);
            }
        };
        let card_id = Uuid::parse_str(&made.id)
            .map_err(|_| AppError::Internal("Карта без имени".into()))?;
        self.repo.mark_studio_card_approved(id, card_id).await?;

        // Первый экземпляр — автору, даром: тот, кто карту придумал, не
        // покупает её за пыль.
        self.repo.give_first_copy(card_id, author).await?;

        // Плата. Идемпотентная по работе, а не по времени: книга это уже умеет.
        let dust = add
            .reward_dust
            .unwrap_or(crate::studio::PAY_CARD_APPROVED)
            .clamp(0, 10_000);
        if dust > 0 {
            self.repo
                .credit_battle_wallet(
                    author,
                    "dust",
                    dust,
                    "card_approved",
                    Some(id),
                    &format!("card:{id}"),
                )
                .await?;
        }
        self.note_about_card(id, "approved", None).await?;
        Self::log_domain_event("studio_card_approved", "studio_card", id, "ok");
        Ok(made)
    }

    /// Записка автору о его карте. Язык — работы, а не хозяина.
    async fn note_about_card(&self, card: Uuid, kind: &str, word: Option<&str>) -> Result<()> {
        use crate::lang::Lang;
        let Some(row) = self.repo.get_studio_card_any(card).await? else {
            return Ok(());
        };
        let lang = Lang::parse(&row.lang).unwrap_or_default();
        let field = match lang {
            Lang::En => "titleEn",
            Lang::Ru => "titleRu",
            Lang::Uk => "titleUk",
            Lang::Es => "titleEs",
            Lang::De => "titleDe",
            Lang::It => "titleIt",
        };
        // Названа карта могла быть не на языке работы: тогда берётся любое
        // написанное, лишь бы записка не называла её пустотой.
        let named = |key: &str| {
            row.card
                .body
                .get(key)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        };
        let name = named(field)
            .or_else(|| named("titleEn"))
            .or_else(|| named("titleRu"))
            .or_else(|| named("titleUk"))
            .or_else(|| named("titleEs"))
            .or_else(|| named("titleDe"))
            .or_else(|| named("titleIt"))
            .unwrap_or("")
            .to_string();
        let word = word.unwrap_or("");
        let (subject, body) = match (kind, lang) {
            ("denied", Lang::Ru) => (
                "Карта возвращена",
                format!(
                    "«{name}» пока не выходит на полку. Хозяин пишет: {word}\n\nОна вернулась к вам в черновики — поправьте и покажите снова."
                ),
            ),
            ("denied", Lang::En) => (
                "Your card is back with you",
                format!(
                    "“{name}” is not going on the shelf yet. The keeper writes: {word}\n\nIt is back in your drafts — mend it and show it again."
                ),
            ),
            ("denied", Lang::Uk) => (
                "Карту повернуто",
                format!(
                    "«{name}» поки не виходить на полицю. Господар пише: {word}\n\nВона повернулася до вас у чернетки — поправте й покажіть знову."
                ),
            ),
            ("approved", Lang::Ru) => (
                "Карта взята в игру",
                format!("«{name}» стоит на полке с вашим именем, и первый её экземпляр — ваш."),
            ),
            ("approved", Lang::En) => (
                "Taken into the game",
                format!("“{name}” is on the shelf with your name on it, and the first copy is yours."),
            ),
            ("approved", Lang::Uk) => (
                "Карту взято в гру",
                format!("«{name}» стоїть на полиці з вашим ім’ям, і перший її примірник — ваш."),
            ),
            ("denied", Lang::Es) => (
                "Su carta ha vuelto a usted",
                format!(
                    "«{name}» todavía no sale a la estantería. La autora escribe: {word}\n\nHa vuelto a sus borradores: corríjala y preséntela de nuevo."
                ),
            ),
            ("denied", Lang::De) => (
                "Ihre Karte ist zurück bei Ihnen",
                format!(
                    "„{name}“ kommt noch nicht ins Regal. Die Autorin schreibt: {word}\n\nSie liegt wieder in Ihren Entwürfen: Überarbeiten Sie sie und reichen Sie sie erneut ein."
                ),
            ),
            ("denied", Lang::It) => (
                "La sua carta è tornata a lei",
                format!(
                    "«{name}» non va ancora sullo scaffale. L’autrice scrive: {word}\n\nÈ tornata nelle sue bozze: la corregga e la presenti di nuovo."
                ),
            ),
            ("approved", Lang::Es) => (
                "Su carta ha entrado en el juego",
                format!("«{name}» está en la estantería con su nombre, y el primer ejemplar es suyo."),
            ),
            ("approved", Lang::De) => (
                "Ihre Karte ist im Spiel",
                format!("„{name}“ steht mit Ihrem Namen im Regal, und das erste Exemplar gehört Ihnen."),
            ),
            ("approved", Lang::It) => (
                "La sua carta è entrata nel gioco",
                format!("«{name}» è sullo scaffale con il suo nome, e il primo esemplare è suo."),
            ),
            _ => return Ok(()),
        };
        let _ = self
            .repo
            .create_thread(row.owner_id, "studio", Some(card), subject, &body, true, &[])
            .await;
        Ok(())
    }

    // ── Записки автору ──────────────────────────────────────────────────────
    //
    // Записка в профиль, а не письмо: письмами дом пишет по делу — бронь,
    // заказ, — и решение о раме не стоит того, чтобы догонять человека почтой.
    // Он придёт сам: за этим сюда и ходят.
    //
    // Язык берётся у РАБОТЫ, а не угадывается: `studio_frames.lang` — это язык,
    // на котором человек работал, когда отдавал её. Тот же приём, что у
    // прошения.

    /// Чья это работа. Отдельной функцией, потому что спрашивается из двух мест.
    async fn frame_owner(&self, frame: Uuid) -> Result<Uuid> {
        self.repo
            .studio_frame_owner(frame)
            .await?
            .map(|(user, _, _, _, _)| user)
            .ok_or_else(|| AppError::NotFound("studio frame".into()))
    }

    async fn note_about_frame(&self, frame: Uuid, kind: &str, word: Option<&str>) -> Result<()> {
        use crate::lang::Lang;
        let Some((user, _, slug, lang, name)) = self.repo.studio_frame_owner(frame).await? else {
            return Ok(());
        };
        let lang = Lang::parse(&lang).unwrap_or_default();
        let word = word.unwrap_or("");
        let page = || {
            slug.as_deref()
                .map(|s| {
                    format!(
                        "\n\n{}: /studio/authors/{s}",
                        lang.pick(
                            "Your page",
                            "Ваша страница",
                            "Ваша сторінка",
                            "Su página",
                            "Ihre Seite",
                            "La sua pagina",
                        )
                    )
                })
                .unwrap_or_default()
        };
        let (subject, body) = match (kind, lang) {
            ("admitted", Lang::Ru) => (
                "Работа в сезоне",
                format!("«{name}» допущена и уже стоит в сезоне — её видно в галерее, и её оценивают."),
            ),
            ("admitted", Lang::En) => (
                "Your work is in the season",
                format!("“{name}” is admitted and already in the season — it is in the gallery, and people are marking it."),
            ),
            ("admitted", Lang::Uk) => (
                "Робота в сезоні",
                format!("«{name}» допущена й уже стоїть у сезоні — її видно в галереї, і її оцінюють."),
            ),
            // Допущена, но в сезон не встала: у автора уже есть заявка на этой
            // неделе, или неделя подведена. Это не отказ, и сказать надо так.
            ("admittedOnly", Lang::Ru) => (
                "Работа вышла на люди",
                format!(
                    "«{name}» допущена: её видно в галерее. В сезон она не встала — на этой неделе у вас уже есть работа. Выставить её можно в следующем сезоне."
                ),
            ),
            ("admittedOnly", Lang::En) => (
                "Your work is out",
                format!("“{name}” is admitted and is in the gallery. It did not go into the season — you already have a work in this one. Next week it can."),
            ),
            ("admittedOnly", Lang::Uk) => (
                "Робота вийшла на люди",
                format!(
                    "«{name}» допущена: її видно в галереї. У сезон вона не стала — цього тижня у вас уже є робота. Виставити її можна в наступному сезоні."
                ),
            ),
            ("denied", Lang::Ru) => (
                "Работа возвращена",
                format!(
                    "«{name}» пока не выходит на люди. Хозяин пишет: {word}\n\nОна вернулась к вам в черновики — поправьте и покажите снова."
                ),
            ),
            ("denied", Lang::En) => (
                "Your work is back with you",
                format!(
                    "“{name}” is not going out yet. The keeper writes: {word}\n\nIt is back in your drafts — mend it and show it again."
                ),
            ),
            ("denied", Lang::Uk) => (
                "Роботу повернуто",
                format!(
                    "«{name}» поки не виходить на люди. Господар пише: {word}\n\nВона повернулася до вас у чернетки — поправте й покажіть знову."
                ),
            ),
            ("approved", Lang::Ru) => (
                "Работа взята в игру",
                format!(
                    "«{name}» утверждена. Она получает тираж лицензий и ваше имя на них — навсегда.{}",
                    page()
                ),
            ),
            ("approved", Lang::En) => (
                "Taken into the game",
                format!(
                    "“{name}” is approved. It gets a run of licences with your name on them — for good.{}",
                    page()
                ),
            ),
            ("approved", Lang::Uk) => (
                "Роботу взято в гру",
                format!(
                    "«{name}» затверджена. Вона отримує наклад ліцензій і ваше ім’я на них — назавжди.{}",
                    page()
                ),
            ),
            ("struck", Lang::Ru) => (
                "Работа снята",
                format!("«{name}» снята с людских глаз. Причина: {word}"),
            ),
            ("struck", Lang::En) => (
                "Your work was taken down",
                format!("“{name}” has been taken down. Why: {word}"),
            ),
            ("struck", Lang::Uk) => (
                "Роботу знято",
                format!("«{name}» знято з людських очей. Причина: {word}"),
            ),
            ("chosen", Lang::Ru) => (
                "Работа в тройке недели",
                format!("«{name}» вошла в тройку сезона и ушла хозяину на решение."),
            ),
            ("chosen", Lang::En) => (
                "Among the three",
                format!("“{name}” is among the three of the season and has gone to the keeper."),
            ),
            ("chosen", Lang::Uk) => (
                "Робота в трійці тижня",
                format!("«{name}» увійшла до трійки сезону й пішла господареві на рішення."),
            ),
            ("admitted", Lang::Es) => (
                "Su trabajo está en la temporada",
                format!("«{name}» ha sido admitido y ya está en la temporada: se ve en la galería y la gente lo valora."),
            ),
            ("admitted", Lang::De) => (
                "Ihre Arbeit ist in der Saison",
                format!("„{name}“ ist zugelassen und bereits in der Saison: Sie ist in der Galerie zu sehen und wird bewertet."),
            ),
            ("admitted", Lang::It) => (
                "Il suo lavoro è nella stagione",
                format!("«{name}» è stato ammesso ed è già nella stagione: si vede nella galleria e viene valutato."),
            ),
            ("admittedOnly", Lang::Es) => (
                "Su trabajo es público",
                format!(
                    "«{name}» ha sido admitido: se ve en la galería. No ha entrado en la temporada porque esta semana ya tiene un trabajo en ella. Podrá presentarlo en la próxima."
                ),
            ),
            ("admittedOnly", Lang::De) => (
                "Ihre Arbeit ist öffentlich",
                format!(
                    "„{name}“ ist zugelassen und in der Galerie zu sehen. In die Saison ist sie nicht gekommen, weil Sie diese Woche schon eine Arbeit darin haben. In der nächsten Saison geht es."
                ),
            ),
            ("admittedOnly", Lang::It) => (
                "Il suo lavoro è pubblico",
                format!(
                    "«{name}» è stato ammesso: si vede nella galleria. Non è entrato nella stagione perché questa settimana ha già un lavoro in gara. Potrà presentarlo nella prossima."
                ),
            ),
            ("denied", Lang::Es) => (
                "Su trabajo ha vuelto a usted",
                format!(
                    "«{name}» todavía no se publica. La autora escribe: {word}\n\nHa vuelto a sus borradores: corríjalo y preséntelo de nuevo."
                ),
            ),
            ("denied", Lang::De) => (
                "Ihre Arbeit ist zurück bei Ihnen",
                format!(
                    "„{name}“ wird noch nicht veröffentlicht. Die Autorin schreibt: {word}\n\nSie liegt wieder in Ihren Entwürfen: Überarbeiten Sie sie und reichen Sie sie erneut ein."
                ),
            ),
            ("denied", Lang::It) => (
                "Il suo lavoro è tornato a lei",
                format!(
                    "«{name}» non viene ancora pubblicato. L’autrice scrive: {word}\n\nÈ tornato nelle sue bozze: lo corregga e lo presenti di nuovo."
                ),
            ),
            ("approved", Lang::Es) => (
                "Su trabajo ha entrado en el juego",
                format!(
                    "«{name}» ha sido aprobado. Recibe una tirada de licencias con su nombre, para siempre.{}",
                    page()
                ),
            ),
            ("approved", Lang::De) => (
                "Ihre Arbeit ist im Spiel",
                format!(
                    "„{name}“ ist bestätigt. Sie erhält eine Auflage von Lizenzen mit Ihrem Namen, dauerhaft.{}",
                    page()
                ),
            ),
            ("approved", Lang::It) => (
                "Il suo lavoro è entrato nel gioco",
                format!(
                    "«{name}» è stato approvato. Riceve una tiratura di licenze con il suo nome, per sempre.{}",
                    page()
                ),
            ),
            ("struck", Lang::Es) => (
                "Su trabajo ha sido retirado",
                format!("«{name}» ya no es público. Motivo: {word}"),
            ),
            ("struck", Lang::De) => (
                "Ihre Arbeit wurde entfernt",
                format!("„{name}“ ist nicht mehr öffentlich. Grund: {word}"),
            ),
            ("struck", Lang::It) => (
                "Il suo lavoro è stato ritirato",
                format!("«{name}» non è più pubblico. Motivo: {word}"),
            ),
            ("chosen", Lang::Es) => (
                "Entre los tres de la semana",
                format!("«{name}» está entre los tres mejores de la temporada y ha pasado a la autora para su decisión."),
            ),
            ("chosen", Lang::De) => (
                "Unter den drei der Woche",
                format!("„{name}“ ist unter den drei besten der Saison und liegt jetzt der Autorin zur Entscheidung vor."),
            ),
            ("chosen", Lang::It) => (
                "Tra i tre della settimana",
                format!("«{name}» è tra i tre migliori della stagione ed è passato all’autrice per la decisione."),
            ),
            _ => return Ok(()),
        };
        let _ = self
            .repo
            .create_thread(user, "studio", Some(frame), subject, &body, true, &[])
            .await;
        Ok(())
    }

    // ── Галерея и Зал авторов ───────────────────────────────────────────────

    /// Утверждённые рамки — для стола одобрения карты: хозяин надевает
    /// чужую работу на конкретную карту сразу там же, а не в отдельной
    /// панели битв.
    pub async fn list_approved_studio_frames(
        &self,
    ) -> Result<Vec<crate::models::StudioShownDto>> {
        self.repo.list_approved_studio_frames().await
    }

    pub async fn studio_gallery(
        &self,
        page: i64,
    ) -> Result<crate::models::StudioGalleryDto> {
        const PER_PAGE: i64 = 24;
        let page = page.max(0);
        Ok(crate::models::StudioGalleryDto {
            works: self
                .repo
                .list_studio_gallery(PER_PAGE, page * PER_PAGE)
                .await?,
            total: self.repo.count_studio_gallery().await?,
        })
    }

    pub async fn studio_author(&self, slug: &str) -> Result<crate::models::StudioAuthorDto> {
        let (id, name) = self
            .repo
            .studio_author(slug)
            .await?
            .ok_or_else(|| AppError::NotFound("author".into()))?;
        let works = self.repo.list_studio_author_works(id).await?;
        let cards = self.repo.list_studio_author_cards(id).await?;
        Ok(crate::models::StudioAuthorDto {
            // Взята в игру — это ОТМЕТКА, а не состояние: состояние говорит про
            // автора, отметка про решение хозяина.
            approved: works.iter().filter(|w| w.approved_at.is_some()).count() + cards.len(),
            cards,
            name,
            slug: slug.to_string(),
            works,
        })
    }

    /// Дать автору адрес — при первом допуске его работы.
    ///
    /// Лениво, потому что до допуска страницы у него нет и быть не должно:
    /// страницу, на которой ничего нет, в доме не заводят. Занятый слуг
    /// разрешается номером, а не отказом: имена людей повторяются, и второй
    /// «Мария» не должна остаться без адреса.
    async fn give_studio_slug(&self, frame: Uuid) -> Result<()> {
        let Some((user, name, slug, _, _)) = self.repo.studio_frame_owner(frame).await? else {
            return Ok(());
        };
        if slug.is_some() {
            return Ok(());
        }
        let base = crate::slug::slugify(&name);
        let base = if base.is_empty() { "author".to_string() } else { base };
        let mut candidate = base.clone();
        let mut n = 2;
        while self.repo.studio_slug_taken(&candidate).await? {
            candidate = format!("{base}-{n}");
            n += 1;
            if n > 50 {
                candidate = format!("{base}-{}", &user.to_string()[..6]);
                break;
            }
        }
        self.repo.set_studio_slug(user, &candidate).await
    }

    /// Тройка нынешнего сезона — то, что ждёт вашего слова.
    pub async fn studio_keeper_queue(&self) -> Result<Vec<crate::models::StudioEntryDto>> {
        let season = self.studio_season_now().await?;
        self.repo.list_studio_keeper_queue(season.id).await
    }

    /// Утвердить работу и назначить тираж лицензий.
    pub async fn approve_studio_frame(&self, id: Uuid, edition: Option<i32>) -> Result<()> {
        // Тираж — число, а не «сколько-нибудь»: ноль лицензий это не тираж, а
        // работа, которую никто не сможет носить.
        let edition = edition.filter(|n| *n > 0);
        if self.repo.approve_studio_frame(id, edition).await? {
            // Тираж печатается АВТОРУ и разом. Утверждение могут нажать дважды
            // — печать смотрит, есть ли у рамы хоть одна лицензия, и второй раз
            // не печатает ничего: тираж это число, а не действие.
            if let Some(count) = edition {
                let owner = self.frame_owner(id).await?;
                let made = self.repo.mint_frame_licences(id, owner, count).await?;
                if made > 0 {
                    Self::log_domain_event("studio_licences_minted", "studio_frame", id, "ok");
                }
            }
            self.note_about_frame(id, "approved", None).await?;
            // Письмо — вдогонку: медленный SMTP не держит стол, а неудача
            // письма не отменяет утверждения.
            if let Some((user, _, _, lang, name)) = self.repo.studio_frame_owner(id).await? {
                if let Ok(Some(who)) = self.repo.find_user_by_id(user).await
                    && let Some(to) = who.email.clone()
                {
                    let svc = self.clone();
                    let lang = crate::lang::Lang::parse(&lang).unwrap_or_default();
                    tokio::spawn(async move {
                        if let Err(e) = svc.send_studio_approved_email(&to, &name, lang).await {
                            tracing::warn!("Studio approval email failed: {e}");
                        }
                    });
                }
            }
            Self::log_domain_event("studio_frame_approved", "studio_frame", id, "ok");
            Ok(())
        } else {
            Err(AppError::NotFound("studio frame".into()))
        }
    }

    pub async fn strike_studio_frame(&self, id: Uuid, word: &str) -> Result<()> {
        let word: String = word.trim().chars().take(600).collect();
        if word.is_empty() {
            return Err(AppError::BadRequest("wordEmpty".into()));
        }
        if self.repo.strike_studio_frame(id, &word).await? {
            self.note_about_frame(id, "struck", Some(&word)).await?;
            Self::log_domain_event("studio_frame_struck", "studio_frame", id, "ok");
            Ok(())
        } else {
            Err(AppError::NotFound("studio frame".into()))
        }
    }

    /// Лицензии человека — то, что он может носить и однажды продать.
    pub async fn my_licences(&self, user_id: Uuid) -> Result<Vec<crate::models::LicenceDto>> {
        self.repo.list_my_licences(user_id).await
    }

    // ── Лавка авторов ───────────────────────────────────────────────────────

    /// Выставить свою вещь — лицензию или экземпляр карты. Цена в коридоре
    /// дома: без потолка первый же автор поставит миллион и лавка встанет, без
    /// пола её зальют мусором.
    pub async fn list_thing(
        &self,
        seller: Uuid,
        kind: &str,
        subject: Uuid,
        price: i32,
        currency: &str,
    ) -> Result<Uuid> {
        let settings = self.studio_admit(seller).await?;
        if kind != "license" && kind != "copy" {
            return Err(AppError::BadRequest("unknownKind".into()));
        }
        if !crate::battles::CURRENCIES.contains(&currency) {
            return Err(AppError::BadRequest("unknownCoin".into()));
        }
        if price < settings.price_floor || price > settings.price_ceil {
            return Err(AppError::BadRequest("priceOutside".into()));
        }
        self.repo
            .list_thing(seller, kind, subject, price, currency)
            .await
    }

    /// Свои экземпляры — все, включая выставленные: снимать их человек
    /// приходит туда же, где выставлял.
    pub async fn my_copies(&self, user_id: Uuid) -> Result<Vec<crate::models::MyCopyDto>> {
        self.repo.list_my_copies(user_id).await
    }

    pub async fn withdraw_listing(&self, seller: Uuid, listing: Uuid) -> Result<()> {
        if self.repo.withdraw_listing(seller, listing).await? {
            Ok(())
        } else {
            Err(AppError::NotFound("listing".into()))
        }
    }

    /// Купить. Деньги, вещь и объявление меняются одной транзакцией — половина
    /// сделки это либо потерянные деньги, либо потерянная вещь.
    pub async fn buy_listing(&self, buyer: Uuid, listing: Uuid) -> Result<i64> {
        let settings = self.studio_admit(buyer).await?;
        let (_, currency, _) = self
            .repo
            .buy_listing(buyer, listing, settings.commission_percent)
            .await?;
        Self::log_domain_event("market_sold", "market_listing", listing, "ok");
        self.repo.battle_wallet_balance(buyer, &currency).await
    }

    // ── Аукцион ─────────────────────────────────────────────────────────────

    pub async fn auctions(&self) -> Result<Vec<crate::models::AuctionDto>> {
        self.repo.list_auctions().await
    }

    pub async fn start_auction(
        &self,
        seller: Uuid,
        req: &crate::models::StartAuctionRequest,
    ) -> Result<Uuid> {
        let settings = self.studio_admit(seller).await?;
        if req.kind != "license" && req.kind != "copy" {
            return Err(AppError::BadRequest("unknownKind".into()));
        }
        if !crate::battles::CURRENCIES.contains(&req.currency.as_str()) {
            return Err(AppError::BadRequest("unknownCoin".into()));
        }
        if req.start_price < settings.price_floor || req.start_price > settings.price_ceil {
            return Err(AppError::BadRequest("priceOutside".into()));
        }
        let days = req.days.unwrap_or(crate::studio::AUCTION_DAYS).clamp(1, 30);
        let ends_at = Utc::now() + chrono::Duration::days(days);
        let id = self
            .repo
            .start_auction(
                Some(seller),
                &req.kind,
                req.subject_id,
                req.start_price,
                &req.currency,
                ends_at,
                false,
            )
            .await?;
        Self::log_domain_event("auction_started", "auction", id, "ok");
        Ok(id)
    }

    pub async fn place_bid(&self, bidder: Uuid, auction: Uuid, amount: i32) -> Result<i32> {
        self.studio_admit(bidder).await?;
        self.repo
            .place_bid(
                bidder,
                auction,
                amount,
                crate::studio::BID_STEP,
                crate::studio::SNIPE_MINUTES,
            )
            .await
    }

    /// Ударить молотком по всем, кому вышел срок. Зовётся фоновой задачей —
    /// той же, что подводит сезон: дом сам не забывает.
    pub async fn close_due_auctions(&self) -> Result<usize> {
        let settings = self.get_studio_settings().await?;
        let mut done = 0;
        for id in self.repo.auctions_due().await? {
            if self
                .repo
                .close_auction(id, settings.commission_percent)
                .await?
                .is_some()
            {
                done += 1;
            }
            Self::log_domain_event("auction_closed", "auction", id, "ok");
        }
        Ok(done)
    }

    /// Вещи ушедших из дома — на молоток.
    ///
    /// `STUDIO.md` §9: труд не отменяется удалением учётной записи, но и висеть
    /// вечно ничьим он не должен. Вырученное сгорает: платить некому.
    pub async fn auction_the_estate(&self) -> Result<usize> {
        let settings = self.get_studio_settings().await?;
        let ends_at = Utc::now() + chrono::Duration::days(crate::studio::AUCTION_DAYS);
        let mut put = 0;
        for (kind, id) in self.repo.ownerless_things().await? {
            self.repo
                .start_auction(
                    None,
                    &kind,
                    id,
                    settings.price_floor,
                    "dust",
                    ends_at,
                    true,
                )
                .await?;
            put += 1;
        }
        Ok(put)
    }

    // ── Мена ────────────────────────────────────────────────────────────────
    //
    // Мена предлагается против объявления: чтобы предложить человеку обмен,
    // надо сперва увидеть, что у него есть, а места, где видно чужое собрание,
    // в доме нет. Прилавок и есть «вот моя вещь, она отдаётся».

    pub async fn offer_trade(
        &self,
        from: Uuid,
        req: &crate::models::OfferTradeRequest,
    ) -> Result<Uuid> {
        self.studio_admit(from).await?;
        if req.items.is_empty() {
            // Мена ни за что — это просьба подарить, и сказать её надо словами,
            // а не пустым списком, который выглядит как поломка страницы.
            return Err(AppError::BadRequest("nothingOffered".into()));
        }
        if req.items.len() > crate::studio::TRADE_ITEMS_MAX {
            return Err(AppError::BadRequest("tooMuchOffered".into()));
        }
        if req
            .items
            .iter()
            .any(|i| i.kind != "license" && i.kind != "copy")
        {
            return Err(AppError::BadRequest("unknownKind".into()));
        }
        let trade = self.repo.offer_trade(from, req.listing_id, &req.items).await?;
        Self::log_domain_event("trade_offered", "trade", trade, "ok");
        Ok(trade)
    }

    pub async fn accept_trade(&self, seller: Uuid, trade: Uuid) -> Result<()> {
        self.studio_admit(seller).await?;
        if self.repo.accept_trade(seller, trade).await? {
            Self::log_domain_event("trade_taken", "trade", trade, "ok");
            Ok(())
        } else {
            Err(AppError::BadRequest("gone".into()))
        }
    }

    /// Отказать или забрать назад — одно движение: делают эти двое одно и то
    /// же, мена кончается и вещи отпираются.
    pub async fn refuse_trade(&self, who: Uuid, trade: Uuid) -> Result<()> {
        if self.repo.refuse_trade(who, trade).await? {
            Ok(())
        } else {
            Err(AppError::NotFound("trade".into()))
        }
    }

    pub async fn trades(&self, who: Uuid) -> Result<Vec<crate::models::TradeDto>> {
        self.repo.list_trades(who).await
    }

    pub async fn market_copies(&self) -> Result<Vec<crate::models::CopyListingDto>> {
        self.repo.list_market_copies().await
    }

    pub async fn market(&self) -> Result<Vec<crate::models::ListingDto>> {
        self.repo.list_market().await
    }

    pub async fn list_studio_reports(&self) -> Result<Vec<crate::models::StudioReportDto>> {
        self.repo.list_studio_reports().await
    }

    pub async fn close_studio_report(&self, id: Uuid) -> Result<()> {
        self.repo.close_studio_report(id).await
    }

    pub async fn studio_waiting(&self) -> Result<crate::models::StudioWaitingDto> {
        let (admissions, queue, reports) = self.repo.studio_waiting().await?;
        Ok(crate::models::StudioWaitingDto {
            admissions,
            queue,
            reports,
        })
    }

    pub async fn save_studio_season_theme(
        &self,
        theme: Option<&str>,
        note: Option<&str>,
    ) -> Result<crate::models::StudioSeasonDto> {
        let season = self.studio_season_now().await?;
        let theme: Option<String> = theme
            .map(|t| t.trim().chars().take(120).collect::<String>())
            .filter(|t| !t.is_empty());
        let note: Option<String> = note
            .map(|t| t.trim().chars().take(400).collect::<String>())
            .filter(|t| !t.is_empty());
        self.repo
            .save_studio_season_theme(season.id, theme.as_deref(), note.as_deref())
            .await?;
        self.studio_season_now().await
    }

    /// Библиотека дома — открытые детали склада. Без сессии: из них собирают
    /// рамку, и увидеть их должен всякий, кто пришёл посмотреть.
    pub async fn studio_library(
        &self,
        role: Option<&str>,
    ) -> Result<Vec<crate::models::BattleAssetDto>> {
        let rows = self.repo.list_public_battle_assets(role).await?;
        Ok(rows.into_iter().map(Self::asset_dto).collect())
    }

    /// Настройки студии. Умолчания дома — в `studio::StudioSettings::default`,
    /// и они же ответ, когда настройки ещё ни разу не сохраняли.
    pub async fn get_studio_settings(&self) -> Result<crate::studio::StudioSettings> {
        let saved: crate::studio::StudioSettings =
            parse_json_setting("studio", self.repo.get_setting("studio").await?)?;
        Ok(crate::studio::normalize_studio_settings(saved))
    }

    pub async fn save_studio_settings(
        &self,
        settings: crate::studio::StudioSettings,
    ) -> Result<crate::studio::StudioSettings> {
        let clean = crate::studio::normalize_studio_settings(settings);
        let json = serde_json::to_string(&clean)
            .map_err(|_| AppError::Internal("Настройки студии не сериализуются".into()))?;
        self.repo.upsert_setting("studio", &json).await?;
        Ok(clean)
    }

    pub async fn get_battle_gift(&self) -> Result<crate::models::BattleGift> {
        parse_json_setting("battle_gift", self.repo.get_setting("battle_gift").await?)
    }

    pub async fn save_battle_gift(
        &self,
        gift: crate::models::BattleGift,
    ) -> Result<crate::models::BattleGift> {
        // Отрицательный дар — это не строгость, а испорченная книга: комната
        // встречала бы человека долгом. Обрезается молча, как и ставки.
        let clean = crate::models::BattleGift {
            dust: gift.dust.clamp(0, 100_000),
            feed: gift.feed.clamp(0, 100_000),
        };
        let json = serde_json::to_string(&clean)
            .map_err(|_| AppError::Internal("Дар не сериализуется".into()))?;
        self.repo.upsert_setting("battle_gift", &json).await?;
        Ok(clean)
    }

    /// Войти в комнату.
    ///
    /// Два движения, оба — однажды по построению, и оба на одной и той же
    /// дописываемой книге:
    ///
    /// **Дар.** Гость без истории приходит с нулём, а дешёвая карта с настоящим
    /// телом стоит двенадцать. Церемония получения карты — и есть продукт
    /// комнаты (`TASKS-BATTLES.md` §0.6), и она должна случиться в первую
    /// минуту, а не на третий визит.
    ///
    /// **Проявка.** Ноль на первом входе — почти всегда ложь: человек, дошедший
    /// до полки, почти наверняка уже смотрел работы, и маячок за них не
    /// сработал, потому что тогда он был без имени. Список ведёт браузер;
    /// сервер проверяет каждую работу по своей базе и платит за неё однажды.
    /// Так ноль превращается в число, у которого есть история: не «дом дал вам
    /// тридцать», а «вы смотрели тридцать работ, дом их считал».
    ///
    /// Это POST, а не чтение полки, именно поэтому: чтение страницы не может
    /// быть источником денег.
    pub async fn enter_battle_room(
        &self,
        user_id: Uuid,
        req: crate::models::BattleEnterRequest,
    ) -> Result<crate::models::BattleEnterResponse> {
        let gift = self.get_battle_gift().await?;
        let mut given = crate::models::BattleGift { dust: 0, feed: 0 };
        if gift.dust > 0 && self.repo.welcome_battle_wallet(user_id, "dust", gift.dust).await? {
            given.dust = gift.dust;
        }
        if gift.feed > 0 && self.repo.welcome_battle_wallet(user_id, "feed", gift.feed).await? {
            given.feed = gift.feed;
        }

        let rates = self.get_battle_dust_rates().await?;
        let seen: Vec<uuid::Uuid> = req
            .seen
            .iter()
            .filter_map(|s| uuid::Uuid::parse_str(s).ok())
            .take(crate::battles::DEVELOP_MAX)
            .collect();
        let developed_works = self
            .repo
            .credit_battle_seen_bulk(
                user_id,
                &seen,
                rates.seen,
                crate::battles::ATTENTION_DAILY_CAP,
            )
            .await?;
        let developed_works = i32::try_from(developed_works).unwrap_or(i32::MAX);

        // Приход отмечается ДО счёта: иначе поручение «зашли сегодня» закрылось
        // бы только на следующем заходе, то есть завтра.
        self.mark_visit_quietly(user_id).await;

        // Считается ПОСЛЕ дара и проявки: «взять первую карту» человек мог
        // закрыть ещё вчера, а «оглядеться» — только что, этой самой проявкой.
        let paid = self.settle_errands_quietly(user_id).await;

        let (me, errands) = tokio::try_join!(
            self.battle_me(user_id),
            self.list_battle_errands(user_id),
        )?;
        Ok(crate::models::BattleEnterResponse {
            me,
            gift: (given.dust > 0 || given.feed > 0).then_some(given),
            developed_works,
            developed_dust: developed_works.saturating_mul(rates.seen),
            paid,
            errands,
        })
    }

    /// Balances and holdings in one answer: the shelf cannot draw a single card
    /// until it knows whose that card is.
    pub async fn battle_me(&self, user_id: Uuid) -> Result<crate::models::BattleMeDto> {
        let (dust, feed, owned, gifts) = tokio::try_join!(
            self.repo.battle_wallet_balance(user_id, "dust"),
            self.repo.battle_wallet_balance(user_id, "feed"),
            self.repo.list_owned_battle_cards(user_id),
            self.repo
                .list_battle_hand_grants(user_id, crate::battles::GIFTS_SHOWN),
        )?;
        Ok(crate::models::BattleMeDto {
            dust,
            feed,
            owned: owned
                .into_iter()
                .map(|o| crate::models::BattleOwnedCardDto {
                    card_id: o.card_id.to_string(),
                    serial: o.serial,
                    level: o.level,
                    is_new: o.seen_at.is_none(),
                })
                .collect(),
            gifts: gifts
                .into_iter()
                .map(|g| crate::models::BattleGiftDto {
                    currency: g.currency,
                    amount: g.amount,
                    reason: g.reason,
                    note: g.note,
                    at: g.created_at.to_rfc3339(),
                })
                .collect(),
        })
    }

    /// Что у гостя сейчас: монеты, карты, записки. То же, что видит он сам.
    ///
    /// Ровно `battle_me`, только чужой книги. Отдельного разбора нет намеренно:
    /// проверять игру надо по тому, что видит человек, а не по второму отчёту,
    /// который может с ним разойтись.
    pub async fn admin_read_battle_guest(
        &self,
        user_id: Uuid,
    ) -> Result<crate::models::BattleMeDto> {
        self.battle_me(user_id).await
    }

    /// Выдать карты гостю напрямую, минуя покупку.
    ///
    /// Кошелька не касается: это подарок, а не покупка. Нужен, чтобы привести
    /// собрание в состояние, в котором игру можно проверить, — иначе для этого
    /// пришлось бы начислить монеты, войти под гостем и купить карты по одной.
    pub async fn admin_give_battle_cards(
        &self,
        req: &crate::models::GiveBattleCardsRequest,
    ) -> Result<crate::models::GiveBattleCardsResponse> {
        let level = crate::battles::clamp_level(req.level);
        let ids: Vec<Uuid> = if req.all {
            // Только те, что могут выйти на поле: выдать карту без здоровья
            // значило бы выдать тело, падающее в тот же миг, как встанет.
            self.list_battle_cards_public()
                .await?
                .into_iter()
                .filter(crate::battles::can_take_the_field)
                .filter_map(|c| c.id.parse().ok())
                .collect()
        } else {
            req.card_ids.clone()
        };
        if ids.is_empty() {
            return Ok(crate::models::GiveBattleCardsResponse { touched: 0 });
        }
        let touched = self
            .repo
            .give_battle_cards(req.user_id, &ids, level)
            .await?;
        Self::log_domain_event("battle_cards_given", "user", req.user_id, "ok");
        Ok(crate::models::GiveBattleCardsResponse { touched })
    }

    /// Забрать карты обратно. Пустой список — все.
    ///
    /// Единственный способ проверить пустое собрание и заём, который его
    /// закрывает: обратно карты сами не уходят.
    pub async fn admin_revoke_battle_cards(
        &self,
        req: &crate::models::RevokeBattleCardsRequest,
    ) -> Result<crate::models::GiveBattleCardsResponse> {
        let ids = if req.all {
            Vec::new()
        } else {
            req.card_ids.clone()
        };
        if !req.all && ids.is_empty() {
            return Ok(crate::models::GiveBattleCardsResponse { touched: 0 });
        }
        let touched = self.repo.revoke_battle_cards(req.user_id, &ids).await?;
        Self::log_domain_event("battle_cards_revoked", "user", req.user_id, "ok");
        Ok(crate::models::GiveBattleCardsResponse { touched })
    }

    /// Дать из рук.
    ///
    /// Единственный способ, которым в доме появляется корм: он не оседает сам,
    /// как пыль, — его даёт хранитель за настоящее (состоявшийся показ,
    /// опубликованное впечатление, заказ работы). Поэтому здесь нет ни ставки,
    /// ни правила: есть человек, число и записка.
    ///
    /// Записка обязательна. Корм без неё был бы просто выросшим счётчиком —
    /// ровно тем, от чего эта комната отказывается.
    pub async fn admin_grant_battle_coin(
        &self,
        req: &crate::models::GrantBattleCoinRequest,
    ) -> Result<crate::models::GrantBattleCoinResponse> {
        if !crate::battles::valid_currency(&req.currency) {
            return Err(AppError::BadRequest("Unknown coin".into()));
        }
        if req.amount == 0 || req.amount.abs() > crate::battles::GRANT_MAX {
            return Err(AppError::BadRequest(
                "A grant of nothing is not a grant".into(),
            ));
        }
        let note = req.note.as_deref().map(str::trim).filter(|s| !s.is_empty());
        if note.is_none() {
            return Err(AppError::BadRequest("A grant needs a note".into()));
        }
        // Ключ приходит от панели и по одному на открытую форму: сервер,
        // чеканящий его сам, сделал бы двойной щелчок второй выдачей.
        let key = req.idem_key.trim();
        if key.is_empty() || key.len() > 100 {
            return Err(AppError::BadRequest(
                "A grant needs a key of its own".into(),
            ));
        }
        let note = crate::battles::clamp_note(note);
        let granted_now = self
            .repo
            .grant_battle_wallet(
                req.user_id,
                &req.currency,
                req.amount,
                note.as_deref(),
                &format!("hand:{key}"),
            )
            .await?;
        let balance = self
            .repo
            .battle_wallet_balance(req.user_id, &req.currency)
            .await?;
        if granted_now {
            Self::log_domain_event("battle_coin_granted", "user", req.user_id, "ok");
        }
        Ok(crate::models::GrantBattleCoinResponse {
            balance,
            granted_now,
        })
    }

    // ── Стол гостя ────────────────────────────────────────────────────────
    //
    // Шесть мест: три на клетках своей половины, три в руке. Чего гость не
    // выбрал — закрывает дом (`lendable`), потому что иначе стол это запертая
    // дверь ровно для того, кому комната нужнее всего: новый гость владеет
    // нулём карт, а партия просит шести.

    /// Стол как он есть — с досчитанным заёмом.
    ///
    /// Заём НЕ хранится вместе с колодой и досчитывается на каждое чтение:
    /// иначе он застыл бы в чужой колоде и пережил бы решение хранителя его
    /// отозвать.
    pub async fn read_battle_deck(&self, user_id: Uuid) -> Result<crate::models::BattleDeckDto> {
        let stored = self.repo.get_battle_deck(user_id).await?;
        let laid = stored.is_some();
        // Строка, которая не читается, не запирает человека в его собственной
        // комнате: стол открывается пустым, и первое же сохранение его чинит.
        let layout: crate::models::DeckLayout = stored
            .as_ref()
            .and_then(|d| serde_json::from_str(&d.layout).ok())
            .unwrap_or_default();
        let loan = self.loan_inputs(user_id).await?;
        let (board, hand, nothing_to_lend) =
            Self::lay_with_loan(&layout, &loan, &crate::battles::DECK_DEFAULT_CELLS);
        Ok(crate::models::BattleDeckDto {
            board,
            hand,
            laid,
            nothing_to_lend,
        })
    }

    /// Что нужно, чтобы досчитать заём: какие карты выходят на поле, какие у
    /// гостя есть и что дом одолжит.
    async fn loan_inputs(&self, user_id: Uuid) -> Result<LoanInputs> {
        let cards = self.list_battle_cards_public().await?;
        let fieldable: std::collections::HashSet<Uuid> = cards
            .iter()
            .filter(|c| crate::battles::can_take_the_field(c))
            .filter_map(|c| c.id.parse().ok())
            .collect();
        // Владение меняется (продажа, сброс, перевыпуск) независимо от того,
        // снята ли карта с публикации — старый стол, ссылающийся на чужую
        // карту, обязан считаться ушедшим ровно так же, как ссылающийся на
        // снятую: иначе гость видит обычную карту, шлёт её же назад при
        // сохранении, и сервер отказывает снова и снова, а обновление
        // страницы ничего не чинит — оно перечитывает то же самое.
        let owned: std::collections::HashSet<Uuid> = self
            .repo
            .list_owned_battle_cards(user_id)
            .await?
            .into_iter()
            .map(|o| o.card_id)
            .collect();
        let pool = self.loan_pool(user_id).await?;
        Ok(LoanInputs {
            fieldable,
            owned,
            pool,
            cards,
        })
    }

    /// Места стола или расстановки с досчитанным заёмом.
    ///
    /// `loan_cells` — клетки, на которые дом ставит временные карты, в порядке
    /// предпочтения; занятые картами гостя пропускаются. Не хватило клеток —
    /// место на поле остаётся пустым: ставить временную карту некуда.
    fn lay_with_loan(
        layout: &crate::models::DeckLayout,
        loan: &LoanInputs,
        loan_cells: &[(u8, u8)],
    ) -> (
        Vec<crate::models::BattleDeckSlotDto>,
        Vec<crate::models::BattleDeckSlotDto>,
        bool,
    ) {
        // Пул перебирается по кругу, а не раздаётся по одной карте: у дома,
        // отметившего одну карту заёмной, иначе закрывалось бы одно место из
        // шести, а остальные пять оставались бы пустыми — то есть отметить
        // меньше шести карт значило бы не отметить ничего. Повтор одного тела
        // на поле правилам не мешает; пустая половина мешает партии.
        let pool = &loan.pool;
        let mut given = 0usize;
        let mut hand_out = |taken: bool| -> Option<String> {
            if !taken || pool.is_empty() {
                return None;
            }
            let id = pool[given % pool.len()];
            given += 1;
            Some(id.to_string())
        };
        let gone = |card: &Uuid| !loan.fieldable.contains(card) || !loan.owned.contains(card);

        let mut board = Vec::with_capacity(crate::battles::DECK_BOARD);
        for slot in layout.board.iter().take(crate::battles::DECK_BOARD) {
            let gone = gone(&slot.card);
            board.push(crate::models::BattleDeckSlotDto {
                card_id: Some(slot.card.to_string()),
                gone,
                lent_card_id: hand_out(gone),
                x: Some(slot.x),
                y: Some(slot.y),
            });
        }
        // Пустые места дома встают на те клетки, которых гость ещё не занял:
        // заём, поставленный поверх своей карты, был бы не заёмом, а подменой.
        // Свободные клетки отбираются ДО того, как в `board` что-то допишут:
        // ленивый фильтр держал бы `board` на всё время дозаписи.
        let taken_cells: Vec<(u8, u8)> = board.iter().filter_map(|s| s.x.zip(s.y)).collect();
        let mut free = loan_cells
            .iter()
            .copied()
            .filter(|cell| !taken_cells.contains(cell));
        while board.len() < crate::battles::DECK_BOARD {
            let Some((x, y)) = free.next() else { break };
            board.push(crate::models::BattleDeckSlotDto {
                card_id: None,
                gone: false,
                lent_card_id: hand_out(true),
                x: Some(x),
                y: Some(y),
            });
        }

        let mut hand = Vec::with_capacity(crate::battles::DECK_HAND);
        for card in layout.hand.iter().take(crate::battles::DECK_HAND) {
            let gone = gone(card);
            hand.push(crate::models::BattleDeckSlotDto {
                card_id: Some(card.to_string()),
                gone,
                lent_card_id: hand_out(gone),
                x: None,
                y: None,
            });
        }
        while hand.len() < crate::battles::DECK_HAND {
            hand.push(crate::models::BattleDeckSlotDto {
                card_id: None,
                gone: false,
                lent_card_id: hand_out(true),
                x: None,
                y: None,
            });
        }

        // Не «не хватило на все места» — а «дому нечего одолжить вовсе».
        // Пул перебирается по кругу, поэтому непокрытое место означает ровно
        // одно: хранитель не отметил ни одной карты.
        let nothing_to_lend = board
            .iter()
            .chain(&hand)
            .any(|s| s.lent_card_id.is_none() && (s.card_id.is_none() || s.gone));
        (board, hand, nothing_to_lend)
    }

    // ── Расстановка перед встречей ────────────────────────────────────────
    //
    // Карты расставляют прямо перед боем, на поле этой встречи: с её
    // величиной, местностью и картами противника. Расстановка запоминается
    // для встречи, и повторная партия открывается с прежней.

    /// Расстановка встречи. Этюд, в котором гость не приводит своих карт,
    /// расстановки не имеет: обе стороны в нём задал хранитель.
    async fn meeting_setup(&self, challenge_id: Uuid) -> Result<ChallengeSetup> {
        let challenge = self.repo.get_battle_challenge(challenge_id).await?;
        if challenge.status != "published" {
            return Err(AppError::NotFound("No such challenge".into()));
        }
        if challenge.player_side != crate::battles::SIDE_DECK {
            return Err(AppError::BadRequest(
                "This challenge sets both sides; there is nothing to arrange".into(),
            ));
        }
        serde_json::from_str(&challenge.setup)
            .map_err(|_| AppError::Internal("Challenge arrangement will not read".into()))
    }

    /// Расстановка гостя на поле встречи — с досчитанным заёмом и с тем, куда
    /// достаёт противник.
    pub async fn read_battle_lineup(
        &self,
        user_id: Uuid,
        challenge_id: Uuid,
    ) -> Result<crate::models::BattleLineupDto> {
        let setup = self.meeting_setup(challenge_id).await?;
        let loan = self.loan_inputs(user_id).await?;
        let mut dto = self.lineup_on(user_id, challenge_id, &setup, &loan).await?;
        dto.threats = self.strike_zones_on(&setup, &dto, &loan).await;
        Ok(dto)
    }

    /// Расстановка без зон удара.
    ///
    /// Не сохранял для этой встречи — берётся прежний стол, поставленный к
    /// середине поля, как его ставил сервер до расстановки перед боем. Карта,
    /// чья клетка с тех пор закрылась (хранитель поставил туда стену), уходит
    /// на ближайшую открытую: расстановка, ставшая незаконной не по вине
    /// гостя, не должна отказывать.
    async fn lineup_on(
        &self,
        user_id: Uuid,
        challenge_id: Uuid,
        setup: &ChallengeSetup,
        loan: &LoanInputs,
    ) -> Result<crate::models::BattleLineupDto> {
        let ground = crate::battles::LineupGround::of(setup);
        let stored = self.repo.get_battle_lineup(user_id, challenge_id).await?;
        let laid = stored.is_some();
        let mut layout: crate::models::DeckLayout = match stored {
            Some(json) => serde_json::from_str(&json).unwrap_or_default(),
            None => self.lineup_from_deck(user_id, ground.field).await?,
        };
        let open = ground.open_cells();
        Self::settle_cells(&mut layout, &open);
        let loan_cells = Self::loan_cells(&layout, &open);
        let (board, hand, nothing_to_lend) = Self::lay_with_loan(&layout, loan, &loan_cells);
        Ok(crate::models::BattleLineupDto {
            board,
            hand,
            laid,
            nothing_to_lend,
            cells: open
                .iter()
                .map(|&(x, y)| battle_core::Cell { x, y })
                .collect(),
            limits: crate::battles::deck_limits(),
            threats: Vec::new(),
        })
    }

    /// Куда встают временные карты: сначала на клетки, выбранные гостем (если
    /// они ещё открыты и не заняты его картами), затем по порядку дома.
    fn loan_cells(layout: &crate::models::DeckLayout, open: &[(u8, u8)]) -> Vec<(u8, u8)> {
        let own: Vec<(u8, u8)> = layout.board.iter().map(|p| (p.x, p.y)).collect();
        let mut cells: Vec<(u8, u8)> = Vec::with_capacity(open.len());
        let chosen = layout.lent.iter().map(|c| (c.x, c.y));
        for cell in chosen.chain(open.iter().copied()) {
            if open.contains(&cell) && !own.contains(&cell) && !cells.contains(&cell) {
                cells.push(cell);
            }
        }
        cells
    }

    /// Куда противник достаёт ударом при этой расстановке.
    ///
    /// Подсказка, а не условие: не собралось (карта противника снята) —
    /// расставлять это не мешает, и зон просто нет.
    async fn strike_zones_on(
        &self,
        setup: &ChallengeSetup,
        lineup: &crate::models::BattleLineupDto,
        loan: &LoanInputs,
    ) -> Vec<crate::models::StrikeZoneDto> {
        let Ok(cards) = self.cards_by_slug().await else {
            return Vec::new();
        };
        let Ok(mut raised) = Self::raise(setup, &cards) else {
            return Vec::new();
        };
        let Ok((board, _)) = Self::bodies_of(lineup, loan) else {
            return Vec::new();
        };
        raised.player_board = board;
        raised.player_hand = Vec::new();
        let state = battle_core::MatchState::begin_with(raised, crate::battles::rules_of(setup));
        state
            .strike_zones(battle_core::Side::Keeper)
            .into_iter()
            .map(|(cell, cells)| crate::models::StrikeZoneDto { cell, cells })
            .collect()
    }

    /// Прежний стол, поставленный на поле встречи: передний ряд стола — к
    /// середине поля (`deck_cell_on`).
    async fn lineup_from_deck(
        &self,
        user_id: Uuid,
        field: battle_core::Field,
    ) -> Result<crate::models::DeckLayout> {
        let Some(deck) = self.repo.get_battle_deck(user_id).await? else {
            return Ok(crate::models::DeckLayout::default());
        };
        let mut layout: crate::models::DeckLayout =
            serde_json::from_str(&deck.layout).unwrap_or_default();
        for p in &mut layout.board {
            if let Some(cell) = battle_core::Cell::new(p.x, p.y) {
                let on = crate::battles::deck_cell_on(field, cell);
                p.x = on.x;
                p.y = on.y;
            }
        }
        Ok(layout)
    }

    /// Карты на закрытых или повторённых клетках уходят на первую свободную
    /// открытую клетку; не нашлось такой — карта снимается с поля.
    fn settle_cells(layout: &mut crate::models::DeckLayout, open: &[(u8, u8)]) {
        layout.board.truncate(crate::battles::DECK_BOARD);
        let mut taken: Vec<(u8, u8)> = Vec::new();
        let mut homeless = Vec::new();
        for (i, p) in layout.board.iter().enumerate() {
            let cell = (p.x, p.y);
            if open.contains(&cell) && !taken.contains(&cell) {
                taken.push(cell);
            } else {
                homeless.push(i);
            }
        }
        let mut dropped = Vec::new();
        for i in homeless {
            match open.iter().find(|c| !taken.contains(c)) {
                Some(&(x, y)) => {
                    layout.board[i].x = x;
                    layout.board[i].y = y;
                    taken.push((x, y));
                }
                None => dropped.push(i),
            }
        }
        for i in dropped.into_iter().rev() {
            layout.board.remove(i);
        }
    }

    /// Расставить карты на поле встречи.
    ///
    /// Проверяется тем же правилом, что перечитывает начало партии, — по полю
    /// этой встречи, а не по половине 3 × 3.
    ///
    /// Страница сохраняет сама после каждой правки, поэтому у ручки свой предел
    /// в минуту: щедрый для человека и тесный для цикла.
    pub async fn save_battle_lineup(
        &self,
        user_id: Uuid,
        challenge_id: Uuid,
        req: &crate::models::SaveBattleDeckRequest,
    ) -> Result<crate::models::BattleLineupDto> {
        self.check_hot_limit(
            "lineup",
            &user_id.to_string(),
            crate::battles::LINEUP_SAVES_PER_MINUTE,
            60,
        )
        .await?;
        let setup = self.meeting_setup(challenge_id).await?;
        let ground = crate::battles::LineupGround::of(&setup);
        let loan = self.loan_inputs(user_id).await?;
        let board: Vec<(Uuid, u8, u8)> = req.board.iter().map(|p| (p.card, p.x, p.y)).collect();
        crate::battles::check_lineup(&ground, &board, &req.hand, &loan.owned, &loan.tier_of())
            .and_then(|()| crate::battles::check_loan_cells(&ground, &board, &req.lent))
            .map_err(|fault| AppError::BadRequest(format!("deck:{}", fault.word())))?;

        let layout = crate::models::DeckLayout {
            board: req
                .board
                .iter()
                .map(|p| crate::models::DeckPlacement {
                    card: p.card,
                    x: p.x,
                    y: p.y,
                })
                .collect(),
            hand: req.hand.clone(),
            lent: req.lent.clone(),
        };
        let json = serde_json::to_string(&layout)
            .map_err(|_| AppError::Internal("Lineup will not serialise".into()))?;
        self.repo
            .upsert_battle_lineup(user_id, challenge_id, &json)
            .await?;
        self.settle_errands_quietly(user_id).await;
        let mut dto = self.lineup_on(user_id, challenge_id, &setup, &loan).await?;
        dto.threats = self.strike_zones_on(&setup, &dto, &loan).await;
        Ok(dto)
    }

    /// Что дом одолжит этому гостю, в одном и том же порядке.
    ///
    /// Свои карты из пула вычеркнуты: одна и та же карта своей и заёмной на
    /// одном поле — не поломка правил, но лишний вопрос в голове у человека.
    async fn loan_pool(&self, user_id: Uuid) -> Result<Vec<Uuid>> {
        let held: std::collections::HashSet<Uuid> = self
            .repo
            .list_owned_battle_cards(user_id)
            .await?
            .into_iter()
            .map(|o| o.card_id)
            .collect();
        let limit =
            (crate::battles::DECK_BOARD + crate::battles::DECK_HAND) as i64 + held.len() as i64;
        // Порядок один и тот же на каждое чтение: стол, который тасует заём при
        // каждом открытии, — это стол, на который нельзя посмотреть дважды.
        Ok(self
            .repo
            .list_lendable_battle_cards(crate::battles::DECK_LOAN_TIER, limit)
            .await?
            .into_iter()
            .map(|c| c.id)
            .filter(|id| !held.contains(id))
            .collect())
    }

    /// Разложить стол.
    ///
    /// Заём сюда не приходит: его не выбирают. Приходит только то, что гость
    /// поставил сам, — и проверяется тем же правилом, которое потом перечитает
    /// начало партии.
    pub async fn save_battle_deck(
        &self,
        user_id: Uuid,
        req: &crate::models::SaveBattleDeckRequest,
    ) -> Result<crate::models::BattleDeckDto> {
        let owned: std::collections::HashSet<Uuid> = self
            .repo
            .list_owned_battle_cards(user_id)
            .await?
            .into_iter()
            .map(|o| o.card_id)
            .collect();
        let tier_of: std::collections::HashMap<Uuid, i16> = self
            .list_battle_cards_public()
            .await?
            .into_iter()
            .filter_map(|c| c.id.parse().ok().map(|id| (id, c.tier)))
            .collect();

        let board: Vec<(Uuid, u8, u8)> = req.board.iter().map(|p| (p.card, p.x, p.y)).collect();
        // Отказ называет причину словом, а не текстом: текст живёт в `i18n`, и
        // сервер, который его сочиняет, сочиняет его на одном языке.
        crate::battles::check_deck(&board, &req.hand, &owned, &tier_of)
            .map_err(|fault| AppError::BadRequest(format!("deck:{}", fault.word())))?;

        let layout = crate::models::DeckLayout {
            board: req
                .board
                .iter()
                .map(|p| crate::models::DeckPlacement {
                    card: p.card,
                    x: p.x,
                    y: p.y,
                })
                .collect(),
            hand: req.hand.clone(),
            lent: Vec::new(),
        };
        let json = serde_json::to_string(&layout)
            .map_err(|_| AppError::Internal("Table will not serialise".into()))?;
        self.repo.upsert_battle_deck(user_id, &json).await?;
        // Стол разложен — поручение об этом закрывается здесь, а не на чтении
        // полки: платит то движение, которое и так случилось.
        self.settle_errands_quietly(user_id).await;
        self.read_battle_deck(user_id).await
    }

    /// Take a card off the shelf.
    ///
    /// The price is read from the table, never from the request. What the
    /// request carries is the price the visitor *saw*, and its only job is to
    /// let the server refuse a stale page: a card repriced while the shelf
    /// stood open must not be taken at yesterday's number, in either direction.
    pub async fn buy_battle_card(
        &self,
        user_id: Uuid,
        req: &crate::models::BuyBattleCardRequest,
    ) -> Result<crate::models::BuyBattleCardResponse> {
        if req.currency != "dust" && req.currency != "feed" {
            return Err(AppError::BadRequest("Unknown coin".into()));
        }
        let card = self
            .repo
            .get_battle_card_admin(req.card_id)
            .await?
            .ok_or_else(|| AppError::NotFound("No such card".into()))?;
        if card.status != "published" {
            return Err(AppError::BadRequest("This card is not on the shelf".into()));
        }
        let price = match req.currency.as_str() {
            "dust" => card.price_dust,
            _ => card.price_feed,
        }
        .ok_or_else(|| AppError::BadRequest("This card is not sold for that coin".into()))?;
        if price != req.expected_price {
            return Err(AppError::BadRequest("The price has changed".into()));
        }

        let taken_now = self
            .repo
            .buy_battle_card(user_id, req.card_id, &req.currency, price)
            .await?;
        let balance = self
            .repo
            .battle_wallet_balance(user_id, &req.currency)
            .await?;
        // Собрание выросло: «первая карта» и «три карты» могли закрыться прямо
        // сейчас. Баланс уже прочитан выше и потому не досчитает эту выплату —
        // полка перечитывает кошелёк сразу после покупки, и второе число здесь
        // было бы обманом ради одного лишнего запроса.
        self.settle_errands_quietly(user_id).await;
        Ok(crate::models::BuyBattleCardResponse {
            card_id: req.card_id.to_string(),
            level: 1,
            balance,
            taken_now,
        })
    }

    /// Raise your own copy a rung.
    ///
    /// The level does not touch a match — not by a point of anything. That is
    /// decided in `TASKS-BATTLE-ENGINE.md` §1.6 and it is the whole reason this
    /// is allowed to be bought with dust at all: dust is a memory of attention,
    /// and attention that bought strength would be pay-to-win with the price
    /// tag in hours. A level is foil, a signature, a notch on the shelf.
    ///
    /// So there is nothing here to guard against a rich opponent. What is
    /// guarded is arithmetic: the rung, its price and the balance are all read
    /// on the server, and the ladder is climbed one rung at a time.
    pub async fn raise_battle_card_level(
        &self,
        user_id: Uuid,
        req: &crate::models::RaiseBattleCardRequest,
    ) -> Result<crate::models::RaiseBattleCardResponse> {
        let card = self
            .repo
            .get_battle_card_admin(req.card_id)
            .await?
            .ok_or_else(|| AppError::NotFound("No such card".into()))?;
        let ladder = card
            .level_price_dust
            .as_ref()
            .filter(|l| l.len() == 4)
            .ok_or_else(|| AppError::BadRequest("This card does not rise".into()))?;

        let held = self
            .repo
            .list_owned_battle_cards(user_id)
            .await?
            .into_iter()
            .find(|o| o.card_id == req.card_id)
            .ok_or_else(|| AppError::BadRequest("This card is not yours".into()))?;
        if held.level >= 5 {
            return Err(AppError::BadRequest(
                "This copy is as high as it goes".into(),
            ));
        }

        // Ступень читается из уровня, а не из запроса: клиент, который умеет
        // назвать ступень, умеет назвать дешёвую.
        let rung = usize::try_from(held.level - 1)
            .map_err(|_| AppError::Internal("Impossible level".into()))?;
        let price = *ladder
            .get(rung)
            .ok_or_else(|| AppError::Internal("Ladder is short a rung".into()))?;
        if price != req.expected_price {
            return Err(AppError::BadRequest("The price has changed".into()));
        }

        let (level, raised_now) = self
            .repo
            .raise_battle_card_level(user_id, req.card_id, held.level, price)
            .await?;
        self.settle_errands_quietly(user_id).await;
        let balance = self.repo.battle_wallet_balance(user_id, "dust").await?;
        Ok(crate::models::RaiseBattleCardResponse {
            card_id: req.card_id.to_string(),
            level,
            balance,
            raised_now,
        })
    }

    pub async fn mark_battle_card_seen(&self, user_id: Uuid, card_id: Uuid) -> Result<()> {
        self.repo.mark_battle_card_seen(user_id, card_id).await
    }

    /// Dust for attention the house already counts.
    ///
    /// One kind, one thing, once — the ledger key is the whole guard. A tale
    /// read twice pays once, and the ceiling on the whole faucet is the size of
    /// the house: there are only so many works to look at.
    ///
    /// Never fails on an unknown thing: attention is reported by a page as it
    /// goes, and a beacon that can return an error is a beacon somebody will
    /// eventually show to a reader.
    pub async fn grant_attention_dust(
        &self,
        user_id: Uuid,
        kind: &str,
        id: Uuid,
        visitor_hashes: &[String],
    ) -> Result<crate::models::BattleAttentionResponse> {
        let rates = self.get_battle_dust_rates().await?;
        let amount = match kind {
            "liked" => rates.liked,
            "seen" => rates.seen,
            "read" => rates.read,
            _ => 0,
        };
        // Вещь, за которую платят, должна существовать и быть открытой.
        // Без этого придуманный UUID — это выдуманная пыль, и ключ книги её не
        // ловит: он стережёт от второй выплаты, а не от первой выдуманной.
        // Платит только внимание, которое сервер видел сам: просмотр страницы
        // (для байки — ещё и присутствие на ней) от этого посетителя за
        // последний час. Страница присылает маячок сама, и один запрос с
        // придуманным id иначе оплачивал бы любую работу без единого просмотра.
        // Сверх того у имени есть суточный потолок: сколько бы имён ни завёл
        // один человек, каждое получает не больше него.
        let settled = if amount > 0
            && self.repo.battle_attention_target_public(kind, id).await?
            && self
                .repo
                .battle_attention_witnessed(
                    kind,
                    id,
                    visitor_hashes,
                    crate::battles::ATTENTION_WITNESS_MINUTES,
                    crate::battles::ATTENTION_READ_MIN_MS,
                )
                .await?
        {
            let credit = self
                .repo
                .credit_attention_capped(
                    user_id,
                    amount,
                    kind,
                    id,
                    &format!("{kind}:{id}"),
                    crate::battles::ATTENTION_DAILY_CAP,
                )
                .await?;
            // Имя, упёршееся в суточный потолок, видно в журнале: человеку
            // столько внимания за день не нужно, а те, кто заводит имена ради
            // пыли, упираются в него каждый день.
            if credit == crate::db::AttentionCredit::CapReached {
                tracing::warn!(
                    target: "gotiga_server::economy",
                    %user_id, kind,
                    "attention dust: daily cap reached"
                );
            }
            credit == crate::db::AttentionCredit::Paid
        } else {
            false
        };
        // Внимание — самый частый повод, и почти вся тропа стоит на нём.
        // Считается до чтения баланса, чтобы вернувшееся число уже включало
        // закрытое поручение: иначе полка показала бы старую сумму и догнала бы
        // её молча на следующем заходе.
        if settled {
            self.settle_errands_quietly(user_id).await;
        }
        let balance = self.repo.battle_wallet_balance(user_id, "dust").await?;
        Ok(crate::models::BattleAttentionResponse {
            dust: if settled { amount } else { 0 },
            balance,
        })
    }

    /// Everything the keeper typed, clamped into what the frame and the table
    /// can hold. Text is cut rather than refused — a title one character too
    /// long is not worth losing a draft over — but a card with no price in
    /// either coin is refused, because such a card can never be taken.
    fn prepare_battle_card_save(
        req: &SaveBattleCardRequest,
        taken: &[String],
    ) -> Result<BattleCardWrite> {
        if !crate::battles::valid_status(&req.status) {
            return Err(AppError::BadRequest("Unknown card status".into()));
        }
        // Карта, которую нельзя ни выложить, ни оплатить, не должна попадать на
        // полку: гость купит её за настоящую пыль и не сможет ничего с ней
        // сделать. Отказ называет причину тем же словом, которое весы показали
        // хранителю, пока он печатал, — одним разбором на оба случая.
        //
        // Черновик так сохранить можно: он затем и черновик.
        let readiness = Self::card_readiness(req);
        if let Some(fault) = readiness.blocking.first() {
            return Err(AppError::BadRequest(format!("card:{fault}")));
        }
        // Черновик из студии может прийти на одном языке — гость пишет на
        // своём, а переводит хозяин. На люди же без обязательного не выходит
        // ни одна карта, каким бы путём она ни пришла.
        // Карту «только для этюдов» гость тоже видит на обоих языках — на
        // поле этюда, — поэтому и ей без обязательного нельзя.
        if req.status == "published" || req.status == crate::battles::CARD_ETUDE_ONLY {
            if let Some(field) = readiness.missing.first() {
                return Err(AppError::BadRequest(format!("card:missing:{field}")));
            }
        }
        let title_en = crate::battles::clamp_title(&req.title_en);
        let title_ru = crate::battles::clamp_title(&req.title_ru);
        if title_en.is_empty() && title_ru.is_empty() {
            return Err(AppError::BadRequest("A title is required".into()));
        }
        // An empty English title used to be filled from the Russian one, and
        // the English shelf then printed Cyrillic as if it were English.
        // A draft may still have one language; publishing both is a blocker
        // in `prose_blockers`, not a silent copy.
        let price_dust = crate::battles::clamp_price(req.price_dust);
        let price_feed = crate::battles::clamp_price(req.price_feed);
        // Снаряжение этюда не продаётся, и цена ему не нужна.
        if price_dust.is_none() && price_feed.is_none() && req.status != crate::battles::CARD_ETUDE_ONLY {
            return Err(AppError::BadRequest(
                "A card needs a price in at least one coin".into(),
            ));
        }
        let figurine_id = match req
            .figurine_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            None => None,
            Some(s) => Some(
                Uuid::parse_str(s)
                    .map_err(|_| AppError::BadRequest("Invalid figurineId".into()))?,
            ),
        };
        let race_id = match req
            .race_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            None => None,
            Some(s) => Some(
                Uuid::parse_str(s).map_err(|_| AppError::BadRequest("Invalid raceId".into()))?,
            ),
        };
        let kind = if crate::battles::valid_kind(req.kind.trim()) {
            req.kind.trim().to_string()
        } else {
            crate::battles::default_kind()
        };
        let attack_channel = if crate::battles::valid_channel(req.attack_channel.trim()) {
            req.attack_channel.trim().to_string()
        } else {
            crate::battles::default_channel()
        };
        let armor = crate::battles::clamp_defence(req.armor);
        let ward = crate::battles::clamp_defence(req.ward);
        let reach = crate::battles::clamp_reach(req.reach);
        let step = crate::battles::clamp_step(req.step);
        let speed = crate::battles::clamp_speed(req.speed);
        let mend = crate::battles::clamp_mend(req.mend);
        let health = crate::battles::clamp_stat(req.health);
        let power = crate::battles::clamp_power(req.power);
        let tier = crate::battles::clamp_tier(req.tier);
        let cost = crate::battles::clamp_cost(req.cost);
        let abilities = crate::battles::normalize_abilities(&req.abilities);

        // The scales run on every save, on the clamped numbers rather than the
        // typed ones — so the verdict is about the card that was actually
        // written, not the one that was asked for.
        let points = crate::battles::body_points(
            health,
            armor,
            ward,
            crate::battles::striking_power(power, &attack_channel),
            reach,
            step,
            mend,
        ) + crate::battles::read_abilities(abilities.as_deref())
            .iter()
            .map(|a| crate::battles::ability_points(a, tier))
            .sum::<f64>();
        let index = crate::battles::balance_index(points, cost);

        Ok(BattleCardWrite {
            slug: crate::battles::unique_slug(
                req.slug.as_deref(),
                if title_en.is_empty() {
                    &title_ru
                } else {
                    &title_en
                },
                taken,
            ),
            figurine_id,
            race_id,
            status: req.status.clone(),
            tier,
            type_en: crate::battles::clamp_type(req.type_en.as_deref()),
            type_ru: crate::battles::clamp_type(req.type_ru.as_deref()),
            title_en,
            title_ru,
            effect_en: crate::battles::clamp_effect(req.effect_en.as_deref()),
            effect_ru: crate::battles::clamp_effect(req.effect_ru.as_deref()),
            lore_en: crate::battles::clamp_lore(req.lore_en.as_deref()),
            lore_ru: crate::battles::clamp_lore(req.lore_ru.as_deref()),
            type_uk: crate::battles::clamp_type(req.type_uk.as_deref()),
            type_es: crate::battles::clamp_type(req.type_es.as_deref()),
            type_de: crate::battles::clamp_type(req.type_de.as_deref()),
            type_it: crate::battles::clamp_type(req.type_it.as_deref()),
            title_uk: crate::battles::clamp_title(&req.title_uk),
            title_es: crate::battles::clamp_title(&req.title_es),
            title_de: crate::battles::clamp_title(&req.title_de),
            title_it: crate::battles::clamp_title(&req.title_it),
            effect_uk: crate::battles::clamp_effect(req.effect_uk.as_deref()),
            effect_es: crate::battles::clamp_effect(req.effect_es.as_deref()),
            effect_de: crate::battles::clamp_effect(req.effect_de.as_deref()),
            effect_it: crate::battles::clamp_effect(req.effect_it.as_deref()),
            lore_uk: crate::battles::clamp_lore(req.lore_uk.as_deref()),
            lore_es: crate::battles::clamp_lore(req.lore_es.as_deref()),
            lore_de: crate::battles::clamp_lore(req.lore_de.as_deref()),
            lore_it: crate::battles::clamp_lore(req.lore_it.as_deref()),
            cost,
            power,
            health,
            mana: crate::battles::clamp_stat(req.mana),
            traits: crate::battles::normalize_traits(&req.traits),
            kind,
            armor,
            ward,
            attack_channel,
            reach,
            step,
            speed,
            mend,
            abilities,
            budget_points: Some((points * 100.0).round() / 100.0),
            balance_index: Some((index * 100.0).round() / 100.0),
            price_dust,
            price_feed,
            level_price_dust: crate::battles::clamp_level_prices(req.level_price_dust.as_deref()),
            art_url: req
                .art_url
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            art_focal: crate::battles::normalize_focal(req.art_focal.as_deref()),
            frame_override: crate::battles::normalize_frame_override(req.frame_override.as_deref()),
            motion_wear: crate::battles::normalize_motion_wear(req.motion_wear.as_deref()),
            lendable: req.lendable,
            shoots: req.shoots,
            // Тираж — целое больше нуля или ничего. Ноль пришедший из формы
            // значит «не ограничивать», а не «не печатать ни одной».
            edition_size: req.edition_size.filter(|n| *n > 0),
            // Подпись не приходит из формы: её ставит студия при утверждении.
            credit_name: None,
        })
    }

    /// Weigh a card that has not been saved.
    ///
    /// Clamps exactly as a save would, so the verdict is about the card that
    /// *would* be written rather than the one that was typed — a keeper who
    /// enters reach 9 should see the number for reach 5.
    /// Годность карты — одним разбором и для весов, и для отказа при сохранении.
    fn card_readiness(req: &SaveBattleCardRequest) -> crate::models::CardReadinessDto {
        let title_en = crate::battles::clamp_title(&req.title_en);
        let title_ru = crate::battles::clamp_title(&req.title_ru);
        let effect_en = crate::battles::clamp_effect(req.effect_en.as_deref());
        let effect_ru = crate::battles::clamp_effect(req.effect_ru.as_deref());
        let mut blocking: Vec<String> = crate::battles::card_blockers(
            &req.status,
            crate::battles::clamp_stat(req.health),
            crate::battles::clamp_cost(req.cost),
            Self::weigh_points(req).0,
            crate::battles::clamp_tier(req.tier),
        )
        .into_iter()
        .map(str::to_string)
        .collect();
        let stored_abilities = crate::battles::read_abilities(
            crate::battles::normalize_abilities(&req.abilities).as_deref(),
        );
        blocking.extend(
            crate::battles::prose_blockers(&req.status, &req.traits, !stored_abilities.is_empty())
                .into_iter()
                .map(str::to_string),
        );
        let missing: Vec<String> = crate::battles::missing_fields(
            &title_en,
            &title_ru,
            effect_en.as_deref(),
            effect_ru.as_deref(),
            &req.traits,
            &stored_abilities,
            crate::battles::clamp_price(req.price_dust),
            crate::battles::clamp_price(req.price_feed),
        )
        .into_iter()
        // Снаряжение этюда не продаётся: цена у него не обязательна.
        .filter(|f| !(*f == "price" && req.status == crate::battles::CARD_ETUDE_ONLY))
        .map(str::to_string)
        .collect();
        blocking.extend(
            crate::battles::ability_blockers(&req.abilities, &req.status)
                .into_iter()
                .map(str::to_string),
        );
        let mut notes: Vec<String> = crate::battles::card_notes(
            &req.status,
            crate::battles::clamp_tier(req.tier),
            req.lendable,
            crate::battles::clamp_price(req.price_dust),
            crate::battles::clamp_price(req.price_feed),
        )
        .into_iter()
        .map(str::to_string)
        .collect();
        if crate::battles::type_is_numeric(req.type_en.as_deref())
            || crate::battles::type_is_numeric(req.type_ru.as_deref())
        {
            notes.push("numericType".into());
        }
        crate::models::CardReadinessDto {
            missing,
            blocking,
            notes,
        }
    }

    /// Вес карты: тело, способности и сумма. Одним счётом и для весов, и для
    /// потолка — два счёта однажды разошлись бы, и хранитель увидел бы одно
    /// число, а отказ пришёл бы по другому.
    ///
    /// Способности читаются через `normalize/read`, а не прямо из запроса:
    /// строка с неизвестным глаголом при сохранении отбрасывается, и весить
    /// надо то, что будет записано.
    fn weigh_points(req: &SaveBattleCardRequest) -> (f64, f64, Vec<crate::battles::CardAbility>) {
        let tier = crate::battles::clamp_tier(req.tier);
        let body = crate::battles::body_points(
            crate::battles::clamp_stat(req.health),
            crate::battles::clamp_defence(req.armor),
            crate::battles::clamp_defence(req.ward),
            crate::battles::striking_power(
                crate::battles::clamp_power(req.power),
                req.attack_channel.trim(),
            ),
            crate::battles::clamp_reach(req.reach),
            crate::battles::clamp_step(req.step),
            crate::battles::clamp_mend(req.mend),
        );
        let stored = crate::battles::read_abilities(
            crate::battles::normalize_abilities(&req.abilities).as_deref(),
        );
        let total = body
            + stored
                .iter()
                .map(|a| crate::battles::ability_points(a, tier))
                .sum::<f64>();
        (total, body, stored)
    }

    /// Весы гостя студии: те же, что у стола хозяина, и обязательное в них —
    /// отказ, а не отдельный список. Одна функция и для весов, и для
    /// отдачи хозяину, иначе гость увидел бы «годна», а приём отказал.
    pub fn weigh_studio_card(req: &SaveBattleCardRequest) -> BattleWeighDto {
        let mut weighed = Self::weigh_battle_card(req);
        let guest = crate::battles::guest_faults(&weighed.readiness.missing);
        weighed.readiness.blocking.splice(0..0, guest);
        weighed
    }

    pub fn weigh_battle_card(req: &SaveBattleCardRequest) -> BattleWeighDto {
        let tier = crate::battles::clamp_tier(req.tier);
        let cost = crate::battles::clamp_cost(req.cost);
        let readiness = Self::card_readiness(req);
        let (total, body, stored) = Self::weigh_points(req);
        let abilities: Vec<AbilityWeightDto> = stored
            .iter()
            .map(|a| AbilityWeightDto {
                id: a.id.clone(),
                points: round2(crate::battles::ability_points(a, tier)),
            })
            .collect();

        BattleWeighDto {
            body_points: round2(body),
            abilities,
            total_points: round2(total),
            balance_index: round2(crate::battles::balance_index(total, cost)),
            tier_budget: crate::battles::tier_budget(tier),
            // The price at which this weight would sit on the curve, from
            // `points ≈ 4 × cost + 2`.
            suggested_cost: (((total - 2.0) / 4.0).round() as i16)
                .clamp(0, crate::battles::COST_MAX),
            readiness,
        }
    }

    /// Сыгранные партии и что из них следует.
    ///
    /// Единственный источник правды о живой игре. Сводка по картам считается по
    /// ЗАМОРОЖЕННОЙ расстановке каждой партии, а не по нынешней полке: карта
    /// могла быть с тех пор переписана, а выиграла или проиграла та, что стояла
    /// на доске. Слуг в снимке — то же имя, каким карта звалась тогда.
    pub async fn admin_read_battle_matches(&self) -> Result<crate::models::BattleMatchesDto> {
        let rows = self
            .repo
            .admin_list_battle_matches(crate::battles::MATCHES_SHOWN)
            .await?;

        // Имена карт по слугу. Из админской полки, а не из публичной: партия
        // могла быть сыграна картой, которую с тех пор сняли.
        let titles: std::collections::HashMap<String, [String; 6]> = self
            .repo
            .list_battle_cards_admin(crate::battles::SHELF_CARDS)
            .await?
            .into_iter()
            .map(|c| {
                (
                    c.slug.clone(),
                    [
                        c.title_ru.clone(),
                        c.title_en.clone(),
                        c.title_uk.clone(),
                        c.title_es.clone(),
                        c.title_de.clone(),
                        c.title_it.clone(),
                    ],
                )
            })
            .collect();

        let mut by_challenge: Vec<crate::models::BattleChallengeTally> = Vec::new();
        let mut cards: std::collections::HashMap<String, crate::models::BattleCardTally> =
            std::collections::HashMap::new();
        let mut out_rows = Vec::with_capacity(rows.len());

        for row in &rows {
            let moves = serde_json::from_str::<Vec<battle_core::Action>>(&row.actions)
                .map(|a| a.len() as i64)
                .unwrap_or(0);

            // ── по испытаниям ───────────────────────────────────────────────
            let key = row.challenge_id.map(|id| id.to_string());
            let tally = match by_challenge.iter_mut().find(|t| t.challenge_id == key) {
                Some(t) => t,
                None => {
                    by_challenge.push(crate::models::BattleChallengeTally {
                        challenge_id: key.clone(),
                        title_ru: row.title_ru.clone(),
                        title_uk: row.title_uk.clone(),
                        title_es: row.title_es.clone(),
                        title_de: row.title_de.clone(),
                        title_it: row.title_it.clone(),
                        title_en: row.title_en.clone(),
                        played: 0,
                        guest_won: 0,
                        keeper_won: 0,
                        draws: 0,
                        unfinished: 0,
                    });
                    by_challenge.last_mut().expect("только что добавлено")
                }
            };
            tally.played += 1;
            match row.outcome.as_deref() {
                Some("player") => tally.guest_won += 1,
                Some("keeper") => tally.keeper_won += 1,
                Some("draw") => tally.draws += 1,
                _ => tally.unfinished += 1,
            }

            // ── по картам ───────────────────────────────────────────────────
            if let (Some(outcome), Ok(setup)) = (
                row.outcome.as_deref(),
                serde_json::from_str::<battle_core::Setup>(&row.setup),
            ) {
                let names = |board: &[(battle_core::CardSnapshot, battle_core::Cell)],
                             hand: &[battle_core::CardSnapshot]| {
                    board
                        .iter()
                        .map(|(c, _)| c.name.clone())
                        .chain(hand.iter().map(|c| c.name.clone()))
                        .collect::<std::collections::HashSet<String>>()
                };
                let sides = [
                    (names(&setup.player_board, &setup.player_hand), "player"),
                    (names(&setup.keeper_board, &setup.keeper_hand), "keeper"),
                ];
                for (slugs, side) in sides {
                    for slug in slugs {
                        let named = titles.get(&slug);
                        let tally = cards.entry(slug.clone()).or_insert_with(|| {
                            crate::models::BattleCardTally {
                                slug: slug.clone(),
                                title_ru: named.map(|t| t[0].clone()),
                                title_en: named.map(|t| t[1].clone()),
                                title_uk: named.map(|t| t[2].clone()),
                                title_es: named.map(|t| t[3].clone()),
                                title_de: named.map(|t| t[4].clone()),
                                title_it: named.map(|t| t[5].clone()),
                                played: 0,
                                won: 0,
                                lost: 0,
                                draws: 0,
                            }
                        });
                        tally.played += 1;
                        if outcome == "draw" {
                            tally.draws += 1;
                        } else if outcome == side {
                            tally.won += 1;
                        } else {
                            tally.lost += 1;
                        }
                    }
                }
            }

            out_rows.push(crate::models::BattleMatchRowDto {
                id: row.id.to_string(),
                guest: row.guest.clone(),
                challenge_id: key,
                title_ru: row.title_ru.clone(),
                title_uk: row.title_uk.clone(),
                title_es: row.title_es.clone(),
                title_de: row.title_de.clone(),
                title_it: row.title_it.clone(),
                title_en: row.title_en.clone(),
                outcome: row.outcome.clone(),
                rounds: row.rounds,
                moves,
                started_at: row.created_at.to_rfc3339(),
                finished_at: row.finished_at.map(|t| t.to_rfc3339()),
            });
        }

        let mut by_card: Vec<crate::models::BattleCardTally> = cards.into_values().collect();
        // Часто выходившие вперёд: по ним число что-то значит. Ровные — по слугу,
        // чтобы порядок не плясал от запроса к запросу.
        by_card.sort_by(|a, b| b.played.cmp(&a.played).then_with(|| a.slug.cmp(&b.slug)));
        by_challenge.sort_by(|a, b| b.played.cmp(&a.played));

        Ok(crate::models::BattleMatchesDto {
            read: out_rows.len() as i64,
            rows: out_rows,
            by_challenge,
            by_card,
        })
    }

    /// Пересмотр записанной партии, ступень за ступенью.
    ///
    /// Правила берутся ИЗ ЗАПИСИ, а не нынешние. Это не мелочь: умолчания
    /// `Rules` за одну неделю менялись трижды, и партия, переигранная новыми
    /// правилами, разошлась бы с тем, что человек видел своими глазами, — а то
    /// и вовсе перестала бы переигрываться на первом же ходу.
    ///
    /// Если запись всё-таки разошлась (правила менялись и до того, как их стали
    /// хранить), доска показывается до места расхождения, а не заменяется
    /// отказом: половина партии честнее пустого экрана.
    pub async fn admin_replay_battle_match(
        &self,
        id: Uuid,
        upto: usize,
    ) -> Result<crate::models::MatchReplayDto> {
        let rec = self
            .repo
            .admin_get_battle_match(id)
            .await?
            .ok_or_else(|| AppError::NotFound("No such match".into()))?;

        let setup: battle_core::Setup = serde_json::from_str(&rec.setup)
            .map_err(|_| AppError::Internal("Frozen arrangement will not read".into()))?;
        let actions: Vec<battle_core::Action> = serde_json::from_str(&rec.actions)
            .map_err(|_| AppError::Internal("Match journal will not read".into()))?;

        let rules = Self::match_rules(&rec);

        let want = upto.min(actions.len());
        let mut state = battle_core::MatchState::begin_with(setup, rules);
        let mut events = Vec::new();
        let mut done = 0usize;
        let mut diverged = false;
        for action in actions.iter().take(want) {
            match battle_core::reduce(&state, action) {
                Ok((next, produced)) => {
                    state = next;
                    // Показываются события ПОСЛЕДНЕЙ ступени: сцена проигрывает
                    // один ход, а не всю партию разом.
                    events = produced;
                    done += 1;
                }
                Err(_) => {
                    diverged = true;
                    break;
                }
            }
        }

        Ok(crate::models::MatchReplayDto {
            outcome: state.outcome.map(|o| match o {
                battle_core::Outcome::Player => "player".to_string(),
                battle_core::Outcome::Keeper => "keeper".to_string(),
                battle_core::Outcome::Draw => "draw".to_string(),
            }),
            state,
            events,
            upto: done,
            total: actions.len(),
            actions,
            diverged,
        })
    }

    // === CHALLENGES AND MATCHES ===

    /// The key a grant is written under. Per challenge, never per victory —
    /// otherwise the easiest challenge becomes a dust mill.
    fn challenge_key(challenge_id: Uuid) -> String {
        format!("battle:{challenge_id}")
    }

    /// Ключ за доведённое до конца.
    ///
    /// Свой, и по-прежнему ОДИН на этюд: переигрывать можно сколько угодно,
    /// заплатят однажды. Прежний ключ победы не тронут — под ним уже лежат
    /// выплаты, и приписать к ним хвост значило бы заплатить всё второй раз.
    fn challenge_finish_key(challenge_id: Uuid) -> String {
        format!("battle:{challenge_id}:finish")
    }

    fn challenge_dto(
        row: crate::models::BattleChallenge,
        paid_keys: Option<&[String]>,
        open_match_id: Option<String>,
        marks: Option<crate::models::ChallengeMarks>,
    ) -> BattleChallengeDto {
        let key = Self::challenge_key(row.id);
        BattleChallengeDto {
            id: row.id.to_string(),
            slug: row.slug,
            title_en: row.title_en,
            title_ru: row.title_ru,
            title_uk: row.title_uk,
            title_es: row.title_es,
            title_de: row.title_de,
            title_it: row.title_it,
            note_en: row.note_en,
            note_ru: row.note_ru,
            note_uk: row.note_uk,
            note_es: row.note_es,
            note_de: row.note_de,
            note_it: row.note_it,
            setup: serde_json::from_str(&row.setup).unwrap_or_default(),
            bot_depth: crate::battles::clamp_bot_depth(row.bot_depth),
            reward_dust: row.reward_dust,
            reward_finish_dust: row.reward_finish_dust,
            player_side: row.player_side,
            status: row.status,
            sort_order: row.sort_order,
            already_paid: paid_keys.map(|keys| keys.iter().any(|k| *k == key)),
            open_match_id,
            marks,
        }
    }

    pub async fn list_battle_challenges(
        &self,
        user_id: Option<Uuid>,
        published_only: bool,
    ) -> Result<Vec<BattleChallengeDto>> {
        let rows = self.repo.list_battle_challenges(published_only).await?;
        // One query for the whole shelf rather than one per challenge.
        let paid = match user_id {
            Some(id) => Some(self.repo.battle_wallet_keys(id).await?),
            None => None,
        };
        let open: HashMap<Uuid, String> = match user_id {
            Some(id) => self
                .repo
                .list_open_battle_match_ids(id)
                .await?
                .into_iter()
                .map(|(challenge_id, match_id)| (challenge_id, match_id.to_string()))
                .collect(),
            None => HashMap::new(),
        };

        // Печати. Тоже по одному запросу на полку, а не по одному на этюд.
        //
        // Планка спрашивается ВСЕГДА, когда гость вошёл, даже если сам он этот
        // этюд ещё не открывал: «лучшее известное — пять» это то, ради чего за
        // доску садятся, а не то, что показывают в награду за победу.
        let (mine, bar) = match user_id {
            Some(id) => (
                self.repo.battle_challenge_marks(id).await?,
                self.repo.battle_challenge_best_lines().await?,
            ),
            None => (Vec::new(), Vec::new()),
        };
        let mine: HashMap<Uuid, (bool, bool, bool, Option<i16>)> = mine
            .into_iter()
            .map(|(id, finished, won, clean, best)| (id, (finished, won, clean, best)))
            .collect();
        let bar: HashMap<Uuid, i16> = bar.into_iter().collect();

        Ok(rows
            .into_iter()
            .map(|r| {
                let open_match_id = open.get(&r.id).cloned();
                let marks = user_id.map(|_| {
                    let (finished, won, clean, your_best) =
                        mine.get(&r.id).copied().unwrap_or((false, false, false, None));
                    crate::models::ChallengeMarks {
                        finished,
                        won,
                        clean,
                        your_best: your_best.map(|b| b as i32),
                        best_known: bar.get(&r.id).map(|b| *b as i32),
                    }
                });
                Self::challenge_dto(r, paid.as_deref(), open_match_id, marks)
            })
            .collect())
    }

    pub async fn admin_save_battle_challenge(
        &self,
        id: Option<Uuid>,
        req: SaveBattleChallengeRequest,
    ) -> Result<BattleChallengeDto> {
        if !crate::battles::valid_status(&req.status) {
            return Err(AppError::BadRequest("Unknown challenge status".into()));
        }
        let mut title_en = crate::battles::clamp_title(&req.title_en);
        let mut title_ru = crate::battles::clamp_title(&req.title_ru);
        if title_en.is_empty() && title_ru.is_empty() {
            return Err(AppError::BadRequest("A challenge needs a title".into()));
        }
        if title_en.is_empty() {
            title_en = title_ru.clone();
        }
        if title_ru.is_empty() {
            title_ru = title_en.clone();
        }
        let title_uk = crate::battles::clamp_title(&req.title_uk);
        let title_es = crate::battles::clamp_title(&req.title_es);
        let title_de = crate::battles::clamp_title(&req.title_de);
        let title_it = crate::battles::clamp_title(&req.title_it);

        if !crate::battles::valid_player_side(&req.player_side) {
            return Err(AppError::BadRequest("Unknown side for the guest".into()));
        }
        // Refused before it is written rather than found when a guest clicks it:
        // an arrangement that cannot be raised is not a challenge, it is a trap.
        //
        // У встречи проверяется только половина хранителя: половину гостя
        // приносит его стол, и требовать её здесь значило бы требовать чужого.
        self.assert_setup_can_be_raised(&req.setup, &req.player_side)
            .await?;

        let taken = self.repo.list_battle_challenge_slugs_except(id).await?;
        // Правила зажимаются ЗДЕСЬ, при записи, а не при начале партии: иначе
        // форма показывала бы хранителю одно число, а гость играл бы под другим,
        // и разошлись бы они молча.
        let mut arrangement = req.setup;
        arrangement.rules = arrangement.rules.as_ref().map(crate::battles::normalize_rules);
        let setup = serde_json::to_string(&arrangement)
            .map_err(|_| AppError::BadRequest("Arrangement will not serialise".into()))?;
        let slug = crate::battles::unique_slug(req.slug.as_deref(), &title_en, &taken);
        let rec = self
            .repo
            .upsert_battle_challenge(
                id,
                &crate::models::BattleChallengeWrite {
                    slug: &slug,
                    title_en: &title_en,
                    title_ru: &title_ru,
                    note_en: req
                        .note_en
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty()),
                    note_ru: req
                        .note_ru
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty()),
                    title_uk: &title_uk,
                    title_es: &title_es,
                    title_de: &title_de,
                    title_it: &title_it,
                    note_uk: req
                        .note_uk
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty()),
                    note_es: req
                        .note_es
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty()),
                    note_de: req
                        .note_de
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty()),
                    note_it: req
                        .note_it
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty()),
                    setup: &setup,
                    bot_depth: crate::battles::clamp_bot_depth(req.bot_depth),
                    reward_dust: req.reward_dust.clamp(0, 1000),
                    reward_finish_dust: req.reward_finish_dust.clamp(0, 1000),
                    player_side: &req.player_side,
                    status: &req.status,
                },
            )
            .await?;
        Self::log_domain_event("battle_challenge_saved", "battle_challenge", rec.id, "ok");
        Ok(Self::challenge_dto(rec, None, None, None))
    }

    /// Порядок полки этюдов. Та же проверка, что и у карт: испытание не может
    /// стоять на полке в двух местах, и полка не может быть длиннее комнаты.
    pub async fn admin_reorder_battle_challenges(&self, ids: Vec<Uuid>) -> Result<()> {
        if ids.len() > crate::battles::SHELF_CARDS as usize {
            return Err(AppError::BadRequest("Shelf is longer than the room".into()));
        }
        let mut seen = std::collections::HashSet::with_capacity(ids.len());
        if !ids.iter().all(|id| seen.insert(*id)) {
            return Err(AppError::BadRequest(
                "A challenge cannot stand in two places on the shelf".into(),
            ));
        }
        self.repo.set_battle_challenge_order(&ids).await?;
        Ok(())
    }

    /// Запустить проверку проходимости в фоне: те же карты и правила, что
    /// получит гость, партиями ботов (`siege::probe`).
    pub async fn admin_start_battle_probe(
        &self,
        req: crate::models::BattleProbeRequest,
    ) -> Result<crate::models::BattleProbeStarted> {
        let cards = self.cards_by_slug().await?;
        if req.setup.player_board.is_empty() || req.setup.keeper_board.is_empty() {
            return Err(AppError::BadRequest("probe:needsBothSides".into()));
        }
        let setup = Self::raise(&req.setup, &cards)?;
        let rules = crate::battles::rules_of(&req.setup);
        let depth = req.bot_depth.clamp(1, battle_core::bot::DEPTH_MAX as i16) as u8;
        let id = crate::siege::start_probe(setup, rules, depth);
        Ok(crate::models::BattleProbeStarted { id: id.to_string() })
    }

    pub fn admin_read_battle_probe(&self, id: Uuid) -> Result<crate::siege::ProbeState> {
        crate::siege::read_probe(id).ok_or_else(|| AppError::NotFound("No such probe".into()))
    }

    pub async fn admin_delete_battle_challenge(&self, id: Uuid) -> Result<()> {
        self.repo.delete_battle_challenge(id).await?;
        Self::log_domain_event("battle_challenge_deleted", "battle_challenge", id, "ok");
        Ok(())
    }

    /// Every card an arrangement may name, by slug — published ones and those
    /// kept for etudes only (walls, towers, a ram).
    async fn cards_by_slug(&self) -> Result<std::collections::BTreeMap<String, BattleCardDto>> {
        Ok(self
            .repo
            .list_battle_cards_for_etudes(crate::battles::SHELF_CARDS)
            .await?
            .into_iter()
            .map(|r| self.battle_card_dto(r, false))
            .filter(crate::battles::can_stand_in_an_etude)
            .map(|c| (c.slug.clone(), c))
            .collect())
    }

    async fn assert_setup_can_be_raised(
        &self,
        setup: &ChallengeSetup,
        player_side: &str,
    ) -> Result<()> {
        let cards = self.cards_by_slug().await?;
        let mut named: Vec<&String> = setup.player_hand.iter().chain(&setup.keeper_hand).collect();
        named.extend(setup.player_board.iter().map(|p| &p.card));
        named.extend(setup.keeper_board.iter().map(|p| &p.card));
        for slug in named {
            if !cards.contains_key(slug) {
                return Err(AppError::BadRequest(format!(
                    "No card for etudes with health under the name {slug}"
                )));
            }
        }
        if setup.keeper_board.is_empty() {
            return Err(AppError::BadRequest(
                "The keeper needs someone standing".into(),
            ));
        }
        // Половина гостя обязательна только у этюда. У встречи её приносит стол,
        // и требовать её от хранителя значило бы требовать чужого.
        if player_side == crate::battles::SIDE_SCRIPTED && setup.player_board.is_empty() {
            return Err(AppError::BadRequest(
                "Both sides need someone standing".into(),
            ));
        }
        // Cells are checked by raising the arrangement for real: the engine
        // knows what a legal cell is, and asking it here means the rule is not
        // written down twice.
        Self::raise(setup, &cards)?;
        Ok(())
    }

    /// Turn an arrangement of slugs into the setup the engine fights with.
    fn raise(
        setup: &ChallengeSetup,
        cards: &std::collections::BTreeMap<String, BattleCardDto>,
    ) -> Result<battle_core::Setup> {
        // Полка под рукой: чара призыва находит по слагу то, что призывает, и
        // замораживает его вместе с умением.
        let body = |slug: &str| -> Result<battle_core::CardSnapshot> {
            cards
                .get(slug)
                .map(|c| crate::battles::to_snapshot_with(c, |want| cards.get(want)))
                .ok_or_else(|| AppError::BadRequest(format!("Unknown card {slug}")))
        };
        let field = crate::battles::check_field(setup.field).map_err(|e| AppError::BadRequest(e.into()))?;
        crate::battles::check_background(&setup.background).map_err(|e| AppError::BadRequest(e.into()))?;
        let place = |list: &[crate::models::ChallengePlacement]| -> Result<Vec<_>> {
            list.iter()
                .map(|p| {
                    let cell = field
                        .cell(p.x, p.y)
                        .ok_or_else(|| AppError::BadRequest("No such cell on the field".into()))?;
                    Ok((body(&p.card)?, cell))
                })
                .collect()
        };
        let hand = |list: &[String]| -> Result<Vec<_>> { list.iter().map(|s| body(s)).collect() };

        Ok(battle_core::Setup {
            player_board: place(&setup.player_board)?,
            player_hand: hand(&setup.player_hand)?,
            keeper_board: place(&setup.keeper_board)?,
            keeper_hand: hand(&setup.keeper_hand)?,
            terrain: crate::battles::check_terrain(setup).map_err(|e| AppError::BadRequest(e.into()))?,
            field,
        })
    }

    /// Что выходит на поле со стороны гостя: его расстановка на поле этой
    /// встречи, с досчитанным заёмом.
    ///
    /// Заём считается ровно там же, где его видит окно расстановки, —
    /// `read_battle_lineup`. Второй расчёт заёма означал бы, что человек видит
    /// перед боем одно, а на поле у него встаёт другое.
    ///
    /// Законность перечитывается ЗДЕСЬ, а не берётся из сохранения: между
    /// сохранением и «В бой» карта могла уйти, а ранг — подняться.
    async fn lineup_takes_the_field(
        &self,
        user_id: Uuid,
        challenge_id: Uuid,
        setup: &ChallengeSetup,
    ) -> Result<(
        Vec<(battle_core::CardSnapshot, battle_core::Cell)>,
        Vec<battle_core::CardSnapshot>,
    )> {
        let ground = crate::battles::LineupGround::of(setup);
        let loan = self.loan_inputs(user_id).await?;
        let table = self.lineup_on(user_id, challenge_id, setup, &loan).await?;

        // Своё — только то, что гость выбрал сам и что ещё выходит на поле.
        // Заём в проверку не идёт: его не выбирают и им не владеют.
        let kept_board: Vec<(Uuid, u8, u8)> = table
            .board
            .iter()
            .filter(|s| !s.gone)
            .filter_map(|s| Some((s.card_id.as_deref()?.parse().ok()?, s.x?, s.y?)))
            .collect();
        let kept_hand: Vec<Uuid> = table
            .hand
            .iter()
            .filter(|s| !s.gone)
            .filter_map(|s| s.card_id.as_deref()?.parse().ok())
            .collect();
        crate::battles::check_lineup(&ground, &kept_board, &kept_hand, &loan.owned, &loan.tier_of())
            .map_err(|fault| AppError::BadRequest(format!("deck:{}", fault.word())))?;

        let (board, hand) = Self::bodies_of(&table, &loan)?;

        // Отказ только если приводить НЕЧЕГО — ни на поле, ни в руке.
        //
        // Проверки `board.is_empty()` здесь нет и не должно быть: нет тел на
        // поле — поражение (`is_spent`), но поле пустым не приходит, пустые
        // места закрывает заём. Проверить это здесь второй раз значило бы
        // переписать правило движка своим.
        if board.is_empty() && hand.is_empty() {
            return Err(AppError::BadRequest("deck:nothingToBring".into()));
        }
        Ok((board, hand))
    }

    /// Что фактически встаёт: свой выбор, а где его нет — заём. Снимки
    /// собираются по полке (`loan.cards`), призыв находит своё по слагу.
    fn bodies_of(
        table: &crate::models::BattleLineupDto,
        loan: &LoanInputs,
    ) -> Result<(
        Vec<(battle_core::CardSnapshot, battle_core::Cell)>,
        Vec<battle_core::CardSnapshot>,
    )> {
        let by_id: std::collections::HashMap<Uuid, &BattleCardDto> = loan
            .cards
            .iter()
            .filter_map(|c| c.id.parse().ok().map(|id| (id, c)))
            .collect();
        let by_slug: std::collections::HashMap<&str, &BattleCardDto> =
            loan.cards.iter().map(|c| (c.slug.as_str(), c)).collect();
        let body = |id: Uuid| -> Result<battle_core::CardSnapshot> {
            by_id
                .get(&id)
                .map(|c| crate::battles::to_snapshot_with(c, |want| by_slug.get(want).copied()))
                .ok_or_else(|| AppError::BadRequest("deck:nothingToBring".into()))
        };
        let mut board = Vec::new();
        for slot in &table.board {
            let Some(id) = Self::slot_takes_the_field(slot) else {
                continue;
            };
            let (Some(x), Some(y)) = (slot.x, slot.y) else {
                continue;
            };
            let cell = battle_core::Cell::new(x, y)
                .ok_or_else(|| AppError::BadRequest("deck:notYourHalf".into()))?;
            board.push((body(id)?, cell));
        }
        let mut hand = Vec::new();
        for slot in &table.hand {
            if let Some(id) = Self::slot_takes_the_field(slot) {
                hand.push(body(id)?);
            }
        }
        Ok((board, hand))
    }

    /// Одно место стола, сведённое к одной карте: свой выбор, а если его нет
    /// или он снят с полки — то, что дом одолжил.
    fn slot_takes_the_field(slot: &crate::models::BattleDeckSlotDto) -> Option<Uuid> {
        let chosen = if slot.gone {
            None
        } else {
            slot.card_id.as_deref()
        };
        chosen
            .or(slot.lent_card_id.as_deref())
            .and_then(|s| s.parse().ok())
    }

    /// Правила, под которыми игралась записанная партия.
    ///
    /// Колонка первой, кэш вторым, дом третьим. Кэш читается не из вежливости к
    /// старым записям: до колонки правила лежали ТОЛЬКО в нём, внутри снимка
    /// состояния, и партия, начатая до этой миграции, называет их именно там.
    /// Дом последним — им и игралось всё, что старше обоих мест.
    fn match_rules(rec: &crate::models::BattleMatch) -> battle_core::Rules {
        if let Some(raw) = rec.rules.as_deref() {
            if let Ok(rules) = serde_json::from_str(raw) {
                return rules;
            }
        }
        rec.board_cache
            .as_deref()
            .and_then(|c| serde_json::from_str::<battle_core::MatchState>(c).ok())
            .map(|s| s.rules)
            .unwrap_or_default()
    }

    /// Fold a journal back into a board. The only definition of "the state of a
    /// match" there is — the cache is a convenience, this is the truth.
    ///
    /// Правила приходят снаружи и обязаны быть теми же, под которыми партия
    /// игралась: свёртка чужими правилами даёт другую доску молча — то же
    /// действие законно, а число другое, — и кэш перестал бы быть кэшем.
    fn replay(
        setup: &battle_core::Setup,
        actions: &[battle_core::Action],
        rules: battle_core::Rules,
    ) -> Result<battle_core::MatchState> {
        let mut state = battle_core::MatchState::begin_with(setup.clone(), rules);
        for action in actions {
            state = battle_core::reduce(&state, action)
                .map_err(|e| AppError::Internal(format!("Recorded match will not replay: {e:?}")))?
                .0;
        }
        Ok(state)
    }

    /// Чем пройдена партия: сколько дел сделал гость и скольких тел лишился.
    ///
    /// **Конец хода делом не считается.** Это не выбор из нескольких, а его
    /// отсутствие, и партия, выигранная пятью ударами в трёх ходах, не должна
    /// читаться длиннее той же партии в пяти ходах: печать мерит решения, а не
    /// нажатия.
    ///
    /// Свёртка идёт по журналу с самого начала, потому что чья очередь была на
    /// каждом действии, знает только доска. Останавливается молча на первом же
    /// действии, которое не применилось: разошедшаяся запись — повод показать
    /// меньше, а не повод отказать в исходе, который уже случился.
    fn count_the_line(
        setup: &battle_core::Setup,
        actions: &[battle_core::Action],
        rules: battle_core::Rules,
    ) -> (i16, i16) {
        let mut state = battle_core::MatchState::begin_with(setup.clone(), rules);
        let mut acts: i16 = 0;
        for action in actions {
            let mine = state.active == battle_core::Side::Player;
            let Ok((next, _)) = battle_core::reduce(&state, action) else {
                break;
            };
            if mine && !matches!(action, battle_core::Action::EndTurn) {
                acts = acts.saturating_add(1);
            }
            state = next;
        }
        let lost = state
            .units
            .iter()
            .filter(|u| u.owner == battle_core::Side::Player && u.health.is_dead())
            .count()
            .min(i16::MAX as usize) as i16;
        (acts, lost)
    }

    fn match_state(rec: &crate::models::BattleMatch) -> Result<battle_core::MatchState> {
        // The cache first, the journal when it is missing or unreadable. Either
        // way the answer is the same board; that is what makes it a cache.
        if let Some(cached) = rec.board_cache.as_deref() {
            if let Ok(state) = serde_json::from_str(cached) {
                return Ok(state);
            }
        }
        let setup: battle_core::Setup = serde_json::from_str(&rec.setup)
            .map_err(|_| AppError::Internal("Match arrangement will not read".into()))?;
        let actions: Vec<battle_core::Action> = serde_json::from_str(&rec.actions)
            .map_err(|_| AppError::Internal("Match journal will not read".into()))?;
        Self::replay(&setup, &actions, Self::match_rules(rec))
    }

    fn match_dto(
        rec: &crate::models::BattleMatch,
        state: battle_core::MatchState,
        events: Vec<battle_core::Event>,
        reward_dust: i32,
    ) -> BattleMatchDto {
        BattleMatchDto {
            id: rec.id.to_string(),
            challenge_id: rec.challenge_id.map(|id| id.to_string()),
            seq: rec.seq,
            legal_actions: battle_core::legal_actions(&state),
            state,
            events,
            outcome: rec.outcome.clone(),
            reward_dust,
            marks: None,
        }
    }

    /// Begin — or, for a guest who left one going, continue.
    pub async fn begin_battle_match(
        &self,
        user_id: Uuid,
        challenge_id: Uuid,
    ) -> Result<BattleMatchDto> {
        if let Some(open) = self
            .repo
            .find_open_battle_match(user_id, challenge_id)
            .await?
        {
            let state = Self::match_state(&open)?;
            return Ok(Self::match_dto(&open, state, Vec::new(), 0));
        }

        let challenge = self.repo.get_battle_challenge(challenge_id).await?;
        if challenge.status != "published" {
            return Err(AppError::NotFound("No such challenge".into()));
        }
        // Гостю — только бои с заданной расстановкой: встреча просит свою
        // колоду, а колода — имя с собранием карт.
        if challenge.player_side == crate::battles::SIDE_DECK && self.repo.user_is_guest(user_id).await? {
            return Err(AppError::Unauthorized);
        }
        let arrangement: ChallengeSetup = serde_json::from_str(&challenge.setup)
            .map_err(|_| AppError::Internal("Challenge arrangement will not read".into()))?;

        let cards = self.cards_by_slug().await?;
        let mut setup = Self::raise(&arrangement, &cards)?;
        // Встреча: половину гостя приносит его стол, половина хранителя остаётся
        // той, что расставлена рукой. Всё, что ниже, не знает, какого рода было
        // испытание, — и это единственная причина, по которой ветка помещается
        // в один вызов: снимок, `MatchState::begin`, запись, награда и пересмотр
        // общие для обоих родов.
        if challenge.player_side == crate::battles::SIDE_DECK {
            // Клетки уже клетки поля этой встречи: гость расставил карты на
            // нём самом (`read_battle_lineup`), подгонять нечего.
            let (board, hand) = self
                .lineup_takes_the_field(user_id, challenge_id, &arrangement)
                .await?;
            setup.player_board = board;
            setup.player_hand = hand;
        }
        // The snapshot is taken here and never again: from this moment editing a
        // card cannot reach into this match.
        let frozen = serde_json::to_string(&setup)
            .map_err(|_| AppError::Internal("Arrangement will not freeze".into()))?;
        // Правила замораживаются вместе с телами и по той же причине: этюд,
        // переписанный посреди начатой партии, менял бы её на ходу.
        let rules = crate::battles::rules_of(&arrangement);
        let written_rules = serde_json::to_string(&rules)
            .map_err(|_| AppError::Internal("Rules will not freeze".into()))?;
        let state = battle_core::MatchState::begin_with(setup, rules);
        let cache = serde_json::to_string(&state)
            .map_err(|_| AppError::Internal("Board will not serialise".into()))?;

        // The highest rules version among the cards that took the field, so a
        // replay knows which numbers it was played under.
        let version = cards.values().map(|c| c.rules_version).max().unwrap_or(1);

        let rec = self
            .repo
            .insert_battle_match(
                user_id,
                challenge_id,
                &frozen,
                version,
                &cache,
                &written_rules,
            )
            .await?;
        Self::log_domain_event("battle_match_started", "battle_match", rec.id, "ok");
        Ok(Self::match_dto(&rec, state, Vec::new(), 0))
    }

    /// Give the field: the open match ends as a keeper win, without a move.
    /// Dust is not granted — they did not take the field, they left it.
    pub async fn yield_battle_match(&self, user_id: Uuid, id: Uuid) -> Result<BattleMatchDto> {
        let rec = self.repo.get_battle_match(id, user_id).await?;
        if rec.outcome.is_some() {
            let state = Self::match_state(&rec)?;
            return Ok(Self::match_dto(&rec, state, Vec::new(), 0));
        }
        let mut state = Self::match_state(&rec)?;
        state.outcome = Some(battle_core::Outcome::Keeper);
        let cache = serde_json::to_string(&state)
            .map_err(|_| AppError::Internal("Board will not serialise".into()))?;
        let Some(saved) = self.repo.yield_battle_match(id, user_id, &cache).await? else {
            let rec = self.repo.get_battle_match(id, user_id).await?;
            let state = Self::match_state(&rec)?;
            return Ok(Self::match_dto(&rec, state, Vec::new(), 0));
        };
        Self::log_domain_event("battle_match_yielded", "battle_match", id, "ok");
        Ok(Self::match_dto(&saved, state, Vec::new(), 0))
    }

    /// Start this challenge from the first position, dropping the open match
    /// if there is one. A replay after a finished match is the same call:
    /// there is nothing to drop, and `begin` inserts a new row.
    pub async fn restart_battle_match(
        &self,
        user_id: Uuid,
        challenge_id: Uuid,
    ) -> Result<BattleMatchDto> {
        self.repo
            .drop_open_battle_match(user_id, challenge_id)
            .await?;
        self.begin_battle_match(user_id, challenge_id).await
    }

    pub async fn read_battle_match(&self, user_id: Uuid, id: Uuid) -> Result<BattleMatchDto> {
        let rec = self.repo.get_battle_match(id, user_id).await?;
        let state = Self::match_state(&rec)?;
        Ok(Self::match_dto(&rec, state, Vec::new(), 0))
    }

    /// One move by the guest, and the keeper's answer, in a single round trip.
    pub async fn act_in_battle_match(
        &self,
        user_id: Uuid,
        id: Uuid,
        req: BattleActRequest,
    ) -> Result<BattleMatchDto> {
        let rec = self.repo.get_battle_match(id, user_id).await?;
        if rec.outcome.is_some() {
            let state = Self::match_state(&rec)?;
            return Ok(Self::match_dto(&rec, state, Vec::new(), 0));
        }

        // The number is checked BEFORE the move is judged, and this order is the
        // whole of it. A double-clicked "play this card" would otherwise be read
        // against a board where the card has already left the hand, and come
        // back as "no such card" — a rules error for what is simply a repeat.
        // A repeat must be answered with the same board, not with a complaint.
        if req.seq != rec.seq {
            let state = Self::match_state(&rec)?;
            return Ok(Self::match_dto(&rec, state, Vec::new(), 0));
        }

        let mut journal: Vec<battle_core::Action> = serde_json::from_str(&rec.actions)
            .map_err(|_| AppError::Internal("Match journal will not read".into()))?;
        let mut state = Self::match_state(&rec)?;

        if state.active != battle_core::Side::Player {
            return Err(AppError::BadRequest("Not your turn".into()));
        }

        let mut events = Vec::new();
        let (next, produced) = battle_core::reduce(&state, &req.action)
            .map_err(|e| AppError::BadRequest(format!("{e:?}")))?;
        state = next;
        events.extend(produced);
        journal.push(req.action);

        // The keeper answers in the same breath: one trip over the network for a
        // whole turn, and the scene plays both halves from one journal.
        // Рука хранителя — та, что задана испытанием. До этого поле `bot_depth`
        // писалось в базу, показывалось на полке этюдов и не читалось никем:
        // единственная ручка сложности всего PvE ничего не делала.
        let depth = self.keeper_hand_depth(rec.challenge_id).await?;
        let mut guard = 0;
        while state.outcome.is_none() && state.active == battle_core::Side::Keeper {
            let action = battle_core::bot::choose_at(&state, depth);
            let (next, produced) = battle_core::reduce(&state, &action)
                .map_err(|e| AppError::Internal(format!("Keeper played an illegal move: {e:?}")))?;
            state = next;
            events.extend(produced);
            journal.push(action);
            guard += 1;
            if guard > 500 {
                return Err(AppError::Internal("Keeper will not stop".into()));
            }
        }

        let outcome = state.outcome.map(|o| match o {
            battle_core::Outcome::Player => "player",
            battle_core::Outcome::Keeper => "keeper",
            battle_core::Outcome::Draw => "draw",
        });

        // Печать под партией ставится ровно здесь и один раз — в тот ход,
        // которым партия кончилась. Считается свёрткой журнала, потому что
        // журнал и есть истина; счётчик, который вели бы параллельно, однажды
        // разошёлся бы с ним молча.
        let line = if outcome.is_some() {
            let frozen: battle_core::Setup = serde_json::from_str(&rec.setup)
                .map_err(|_| AppError::Internal("Match arrangement will not read".into()))?;
            Some(Self::count_the_line(
                &frozen,
                &journal,
                Self::match_rules(&rec),
            ))
        } else {
            None
        };
        // Спрошено ДО записи: иначе эта же партия станет собственной планкой, и
        // всякая победа читалась бы как повторение рекорда.
        let stood = match (outcome, rec.challenge_id) {
            (Some("player"), Some(challenge_id)) => {
                self.repo.battle_challenge_best_line(challenge_id).await?
            }
            _ => None,
        };

        let written = serde_json::to_string(&journal)
            .map_err(|_| AppError::Internal("Journal will not serialise".into()))?;
        let cache = serde_json::to_string(&state)
            .map_err(|_| AppError::Internal("Board will not serialise".into()))?;

        // The guard against a double click: the row moves only from the number
        // the caller had. A repeat finds nothing and changes nothing.
        let Some(saved) = self
            .repo
            .advance_battle_match(
                id,
                req.seq,
                &written,
                &cache,
                outcome,
                state.round as i16,
                line.map(|(acts, _)| acts),
                line.map(|(_, lost)| lost),
            )
            .await?
        else {
            let rec = self.repo.get_battle_match(id, user_id).await?;
            let state = Self::match_state(&rec)?;
            return Ok(Self::match_dto(&rec, state, Vec::new(), 0));
        };

        // Две выплаты, два ключа, оба по одному на этюд.
        //
        // За доведённое до конца платят ВСЕГДА, победа или нет: до сих пор
        // новичок, проигравший хранителю первую партию, не получал ровно
        // ничего — человек сел за доску, дошёл до исхода, и комната промолчала.
        // Ферму это не открывает: ключ прежний, один на этюд, сколько бы раз
        // его ни переигрывали.
        //
        // Гостю пыль не платится вовсе: гостевых имён можно завести сколько
        // угодно, и «однажды на этюд» стало бы «однажды на каждое имя».
        let guest = self.repo.user_is_guest(user_id).await?;
        let mut reward = 0;
        if let (Some(challenge_id), Some(done), false) = (rec.challenge_id, outcome, guest) {
            let challenge = self.repo.get_battle_challenge(challenge_id).await?;

            if challenge.reward_finish_dust > 0
                && self
                    .repo
                    .credit_battle_wallet(
                        user_id,
                        "dust",
                        challenge.reward_finish_dust,
                        "battle_finish",
                        Some(challenge_id),
                        &Self::challenge_finish_key(challenge_id),
                    )
                    .await?
            {
                reward += challenge.reward_finish_dust;
                Self::log_domain_event("battle_finish_paid", "battle_match", id, "ok");
            }

            if done == "player"
                && challenge.reward_dust > 0
                && self
                    .repo
                    .credit_battle_wallet(
                        user_id,
                        "dust",
                        challenge.reward_dust,
                        "battle_challenge",
                        Some(challenge_id),
                        &Self::challenge_key(challenge_id),
                    )
                    .await?
            {
                reward += challenge.reward_dust;
                Self::log_domain_event("battle_reward_paid", "battle_match", id, "ok");
            }
        }

        // Партия дошла до исхода — «довести этюд до конца» закрывается здесь.
        // Победа или нет: сегодня проигравший первую партию не получает ровно
        // ничего, и это худший из возможных ответов на первую же попытку.
        if outcome.is_some() && !guest {
            self.settle_errands_quietly(user_id).await;
        }

        let mut dto = Self::match_dto(&saved, state, events, reward);
        dto.marks = line.map(|(acts, lost)| crate::models::MatchMarks {
            acts: acts as i32,
            bodies_lost: lost as i32,
            best_known: stood.map(|b| b as i32),
            // Рекорд — только у победы: проигранная линия из трёх дел короче
            // всякой выигранной, и мерить её тем же числом значило бы платить
            // за то, чтобы сдаться быстрее.
            record: outcome == Some("player") && stood.is_none_or(|b| acts < b),
        });
        Ok(dto)
    }

    /// Рука хранителя в этой партии. Испытание её и назначает.
    async fn keeper_hand_depth(&self, challenge_id: Option<Uuid>) -> Result<u8> {
        Ok(match challenge_id {
            Some(id) => crate::battles::clamp_bot_depth(
                self.repo.get_battle_challenge(id).await?.bot_depth,
            ) as u8,
            None => crate::battles::BOT_DEPTH_MIN as u8,
        })
    }

    /// «Если сделать это и на этом закончить ход — чем ответит хранитель».
    ///
    /// Не подсказка и не поддавки: ответ хранителя ВЫЧИСЛИМ. Случайности в
    /// движке нет, скрытых карт у него нет, `reduce` чиста — то есть человек с
    /// карандашом получил бы то же самое, только за полчаса. Прятать вычислимое
    /// значит продавать не глубину, а неудобство.
    ///
    /// Ход закрывается прямо здесь, и вопрос поставлен именно так — «если на
    /// этом закончить». Иначе ответа не существует: хранитель отвечает не на
    /// удар, а на конец хода, и предвестие, показанное после удара, показывало
    /// бы ответ на ход, который ещё не сделан.
    ///
    /// Ничего не пишет. Поэтому здесь нет и защиты от повтора: повторять нечего,
    /// а `seq` спрашивается ровно затем, чтобы не отвечать про доску, которой
    /// уже нет.
    pub async fn foresee_battle_match(
        &self,
        user_id: Uuid,
        id: Uuid,
        req: crate::models::ForeseeRequest,
    ) -> Result<crate::models::ForeseeDto> {
        let rec = self.repo.get_battle_match(id, user_id).await?;
        let state = Self::match_state(&rec)?;
        // Пустое предвестие честнее сочинённого: доска ушла вперёд, партия
        // кончилась или ход не наш — сказать нечего.
        if rec.outcome.is_some() || req.seq != rec.seq || state.active != battle_core::Side::Player
        {
            return Ok(crate::models::ForeseeDto {
                seq: rec.seq,
                yours: Vec::new(),
                theirs: Vec::new(),
                outcome: rec.outcome.clone(),
            });
        }

        let (mut state, yours) = battle_core::reduce(&state, &req.action)
            .map_err(|e| AppError::BadRequest(format!("{e:?}")))?;

        let mut theirs = Vec::new();
        if state.outcome.is_none() {
            let depth = self.keeper_hand_depth(rec.challenge_id).await?;
            if let Ok((next, produced)) =
                battle_core::reduce(&state, &battle_core::Action::EndTurn)
            {
                state = next;
                theirs.extend(produced);
            }
            let mut guard = 0;
            while state.outcome.is_none() && state.active == battle_core::Side::Keeper {
                let action = battle_core::bot::choose_at(&state, depth);
                let Ok((next, produced)) = battle_core::reduce(&state, &action) else {
                    break;
                };
                state = next;
                theirs.extend(produced);
                guard += 1;
                if guard > 500 {
                    break;
                }
            }
        }

        Ok(crate::models::ForeseeDto {
            seq: rec.seq,
            yours,
            theirs,
            outcome: state.outcome.map(|o| match o {
                battle_core::Outcome::Player => "player".to_string(),
                battle_core::Outcome::Keeper => "keeper".to_string(),
                battle_core::Outcome::Draw => "draw".to_string(),
            }),
        })
    }

    /// The bench: an arrangement played by hand, without a trace.
    ///
    /// The keeper's own testing table. Deliberately stateless — see `BenchRequest`.
    pub async fn bench_battle(&self, req: BenchRequest) -> Result<BenchDto> {
        let cards = self.cards_by_slug().await?;
        let setup = Self::raise(&req.setup, &cards)?;

        // Правила берутся из расстановки, как их возьмёт партия гостя. Стол
        // затем и нужен, чтобы этюд проверялся тем же, чем он будет сыгран;
        // стол, играющий домашними правилами при своих в испытании, проверял бы
        // не тот этюд.
        let rules = crate::battles::rules_of(&req.setup);

        // Everything already played is folded back in silently: its events were
        // shown when they happened and showing them again would be a lie.
        let mut state = Self::replay(&setup, &req.actions, rules)?;
        let mut journal = req.actions;
        let mut events = Vec::new();

        if let Some(action) = req.next {
            let (next, produced) = battle_core::reduce(&state, &action)
                .map_err(|e| AppError::BadRequest(format!("{e:?}")))?;
            state = next;
            events.extend(produced);
            journal.push(action);
        }

        // The far side answers, or both sides play themselves to the end.
        let mut guard = 0;
        while state.outcome.is_none()
            && (req.play_out || (req.auto_keeper && state.active == battle_core::Side::Keeper))
        {
            let action = battle_core::bot::choose_at(
                &state,
                crate::battles::clamp_bot_depth(req.bot_depth) as u8,
            );
            let (next, produced) = battle_core::reduce(&state, &action)
                .map_err(|e| AppError::Internal(format!("Bot played an illegal move: {e:?}")))?;
            state = next;
            events.extend(produced);
            journal.push(action);
            guard += 1;
            if guard > 2000 {
                return Err(AppError::Internal("The bench will not settle".into()));
            }
        }

        Ok(BenchDto {
            legal_actions: battle_core::legal_actions(&state),
            outcome: state.outcome.map(|o| match o {
                battle_core::Outcome::Player => "player".to_string(),
                battle_core::Outcome::Keeper => "keeper".to_string(),
                battle_core::Outcome::Draw => "draw".to_string(),
            }),
            state,
            events,
            actions: journal,
        })
    }

    pub async fn battle_dust_balance(&self, user_id: Uuid) -> Result<i64> {
        self.repo.battle_wallet_balance(user_id, "dust").await
    }

    // === KEYWORDS ===
    //
    // The dictionary where the exchange rates live. Editing a `point_value` here
    // rebalances every card that names the keyword — which is the whole reason
    // the rate is a row and not a constant in the code.

    fn battle_keyword_dto(row: crate::models::BattleKeyword) -> BattleKeywordDto {
        BattleKeywordDto {
            id: row.id.to_string(),
            slug: row.slug,
            name_en: row.name_en,
            name_ru: row.name_ru,
            name_uk: row.name_uk,
            name_es: row.name_es,
            name_de: row.name_de,
            name_it: row.name_it,
            rules_en: row.rules_en,
            rules_ru: row.rules_ru,
            rules_uk: row.rules_uk,
            rules_es: row.rules_es,
            rules_de: row.rules_de,
            rules_it: row.rules_it,
            icon_url: row.icon_url,
            point_value: row.point_value,
            sort_order: row.sort_order,
        }
    }

    pub async fn list_battle_keywords(&self) -> Result<Vec<BattleKeywordDto>> {
        let rows = self.repo.list_battle_keywords().await?;
        Ok(rows.into_iter().map(Self::battle_keyword_dto).collect())
    }

    pub async fn admin_create_battle_keyword(
        &self,
        req: SaveBattleKeywordRequest,
    ) -> Result<BattleKeywordDto> {
        let taken = self.repo.list_battle_keyword_slugs_except(None).await?;
        let p = Self::prepare_keyword(&req, &taken)?;
        let rec = self
            .repo
            .insert_battle_keyword(
                &p.slug,
                &p.name_en,
                &p.name_ru,
                p.rules_en.as_deref(),
                p.rules_ru.as_deref(),
                p.icon_url.as_deref(),
                p.point_value,
                &p.name_uk,
                &p.name_es,
                &p.name_de,
                &p.name_it,
                p.rules_uk.as_deref(),
                p.rules_es.as_deref(),
                p.rules_de.as_deref(),
                p.rules_it.as_deref(),
            )
            .await?;
        Self::log_domain_event("battle_keyword_created", "battle_keyword", rec.id, "ok");
        Ok(Self::battle_keyword_dto(rec))
    }

    pub async fn admin_update_battle_keyword(
        &self,
        id: Uuid,
        req: SaveBattleKeywordRequest,
    ) -> Result<BattleKeywordDto> {
        let taken = self.repo.list_battle_keyword_slugs_except(Some(id)).await?;
        let p = Self::prepare_keyword(&req, &taken)?;
        let rec = self
            .repo
            .update_battle_keyword(
                id,
                &p.slug,
                &p.name_en,
                &p.name_ru,
                p.rules_en.as_deref(),
                p.rules_ru.as_deref(),
                p.icon_url.as_deref(),
                p.point_value,
                &p.name_uk,
                &p.name_es,
                &p.name_de,
                &p.name_it,
                p.rules_uk.as_deref(),
                p.rules_es.as_deref(),
                p.rules_de.as_deref(),
                p.rules_it.as_deref(),
            )
            .await?;
        Self::log_domain_event("battle_keyword_updated", "battle_keyword", rec.id, "ok");
        Ok(Self::battle_keyword_dto(rec))
    }

    /// Removing a keyword does not touch the cards that named it: their ability
    /// still carries the slug, it simply no longer resolves to a wording.
    pub async fn admin_delete_battle_keyword(&self, id: Uuid) -> Result<()> {
        self.repo.delete_battle_keyword(id).await?;
        Self::log_domain_event("battle_keyword_deleted", "battle_keyword", id, "ok");
        Ok(())
    }

    pub async fn admin_reorder_battle_keywords(&self, ids: Vec<Uuid>) -> Result<()> {
        let mut seen = std::collections::HashSet::with_capacity(ids.len());
        if !ids.iter().all(|id| seen.insert(*id)) {
            return Err(AppError::BadRequest(
                "A keyword cannot stand in two places".into(),
            ));
        }
        self.repo.set_battle_keyword_order(&ids).await?;
        Ok(())
    }

    fn prepare_keyword(
        req: &SaveBattleKeywordRequest,
        taken: &[String],
    ) -> Result<PreparedKeyword> {
        let cut = |raw: &str, max: usize| -> String { raw.trim().chars().take(max).collect() };
        let mut name_en = cut(&req.name_en, crate::battles::KEYWORD_NAME_MAX);
        let mut name_ru = cut(&req.name_ru, crate::battles::KEYWORD_NAME_MAX);
        if name_en.is_empty() && name_ru.is_empty() {
            return Err(AppError::BadRequest("A keyword needs a name".into()));
        }
        if name_en.is_empty() {
            name_en = name_ru.clone();
        }
        if name_ru.is_empty() {
            name_ru = name_en.clone();
        }
        let rules = |raw: Option<&str>| -> Option<String> {
            raw.map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.chars().take(crate::battles::KEYWORD_RULES_MAX).collect())
        };
        // A negative rate is nonsense in every direction, and a runaway one
        // would quietly overload every card naming the keyword.
        let point_value = req
            .point_value
            .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 100.0);
        Ok(PreparedKeyword {
            slug: crate::battles::unique_slug(req.slug.as_deref(), &name_en, taken),
            name_en,
            name_ru,
            rules_en: rules(req.rules_en.as_deref()),
            rules_ru: rules(req.rules_ru.as_deref()),
            name_uk: cut(&req.name_uk, crate::battles::KEYWORD_NAME_MAX),
            name_es: cut(&req.name_es, crate::battles::KEYWORD_NAME_MAX),
            name_de: cut(&req.name_de, crate::battles::KEYWORD_NAME_MAX),
            name_it: cut(&req.name_it, crate::battles::KEYWORD_NAME_MAX),
            rules_uk: rules(req.rules_uk.as_deref()),
            rules_es: rules(req.rules_es.as_deref()),
            rules_de: rules(req.rules_de.as_deref()),
            rules_it: rules(req.rules_it.as_deref()),
            icon_url: req
                .icon_url
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            point_value,
        })
    }

    // === RACES ===

    fn battle_race_dto(row: BattleRaceListed) -> BattleRaceDto {
        BattleRaceDto {
            id: row.id.to_string(),
            slug: row.slug,
            name_en: row.name_en,
            name_ru: row.name_ru,
            name_uk: row.name_uk,
            name_es: row.name_es,
            name_de: row.name_de,
            name_it: row.name_it,
            note_en: row.note_en,
            note_ru: row.note_ru,
            note_uk: row.note_uk,
            note_es: row.note_es,
            note_de: row.note_de,
            note_it: row.note_it,
            icon_url: row.icon_url,
            level_frames: row.level_frames,
            motion_wear: row.motion_wear,
            sort_order: row.sort_order,
            card_count: row.card_count,
            race_damage: crate::battles::parse_race_damage(row.race_damage.as_deref()),
            race_damage_step: crate::battles::RACE_DAMAGE_MAX,
        }
    }

    pub async fn list_battle_races(&self) -> Result<Vec<BattleRaceDto>> {
        let rows = self.repo.list_battle_races().await?;
        Ok(rows.into_iter().map(Self::battle_race_dto).collect())
    }

    pub async fn admin_create_battle_race(
        &self,
        req: SaveBattleRaceRequest,
    ) -> Result<BattleRaceDto> {
        let taken = self.repo.list_battle_race_slugs_except(None).await?;
        let p = Self::prepare_race(&req, &taken)?;
        let damage = self.race_damage_text(None, req.race_damage.as_ref()).await?;
        let rec = self
            .repo
            .insert_battle_race(
                &p.slug,
                &p.name_en,
                &p.name_ru,
                p.note_en.as_deref(),
                p.note_ru.as_deref(),
                p.icon_url.as_deref(),
                p.level_frames.as_deref(),
                p.motion_wear.as_deref(),
                &p.name_uk,
                &p.name_es,
                &p.name_de,
                &p.name_it,
                p.note_uk.as_deref(),
                p.note_es.as_deref(),
                p.note_de.as_deref(),
                p.note_it.as_deref(),
                damage.flatten().as_deref(),
            )
            .await?;
        Self::log_domain_event("battle_race_created", "battle_race", rec.id, "ok");
        self.find_race(rec.id).await
    }

    pub async fn admin_update_battle_race(
        &self,
        id: Uuid,
        req: SaveBattleRaceRequest,
    ) -> Result<BattleRaceDto> {
        let taken = self.repo.list_battle_race_slugs_except(Some(id)).await?;
        let p = Self::prepare_race(&req, &taken)?;
        // Таблица урона проверяется ДО записи и пишется той же командой, что
        // и остальная раса: две записи подряд могли оставить имя сохранённым,
        // а таблицу прежней, и ответить ошибкой на наполовину сделанное.
        let damage = self.race_damage_text(Some(id), req.race_damage.as_ref()).await?;
        let rec = self
            .repo
            .update_battle_race(
                id,
                &p.slug,
                &p.name_en,
                &p.name_ru,
                p.note_en.as_deref(),
                p.note_ru.as_deref(),
                p.icon_url.as_deref(),
                p.level_frames.as_deref(),
                p.motion_wear.as_deref(),
                &p.name_uk,
                &p.name_es,
                &p.name_de,
                &p.name_it,
                p.note_uk.as_deref(),
                p.note_es.as_deref(),
                p.note_de.as_deref(),
                p.note_it.as_deref(),
                damage.as_ref().map(|d| d.as_deref()),
            )
            .await?;
        Self::log_domain_event("battle_race_updated", "battle_race", rec.id, "ok");
        self.find_race(rec.id).await
    }

    /// Таблица урона в том виде, в каком её пишут в колонку.
    ///
    /// Внешний `None` — таблицу не прислали, и колонка не трогается
    /// (утверждение из студии пишет расу без неё). Внутренний `None` — таблица
    /// пуста. Числа приводятся к пределу, ссылки — только на расы, которые
    /// есть, и не на саму себя (`own`).
    async fn race_damage_text(
        &self,
        own: Option<Uuid>,
        damage: Option<&battle_core::RaceDamage>,
    ) -> Result<Option<Option<String>>> {
        let Some(damage) = damage else { return Ok(None) };
        let own = own.map(|id| id.to_string());
        let known: Vec<String> = self
            .repo
            .list_battle_races()
            .await?
            .into_iter()
            .map(|r| r.id.to_string())
            .filter(|id| Some(id) != own.as_ref())
            .collect();
        let clean = crate::battles::normalize_race_damage(damage, &known);
        Ok(Some(Self::race_damage_json(&clean)?))
    }

    fn race_damage_json(clean: &battle_core::RaceDamage) -> Result<Option<String>> {
        if clean.is_empty() {
            return Ok(None);
        }
        serde_json::to_string(clean)
            .map(Some)
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    /// Removing a race does not remove the cards that wore it — they simply
    /// stand without one. Said here as well as in the schema, because that is
    /// the thing a keeper is right to fear before clicking.
    ///
    /// Из таблиц урона остальных рас она вычёркивается той же транзакцией:
    /// иначе её id лежал бы там, пока те расы не пересохранят.
    pub async fn admin_delete_battle_race(&self, id: Uuid) -> Result<()> {
        let gone = id.to_string();
        let rows = self.repo.list_battle_races().await?;
        let known: Vec<String> = rows
            .iter()
            .map(|r| r.id.to_string())
            .filter(|r| *r != gone)
            .collect();
        let mut unlink = Vec::new();
        for row in rows.iter().filter(|r| r.id != id) {
            let was = crate::battles::parse_race_damage(row.race_damage.as_deref());
            if !was.against.contains(&gone) && !was.from.contains(&gone) {
                continue;
            }
            let own: Vec<String> = known.iter().filter(|k| **k != row.id.to_string()).cloned().collect();
            let clean = crate::battles::normalize_race_damage(&was, &own);
            unlink.push((row.id, Self::race_damage_json(&clean)?));
        }
        self.repo.delete_battle_race(id, &unlink).await?;
        Self::log_domain_event("battle_race_deleted", "battle_race", id, "ok");
        Ok(())
    }

    // === BATTLE ASSETS ===

    /// Matches the CHECK on both `name` columns.
    const ASSET_NAME_MAX: usize = 80;
    /// The box a proposal's preview is shrunk into. Big enough to tell a
    /// corner from an ornament, small enough that eighty of them fit in one
    /// response without it becoming a download.
    const PART_PREVIEW_PX: u32 = 220;
    /// Mirrors the CHECK on `battle_assets.role`. The five slots a `sliced`
    /// frame has, plus a picture and a catch-all.
    /// How many rectangles one drawing may carry. Generous — a busy sheet of
    /// charms can hold a dozen — but bounded, so a broken client cannot ask
    /// for a thousand crops of one picture.
    const SPLIT_RECTS_MAX: usize = 32;
    /// What counts as artwork when a hand-drawn rectangle is trimmed. The
    /// piece has already had its ground taken off, so anything faintly there
    /// is real.
    const SPLIT_ALPHA_FLOOR: u8 = 8;
    /// Седьмое слово — `motion`: деталь, из которой собрано движение (стрела,
    /// вспышка, полоса кадров). Роль — это слово, которым хранитель ИЩЕТ, а не
    /// то, что смотрит игра; без него стрела терялась бы среди накладок рам.
    const ASSET_ROLES: [&'static str; 7] =
        ["corner", "sideH", "sideV", "accent", "art", "motion", "other"];

    fn asset_sheet_dto(row: BattleAssetSheetListed) -> BattleAssetSheetDto {
        BattleAssetSheetDto {
            id: row.id.to_string(),
            name: row.name,
            source_url: row.source_url,
            width: row.width,
            height: row.height,
            settings: row.settings,
            sort_order: row.sort_order,
            created_at: row.created_at,
            part_count: row.part_count,
        }
    }

    fn asset_dto(row: BattleAssetListed) -> BattleAssetDto {
        BattleAssetDto {
            id: row.id.to_string(),
            sheet_id: row.sheet_id.map(|id| id.to_string()),
            sheet_name: row.sheet_name,
            name: row.name,
            role: row.role,
            url: row.url,
            width: row.width,
            height: row.height,
            sort_order: row.sort_order,
            public: row.public,
            created_at: row.created_at,
        }
    }

    fn clean_asset_name(raw: &str, fallback: &str) -> String {
        let trimmed: String = raw.trim().chars().take(Self::ASSET_NAME_MAX).collect();
        if trimmed.is_empty() {
            fallback.chars().take(Self::ASSET_NAME_MAX).collect()
        } else {
            trimmed
        }
    }

    /// An unknown role is refused rather than quietly turned into `other`:
    /// filing a part under the wrong slot is worse than being told no.
    fn clean_asset_role(raw: Option<&str>, fallback: &str) -> Result<String> {
        let value = raw.map(|r| r.trim()).filter(|r| !r.is_empty());
        match value {
            None => Ok(fallback.to_string()),
            Some(r) if Self::ASSET_ROLES.contains(&r) => Ok(r.to_string()),
            Some(r) => Err(AppError::BadRequest(format!("Unknown asset role: {r}"))),
        }
    }

    pub async fn list_battle_asset_sheets(&self) -> Result<Vec<BattleAssetSheetDto>> {
        let rows = self.repo.list_battle_asset_sheets().await?;
        Ok(rows.into_iter().map(Self::asset_sheet_dto).collect())
    }

    pub async fn admin_add_battle_asset_sheet(
        &self,
        name: &str,
        bytes: Vec<u8>,
    ) -> Result<BattleAssetSheetDto> {
        // The format is read from the bytes, never from the filename: an
        // extension is a claim, and this one decides where the file lands.
        let format = image::guess_format(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Unrecognised image: {e}")))?;
        let extension = format.extensions_str().first().copied().unwrap_or("bin");
        let probe = image::load_from_memory(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid image file: {e}")))?;
        let (width, height) = (probe.width(), probe.height());
        if width as u64 * height as u64 > crate::sheet::SHEET_MAX_PIXELS {
            return Err(AppError::BadRequest(format!(
                "Sheet is too large: {} pixels, limit is {}",
                width as u64 * height as u64,
                crate::sheet::SHEET_MAX_PIXELS
            )));
        }
        drop(probe);

        let relative = format!("sheets/{}.{}", Uuid::new_v4(), extension);
        self.write_upload(&relative, &bytes).await?;
        let rec = self
            .repo
            .insert_battle_asset_sheet(
                &Self::clean_asset_name(name, "Лист"),
                &format!("/static/{}", relative),
                width as i32,
                height as i32,
            )
            .await?;
        Self::log_domain_event("battle_asset_sheet_added", "battle_asset_sheet", rec.id, "ok");
        self.find_asset_sheet(rec.id).await
    }

    pub async fn admin_rename_battle_asset_sheet(
        &self,
        id: Uuid,
        name: &str,
    ) -> Result<BattleAssetSheetDto> {
        let rec = self
            .repo
            .rename_battle_asset_sheet(id, &Self::clean_asset_name(name, "Лист"))
            .await?;
        self.find_asset_sheet(rec.id).await
    }

    /// Clearing a sheet away leaves its parts standing, loose. Said here as
    /// well as in the schema, because that is the thing a keeper is right to
    /// fear before clicking — a frame may already be wearing one of them.
    pub async fn admin_delete_battle_asset_sheet(&self, id: Uuid) -> Result<()> {
        let sheet = self.repo.get_battle_asset_sheet(id).await?;
        self.repo.delete_battle_asset_sheet(id).await?;
        self.remove_upload(&sheet.source_url).await;
        Self::log_domain_event("battle_asset_sheet_deleted", "battle_asset_sheet", id, "ok");
        Ok(())
    }

    /// Cut the sheet and show what would come off it — without writing a
    /// single file. The keeper re-cuts until the numbers look right, and only
    /// then commits.
    pub async fn admin_slice_battle_asset_sheet(
        &self,
        id: Uuid,
        settings: crate::sheet::SliceSettings,
    ) -> Result<BattleSheetCutDto> {
        let cut = self.cut_sheet(id, settings.clone()).await?;
        Ok(Self::sliced_dto(cut, settings))
    }

    /// Предложение разреза, собранное для показа. Одно на оба стола: у склада
    /// дома и у склада студии режет один и тот же `sheet.rs`, и второй сборщик
    /// превью однажды показал бы не то, что вырежется.
    fn sliced_dto(
        cut: crate::sheet::Sliced,
        settings: crate::sheet::SliceSettings,
    ) -> BattleSheetCutDto {
        let parts = cut
            .parts
            .iter()
            .map(|part| {
                let thumb = crate::sheet::thumbnail(&part.image, Self::PART_PREVIEW_PX);
                let webp = crate::sheet::to_webp(&thumb, 80.0);
                BattleSheetPartDto {
                    index: part.index,
                    x: part.x,
                    y: part.y,
                    width: part.width,
                    height: part.height,
                    is_text: part.is_text,
                    role: Self::role_name(part.role).to_string(),
                    preview: format!(
                        "data:image/webp;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(&webp)
                    ),
                }
            })
            .collect();
        BattleSheetCutDto {
            width: cut.width,
            height: cut.height,
            source: match cut.source {
                crate::sheet::MaskSource::Alpha => "alpha".into(),
                crate::sheet::MaskSource::Background => "background".into(),
            },
            settings,
            parts,
        }
    }

    /// Take the chosen parts off the sheet for keeps.
    ///
    /// The cut is run again rather than cached from the proposal: a cached
    /// proposal would be a second source of truth about what the keeper saw,
    /// and it would have to be invalidated by hand. Instead the settings come
    /// back with the picks, and every pick carries the shape it was shown at —
    /// so a mismatch is caught rather than silently saving a different part
    /// under the name of the one the keeper chose.
    pub async fn admin_cut_battle_asset_sheet(
        &self,
        id: Uuid,
        settings: crate::sheet::SliceSettings,
        picks: Vec<BattleAssetPick>,
    ) -> Result<Vec<BattleAssetDto>> {
        if picks.is_empty() {
            return Err(AppError::BadRequest("Nothing was chosen".into()));
        }
        let remembered = serde_json::to_string(&settings)
            .map_err(|e| AppError::Internal(format!("Settings do not serialise: {e}")))?;
        let cut = self.cut_sheet(id, settings).await?;
        let mut order = self.repo.next_battle_asset_order(Some(id)).await?;
        let mut saved = Vec::with_capacity(picks.len());

        for pick in picks.iter() {
            let part = cut
                .parts
                .iter()
                .find(|p| p.index == pick.index)
                .ok_or_else(|| {
                    AppError::BadRequest(format!("The sheet has no part {}", pick.index))
                })?;
            if part.width != pick.width || part.height != pick.height {
                return Err(AppError::BadRequest(format!(
                    "Part {} is now {}x{}, not {}x{} — the sheet was cut with other settings; \
                     look at it again",
                    pick.index, part.width, part.height, pick.width, pick.height
                )));
            }
            let name = Self::clean_asset_name(
                pick.name.as_deref().unwrap_or(""),
                &format!("{:02}", pick.index),
            );
            let role = Self::clean_asset_role(
                pick.role.as_deref(),
                Self::role_name(part.role),
            )?;
            let rec = self
                .save_asset_image(Some(id), &name, &role, &part.image, order)
                .await?;
            order += 1;
            saved.push(rec.id);

            // ...and whatever the keeper drew on it. The part itself was just
            // saved and stays saved: the rectangles are additions, not a
            // replacement.
            if pick.rects.len() > Self::SPLIT_RECTS_MAX {
                return Err(AppError::BadRequest(format!(
                    "At most {} rectangles on one part",
                    Self::SPLIT_RECTS_MAX
                )));
            }
            for (n, rect) in pick.rects.iter().enumerate() {
                let cropped = crate::sheet::crop_to_content(
                    &part.image,
                    &rect.frame(),
                    Self::SPLIT_ALPHA_FLOOR,
                )
                .ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "Rectangle {} on part {} caught nothing but air",
                        n + 1,
                        pick.index
                    ))
                })?;
                let piece_name = Self::clean_asset_name(
                    rect.name.as_deref().unwrap_or(""),
                    &format!("{} · {}", name, n + 1),
                );
                let piece_role = Self::clean_asset_role(rect.role.as_deref(), &role)?;
                let piece = self
                    .save_asset_image(Some(id), &piece_name, &piece_role, &cropped, order)
                    .await?;
                order += 1;
                saved.push(piece.id);
            }
        }

        self.repo
            .save_battle_asset_sheet_settings(id, &remembered)
            .await?;
        Self::log_domain_event("battle_assets_cut", "battle_asset_sheet", id, "ok");

        let all = self
            .repo
            .list_battle_assets(AssetSheetFilter::One(id), None, None)
            .await?;
        Ok(all
            .into_iter()
            .filter(|row| saved.contains(&row.id))
            .map(Self::asset_dto)
            .collect())
    }

    pub async fn list_battle_assets(
        &self,
        sheet: AssetSheetFilter,
        role: Option<String>,
        query: Option<String>,
    ) -> Result<Vec<BattleAssetDto>> {
        let role = role.map(|r| r.trim().to_string()).filter(|r| !r.is_empty());
        if let Some(r) = role.as_deref()
            && !Self::ASSET_ROLES.contains(&r)
        {
            return Err(AppError::BadRequest(format!("Unknown asset role: {r}")));
        }
        let query = query.map(|q| q.trim().to_string()).filter(|q| !q.is_empty());
        let rows = self
            .repo
            .list_battle_assets(sheet, role.as_deref(), query.as_deref())
            .await?;
        Ok(rows.into_iter().map(Self::asset_dto).collect())
    }

    /// A part that arrived on its own, with no sheet behind it. Cut elsewhere,
    /// or drawn as one piece — either way an ordinary member of the store.
    pub async fn admin_add_battle_asset(
        &self,
        name: &str,
        role: Option<&str>,
        bytes: Vec<u8>,
    ) -> Result<BattleAssetDto> {
        let decoded = image::load_from_memory(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid image file: {e}")))?;
        if decoded.width() as u64 * decoded.height() as u64 > crate::sheet::SHEET_MAX_PIXELS {
            return Err(AppError::BadRequest("Image dimensions are too large".into()));
        }
        let rgba = decoded.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());
        let webp = crate::sheet::to_webp(&rgba, 88.0);
        let relative = format!("assets/{}.webp", Uuid::new_v4());
        self.write_upload(&relative, &webp).await?;

        let order = self.repo.next_battle_asset_order(None).await?;
        let rec = self
            .repo
            .insert_battle_asset(
                None,
                &Self::clean_asset_name(name, "Деталь"),
                &Self::clean_asset_role(role, "other")?,
                &format!("/static/{}", relative),
                width as i32,
                height as i32,
                order,
            )
            .await?;
        Self::log_domain_event("battle_asset_added", "battle_asset", rec.id, "ok");
        self.find_asset(rec.id).await
    }

    pub async fn admin_update_battle_asset(
        &self,
        id: Uuid,
        req: SaveBattleAssetRequest,
    ) -> Result<BattleAssetDto> {
        let name = req
            .name
            .as_deref()
            .map(|n| Self::clean_asset_name(n, ""))
            .filter(|n| !n.is_empty());
        let role = match req.role.as_deref() {
            None => None,
            Some(r) => Some(Self::clean_asset_role(Some(r), "other")?),
        };
        // Absent leaves the part where it is; present-and-null makes it loose.
        let (move_sheet, sheet_id) = match req.sheet_id {
            None => (false, None),
            Some(None) => (true, None),
            Some(Some(raw)) => (
                true,
                Some(Uuid::parse_str(&raw).map_err(|_| {
                    AppError::BadRequest("That is not a sheet id".into())
                })?),
            ),
        };
        let rec = self
            .repo
            .update_battle_asset(
                id,
                name.as_deref(),
                role.as_deref(),
                move_sheet,
                sheet_id,
                req.public,
            )
            .await?;
        self.find_asset(rec.id).await
    }

    pub async fn admin_delete_battle_asset(&self, id: Uuid) -> Result<()> {
        let removed = self.repo.delete_battle_asset(id).await?;
        self.remove_upload(&removed.url).await;
        Self::log_domain_event("battle_asset_deleted", "battle_asset", id, "ok");
        Ok(())
    }

    pub async fn admin_reorder_battle_assets(&self, ids: Vec<Uuid>) -> Result<()> {
        let mut seen = HashSet::with_capacity(ids.len());
        if !ids.iter().all(|id| seen.insert(*id)) {
            return Err(AppError::BadRequest(
                "A part cannot stand in two places".into(),
            ));
        }
        self.repo.set_battle_asset_order(&ids).await?;
        Ok(())
    }

    /// Write one picture into the store as a part. The single place a part's
    /// file is made, so a cut, a hand-drawn rectangle and a lone upload can
    /// never disagree about format or quality.
    async fn save_asset_image(
        &self,
        sheet_id: Option<Uuid>,
        name: &str,
        role: &str,
        image: &image::RgbaImage,
        order: i32,
    ) -> Result<BattleAsset> {
        let webp = crate::sheet::to_webp(image, 88.0);
        let relative = format!("assets/{}.webp", Uuid::new_v4());
        self.write_upload(&relative, &webp).await?;
        self.repo
            .insert_battle_asset(
                sheet_id,
                name,
                role,
                &format!("/static/{}", relative),
                image.width() as i32,
                image.height() as i32,
                order,
            )
            .await
    }

    /// One part of a proposed cut, at its full size, for drawing on.
    pub async fn admin_battle_sheet_part(
        &self,
        id: Uuid,
        settings: crate::sheet::SliceSettings,
        index: u32,
    ) -> Result<BattleSheetPartFullDto> {
        let cut = self.cut_sheet(id, settings).await?;
        let part = cut
            .parts
            .into_iter()
            .find(|p| p.index == index)
            .ok_or_else(|| AppError::BadRequest(format!("The sheet has no part {index}")))?;
        let webp = crate::sheet::to_webp(&part.image, 90.0);
        Ok(BattleSheetPartFullDto {
            index: part.index,
            width: part.width,
            height: part.height,
            image: format!(
                "data:image/webp;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&webp)
            ),
        })
    }

    /// Take hand-drawn rectangles off a part that is already in the store.
    ///
    /// The part itself stays where it is. A glued piece is sometimes worth
    /// keeping whole — a book and the bar beside it may yet be wanted as one
    /// picture — and deciding that for the keeper would be deciding it wrong
    /// half the time.
    pub async fn admin_split_battle_asset(
        &self,
        id: Uuid,
        rects: Vec<BattleAssetSplitRect>,
    ) -> Result<Vec<BattleAssetDto>> {
        if rects.is_empty() {
            return Err(AppError::BadRequest("No rectangles were drawn".into()));
        }
        if rects.len() > Self::SPLIT_RECTS_MAX {
            return Err(AppError::BadRequest(format!(
                "At most {} rectangles at a time",
                Self::SPLIT_RECTS_MAX
            )));
        }
        let parent = self.repo.get_battle_asset(id).await?;
        let path = Path::new(&self.config.upload_dir)
            .join(parent.url.trim_start_matches("/static/"));
        let bytes = tokio::fs::read(&path).await.map_err(|e| {
            AppError::NotFound(format!("The part's file is gone ({}): {e}", path.display()))
        })?;
        let source = tokio::task::spawn_blocking(move || {
            image::load_from_memory(&bytes).map(|img| img.to_rgba8())
        })
        .await
        .map_err(|e| AppError::Internal(format!("Split task failed: {e}")))?
        .map_err(|e| AppError::BadRequest(format!("Invalid part file: {e}")))?;

        let mut order = self.repo.next_battle_asset_order(parent.sheet_id).await?;
        let mut saved = Vec::with_capacity(rects.len());
        for (n, rect) in rects.iter().enumerate() {
            let cropped =
                crate::sheet::crop_to_content(&source, &rect.frame(), Self::SPLIT_ALPHA_FLOOR)
                    .ok_or_else(|| {
                        AppError::BadRequest(format!(
                            "Rectangle {} caught nothing but air",
                            n + 1
                        ))
                    })?;
            let name = Self::clean_asset_name(
                rect.name.as_deref().unwrap_or(""),
                &format!("{} · {}", parent.name, n + 1),
            );
            let role = Self::clean_asset_role(rect.role.as_deref(), &parent.role)?;
            let rec = self
                .save_asset_image(parent.sheet_id, &name, &role, &cropped, order)
                .await?;
            order += 1;
            saved.push(rec.id);
        }
        Self::log_domain_event("battle_asset_split", "battle_asset", id, "ok");

        let all = self
            .repo
            .list_battle_assets(AssetSheetFilter::Any, None, None)
            .await?;
        Ok(all
            .into_iter()
            .filter(|row| saved.contains(&row.id))
            .map(Self::asset_dto)
            .collect())
    }

    fn role_name(role: crate::sheet::RoleGuess) -> &'static str {
        match role {
            crate::sheet::RoleGuess::Corner => "corner",
            crate::sheet::RoleGuess::SideH => "sideH",
            crate::sheet::RoleGuess::SideV => "sideV",
            crate::sheet::RoleGuess::Accent => "accent",
        }
    }

    /// Read the stored sheet and cut it. CPU-bound from decode to encode, so
    /// it runs off the async workers.
    async fn cut_sheet(
        &self,
        id: Uuid,
        settings: crate::sheet::SliceSettings,
    ) -> Result<crate::sheet::Sliced> {
        let sheet = self.repo.get_battle_asset_sheet(id).await?;
        let path = Path::new(&self.config.upload_dir)
            .join(sheet.source_url.trim_start_matches("/static/"));
        let bytes = tokio::fs::read(&path).await.map_err(|e| {
            AppError::NotFound(format!("The sheet's file is gone ({}): {e}", path.display()))
        })?;
        tokio::task::spawn_blocking(move || crate::sheet::slice(&bytes, &settings))
            .await
            .map_err(|e| AppError::Internal(format!("Sheet cutting task failed: {e}")))?
            .map_err(|e| AppError::BadRequest(e.to_string()))
    }

    async fn find_asset_sheet(&self, id: Uuid) -> Result<BattleAssetSheetDto> {
        self.list_battle_asset_sheets()
            .await?
            .into_iter()
            .find(|s| s.id == id.to_string())
            .ok_or_else(|| AppError::NotFound(format!("Battle asset sheet {id} not found")))
    }

    async fn find_asset(&self, id: Uuid) -> Result<BattleAssetDto> {
        let rows = self
            .repo
            .list_battle_assets(AssetSheetFilter::Any, None, None)
            .await?;
        rows.into_iter()
            .find(|a| a.id == id)
            .map(Self::asset_dto)
            .ok_or_else(|| AppError::NotFound(format!("Battle asset {id} not found")))
    }

    async fn write_upload(&self, relative: &str, bytes: &[u8]) -> Result<()> {
        let path = Path::new(&self.config.upload_dir).join(relative);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(AppError::Io)?;
        }
        tokio::fs::write(&path, bytes).await.map_err(AppError::Io)?;
        Ok(())
    }

    /// Best-effort: a row that is already gone must not be resurrected by a
    /// missing file, and a file left behind is a job for the media sweep.
    async fn remove_upload(&self, public_url: &str) {
        let relative = public_url.trim_start_matches("/static/");
        if relative.is_empty() || relative == public_url {
            return;
        }
        let path = Path::new(&self.config.upload_dir).join(relative);
        let _ = tokio::fs::remove_file(&path).await;
    }

    /// Массовое назначение расы. Раса, которой нет, — отказ словами, а не
    /// нарушение внешнего ключа: стол держит список рас, и он мог устареть.
    pub async fn admin_set_battle_cards_race(
        &self,
        card_ids: Vec<Uuid>,
        race_id: Option<Uuid>,
    ) -> Result<u64> {
        if card_ids.len() > crate::battles::SHELF_CARDS as usize {
            return Err(AppError::BadRequest("More cards than the shelf holds".into()));
        }
        if let Some(id) = race_id {
            self.find_race(id).await?;
        }
        let changed = self.repo.set_battle_cards_race(&card_ids, race_id).await?;
        for card in &card_ids {
            Self::log_domain_event("battle_card_race_set", "battle_card", *card, "ok");
        }
        Ok(changed)
    }

    pub async fn admin_reorder_battle_races(&self, ids: Vec<Uuid>) -> Result<()> {
        let mut seen = std::collections::HashSet::with_capacity(ids.len());
        if !ids.iter().all(|id| seen.insert(*id)) {
            return Err(AppError::BadRequest(
                "A race cannot stand in two places".into(),
            ));
        }
        self.repo.set_battle_race_order(&ids).await?;
        Ok(())
    }

    /// Read back through the listing, so the card count comes with it.
    async fn find_race(&self, id: Uuid) -> Result<BattleRaceDto> {
        self.list_battle_races()
            .await?
            .into_iter()
            .find(|r| r.id == id.to_string())
            .ok_or_else(|| AppError::NotFound(format!("Battle race {id} not found")))
    }

    fn prepare_race(req: &SaveBattleRaceRequest, taken: &[String]) -> Result<PreparedRace> {
        let cut = |raw: &str| -> String {
            raw.trim()
                .chars()
                .take(crate::battles::RACE_NAME_MAX)
                .collect()
        };
        let name_en = cut(&req.name_en);
        let name_ru = cut(&req.name_ru);
        if name_en.is_empty() || name_ru.is_empty() {
            return Err(AppError::BadRequest(
                "A race needs a name in both languages".into(),
            ));
        }
        let note = |raw: Option<&str>| -> Option<String> {
            raw.map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.chars().take(200).collect())
        };
        let icon_url = req
            .icon_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let level_frames = crate::battles::normalize_level_frames(req.level_frames.as_deref());
        let motion_wear = crate::battles::normalize_motion_wear(req.motion_wear.as_deref());
        Ok(PreparedRace {
            slug: crate::battles::unique_slug(req.slug.as_deref(), &name_en, taken),
            name_en,
            name_ru,
            note_en: note(req.note_en.as_deref()),
            note_ru: note(req.note_ru.as_deref()),
            name_uk: cut(&req.name_uk),
            name_es: cut(&req.name_es),
            name_de: cut(&req.name_de),
            name_it: cut(&req.name_it),
            note_uk: note(req.note_uk.as_deref()),
            note_es: note(req.note_es.as_deref()),
            note_de: note(req.note_de.as_deref()),
            note_it: note(req.note_it.as_deref()),
            icon_url,
            level_frames,
            motion_wear,
        })
    }
}

/// Two places after the point: the scales are read by a person, not summed.
fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

struct PreparedKeyword {
    slug: String,
    name_en: String,
    name_ru: String,
    rules_en: Option<String>,
    rules_ru: Option<String>,
    name_uk: String,
    name_es: String,
    name_de: String,
    name_it: String,
    rules_uk: Option<String>,
    rules_es: Option<String>,
    rules_de: Option<String>,
    rules_it: Option<String>,
    icon_url: Option<String>,
    point_value: Option<f64>,
}

struct PreparedRace {
    slug: String,
    name_en: String,
    name_ru: String,
    note_en: Option<String>,
    note_ru: Option<String>,
    name_uk: String,
    name_es: String,
    name_de: String,
    name_it: String,
    note_uk: Option<String>,
    note_es: Option<String>,
    note_de: Option<String>,
    note_it: Option<String>,
    icon_url: Option<String>,
    level_frames: Option<String>,
    motion_wear: Option<String>,
}

/// The card as it will be written, after clamping. Mirrors `PreparedGazetteLeaf`:
/// the service validates once, the repository only writes.
struct PreparedGazetteLeaf {
    slug: String,
    kind: String,
    status: String,
    title_en: String,
    title_ru: String,
    dek_en: Option<String>,
    dek_ru: Option<String>,
    body_en: Option<String>,
    body_ru: Option<String>,
    title_uk: String,
    title_es: String,
    title_de: String,
    title_it: String,
    dek_uk: Option<String>,
    dek_es: Option<String>,
    dek_de: Option<String>,
    dek_it: Option<String>,
    body_uk: Option<String>,
    body_es: Option<String>,
    body_de: Option<String>,
    body_it: Option<String>,
    figurine_id: Option<Uuid>,
    href: Option<String>,
    source_name: Option<String>,
    source_url: Option<String>,
    author: Option<String>,
    image_url: Option<String>,
    image_urls: Vec<String>,
    pinned: bool,
    published_at: Option<DateTime<Utc>>,
    scheduled_at: Option<DateTime<Utc>>,
    expected_from: Option<chrono::NaiveDate>,
    expected_to: Option<chrono::NaiveDate>,
}

#[cfg(test)]
mod tests;


/// Типы переходов, которые знает отрисовщик плиты (`TaleScenePlate.svelte`).
/// Список закрыт по той же причине, по которой закрыт `ABILITY_VERBS`: новый
/// переход — это новый кусок CSS, а не строка из браузера.
const SCENE_TRANSITIONS: &[&str] = &["fade", "push", "bloom", "wipe", "rise", "flush", "surge"];
const SCENE_RATIOS: &[&str] = &["3 / 4", "4 / 5", "1 / 1", "1 / 2"];
const SCENE_FRAMES_MAX: usize = 8;

/// `object-position` из двух процентов: `22% 50%`. Страница ставит эту строку
/// в стиль как есть, поэтому ничего, кроме двух чисел со знаком процента.
fn scene_position_ok(p: &str) -> bool {
    let mut it = p.split(' ');
    let part = |p: Option<&str>| {
        p.and_then(|p| p.strip_suffix('%'))
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| n <= 100)
    };
    part(it.next()) && part(it.next()) && it.next().is_none()
}

/// Привести сцену в порядок. Браузер пишет что угодно, а страница читателя
/// потом ставит адреса в `src` и числа в стиль, поэтому здесь каждое поле
/// доводится до допустимого, а недопустимое заменяется значением по умолчанию:
/// сцена, которую можно сохранить только безупречной, терялась бы вместе с
/// вечером работы.
fn normalize_tale_scene(mut scene: TaleScene) -> Result<TaleScene> {
    scene.frames.retain(|f| {
        let u = f.url.trim();
        !u.is_empty()
            && u.len() <= 512
            && (u.starts_with('/') || u.starts_with("https://") || u.starts_with("http://localhost"))
            && !u.chars().any(|c| c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '>')
    });
    scene.frames.truncate(SCENE_FRAMES_MAX);
    for f in &mut scene.frames {
        f.url = f.url.trim().to_string();
        f.caption = f.caption.trim().chars().take(200).collect();
        f.caption_ru = f.caption_ru.trim().chars().take(200).collect();
        f.caption_uk = f.caption_uk.trim().chars().take(200).collect();
        f.caption_es = f.caption_es.trim().chars().take(200).collect();
        f.caption_de = f.caption_de.trim().chars().take(200).collect();
        f.caption_it = f.caption_it.trim().chars().take(200).collect();
    }
    if scene.frames.len() < 2 {
        return Err(AppError::BadRequest("A scene needs at least two frames".into()));
    }

    if !SCENE_RATIOS.contains(&scene.ratio.as_str()) {
        scene.ratio = "3 / 4".into();
    }
    if !scene_position_ok(&scene.position) {
        scene.position = "50% 50%".into();
    }
    // Своё положение кадра — либо верное, либо пустое («как у сцены»).
    for f in &mut scene.frames {
        if !scene_position_ok(&f.position) {
            f.position.clear();
        }
    }

    // Переходов на один меньше, чем кадров: лишние отбрасываются, недостающие
    // дописываются спокойным наплывом, а моменты идут по возрастанию — переход
    // не может случиться раньше предыдущего.
    let n = scene.frames.len() - 1;
    scene.transitions.truncate(n);
    while scene.transitions.len() < n {
        scene.transitions.push(TaleSceneTransition {
            kind: "fade".into(),
            at: 0.0,
            span: 0.0,
            manual: false,
        });
    }
    let mut floor = 0.0_f64;
    for (i, t) in scene.transitions.iter_mut().enumerate() {
        if !SCENE_TRANSITIONS.contains(&t.kind.as_str()) {
            t.kind = "fade".into();
        }
        let even = (i as f64 + 1.0) / (n as f64 + 1.0);
        let at = if t.at.is_finite() && t.at > 0.0 { t.at } else { even };
        t.at = at.clamp(floor, 1.0);
        floor = t.at;
        t.span = if t.span.is_finite() && t.span > 0.0 { t.span.clamp(0.04, 0.6) } else { 0.14 };
    }
    Ok(scene)
}

#[cfg(test)]
mod tale_scene_tests {
    use super::*;

    fn frame(url: &str) -> TaleSceneFrame {
        TaleSceneFrame {
            url: url.into(),
            caption: String::new(),
            caption_ru: String::new(),
            caption_uk: String::new(),
            caption_es: String::new(),
            caption_de: String::new(),
            caption_it: String::new(),
            position: String::new(),
        }
    }

    fn scene(frames: Vec<TaleSceneFrame>, transitions: Vec<TaleSceneTransition>) -> TaleScene {
        TaleScene {
            enabled: true,
            ratio: "weird".into(),
            position: "50% 50%".into(),
            frames,
            transitions,
        }
    }

    #[test]
    fn needs_two_frames_and_drops_bad_urls() {
        assert!(normalize_tale_scene(scene(vec![frame("/static/a.jpg")], vec![])).is_err());
        let s = normalize_tale_scene(scene(
            vec![frame("/static/a.jpg"), frame("javascript:alert(1)"), frame("/static/b.jpg"), frame("/x y.jpg")],
            vec![],
        ))
        .unwrap();
        assert_eq!(s.frames.len(), 2);
        assert_eq!(s.ratio, "3 / 4");
    }

    #[test]
    fn transitions_match_frames_are_sorted_and_whitelisted() {
        let t = |kind: &str, at: f64| TaleSceneTransition { kind: kind.into(), at, span: 0.0, manual: false };
        let s = normalize_tale_scene(scene(
            vec![frame("/a"), frame("/b"), frame("/c"), frame("/d")],
            vec![t("bloom", 0.7), t("<script>", 0.2), t("push", 0.9), t("fade", 0.1)],
        ))
        .unwrap();
        assert_eq!(s.transitions.len(), 3);
        assert_eq!(s.transitions[1].kind, "fade");
        assert!(s.transitions.windows(2).all(|w| w[0].at <= w[1].at));
        assert!(s.transitions.iter().all(|t| t.span > 0.0));
    }

    #[test]
    fn position_must_be_two_percentages() {
        let mut sc = scene(vec![frame("/a"), frame("/b")], vec![]);
        sc.position = "left top".into();
        assert_eq!(normalize_tale_scene(sc).unwrap().position, "50% 50%");
        let mut sc = scene(vec![frame("/a"), frame("/b")], vec![]);
        sc.position = "22% 40%".into();
        assert_eq!(normalize_tale_scene(sc).unwrap().position, "22% 40%");
    }
}

/// Что нужно, чтобы досчитать заём (`AppService::lay_with_loan`).
struct LoanInputs {
    fieldable: std::collections::HashSet<Uuid>,
    owned: std::collections::HashSet<Uuid>,
    pool: Vec<Uuid>,
    /// Полка, по которой всё это посчитано: из неё же берутся ранги и снимки,
    /// чтобы не читать её второй раз.
    cards: Vec<BattleCardDto>,
}

impl LoanInputs {
    fn tier_of(&self) -> std::collections::HashMap<Uuid, i16> {
        self.cards
            .iter()
            .filter_map(|c| c.id.parse().ok().map(|id| (id, c.tier)))
            .collect()
    }
}

// ── Words the server writes to visitors, one place per message ─────────────

fn quoted(lang: crate::lang::Lang, name: &str) -> String {
    match lang {
        crate::lang::Lang::En => format!("“{name}”"),
        crate::lang::Lang::De => format!("„{name}“"),
        _ => format!("«{name}»"),
    }
}

/// Тема и первая строка записки «на ваш запрос ответили».
fn order_reply_words(lang: crate::lang::Lang, name: &str) -> (String, String) {
    use crate::lang::Lang;
    let q = quoted(lang, name);
    match lang {
        Lang::En => (format!("A reply to your request — {name}"), format!("Your request about {q} has a reply.")),
        Lang::Ru => (format!("Ответ на ваш запрос — {name}"), format!("Ваш запрос по {q} получил ответ.")),
        Lang::Uk => (format!("Відповідь на ваш запит — {name}"), format!("Ваш запит щодо {q} отримав відповідь.")),
        Lang::Es => (format!("Respuesta a su consulta — {name}"), format!("Su consulta sobre {q} tiene respuesta.")),
        Lang::De => (format!("Antwort auf Ihre Anfrage — {name}"), format!("Ihre Anfrage zu {q} wurde beantwortet.")),
        Lang::It => (format!("Risposta alla sua richiesta — {name}"), format!("La sua richiesta su {q} ha ricevuto una risposta.")),
    }
}

/// Тема и текст записки о брони; `None` — о таком состоянии не пишут.
fn booking_status_words(
    lang: crate::lang::Lang,
    status: &BookingStatus,
    name: &str,
    from: &str,
    to: &str,
) -> Option<(String, String)> {
    use crate::lang::Lang;
    let q = quoted(lang, name);
    let words = match (status, lang) {
        (BookingStatus::Confirmed, Lang::En) => (format!("Booking confirmed — {name}"), format!("Your booking request for {q} ({from} — {to}) is confirmed.")),
        (BookingStatus::Confirmed, Lang::Ru) => (format!("Бронирование подтверждено — {name}"), format!("Ваш запрос на бронирование {q} ({from} — {to}) подтверждён.")),
        (BookingStatus::Confirmed, Lang::Uk) => (format!("Бронювання підтверджено — {name}"), format!("Ваш запит на бронювання {q} ({from} — {to}) підтверджено.")),
        (BookingStatus::Confirmed, Lang::Es) => (format!("Reserva confirmada — {name}"), format!("Su solicitud de reserva de {q} ({from} — {to}) está confirmada.")),
        (BookingStatus::Confirmed, Lang::De) => (format!("Reservierung bestätigt — {name}"), format!("Ihre Reservierungsanfrage für {q} ({from} — {to}) ist bestätigt.")),
        (BookingStatus::Confirmed, Lang::It) => (format!("Prenotazione confermata — {name}"), format!("La sua richiesta di prenotazione di {q} ({from} — {to}) è confermata.")),
        (BookingStatus::Rejected, Lang::En) => (format!("Booking declined — {name}"), format!("Your booking request for {q} ({from} — {to}) has been declined.")),
        (BookingStatus::Rejected, Lang::Ru) => (format!("Бронирование отклонено — {name}"), format!("Ваш запрос на бронирование {q} ({from} — {to}) отклонён.")),
        (BookingStatus::Rejected, Lang::Uk) => (format!("Бронювання відхилено — {name}"), format!("Ваш запит на бронювання {q} ({from} — {to}) відхилено.")),
        (BookingStatus::Rejected, Lang::Es) => (format!("Reserva rechazada — {name}"), format!("Su solicitud de reserva de {q} ({from} — {to}) ha sido rechazada.")),
        (BookingStatus::Rejected, Lang::De) => (format!("Reservierung abgelehnt — {name}"), format!("Ihre Reservierungsanfrage für {q} ({from} — {to}) wurde abgelehnt.")),
        (BookingStatus::Rejected, Lang::It) => (format!("Prenotazione rifiutata — {name}"), format!("La sua richiesta di prenotazione di {q} ({from} — {to}) è stata rifiutata.")),
        (BookingStatus::Cancelled, Lang::En) => (format!("Booking cancelled — {name}"), format!("The booking of {q} ({from} — {to}) is cancelled.")),
        (BookingStatus::Cancelled, Lang::Ru) => (format!("Бронирование отменено — {name}"), format!("Бронирование {q} ({from} — {to}) отменено.")),
        (BookingStatus::Cancelled, Lang::Uk) => (format!("Бронювання скасовано — {name}"), format!("Бронювання {q} ({from} — {to}) скасовано.")),
        (BookingStatus::Cancelled, Lang::Es) => (format!("Reserva cancelada — {name}"), format!("La reserva de {q} ({from} — {to}) está cancelada.")),
        (BookingStatus::Cancelled, Lang::De) => (format!("Reservierung storniert — {name}"), format!("Die Reservierung von {q} ({from} — {to}) ist storniert.")),
        (BookingStatus::Cancelled, Lang::It) => (format!("Prenotazione annullata — {name}"), format!("La prenotazione di {q} ({from} — {to}) è annullata.")),
        (BookingStatus::Completed, Lang::En) => (format!("Booking completed — {name}"), format!("The booking of {q} ({from} — {to}) is completed. Thank you.")),
        (BookingStatus::Completed, Lang::Ru) => (format!("Бронирование завершено — {name}"), format!("Бронирование {q} ({from} — {to}) завершено. Спасибо!")),
        (BookingStatus::Completed, Lang::Uk) => (format!("Бронювання завершено — {name}"), format!("Бронювання {q} ({from} — {to}) завершено. Дякуємо!")),
        (BookingStatus::Completed, Lang::Es) => (format!("Reserva terminada — {name}"), format!("La reserva de {q} ({from} — {to}) ha terminado. Gracias.")),
        (BookingStatus::Completed, Lang::De) => (format!("Reservierung abgeschlossen — {name}"), format!("Die Reservierung von {q} ({from} — {to}) ist abgeschlossen. Danke.")),
        (BookingStatus::Completed, Lang::It) => (format!("Prenotazione conclusa — {name}"), format!("La prenotazione di {q} ({from} — {to}) è conclusa. Grazie.")),
        _ => return None,
    };
    Some(words)
}

/// Записка тем, кто ждал работу в очереди.
fn waitlist_words(lang: crate::lang::Lang, name: &str) -> (String, String) {
    use crate::lang::Lang;
    let q = quoted(lang, name);
    match lang {
        Lang::En => (format!("{q} is available again"), format!("{q}, which you were waiting for, is available again.\n\nThe details are in the archive.")),
        Lang::Ru => (format!("Фигурина {q} снова доступна"), format!("Хорошие новости — фигурина {q}, которую вы ждали, снова доступна.\n\nПосетите архив, чтобы узнать подробности.")),
        Lang::Uk => (format!("Фігурка {q} знову доступна"), format!("Фігурка {q}, на яку ви чекали, знову доступна.\n\nПодробиці — в архіві.")),
        Lang::Es => (format!("{q} vuelve a estar disponible"), format!("{q}, la obra que esperaba, vuelve a estar disponible.\n\nLos detalles están en el archivo.")),
        Lang::De => (format!("{q} ist wieder verfügbar"), format!("{q}, auf die Sie gewartet haben, ist wieder verfügbar.\n\nDie Einzelheiten finden Sie im Archiv.")),
        Lang::It => (format!("{q} è di nuovo disponibile"), format!("{q}, l’opera che aspettava, è di nuovo disponibile.\n\nI dettagli sono nell’archivio.")),
    }
}

/// Записка в Telegram о входе в имя знаками.
fn new_door_note(lang: crate::lang::Lang, where_from: &str) -> String {
    let head = lang.pick(
        "<b>New sign-in</b>\nSomeone has just signed in to your account with the signs.",
        "<b>Новый вход</b>\nВ ваше имя только что вошли знаками.",
        "<b>Новий вхід</b>\nУ ваше ім’я щойно увійшли знаками.",
        "<b>Nueva entrada</b>\nAlguien acaba de entrar en su cuenta con los signos.",
        "<b>Neue Anmeldung</b>\nGerade hat sich jemand mit den Zeichen in Ihrem Konto angemeldet.",
        "<b>Nuovo accesso</b>\nQualcuno ha appena effettuato l’accesso al suo account con i segni.",
    );
    if where_from.is_empty() {
        return head.to_string();
    }
    let from = lang.pick("From", "Откуда", "Звідки", "Desde", "Von", "Da");
    format!("{head}\n{from}: {}", crate::telegram::esc(where_from))
}

/// Тема и подписи письма с ответом на отзыв о работе.
fn impression_reply_words(lang: crate::lang::Lang, name: &str) -> (String, String, &'static str, &'static str) {
    use crate::lang::Lang;
    let q = quoted(lang, name);
    let (subject, yours) = match lang {
        Lang::En => (format!("Re: Your impression of {q}"), format!("Your impression of {q}:")),
        Lang::Ru => (format!("Ответ на ваш отзыв о {q}"), format!("Ваш отзыв о работе {q}:")),
        Lang::Uk => (format!("Відповідь на ваш відгук про {q}"), format!("Ваш відгук про роботу {q}:")),
        Lang::Es => (format!("Re: Su opinión sobre {q}"), format!("Su opinión sobre {q}:")),
        Lang::De => (format!("Re: Ihre Bewertung von {q}"), format!("Ihre Bewertung von {q}:")),
        Lang::It => (format!("Re: Il suo parere su {q}"), format!("Il suo parere su {q}:")),
    };
    let answer = lang.pick(
        "The author's reply:",
        "Ответ автора:",
        "Відповідь автора:",
        "Respuesta de la autora:",
        "Antwort der Autorin:",
        "Risposta dell’autrice:",
    );
    let view = lang.pick("The work", "Работа", "Робота", "La obra", "Die Arbeit", "L’opera");
    (subject, yours, answer, view)
}
