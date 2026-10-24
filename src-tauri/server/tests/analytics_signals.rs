//! Замер: время визита, куда ушли, поступки, сбои, скорость.
//!
//! Проверяется то, что нельзя увидеть глазами в коде: что повторные отправки
//! времени сливаются в одну строку визита (в одной пачке и в разных), что
//! суточные свёртки считают именно то, что должны, и что страница отчёта
//! получает числа, а не ошибку SQL — запросы в рантайме не проверяются
//! компилятором.

use axum::http::{HeaderMap, HeaderValue};
use gotiga_server::analytics::{
    AnalyticsRequestContext, MAX_ACTIVE_MS, build_event_record, merge_visit_snapshots,
};
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::{AdminAnalyticsQuery, AnalyticsEventRecord};
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

fn record(body: Value) -> AnalyticsEventRecord {
    record_with(body, CHROME, "10.1.2.3")
        .expect("событие принято")
        .expect("не бот")
}

fn visit(pv: Uuid, kind: &str, path: &str, extra: Value) -> Value {
    let mut base = json!({"eventType": kind, "path": path, "pageViewId": pv.to_string(), "lang": "ru"});
    for (k, v) in extra.as_object().unwrap() {
        base[k] = v.clone();
    }
    base
}

#[sqlx::test]
async fn rolling_snapshots_of_one_visit_collapse_into_one_row(pool: PgPool) {
    let repo = Repository::new(pool.clone());
    let pv = Uuid::new_v4();

    // Две отправки одного визита в одной пачке и одна — в следующей.
    let first = vec![
        record(visit(pv, "page_engaged", "/author", json!({"durationMs": 10000, "scrollDepth": 20, "exitTo": "hide"}))),
        record(visit(pv, "page_engaged", "/author", json!({"durationMs": 40000, "scrollDepth": 60, "exitTo": "hide", "lastSection": "Мастерская"}))),
    ];
    let merged = merge_visit_snapshots(first);
    assert_eq!(merged.len(), 1, "в пачке повтор визита сливается до записи");
    repo.bulk_insert_analytics_events(&merged).await.expect("первая пачка");

    let later = vec![record(visit(
        pv,
        "page_engaged",
        "/author",
        json!({"durationMs": 30000, "scrollDepth": 90, "exitTo": "/figurines/x?utm=1"}),
    ))];
    repo.bulk_insert_analytics_events(&merge_visit_snapshots(later)).await.expect("вторая пачка");

    let rows: Vec<(i32, i32, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT duration_ms, scroll_depth, exit_to, last_section
           FROM figurine_analytics_events WHERE page_view_id = $1",
    )
    .bind(pv)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1, "один визит — одна строка");
    let (duration, scroll, exit, section) = &rows[0];
    assert_eq!(*duration, 40000, "время остаётся наибольшим, а не последним");
    assert_eq!(*scroll, 90);
    assert_eq!(exit.as_deref(), Some("/figurines/x"), "последнее слово о выходе — за поздним, без строки запроса");
    assert_eq!(section.as_deref(), Some("Мастерская"), "раздел, названный раньше, не стирается пустым");
}

#[sqlx::test]
async fn a_visit_longer_than_the_ceiling_is_clamped(_pool: PgPool) {
    let r = record(visit(Uuid::new_v4(), "page_engaged", "/", json!({"durationMs": 900_000_000})));
    assert_eq!(r.duration_ms, Some(MAX_ACTIVE_MS));
}

#[sqlx::test]
async fn bad_signals_are_refused_or_scrubbed(_pool: PgPool) {
    let bad = |body: Value| record_with(body, CHROME, "10.1.2.3").is_err();
    assert!(bad(json!({"eventType": "interaction", "path": "/"})), "поступок без имени");
    assert!(bad(json!({"eventType": "interaction", "path": "/", "name": "Bad Name"})));
    assert!(bad(json!({"eventType": "client_error", "path": "/", "name": "weird", "detail": "x"})), "вид сбоя из закрытого списка");
    assert!(bad(json!({"eventType": "client_error", "path": "/", "name": "js"})), "сбой без текста");
    assert!(bad(json!({"eventType": "web_vitals", "path": "/"})), "скорость без единой меры");
    assert!(bad(json!({"eventType": "web_vitals", "path": "/", "vitalLcp": -5})));
    assert!(bad(json!({"eventType": "figurine_view", "path": "/figurines/x"})), "работа по-прежнему нужна своим событиям");

    let r = record(json!({
        "eventType": "interaction", "path": "/figurines", "name": "search",
        "detail": "лантерна me@example.com https://x.test/a?token=SECRET", "value": 0
    }));
    let detail = r.signals.detail.expect("запрос сохранён");
    assert!(!detail.contains("@"), "адрес почты не сохраняется: {detail}");
    assert!(!detail.contains("SECRET"), "ключ из ссылки не сохраняется: {detail}");
    assert!(detail.contains("лантерна"));

    let r = record(json!({"eventType": "page_engaged", "path": "/", "exitTo": "javascript:alert(1)"}));
    assert_eq!(r.signals.exit_to, None, "куда ушли — только путь, чужой хост или hide");
    let r = record(json!({"eventType": "page_engaged", "path": "/", "exitTo": "ext:T.me"}));
    assert_eq!(r.signals.exit_to.as_deref(), Some("ext:t.me"));
}

#[sqlx::test]
async fn robots_are_not_counted(_pool: PgPool) {
    let robot = record_with(
        json!({"eventType": "page_view", "path": "/"}),
        "python-requests/2.31",
        "10.9.9.9",
    );
    assert!(matches!(robot, Ok(None)), "скрипт отброшен как бот");
    let headless = record_with(
        json!({"eventType": "page_view", "path": "/"}),
        "Mozilla/5.0 HeadlessChrome/126",
        "10.9.9.9",
    );
    assert!(matches!(headless, Ok(None)));
}

#[sqlx::test]
async fn rollups_and_reports_agree_with_the_events(pool: PgPool) {
    let repo = Repository::new(pool.clone());
    let svc = service(&pool);
    // Служба запускает фоновый пересчёт сразу; ждём, пока он отработает на
    // пустой базе, иначе он столкнулся бы с нашим ручным пересчётом.
    svc.refresh_analytics_hot_window_if_due().await.expect("первый пересчёт");
    let today = chrono::Utc::now().date_naive();

    let mut events = Vec::new();
    // Три визита на «Об авторе»: быстрый уход, средний, долгий; один вернувшийся.
    for (ms, scroll, returning, exit) in [
        (4000, 5, false, "/figurines"),
        (45000, 55, true, "ext:t.me"),
        (200_000, 95, true, "hide"),
    ] {
        let pv = Uuid::new_v4();
        events.push(record(visit(pv, "page_view", "/author", json!({"detail": "/"}))));
        events.push(record(visit(
            pv,
            "page_engaged",
            "/author",
            json!({"durationMs": ms, "scrollDepth": scroll, "isReturning": returning,
                   "exitTo": exit, "lastSection": "Об авторе"}),
        )));
    }

    // Поиск: два с находкой, один пустой.
    for (q, found) in [("лантерна", 3), ("лантерна", 3), ("русалка", 0)] {
        events.push(record(visit(
            Uuid::new_v4(),
            "interaction",
            "/figurines",
            json!({"name": "search", "detail": q, "value": found}),
        )));
    }

    // Сбои: два одинаковых скриптовых, мёртвая ссылка.
    for _ in 0..2 {
        events.push(record(visit(
            Uuid::new_v4(),
            "client_error",
            "/figurines",
            json!({"name": "js", "detail": "TypeError: x is undefined"}),
        )));
    }
    events.push(record(visit(
        Uuid::new_v4(),
        "not_found",
        "/figurines/old-name",
        json!({"referrer": "https://t.me/somechannel/12"}),
    )));

    // Скорость: LCP 1800 (хорошо) и 4200 (плохо) на «Об авторе».
    for lcp in [1800, 4200] {
        events.push(record(visit(
            Uuid::new_v4(),
            "web_vitals",
            "/author",
            json!({"vitalLcp": lcp, "vitalInp": 120, "vitalCls": 30, "vitalTtfb": 200, "conn": "4g", "viewportW": 390}),
        )));
    }

    // Байка: три визита, у двух — до конца; арм опыта разный.
    for (finished, arm) in [(true, "scene:on"), (true, "scene:on"), (false, "scene:off")] {
        let pv = Uuid::new_v4();
        events.push(record(visit(pv, "page_view", "/tales/lantern", json!({"variant": arm}))));
        events.push(record(visit(
            pv,
            "page_engaged",
            "/tales/lantern",
            json!({"durationMs": if finished { 120000 } else { 6000 }, "scrollDepth": 80, "variant": arm}),
        )));
        if finished {
            events.push(record(visit(
                pv,
                "interaction",
                "/tales/lantern",
                json!({"name": "tale_finished", "variant": arm}),
            )));
        }
    }

    repo.bulk_insert_analytics_events(&merge_visit_snapshots(events))
        .await
        .expect("события записаны");
    repo.refresh_analytics_aggregates(today - chrono::Duration::days(1), today)
        .await
        .expect("свёртки пересчитаны");

    let none = || AdminAnalyticsQuery { from: None, to: None, sort: None, dir: None };

    let b = svc.admin_behaviour(none()).await.expect("отчёт о поведении");

    let author = b.shape.iter().find(|r| r.path_group == "author").expect("комната «автор»");
    assert_eq!(author.visits, 3);
    assert_eq!(author.dur_lt10, 1, "один ушёл быстрее десяти секунд");
    assert_eq!(author.dur_30_60, 1);
    assert_eq!(author.dur_ge180, 1);
    assert_eq!(author.scroll_lt25, 1);
    assert_eq!(author.scroll_ge75, 1);
    assert_eq!(author.new_visits, 1);
    assert_eq!(author.returning_visits, 2);
    assert_eq!(author.returning_long, 2, "оба вернувшихся пробыли дольше тридцати секунд");
    assert_eq!(author.new_long, 0);

    let exits: Vec<_> = b.exits.iter().filter(|e| e.path_group == "author").collect();
    assert!(exits.iter().any(|e| e.exit_to == "to:archive" && e.visits == 1), "{exits:?}");
    assert!(exits.iter().any(|e| e.exit_to == "ext:t.me" && e.visits == 1));
    assert!(exits.iter().any(|e| e.exit_to == "hide" && e.visits == 1));

    let sec = b.sections.iter().find(|s| s.path_group == "author").expect("последний раздел");
    assert_eq!((sec.section.as_str(), sec.visits), ("Об авторе", 3));

    let search: Vec<_> = b.interactions.iter().filter(|i| i.name == "search").collect();
    let lantern = search.iter().find(|i| i.detail == "лантерна").unwrap();
    assert_eq!(lantern.events, 2);
    assert_eq!(lantern.zero_events, 0);
    let mermaid = search.iter().find(|i| i.detail == "русалка").unwrap();
    assert_eq!(mermaid.zero_events, 1, "пустая выдача видна отдельно");

    let on: Vec<_> = b.tales.iter().filter(|t| t.slug == "lantern" && t.variant == "scene:on").collect();
    let off: Vec<_> = b.tales.iter().filter(|t| t.slug == "lantern" && t.variant == "scene:off").collect();
    assert_eq!((on[0].opened, on[0].finished), (2, 2));
    assert_eq!((off[0].opened, off[0].finished, off[0].quick_exits), (1, 0, 1));

    let h = svc.admin_health(none()).await.expect("отчёт о здоровье");
    let js = h.errors.iter().find(|e| e.kind == "js").expect("сбой скрипта");
    assert_eq!(js.events, 2);
    let dead = h.errors.iter().find(|e| e.kind == "not_found").expect("мёртвая ссылка");
    assert_eq!(dead.detail, "/figurines/old-name");
    assert!(
        h.dead_links.iter().any(|d| d.path == "/figurines/old-name" && d.came_from == "t.me"),
        "откуда пришли на мёртвый адрес: {:?}",
        h.dead_links
    );
    let v = h.vitals.iter().find(|v| v.path_group == "author").expect("скорость");
    assert_eq!(v.samples, 2);
    assert_eq!(v.lcp_good, 1, "один из двух уложился в 2,5 с");
    assert_eq!(v.inp_good, 2);
    assert_eq!(v.cls_good, 2);
    assert!(h.trust.page_views >= 6);
    assert_eq!(h.trust.web_vitals, 2);

    // Второй пересчёт даёт то же, а не удвоение.
    repo.refresh_analytics_aggregates(today - chrono::Duration::days(1), today)
        .await
        .unwrap();
    let again = svc.admin_behaviour(none()).await.unwrap();
    assert_eq!(again.shape.iter().find(|r| r.path_group == "author").unwrap().visits, 3);

    // Отчёт по битвам читается и без единой партии.
    let battles = svc.admin_battle_report(none()).await.expect("отчёт о битвах");
    assert!(battles.challenges.is_empty());
}

#[sqlx::test]
async fn automatic_chart_notes_do_not_stack(pool: PgPool) {
    let svc = service(&pool);
    svc.note_on_chart("Tale letters: Lantern").await;
    svc.note_on_chart("Tale letters: Lantern").await;
    svc.note_on_chart("Channel post: Lantern").await;
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM analytics_annotations WHERE auto")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 2, "повтор в тот же день не множит заметки");
}
