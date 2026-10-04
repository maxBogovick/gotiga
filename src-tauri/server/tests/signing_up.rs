//! Заведение имени и вход знаками — то, из-за чего они устроены так, а не проще.
//!
//! Проверяется не «ручка ответила 200», а пять вещей, каждая из которых была
//! настоящей дырой:
//!
//! * заведение имени отвечает ОДИНАКОВО на свободный и на занятый адрес,
//!   поэтому перечислить жильцов дома по ответу нельзя;
//! * сессия не выдаётся, пока письмо не открыто, — и открывается письмо один
//!   раз;
//! * имя, до которого не дозвонились, принадлежит тому, кто докажет ящик;
//! * ключ сессии в базе не лежит — лежит его отпечаток;
//! * пять ошибок из чужой сети не запирают хозяина.
//!
//! SMTP здесь не настроен намеренно: письма уходят вдогонку (`spawn`) и
//! падают, а проверка как раз в том, что ответ страницы от этого не зависит —
//! иначе по времени ответа было бы видно, какая ветка сработала.

use gotiga_server::api;
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::logs::AdminLogStore;
use gotiga_server::services::AppService;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

const PEPPER: &str = "pepper-for-tests-0123456789";

async fn spawn(pool: PgPool) -> String {
    spawn_with_mail(pool, true).await
}

/// `mail` — есть ли у дома чем отправить письмо. Им и выбирается порядок:
/// строгий (письмо, сессии нет) или открытый (сессия сразу).
async fn spawn_with_mail(pool: PgPool, mail: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let addr = format!("http://127.0.0.1:{port}");

    let config = Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-signup-uploads-{}", Uuid::new_v4()),
        public_url: addr.clone(),
        rust_log: String::new(),
        admin_login: "admin".into(),
        admin_password: "test-password-123".into(),
        cors_allowed_origins: vec![],
        telegram_bot_token: None,
        telegram_chat_id: None,
        telegram_login_bot_token: None,
        telegram_login_bot_username: None,
        telegram_webhook_secret: None,
        telegram_channel_id: None,
        // Отправить всё равно не удастся — сервера нет, — но письма уходят
        // вдогонку, а порядок двери выбирается именно по наличию настроек.
        smtp_host: mail.then(|| "smtp.example.test".to_string()),
        smtp_port: Some(587),
        smtp_user: mail.then(|| "pochta@example.test".to_string()),
        smtp_pass: mail.then(|| "secret".to_string()),
        smtp_from: mail.then(|| "dom@example.test".to_string()),
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-signup-logs-{}.sqlite", Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".into(),
        auth_pepper: PEPPER.into(),
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

fn print_of(token: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Набор из шестнадцати значков на каждую полосу — ровно то, что сервер и
/// ждёт: меньше или больше он не принимает.
const ANIMALS: [&str; 16] = [
    "wolf", "raven", "fox", "owl", "snake", "deer", "bat", "cat", "bear", "hare", "boar", "lynx",
    "crow", "moth", "spider", "frog",
];
const DISHES: [&str; 16] = [
    "mushroom", "apple", "bread", "cup", "fish", "berry", "honey", "herb", "pear", "plum", "egg",
    "cheese", "grapes", "carrot", "onion", "pumpkin",
];
const SEASONS: [&str; 16] = [
    "snowflake", "bare_tree", "sprout", "rain", "sun", "wheat", "leaf", "acorn", "icicle",
    "frost_pane", "bud", "blossom", "cloud", "lightning", "mist", "pinecone",
];
const SYMBOLS: [&str; 16] = [
    "key", "candle", "hourglass", "skull", "moon", "star", "cross", "anchor", "bell", "clock",
    "feather", "inkpot", "scroll", "dagger", "crown", "eye",
];

fn pool() -> Value {
    json!([ANIMALS, DISHES, SEASONS, SYMBOLS])
}

fn signs() -> Value {
    json!([ANIMALS[0], DISHES[1], SEASONS[2], SYMBOLS[3]])
}

/// Без почты дом впускает сразу: подтверждать нечем, и держать человека у
/// двери ради письма, которое не придёт, — это просто закрытый дом.
#[sqlx::test]
async fn with_no_way_to_send_a_letter_the_door_opens_at_once(pool_db: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool_db).await.unwrap();
    let addr = spawn_with_mail(pool_db.clone(), false).await;

    let (status, said) = start_a_name(&addr, "bezpisma@example.test").await;
    assert!(status.is_success(), "открытый порядок не впустил: {said}");
    assert_eq!(said["pending"], false);
    let token = said["sessionToken"].as_str().expect("сессии нет");

    let me = reqwest::Client::new()
        .get(format!("{addr}/api/v1/auth/me"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert!(me.status().is_success(), "сессия ненастоящая");

    // Адрес при этом НЕ подтверждён: письма не было, значит ящик не доказан, и
    // дорога назад по нему не открывается.
    let confirmed: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT email_confirmed_at FROM users WHERE email = 'bezpisma@example.test'",
    )
    .fetch_one(&pool_db)
    .await
    .unwrap();
    assert!(confirmed.is_none(), "адрес подтверждён без письма");

    // Занятый адрес в открытом порядке отвечает отказом — это и есть названная
    // вслух цена: перечисление жильцов сюда возвращается.
    let (again, _) = start_a_name(&addr, "bezpisma@example.test").await;
    assert_eq!(again, 409);
}

async fn start_a_name(addr: &str, email: &str) -> (reqwest::StatusCode, Value) {
    let resp = reqwest::Client::new()
        .post(format!("{addr}/api/v1/auth/register"))
        .json(&json!({
            "email": email,
            "displayName": "Пришедший",
            "selections": signs(),
            "pool": pool(),
            "ageConfirmed": true,
        }))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    (status, resp.json().await.unwrap_or(Value::Null))
}

/// Заведение имени ничего не рассказывает о том, кто в доме уже живёт.
#[sqlx::test]
async fn a_taken_address_and_a_free_one_answer_the_same(pool_db: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool_db).await.unwrap();
    let addr = spawn(pool_db.clone()).await;

    sqlx::query(
        "INSERT INTO users (email, display_name, visual_password_hash, email_confirmed_at)
         VALUES ('zhivet@example.test', 'Живёт', 'hash', NOW())",
    )
    .execute(&pool_db)
    .await
    .unwrap();

    let (free_status, free) = start_a_name(&addr, "svobodnyj@example.test").await;
    let (taken_status, taken) = start_a_name(&addr, "zhivet@example.test").await;

    assert_eq!(free_status, taken_status, "ответы разошлись статусом");
    assert_eq!(free["pending"], true);
    assert_eq!(taken["pending"], true);
    assert_eq!(taken["email"], "zhivet@example.test");
    // Главное: ни в одном из двух ответов нет ключа. Пока он был, свободный
    // адрес отвечал сессией, а занятый — отказом, и это и было перечисление.
    assert!(free["sessionToken"].is_null(), "имя впустило до письма");
    assert!(taken["sessionToken"].is_null());

    // Чужое имя не тронуто: ни знаки, ни отметка об открытом ящике.
    let (hash, confirmed): (Option<String>, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT visual_password_hash, email_confirmed_at FROM users WHERE email = 'zhivet@example.test'")
            .fetch_one(&pool_db)
            .await
            .unwrap();
    assert_eq!(hash.as_deref(), Some("hash"), "чужие знаки переписаны");
    assert!(confirmed.is_some());
}

/// Письмо открывает дверь — и открывает её один раз.
#[sqlx::test]
async fn the_letter_opens_the_door_once(pool_db: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool_db).await.unwrap();
    let addr = spawn(pool_db.clone()).await;

    let (_, said) = start_a_name(&addr, "pervyj@example.test").await;
    assert_eq!(said["pending"], true);

    // Письма не видно (SMTP не настроен), поэтому ключ берётся так же, как его
    // взял бы человек из ящика: строка в базе хранит ОТПЕЧАТОК, значит ключ
    // подставляется свой, а отпечаток кладётся под него.
    let key = Uuid::new_v4().to_string();
    sqlx::query("UPDATE users SET email_confirm_hash = $1 WHERE email = 'pervyj@example.test'")
        .bind(print_of(&key))
        .execute(&pool_db)
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let opened: Value = client
        .post(format!("{addr}/api/v1/auth/confirm"))
        .json(&json!({ "token": key }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = opened["sessionToken"].as_str().expect("сессия не выдана");
    assert_eq!(opened["user"]["emailConfirmed"], true);

    // Сессия настоящая.
    let me = client
        .get(format!("{addr}/api/v1/auth/me"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert!(me.status().is_success());

    // Ключ сессии в базе не лежит — лежит его отпечаток.
    let stored: Option<String> =
        sqlx::query_scalar("SELECT token FROM user_sessions WHERE token = $1")
            .bind(token)
            .fetch_optional(&pool_db)
            .await
            .unwrap();
    assert!(stored.is_none(), "ключ сессии лежит в базе открытым");
    let by_print: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_sessions WHERE token = $1")
            .bind(print_of(token))
            .fetch_one(&pool_db)
            .await
            .unwrap();
    assert_eq!(by_print, 1);

    // Второй переход по той же ссылке не открывает ничего.
    let again = client
        .post(format!("{addr}/api/v1/auth/confirm"))
        .json(&json!({ "token": key }))
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), 400, "письмо сработало дважды");
}

/// Имя, до которого не дозвонились, принадлежит тому, кто докажет ящик.
///
/// Без этого правила занять чужой адрес можно было бы, просто набрав его:
/// настоящий хозяин упёрся бы в уникальный индекс навсегда.
#[sqlx::test]
async fn an_unopened_name_belongs_to_whoever_proves_the_mailbox(pool_db: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool_db).await.unwrap();
    let addr = spawn(pool_db.clone()).await;

    start_a_name(&addr, "spornyj@example.test").await;
    let first: Option<String> =
        sqlx::query_scalar("SELECT email_confirm_hash FROM users WHERE email = 'spornyj@example.test'")
            .fetch_one(&pool_db)
            .await
            .unwrap();

    start_a_name(&addr, "spornyj@example.test").await;
    let (second, count): (Option<String>, i64) = (
        sqlx::query_scalar("SELECT email_confirm_hash FROM users WHERE email = 'spornyj@example.test'")
            .fetch_one(&pool_db)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = 'spornyj@example.test'")
            .fetch_one(&pool_db)
            .await
            .unwrap(),
    );

    assert_eq!(count, 1, "второе заведение развело два имени на один адрес");
    assert_ne!(first, second, "прежняя ссылка осталась живой");
    assert!(second.is_some());
}

/// Знаки, записанные прежним способом, открывают дверь — и дверь переписывает
/// их на нынешний лад: с перцем и с набором из шестнадцати.
#[sqlx::test]
async fn old_signs_open_the_door_and_are_rewritten(pool_db: PgPool) {
    use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
    use argon2::Argon2;

    sqlx::migrate!("./migrations/").run(&pool_db).await.unwrap();
    let addr = spawn(pool_db.clone()).await;

    // Прежняя запись: Argon2 без перца, и набор из восьми значков на полосу.
    let salt = SaltString::generate(&mut OsRng);
    let legacy = Argon2::default()
        .hash_password(
            b"animals:wolf|dishes:apple|seasons:leaf|symbols:skull",
            &salt,
        )
        .unwrap()
        .to_string();
    let small = json!({
        "animals": &ANIMALS[..8],
        "dishes": &DISHES[..8],
        "seasons": &SEASONS[..8],
        "symbols": &SYMBOLS[..8],
    });
    sqlx::query(
        "INSERT INTO users (email, display_name, visual_password_hash, visual_pool, email_confirmed_at)
         VALUES ('staraya@example.test', 'Старая запись', $1, $2, NOW())",
    )
    .bind(&legacy)
    .bind(&small)
    .execute(&pool_db)
    .await
    .unwrap();

    let client = reqwest::Client::new();
    let challenge: Value = client
        .post(format!("{addr}/api/v1/auth/login/challenge"))
        .json(&json!({ "email": "staraya@example.test" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Набор ей показан её собственный — тот, что лежит в базе, из восьми.
    let steps = challenge["steps"].as_array().unwrap();
    assert_eq!(steps[0]["icons"].as_array().unwrap().len(), 8);

    let want = ["wolf", "apple", "leaf", "skull"];
    let tokens: Vec<String> = steps
        .iter()
        .zip(want)
        .map(|(step, id)| {
            step["icons"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["iconId"] == id)
                .unwrap_or_else(|| panic!("{id} не показан"))["token"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();

    let entered: Value = client
        .post(format!("{addr}/api/v1/auth/login/verify"))
        .json(&json!({ "challengeId": challenge["challengeId"], "tokens": tokens }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        entered["sessionToken"].is_string(),
        "прежние знаки перестали открывать: {entered}"
    );

    // Запись переписана: хеш другой, а набор дорос до нынешнего размера — и
    // выбранный знак в нём остался.
    let (hash, pool_now): (String, Value) = sqlx::query_as(
        "SELECT visual_password_hash, visual_pool FROM users WHERE email = 'staraya@example.test'",
    )
    .fetch_one(&pool_db)
    .await
    .unwrap();
    assert_ne!(hash, legacy, "знаки остались записанными по-старому");
    let animals = pool_now["animals"].as_array().unwrap();
    assert_eq!(animals.len(), 16, "набор не дорос");
    assert!(animals.iter().any(|v| v == "wolf"), "выбор выпал из набора");
}

/// Пять ошибок из чужой сети не запирают хозяина.
///
/// Прежде счёт шёл по одной почте, и запереть человека на четверть часа мог
/// кто угодно, бесплатно и сколько угодно раз.
#[sqlx::test]
async fn a_stranger_guessing_does_not_lock_the_owner_out(pool_db: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool_db).await.unwrap();
    let addr = spawn(pool_db.clone()).await;

    // Шесть неудач с одного чужого адреса — больше, чем терпит пара.
    for _ in 0..6 {
        sqlx::query(
            "INSERT INTO login_attempts (email, success, ip)
             VALUES ('hozyain@example.test', false, '203.0.113.9')",
        )
        .execute(&pool_db)
        .await
        .unwrap();
    }

    let client = reqwest::Client::new();
    let ask = |ip: &'static str| {
        let client = client.clone();
        let addr = addr.clone();
        async move {
            client
                .post(format!("{addr}/api/v1/auth/login/challenge"))
                .header("x-forwarded-for", ip)
                .json(&json!({ "email": "hozyain@example.test" }))
                .send()
                .await
                .unwrap()
                .status()
        }
    };

    assert_eq!(ask("203.0.113.9").await, 400, "подбирающий не остановлен");
    assert!(
        ask("198.51.100.4").await.is_success(),
        "хозяина заперли чужими ошибками"
    );
}
