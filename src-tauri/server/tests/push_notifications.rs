//! Уведомления в браузере: подписки и журнал рассылки.
//!
//! Стенд локальный (`PUBLIC_URL` на 127.0.0.1), поэтому уведомление никуда не уходит, а
//! рассылка печатает в журнал и отмечает вещь ушедшей — тот же путь, что на рабочем сервере,
//! кроме самого запроса к узлу рассылки. Сеть тут не нужна и не используется.
//! Проверяется то, что решает база: кому и когда слать, и что принимает подписка.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::{PushKeys, PushSubscribeRequest};
use gotiga_server::services::AppService;
use ring::{
    agreement,
    rand::{SecureRandom, SystemRandom},
};
use sqlx::PgPool;
use uuid::Uuid;

const FCM: &str = "https://fcm.googleapis.com/fcm/send/";

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-push-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-push-logs-{}.sqlite", Uuid::new_v4()),
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

/// Настоящая пара ключей подписчика: открытая точка на кривой и секрет `auth`.
fn keys() -> PushKeys {
    let rng = SystemRandom::new();
    let private = agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng).unwrap();
    let public = private.compute_public_key().unwrap();
    let mut auth = [0u8; 16];
    rng.fill(&mut auth).unwrap();
    PushKeys { p256dh: B64.encode(public.as_ref()), auth: B64.encode(auth) }
}

fn request(endpoint: &str, lang: &str, topics: &[&str], old: Option<&str>) -> PushSubscribeRequest {
    PushSubscribeRequest {
        endpoint: endpoint.to_string(),
        keys: keys(),
        lang: lang.to_string(),
        topics: topics.iter().map(|t| t.to_string()).collect(),
        old_endpoint: old.map(str::to_string),
    }
}

async fn row(pool: &PgPool, endpoint: &str) -> Option<(String, Vec<String>)> {
    sqlx::query_as("SELECT lang, topics FROM push_subscriptions WHERE endpoint = $1")
        .bind(endpoint)
        .fetch_optional(pool)
        .await
        .expect("подписка")
}

async fn seed_work(pool: &PgPool, name: &str, visible: bool) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO figurines (name, short_text, is_visible, status)
         VALUES ($1, 'Бронза и воск', $2, 'available') RETURNING id",
    )
    .bind(name)
    .bind(visible)
    .fetch_one(pool)
    .await
    .expect("завести работу")
}

async fn seed_card(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO battle_cards (slug, status, tier, title_en, title_ru, cost, power, health, price_dust)
         VALUES ($1, 'published', 1, 'Ghost', 'Призрак', 1, 2, 2, 10) RETURNING id",
    )
    .bind(slug)
    .fetch_one(pool)
    .await
    .expect("завести карту")
}

async fn seed_battle(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO battle_challenges (slug, title_en, title_ru, setup, status)
         VALUES ($1, 'Gate', 'Ворота', '{}', 'published') RETURNING id",
    )
    .bind(slug)
    .fetch_one(pool)
    .await
    .expect("завести битву")
}

async fn setting(pool: &PgPool, key: &str) -> Option<String> {
    sqlx::query_scalar("SELECT value FROM settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .expect("настройка")
}

/// Состояние строки журнала: ушла ли вещь и сколько раз пытались.
async fn ledger(pool: &PgPool, id: Uuid) -> Option<(bool, i32)> {
    sqlx::query_as("SELECT sent_at IS NOT NULL, attempts FROM push_posts WHERE target_id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .expect("журнал")
}

async fn age(pool: &PgPool, id: Uuid, hours: i32) {
    sqlx::query("UPDATE push_posts SET first_seen_at = NOW() - make_interval(hours => $2) WHERE target_id = $1")
        .bind(id)
        .bind(hours)
        .execute(pool)
        .await
        .expect("сдвинуть время");
}

/// Чужие узлы и выдуманные ключи подписка не принимает.
#[sqlx::test]
async fn subscription_takes_only_push_services_and_real_keys(pool: PgPool) {
    let svc = service(&pool);
    for bad in [
        "http://fcm.googleapis.com/x",
        "https://evil.example/fcm.googleapis.com",
        "https://127.0.0.1/x",
        "https://fcm.googleapis.com@evil.example/x",
    ] {
        let err = svc.push_subscribe(request(bad, "en", &["works"], None), "1.1.1.1").await;
        assert!(err.is_err(), "{bad}");
    }
    // Длина и первый байт верны, но точки на кривой нет.
    let mut fake = request(&format!("{FCM}fake"), "en", &["works"], None);
    let mut point = vec![4u8];
    point.extend_from_slice(&[7u8; 64]);
    fake.keys.p256dh = B64.encode(point);
    assert!(svc.push_subscribe(fake, "1.1.1.1").await.is_err());
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM push_subscriptions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "ничего не записано");
}

/// Без тем подписки нет; тема не из списка не считается.
#[sqlx::test]
async fn at_least_one_known_topic_is_required(pool: PgPool) {
    let svc = service(&pool);
    let e = format!("{FCM}a");
    assert!(svc.push_subscribe(request(&e, "en", &[], None), "1.1.1.1").await.is_err());
    assert!(svc.push_subscribe(request(&e, "en", &["games"], None), "1.1.1.1").await.is_err());
    svc.push_subscribe(request(&e, "ru", &["tales", "games"], None), "1.1.1.1")
        .await
        .expect("подписка");
    assert_eq!(row(&pool, &e).await, Some(("ru".into(), vec!["tales".into()])));
}

/// Браузер сам сменил подписку: Service Worker не знает ни тем, ни языка, и они переезжают
/// из прежней записи, а прежняя стирается.
#[sqlx::test]
async fn a_replaced_subscription_inherits_topics_and_language(pool: PgPool) {
    let svc = service(&pool);
    let old = format!("{FCM}old");
    let new = format!("{FCM}new");
    svc.push_subscribe(request(&old, "de", &["works", "gazette"], None), "1.1.1.1")
        .await
        .unwrap();
    svc.push_subscribe(request(&new, "", &[], Some(&old)), "1.1.1.1")
        .await
        .expect("замена");
    assert_eq!(row(&pool, &old).await, None, "прежняя стёрта");
    assert_eq!(
        row(&pool, &new).await,
        Some(("de".into(), vec!["works".into(), "gazette".into()]))
    );
    // Прежней записи нет и тем в запросе нет — принимать нечего.
    let lost = format!("{FCM}lost");
    let nothing = format!("{FCM}nothing");
    assert!(svc.push_subscribe(request(&nothing, "", &[], Some(&lost)), "1.1.1.1").await.is_err());
}

/// Предел «новых подписок в час» — на адрес, но обновления известных ему не подчиняются:
/// за общим адресом они иначе оставляли бы других без подписки.
#[sqlx::test]
async fn the_hourly_limit_counts_new_subscriptions_only(pool: PgPool) {
    let svc = service(&pool);
    let ip = "10.0.0.1";
    for i in 0..30 {
        svc.push_subscribe(request(&format!("{FCM}{i}"), "en", &["works"], None), ip)
            .await
            .expect("в пределе");
    }
    assert!(
        svc.push_subscribe(request(&format!("{FCM}31"), "en", &["works"], None), ip).await.is_err(),
        "тридцать первая новая — отказ"
    );
    // Известная подписка обновляется сколько угодно: язык и темы меняются.
    for _ in 0..40 {
        svc.push_subscribe(request(&format!("{FCM}0"), "uk", &["tales"], None), ip)
            .await
            .expect("обновление");
    }
    assert_eq!(row(&pool, &format!("{FCM}0")).await, Some(("uk".into(), vec!["tales".into()])));
    // Другой адрес не затронут.
    svc.push_subscribe(request(&format!("{FCM}other"), "en", &["works"], None), "10.0.0.2")
        .await
        .expect("другой адрес");
}

/// Архив, стоявший до подключения, не уходит; новое ждёт выдержку и уходит один раз.
#[sqlx::test]
async fn the_archive_stays_quiet_and_new_work_waits_for_the_grace(pool: PgPool) {
    let old = seed_work(&pool, "Старый страж", true).await;
    let svc = service(&pool);

    assert_eq!(svc.announce_news().await.expect("подключение"), (0, 0));
    assert_eq!(ledger(&pool, old).await, Some((true, 0)), "архив отмечен ушедшим");

    let fresh = seed_work(&pool, "Новый страж", true).await;
    svc.announce_news().await.expect("тик");
    assert_eq!(ledger(&pool, fresh).await, Some((false, 0)), "замечена, выдержка не прошла");

    // Подписчик есть; выдержка прошла. На стенде уведомление не уходит, но вещь отмечена.
    svc.push_subscribe(request(&format!("{FCM}reader"), "ru", &["works"], None), "1.1.1.1")
        .await
        .unwrap();
    age(&pool, fresh, 1).await;
    svc.announce_news().await.expect("тик");
    assert_eq!(ledger(&pool, fresh).await, Some((true, 0)));
    assert_eq!(ledger(&pool, old).await, Some((true, 0)));
}

/// Снятая с сайта вещь остаётся ждать, но не занимает места новым: три самые старые такие
/// не должны закрывать дорогу четвёртой.
#[sqlx::test]
async fn hidden_waiting_items_do_not_starve_new_ones(pool: PgPool) {
    let svc = service(&pool);
    svc.announce_news().await.expect("подключение");

    let mut hidden = Vec::new();
    for i in 0..3 {
        hidden.push(seed_work(&pool, &format!("Скрытая {i}"), true).await);
    }
    let fresh = seed_work(&pool, "Новая", true).await;
    svc.announce_news().await.expect("тик: замечены");
    for id in &hidden {
        age(&pool, *id, 5).await;
    }
    age(&pool, fresh, 1).await;
    for id in &hidden {
        sqlx::query("UPDATE figurines SET is_visible = FALSE WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }

    svc.announce_news().await.expect("тик");
    assert_eq!(ledger(&pool, fresh).await, Some((true, 0)), "новая прошла");
    for id in &hidden {
        assert_eq!(ledger(&pool, *id).await, Some((false, 0)), "скрытые ждут");
    }
}

/// Вещь, замеченная давно и не ушедшая (сервер лежал), уже не новость.
#[sqlx::test]
async fn a_stale_item_is_dropped_not_announced(pool: PgPool) {
    let svc = service(&pool);
    svc.announce_news().await.expect("подключение");
    let work = seed_work(&pool, "Забытая", true).await;
    svc.announce_news().await.expect("тик");
    svc.push_subscribe(request(&format!("{FCM}reader"), "en", &["works"], None), "1.1.1.1")
        .await
        .unwrap();
    age(&pool, work, 24 * 4).await;
    assert_eq!(svc.announce_news().await.expect("тик"), (0, 0));
    assert_eq!(ledger(&pool, work).await, Some((true, 0)), "отмечена без отправки");
}

/// Попытки доставить вещь считаются в журнале.
#[sqlx::test]
async fn failed_attempts_are_counted_in_the_ledger(pool: PgPool) {
    let repo = Repository::new(pool.clone());
    let id = Uuid::new_v4();
    repo.push_note_seen("work", &[id], false).await.unwrap();
    assert_eq!(repo.push_attempt("work", id).await.unwrap(), 1);
    assert_eq!(repo.push_attempt("work", id).await.unwrap(), 2);
    assert_eq!(ledger(&pool, id).await, Some((false, 2)));
}

/// Четвёртая тема «Игра» принимается как есть; подписка без неё её не получает.
#[sqlx::test]
async fn the_game_topic_is_accepted_and_never_added_by_itself(pool: PgPool) {
    let svc = service(&pool);
    let new = format!("{FCM}new");
    svc.push_subscribe(request(&new, "en", &["works", "game"], None), "1.1.1.1")
        .await
        .expect("подписка с игрой");
    assert_eq!(row(&pool, &new).await, Some(("en".into(), vec!["works".into(), "game".into()])));
    // Старая подписка (до темы) остаётся при своих; обновление записи игры ей не добавляет.
    let old = format!("{FCM}old");
    svc.push_subscribe(request(&old, "de", &["works", "tales", "gazette"], None), "1.1.1.1")
        .await
        .unwrap();
    svc.push_subscribe(request(&old, "de", &["works", "tales", "gazette"], None), "1.1.1.1")
        .await
        .unwrap();
    assert_eq!(
        row(&pool, &old).await,
        Some(("de".into(), vec!["works".into(), "tales".into(), "gazette".into()]))
    );
}

/// Рассылка работала до темы «Игра»: карты и битвы, уже стоящие на сайте, не уходят;
/// новая карта и новая битва — уходят, и только после выдержки.
#[sqlx::test]
async fn cards_and_battles_already_on_the_site_are_not_announced(pool: PgPool) {
    let svc = service(&pool);
    // Рассылка уже подключена до выкладки темы: ключ игры ещё не стоит.
    let work = seed_work(&pool, "Старый страж", true).await;
    svc.announce_news().await.expect("подключение");
    sqlx::query("DELETE FROM settings WHERE key = 'push_game_since'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(ledger(&pool, work).await, Some((true, 0)));

    let old_card = seed_card(&pool, "staraya").await;
    let old_battle = seed_battle(&pool, "staraya-bitva").await;
    svc.push_subscribe(request(&format!("{FCM}reader"), "ru", &["game"], None), "1.1.1.1")
        .await
        .unwrap();

    // Первый проход с темой: старое отмечено ушедшим, ключ поставлен.
    assert_eq!(svc.announce_news().await.expect("тик"), (0, 0));
    assert_eq!(ledger(&pool, old_card).await, Some((true, 0)), "старая карта");
    assert_eq!(ledger(&pool, old_battle).await, Some((true, 0)), "старая битва");
    assert!(setting(&pool, "push_game_since").await.is_some());

    // Новое замечается и ждёт выдержку, затем уходит.
    let new_card = seed_card(&pool, "novaya").await;
    let new_battle = seed_battle(&pool, "novaya-bitva").await;
    svc.announce_news().await.expect("тик");
    assert_eq!(ledger(&pool, new_card).await, Some((false, 0)));
    assert_eq!(ledger(&pool, new_battle).await, Some((false, 0)));
    age(&pool, new_card, 1).await;
    age(&pool, new_battle, 1).await;
    svc.announce_news().await.expect("тик");
    assert_eq!(ledger(&pool, new_card).await, Some((true, 0)));
    assert_eq!(ledger(&pool, new_battle).await, Some((true, 0)));
}

/// Игру получают только подписанные на неё: подписчик без темы «Игра» вещь не задерживает
/// и не получает, а вещь отмечается ушедшей.
#[sqlx::test]
async fn a_card_goes_only_to_the_game_topic(pool: PgPool) {
    let svc = service(&pool);
    svc.announce_news().await.expect("подключение");
    svc.push_subscribe(request(&format!("{FCM}works"), "en", &["works"], None), "1.1.1.1")
        .await
        .unwrap();
    let card = seed_card(&pool, "tolko-igre").await;
    svc.announce_news().await.expect("тик");
    age(&pool, card, 1).await;
    svc.announce_news().await.expect("тик");
    assert_eq!(ledger(&pool, card).await, Some((true, 0)), "получателей нет, вещь закрыта");
}
