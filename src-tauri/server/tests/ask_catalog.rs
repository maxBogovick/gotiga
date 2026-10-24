//! Каталог ответов «Спросить сайт»: что кладёт миграция, как записи пишутся и
//! удаляются и какую запись подбирает ручка.
//!
//! Модель эмбеддингов в тестовой сборке обычно не лежит, и тогда подбор идёт по
//! словам и умеет только предлагать; тесты сказаны так, чтобы быть верными и с
//! моделью, и без неё. Пороги самой модели меряет `examples/ask_probe.rs`.

use gotiga_server::ask::{AskAnswer, LangTexts};
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::lang::Lang;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-ask-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-ask-logs-{}.sqlite", Uuid::new_v4()),
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

fn texts(en: &str, ru: &str) -> LangTexts {
    LangTexts { en: en.into(), ru: ru.into(), ..Default::default() }
}

fn entry(key: &str, phrases: &[&str], enabled: bool) -> AskAnswer {
    AskAnswer {
        key: key.into(),
        enabled,
        phrases: phrases.iter().map(|p| p.to_string()).collect(),
        question: texts(phrases[0], phrases[0]),
        answer: texts("An answer.", "Ответ."),
        ..Default::default()
    }
}

#[sqlx::test]
async fn the_migration_lays_out_a_catalog_with_honest_drafts(pool: PgPool) {
    let svc = service(&pool);
    let all = svc.admin_list_ask_answers().await.expect("list");
    assert!(all.len() >= 30, "{} entries", all.len());

    // Что сайт сам не говорит, лежит выключенным черновиком без ответа.
    for key in [
        "lead_time", "care", "discounts", "gift", "visit_workshop", "repair", "certificate", "customs", "custom_size",
    ] {
        let e = all.iter().find(|e| e.key == key).unwrap_or_else(|| panic!("no {key}"));
        assert!(!e.enabled, "{key} must be a draft");
        assert!(e.answer.en.is_empty() && e.answer.ru.is_empty(), "{key} must have no invented answer");
        assert!(e.phrases.len() >= 3, "{key}: a draft still has to be recognised");
    }
    // Что сайт говорит сам, включено и с текстом на английском и русском.
    // «Как получить новые карты?» включена второй партией: сайт сам говорит, что карты покупают за пыль.
    for key in ["how_order", "price", "returns", "contact", "play_rules", "bot_nature", "play_cards_get", "photo_share", "studio_what"] {
        let e = all.iter().find(|e| e.key == key).unwrap_or_else(|| panic!("no {key}"));
        assert!(e.enabled, "{key}");
        assert!(!e.answer.en.is_empty() && !e.answer.ru.is_empty(), "{key}");
        assert!(e.phrases.len() >= 3, "{key}");
    }
    // Данные собирает страница, текста у записи нет.
    let count = all.iter().find(|e| e.key == "data_count").unwrap();
    assert_eq!(count.handler.as_deref(), Some("count"));
    assert!(count.enabled && count.answer.en.is_empty());
}

#[sqlx::test]
async fn an_entry_is_saved_changed_and_deleted(pool: PgPool) {
    let svc = service(&pool);
    let saved = svc
        .admin_save_ask_answer(entry("opening_hours", &["When are you open?", "Часы работы", "Opening hours"], true))
        .await
        .expect("save");
    let id = saved.id.expect("an id is given");
    assert_eq!(saved.key, "opening_hours");

    let mut changed = saved.clone();
    changed.answer.en = "Always.".into();
    let again = svc.admin_save_ask_answer(changed).await.expect("update");
    assert_eq!(again.id, Some(id), "an update keeps the entry");
    assert_eq!(again.answer.en, "Always.");

    svc.admin_delete_ask_answer(id).await.expect("delete");
    let all = svc.admin_list_ask_answers().await.unwrap();
    assert!(all.iter().all(|e| e.key != "opening_hours"));
}

#[sqlx::test]
async fn a_bad_or_repeated_entry_is_refused_with_words(pool: PgPool) {
    let svc = service(&pool);
    // Включённая запись без русского ответа.
    let mut half = entry("half_done", &["a b c", "d e f", "g h i"], true);
    half.answer.ru.clear();
    let err = svc.admin_save_ask_answer(half).await.unwrap_err().to_string();
    assert!(err.contains("Russian"), "{err}");

    // Имя, уже занятое другой записью (она пришла из миграции).
    let err = svc
        .admin_save_ask_answer(entry("price", &["a b c", "d e f", "g h i"], false))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("already used"), "{err}");
}

#[sqlx::test]
async fn a_draft_is_recognised_but_never_answers_and_a_switched_on_entry_is(pool: PgPool) {
    let svc = service(&pool);
    svc.admin_save_ask_answer(entry("zork_draft", &["xyzzy plugh frobozz", "plugh xyzzy", "frobozz zork"], false))
        .await
        .unwrap();
    let draft = svc.ask_route("xyzzy plugh frobozz", Lang::En).await.unwrap();
    // Черновик узнаёт свой вопрос, но ответа у него нет: это не «цен нет» соседа.
    assert!(matches!(draft.decision, "unanswered" | "none"), "{}", draft.decision);
    assert!(draft.answer.as_ref().map_or(true, |a| a.key == "zork_draft" && a.text.is_empty()));

    // Включили — запись подбирается (с моделью — ответом, без неё — предложением).
    let mut on = svc.admin_list_ask_answers().await.unwrap().into_iter().find(|e| e.key == "zork_draft").unwrap();
    on.enabled = true;
    svc.admin_save_ask_answer(on).await.unwrap();
    let route = svc.ask_route("xyzzy plugh frobozz", Lang::En).await.unwrap();
    assert!(matches!(route.decision, "answer" | "suggest"), "{}", route.decision);
    let first = match (&route.answer, route.candidates.first()) {
        (Some(a), _) => a.key.clone(),
        (_, Some(c)) => c.key.clone(),
        _ => panic!("nothing offered"),
    };
    assert_eq!(first, "zork_draft");
}

#[sqlx::test]
async fn a_question_in_the_reader_language_gets_the_answer_in_that_language(pool: PgPool) {
    let svc = service(&pool);
    let route = svc.ask_route("How do I unsubscribe from the newsletter?", Lang::Ru).await.unwrap();
    if let Some(a) = route.answer {
        assert_eq!(a.key, "unsubscribe");
        assert!(a.text.contains("рассылки"), "{}", a.text);
    } else {
        let c = route.candidates.first().expect("at least a suggestion");
        assert_eq!(c.key, "unsubscribe");
        assert!(c.question.contains("рассылки"), "the button is in Russian: {}", c.question);
    }
}

#[sqlx::test]
async fn a_question_that_belongs_to_a_draft_is_not_given_to_a_neighbour(pool: PgPool) {
    let svc = service(&pool);
    // `lead_time` из миграции — черновик без ответа.
    let route = svc.ask_route("How long does it take to make a work?", Lang::En).await.unwrap();
    assert_ne!(route.decision, "answer", "no neighbour may answer for a draft");
    for c in &route.candidates {
        assert_ne!(c.key, "lead_time", "a draft is never offered as a button");
    }
}

#[sqlx::test]
async fn a_blank_or_one_letter_question_gets_nothing(pool: PgPool) {
    let svc = service(&pool);
    for q in ["", "   ", "a"] {
        assert_eq!(svc.ask_route(q, Lang::En).await.unwrap().decision, "none", "{q:?}");
    }
}

#[sqlx::test]
async fn a_question_printed_on_a_button_is_answered_by_its_own_entry(pool: PgPool) {
    let svc = service(&pool);
    for q in ["Как выиграть бой?", "как выиграть бой", "How do I win a battle?"] {
        let route = svc.ask_route(q, Lang::Ru).await.unwrap();
        assert_eq!(route.decision, "answer", "{q:?}");
        assert_eq!(route.answer.expect("answer").key, "play_win", "{q:?}");
    }
}

#[sqlx::test]
async fn every_printed_question_and_phrase_reaches_its_own_entry(pool: PgPool) {
    let svc = service(&pool);
    let entries = svc.admin_list_ask_answers().await.unwrap();
    assert!(entries.len() > 20);
    for e in &entries {
        for p in gotiga_server::ask::index_phrases(e) {
            let route = svc.ask_route(&p, Lang::En).await.unwrap();
            if e.enabled {
                assert_eq!(route.decision, "answer", "{p:?} of {}", e.key);
                assert_eq!(route.answer.expect("answer").key, e.key, "{p:?}");
            } else {
                assert_eq!(route.decision, "unanswered", "{p:?} of {}", e.key);
            }
        }
    }
}

#[sqlx::test]
async fn the_migration_gives_every_entry_a_topic_and_the_list_carries_it(pool: PgPool) {
    let svc = service(&pool);
    for e in svc.admin_list_ask_answers().await.unwrap() {
        assert!(gotiga_server::ask::TOPICS.contains(&e.topic.as_str()), "{}: {}", e.key, e.topic);
    }
    let list = svc.ask_questions(Lang::Ru).await.unwrap();
    assert!(list.len() > 20);
    let win = list.iter().find(|q| q.key == "play_win").expect("play_win");
    assert_eq!(win.topic, "play");
    assert_eq!(win.pages, vec!["play"]);
    assert_eq!(win.question, "Как выиграть бой?");
    assert!(win.phrases.iter().any(|p| p == "How do I win a battle?"));
    // Черновики в список не попадают: на кнопку нужен ответ.
    assert!(list.iter().all(|q| q.key != "lead_time"));
}

#[sqlx::test]
async fn topic_and_pages_are_saved_and_come_back(pool: PgPool) {
    let svc = service(&pool);
    let mut e = svc.admin_list_ask_answers().await.unwrap().into_iter().find(|e| e.key == "size").unwrap();
    e.topic = "works".into();
    e.pages = vec!["work".into(), "tale".into()];
    let saved = svc.admin_save_ask_answer(e).await.unwrap();
    assert_eq!(saved.pages, vec!["work", "tale"]);
    let again = svc.admin_list_ask_answers().await.unwrap().into_iter().find(|e| e.key == "size").unwrap();
    assert_eq!(again.pages, vec!["work", "tale"]);
    assert_eq!(again.topic, "works");
}

#[sqlx::test]
async fn every_switched_on_entry_speaks_six_languages(pool: PgPool) {
    let svc = service(&pool);
    let entries = svc.admin_list_ask_answers().await.unwrap();
    let on: Vec<_> = entries.iter().filter(|e| e.enabled).collect();
    assert!(on.len() >= 45, "{} entries on", on.len());
    for e in on {
        let q = &e.question;
        for (lang, text) in [("en", &q.en), ("ru", &q.ru), ("uk", &q.uk), ("es", &q.es), ("de", &q.de), ("it", &q.it)] {
            assert!(!text.is_empty(), "{}: question in {lang}", e.key);
        }
        if e.handler.is_none() {
            let a = &e.answer;
            for (lang, text) in [("en", &a.en), ("ru", &a.ru), ("uk", &a.uk), ("es", &a.es), ("de", &a.de), ("it", &a.it)] {
                assert!(!text.is_empty(), "{}: answer in {lang}", e.key);
            }
        }
        for l in &e.links {
            let t = &l.label;
            for (lang, text) in [("en", &t.en), ("ru", &t.ru), ("uk", &t.uk), ("es", &t.es), ("de", &t.de), ("it", &t.it)] {
                assert!(!text.is_empty(), "{}: link {} in {lang}", e.key, l.href);
            }
        }
        assert!(gotiga_server::ask::TOPICS.contains(&e.topic.as_str()), "{}", e.key);
    }
    // Темы, в которых теперь есть вопросы: «Фото и права» и «Студия» больше не пустые.
    let list = svc.ask_questions(Lang::De).await.unwrap();
    for topic in ["rights", "studio", "works", "order", "site", "stories", "play"] {
        assert!(list.iter().any(|q| q.topic == topic), "no question in {topic}");
    }
    let sheet = list.iter().find(|q| q.key == "photo_share").expect("photo_share");
    assert_eq!(sheet.question, "Darf ich eure Fotos in sozialen Netzwerken teilen?");
}

#[sqlx::test]
async fn running_the_second_batch_again_keeps_what_the_author_wrote(pool: PgPool) {
    use sqlx::Executor;
    let svc = service(&pool);
    // Автор правит немецкий ответ и ссылку, и стирает итальянский вопрос.
    let mut price = svc.admin_list_ask_answers().await.unwrap().into_iter().find(|e| e.key == "price").unwrap();
    price.answer.de = "Mein eigener Text".into();
    price.question.it = String::new();
    let before_links = price.links.len();
    price.links[0].href = "/figurines".into();
    svc.admin_save_ask_answer(price).await.unwrap();

    let sql = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations/20261023000001_ask_more_answers.sql"),
    )
    .unwrap();
    pool.execute(sql.as_str()).await.unwrap();

    let again = svc.admin_list_ask_answers().await.unwrap();
    let price = again.iter().find(|e| e.key == "price").unwrap();
    assert_eq!(price.answer.de, "Mein eigener Text", "an edited language must stay");
    assert!(!price.question.it.is_empty(), "an emptied language is filled again");
    assert_eq!(price.links.len(), before_links);
    assert_eq!(price.links[0].href, "/figurines", "edited links are left alone");
    assert_eq!(again.iter().filter(|e| e.key == "photo_share").count(), 1, "no duplicates");
}
