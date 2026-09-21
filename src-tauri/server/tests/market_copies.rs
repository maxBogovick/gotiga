//! Вторичный рынок ЭКЗЕМПЛЯРОВ: человек продаёт человеку свою карту.
//!
//! Ради этого владение и переписывалось на экземпляры: пока в схеме было
//! «человек X владеет картой Y», правило «продал — и у тебя этого больше нет»
//! было невыразимо, потому что продавать нечего, когда у вещи нет личности.
//!
//! Проверяется не «запрос отработал», а само правило: вещь УШЛА, деньги
//! перешли, доля дома сгорела, и выставленным нельзя ни играть, ни торговать
//! второй раз.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::{
    BuyBattleCardRequest, GrantBattleCoinRequest, OfferTradeRequest, SaveBattleCardRequest,
    TradeItem,
};
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-market-uploads-{}", Uuid::new_v4()),
        public_url: "http://127.0.0.1".into(),
        rust_log: String::new(),
        admin_login: "admin".into(),
        admin_password: "test-password-123".into(),
        cors_allowed_origins: vec![],
        telegram_bot_token: None,
        telegram_login_bot_token: None,
        telegram_login_bot_username: None,
        telegram_webhook_secret: None,
        telegram_chat_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-market-logs-{}.sqlite", Uuid::new_v4()),
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
    .bind(format!("market-{}@example.test", Uuid::new_v4()))
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("завести человека")
}

/// Ворота настежь: проверки про сделку, а не про то, кого пускают в студию.
async fn open_the_gate(svc: &AppService) {
    let mut settings = svc.get_studio_settings().await.expect("настройки");
    settings.gate = "all".into();
    svc.save_studio_settings(settings).await.expect("сохранить");
}

fn card(slug: &str) -> SaveBattleCardRequest {
    SaveBattleCardRequest {
        slug: Some(slug.into()),
        status: "published".into(),
        tier: 1,
        race_id: None,
        type_en: None,
        type_ru: None,
        title_en: slug.into(),
        title_ru: slug.into(),
        effect_en: Some("A card.".into()),
        effect_ru: Some("Карта.".into()),
        lore_en: None,
        lore_ru: None,
        cost: 1,
        power: 3,
        health: 6,
        mana: 0,
        traits: Vec::new(),
        kind: "unit".into(),
        armor: 0,
        ward: 0,
        attack_channel: "physical".into(),
        reach: 1,
        step: 1,
        speed: 3,
        mend: 0,
        abilities: Vec::new(),
        price_dust: Some(10),
        price_feed: None,
        level_price_dust: None,
        art_url: None,
        art_focal: None,
        frame_override: None,
        motion_wear: None,
        lendable: false,
        edition_size: None,
        figurine_id: None,
    }
}

async fn give_dust(svc: &AppService, who: Uuid, amount: i32, key: &str) {
    svc.admin_grant_battle_coin(&GrantBattleCoinRequest {
        user_id: who,
        currency: "dust".into(),
        amount,
        note: Some("на опыты".into()),
        idem_key: key.into(),
    })
    .await
    .expect("пыль из рук");
}

/// Карта, купленная человеком: его экземпляр с номером.
async fn a_bought_copy(svc: &AppService, pool: &PgPool, who: Uuid, slug: &str) -> Uuid {
    let made = svc.admin_create_battle_card(card(slug)).await.expect("карта");
    let card_id = Uuid::parse_str(&made.id).unwrap();
    give_dust(svc, who, 100, &format!("{who}-{slug}")).await;
    svc.buy_battle_card(
        who,
        &BuyBattleCardRequest {
            card_id,
            currency: "dust".into(),
            expected_price: 10,
        },
    )
    .await
    .expect("взял");
    sqlx::query_scalar("SELECT id FROM card_copies WHERE owner_id = $1 AND card_id = $2")
        .bind(who)
        .bind(card_id)
        .fetch_one(pool)
        .await
        .expect("экземпляр")
}

/// Продал — и у тебя этого больше нет.
///
/// Главное правило рынка, и оно выполняется ФОРМОЙ таблицы: смена владельца в
/// одной строке. Ничего не копируется, ничего не исчезает — поэтому проверяем
/// обе стороны разом, а не только приход.
#[sqlx::test]
async fn what_is_sold_leaves_the_seller_for_good(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Продавец").await;
    let buyer = seed_user(&pool, "Покупатель").await;
    let copy = a_bought_copy(&svc, &pool, seller, "boec").await;

    // Уровень поднят: продаётся именно ЭТОТ экземпляр, со своим уровнем.
    sqlx::query("UPDATE card_copies SET level = 3 WHERE id = $1")
        .bind(copy)
        .execute(&pool)
        .await
        .expect("уровень");

    let listing = svc
        .list_thing(seller, "copy", copy, 200, "dust")
        .await
        .expect("выставить");
    give_dust(&svc, buyer, 500, "buyer-purse").await;
    svc.buy_listing(buyer, listing).await.expect("купить");

    let sellers = svc.battle_me(seller).await.expect("книга продавца");
    assert!(sellers.owned.is_empty(), "у продавца карты больше нет");
    let buyers = svc.battle_me(buyer).await.expect("книга покупателя");
    assert_eq!(buyers.owned.len(), 1, "она у покупателя");
    assert_eq!(buyers.owned[0].level, 3, "и с тем же уровнем");

    // Экземпляр ОДИН и тот же: продажа не печатает второго.
    let how_many: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM card_copies")
        .fetch_one(&pool)
        .await
        .expect("экземпляры");
    assert_eq!(how_many, 1, "экземпляр не размножился");
}

/// Выставленным нельзя ИГРАТЬ.
///
/// Иначе вещь стоит на доске в тот самый миг, когда её у тебя покупают.
#[sqlx::test]
async fn a_thing_on_the_counter_is_out_of_play(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Продавец").await;
    let copy = a_bought_copy(&svc, &pool, seller, "boec").await;

    assert_eq!(
        svc.battle_me(seller).await.expect("книга").owned.len(),
        1,
        "до прилавка карта у него"
    );
    let listing = svc
        .list_thing(seller, "copy", copy, 200, "dust")
        .await
        .expect("выставить");
    assert!(
        svc.battle_me(seller).await.expect("книга").owned.is_empty(),
        "выставленной картой не играют"
    );

    // И второй раз её не выставить.
    assert!(
        svc.list_thing(seller, "copy", copy, 300, "dust").await.is_err(),
        "второе объявление на ту же вещь"
    );

    // Снял — вернулась и в руки, и в игру.
    svc.withdraw_listing(seller, listing).await.expect("снять");
    assert_eq!(
        svc.battle_me(seller).await.expect("книга").owned.len(),
        1,
        "снятая с прилавка снова играет"
    );
}

/// Чужую вещь не выставить, и это не проверка на странице, а отбор в запросе.
#[sqlx::test]
async fn nobody_sells_what_is_not_theirs(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let owner = seed_user(&pool, "Хозяйка вещи").await;
    let stranger = seed_user(&pool, "Чужой").await;
    let copy = a_bought_copy(&svc, &pool, owner, "boec").await;

    assert!(
        svc.list_thing(stranger, "copy", copy, 200, "dust").await.is_err(),
        "чужой экземпляр не выставляется"
    );
    assert!(
        svc.list_thing(owner, "copy", Uuid::new_v4(), 200, "dust")
            .await
            .is_err(),
        "и несуществующий тоже"
    );
}

/// Сделка на экземпляр: деньги перешли, доля дома СГОРЕЛА.
///
/// Сгоревшая доля — единственный сток валюты, и проверяется не «комиссия
/// посчитана», а то, ради чего она есть: денег в доме стало МЕНЬШЕ.
#[sqlx::test]
async fn a_sale_of_a_copy_burns_the_house_share(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Продавец").await;
    let buyer = seed_user(&pool, "Покупатель").await;
    let copy = a_bought_copy(&svc, &pool, seller, "boec").await;
    give_dust(&svc, buyer, 500, "buyer-purse").await;
    let listing = svc
        .list_thing(seller, "copy", copy, 200, "dust")
        .await
        .expect("выставить");

    let before: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)::bigint FROM battle_wallet_entries WHERE currency = 'dust'",
    )
    .fetch_one(&pool)
    .await
    .expect("в доме");

    svc.buy_listing(buyer, listing).await.expect("купить");

    let after: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)::bigint FROM battle_wallet_entries WHERE currency = 'dust'",
    )
    .fetch_one(&pool)
    .await
    .expect("в доме");
    assert_eq!(before - after, 20, "десятая часть двухсот сгорела");

    // На прилавке её больше нет.
    assert!(svc.market_copies().await.expect("прилавок").is_empty());
}

/// Прилавок показывает экземпляр так, чтобы по нему можно было решать:
/// какая карта, чей уровень, чей номер и чьё имя стоит на карте.
#[sqlx::test]
async fn the_counter_says_what_it_is_selling(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Продавец").await;
    let copy = a_bought_copy(&svc, &pool, seller, "boec").await;
    svc.list_thing(seller, "copy", copy, 200, "dust")
        .await
        .expect("выставить");

    let counter = svc.market_copies().await.expect("прилавок");
    assert_eq!(counter.len(), 1);
    assert_eq!(counter[0].title_ru, "boec");
    assert_eq!(counter[0].level, 1);
    assert_eq!(counter[0].serial, Some(1));
    assert_eq!(counter[0].seller, "Продавец");

    // Своё видно и человеку — вместе с тем, что оно заперто: снимать он
    // приходит туда же, где выставлял.
    let mine = svc.my_copies(seller).await.expect("свои вещи");
    assert_eq!(mine.len(), 1);
    assert!(mine[0].locked, "выставленное помечено запертым");
}

// ── Мена ────────────────────────────────────────────────────────────────────
//
// Вещь за вещь, без денег. Предлагается против объявления: чтобы предложить
// человеку обмен, надо сперва увидеть, что у него есть, а места, где видно
// чужое собрание, в доме нет и заводить его незачем.

/// Мена целиком: обе вещи поменялись хозяевами, и ни пылинки не тронуто.
#[sqlx::test]
async fn a_trade_moves_both_things_and_no_dust(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Хозяйка прилавка").await;
    let other = seed_user(&pool, "Меняла").await;
    let his = a_bought_copy(&svc, &pool, seller, "boec").await;
    let hers = a_bought_copy(&svc, &pool, other, "voron").await;

    let listing = svc
        .list_thing(seller, "copy", his, 200, "dust")
        .await
        .expect("выставить");

    let purse_before = (
        svc.battle_me(seller).await.expect("книга").dust,
        svc.battle_me(other).await.expect("книга").dust,
    );

    let trade = svc
        .offer_trade(
            other,
            &OfferTradeRequest {
                listing_id: listing,
                items: vec![TradeItem {
                    kind: "copy".into(),
                    subject_id: hers,
                }],
            },
        )
        .await
        .expect("предложить мену");
    svc.accept_trade(seller, trade).await.expect("принять");

    // Каждый держит теперь чужое — и ровно по одной вещи.
    let mine = svc.battle_me(seller).await.expect("книга");
    let theirs = svc.battle_me(other).await.expect("книга");
    assert_eq!(mine.owned.len(), 1);
    assert_eq!(theirs.owned.len(), 1);
    assert_ne!(mine.owned[0].card_id, theirs.owned[0].card_id, "вещи разошлись");

    // Пыль не тронута: мена её не создаёт и доли дома не платит.
    assert_eq!(
        (mine.dust, theirs.dust),
        purse_before,
        "мена кошелька не касается"
    );

    // Экземпляров по-прежнему два: ничего не напечаталось.
    let how_many: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM card_copies")
        .fetch_one(&pool)
        .await
        .expect("экземпляры");
    assert_eq!(how_many, 2);
}

/// Предложенное ЗАПЕРТО, пока мена стоит: им нельзя ни играть, ни выставить.
/// Отказ отпирает — и вещь возвращается в игру.
#[sqlx::test]
async fn what_is_offered_is_held_until_the_answer(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Хозяйка прилавка").await;
    let other = seed_user(&pool, "Меняла").await;
    let his = a_bought_copy(&svc, &pool, seller, "boec").await;
    let hers = a_bought_copy(&svc, &pool, other, "voron").await;
    let listing = svc
        .list_thing(seller, "copy", his, 200, "dust")
        .await
        .expect("выставить");

    let trade = svc
        .offer_trade(
            other,
            &OfferTradeRequest {
                listing_id: listing,
                items: vec![TradeItem {
                    kind: "copy".into(),
                    subject_id: hers,
                }],
            },
        )
        .await
        .expect("предложить");

    assert!(
        svc.battle_me(other).await.expect("книга").owned.is_empty(),
        "предложенным не играют"
    );
    assert!(
        svc.list_thing(other, "copy", hers, 200, "dust").await.is_err(),
        "и на прилавок его не выставить"
    );

    svc.refuse_trade(seller, trade).await.expect("отказать");
    assert_eq!(
        svc.battle_me(other).await.expect("книга").owned.len(),
        1,
        "после отказа вещь снова в руках и в игре"
    );
}

/// Чужое в мену не положишь, и своё объявление меной себе не закроешь.
#[sqlx::test]
async fn a_trade_offers_only_what_is_yours(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Хозяйка прилавка").await;
    let other = seed_user(&pool, "Меняла").await;
    let his = a_bought_copy(&svc, &pool, seller, "boec").await;
    let hers = a_bought_copy(&svc, &pool, other, "voron").await;
    let listing = svc
        .list_thing(seller, "copy", his, 200, "dust")
        .await
        .expect("выставить");

    let offer = |what: Uuid| OfferTradeRequest {
        listing_id: listing,
        items: vec![TradeItem {
            kind: "copy".into(),
            subject_id: what,
        }],
    };
    assert!(
        svc.offer_trade(other, &offer(his)).await.is_err(),
        "чужую вещь в мену не положить"
    );
    assert!(
        svc.offer_trade(seller, &offer(his)).await.is_err(),
        "и своё объявление себе не закрыть"
    );

    // Ни одной половины: неудачная мена не оставляет запертого.
    assert_eq!(
        svc.battle_me(other).await.expect("книга").owned.len(),
        1,
        "вещь предлагавшего осталась при нём и отперта"
    );
    let _ = hers;
}

/// Вещь ушла за пыль — мены на неё отпадают, и предложенное возвращается.
///
/// Иначе человек, предложивший мену, теряет свою вещь в запертом виде за
/// объявлением, которого больше нет.
#[sqlx::test]
async fn offers_fall_away_with_the_thing_they_wanted(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Хозяйка прилавка").await;
    let other = seed_user(&pool, "Меняла").await;
    let buyer = seed_user(&pool, "Покупатель").await;
    let his = a_bought_copy(&svc, &pool, seller, "boec").await;
    let hers = a_bought_copy(&svc, &pool, other, "voron").await;
    let listing = svc
        .list_thing(seller, "copy", his, 200, "dust")
        .await
        .expect("выставить");
    svc.offer_trade(
        other,
        &OfferTradeRequest {
            listing_id: listing,
            items: vec![TradeItem {
                kind: "copy".into(),
                subject_id: hers,
            }],
        },
    )
    .await
    .expect("предложить");

    give_dust(&svc, buyer, 500, "buyer-purse").await;
    svc.buy_listing(buyer, listing).await.expect("купить за пыль");

    assert_eq!(
        svc.battle_me(other).await.expect("книга").owned.len(),
        1,
        "предложенное вернулось хозяину отпертым"
    );
    assert!(
        svc.trades(other).await.expect("мены").is_empty(),
        "и самой мены больше нет"
    );
}
