//! Объявления в Telegram-канале.
//!
//! Стенд здесь локальный (`PUBLIC_URL` на 127.0.0.1), поэтому пост не уходит в
//! Telegram, а печатается в журнал и отмечается вышедшим, — ровно тот путь,
//! которым канал ходит на рабочем сервере, кроме самого вызова бота.
//! Проверяется журнал: что выходит, когда и сколько раз.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::lang::Lang;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config(channel: Option<&str>, extra: &[(Lang, &str)]) -> Config {
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
        telegram_channel_extra: extra.iter().map(|(l, c)| (*l, c.to_string())).collect(),
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
        google_client_id: None,
        google_client_secret: None,
        apple_client_id: None,
        apple_team_id: None,
        apple_key_id: None,
        apple_private_key: None,
        auth_pepper_old: None,
    }
}

fn service(pool: &PgPool, channel: Option<&str>) -> AppService {
    AppService::new(Repository::new(pool.clone()), config(channel, &[]))
}

fn service_with(pool: &PgPool, channel: Option<&str>, extra: &[(Lang, &str)]) -> AppService {
    AppService::new(Repository::new(pool.clone()), config(channel, extra))
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

async fn lang_rows(pool: &PgPool, id: Uuid) -> Vec<(String, bool, bool)> {
    sqlx::query_as(
        "SELECT lang, skipped, posted_at IS NOT NULL FROM telegram_channel_posts
          WHERE target_id = $1 ORDER BY lang",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .expect("журнал")
}

/// У каждого языка свой канал и своя строка журнала: вещь выходит в оба, и
/// сбой одного не мешает другому.
#[sqlx::test]
async fn every_channel_has_its_own_ledger_row(pool: PgPool) {
    let old = seed_work(&pool, "Старый страж", "available").await;
    let svc = service_with(&pool, Some("@ritunia_en"), &[(Lang::De, "@ritunia_de")]);
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 0);
    assert_eq!(
        lang_rows(&pool, old).await,
        vec![("de".to_string(), true, false), ("en".to_string(), true, false)],
        "архив пропущен в обоих каналах"
    );

    let fresh = seed_work(&pool, "Новый страж", "available").await;
    svc.announce_to_channel().await.expect("тик");
    let_time_pass(&pool).await;
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 2, "по одному в каждый канал");
    assert_eq!(
        lang_rows(&pool, fresh).await,
        vec![("de".to_string(), false, true), ("en".to_string(), false, true)]
    );
    assert_eq!(svc.announce_to_channel().await.expect("тик"), 0, "второго поста нет");
}

/// Канал, подключённый позже, не получает архив — как и первый в свой день.
#[sqlx::test]
async fn a_channel_added_later_does_not_get_the_archive(pool: PgPool) {
    let en = service(&pool, Some("@ritunia_en"));
    en.announce_to_channel().await.expect("подключение");
    let work = seed_work(&pool, "Страж", "available").await;
    en.announce_to_channel().await.expect("тик");
    let_time_pass(&pool).await;
    assert_eq!(en.announce_to_channel().await.expect("тик"), 1);

    // Спустя время заводят немецкий канал.
    let both = service_with(&pool, Some("@ritunia_en"), &[(Lang::De, "@ritunia_de")]);
    assert_eq!(both.announce_to_channel().await.expect("тик"), 0);
    let_time_pass(&pool).await;
    assert_eq!(both.announce_to_channel().await.expect("тик"), 0);
    assert_eq!(
        lang_rows(&pool, work).await,
        vec![("de".to_string(), true, false), ("en".to_string(), false, true)]
    );
}

fn notes(en: &[&str], de: &[&str]) -> String {
    let side = |items: &[&str]| {
        serde_json::json!({ "new": items, "better": Vec::<&str>::new() })
    };
    let mut map = serde_json::Map::new();
    if !en.is_empty() {
        map.insert("en".into(), side(en));
    }
    if !de.is_empty() {
        map.insert("de".into(), side(de));
    }
    serde_json::Value::Object(map).to_string()
}

/// Черновик релиза: пост на языке канала, список нового с прошлого релиза,
/// отправка один раз.
#[sqlx::test]
async fn a_release_post_goes_out_once_in_every_language(pool: PgPool) {
    let svc = service_with(&pool, Some("@ritunia_en"), &[(Lang::De, "@ritunia_de")]);
    let repo = Repository::new(pool.clone());
    svc.announce_to_channel().await.expect("подключение");

    // Работа вышла после прошлого релиза.
    seed_work(&pool, "Raven", "available").await;
    svc.announce_to_channel().await.expect("тик");

    let since = chrono::Utc::now() - chrono::Duration::hours(1);
    repo.release_create("r1", Some(since), &notes(&["Spanish is on the site"], &["Spanisch ist da"]))
        .await
        .expect("черновик");

    let overview = svc.admin_channel_overview().await.expect("обзор");
    let release = overview.release.expect("черновик виден");
    assert_eq!(release.status, "draft");
    assert_eq!(release.news_count, 1);
    let caption = |lang: &str| {
        release
            .channels
            .iter()
            .find(|c| c.lang == lang)
            .and_then(|c| c.caption.clone())
            .unwrap_or_default()
    };
    assert!(caption("en").contains("Spanish is on the site"), "{}", caption("en"));
    assert!(caption("en").contains("New works: "), "{}", caption("en"));
    assert!(caption("en").contains("Raven"));
    assert!(caption("de").contains("Spanisch ist da"), "{}", caption("de"));
    assert!(caption("de").contains("Neue Werke: "), "{}", caption("de"));
    assert!(!caption("de").contains("Auf Englisch"));

    let sent = svc.admin_release_send("r1", &[]).await.expect("отправка");
    let release = sent.release.expect("релиз");
    assert_eq!(release.status, "sent");
    assert!(release.channels.iter().all(|c| c.posted_at.is_some()));

    // Второй раз и правка после отправки — отказ.
    assert!(svc.admin_release_send("r1", &[]).await.is_err());
    let notes_back: gotiga_server::channel::Notes = serde_json::from_str(&notes(&["x"], &[])).unwrap();
    assert!(svc.admin_release_save_notes("r1", notes_back).await.is_err());
}

/// Без перевода пост о релизе выходит на английском и говорит об этом;
/// без каналов и без пунктов черновика нет и отправлять нечего.
#[sqlx::test]
async fn a_release_without_translation_says_so_and_nothing_means_no_post(pool: PgPool) {
    let svc = service_with(&pool, Some("@ritunia_en"), &[(Lang::De, "@ritunia_de")]);
    let repo = Repository::new(pool.clone());
    svc.announce_to_channel().await.expect("подключение");

    repo.release_create("r2", None, &notes(&["Only English"], &[])).await.unwrap();
    let release = svc.admin_channel_overview().await.unwrap().release.unwrap();
    let de = release.channels.iter().find(|c| c.lang == "de").unwrap();
    assert!(de.caption.as_deref().unwrap().contains("Auf Englisch"));
    assert_eq!(de.written_in.as_deref(), Some("en"));

    repo.release_create("r3", None, "{}").await.unwrap();
    // Свежий черновик пуст: писать нечего.
    sqlx::query("UPDATE release_posts SET created_at = NOW() + INTERVAL '1 minute' WHERE id = 'r3'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(svc.admin_release_send("r3", &[]).await.is_err());
}

/// Новый релиз из файла заметок заводит черновик один раз; без каналов — ни
/// одного.
#[sqlx::test]
async fn the_notes_file_makes_one_draft_per_release(pool: PgPool) {
    let silent = service(&pool, None);
    assert_eq!(silent.prepare_release_draft().await.unwrap(), None);

    let svc = service(&pool, Some("@ritunia_en"));
    let id = svc.prepare_release_draft().await.unwrap().expect("первый запуск заводит черновик");
    assert_eq!(svc.prepare_release_draft().await.unwrap(), None, "повтор: черновик уже есть");
    let release = svc.admin_channel_overview().await.unwrap().release.unwrap();
    assert_eq!(release.id, id);
    assert_eq!(release.status, "draft");
}

/// Читателя зовут в канал на его языке; нет такого — в английский.
#[sqlx::test]
async fn the_tale_page_points_to_the_readers_channel(pool: PgPool) {
    let svc = service_with(&pool, Some("@ritunia_en"), &[(Lang::De, "@ritunia_de")]);
    assert_eq!(svc.tale_doors(Lang::De).await.telegram.as_deref(), Some("https://t.me/ritunia_de"));
    assert_eq!(svc.tale_doors(Lang::En).await.telegram.as_deref(), Some("https://t.me/ritunia_en"));
    assert_eq!(svc.tale_doors(Lang::It).await.telegram.as_deref(), Some("https://t.me/ritunia_en"));
}

/// Повторная публикация идёт во все каналы сразу, мимо журнала: вещь, которую
/// канал пропустил при подключении, выходит и отмечается вышедшей. Скрытой
/// вещи на сайте нет, и канал её не показывает.
#[sqlx::test]
async fn a_repost_goes_to_every_channel_and_skips_what_is_hidden(pool: PgPool) {
    let old = seed_work(&pool, "Старый страж", "available").await;
    let svc = service_with(&pool, Some("@ritunia_en"), &[(Lang::De, "@ritunia_de")]);
    svc.announce_to_channel().await.expect("подключение");
    assert_eq!(
        lang_rows(&pool, old).await,
        vec![("de".to_string(), true, false), ("en".to_string(), true, false)]
    );

    let result = svc.admin_channel_repost("work", old).await.expect("повтор");
    assert!(result.local);
    let langs: Vec<_> = result.results.iter().map(|r| (r.lang.as_str(), r.status.as_str())).collect();
    assert_eq!(langs.len(), 2);
    assert!(langs.iter().all(|(_, s)| *s == "local"), "{langs:?}");
    assert_eq!(
        lang_rows(&pool, old).await,
        vec![("de".to_string(), true, true), ("en".to_string(), true, true)]
    );

    sqlx::query("UPDATE figurines SET is_visible = FALSE WHERE id = $1")
        .bind(old)
        .execute(&pool)
        .await
        .unwrap();
    let result = svc.admin_channel_repost("work", old).await.expect("повтор");
    assert!(result.results.iter().all(|r| r.status == "not_on_display"));

    assert!(svc.admin_channel_repost("nonsense", old).await.is_err());
}
