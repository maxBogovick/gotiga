//! Картинки на печатях шапки в ответе `GET /site/counts` (`section_counts`):
//! последняя вышедшая небылица, фото случайной работы и случаи, когда их нет.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::SectionCountsQuery;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-nav-seals-uploads-{}", Uuid::new_v4()),
        public_url: "http://127.0.0.1".into(),
        rust_log: String::new(),
        admin_login: "admin".into(),
        admin_password: "test-password-123".into(),
        cors_allowed_origins: vec![],
        telegram_bot_token: None,
        telegram_login_bot_token: None,
        telegram_login_bot_username: None,
        telegram_webhook_secret: None,
        telegram_channel_id: Some("@ritunia_tales".into()),
        telegram_channel_extra: vec![],
        telegram_chat_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-nav-seals-logs-{}.sqlite", Uuid::new_v4()),
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

async fn counts(svc: &AppService) -> serde_json::Value {
    let q = SectionCountsQuery { works_since: None, gazette_since: None, tales_since: None };
    serde_json::to_value(svc.section_counts(q).await.expect("counts")).expect("json")
}

async fn seed_tale(pool: &PgPool, slug: &str, status: &str, published: &str, scheduled: Option<&str>) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO gazette_leaves (slug, kind, status, title_en, title_ru, published_at, scheduled_at)
         VALUES ($1, 'tale', $2, $1, $1, $3::timestamptz, $4::timestamptz)
         RETURNING id",
    )
    .bind(slug)
    .bind(status)
    .bind(published)
    .bind(scheduled)
    .fetch_one(pool)
    .await
    .expect("insert tale")
}

async fn seed_work_with_photo(pool: &PgPool, name: &str, photo: bool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO figurines (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .expect("insert work");
    if photo {
        sqlx::query(
            "INSERT INTO images (id, figurine_id, image_type, file_path, sort_order)
             VALUES ($1, $2, 'face'::image_type, '/static/x.jpg', 0)",
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .execute(pool)
        .await
        .expect("insert photo");
    }
    id
}

#[sqlx::test]
async fn the_seal_takes_the_latest_published_tale_not_a_draft_or_a_future_one(pool: PgPool) {
    let svc = AppService::new(Repository::new(pool.clone()), config());
    seed_tale(&pool, "old", "published", "2026-01-01T00:00:00Z", None).await;
    seed_tale(&pool, "new", "published", "2026-03-01T00:00:00Z", None).await;
    // Черновик с самой поздней датой и запланированная на будущее не на людях.
    seed_tale(&pool, "draft", "draft", "2026-06-01T00:00:00Z", None).await;
    seed_tale(&pool, "future", "scheduled", "2026-07-01T00:00:00Z", Some("2999-01-01T00:00:00Z")).await;

    let json = counts(&svc).await;
    assert_eq!(json["sealTale"]["titleEn"], "new");
    // Тела небылицы в ответ не попадают: печати нужны только названия и картинки.
    assert!(json["sealTale"].get("bodyEn").is_none());
}

#[sqlx::test]
async fn no_tale_and_no_photo_means_no_fields_rather_than_empty_ones(pool: PgPool) {
    let svc = AppService::new(Repository::new(pool.clone()), config());
    seed_work_with_photo(&pool, "Без фото", false).await;

    let json = counts(&svc).await;
    assert!(json.get("sealTale").is_none());
    assert!(json.get("sealWork").is_none());
    // Числа при этом на месте.
    assert_eq!(json["works"]["total"], 1);
}

#[sqlx::test]
async fn the_seal_work_is_a_visible_work_with_a_photo(pool: PgPool) {
    let svc = AppService::new(Repository::new(pool.clone()), config());
    seed_work_with_photo(&pool, "Без фото", false).await;
    seed_work_with_photo(&pool, "С фото", true).await;
    // Скрытая работа с фото на печать не попадает.
    let hidden = seed_work_with_photo(&pool, "Скрытая", true).await;
    sqlx::query("UPDATE figurines SET is_visible = false WHERE id = $1")
        .bind(hidden)
        .execute(&pool)
        .await
        .unwrap();

    // Выбор случайный, поэтому несколько раз: попасть можно только на «С фото».
    for _ in 0..5 {
        let json = counts(&svc).await;
        assert!(json["sealWork"]["src"].as_str().unwrap().contains("x.jpg"));
    }
}

#[sqlx::test]
async fn the_seal_work_is_not_sent_when_the_author_set_own_art(pool: PgPool) {
    let svc = AppService::new(Repository::new(pool.clone()), config());
    seed_work_with_photo(&pool, "С фото", true).await;
    assert!(counts(&svc).await.get("sealWork").is_some());

    let repo = Repository::new(pool.clone());
    repo.upsert_setting(
        "nav_seals",
        r#"{"sections":{"works":{"images":[{"id":"a","url":"/static/seals/a.webp","urlSmall":"/static/seals/a-s.webp","width":64,"height":64,"widthSmall":32,"heightSmall":32}],"intervalSec":20,"shape":"free"}}}"#,
    )
    .await
    .expect("setting");
    // Кэш общей части живёт минуту; сохранение из админки его сбрасывает, прямая
    // запись в настройку — нет, поэтому пересоздаём сервис.
    let fresh = AppService::new(Repository::new(pool.clone()), config());
    assert!(counts(&fresh).await.get("sealWork").is_none());
}
