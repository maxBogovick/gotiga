//! Вход через Telegram: слово, бот, сессия.
//!
//! Проверяется не «ручка ответила 200», а то, из-за чего вся затея и устроена
//! так, а не проще:
//!
//! * вернувшегося узнают по ЧИСЛУ, и имя в профиле при этом не переписывается;
//! * сессия отдаётся странице ровно один раз;
//! * нажать «это я» может только тот Telegram, которому записку показали;
//! * webhook без обоих доказательств секрета — это страница, которой нет;
//! * заблокированному вторая дверь закрыта так же, как первая.
//!
//! Бот здесь настоящий, а токен у него поддельный: исходящие вызовы уходят на
//! `api.telegram.org` и возвращают 401. Это намеренно — все они помечены как
//! необязательные (`let _ = …`), и проверка как раз в том, что вход доходит до
//! конца, когда бот ответить не смог.

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

/// Отпечаток ключа — то, что лежит в базе. Сам ключ уезжает в браузер, и
/// подложить его в `user_sessions` напрямую больше нельзя.
fn print_of(token: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Начать вход. `as_user` — сессия, и тогда это привязка из профиля.
async fn ask_for_word(addr: &str, as_user: Option<&str>) -> Value {
    let client = reqwest::Client::new();
    let mut req = client
        .post(format!("{addr}/api/v1/auth/telegram/code"))
        .header("user-agent", "Mozilla/5.0 (Macintosh) Chrome/131.0");
    if let Some(token) = as_user {
        req = req.header("authorization", format!("Bearer {token}"));
    }
    let resp = req.send().await.unwrap();
    assert!(resp.status().is_success(), "слово не выдано: {}", resp.status());
    resp.json().await.unwrap()
}

async fn poll(addr: &str, code: &str) -> Value {
    reqwest::get(format!("{addr}/api/v1/auth/telegram/code/{code}"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

/// Обновление от Telegram — с обоими доказательствами секрета.
async fn webhook(addr: &str, update: Value) -> reqwest::StatusCode {
    webhook_with(addr, SECRET, Some(SECRET), update).await
}

async fn webhook_with(
    addr: &str,
    path_secret: &str,
    header_secret: Option<&str>,
    update: Value,
) -> reqwest::StatusCode {
    let client = reqwest::Client::new();
    let mut req = client
        .post(format!("{addr}/api/v1/telegram/webhook/{path_secret}"))
        .json(&update);
    if let Some(h) = header_secret {
        req = req.header("x-telegram-bot-api-secret-token", h);
    }
    req.send().await.unwrap().status()
}

fn person(id: i64, username: &str, first_name: &str) -> Value {
    json!({
        "id": id,
        "is_bot": false,
        "username": username,
        "first_name": first_name,
        "language_code": "ru",
    })
}

fn start(id: i64, username: &str, first_name: &str, code: &str) -> Value {
    json!({
        "update_id": 1,
        "message": {
            "message_id": 10,
            "date": 0,
            "chat": { "id": id, "type": "private" },
            "from": person(id, username, first_name),
            "text": format!("/start {code}"),
        }
    })
}

fn press(id: i64, username: &str, first_name: &str, data: &str) -> Value {
    json!({
        "update_id": 2,
        "callback_query": {
            "id": "cb-1",
            "from": person(id, username, first_name),
            "data": data,
            "message": {
                "message_id": 11,
                "date": 0,
                "chat": { "id": id, "type": "private" },
                "text": "записка",
            }
        }
    })
}

/// Полный путь: слово → бот → кнопка → сессия, и человек внутри.
#[sqlx::test]
async fn a_stranger_presses_start_and_is_inside(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    let word = ask_for_word(&addr, None).await;
    let code = word["code"].as_str().unwrap().to_string();
    // Слово сверяют взглядом: оно короткое и не пустое, а ссылка ведёт к боту.
    assert!(word["word"].as_str().unwrap().len() >= 5);
    assert!(word["link"]
        .as_str()
        .unwrap()
        .starts_with("https://t.me/gotiga_test_bot?start="));
    assert_eq!(poll(&addr, &code).await["state"], "waiting");

    assert_eq!(webhook(&addr, start(777, "masha", "Маша", &code)).await, 200);
    assert_eq!(poll(&addr, &code).await["state"], "asked");

    assert_eq!(
        webhook(&addr, press(777, "masha", "Маша", &format!("ok:{code}"))).await,
        200
    );

    let ready = poll(&addr, &code).await;
    assert_eq!(ready["state"], "ready", "слово не отдало сессию: {ready}");
    let token = ready["sessionToken"].as_str().unwrap().to_string();

    // Имя взято из Telegram, почты нет вовсе — её спросит первое дело.
    assert_eq!(ready["user"]["displayName"], "Маша");
    assert!(ready["user"]["email"].is_null(), "почта взялась ниоткуда");
    assert_eq!(ready["user"]["telegramUsername"], "masha");
    assert_eq!(ready["user"]["telegramLinked"], true);

    // Сессия настоящая.
    let me: Value = reqwest::Client::new()
        .get(format!("{addr}/api/v1/auth/me"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["id"], ready["user"]["id"]);

    // Сессия отдаётся ОДИН раз: два опроса не унесут её дважды.
    assert_eq!(poll(&addr, &code).await["state"], "expired");
}

/// Вернувшегося узнают по числу. `@имя` освежается, имя в профиле — нет.
#[sqlx::test]
async fn a_returning_visitor_is_known_by_the_number_not_the_name(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    let first = ask_for_word(&addr, None).await;
    let code = first["code"].as_str().unwrap().to_string();
    webhook(&addr, start(500, "masha", "Маша", &code)).await;
    webhook(&addr, press(500, "masha", "Маша", &format!("ok:{code}"))).await;
    let one = poll(&addr, &code).await;
    let user_id = one["user"]["id"].as_str().unwrap().to_string();

    // Человек переименовался в Telegram и взял другое `@имя`. Число то же.
    let second = ask_for_word(&addr, None).await;
    let code2 = second["code"].as_str().unwrap().to_string();
    webhook(&addr, start(500, "marusya", "Маруся", &code2)).await;
    webhook(&addr, press(500, "marusya", "Маруся", &format!("ok:{code2}"))).await;
    let two = poll(&addr, &code2).await;

    assert_eq!(two["user"]["id"], user_id, "завёлся второй архив");
    assert_eq!(two["user"]["telegramUsername"], "marusya", "`@имя` не освежилось");
    assert_eq!(
        two["user"]["displayName"], "Маша",
        "имя в профиле переписано вторым входом"
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE telegram_id = 500")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

/// Кнопку нажимает только тот, кому записку показали. Её можно переслать.
#[sqlx::test]
async fn someone_elses_finger_does_not_open_the_door(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    let word = ask_for_word(&addr, None).await;
    let code = word["code"].as_str().unwrap().to_string();
    webhook(&addr, start(111, "masha", "Маша", &code)).await;

    // Записку переслали, и нажал другой человек.
    webhook(&addr, press(222, "kolya", "Коля", &format!("ok:{code}"))).await;

    assert_eq!(poll(&addr, &code).await["state"], "asked", "чужое нажатие впустило");
    let made: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
    assert_eq!(made, 0, "чужое нажатие завело аккаунт");
}

/// «Не я» гасит слово немедленно.
#[sqlx::test]
async fn not_me_puts_the_word_out(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    let word = ask_for_word(&addr, None).await;
    let code = word["code"].as_str().unwrap().to_string();
    webhook(&addr, start(333, "masha", "Маша", &code)).await;
    webhook(&addr, press(333, "masha", "Маша", &format!("no:{code}"))).await;

    assert_eq!(poll(&addr, &code).await["state"], "refused");
    let made: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
    assert_eq!(made, 0);
}

/// Доказательств секрета два, и нужны оба.
#[sqlx::test]
async fn a_webhook_without_both_proofs_is_a_page_that_does_not_exist(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    let word = ask_for_word(&addr, None).await;
    let code = word["code"].as_str().unwrap().to_string();

    // Секрет в адресе чужой.
    assert_eq!(
        webhook_with(&addr, "ne-tot-secret", Some(SECRET), start(444, "x", "X", &code)).await,
        404
    );
    // Заголовка нет вовсе.
    assert_eq!(
        webhook_with(&addr, SECRET, None, start(444, "x", "X", &code)).await,
        404
    );
    // Заголовок чужой.
    assert_eq!(
        webhook_with(&addr, SECRET, Some("ne-tot-secret"), start(444, "x", "X", &code)).await,
        404
    );

    assert_eq!(poll(&addr, &code).await["state"], "waiting", "слово сдвинулось");
}

/// Заблокированному вторая дверь закрыта так же, как первая.
#[sqlx::test]
async fn a_blocked_visitor_does_not_walk_in_through_the_second_door(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    sqlx::query(
        "INSERT INTO users (display_name, telegram_id, telegram_linked_at, is_blocked)
         VALUES ('Закрытый', 909, NOW(), true)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let word = ask_for_word(&addr, None).await;
    let code = word["code"].as_str().unwrap().to_string();
    webhook(&addr, start(909, "blocked", "Закрытый", &code)).await;
    webhook(&addr, press(909, "blocked", "Закрытый", &format!("ok:{code}"))).await;

    assert_ne!(poll(&addr, &code).await["state"], "ready", "заблокированный вошёл");
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sessions, 0);
}

/// Привязка из профиля: тот же обряд, но в конце не новый человек, а вторая
/// дверь у прежнего — и почта со значками остаются при нём.
#[sqlx::test]
async fn a_linked_account_keeps_its_first_door(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, display_name, visual_password_hash)
         VALUES ('masha@example.test', 'Маша', 'hash') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let session = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO user_sessions (user_id, token, expires_at)
         VALUES ($1, $2, NOW() + INTERVAL '30 days')",
    )
    .bind(user_id)
    .bind(print_of(&session))
    .execute(&pool)
    .await
    .unwrap();

    let word = ask_for_word(&addr, Some(&session)).await;
    let code = word["code"].as_str().unwrap().to_string();
    webhook(&addr, start(616, "masha", "Маша", &code)).await;
    webhook(&addr, press(616, "masha", "Маша", &format!("ok:{code}"))).await;

    let ready = poll(&addr, &code).await;
    assert_eq!(ready["state"], "ready");
    assert_eq!(ready["user"]["id"], user_id.to_string(), "завёлся второй архив");
    assert_eq!(ready["user"]["email"], "masha@example.test", "почта потерялась");

    let (tg, hash): (Option<i64>, Option<String>) =
        sqlx::query_as("SELECT telegram_id, visual_password_hash FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(tg, Some(616));
    assert_eq!(hash.as_deref(), Some("hash"), "значки стёрлись привязкой");

    let people: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
    assert_eq!(people, 1);
}

/// Один Telegram — один архив: привязать занятое число нельзя.
#[sqlx::test]
async fn one_telegram_cannot_hold_two_archives(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    sqlx::query(
        "INSERT INTO users (display_name, telegram_id, telegram_linked_at)
         VALUES ('Первый', 808, NOW())",
    )
    .execute(&pool)
    .await
    .unwrap();
    let other: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, display_name, visual_password_hash)
         VALUES ('vtoroy@example.test', 'Второй', 'hash') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let session = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO user_sessions (user_id, token, expires_at)
         VALUES ($1, $2, NOW() + INTERVAL '30 days')",
    )
    .bind(other)
    .bind(print_of(&session))
    .execute(&pool)
    .await
    .unwrap();

    let word = ask_for_word(&addr, Some(&session)).await;
    let code = word["code"].as_str().unwrap().to_string();
    webhook(&addr, start(808, "zanyato", "Занято", &code)).await;
    webhook(&addr, press(808, "zanyato", "Занято", &format!("ok:{code}"))).await;

    assert_ne!(poll(&addr, &code).await["state"], "ready");
    let tg: Option<i64> = sqlx::query_scalar("SELECT telegram_id FROM users WHERE id = $1")
        .bind(other)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(tg, None, "занятое число переписано на второй архив");
}

/// Отвязать последнюю дверь нельзя: человек запер бы себя снаружи.
#[sqlx::test]
async fn the_only_door_cannot_be_taken_off_its_hinges(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    let word = ask_for_word(&addr, None).await;
    let code = word["code"].as_str().unwrap().to_string();
    webhook(&addr, start(1212, "odin", "Один", &code)).await;
    webhook(&addr, press(1212, "odin", "Один", &format!("ok:{code}"))).await;
    let ready = poll(&addr, &code).await;
    let token = ready["sessionToken"].as_str().unwrap().to_string();
    // Ни почты, ни знаков — обе половины первой двери отсутствуют.
    assert!(ready["user"]["email"].is_null());
    assert_eq!(ready["user"]["hasSigns"], false);

    let resp = reqwest::Client::new()
        .delete(format!("{addr}/api/v1/auth/telegram/link"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "единственную дверь сняли с петель");

    let tg: Option<i64> = sqlx::query_scalar("SELECT telegram_id FROM users WHERE telegram_id = 1212")
        .fetch_optional(&pool)
        .await
        .unwrap()
        .flatten();
    assert_eq!(tg, Some(1212));
}

/// Когда есть чем войти без Telegram — отвязывается.
#[sqlx::test]
async fn a_second_door_may_be_taken_off(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, display_name, visual_password_hash, email_confirmed_at,
                             telegram_id, telegram_username, telegram_linked_at)
         VALUES ('oba@example.test', 'Оба', 'hash', NOW(), 1313, 'oba', NOW()) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let session = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO user_sessions (user_id, token, expires_at)
         VALUES ($1, $2, NOW() + INTERVAL '30 days')",
    )
    .bind(user_id)
    .bind(print_of(&session))
    .execute(&pool)
    .await
    .unwrap();

    let resp = reqwest::Client::new()
        .delete(format!("{addr}/api/v1/auth/telegram/link"))
        .header("authorization", format!("Bearer {session}"))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let back: Value = resp.json().await.unwrap();
    assert_eq!(back["telegramLinked"], false);
    assert!(back["telegramUsername"].is_null());

    // Почта и знаки на месте: сняли вторую дверь, а не обе.
    let (tg, email, hash): (Option<i64>, Option<String>, Option<String>) =
        sqlx::query_as("SELECT telegram_id, email, visual_password_hash FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(tg, None);
    assert_eq!(email.as_deref(), Some("oba@example.test"));
    assert_eq!(hash.as_deref(), Some("hash"));
}

/// Почту спрашивает дело — и спрашивает один раз.
#[sqlx::test]
async fn the_errand_asks_for_an_email_and_the_name_keeps_it(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;
    let client = reqwest::Client::new();

    let word = ask_for_word(&addr, None).await;
    let code = word["code"].as_str().unwrap().to_string();
    webhook(&addr, start(2424, "bezpochty", "Без Почты", &code)).await;
    webhook(&addr, press(2424, "bezpochty", "Без Почты", &format!("ok:{code}"))).await;
    let token = poll(&addr, &code).await["sessionToken"].as_str().unwrap().to_string();

    // Дело спросило — человек назвал.
    let said: Value = client
        .post(format!("{addr}/api/v1/auth/email"))
        .header("authorization", format!("Bearer {token}"))
        .json(&json!({ "email": "  Bez.Pochty@Example.TEST " }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // Пробелы и регистр приведены: почта — ключ, и «А@» с «а@» одно и то же.
    assert_eq!(said["email"], "bez.pochty@example.test");

    // Второй раз не спрашивают, и молча заменить нельзя.
    let again = client
        .post(format!("{addr}/api/v1/auth/email"))
        .header("authorization", format!("Bearer {token}"))
        .json(&json!({ "email": "drugaya@example.test" }))
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), 400, "почта заменилась по дороге к заказу");

    // Та же самая — не отказ: повтор запроса не должен выглядеть поломкой.
    let same = client
        .post(format!("{addr}/api/v1/auth/email"))
        .header("authorization", format!("Bearer {token}"))
        .json(&json!({ "email": "bez.pochty@example.test" }))
        .send()
        .await
        .unwrap();
    assert!(same.status().is_success());

    let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE telegram_id = 2424")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(email.as_deref(), Some("bez.pochty@example.test"));
}

/// Чужая почта в своё имя не записывается.
#[sqlx::test]
async fn a_taken_email_is_refused(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let addr = spawn(pool.clone()).await;

    sqlx::query(
        "INSERT INTO users (email, display_name, visual_password_hash)
         VALUES ('zanyato@example.test', 'Занято', 'hash')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let word = ask_for_word(&addr, None).await;
    let code = word["code"].as_str().unwrap().to_string();
    webhook(&addr, start(2525, "chuzhoy", "Чужой", &code)).await;
    webhook(&addr, press(2525, "chuzhoy", "Чужой", &format!("ok:{code}"))).await;
    let token = poll(&addr, &code).await["sessionToken"].as_str().unwrap().to_string();

    let resp = reqwest::Client::new()
        .post(format!("{addr}/api/v1/auth/email"))
        .header("authorization", format!("Bearer {token}"))
        .json(&json!({ "email": "zanyato@example.test" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409, "чужая почта ушла в чужое имя");

    let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE telegram_id = 2525")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(email, None);
}
