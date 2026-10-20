//! Помощник админки: что у работ и байек не доделано (`admin_readiness`).
//!
//! Проверяется каждый пропуск по отдельности и то, чего помощник **не**
//! должен говорить: второй ракурс у работы с одним фото, похожие при
//! собрании меньше четырёх работ, черновик байки.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::ReadinessReport;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-readiness-uploads-{}", Uuid::new_v4()),
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
        telegram_chat_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-readiness-logs-{}.sqlite", Uuid::new_v4()),
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

async fn seed_work(pool: &PgPool, name: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO figurines (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .expect("insert work");
    id
}

async fn add_photo(pool: &PgPool, work: Uuid, kind: &str, order: i32) {
    sqlx::query(
        "INSERT INTO images (id, figurine_id, image_type, file_path, sort_order)
         VALUES ($1, $2, $3::image_type, '/static/x.jpg', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(work)
    .bind(kind)
    .bind(order)
    .execute(pool)
    .await
    .expect("insert photo");
}

async fn add_similar(pool: &PgPool, work: Uuid, others: &[Uuid]) {
    for (i, other) in others.iter().enumerate() {
        sqlx::query(
            "INSERT INTO figurine_similar (figurine_id, similar_id, position) VALUES ($1, $2, $3)",
        )
        .bind(work)
        .bind(other)
        .bind(i as i32)
        .execute(pool)
        .await
        .expect("insert similar");
    }
}

fn gaps_of(report: &ReadinessReport, id: Uuid) -> Vec<String> {
    report
        .works
        .iter()
        .find(|w| w.id == id.to_string())
        .map(|w| w.gaps.clone())
        .unwrap_or_default()
}

#[sqlx::test]
async fn names_each_missing_step_of_a_work(pool: PgPool) {
    let svc = service(&pool);
    let bare = seed_work(&pool, "Без фото").await;
    let no_cover = seed_work(&pool, "Без обложки").await;
    let no_angle = seed_work(&pool, "Без ракурса").await;
    let one_photo = seed_work(&pool, "Одно фото").await;
    let done = seed_work(&pool, "Готовая").await;

    add_photo(&pool, no_cover, "full", 0).await;
    add_photo(&pool, no_angle, "face", 0).await;
    add_photo(&pool, no_angle, "full", 1).await;
    add_photo(&pool, one_photo, "face", 0).await;
    add_photo(&pool, done, "face", 0).await;
    add_photo(&pool, done, "detail", 1).await;
    add_similar(&pool, done, &[bare, no_cover, no_angle]).await;
    add_similar(&pool, one_photo, &[done]).await;

    let r = svc.admin_readiness().await.expect("readiness");
    assert_eq!(gaps_of(&r, bare), vec!["photos", "similar"]);
    assert_eq!(gaps_of(&r, no_cover), vec!["cover", "similar"]);
    assert_eq!(gaps_of(&r, no_angle), vec!["angle", "similar"]);
    // Одно фото — второго ракурса нет и быть не может.
    assert_eq!(gaps_of(&r, one_photo), vec!["similar"]);
    let one = r.works.iter().find(|w| w.id == one_photo.to_string()).unwrap();
    assert_eq!(one.similar_count, 1);
    // Готовая работа в ответ не попадает вовсе.
    assert!(gaps_of(&r, done).is_empty());
    assert!(r.works.iter().all(|w| w.id != done.to_string()));
}

#[sqlx::test]
async fn similar_is_asked_only_from_those_there_are(pool: PgPool) {
    let svc = service(&pool);
    let a = seed_work(&pool, "A").await;
    let b = seed_work(&pool, "B").await;
    add_photo(&pool, a, "face", 0).await;
    add_photo(&pool, b, "face", 0).await;
    add_similar(&pool, a, &[b]).await;

    // Из двух работ у каждой один сосед: одного похожего и хватит.
    let r = svc.admin_readiness().await.expect("readiness");
    assert!(gaps_of(&r, a).is_empty());
    assert_eq!(gaps_of(&r, b), vec!["similar"]);
}

#[sqlx::test]
async fn hidden_work_is_marked_hidden(pool: PgPool) {
    let svc = service(&pool);
    let w = seed_work(&pool, "Скрытая").await;
    sqlx::query("UPDATE figurines SET is_visible = false WHERE id = $1")
        .bind(w)
        .execute(&pool)
        .await
        .unwrap();
    let r = svc.admin_readiness().await.expect("readiness");
    let row = r.works.iter().find(|x| x.id == w.to_string()).unwrap();
    assert!(row.hidden);
}

#[sqlx::test]
async fn tales_on_display_are_checked_and_drafts_are_not(pool: PgPool) {
    let svc = service(&pool);
    sqlx::query(
        "INSERT INTO gazette_leaves (slug, kind, status, title_en, title_ru, title_uk,
                                     body_en, body_ru, body_uk, image_urls, published_at)
         VALUES ('full', 'tale', 'published', 'T', 'Т', 'Т', 'b', 'б', 'б',
                 ARRAY['/static/p.jpg'], NOW()),
                ('gaps', 'tale', 'published', 'T', 'Т', '', 'b', 'б', NULL, '{}', NOW()),
                ('notitle', 'tale', 'scheduled', 'T', 'Т', '', 'b', 'б', 'б',
                 ARRAY['/static/p.jpg'], NULL),
                ('draft', 'tale', 'draft', 'T', 'Т', '', NULL, NULL, NULL, '{}', NULL)",
    )
    .execute(&pool)
    .await
    .expect("insert tales");

    let r = svc.admin_readiness().await.expect("readiness");
    assert_eq!(r.tales.len(), 2, "{:?}", r.tales);
    // Без текста на одном языке и без фото.
    assert_eq!(
        r.tales.iter().find(|t| t.status == "published").map(|t| t.gaps.clone()),
        Some(vec!["body_uk".to_string(), "photo".to_string()])
    );
    // Текст есть, заглавия нет.
    assert_eq!(
        r.tales.iter().find(|t| t.status == "scheduled").map(|t| t.gaps.clone()),
        Some(vec!["title_uk".to_string()])
    );
}
