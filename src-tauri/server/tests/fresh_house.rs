//! Пустой дом: что видит первый человек, пришедший туда, где ещё ничего нет.
//!
//! Всё, что построено, проверялось на доме, где уже есть карты, рамы, сезоны и
//! люди. Первый гость встречает другое: полка пуста, библиотека дома пуста,
//! сезона нет ни одного, одолжить нечего, у автора нет адреса. Пустое
//! состояние — это не «то же самое, только меньше»: именно на нём ломаются
//! запросы, которые молча полагаются на «хоть что-то есть».
//!
//! Проверяется НЕ «функция вернула пустой список», а то, что дом на пустом
//! месте ОТВЕЧАЕТ, а не падает и не молчит.

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
        upload_dir: format!("/tmp/gotiga-fresh-uploads-{}", Uuid::new_v4()),
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
        telegram_chat_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-fresh-logs-{}.sqlite", Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".into(),
        auth_pepper: "pepper-for-tests-0123456789".into(),
        auth_pepper_old: None,
    }
}

async fn service(pool: &PgPool) -> AppService {
    AppService::new(Repository::new(pool.clone()), config())
}

async fn seed_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (email, display_name, visual_password_hash)
         VALUES ($1, 'Первый', 'x') RETURNING id",
    )
    .bind(format!("fresh-{}@example.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("завести человека")
}

/// Всё, что видно БЕЗ ИМЕНИ, на доме, где нет ничего.
///
/// Это те самые страницы, на которые приходят из соцсетей, и первая из них
/// открывается раньше, чем в доме появится хоть одна работа.
#[sqlx::test]
async fn the_open_rooms_answer_on_an_empty_house(pool: PgPool) {
    let svc = service(&pool).await;

    assert!(
        svc.list_battle_cards_public().await.expect("полка").is_empty(),
        "полка пуста и отвечает"
    );
    let gallery = svc.studio_gallery(0).await.expect("галерея");
    assert!(gallery.works.is_empty() && gallery.total == 0);
    assert!(svc.market().await.expect("лавка").is_empty());
    assert!(
        svc.studio_library(None).await.expect("библиотека").is_empty(),
        "библиотека дома пуста — новичку в сборщике выбирать не из чего, и \
         сказать об этом должна страница, а не пустой ответ"
    );

    // Страница автора, которого нет, — это «нет такого», а не поломка.
    assert!(svc.studio_author("nikto").await.is_err());
}

/// Сезон заводится САМ при первом обращении.
///
/// Иначе первая неделя студии зависит от того, что кто-то вовремя нажал
/// кнопку, — а нажать её некому: людей ещё нет.
#[sqlx::test]
async fn the_first_season_opens_itself(pool: PgPool) {
    let svc = service(&pool).await;
    let page = svc.studio_season_page(None, None).await.expect("сезон");
    assert_eq!(page.season.number, 1, "первый сезон — первый");
    assert_eq!(page.season.state, "open");
    assert!(page.entries.is_empty(), "и в нём пока никого");

    // Второе обращение не заводит второго сезона.
    let again = svc.studio_season_page(None, None).await.expect("сезон");
    assert_eq!(again.season.id, page.season.id);
}

/// Первый человек в пустом доме: ни карт, ни пыли, ни стола.
#[sqlx::test]
async fn the_first_person_finds_a_room_that_answers(pool: PgPool) {
    let svc = service(&pool).await;
    let who = seed_user(&pool).await;

    let me = svc.battle_me(who).await.expect("книга");
    assert_eq!((me.dust, me.feed), (0, 0));
    assert!(me.owned.is_empty() && me.gifts.is_empty());

    // Стол: колоды нет, и дом НЕ МОЖЕТ ничего одолжить — заёмных карт в доме
    // не заведено. Это самое опасное пустое место в комнате: партия просит
    // шести тел, и до заёма стол был запертой дверью ровно для того, кому он
    // нужнее всего.
    let table = svc.read_battle_deck(who).await.expect("стол");
    assert!(!table.laid, "стол ещё не раскладывали");
    assert!(
        table.nothing_to_lend,
        "дому нечего одолжить, и комната говорит это ВСЛУХ, а не рисует \
         пустые места без объяснения"
    );
}

/// Студия на пустом доме: ворота дома пускают того, у кого есть карты, — а
/// карт в доме нет ни у кого. Дверь заперта ЧЕСТНО, словом, а не ошибкой.
#[sqlx::test]
async fn the_studio_door_is_shut_politely_when_nobody_owns_anything(pool: PgPool) {
    let svc = service(&pool).await;
    let who = seed_user(&pool).await;

    // Умолчание дома — `owners`, и это осознанный порядок: сперва поиграй,
    // потом твори. На пустом доме он значит «студия закрыта для всех».
    let state = svc.studio_state(who).await.expect("состояние студии");
    assert_eq!(state.gate, "owners");
    assert!(!state.open, "дверь заперта");
    assert!(state.frames.is_empty());
    assert_eq!(state.r#box.used, 0);
    // Настройки читаются умолчаниями: их никто не сохранял.
    assert_eq!(state.settings.cards_open, 10);

    // И всякое действие за этой дверью отвечает словом, а не паникой.
    assert!(svc.studio_cards(who).await.is_err());
}
