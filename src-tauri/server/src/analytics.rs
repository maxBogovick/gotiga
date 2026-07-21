use crate::db::Repository;
use crate::error::Result;
use crate::models::{AnalyticsEventRecord, AnalyticsEventRequest, AnalyticsEventType};
use chrono::{Duration as ChronoDuration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::net::IpAddr;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, mpsc, watch};
use tokio::task::JoinHandle;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const CHANNEL_CAPACITY: usize = 10_000;
const MAX_BATCH_SIZE: usize = 500;
const FLUSH_INTERVAL: Duration = Duration::from_secs(10);
const AGGREGATE_INTERVAL: Duration = Duration::from_secs(5 * 60);
const SHUTDOWN_FLUSH_DEADLINE: Duration = Duration::from_secs(5);

/// How long raw `figurine_analytics_events` rows survive before
/// `prune_old_analytics_events_chunked` deletes them. Single source of truth
/// for both the prune job (main.rs) and any admin query that needs to know
/// whether a requested date range still has raw events behind it (medians,
/// breakdowns, top-sources) — those must report "no data", not a silent zero,
/// for dates older than this.
pub const RETENTION_DAYS: i64 = 30;

/// A `page_engaged` visit shorter than this counts as a "quick exit" (a near-
/// bounce). Ten seconds is long enough to rule out an accidental open, short
/// enough that the visitor plainly didn't engage. Single source of truth for the
/// quick-exit-rate query in `get_admin_site_page_engagement`.
pub const QUICK_EXIT_MS: i32 = 10_000;

#[derive(Clone)]
pub struct AnalyticsRuntime {
    sender: Arc<Mutex<Option<mpsc::Sender<AnalyticsEventRecord>>>>,
    dropped_total: Arc<AtomicU64>,
    writer_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    aggregate_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    shutdown_tx: watch::Sender<bool>,
    /// Single gate for every caller of `refresh_hot_window` — the background
    /// aggregate_loop tick (which fires immediately on startup, then every
    /// AGGREGATE_INTERVAL) AND any admin HTTP request via
    /// `refresh_hot_window_if_due`. Without one shared gate, a server restart
    /// followed immediately by opening the analytics panel let the
    /// background tick and the panel's several parallel requests all pass a
    /// "not due yet" check at once and run the DELETE+re-INSERT refresh
    /// concurrently — Postgres throws a duplicate-key error for the losers.
    refresh_gate: Arc<Mutex<Option<Instant>>>,
}

pub struct AnalyticsRequestContext<'a> {
    pub headers: &'a axum::http::HeaderMap,
    pub admin_api_key: &'a str,
    pub hash_secret: &'a str,
    pub country_code: Option<String>,
    /// The site's own host (from `PUBLIC_URL`) — lets `classify_source` tell
    /// a same-origin referrer (internal navigation, e.g. archive → figurine
    /// detail) from a genuine external referral, instead of dumping both into
    /// "referral".
    pub site_host: Option<String>,
}

impl AnalyticsRuntime {
    pub fn new(repo: Repository) -> Self {
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let writer_repo = repo.clone();
        let aggregate_repo = repo;
        let refresh_gate: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let aggregate_gate = refresh_gate.clone();

        let writer_handle = tokio::spawn(async move {
            writer_loop(writer_repo, rx).await;
        });
        let aggregate_handle = tokio::spawn(async move {
            aggregate_loop(aggregate_repo, shutdown_rx, aggregate_gate).await;
        });

        Self {
            sender: Arc::new(Mutex::new(Some(tx))),
            dropped_total: Arc::new(AtomicU64::new(0)),
            writer_handle: Arc::new(Mutex::new(Some(writer_handle))),
            aggregate_handle: Arc::new(Mutex::new(Some(aggregate_handle))),
            shutdown_tx,
            refresh_gate,
        }
    }

    /// Refresh figurine_analytics_daily/site_page_views_daily for
    /// yesterday+today, but only if the shared gate hasn't run in the last
    /// 30s — called both by admin HTTP handlers and by the background
    /// aggregate_loop tick, so every caller serializes through one lock.
    pub async fn refresh_hot_window_if_due(&self, repo: &Repository) -> Result<()> {
        refresh_hot_window_gated(repo, &self.refresh_gate).await
    }

    pub async fn try_enqueue(&self, event: AnalyticsEventRecord) -> bool {
        let guard = self.sender.lock().await;
        let Some(sender) = guard.as_ref() else {
            self.dropped_total.fetch_add(1, Ordering::Relaxed);
            return false;
        };
        match sender.try_send(event) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.dropped_total.fetch_add(1, Ordering::Relaxed);
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.dropped_total.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    pub fn dropped_total(&self) -> u64 {
        self.dropped_total.load(Ordering::Relaxed)
    }

    pub async fn shutdown(&self) {
        {
            let mut sender = self.sender.lock().await;
            sender.take();
        }
        let _ = self.shutdown_tx.send(true);

        if let Some(handle) = self.writer_handle.lock().await.take() {
            match tokio::time::timeout(SHUTDOWN_FLUSH_DEADLINE, handle).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::warn!("analytics writer task failed during shutdown: {e}"),
                Err(_) => tracing::warn!("analytics writer shutdown flush deadline exceeded"),
            }
        }
        if let Some(handle) = self.aggregate_handle.lock().await.take() {
            match tokio::time::timeout(Duration::from_secs(2), handle).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::warn!("analytics aggregate task failed during shutdown: {e}")
                }
                Err(_) => tracing::warn!("analytics aggregate shutdown deadline exceeded"),
            }
        }
    }
}

pub fn build_event_record(
    req: AnalyticsEventRequest,
    ctx: AnalyticsRequestContext<'_>,
) -> Result<Option<AnalyticsEventRecord>> {
    if should_skip_request(ctx.headers, ctx.admin_api_key) {
        return Ok(None);
    }

    let figurine_id = req
        .figurine_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| crate::error::AppError::BadRequest("Invalid figurine ID".into()))?;
    let page_view_id = req
        .page_view_id
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| crate::error::AppError::BadRequest("Invalid pageViewId".into()))?;

    validate_event(&req)?;

    let occurred_at = Utc::now();
    let event_date = occurred_at.date_naive();
    let ip_prefix = normalized_ip_prefix(client_ip(ctx.headers).as_deref());
    let hints = client_hints(ctx.headers);
    let visitor_hash = Some(visitor_hash(
        ctx.hash_secret,
        &ip_prefix,
        &hints,
        event_date,
    ));
    let referrer_host = req.referrer.as_deref().and_then(host_of);
    let source = classify_source(
        req.utm_source.as_deref(),
        req.utm_medium.as_deref(),
        referrer_host.as_deref(),
        req.path.as_str(),
        ctx.site_host.as_deref(),
    );
    let user_agent = header_str(ctx.headers, "user-agent");
    let device_class = classify_device(ctx.headers, user_agent.as_deref());
    let browser_family = user_agent.as_deref().map(classify_browser);
    let lang = req.lang.as_deref().and_then(normalize_lang);

    Ok(Some(AnalyticsEventRecord {
        occurred_at,
        event_date,
        event_type: req.event_type.as_str(),
        figurine_id,
        visitor_hash,
        page_view_id,
        path: truncate(req.path, 512),
        source,
        referrer_host,
        utm_source: clean_optional(req.utm_source, 120),
        utm_medium: clean_optional(req.utm_medium, 120),
        utm_campaign: clean_optional(req.utm_campaign, 160),
        device_class,
        browser_family,
        country_code: ctx.country_code,
        duration_ms: req.duration_ms,
        scroll_depth: req.scroll_depth,
        works_seen: req.works_seen,
        cta_type: clean_optional(req.cta_type, 80),
        user_id: None,
        lang,
        internal_source: clean_optional(req.internal_source, 60),
    }))
}

fn normalize_lang(raw: &str) -> Option<String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "en" => Some("en".into()),
        "ru" => Some("ru".into()),
        _ => None,
    }
}

fn validate_event(req: &AnalyticsEventRequest) -> Result<()> {
    if req.path.trim().is_empty() || req.path.len() > 512 {
        return Err(crate::error::AppError::BadRequest("Invalid path".into()));
    }
    let site_wide = matches!(
        req.event_type,
        AnalyticsEventType::PageView | AnalyticsEventType::PageEngaged
    );
    if !site_wide
        && req
            .figurine_id
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        return Err(crate::error::AppError::BadRequest(
            "figurineId is required for this event type".into(),
        ));
    }
    if let Some(works) = req.works_seen
        && works < 0
    {
        return Err(crate::error::AppError::BadRequest(
            "worksSeen must be non-negative".into(),
        ));
    }
    if let Some(duration) = req.duration_ms
        && duration < 0
    {
        return Err(crate::error::AppError::BadRequest(
            "durationMs must be non-negative".into(),
        ));
    }
    if let Some(scroll) = req.scroll_depth
        && !(0..=100).contains(&scroll)
    {
        return Err(crate::error::AppError::BadRequest(
            "scrollDepth must be 0..100".into(),
        ));
    }
    if req.event_type == AnalyticsEventType::FigurineCtaClick
        && req.cta_type.as_deref().unwrap_or("").trim().is_empty()
    {
        return Err(crate::error::AppError::BadRequest(
            "ctaType is required for cta events".into(),
        ));
    }
    Ok(())
}

async fn writer_loop(repo: Repository, mut rx: mpsc::Receiver<AnalyticsEventRecord>) {
    let mut batch = Vec::with_capacity(MAX_BATCH_SIZE);
    let mut tick = tokio::time::interval(FLUSH_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            maybe = rx.recv() => {
                match maybe {
                    Some(event) => {
                        batch.push(event);
                        if batch.len() >= MAX_BATCH_SIZE {
                            flush_batch(&repo, &mut batch).await;
                        }
                    }
                    None => {
                        flush_batch(&repo, &mut batch).await;
                        break;
                    }
                }
            }
            _ = tick.tick() => {
                flush_batch(&repo, &mut batch).await;
            }
        }
    }
}

async fn flush_batch(repo: &Repository, batch: &mut Vec<AnalyticsEventRecord>) {
    if batch.is_empty() {
        return;
    }
    let events = std::mem::take(batch);
    let count = events.len();
    if let Err(e) = repo.bulk_insert_analytics_events(&events).await {
        tracing::warn!(count, error = %e, "analytics batch insert failed");
    }
}

async fn aggregate_loop(
    repo: Repository,
    mut shutdown_rx: watch::Receiver<bool>,
    refresh_gate: Arc<Mutex<Option<Instant>>>,
) {
    let mut tick = tokio::time::interval(AGGREGATE_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    break;
                }
            }
            _ = tick.tick() => {
                // tokio::time::interval fires its first tick immediately, so
                // this runs right at startup — routed through the same gate
                // an admin HTTP request would use, so it can't race one.
                if let Err(e) = refresh_hot_window_gated(&repo, &refresh_gate).await {
                    tracing::warn!(error = %e, "analytics aggregate refresh failed");
                }
            }
        }
    }
}

pub async fn refresh_hot_window(repo: &Repository) -> Result<()> {
    let today = Utc::now().date_naive();
    let from = today - ChronoDuration::days(1);
    repo.refresh_analytics_aggregates(from, today).await
}

async fn refresh_hot_window_gated(repo: &Repository, gate: &Mutex<Option<Instant>>) -> Result<()> {
    const MIN_REFRESH_GAP: Duration = Duration::from_secs(30);
    // Held across the refresh itself, not just the check, so concurrent
    // callers serialize instead of racing (see AnalyticsRuntime::refresh_gate).
    let mut last = gate.lock().await;
    if last.is_some_and(|instant| instant.elapsed() < MIN_REFRESH_GAP) {
        return Ok(());
    }
    refresh_hot_window(repo).await?;
    *last = Some(Instant::now());
    Ok(())
}

fn should_skip_request(headers: &axum::http::HeaderMap, admin_api_key: &str) -> bool {
    if header_str(headers, "dnt").as_deref() == Some("1") {
        return true;
    }
    if header_str(headers, "authorization")
        .as_deref()
        .is_some_and(|v| v == format!("Bearer {admin_api_key}"))
    {
        return true;
    }
    let ua = header_str(headers, "user-agent")
        .unwrap_or_default()
        .to_ascii_lowercase();
    ua.contains("bot")
        || ua.contains("crawler")
        || ua.contains("spider")
        || ua.contains("preview")
        || ua.contains("lighthouse")
}

fn client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    for name in ["cf-connecting-ip", "x-real-ip", "x-forwarded-for"] {
        if let Some(value) = header_str(headers, name) {
            let first = value.split(',').next().unwrap_or("").trim();
            if !first.is_empty() {
                return Some(first.to_string());
            }
        }
    }
    None
}

fn normalized_ip_prefix(ip: Option<&str>) -> String {
    let Some(ip) = ip.and_then(|s| s.parse::<IpAddr>().ok()) else {
        return "unknown".into();
    };
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            format!("{}.{}.{}.0/24", o[0], o[1], o[2])
        }
        IpAddr::V6(v6) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}:0:0:0:0:0/48", s[0], s[1], s[2])
        }
    }
}

fn client_hints(headers: &axum::http::HeaderMap) -> String {
    let ch = [
        header_str(headers, "sec-ch-ua"),
        header_str(headers, "sec-ch-ua-mobile"),
        header_str(headers, "sec-ch-ua-platform"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("|");
    if ch.trim().is_empty() {
        header_str(headers, "user-agent").unwrap_or_else(|| "unknown".into())
    } else {
        ch
    }
}

fn visitor_hash(secret: &str, ip_prefix: &str, hints: &str, day: chrono::NaiveDate) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length for SHA-256");
    mac.update(ip_prefix.as_bytes());
    mac.update(b"|");
    mac.update(hints.as_bytes());
    mac.update(b"|");
    mac.update(day.to_string().as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Parse a URL string down to its lowercased host. Used both for a request's
/// `Referer` header and — via `AnalyticsRequestContext::site_host` — for the
/// site's own `PUBLIC_URL`, so `classify_source` can recognize a same-origin
/// referrer as internal navigation rather than an external referral.
pub(crate) fn host_of(raw: &str) -> Option<String> {
    let parsed = raw.parse::<axum::http::Uri>().ok()?;
    parsed.host().map(|h| truncate(h.to_ascii_lowercase(), 180))
}

/// True when `host` is the site's own domain (a leading "www." on either side
/// is ignored), so browsing from one page to another on the same site (e.g.
/// the archive to a figurine's detail page) classifies as "internal", not a
/// "referral" — which previously lumped genuine external referrals together
/// with the site's own internal navigation.
fn is_same_site(host: &str, site_host: Option<&str>) -> bool {
    let Some(site) = site_host else {
        return false;
    };
    strip_www(host) == strip_www(site)
}

fn strip_www(host: &str) -> &str {
    host.strip_prefix("www.").unwrap_or(host)
}

fn classify_source(
    utm_source: Option<&str>,
    utm_medium: Option<&str>,
    referrer_host: Option<&str>,
    path: &str,
    site_host: Option<&str>,
) -> String {
    let source = utm_source.unwrap_or("").trim().to_ascii_lowercase();
    let medium = utm_medium.unwrap_or("").trim().to_ascii_lowercase();
    if !source.is_empty() {
        if source.contains("instagram")
            || source.contains("facebook")
            || source.contains("tiktok")
            || source.contains("pinterest")
            || source.contains("telegram")
            || medium == "social"
        {
            return "social".into();
        }
        if source.contains("newsletter") || medium == "email" {
            return "newsletter".into();
        }
        if source.contains("google")
            || source.contains("yandex")
            || source.contains("bing")
            || medium == "organic"
        {
            return "search".into();
        }
        return truncate(source, 80);
    }
    if path.starts_with("/figurines") && referrer_host.is_none() {
        return "direct".into();
    }
    let Some(host) = referrer_host else {
        return "direct".into();
    };
    if is_same_site(host, site_host) {
        return "internal".into();
    }
    if host.contains("google.") || host.contains("yandex.") || host.contains("bing.") {
        "search".into()
    } else if host.contains("instagram.")
        || host.contains("facebook.")
        || host.contains("tiktok.")
        || host.contains("pinterest.")
        || host.contains("t.me")
    {
        "social".into()
    } else {
        "referral".into()
    }
}

fn classify_device(headers: &axum::http::HeaderMap, user_agent: Option<&str>) -> Option<String> {
    if header_str(headers, "sec-ch-ua-mobile").as_deref() == Some("?1") {
        return Some("mobile".into());
    }
    let ua = user_agent.unwrap_or("").to_ascii_lowercase();
    if ua.contains("mobile") || ua.contains("iphone") || ua.contains("android") {
        Some("mobile".into())
    } else if ua.contains("ipad") || ua.contains("tablet") {
        Some("tablet".into())
    } else if ua.is_empty() {
        None
    } else {
        Some("desktop".into())
    }
}

fn classify_browser(user_agent: &str) -> String {
    let ua = user_agent.to_ascii_lowercase();
    if ua.contains("firefox") {
        "firefox".into()
    } else if ua.contains("edg/") {
        "edge".into()
    } else if ua.contains("chrome") || ua.contains("chromium") {
        "chromium".into()
    } else if ua.contains("safari") {
        "safari".into()
    } else {
        "other".into()
    }
}

fn header_str(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn clean_optional(value: Option<String>, max: usize) -> Option<String> {
    value
        .map(|s| truncate(s.trim(), max))
        .filter(|s| !s.is_empty())
}

fn truncate(value: impl AsRef<str>, max: usize) -> String {
    value.as_ref().chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn visitor_hash_changes_by_day() {
        let a = visitor_hash(
            "secretsecretsecret",
            "127.0.0.0/24",
            "ua",
            Utc::now().date_naive(),
        );
        let b = visitor_hash(
            "secretsecretsecret",
            "127.0.0.0/24",
            "ua",
            Utc::now().date_naive() + ChronoDuration::days(1),
        );
        assert_ne!(a, b);
    }

    #[test]
    fn source_uses_social_utm() {
        assert_eq!(
            classify_source(Some("instagram"), Some("social"), None, "/figurines/x", None),
            "social"
        );
    }

    #[test]
    fn source_classifies_same_site_referrer_as_internal() {
        assert_eq!(
            classify_source(
                None,
                None,
                Some("gotiga.example"),
                "/figurines/x",
                Some("gotiga.example"),
            ),
            "internal"
        );
        // A leading "www." on either side shouldn't defeat the match.
        assert_eq!(
            classify_source(
                None,
                None,
                Some("www.gotiga.example"),
                "/figurines/x",
                Some("gotiga.example"),
            ),
            "internal"
        );
    }

    #[test]
    fn source_classifies_other_domain_as_referral() {
        assert_eq!(
            classify_source(
                None,
                None,
                Some("some-other-blog.example"),
                "/figurines/x",
                Some("gotiga.example"),
            ),
            "referral"
        );
    }

    #[test]
    fn skips_admin_authorized_event() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer strong_admin_token"),
        );
        assert!(should_skip_request(&headers, "strong_admin_token"));
    }

    #[test]
    fn rejects_invalid_scroll_depth() {
        let req = AnalyticsEventRequest {
            event_type: AnalyticsEventType::FigurineEngaged,
            figurine_id: Some(Uuid::new_v4().to_string()),
            path: "/figurines/x".into(),
            referrer: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            duration_ms: Some(1000),
            scroll_depth: Some(101),
            works_seen: None,
            cta_type: None,
            page_view_id: Some(Uuid::new_v4().to_string()),
            client_ts: None,
            lang: None,
            internal_source: None,
        };
        assert!(validate_event(&req).is_err());
    }

    #[test]
    fn rejects_missing_figurine_id_for_figurine_events() {
        let req = AnalyticsEventRequest {
            event_type: AnalyticsEventType::FigurineView,
            figurine_id: None,
            path: "/figurines/x".into(),
            referrer: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            duration_ms: None,
            scroll_depth: None,
            works_seen: None,
            cta_type: None,
            page_view_id: None,
            client_ts: None,
            lang: None,
            internal_source: None,
        };
        assert!(validate_event(&req).is_err());
    }

    #[test]
    fn accepts_missing_figurine_id_for_page_engaged() {
        let req = AnalyticsEventRequest {
            event_type: AnalyticsEventType::PageEngaged,
            figurine_id: None,
            path: "/".into(),
            referrer: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            duration_ms: Some(12000),
            scroll_depth: Some(60),
            works_seen: Some(7),
            cta_type: None,
            page_view_id: Some(Uuid::new_v4().to_string()),
            client_ts: None,
            lang: Some("ru".into()),
            internal_source: None,
        };
        assert!(validate_event(&req).is_ok());
    }

    #[test]
    fn rejects_negative_works_seen() {
        let req = AnalyticsEventRequest {
            event_type: AnalyticsEventType::PageEngaged,
            figurine_id: None,
            path: "/".into(),
            referrer: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            duration_ms: Some(1000),
            scroll_depth: Some(10),
            works_seen: Some(-1),
            cta_type: None,
            page_view_id: None,
            client_ts: None,
            lang: None,
            internal_source: None,
        };
        assert!(validate_event(&req).is_err());
    }

    #[test]
    fn accepts_missing_figurine_id_for_page_view() {
        let req = AnalyticsEventRequest {
            event_type: AnalyticsEventType::PageView,
            figurine_id: None,
            path: "/".into(),
            referrer: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            duration_ms: None,
            scroll_depth: None,
            works_seen: None,
            cta_type: None,
            page_view_id: None,
            client_ts: None,
            lang: Some("RU".into()),
            internal_source: None,
        };
        assert!(validate_event(&req).is_ok());
    }

    #[test]
    fn builds_record_from_client_hints_without_raw_ip_storage() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("192.168.10.22"));
        headers.insert(
            "sec-ch-ua",
            HeaderValue::from_static("\"Chromium\";v=\"126\""),
        );
        headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
        headers.insert("sec-ch-ua-platform", HeaderValue::from_static("\"macOS\""));
        let figurine_id = Uuid::new_v4();
        let req = AnalyticsEventRequest {
            event_type: AnalyticsEventType::FigurineView,
            figurine_id: Some(figurine_id.to_string()),
            path: "/figurines/x?utm_source=instagram".into(),
            referrer: Some("https://instagram.com/some/path?private=1".into()),
            utm_source: Some("instagram".into()),
            utm_medium: Some("social".into()),
            utm_campaign: None,
            duration_ms: None,
            scroll_depth: None,
            works_seen: None,
            cta_type: None,
            page_view_id: Some(Uuid::new_v4().to_string()),
            client_ts: None,
            lang: Some("en".into()),
            internal_source: Some("home_afisha".into()),
        };
        let record = build_event_record(
            req,
            AnalyticsRequestContext {
                headers: &headers,
                admin_api_key: "admin-token",
                hash_secret: "analytics-secret-for-tests",
                country_code: None,
                site_host: None,
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(record.figurine_id, Some(figurine_id));
        assert_eq!(record.source, "social");
        assert_eq!(record.referrer_host.as_deref(), Some("instagram.com"));
        assert_ne!(record.visitor_hash.as_deref(), Some("192.168.10.22"));
        assert_eq!(record.lang.as_deref(), Some("en"));
        assert_eq!(record.internal_source.as_deref(), Some("home_afisha"));
    }
}
