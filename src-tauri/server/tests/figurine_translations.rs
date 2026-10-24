//! Тексты работы на других языках (`figurine_translations`) и описания её
//! фотографий (`figurine_image_translations`): запись вместе с работой, чтение
//! в списке и на странице работы.
//!
//! Проверяется то, что легко сломать молча: сохранение без поля не трогает
//! переводы, пустой язык удаляется, список несёт только поля списка, снятая
//! фотография уносит своё описание, а пересохранение работы — нет.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::{FigurineQuery, SaveFigurineRequest};
use gotiga_server::services::AppService;
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

fn request(id: Uuid, photo: Uuid, extra: serde_json::Value) -> SaveFigurineRequest {
    let mut body = json!({
        "id": id.to_string(),
        "name": "Witch",
        "shortText": "A small witch",
        "material": "Clay",
        "status": "available",
        "sortOrder": 1,
        "isVisible": true,
        "isFeatured": false,
        "images": [{
            "id": photo.to_string(),
            "imageType": "face",
            "url": "/static/images/x.jpg",
            "altText": "A witch with a broom"
        }]
    });
    let (Some(target), Some(more)) = (body.as_object_mut(), extra.as_object()) else {
        unreachable!()
    };
    for (k, v) in more {
        target.insert(k.clone(), v.clone());
    }
    serde_json::from_value(body).expect("request")
}

async fn list(svc: &AppService) -> Vec<gotiga_server::models::FigurineListItemDto> {
    let query: FigurineQuery = serde_json::from_value(json!({})).expect("query");
    svc.list_figurines(true, query).await.expect("list").items
}

#[sqlx::test]
async fn translations_are_written_with_the_work_and_read_back(pool: PgPool) {
    let svc = service(&pool);
    let id = Uuid::new_v4();
    let photo = Uuid::new_v4();
    svc.save_figurine(request(
        id,
        photo,
        json!({
            "translations": {
                "es": { "name": "Bruja", "shortText": "  Una bruja pequeña ", "fullDescription": "Larga", "material": "" },
                "de": { "name": "Hexe", "secretText": "Geheim" }
            }
        }),
    ))
    .await
    .expect("save");

    let detail = svc.get_figurine_details(id.to_string(), false).await.expect("detail");
    let es = &detail.translations["es"];
    assert_eq!(es.name.as_deref(), Some("Bruja"));
    assert_eq!(es.short_text.as_deref(), Some("Una bruja pequeña"));
    assert_eq!(es.full_description.as_deref(), Some("Larga"));
    assert_eq!(es.material, None, "an empty string is stored as NULL");
    assert_eq!(detail.translations["de"].secret_text.as_deref(), Some("Geheim"));

    // В списке — только поля списка, и язык без них не печатается вовсе.
    let items = list(&svc).await;
    let item = items.iter().find(|i| i.id == id.to_string()).expect("item");
    assert_eq!(item.translations["es"].name.as_deref(), Some("Bruja"));
    assert_eq!(item.translations["es"].full_description, None);
    assert_eq!(item.translations["de"].secret_text, None);
    let shown = serde_json::to_value(item).expect("json");
    assert_eq!(shown["translations"]["es"], json!({ "name": "Bruja", "shortText": "Una bruja pequeña" }));
}

#[sqlx::test]
async fn a_save_without_the_field_leaves_translations_alone(pool: PgPool) {
    let svc = service(&pool);
    let id = Uuid::new_v4();
    let photo = Uuid::new_v4();
    svc.save_figurine(request(
        id,
        photo,
        json!({
            "translations": { "it": { "name": "Strega" }, "ru": { "name": "Ведьма" } },
            "images": [{
                "id": photo.to_string(), "imageType": "face", "url": "/static/images/x.jpg",
                "altText": "A witch", "altTranslations": { "it": "Una strega", "ru": "Ведьма" }
            }]
        }),
    ))
    .await
    .expect("first save");

    // Старая вкладка админки: ни `translations`, ни `altTranslations`.
    svc.save_figurine(request(id, photo, json!({}))).await.expect("second save");
    let detail = svc.get_figurine_details(id.to_string(), true).await.expect("detail");
    assert_eq!(detail.translations["it"].name.as_deref(), Some("Strega"));
    assert_eq!(detail.images[0].alt_translations["it"], "Una strega");

    // Названный язык переписывается, пустой — удаляется, не названный остаётся.
    svc.save_figurine(request(
        id,
        photo,
        json!({
            "translations": { "it": { "name": "" } },
            "images": [{
                "id": photo.to_string(), "imageType": "face", "url": "/static/images/x.jpg",
                "altTranslations": { "it": "" }
            }]
        }),
    ))
    .await
    .expect("third save");
    let detail = svc.get_figurine_details(id.to_string(), true).await.expect("detail");
    assert!(!detail.translations.contains_key("it"));
    assert_eq!(detail.translations["ru"].name.as_deref(), Some("Ведьма"));
    assert!(!detail.images[0].alt_translations.contains_key("it"));
    assert_eq!(detail.images[0].alt_translations["ru"], "Ведьма");

    // Снятая фотография уносит свои описания.
    let other = Uuid::new_v4();
    svc.save_figurine(request(
        id,
        other,
        json!({}),
    ))
    .await
    .expect("photo replaced");
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM figurine_image_translations")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(left, 0);
}

#[sqlx::test]
async fn english_and_unknown_languages_are_refused(pool: PgPool) {
    let svc = service(&pool);
    let id = Uuid::new_v4();
    for lang in ["en", "fr"] {
        let err = svc
            .save_figurine(request(
                id,
                Uuid::new_v4(),
                json!({ "translations": { lang: { "name": "X" } } }),
            ))
            .await;
        assert!(err.is_err(), "{lang} must be refused");
    }
    let err = svc
        .save_figurine(request(
            id,
            Uuid::new_v4(),
            json!({ "translations": { "es": { "name": "x".repeat(201) } } }),
        ))
        .await;
    assert!(err.is_err(), "a name over 200 characters must be refused");
}

#[sqlx::test]
async fn new_language_columns_and_checks_accept_es_de_it(pool: PgPool) {
    // Колонки `_es/_de/_it` есть и пустыми проходят; язык письма — любой из шести.
    sqlx::query(
        "INSERT INTO gazette_leaves (slug, kind, status, title_en, title_ru, title_es, title_de, title_it)
         VALUES ('t', 'tale', 'draft', 'T', 'Т', 'Cuento', '', 'Racconto')",
    )
    .execute(&pool)
    .await
    .expect("leaf with es/de/it");
    let too_long = sqlx::query(
        "INSERT INTO gazette_leaves (slug, kind, status, title_en, title_ru, title_de)
         VALUES ('t2', 'tale', 'draft', 'T', 'Т', $1)",
    )
    .bind("x".repeat(201))
    .execute(&pool)
    .await;
    assert!(too_long.is_err(), "the German title keeps the Ukrainian limit");
    sqlx::query(
        "INSERT INTO telegram_subscribers (unsubscribe_token, lang) VALUES ('tok', 'de')",
    )
    .execute(&pool)
    .await
    .map(|_| ())
    .or_else(|e| {
        // Другие обязательные колонки — не предмет проверки; важен только язык.
        let msg = e.to_string();
        assert!(!msg.contains("lang_check"), "{msg}");
        Ok::<(), sqlx::Error>(())
    })
    .expect("telegram subscriber in German");
}

#[sqlx::test]
async fn summaries_that_name_a_work_carry_its_translated_names(pool: PgPool) {
    let svc = service(&pool);
    let id = Uuid::new_v4();
    svc.save_figurine(request(
        id,
        Uuid::new_v4(),
        json!({ "translations": { "de": { "name": "Hexe", "fullDescription": "Lang" } } }),
    ))
    .await
    .expect("save");

    // Лист вестника о работе: название работы на других языках.
    sqlx::query(
        "INSERT INTO gazette_leaves (slug, kind, status, title_en, title_ru, figurine_id, published_at)
         VALUES ('leaf', 'tale', 'published', 'Leaf', 'Лист', $1, now())",
    )
    .bind(id)
    .execute(&pool)
    .await
    .expect("leaf");
    let leaves = svc.list_gazette_for_work(id).await.expect("leaves");
    let leaf = leaves.first().expect("one leaf");
    assert_eq!(leaf.figurine_name_translations.get("de").map(String::as_str), Some("Hexe"));

    // Карточки, собранные не основным списком (здесь — «в работе»), тоже несут
    // переводы, и только поля списка.
    sqlx::query("UPDATE figurines SET status = 'in_progress' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("status");
    let cards = svc.list_in_progress_figurines().await.expect("in progress");
    let card = cards.iter().find(|c| c.id == id.to_string()).expect("card");
    assert_eq!(card.translations["de"].name.as_deref(), Some("Hexe"));
    assert_eq!(card.translations["de"].full_description, None);
}

#[test]
fn scene_captions_keep_their_languages() {
    let frame: gotiga_server::models::TaleSceneFrame = serde_json::from_value(json!({
        "url": "/static/a.jpg", "caption": "Witch", "captionDe": " Hexe "
    }))
    .expect("frame");
    assert_eq!(frame.caption_de, " Hexe ");
    let out = serde_json::to_value(&frame).expect("json");
    assert_eq!(out["captionDe"], " Hexe ");
    assert!(out.get("captionRu").is_none(), "empty languages are not printed");
}

#[sqlx::test]
async fn showing_room_names_and_home_texts_keep_their_languages(pool: PgPool) {
    use gotiga_server::models::{HomeContent, SaveShowingRoomRequest};
    let svc = service(&pool);
    let id = Uuid::new_v4();
    let room = |extra: serde_json::Value| -> SaveShowingRoomRequest {
        let mut body = json!({ "id": id.to_string(), "name": "Night Hall", "openFromMin": 0, "openUntilMin": 60 });
        for (k, v) in extra.as_object().unwrap() {
            body[k] = v.clone();
        }
        serde_json::from_value(body).expect("room")
    };
    svc.save_showing_room(room(json!({ "nameRu": "Ночной зал", "nameDe": " Nachtsaal " })))
        .await
        .expect("save room");
    // Старая вкладка без полей: названия остаются; пустая строка снимает.
    svc.save_showing_room(room(json!({}))).await.expect("resave");
    svc.save_showing_room(room(json!({ "nameRu": "" }))).await.expect("clear ru");
    let rooms = svc.get_showing_rooms().await.expect("rooms");
    let shown = serde_json::to_value(&rooms[0]).expect("json");
    assert_eq!(shown["nameDe"], "Nachtsaal");
    assert!(shown.get("nameRu").is_none());

    let first: HomeContent = serde_json::from_value(json!({ "kicker": "Dolls", "kickerEs": " Muñecas " })).unwrap();
    svc.save_home_content(first).await.expect("home");
    let again: HomeContent = serde_json::from_value(json!({ "kicker": "Dolls", "leadDe": "Puppen" })).unwrap();
    svc.save_home_content(again).await.expect("home again");
    let home = serde_json::to_value(svc.get_home_content().await.expect("read")).expect("json");
    assert_eq!(home["kickerEs"], "Muñecas", "a field the old form did not send is kept");
    assert_eq!(home["leadDe"], "Puppen");
    let cleared: HomeContent = serde_json::from_value(json!({ "kicker": "Dolls", "kickerEs": "" })).unwrap();
    svc.save_home_content(cleared).await.expect("clear");
    let home = serde_json::to_value(svc.get_home_content().await.expect("read")).expect("json");
    assert!(home.get("kickerEs").is_none());
}

#[sqlx::test]
async fn a_request_remembers_the_language_it_was_sent_in(pool: PgPool) {
    use gotiga_server::models::OrderRequest;
    let svc = service(&pool);
    let id = Uuid::new_v4();
    svc.save_figurine(request(id, Uuid::new_v4(), json!({}))).await.expect("save");
    let order = |lang: Option<&str>| -> OrderRequest {
        let mut body = json!({
            "figurineId": id.to_string(), "figurineName": "Witch",
            "requesterName": "A", "requesterEmail": "a@example.org",
            "message": "Hello", "mode": "question", "ageConfirmed": true
        });
        if let Some(l) = lang {
            body["lang"] = json!(l);
        }
        serde_json::from_value(body).expect("order")
    };
    let de = svc.create_order(&order(Some("de")), None).await.expect("de order");
    let none = svc.create_order(&order(Some("xx")), None).await.expect("unknown lang order");
    let lang_of = |id: Uuid| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, Option<String>>("SELECT lang FROM orders WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .expect("lang")
        }
    };
    assert_eq!(lang_of(de.id).await.as_deref(), Some("de"));
    assert_eq!(lang_of(none.id).await, None, "an unknown code is not stored");
}

#[sqlx::test]
async fn author_texts_keep_their_languages(pool: PgPool) {
    use gotiga_server::models::AuthorProfile;
    let svc = service(&pool);
    let first: AuthorProfile =
        serde_json::from_value(json!({ "name": "Rita", "tagline": "Dolls", "taglineIt": " Bambole ", "bioDe": "Text" })).unwrap();
    svc.save_author_profile(first).await.expect("save");
    let again: AuthorProfile = serde_json::from_value(json!({ "name": "Rita", "tagline": "Dolls" })).unwrap();
    svc.save_author_profile(again).await.expect("save again");
    let shown = serde_json::to_value(svc.get_author_profile().await.expect("read")).unwrap();
    assert_eq!(shown["taglineIt"], "Bambole");
    assert_eq!(shown["bioDe"], "Text");
}
