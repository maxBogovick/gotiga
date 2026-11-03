//! Массовый перевод (`services/translations*.rs`): выгрузка того, чего не хватает,
//! и вставка готового, для работ и для остальных разделов.
//!
//! Проверяется то, что легко сломать молча: вставка заполняет только пустое и не
//! затирает прежнее, `overwrite` заменяет, чужой язык и слишком длинный текст
//! пропускаются с пояснением, а остальные записи сохраняются; тексты внутри JSON
//! карты (черты) и настройки (рамки) меняются, не задевая соседние поля.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::services::{AppService, SectionImport, TranslationImport};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-translations-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-translations-logs-{}.sqlite", Uuid::new_v4()),
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

fn langs(l: &[&str]) -> Vec<String> {
    l.iter().map(|s| s.to_string()).collect()
}

fn section_import(section: &str, items: serde_json::Value, overwrite: bool) -> SectionImport {
    serde_json::from_value(json!({ "section": section, "items": items, "overwrite": overwrite }))
        .expect("section import")
}

#[sqlx::test]
async fn tale_columns_are_filled_once_and_replaced_on_request(pool: PgPool) {
    let svc = service(&pool);
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO gazette_leaves (slug, kind, status, title_en, title_ru, title_es)
         VALUES ('t', 'tale', 'draft', 'Tale', 'Байка', 'Cuento') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let miss = svc.missing_section("tales", &langs(&["es", "de"]), None, None).await.unwrap();
    assert_eq!(miss.total, 1);
    let task = &miss.items[0];
    assert_eq!(task.id, format!("tale:{id}"));
    assert_eq!(task.need.get("de").map(|v| v.as_slice()), Some(&["title".to_string()][..]));
    assert!(task.need.get("es").is_none(), "Spanish title is already there");

    let first = svc
        .import_section(section_import(
            "tales",
            json!([
                { "id": task.id, "lang": "de", "title": "Geschichte" },
                { "id": task.id, "lang": "es", "title": "Otro" },
                { "id": task.id, "lang": "xx", "title": "?" },
                { "id": task.id, "lang": "de", "title": "x".repeat(201) },
            ]),
            false,
        ))
        .await
        .unwrap();
    assert_eq!(first.saved, 1, "only the empty German title is written");
    assert_eq!(first.skipped, 1, "Spanish was filled, so nothing changed");
    assert_eq!(first.errors.len(), 2, "unknown language and too long: {:?}", first.errors);

    let (de, es): (String, String) =
        sqlx::query_as("SELECT title_de, title_es FROM gazette_leaves WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((de.as_str(), es.as_str()), ("Geschichte", "Cuento"));

    let again = svc
        .import_section(section_import(
            "tales",
            json!([{ "id": format!("tale:{id}"), "lang": "es", "title": "Otro" }]),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(again.saved, 1);
    let (es,): (String,) = sqlx::query_as("SELECT title_es FROM gazette_leaves WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(es, "Otro");
}

#[sqlx::test]
async fn card_trait_names_live_inside_json_and_neighbours_stay(pool: PgPool) {
    let svc = service(&pool);
    let traits = json!([
        { "nameEn": "Brave", "nameRu": "Храбрый", "textEn": "Strong", "textRu": "Сильный", "textEs": "Fuerte" }
    ])
    .to_string();
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO battle_cards (slug, title_en, title_ru, price_dust, traits)
         VALUES ('c', 'Card', 'Карта', 5, $1) RETURNING id",
    )
    .bind(&traits)
    .fetch_one(&pool)
    .await
    .unwrap();

    let miss = svc.missing_section("cards", &langs(&["es"]), None, None).await.unwrap();
    let t = miss.items.iter().find(|i| i.part == "trait").expect("trait task");
    assert_eq!(t.id, format!("trait:{id}#0"));
    assert_eq!(t.need["es"], vec!["name".to_string()], "text_es exists, name_es does not");

    let r = svc
        .import_section(section_import(
            "cards",
            json!([{ "id": t.id, "lang": "es", "name": "Valiente", "text": "Nuevo" }]),
            false,
        ))
        .await
        .unwrap();
    assert_eq!(r.saved, 1);
    let (raw,): (String,) = sqlx::query_as("SELECT traits FROM battle_cards WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v[0]["nameEs"], "Valiente");
    assert_eq!(v[0]["textEs"], "Fuerte", "an existing text is not replaced");
    assert_eq!(v[0]["nameRu"], "Храбрый");
}

#[sqlx::test]
async fn frame_names_and_site_copy_live_in_settings(pool: PgPool) {
    let svc = service(&pool);
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES
           ('battle_frames', $1), ('copy_overrides', $2)",
    )
    .bind(json!({ "frames": [{ "tier": 3, "nameEn": "Third", "nameRu": "Третий", "paper": "#fff" }] }).to_string())
    .bind(json!({ "en": { "home.title": "Hello {brand}" }, "ru": {} }).to_string())
    .execute(&pool)
    .await
    .unwrap();

    let miss = svc.missing_section("terms", &langs(&["it"]), None, None).await.unwrap();
    let frame = miss.items.iter().find(|i| i.part == "frame").expect("frame task");
    assert_eq!(frame.id, "frame:3");
    let site = svc.missing_section("site", &langs(&["it"]), None, None).await.unwrap();
    assert_eq!(site.items.len(), 1);
    assert_eq!(site.items[0].id, "copy:home.title");

    let r = svc
        .import_section(section_import(
            "terms",
            json!([{ "id": "frame:3", "lang": "it", "name": "Terzo" }]),
            false,
        ))
        .await
        .unwrap();
    assert_eq!(r.saved, 1);
    let r = svc
        .import_section(section_import(
            "site",
            json!([
                { "id": "copy:home.title", "lang": "it", "text": "Ciao {brand}" },
                { "id": "copy:no.such.key", "lang": "it", "text": "?" },
            ]),
            false,
        ))
        .await
        .unwrap();
    assert_eq!((r.saved, r.errors.len()), (1, 1), "{:?}", r.errors);

    let (frames,): (String,) = sqlx::query_as("SELECT value FROM settings WHERE key = 'battle_frames'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&frames).unwrap();
    assert_eq!(v["frames"][0]["nameIt"], "Terzo");
    assert_eq!(v["frames"][0]["paper"], "#fff", "other frame fields stay");
    let (copy,): (String,) = sqlx::query_as("SELECT value FROM settings WHERE key = 'copy_overrides'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&copy).unwrap();
    assert_eq!(v["it"]["home.title"], "Ciao {brand}");
    assert_eq!(v["en"]["home.title"], "Hello {brand}");
}

#[sqlx::test]
async fn works_import_fills_only_empty_and_reports_unknown_work(pool: PgPool) {
    let svc = service(&pool);
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO figurines (name, short_text) VALUES ('Gnome', 'A gnome.') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let miss = svc.missing_translations(&langs(&["es"]), None, None).await.unwrap();
    assert_eq!(miss.total, 1);
    assert_eq!(miss.items[0].need["es"], vec!["name".to_string(), "shortText".to_string()]);

    let req = |overwrite: bool, name: &str| -> TranslationImport {
        serde_json::from_value(json!({
            "overwrite": overwrite,
            "items": [
                { "id": id, "lang": "es", "name": name, "shortText": "Un gnomo." },
                { "id": Uuid::new_v4(), "lang": "es", "name": "?" },
            ],
        }))
        .unwrap()
    };
    let r = svc.import_translations(req(false, "Gnomo")).await.unwrap();
    assert_eq!((r.saved, r.errors.len()), (1, 1), "{:?}", r.errors);
    let r = svc.import_translations(req(false, "Duende")).await.unwrap();
    assert_eq!(r.saved, 0, "nothing changes on a second fill");
    assert_eq!(svc.missing_translations(&langs(&["es"]), None, None).await.unwrap().total, 0);
    let r = svc.import_translations(req(true, "Duende")).await.unwrap();
    assert_eq!(r.saved, 1);
    let (name,): (String,) =
        sqlx::query_as("SELECT name FROM figurine_translations WHERE figurine_id = $1 AND lang = 'es'")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(name, "Duende");
}

#[sqlx::test]
async fn every_section_can_be_counted_on_an_empty_base(pool: PgPool) {
    let svc = service(&pool);
    let counts = svc
        .translation_section_counts(&langs(&["en", "ru", "uk", "es", "de", "it"]))
        .await
        .expect("all parts query real columns");
    assert_eq!(counts.len(), 6);
    assert!(counts.iter().all(|c| c.missing == 0 || c.id == "terms" || c.id == "errands" || c.id == "cards"));
}
