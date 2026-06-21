use crate::error::Result;
use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::time::Instant;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::field::{Field, Visit};
use tracing::{Event as TracingEvent, Level, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context as LayerContext;

const CHANNEL_CAPACITY: usize = 4096;
const BROADCAST_CAPACITY: usize = 512;
const BATCH_MAX: usize = 128;
const BATCH_MAX_WAIT: Duration = Duration::from_millis(250);
const MAX_MESSAGE_LEN: usize = 2_000;
const MAX_FIELD_VALUE_LEN: usize = 1_000;
const MAX_TEXT_QUERY_LEN: usize = 200;
const DEFAULT_RETENTION_DAYS: i64 = 14;

#[derive(Clone)]
pub struct AdminLogStore {
    pool: SqlitePool,
    sender: mpsc::Sender<LogWrite>,
    broadcaster: broadcast::Sender<AdminLogEntry>,
    dropped: Arc<AtomicU64>,
}

#[derive(Clone)]
pub struct AdminLogLayer {
    sender: mpsc::Sender<LogWrite>,
    dropped: Arc<AtomicU64>,
}

#[derive(Clone, Debug)]
struct LogWrite {
    ts: String,
    level: String,
    target: String,
    message: String,
    request_id: Option<String>,
    method: Option<String>,
    route: Option<String>,
    status: Option<i64>,
    latency_ms: Option<i64>,
    fields_json: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminLogEntry {
    id: i64,
    ts: String,
    level: String,
    target: String,
    message: String,
    request_id: Option<String>,
    method: Option<String>,
    route: Option<String>,
    status: Option<i64>,
    latency_ms: Option<i64>,
    fields: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminLogsPage {
    items: Vec<AdminLogEntry>,
    next_before_id: Option<i64>,
    next_offset: Option<i64>,
    dropped_total: u64,
}

#[derive(Debug, Deserialize)]
pub struct AdminLogsQuery {
    before_id: Option<i64>,
    from: Option<String>,
    to: Option<String>,
    level: Option<String>,
    request_id: Option<String>,
    route: Option<String>,
    method: Option<String>,
    status: Option<i64>,
    status_class: Option<i64>,
    min_latency_ms: Option<i64>,
    max_latency_ms: Option<i64>,
    target: Option<String>,
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
}

impl AdminLogStore {
    pub async fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(2));
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await?;

        Self::migrate(&pool).await?;

        let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let (broadcaster, _) = broadcast::channel(BROADCAST_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));

        tokio::spawn(writer_task(
            pool.clone(),
            receiver,
            broadcaster.clone(),
            dropped.clone(),
        ));
        tokio::spawn(retention_task(pool.clone()));

        Ok(Self {
            pool,
            sender,
            broadcaster,
            dropped,
        })
    }

    pub fn layer(&self) -> AdminLogLayer {
        AdminLogLayer {
            sender: self.sender.clone(),
            dropped: self.dropped.clone(),
        }
    }

    async fn migrate(pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS admin_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts TEXT NOT NULL,
                level TEXT NOT NULL,
                target TEXT NOT NULL,
                message TEXT NOT NULL,
                request_id TEXT,
                method TEXT,
                route TEXT,
                status INTEGER,
                latency_ms INTEGER,
                fields_json TEXT NOT NULL
            );
            "#,
        )
        .execute(pool)
        .await?;
        for ddl in [
            "CREATE INDEX IF NOT EXISTS idx_admin_logs_ts ON admin_logs(ts DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_admin_logs_level_ts ON admin_logs(level, ts DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_admin_logs_request_id ON admin_logs(request_id)",
            "CREATE INDEX IF NOT EXISTS idx_admin_logs_route_ts ON admin_logs(route, ts DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_admin_logs_status_ts ON admin_logs(status, ts DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_admin_logs_latency_ts ON admin_logs(latency_ms, ts DESC, id DESC)",
        ] {
            sqlx::query(ddl).execute(pool).await?;
        }
        Ok(())
    }

    pub async fn query(&self, params: &AdminLogsQuery) -> Result<AdminLogsPage> {
        let limit = params.limit.unwrap_or(200).clamp(1, 500);
        let offset = params.offset.unwrap_or(0).clamp(0, 100_000);
        let mut sql = String::from(
            "SELECT id, ts, level, target, message, request_id, method, route, status, latency_ms, fields_json FROM admin_logs WHERE 1=1",
        );
        let mut args: Vec<BindValue> = Vec::new();

        if let Some(v) = params.before_id {
            sql.push_str(" AND id < ?");
            args.push(BindValue::I64(v));
        }
        if let Some(v) = clean_filter(params.from.as_deref(), 64) {
            sql.push_str(" AND ts >= ?");
            args.push(BindValue::Text(v));
        }
        if let Some(v) = clean_filter(params.to.as_deref(), 64) {
            sql.push_str(" AND ts <= ?");
            args.push(BindValue::Text(v));
        }
        if let Some(v) = clean_filter(params.level.as_deref(), 16) {
            sql.push_str(" AND level = ?");
            args.push(BindValue::Text(v.to_ascii_uppercase()));
        }
        if let Some(v) = clean_filter(params.request_id.as_deref(), 128) {
            sql.push_str(" AND request_id = ?");
            args.push(BindValue::Text(v));
        }
        if let Some(v) = clean_filter(params.route.as_deref(), 200) {
            sql.push_str(" AND route = ?");
            args.push(BindValue::Text(v));
        }
        if let Some(v) = clean_filter(params.method.as_deref(), 16) {
            sql.push_str(" AND method = ?");
            args.push(BindValue::Text(v.to_ascii_uppercase()));
        }
        if let Some(v) = params.status {
            sql.push_str(" AND status = ?");
            args.push(BindValue::I64(v));
        }
        if let Some(v) = params.status_class {
            let low = if (1..=5).contains(&v) {
                v * 100
            } else {
                (v / 100) * 100
            };
            if (100..600).contains(&low) {
                sql.push_str(" AND status >= ? AND status < ?");
                args.push(BindValue::I64(low));
                args.push(BindValue::I64(low + 100));
            }
        }
        if let Some(v) = params.min_latency_ms {
            sql.push_str(" AND latency_ms >= ?");
            args.push(BindValue::I64(v));
        }
        if let Some(v) = params.max_latency_ms {
            sql.push_str(" AND latency_ms <= ?");
            args.push(BindValue::I64(v));
        }
        if let Some(v) = clean_filter(params.target.as_deref(), 200) {
            sql.push_str(" AND target = ?");
            args.push(BindValue::Text(v));
        }
        if let Some(v) = clean_filter(params.q.as_deref(), MAX_TEXT_QUERY_LEN) {
            sql.push_str(" AND (message LIKE ? ESCAPE '\\' OR fields_json LIKE ? ESCAPE '\\')");
            let like = format!("%{}%", escape_like(&v));
            args.push(BindValue::Text(like.clone()));
            args.push(BindValue::Text(like));
        }

        let (sort_column, nulls_last) = sort_column(params.sort_by.as_deref());
        let sort_dir = sort_direction(params.sort_dir.as_deref());
        sql.push_str(" ORDER BY ");
        if nulls_last {
            sql.push_str(sort_column);
            sql.push_str(" IS NULL, ");
        }
        sql.push_str(sort_column);
        sql.push(' ');
        sql.push_str(sort_dir);
        sql.push_str(", id ");
        sql.push_str(sort_dir);
        sql.push_str(" LIMIT ? OFFSET ?");
        args.push(BindValue::I64(limit));
        args.push(BindValue::I64(offset));

        let mut query = sqlx::query(&sql);
        for arg in args {
            query = match arg {
                BindValue::Text(v) => query.bind(v),
                BindValue::I64(v) => query.bind(v),
            };
        }

        let rows = query.fetch_all(&self.pool).await?;
        let items = rows.into_iter().map(row_to_entry).collect::<Vec<_>>();
        let next_before_id = (items.len() as i64 == limit)
            .then(|| items.last().map(|i| i.id))
            .flatten();
        let next_offset = (items.len() as i64 == limit).then_some(offset + limit);
        Ok(AdminLogsPage {
            items,
            next_before_id,
            next_offset,
            dropped_total: self.dropped.load(Ordering::Relaxed),
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AdminLogEntry> {
        self.broadcaster.subscribe()
    }
}

enum BindValue {
    Text(String),
    I64(i64),
}

async fn writer_task(
    pool: SqlitePool,
    mut receiver: mpsc::Receiver<LogWrite>,
    broadcaster: broadcast::Sender<AdminLogEntry>,
    dropped: Arc<AtomicU64>,
) {
    let mut batch = Vec::with_capacity(BATCH_MAX);
    let mut flush_at = Instant::now() + BATCH_MAX_WAIT;
    loop {
        tokio::select! {
            item = receiver.recv() => {
                let Some(item) = item else { break; };
                batch.push(item);
                if batch.len() >= BATCH_MAX {
                    flush_batch(&pool, &broadcaster, &dropped, &mut batch).await;
                    flush_at = Instant::now() + BATCH_MAX_WAIT;
                }
            }
            _ = tokio::time::sleep_until(flush_at), if !batch.is_empty() => {
                flush_batch(&pool, &broadcaster, &dropped, &mut batch).await;
                flush_at = Instant::now() + BATCH_MAX_WAIT;
            }
        }
    }
    if !batch.is_empty() {
        flush_batch(&pool, &broadcaster, &dropped, &mut batch).await;
    }
}

async fn flush_batch(
    pool: &SqlitePool,
    broadcaster: &broadcast::Sender<AdminLogEntry>,
    dropped: &AtomicU64,
    batch: &mut Vec<LogWrite>,
) {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            dropped.fetch_add(batch.len() as u64, Ordering::Relaxed);
            batch.clear();
            return;
        }
    };

    let mut inserted = Vec::with_capacity(batch.len());
    for item in batch.drain(..) {
        let res = sqlx::query(
            r#"
            INSERT INTO admin_logs
                (ts, level, target, message, request_id, method, route, status, latency_ms, fields_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, ts, level, target, message, request_id, method, route, status, latency_ms, fields_json
            "#,
        )
        .bind(item.ts)
        .bind(item.level)
        .bind(item.target)
        .bind(item.message)
        .bind(item.request_id)
        .bind(item.method)
        .bind(item.route)
        .bind(item.status)
        .bind(item.latency_ms)
        .bind(item.fields_json)
        .fetch_one(&mut *tx)
        .await;

        match res {
            Ok(row) => {
                inserted.push(row_to_entry(row));
            }
            Err(_) => {
                dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    if tx.commit().await.is_ok() {
        for entry in inserted {
            let _ = broadcaster.send(entry);
        }
    } else {
        dropped.fetch_add(inserted.len() as u64, Ordering::Relaxed);
    }
}

async fn retention_task(pool: SqlitePool) {
    let mut tick = tokio::time::interval(Duration::from_secs(3600));
    loop {
        tick.tick().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(DEFAULT_RETENTION_DAYS);
        let cutoff = cutoff.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let _ = sqlx::query("DELETE FROM admin_logs WHERE ts < ?")
            .bind(cutoff)
            .execute(&pool)
            .await;
    }
}

fn row_to_entry(row: sqlx::sqlite::SqliteRow) -> AdminLogEntry {
    let fields_json: String = row.get("fields_json");
    AdminLogEntry {
        id: row.get("id"),
        ts: row.get("ts"),
        level: row.get("level"),
        target: row.get("target"),
        message: row.get("message"),
        request_id: row.get("request_id"),
        method: row.get("method"),
        route: row.get("route"),
        status: row.get("status"),
        latency_ms: row.get("latency_ms"),
        fields: serde_json::from_str(&fields_json).unwrap_or_else(|_| serde_json::json!({})),
    }
}

impl<S> Layer<S> for AdminLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &TracingEvent<'_>, _ctx: LayerContext<'_, S>) {
        let meta = event.metadata();
        if meta.target().starts_with("sqlx") || meta.target().starts_with("hyper") {
            return;
        }
        if self.sender.capacity() == 0 {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        let fields_json = serde_json::to_string(&visitor.fields).unwrap_or_else(|_| "{}".into());
        let message = visitor.message.clone().unwrap_or_default();
        let write = LogWrite {
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: level_string(meta.level()),
            target: truncate(meta.target(), 200),
            message: truncate(&message, MAX_MESSAGE_LEN),
            request_id: visitor.take_string("request_id", 128),
            method: visitor.take_string("method", 16),
            route: visitor.take_string("route", 200),
            status: visitor.take_i64("status"),
            latency_ms: visitor.take_i64("latency_ms"),
            fields_json,
        };

        if self.sender.try_send(write).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[derive(Default)]
struct EventVisitor {
    message: Option<String>,
    fields: BTreeMap<String, serde_json::Value>,
}

impl EventVisitor {
    fn take_string(&mut self, key: &str, max: usize) -> Option<String> {
        self.fields
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, max))
    }

    fn take_i64(&mut self, key: &str) -> Option<i64> {
        self.fields.get(key).and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_u64().and_then(|n| i64::try_from(n).ok()))
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
    }

    fn insert_value(&mut self, field: &Field, value: serde_json::Value) {
        let name = field.name();
        if sensitive_field(name) {
            self.fields.insert(
                name.to_string(),
                serde_json::Value::String("[redacted]".into()),
            );
            return;
        }
        self.fields.insert(name.to_string(), value);
    }
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let text = truncate(&format!("{value:?}"), MAX_FIELD_VALUE_LEN);
        if field.name() == "message" {
            self.message = Some(text.trim_matches('"').to_string());
        } else {
            self.insert_value(field, serde_json::Value::String(text));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        let text = truncate(value, MAX_FIELD_VALUE_LEN);
        if field.name() == "message" {
            self.message = Some(text);
        } else {
            self.insert_value(field, serde_json::Value::String(text));
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert_value(field, serde_json::json!(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert_value(field, serde_json::json!(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert_value(field, serde_json::json!(value));
    }
}

fn level_string(level: &Level) -> String {
    level.as_str().to_ascii_uppercase()
}

fn sensitive_field(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("token")
        || n.contains("password")
        || n.contains("authorization")
        || n.contains("api_key")
        || n.contains("secret")
        || n.contains("cookie")
}

fn truncate(value: &str, max: usize) -> String {
    if value.len() <= max {
        return value.to_string();
    }
    value.chars().take(max).collect()
}

fn clean_filter(value: Option<&str>, max: usize) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| truncate(s, max))
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn sort_column(value: Option<&str>) -> (&'static str, bool) {
    match value.unwrap_or("time") {
        "time" | "ts" => ("ts", false),
        "level" => ("level", false),
        "request" | "request_id" => ("request_id", true),
        "route" => ("route", true),
        "status" => ("status", true),
        "latency" | "latency_ms" => ("latency_ms", true),
        "message" => ("message", false),
        _ => ("ts", false),
    }
}

fn sort_direction(value: Option<&str>) -> &'static str {
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    }
}

pub async fn admin_list_logs(
    State(store): State<AdminLogStore>,
    Query(params): Query<AdminLogsQuery>,
) -> Result<axum::Json<AdminLogsPage>> {
    Ok(axum::Json(store.query(&params).await?))
}

pub async fn admin_stream_logs(
    State(store): State<AdminLogStore>,
) -> Sse<impl tokio_stream::Stream<Item = std::result::Result<Event, std::convert::Infallible>>> {
    let stream = BroadcastStream::new(store.subscribe()).filter_map(|res| match res {
        Ok(item) => {
            let json = serde_json::to_string(&item).unwrap_or_else(|_| "{}".into());
            Some(Ok(Event::default().event("log").data(json)))
        }
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_sensitive_fields() {
        assert!(sensitive_field("reset_token"));
        assert!(sensitive_field("Authorization"));
        assert!(!sensitive_field("route"));
    }

    #[test]
    fn escapes_like_filters() {
        assert_eq!(escape_like("a%b_c\\d"), "a\\%b\\_c\\\\d");
    }

    #[tokio::test]
    async fn async_writer_persists_and_queries_logs() {
        let path = std::env::temp_dir().join(format!(
            "gotiga-admin-log-test-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let store = AdminLogStore::open(&path).await.unwrap();
        store
            .sender
            .send(LogWrite {
                ts: "2026-06-21T12:00:00.000Z".into(),
                level: "INFO".into(),
                target: "gotiga_server::http".into(),
                message: "request completed".into(),
                request_id: Some("req-1".into()),
                method: Some("GET".into()),
                route: Some("/api/v1/health".into()),
                status: Some(200),
                latency_ms: Some(3),
                fields_json: serde_json::json!({"request_id":"req-1"}).to_string(),
            })
            .await
            .unwrap();
        store
            .sender
            .send(LogWrite {
                ts: "2026-06-21T12:00:01.000Z".into(),
                level: "WARN".into(),
                target: "gotiga_server::http".into(),
                message: "not found".into(),
                request_id: Some("req-2".into()),
                method: Some("GET".into()),
                route: Some("/api/v1/missing".into()),
                status: Some(404),
                latency_ms: Some(12),
                fields_json: serde_json::json!({"request_id":"req-2"}).to_string(),
            })
            .await
            .unwrap();

        tokio::time::sleep(BATCH_MAX_WAIT + Duration::from_millis(100)).await;
        let page = store
            .query(&AdminLogsQuery {
                request_id: Some("req-1".into()),
                limit: Some(10),
                before_id: None,
                from: None,
                to: None,
                level: None,
                route: None,
                method: None,
                status: None,
                status_class: None,
                min_latency_ms: None,
                max_latency_ms: None,
                target: None,
                q: None,
                offset: None,
                sort_by: None,
                sort_dir: None,
            })
            .await
            .unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].request_id.as_deref(), Some("req-1"));
        assert_eq!(page.items[0].route.as_deref(), Some("/api/v1/health"));

        let page = store
            .query(&AdminLogsQuery {
                status_class: Some(4),
                limit: Some(10),
                before_id: None,
                from: None,
                to: None,
                level: None,
                request_id: None,
                route: None,
                method: None,
                status: None,
                min_latency_ms: None,
                max_latency_ms: None,
                target: None,
                q: None,
                offset: None,
                sort_by: None,
                sort_dir: None,
            })
            .await
            .unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].status, Some(404));

        let page = store
            .query(&AdminLogsQuery {
                sort_by: Some("latency".into()),
                sort_dir: Some("asc".into()),
                limit: Some(10),
                before_id: None,
                from: None,
                to: None,
                level: None,
                request_id: None,
                route: None,
                method: None,
                status: None,
                status_class: None,
                min_latency_ms: None,
                max_latency_ms: None,
                target: None,
                q: None,
                offset: None,
            })
            .await
            .unwrap();
        assert_eq!(
            page.items
                .iter()
                .map(|item| item.latency_ms)
                .collect::<Vec<_>>(),
            vec![Some(3), Some(12)]
        );

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn opens_sqlite_paths_with_spaces_without_url_encoding() {
        let dir =
            std::env::temp_dir().join(format!("gotiga admin log test {}", uuid::Uuid::new_v4()));
        let path = dir.join("admin logs.sqlite");
        let store = AdminLogStore::open(&path).await.unwrap();
        store
            .query(&AdminLogsQuery {
                limit: Some(1),
                before_id: None,
                from: None,
                to: None,
                level: None,
                request_id: None,
                route: None,
                method: None,
                status: None,
                status_class: None,
                min_latency_ms: None,
                max_latency_ms: None,
                target: None,
                q: None,
                offset: None,
                sort_by: None,
                sort_dir: None,
            })
            .await
            .unwrap();
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
