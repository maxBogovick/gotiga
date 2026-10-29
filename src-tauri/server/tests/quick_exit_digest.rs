//! Недельный разбор быстрых уходов: свёртка по источнику и устройству, находка,
//! отправка владельцу. Запросы в рантайме компилятор не проверяет, поэтому SQL свёртки
//! (в том числе склейка просмотра работы с её «вовлечённым» снимком) пробуется здесь.


use axum::http::{HeaderMap, HeaderValue};
use gotiga_server::analytics::{AnalyticsRequestContext, build_event_record, merge_visit_snapshots};
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::AnalyticsEventRecord;
use gotiga_server::services::AppService;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-signals-uploads-{}", Uuid::new_v4()),
        public_url: "http://127.0.0.1".into(),
        rust_log: String::new(),
        admin_login: "admin".into(),
        admin_password: "test-password-123".into(),
        cors_allowed_origins: vec![],
        telegram_bot_token: None,
        telegram_login_bot_token: None,
        telegram_login_bot_username: None,
        telegram_webhook_secret: None,
        telegram_channel_id: None,
        telegram_channel_extra: vec![],
        telegram_chat_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-signals-logs-{}.sqlite", Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".into(),
        auth_pepper: "pepper-for-tests-0123456789".into(),
        google_client_id: None,
        google_client_secret: None,
        apple_client_id: None,
        apple_team_id: None,
        apple_key_id: None,
        apple_private_key: None,
        auth_pepper_old: None,
    }
}

fn service(pool: &PgPool) -> AppService {
    AppService::new(Repository::new(pool.clone()), config())
}

fn headers(user_agent: &str, ip: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("user-agent", HeaderValue::from_str(user_agent).unwrap());
    h.insert("x-real-ip", HeaderValue::from_str(ip).unwrap());
    h
}

fn record_with(body: Value, ua: &str, ip: &str) -> Result<Option<AnalyticsEventRecord>, String> {
    let req = serde_json::from_value(body).map_err(|e| e.to_string())?;
    let h = headers(ua, ip);
    build_event_record(
        req,
        AnalyticsRequestContext {
            headers: &h,
            admin_api_key: "test-admin-api-key-0123456789",
            hash_secret: "test-analytics-secret-0123456789",
            country_code: None,
            site_host: Some("127.0.0.1".into()),
        },
    )
    .map_err(|e| e.to_string())
}

const CHROME: &str = "Mozilla/5.0 (Macintosh) AppleWebKit/537.36 Chrome/126 Safari/537.36";

fn visit(pv: Uuid, kind: &str, path: &str, extra: Value) -> Value {
    let mut base = json!({"eventType": kind, "path": path, "pageViewId": pv.to_string(), "lang": "ru"});
    for (k, v) in extra.as_object().unwrap() {
        base[k] = v.clone();
    }
    base
}


const IPHONE: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148 Safari/604.1";

/// Визит комнаты: `visits` штук, из них `quick` быстрых (3 с), остальные по минуте.
fn room_visits(
    path: &str,
    ua: &str,
    referrer: Option<&str>,
    visits: usize,
    quick: usize,
) -> Vec<AnalyticsEventRecord> {
    let mut out = Vec::new();
    for i in 0..visits {
        let pv = Uuid::new_v4();
        let (ms, scroll) = if i < quick { (3000, 5) } else { (60_000, 70) };
        let mut body = visit(
            pv,
            "page_engaged",
            path,
            json!({"durationMs": ms, "scrollDepth": scroll, "exitTo": "hide"}),
        );
        if let Some(r) = referrer {
            body["referrer"] = json!(r);
        }
        out.push(record_with(body, ua, "10.1.2.3").unwrap().unwrap());
    }
    out
}

#[derive(sqlx::FromRow, Debug)]
struct Cell {
    path_group: String,
    origin: String,
    device_class: String,
    visits: i32,
    quick: i32,
    quick_unscrolled: i32,
    engaged: i32,
}

async fn cells(pool: &PgPool) -> Vec<Cell> {
    sqlx::query_as::<_, Cell>(
        "SELECT path_group, origin, device_class, visits, quick, quick_unscrolled, engaged
           FROM site_engagement_source_daily ORDER BY path_group, origin, device_class",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn the_rollup_counts_rooms_and_work_pages_by_source_and_device(pool: PgPool) {
    let repo = Repository::new(pool.clone());
    let today = chrono::Utc::now().date_naive();

    let mut events = Vec::new();
    // Небылицы: Pinterest с телефона — пять визитов, четыре быстрых; прямой с компьютера — два долгих.
    events.extend(room_visits("/tales/lantern", IPHONE, Some("https://www.pinterest.com/pin/1"), 5, 4));
    events.extend(room_visits("/tales/lantern", CHROME, None, 2, 0));

    // Работа: свои события. Один просмотр без «вовлечённого» снимка (ушёл раньше восьми
    // секунд), один с сорока секундами, один с четырьмя секундами и прокруткой в полстраницы.
    let work = Uuid::new_v4();
    for snapshot in [None, Some((40_000, 80)), Some((4_000, 55))] {
        let pv = Uuid::new_v4();
        let mut v = visit(pv, "figurine_view", "/figurines/witch", json!({"figurineId": work.to_string(), "detail": "/"}));
        v["referrer"] = json!("https://t.me/somechannel/1");
        events.push(record_with(v, IPHONE, "10.1.2.3").unwrap().unwrap());
        if let Some((ms, scroll)) = snapshot {
            let mut e = visit(
                pv,
                "figurine_engaged",
                "/figurines/witch",
                json!({"figurineId": work.to_string(), "durationMs": ms, "scrollDepth": scroll}),
            );
            e["referrer"] = json!("https://t.me/somechannel/1");
            events.push(record_with(e, IPHONE, "10.1.2.3").unwrap().unwrap());
        }
    }
    repo.bulk_insert_analytics_events(&merge_visit_snapshots(events)).await.expect("события записаны");
    repo.refresh_signal_rollups_only(today, today).await.expect("свёртка");

    let all = cells(&pool).await;
    let tale_pin = all
        .iter()
        .find(|c| c.path_group == "tale" && c.origin == "pinterest.com")
        .unwrap_or_else(|| panic!("нет строки Pinterest: {all:?}"));
    assert_eq!(tale_pin.device_class, "mobile");
    assert_eq!((tale_pin.visits, tale_pin.quick, tale_pin.quick_unscrolled, tale_pin.engaged), (5, 4, 4, 1));

    let tale_direct = all.iter().find(|c| c.path_group == "tale" && c.origin == "direct").unwrap();
    assert_eq!((tale_direct.device_class.as_str(), tale_direct.visits, tale_direct.quick), ("desktop", 2, 0));

    let work_row = all
        .iter()
        .find(|c| c.path_group == "work")
        .unwrap_or_else(|| panic!("страницы работ в свёртке нет: {all:?}"));
    assert_eq!(work_row.visits, 3, "каждый просмотр работы — визит");
    assert_eq!(work_row.quick, 2, "без снимка и с четырьмя секундами — быстрые");
    assert_eq!(work_row.quick_unscrolled, 1, "четыре секунды с прокруткой в полстраницы не «без прокрутки»");
    assert_eq!(work_row.engaged, 1);
    assert_eq!(work_row.origin, "t.me");

    // Повторный пересчёт того же дня ничего не множит.
    repo.refresh_signal_rollups_only(today, today).await.expect("повтор");
    assert_eq!(cells(&pool).await.len(), all.len());
}

/// События, перенесённые на последний день законченной недели: проход разбирает её.
async fn last_week_with_a_bad_source(pool: &PgPool) {
    let repo = Repository::new(pool.clone());
    let (_, end) = gotiga_server::digest::last_full_week(chrono::Utc::now().date_naive());
    let mut events = Vec::new();
    events.extend(room_visits("/tales/lantern", IPHONE, Some("https://www.pinterest.com/pin/1"), 40, 34));
    events.extend(room_visits("/tales/lantern", CHROME, None, 120, 36));
    repo.bulk_insert_analytics_events(&merge_visit_snapshots(events)).await.expect("события");
    sqlx::query("UPDATE figurine_analytics_events SET event_date = $1").bind(end).execute(pool).await.unwrap();
}

#[sqlx::test]
async fn the_weekly_round_finds_the_bad_source_and_keeps_the_digest(pool: PgPool) {
    last_week_with_a_bad_source(&pool).await;
    let svc = service(&pool);

    // Бот не подключён: разбор делается и лежит в админке, письма нет.
    assert_eq!(svc.weekly_digest_round().await.expect("проход"), (true, 0));
    let digests = svc.admin_digests().await.unwrap();
    assert_eq!(digests.len(), 1);
    let d = &digests[0];
    assert_eq!((d.visits, d.quick), (160, 70));
    assert!(d.telegram_at.is_none());
    let f = d.findings.0.first().expect("находка");
    assert_eq!((f.path_group.as_str(), f.origin.as_deref()), ("tale", Some("pinterest.com")));
    assert!(f.rate > 0.8, "{f:?}");

    // Повторный проход разбор не переделывает.
    svc.weekly_digest_round().await.unwrap();
    assert_eq!(svc.admin_digests().await.unwrap().len(), 1);
}

#[sqlx::test]
async fn a_rebuild_replaces_the_digest_and_the_owner_gets_it_once(pool: PgPool) {
    last_week_with_a_bad_source(&pool).await;
    let mut cfg = config();
    // Локальный адрес: сообщение печатается в журнал и отмечается ушедшим, в сеть не идёт.
    cfg.telegram_bot_token = Some("test-token".into());
    cfg.telegram_chat_id = Some("1".into());
    let svc = AppService::new(Repository::new(pool.clone()), cfg);

    svc.weekly_digest_round().await.unwrap();
    let d = svc.admin_digests().await.unwrap();
    assert_eq!(d.len(), 1);
    assert!(d[0].telegram_at.is_some(), "владельцу отправлено (здесь — в журнал)");

    let again = svc.admin_rebuild_digest().await.unwrap();
    assert_eq!(again.len(), 1, "неделя одна, разбор один");
}
