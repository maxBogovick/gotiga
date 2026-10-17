//! Книга дома в Telegram: подписка кнопкой «Запустить», записки о новых
//! небылицах, выход командой `/stop`.
//!
//! Подтверждение через Telegram доказывает Telegram, а не почту, поэтому это
//! отдельная подписка, а не способ подтвердить адрес. Бот здесь настоящий с
//! поддельным токеном: исходящие вызовы возвращают 401, и проверяется, что
//! подписка устроена правильно и без них.

use gotiga_server::api;
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::logs::AdminLogStore;
use gotiga_server::services::AppService;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

const SECRET: &str = "test-webhook-secret-0123456789";

async fn spawn(pool: PgPool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let addr = format!("http://127.0.0.1:{port}");

    let config = Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-tg-uploads-{}", Uuid::new_v4()),
        public_url: addr.clone(),
        rust_log: String::new(),
        admin_login: "admin".into(),
        admin_password: "test-password-123".into(),
        cors_allowed_origins: vec![],
        telegram_bot_token: None,
        telegram_chat_id: None,
        telegram_login_bot_token: Some("123456:poddelnyj-token".into()),
        telegram_login_bot_username: Some("gotiga_test_bot".into()),
        telegram_webhook_secret: Some(SECRET.into()),
        telegram_channel_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-tg-logs-{}.sqlite", Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".into(),
        auth_pepper: "pepper-for-tests-0123456789".into(),
        google_client_id: None,
        google_client_secret: None,
        apple_client_id: None,
        apple_team_id: None,
        apple_key_id: None,
        apple_private_key: None,
        auth_pepper_old: None,
    };

    let service = AppService::new(Repository::new(pool), config.clone());
    let log_store = AdminLogStore::open(&config.admin_log_db_path).await.unwrap();
    let router = api::router(service, config.clone(), log_store);
    tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    addr
}


fn person(id: i64, username: &str) -> Value {
    json!({ "id": id, "is_bot": false, "username": username, "first_name": "Читатель", "language_code": "ru" })
}

fn say(id: i64, username: &str, text: &str) -> Value {
    json!({
        "update_id": 1,
        "message": {
            "message_id": 10, "date": 0,
            "chat": { "id": id, "type": "private" },
            "from": person(id, username),
            "text": text,
        }
    })
}

async fn webhook(addr: &str, update: Value) {
    let r = reqwest::Client::new()
        .post(format!("{addr}/api/v1/telegram/webhook/{SECRET}"))
        .header("x-telegram-bot-api-secret-token", SECRET)
        .json(&update)
        .send().await.unwrap();
    assert!(r.status().is_success(), "{}", r.status());
}

async fn ask(addr: &str, session: Option<&str>) -> reqwest::Response {
    let mut r = reqwest::Client::new()
        .post(format!("{addr}/api/v1/subscribe/telegram"))
        .json(&json!({ "lang": "ru", "source": "home", "ageConfirmed": true }));
    if let Some(t) = session { r = r.bearer_auth(t); }
    r.send().await.unwrap()
}

fn code_of(link: &str) -> String {
    link.split("?start=").nth(1).expect("ссылка с кодом").to_string()
}

async fn seed_tale(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO gazette_leaves (slug, kind, status, title_en, title_ru, published_at)
         VALUES ($1, 'tale', 'published', 'Tale', 'Байка', NOW() - INTERVAL '1 hour') RETURNING id",
    ).bind(slug).fetch_one(pool).await.unwrap()
}

#[sqlx::test]
async fn pressing_start_makes_a_subscription_and_stop_ends_it(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;
    let repo = Repository::new(pool.clone());

    // Без возраста подписки нет, как и на любой форме с контактами.
    let r = reqwest::Client::new().post(format!("{addr}/api/v1/subscribe/telegram"))
        .json(&json!({ "ageConfirmed": false })).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 400);

    // Гость получает ссылку на бота с кодом, и пока он не нажал «Запустить»,
    // подписчика нет.
    let r: Value = ask(&addr, None).await.json().await.unwrap();
    assert_eq!(r["confirmed"], false);
    let link = r["link"].as_str().unwrap().to_string();
    assert!(link.starts_with("https://t.me/gotiga_test_bot?start=sub_"), "{link}");
    let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telegram_subscribers WHERE confirmed_at IS NOT NULL")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(active, 0);

    // Чужой и выдуманный код ничего не подписывает.
    webhook(&addr, say(555, "reader", "/start sub_0000000000000000000000000000dead")).await;
    let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telegram_subscribers WHERE confirmed_at IS NOT NULL")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(active, 0);

    // «Запустить» с настоящим кодом: чат берётся из сообщения Telegram.
    webhook(&addr, say(555, "reader", &format!("/start {}", code_of(&link)))).await;
    let row: (Option<i64>, Option<String>, bool) = sqlx::query_as(
        "SELECT chat_id, username, confirm_hash IS NULL FROM telegram_subscribers")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(row, (Some(555), Some("reader".into()), true));

    // Тот же код второй раз — ничего нового; и тот же чат вторым не заводится.
    webhook(&addr, say(555, "reader", &format!("/start {}", code_of(&link)))).await;
    let r2: Value = ask(&addr, None).await.json().await.unwrap();
    webhook(&addr, say(555, "reader", &format!("/start {}", code_of(r2["link"].as_str().unwrap())))).await;
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telegram_subscribers").fetch_one(&pool).await.unwrap();
    assert_eq!(rows, 1, "тот же чат записан дважды");

    // Новая небылица: записка кладётся в очередь подписчика, не в почтовую.
    let tale = seed_tale(&pool, "t-one").await;
    assert_eq!(repo.lay_out_tale_letters(tale).await.unwrap(), 1);
    let queued: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tale_book_notes WHERE tale_id = $1")
        .bind(tale).fetch_one(&pool).await.unwrap();
    let mails: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tale_letters WHERE tale_id = $1")
        .bind(tale).fetch_one(&pool).await.unwrap();
    assert_eq!((queued, mails), (1, 0));

    // /stop снимает подписку: следующая небылица ему не пишется.
    webhook(&addr, say(555, "reader", "/stop")).await;
    let tale2 = seed_tale(&pool, "t-two").await;
    assert_eq!(repo.lay_out_tale_letters(tale2).await.unwrap(), 0);
}

#[sqlx::test]
async fn the_unsubscribe_door_opens_for_a_telegram_subscriber(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;
    let r: Value = ask(&addr, None).await.json().await.unwrap();
    let token = r["unsubscribeToken"].as_str().unwrap().to_string();
    webhook(&addr, say(777, "leaver", &format!("/start {}", code_of(r["link"].as_str().unwrap())))).await;

    let info: Value = reqwest::get(format!("{addr}/api/v1/subscribe/leave/{token}"))
        .await.unwrap().json().await.unwrap();
    assert_eq!(info["email"], "Telegram @leaver");
    let out = reqwest::Client::new().post(format!("{addr}/api/v1/subscribe/leave/{token}")).send().await.unwrap();
    assert!(out.status().is_success());
    let left: bool = sqlx::query_scalar("SELECT unsubscribed_at IS NOT NULL FROM telegram_subscribers")
        .fetch_one(&pool).await.unwrap();
    assert!(left);
}

#[sqlx::test]
async fn a_telegram_login_that_the_bot_cannot_write_to_falls_back_to_the_link(pool: PgPool) {
    use sha2::{Digest, Sha256};
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;
    let user: Uuid = sqlx::query_scalar(
        "INSERT INTO users (display_name, telegram_id, telegram_username) VALUES ('Т', 888, 'tg') RETURNING id")
        .fetch_one(&pool).await.unwrap();
    let token = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO user_sessions (user_id, token, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 day')")
        .bind(user).bind(hex::encode(Sha256::digest(token.as_bytes()))).execute(&pool).await.unwrap();

    // Бот (здесь с поддельным токеном) написать не смог: подписка без канала —
    // не подписка, человеку даётся ссылка нажать «Запустить» самому.
    let r: Value = ask(&addr, Some(&token)).await.json().await.unwrap();
    assert_eq!(r["confirmed"], false);
    assert!(r["link"].as_str().is_some());
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM telegram_subscribers WHERE confirmed_at IS NOT NULL AND unsubscribed_at IS NULL")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(active, 0);
}
