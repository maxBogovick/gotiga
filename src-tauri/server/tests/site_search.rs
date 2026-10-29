//! Поиск по всему сайту на настоящей базе: разобранный набор работ (`SiteLexical`) пересобирается
//! после сохранения подписи, а другая форма слова находит работу.
//!
//! Модели эмбеддингов в тестовой сборке обычно нет, поэтому вещь подтверждают слова и
//! бонус за слова запроса; тесты верны и с моделью, и без неё.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-sitesearch-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-sitesearch-logs-{}.sqlite", Uuid::new_v4()),
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

async fn seed_work_with_text(pool: &PgPool, name: &str, short_text: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO figurines (id, name, short_text) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(name)
        .bind(short_text)
        .execute(pool)
        .await
        .expect("insert work");
    id
}

/// Слова, которые есть у четверти записей и больше, сайт считает общими и бонус за них не даёт, поэтому
/// в базе должно быть больше нескольких работ, как в настоящей.
async fn seed_filler(pool: &PgPool) {
    for (name, text) in [
        ("Tiny witch with broom", "Witch with a broom and a hat."),
        ("Old monk with candle", "Monk in a grey robe holding a candle."),
        ("Sailor with parrot", "Pirate sailor with a green parrot."),
        ("Girl with sparrow", "Village girl holding a sparrow."),
        ("Baker with bread", "Baker with fresh bread and an apron."),
        ("Musician with accordion", "Street musician playing the accordion."),
        ("Mermaid on a rock", "Sea maiden resting on a wet rock."),
        ("Vampire in a cape", "Vampire with a long red cape."),
        ("Shepherdess with lamb", "Old shepherdess with a small lamb."),
        ("Cook with ladle", "Cook stirring a pot with a ladle."),
    ] {
        seed_work_with_text(pool, name, text).await;
    }
}

async fn finds(svc: &AppService, query: &str, work: Uuid) -> bool {
    svc.site_search(query, 10)
        .await
        .expect("site search")
        .iter()
        .any(|h| h.id == work.to_string())
}

/// Слов подписи в тексте работы нет, поэтому до подписи её не находят; после сохранения подписи
/// найдут сразу, а не через минуту, пока живёт разобранный набор.
#[sqlx::test]
async fn a_saved_caption_is_searchable_at_once(pool: PgPool) {
    let svc = service(&pool);
    let gnome = seed_work_with_text(&pool, "Small art doll Gnome with axe", "Little gnome with axe, handmade.").await;
    seed_work_with_text(&pool, "Granny Baba Yaga", "Old woman with a broom.").await;
    seed_filler(&pool).await;

    // Первый запрос строит набор.
    assert!(!finds(&svc, "weapon", gnome).await, "до подписи слова «weapon» в тексте нет");

    svc.set_figurine_caption(
        gnome.to_string(),
        Some("A gnome holds an axe.\nKeywords: gnome, axe, weapon, tool | гном, топор, оружие, инструмент".into()),
    )
    .await
    .expect("set caption");

    assert!(finds(&svc, "weapon", gnome).await || finds(&svc, "оружие", gnome).await, "после подписи работа находится");
}

/// Другая форма слова: в тексте «зелёный», запрос «зеленая».
#[sqlx::test]
async fn another_form_of_a_word_finds_the_work(pool: PgPool) {
    let svc = service(&pool);
    let moss = seed_work_with_text(&pool, "Granny Baba Yaga", "Старая кукла: зелёный мох на юбке.").await;
    seed_work_with_text(&pool, "Small art doll Gnome with axe", "Little gnome with axe, handmade.").await;
    seed_filler(&pool).await;
    assert!(finds(&svc, "зеленая", moss).await, "«зеленая» находит «зелёный»");
    assert!(finds(&svc, "зелёного мха", moss).await || finds(&svc, "зелёного", moss).await, "«зелёного» находит «зелёный»");
}
