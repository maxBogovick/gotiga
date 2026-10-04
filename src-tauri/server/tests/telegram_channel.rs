//! Объявления в Telegram-канале.
//!
//! Стенд здесь локальный (`PUBLIC_URL` на 127.0.0.1), поэтому пост не уходит в
//! Telegram, а печатается в журнал и отмечается вышедшим, — ровно тот путь,
//! которым канал ходит на рабочем сервере, кроме самого вызова бота.
//! Проверяется журнал: что выходит, когда и сколько раз.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config(channel: Option<&str>) -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-channel-uploads-{}", Uuid::new_v4()),
        public_url: "http://127.0.0.1".into(),
        rust_log: String::new(),
        admin_login: "admin".into(),
        admin_password: "test-password-123".into(),
        cors_allowed_origins: vec![],
        telegram_bot_token: Some("123:test".into()),
        telegram_login_bot_token: None,
        telegram_login_bot_username: None,
        telegram_webhook_secret: None,
        telegram_channel_id: channel.map(str::to_string),
        telegram_chat_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-channel-logs-{}.sqlite", Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".into(),
        auth_pepper: "pepper-for-tests-0123456789".into(),
        auth_pepper_old: None,
    }
}

fn service(pool: &PgPool, channel: Option<&str>) -> AppService {
    AppService::new(Repository::new(pool.clone()), config(channel))
}

async fn seed_work(pool: &PgPool, name: &str, status: &str) -> Uuid {
    sqlx::query_scalar(&format!(
        "INSERT INTO figurines (name, short_text, is_visible, status)
         VALUES ($1, 'Бронза и воск', TRUE, '{status}') RETURNING id"
    ))
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("завести работу")
}

async fn seed_leaf(pool: &PgPool, kind: &str, status: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO gazette_leaves (slug, kind, status, title_en, title_ru, dek_ru, published_at)
         VALUES ($1, $2, $3, 'A tale', 'Байка', 'О том, как', NOW()) RETURNING id",
    )
    .bind(format!("leaf-{}", Uuid::new_v4().simple()))
    .bind(kind)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("завести лист")
}

/// Выдержка прошла: отодвинуть «замечено» в прошлое.
async fn let_time_pass(pool: &PgPool) {
    sqlx::query("UPDATE telegram_channel_posts SET seen_at = seen_at - INTERVAL '1 hour'")
        .execute(pool)
        .await
        .expect("сдвинуть время");
}

async fn ledger(pool: &PgPool, id: Uuid) -> Option<(bool, bool)> {
    sqlx::query_as("SELECT skipped, posted_at IS NOT NULL FROM telegram_channel_posts WHERE target_id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .expect("журнал")
}

/// Без канала — ни одной строки журнала: объявлять некуда.
#[sqlx::test]
async fn no_channel_no_ledger(pool: PgPool) {
    seed_work(&pool, "Ворон", "available").await;
    let svc = service(&pool, None);
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 0);
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM telegram_channel_posts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}

/// Архив, стоявший до подключения канала, в канал не выходит; новое выходит
/// один раз и только после выдержки.
#[sqlx::test]
async fn the_archive_stays_quiet_and_new_work_goes_out_once(pool: PgPool) {
    let old = seed_work(&pool, "Старый страж", "available").await;
    let svc = service(&pool, Some("@ritunia_test"));

    // Подключение: архив отмечен пропущенным, ничего не вышло.
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 0);
    assert_eq!(ledger(&pool, old).await, Some((true, false)));

    // Новая работа замечена, но выдержка не прошла.
    let fresh = seed_work(&pool, "Новый страж", "available").await;
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 0);
    assert_eq!(ledger(&pool, fresh).await, Some((false, false)));

    // Выдержка прошла — выходит, и только она.
    let_time_pass(&pool).await;
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 1);
    assert_eq!(ledger(&pool, fresh).await, Some((false, true)));
    assert_eq!(ledger(&pool, old).await, Some((true, false)));

    // Скрыли и снова показали — второго поста нет.
    sqlx::query("UPDATE figurines SET is_visible = FALSE WHERE id = $1")
        .bind(fresh)
        .execute(&pool)
        .await
        .unwrap();
    svc.announce_to_channel().await.expect("тик");
    sqlx::query("UPDATE figurines SET is_visible = TRUE WHERE id = $1")
        .bind(fresh)
        .execute(&pool)
        .await
        .unwrap();
    let_time_pass(&pool).await;
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 0);
}

/// Работа «в работе» и черновик листа не выходят; снятое с «в работе» — выходит.
#[sqlx::test]
async fn only_what_is_on_display_goes_out(pool: PgPool) {
    let svc = service(&pool, Some("@ritunia_test"));
    svc.announce_to_channel().await.expect("подключение");

    let unfinished = seed_work(&pool, "Незаконченный", "in_progress").await;
    let draft = seed_leaf(&pool, "tale", "draft").await;
    let world = seed_leaf(&pool, "world", "published").await;
    let tale = seed_leaf(&pool, "tale", "published").await;
    svc.announce_to_channel().await.expect("тик");
    let_time_pass(&pool).await;
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 1, "только байка");
    assert_eq!(ledger(&pool, tale).await, Some((false, true)));
    assert_eq!(ledger(&pool, unfinished).await, None);
    assert_eq!(ledger(&pool, draft).await, None);
    assert_eq!(ledger(&pool, world).await, None, "чужие новости канал не пересказывает");

    // Работу закончили — теперь она на людях и выходит.
    sqlx::query("UPDATE figurines SET status = 'available' WHERE id = $1")
        .bind(unfinished)
        .execute(&pool)
        .await
        .unwrap();
    svc.announce_to_channel().await.expect("тик");
    let_time_pass(&pool).await;
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 1);
    assert_eq!(ledger(&pool, unfinished).await, Some((false, true)));
}
