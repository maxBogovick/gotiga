//! Своя карта человека: стол → хозяин → полка дома.
//!
//! Здесь проверяется то, что бывает ТОЛЬКО НА СТЫКЕ: что домовое поле не
//! доезжает до карты, что отданную нельзя переписать, что утверждение пишет
//! настоящую карту тем же путём, каким её пишет стол хозяина, и что второе
//! нажатие не заводит второй.
//!
//! Вырезание домовых полей само по себе проверено без базы в `studio.rs` —
//! дублировать его тут незачем. Здесь про ПУТЬ.
//!
//! Требует `DATABASE_URL` и права CREATE DATABASE.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::ApproveStudioCardRequest;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-cards-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-cards-logs-{}.sqlite", Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".into(),
        auth_pepper: "pepper-for-tests-0123456789".into(),
        auth_pepper_old: None,
    }
}

async fn service(pool: &PgPool) -> AppService {
    AppService::new(Repository::new(pool.clone()), config())
}

async fn seed_user(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (email, display_name, visual_password_hash)
         VALUES ($1, $2, 'x') RETURNING id",
    )
    .bind(format!("cards-{}@example.test", Uuid::new_v4()))
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("завести человека")
}

async fn open_the_gate(svc: &AppService) {
    let mut settings = svc.get_studio_settings().await.expect("настройки");
    settings.gate = "all".into();
    svc.save_studio_settings(settings).await.expect("сохранить");
}

/// Годная карта: числа такие, что весы её пропускают, и есть всё, чего дом
/// требует от опубликованной (имя на двух языках, эффект, здоровье).
fn a_fair_card(name: &str) -> serde_json::Value {
    serde_json::json!({
        "status": "draft",
        "tier": 1,
        "titleEn": name,
        "titleRu": name,
        "effectEn": "A card made by a person.",
        "effectRu": "Карта, сделанная человеком.",
        "cost": 1,
        "power": 3,
        "health": 6,
        "mana": 0,
        "traits": [],
        "kind": "unit",
        "armor": 0,
        "ward": 0,
        "attackChannel": "physical",
        "reach": 1,
        "step": 1,
        "speed": 3,
        "mend": 0,
        "abilities": [],
    })
}

/// Карта, доведённая до «отдана хозяину»: соглашение автора включено — без него
/// работа из рук не уходит.
async fn a_shown_card(svc: &AppService, owner: Uuid, name: &str) -> Uuid {
    let work = svc
        .save_studio_card(owner, None, &a_fair_card(name), "ru")
        .await
        .expect("завести работу");
    svc.accept_studio_agreement(owner).await.expect("соглашение");
    svc.show_studio_card(owner, work.id).await.expect("отдать");
    work.id
}

fn keeper_word() -> ApproveStudioCardRequest {
    ApproveStudioCardRequest {
        slug: Some("chelovekova".into()),
        tier: None,
        price_dust: Some(10),
        price_feed: None,
        level_price_dust: None,
        edition_size: None,
        lendable: false,
        frame_override: None,
        motion_wear: None,
        art_url: None,
        figurine_id: None,
        credit_name: None,
        status: "published".into(),
        reward_dust: None,
    }
}

/// Домовое поле, присланное человеком, до карты не доезжает.
///
/// Проверяется не «функция вырезала ключ» — это проверено без базы, — а то,
/// ради чего вырезание есть: человек, пославший запрос МИМО страницы со своей
/// ценой и своим слугом, не получает ни того, ни другого. Цену и слуг
/// назначает хозяин, и только он.
#[sqlx::test]
async fn what_belongs_to_the_house_never_arrives_from_a_person(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;

    let mut sneaky = a_fair_card("Даровая");
    sneaky["priceDust"] = serde_json::json!(1);
    sneaky["slug"] = serde_json::json!("darovaya");
    sneaky["lendable"] = serde_json::json!(true);
    sneaky["status"] = serde_json::json!("published");

    let work = svc
        .save_studio_card(author, None, &sneaky, "ru")
        .await
        .expect("завести работу");
    // Тело хранится РАЗБОРОМ в запрос карты и обратно, поэтому домовые поля в
    // нём не отсутствуют, а ПУСТЫ, — и это ровно то, что нужно: присланное
    // человеком не сохранилось.
    assert!(work.body["priceDust"].is_null(), "цена не его дело");
    assert!(work.body["slug"].is_null(), "слуг назначает дом");
    // `lendable` и `status` домовые тоже, и в теле их быть не должно; то, что
    // разбор запроса подставит им умолчания, — не то же самое, что принять
    // присланное.
    assert_eq!(work.body["lendable"], serde_json::json!(false));
    assert_eq!(work.body["status"], serde_json::json!("draft"));

    svc.accept_studio_agreement(author).await.expect("соглашение");
    svc.show_studio_card(author, work.id).await.expect("отдать");
    let made = svc
        .approve_studio_card(work.id, keeper_word())
        .await
        .expect("утвердить");
    assert_eq!(made.slug, "chelovekova", "слуг тот, что назвал хозяин");
    assert_eq!(made.price_dust, Some(10), "и цена тоже");
    assert!(!made.lendable, "заём человек себе не назначил");
}

/// Отданную работу автор не правит, снятую — правит.
///
/// Иначе хозяин одобрял бы одно, а на полку вставало бы другое: самая дорогая
/// ошибка этой комнаты.
#[sqlx::test]
async fn a_work_given_away_is_not_the_authors_to_change(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let work = svc
        .save_studio_card(author, None, &a_fair_card("Первая"), "ru")
        .await
        .expect("завести");
    svc.accept_studio_agreement(author).await.expect("соглашение");
    svc.show_studio_card(author, work.id).await.expect("отдать");
    let changed = a_fair_card("Подменённая");
    assert!(
        svc.save_studio_card(author, Some(work.id), &changed, "ru")
            .await
            .is_err(),
        "отданная не правится"
    );

    svc.withdraw_studio_card(author, work.id).await.expect("снять");
    let mended = svc
        .save_studio_card(author, Some(work.id), &changed, "ru")
        .await
        .expect("снятая правится");
    assert_eq!(mended.body["titleRu"], "Подменённая");
}

/// Перевешенную карту нельзя ни отдать, ни утвердить.
///
/// Отказ не вкусовой: весы у человека и у хозяина — ОДНА функция, и она же
/// стоит на пути публикации. Хозяин, нажавший «утвердить» не глядя, баланс не
/// сломает.
#[sqlx::test]
async fn a_card_that_outweighs_its_rank_goes_nowhere(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;

    let mut heavy = a_fair_card("Тяжёлая");
    heavy["power"] = serde_json::json!(99);
    heavy["health"] = serde_json::json!(99);
    let work = svc
        .save_studio_card(author, None, &heavy, "ru")
        .await
        .expect("завести");

    assert!(
        svc.show_studio_card(author, work.id).await.is_err(),
        "перевешенную не отдать"
    );

    // И даже если она каким-то путём оказалась отданной, приём её не выложит.
    sqlx::query("UPDATE studio_cards SET status = 'shown' WHERE id = $1")
        .bind(work.id)
        .execute(&pool)
        .await
        .expect("мимо службы");
    assert!(
        svc.approve_studio_card(work.id, keeper_word()).await.is_err(),
        "и утвердить нельзя"
    );
}

/// Утверждение пишет НАСТОЯЩУЮ карту: она на полке, подписана автором, и
/// первый экземпляр у него.
#[sqlx::test]
async fn approving_puts_a_real_card_on_the_shelf(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Мастерица").await;
    let work = svc
        .save_studio_card(author, None, &a_fair_card("Пряха"), "ru")
        .await
        .expect("завести");
    svc.accept_studio_agreement(author).await.expect("соглашение");
    svc.show_studio_card(author, work.id).await.expect("отдать");

    let made = svc
        .approve_studio_card(work.id, keeper_word())
        .await
        .expect("утвердить");

    // На полке — той же, что и карты дома.
    let shelf = svc.list_battle_cards_public().await.expect("полка");
    assert_eq!(shelf.len(), 1);
    assert_eq!(shelf[0].id, made.id);
    assert_eq!(
        shelf[0].credit_name.as_deref(),
        Some("Мастерица"),
        "автограф стоит по умолчанию: это чужой труд"
    );

    // Первый экземпляр — автору, даром.
    let me = svc.battle_me(author).await.expect("книга");
    assert_eq!(me.owned.len(), 1, "экземпляр у автора");
    assert_eq!(me.owned[0].serial, Some(1), "и он первый");
    assert_eq!(me.dust, i64::from(gotiga_server::studio::PAY_CARD_APPROVED));
}

/// Два «утвердить» ОДНОВРЕМЕННО — одна карта и одна плата.
///
/// Медленная сеть и второе нажатие не должны ни завести на полке близнеца, ни
/// заплатить дважды. Проверяется именно ОДНОВРЕМЕННО, а не по очереди:
/// последовательный второй заход отбивается ещё и занятым слугом, то есть
/// проверка по очереди зелена даже без замка — и однажды соврала бы.
#[sqlx::test]
async fn approving_twice_at_once_makes_one_card_and_pays_once(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let work = svc
        .save_studio_card(author, None, &a_fair_card("Одна"), "ru")
        .await
        .expect("завести");
    svc.accept_studio_agreement(author).await.expect("соглашение");
    svc.show_studio_card(author, work.id).await.expect("отдать");

    let one = svc.clone();
    let two = svc.clone();
    let id = work.id;
    let (a, b) = tokio::join!(
        async move { one.approve_studio_card(id, keeper_word()).await },
        async move { two.approve_studio_card(id, keeper_word()).await },
    );
    assert!(
        a.is_ok() != b.is_ok(),
        "ровно одно нажатие должно было сработать"
    );

    let shelf = svc.list_battle_cards_public().await.expect("полка");
    assert_eq!(shelf.len(), 1, "карта одна");
    let me = svc.battle_me(author).await.expect("книга");
    assert_eq!(me.owned.len(), 1, "экземпляр один");
    assert_eq!(
        me.dust,
        i64::from(gotiga_server::studio::PAY_CARD_APPROVED),
        "заплачено один раз"
    );
}

/// Взятая домом работа — ТУПИК ДЛЯ ПРАВОК, и это её единственный выход.
///
/// Разбор `STUDIO-CARDS-REVIEW.md` П1: замок утверждения носил чужое слово
/// (`withdrawn`), и всё, что дом разрешает снятому, он разрешал и взятому.
/// Прогон печатал: правится — да, показывается снова — да, и после этого
/// работа вечно висела в очереди хозяина, откуда её нельзя ни утвердить, ни
/// вернуть.
///
/// Проверяется не слово состояния, а три следствия разом: память о принесённом
/// не переписать, второй раз не отдать, в очереди не появиться.
#[sqlx::test]
async fn a_work_the_house_took_is_out_of_the_authors_hands(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let work = a_shown_card(&svc, author, "Пряха").await;
    svc.approve_studio_card(work, keeper_word())
        .await
        .expect("утвердить");

    assert!(
        svc.save_studio_card(author, Some(work), &a_fair_card("Подменённая"), "ru")
            .await
            .is_err(),
        "память о принесённом не переписывается"
    );
    assert!(
        svc.show_studio_card(author, work).await.is_err(),
        "второй раз работу не отдать"
    );
    assert!(
        svc.withdraw_studio_card(author, work).await.is_err(),
        "и не снять: она уже не у автора"
    );
    assert!(
        svc.studio_card_queue().await.expect("очередь").is_empty(),
        "в очереди хозяина её нет"
    );
}

/// Соглашение автора — то же, что у рамы.
///
/// П2: у карт его не спрашивали вовсе, и работа уходила на полку дома, в
/// продажу и в чужие руки без единого слова о правах.
#[sqlx::test]
async fn nothing_leaves_a_persons_hands_without_the_agreement(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let work = svc
        .save_studio_card(author, None, &a_fair_card("Пряха"), "ru")
        .await
        .expect("завести");

    assert!(
        svc.show_studio_card(author, work.id).await.is_err(),
        "без соглашения работа никуда не уходит"
    );
    svc.accept_studio_agreement(author).await.expect("согласие");
    svc.show_studio_card(author, work.id)
        .await
        .expect("с согласием — уходит");
}

/// Потолок незаконченных карт — свой, а не рамочный (П4).
#[sqlx::test]
async fn a_person_cannot_start_endless_cards(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let mut settings = svc.get_studio_settings().await.expect("настройки");
    settings.cards_open = 2;
    svc.save_studio_settings(settings).await.expect("сохранить");
    let author = seed_user(&pool, "Автор").await;

    for i in 0..2 {
        svc.save_studio_card(author, None, &a_fair_card(&format!("Раз {i}")), "ru")
            .await
            .expect("заводится");
    }
    assert!(
        svc.save_studio_card(author, None, &a_fair_card("Третья"), "ru")
            .await
            .is_err(),
        "третья сверх потолка не заводится"
    );
}

/// Автограф стоит в ТОЙ ЖЕ записи, которой карта заводится (П7), и карта автора
/// видна в Зале авторов (П6) — обещанное третье место подписи.
#[sqlx::test]
async fn the_signature_is_on_the_card_and_in_the_hall(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Мастерица").await;
    let work = a_shown_card(&svc, author, "Пряха").await;
    let made = svc
        .approve_studio_card(work, keeper_word())
        .await
        .expect("утвердить");
    assert_eq!(made.credit_name.as_deref(), Some("Мастерица"));

    // Адрес автору дом заводит при первом допуске рамы; у карт этого шага нет,
    // поэтому берём слуг прямо из учётной записи.
    let slug: Option<String> = sqlx::query_scalar("SELECT studio_slug FROM users WHERE id = $1")
        .bind(author)
        .fetch_one(&pool)
        .await
        .expect("слуг");
    if let Some(slug) = slug {
        let page = svc.studio_author(&slug).await.expect("страница автора");
        assert_eq!(page.cards.len(), 1, "карта стоит в Зале авторов");
    }
}

// ── Свои роды ───────────────────────────────────────────────────────────────
//
// Род — не карта: взвешивать в нём нечего, есть имя, слово о нём и значок. Но
// путь у него ТОТ ЖЕ, и слова состояния те же — третий набор слов для третьей
// вещи был бы третьим заходом на грабли, которые дом уже прошёл дважды.

fn a_race(name: &str) -> gotiga_server::models::SaveStudioRaceRequest {
    gotiga_server::models::SaveStudioRaceRequest {
        name_en: name.into(),
        name_ru: name.into(),
        note_en: Some("Made by a person.".into()),
        note_ru: Some("Придумано человеком.".into()),
        icon_url: None,
        lang: "ru".into(),
    }
}

/// Утверждение рода ставит его в СЛОВАРЬ дома — тот самый, из которого роды
/// выбирают все карты, — и подписывает автором.
#[sqlx::test]
async fn an_approved_race_enters_the_house_dictionary(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Мастерица").await;
    let work = svc
        .save_studio_race(author, None, &a_race("Болотники"))
        .await
        .expect("завести род");
    svc.accept_studio_agreement(author).await.expect("соглашение");
    svc.show_studio_race(author, work.id).await.expect("отдать");

    let made = svc
        .approve_studio_race(
            work.id,
            gotiga_server::models::ApproveStudioRaceRequest {
                slug: Some("bolotniki".into()),
                credit_name: None,
            },
        )
        .await
        .expect("утвердить");
    assert_eq!(made.slug, "bolotniki");

    // Он в словаре, из которого выбирают все карты дома.
    let dictionary = svc.list_battle_races().await.expect("словарь");
    assert_eq!(dictionary.len(), 1);
    assert_eq!(dictionary[0].name_ru, "Болотники");

    // И подписан автором.
    let credit: Option<String> =
        sqlx::query_scalar("SELECT credit_name FROM battle_races WHERE id = $1")
            .bind(Uuid::parse_str(&made.id).unwrap())
            .fetch_one(&pool)
            .await
            .expect("подпись");
    assert_eq!(credit.as_deref(), Some("Мастерица"));
}

/// Взятый домом род — тупик для правок, как и карта: его уже носят чужие карты.
/// И второе утверждение не заводит второй строки словаря.
#[sqlx::test]
async fn a_race_the_house_took_is_settled(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let work = svc
        .save_studio_race(author, None, &a_race("Болотники"))
        .await
        .expect("завести");
    svc.accept_studio_agreement(author).await.expect("соглашение");
    svc.show_studio_race(author, work.id).await.expect("отдать");
    let keeper = || gotiga_server::models::ApproveStudioRaceRequest {
        slug: Some("bolotniki".into()),
        credit_name: None,
    };
    svc.approve_studio_race(work.id, keeper()).await.expect("утвердить");

    assert!(
        svc.save_studio_race(author, Some(work.id), &a_race("Другие"))
            .await
            .is_err(),
        "взятый домом не правится"
    );
    assert!(
        svc.show_studio_race(author, work.id).await.is_err(),
        "и второй раз не отдаётся"
    );
    assert!(
        svc.approve_studio_race(work.id, keeper()).await.is_err(),
        "второе утверждение уходит ни с чем"
    );
    assert_eq!(
        svc.list_battle_races().await.expect("словарь").len(),
        1,
        "строка словаря одна"
    );
}

/// Род зовут на обоих языках, и без соглашения он никуда не уходит.
#[sqlx::test]
async fn a_race_needs_both_names_and_the_agreement(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;

    let mut half = a_race("Болотники");
    half.name_en = "  ".into();
    assert!(
        svc.save_studio_race(author, None, &half).await.is_err(),
        "пустое английское имя ставило бы кириллицу в шапку английской полки"
    );

    let work = svc
        .save_studio_race(author, None, &a_race("Болотники"))
        .await
        .expect("завести");
    assert!(
        svc.show_studio_race(author, work.id).await.is_err(),
        "без соглашения работа из рук не уходит"
    );
}

// ── Свои движения ───────────────────────────────────────────────────────────
//
// Движение человека — сочетание ГОТОВЫХ жестов: движок не умеет ничего сверх
// закрытого списка тел, и принести через движение новое правило нельзя. Отсюда
// и путь у него тот же, что у карты и рода.

fn a_motion(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "nameEn": id,
        "nameRu": id,
        "occasion": "blow",
        "span": 400,
        "gestures": [{ "whom": "striker", "body": "lunge", "at": 0, "ms": 220 }],
    })
}

async fn my_motions(
    svc: &AppService,
    who: Uuid,
    what: Vec<serde_json::Value>,
) -> Vec<gotiga_server::models::StudioMotionDto> {
    svc.save_studio_motions(
        who,
        &gotiga_server::models::SaveStudioMotionsRequest {
            motions: what,
            lang: "ru".into(),
        },
    )
    .await
    .expect("сохранить ящик")
}

/// Утверждённое движение встаёт В СВОД ДОМА — тот самый, из которого его
/// возьмёт сцена, — и несёт имя автора.
#[sqlx::test]
async fn an_approved_motion_joins_the_house_sheaf(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Мастерица").await;
    let mine = my_motions(&svc, author, vec![a_motion("vypad")]).await;
    assert_eq!(mine.len(), 1);
    svc.accept_studio_agreement(author).await.expect("соглашение");
    svc.show_studio_motion(author, mine[0].id).await.expect("отдать");

    let name = svc
        .approve_studio_motion(mine[0].id)
        .await
        .expect("утвердить");

    let house = svc.get_battle_motions().await.expect("свод");
    let one = house
        .motions
        .iter()
        .find(|m| m.id == name)
        .expect("движение в своде");
    assert_eq!(one.credit, "Мастерица", "автограф стоит");
    assert!(!one.gestures.is_empty(), "и жесты на месте");
}

/// Имя, занятое домом, не затирается: на имя движения показывают карты и расы.
#[sqlx::test]
async fn a_motion_never_overwrites_one_the_house_already_has(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;

    // Дом уже держит движение с этим именем.
    svc.save_battle_motions(gotiga_server::battles::BattleMotions {
        motions: vec![serde_json::from_value(a_motion("vypad")).unwrap()],
    })
    .await
    .expect("свод дома");

    let mine = my_motions(&svc, author, vec![a_motion("vypad")]).await;
    svc.accept_studio_agreement(author).await.expect("соглашение");
    svc.show_studio_motion(author, mine[0].id).await.expect("отдать");
    let name = svc.approve_studio_motion(mine[0].id).await.expect("утвердить");

    assert_ne!(name, "vypad", "движение получило своё имя");
    let house = svc.get_battle_motions().await.expect("свод");
    assert_eq!(house.motions.len(), 2, "оба на месте");
    assert!(
        house.motions.iter().any(|m| m.id == "vypad" && m.credit.is_empty()),
        "домашнее не тронуто"
    );
}

/// Отданное хозяину движение автор не переписывает, а взятое домом — тем более.
#[sqlx::test]
async fn a_motion_given_away_is_not_the_authors_to_change(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let mine = my_motions(&svc, author, vec![a_motion("vypad")]).await;
    svc.accept_studio_agreement(author).await.expect("соглашение");
    svc.show_studio_motion(author, mine[0].id).await.expect("отдать");

    // Стол сохраняет ящик целиком — и отданное в нём остаётся прежним.
    let mut changed = a_motion("vypad");
    changed["nameRu"] = serde_json::json!("Подменённое");
    let after = my_motions(&svc, author, vec![changed]).await;
    assert_eq!(
        after[0].body["nameRu"], "vypad",
        "отданное хозяину не переписывается"
    );

    // И стереть его из ящика нельзя: хозяин уже смотрит.
    let kept = my_motions(&svc, author, vec![]).await;
    assert_eq!(kept.len(), 1, "отданное остаётся");
}

/// Автограф ставит СТУДИЯ, а не человек у себя: имя, приехавшее в теле,
/// было бы чужой подписью на своей работе.
#[sqlx::test]
async fn a_person_cannot_sign_a_motion_with_another_name(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let mut sneaky = a_motion("vypad");
    sneaky["credit"] = serde_json::json!("Хозяин дома");

    let mine = my_motions(&svc, author, vec![sneaky]).await;
    // Пустая подпись не пишется в тело вовсе — её там просто нет.
    assert!(
        mine[0].body.get("credit").is_none(),
        "подпись из тела не сохраняется"
    );
}
