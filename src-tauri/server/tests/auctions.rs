//! Молоток: вещь уходит тому, кто больше дал.
//!
//! Два случая, механизм один: человек сам отдаёт вещь на торг, и вещь ушедшего
//! из дома уходит с молотка, а вырученное сгорает — платить некому.
//!
//! Главное здесь — что СТАВКА НЕ ПЛАТЁЖ. С неё ничего не списывается, деньги у
//! человека на руках всю неделю, и за обещание спрашивают в момент удара
//! молотком. Из этого следует и то, что победитель может не суметь заплатить, —
//! и лот тогда уходит следующему, кто может.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::{BuyBattleCardRequest, GrantBattleCoinRequest, SaveBattleCardRequest};
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-auction-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-auction-logs-{}.sqlite", Uuid::new_v4()),
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
    .bind(format!("auction-{}@example.test", Uuid::new_v4()))
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

async fn a_copy(svc: &AppService, pool: &PgPool, who: Uuid, slug: &str) -> Uuid {
    let made = svc.admin_create_battle_card(card(slug)).await.expect("карта");
    let card_id = Uuid::parse_str(&made.id).unwrap();
    give_dust(svc, who, 10, &format!("{who}-{slug}")).await;
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

fn lot(kind: &str, subject: Uuid, price: i32) -> gotiga_server::models::StartAuctionRequest {
    gotiga_server::models::StartAuctionRequest {
        kind: kind.into(),
        subject_id: subject,
        start_price: price,
        currency: "dust".into(),
        days: Some(1),
    }
}

/// Торг целиком: вещь заперта, ставка ничего не списывает, молоток отдаёт её
/// тому, кто дал больше, и берёт деньги ровно в этот миг.
#[sqlx::test]
async fn a_bid_is_a_promise_and_the_hammer_is_the_payment(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Продавец").await;
    let one = seed_user(&pool, "Первый").await;
    let two = seed_user(&pool, "Второй").await;
    let copy = a_copy(&svc, &pool, seller, "boec").await;
    give_dust(&svc, one, 500, "one").await;
    give_dust(&svc, two, 500, "two").await;

    let auction = svc
        .start_auction(seller, &lot("copy", copy, 50))
        .await
        .expect("на молоток");

    // Выставленное — вне игры, как и на прилавке.
    assert!(
        svc.battle_me(seller).await.expect("книга").owned.is_empty(),
        "вещь с молотка не играет"
    );

    svc.place_bid(one, auction, 50).await.expect("первая ставка");
    assert!(
        svc.place_bid(two, auction, 55).await.is_err(),
        "перебивать надо на шаг, а не на пылинку"
    );
    svc.place_bid(two, auction, 60).await.expect("перебил");

    // Ставка НЕ списывает: пока идёт торг, деньги на руках.
    assert_eq!(svc.battle_me(two).await.expect("книга").dust, 500);

    // Что было у продавца до удара молотком: дом мог заплатить ему за поручения
    // («первая карта» и прочее), и сравнивать с постоянным числом значило бы
    // проверять не сделку, а погоду в доме.
    let seller_before = svc.battle_me(seller).await.expect("книга").dust;

    sqlx::query("UPDATE auctions SET ends_at = NOW() - interval '1 minute' WHERE id = $1")
        .bind(auction)
        .execute(&pool)
        .await
        .expect("срок вышел");
    assert_eq!(svc.close_due_auctions().await.expect("молоток"), 1);

    let winner = svc.battle_me(two).await.expect("книга");
    assert_eq!(winner.owned.len(), 1, "вещь у того, кто дал больше");
    assert_eq!(winner.dust, 440, "и заплатил он ровно в этот миг");
    assert_eq!(
        svc.battle_me(one).await.expect("книга").dust,
        500,
        "с проигравшего не взято ничего"
    );
    // Комиссия дома (10% по умолчанию) сгорает и здесь — тот же сток, что и у
    // прилавка: 60 за вещь, продавцу 54, шесть — никому.
    assert_eq!(
        svc.battle_me(seller).await.expect("книга").dust - seller_before,
        54,
        "продавцу пришло за вычетом комиссии дома"
    );
}

/// Победитель не смог заплатить — лот уходит следующему, кто может.
///
/// Ровно то, ради чего хранятся ВСЕ ставки, а не одна высшая: «следующего»
/// неоткуда взять, когда в доме записана одна цифра.
#[sqlx::test]
async fn the_lot_falls_to_the_next_who_can_pay(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Продавец").await;
    let loud = seed_user(&pool, "Громкий").await;
    let quiet = seed_user(&pool, "Тихий").await;
    let copy = a_copy(&svc, &pool, seller, "boec").await;
    give_dust(&svc, loud, 300, "loud").await;
    give_dust(&svc, quiet, 300, "quiet").await;

    let auction = svc
        .start_auction(seller, &lot("copy", copy, 50))
        .await
        .expect("на молоток");
    svc.place_bid(quiet, auction, 100).await.expect("ставка");
    svc.place_bid(loud, auction, 200).await.expect("ставка выше");

    // Громкий за неделю потратил всё на другое.
    give_dust(&svc, loud, -250, "loud-spent").await;

    sqlx::query("UPDATE auctions SET ends_at = NOW() - interval '1 minute' WHERE id = $1")
        .bind(auction)
        .execute(&pool)
        .await
        .expect("срок вышел");
    svc.close_due_auctions().await.expect("молоток");

    assert_eq!(
        svc.battle_me(quiet).await.expect("книга").owned.len(),
        1,
        "вещь ушла тому, кто смог заплатить"
    );
    assert_eq!(svc.battle_me(quiet).await.expect("книга").dust, 200);
    assert!(svc.battle_me(loud).await.expect("книга").owned.is_empty());
}

/// Никто не взял — вещь возвращается хозяину ОТПЕРТОЙ.
///
/// Иначе она висела бы за торгом, которого больше нет: ни играть, ни продать.
#[sqlx::test]
async fn a_lot_nobody_took_comes_back_unlocked(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Продавец").await;
    let copy = a_copy(&svc, &pool, seller, "boec").await;
    let auction = svc
        .start_auction(seller, &lot("copy", copy, 50))
        .await
        .expect("на молоток");

    sqlx::query("UPDATE auctions SET ends_at = NOW() - interval '1 minute' WHERE id = $1")
        .bind(auction)
        .execute(&pool)
        .await
        .expect("срок вышел");
    svc.close_due_auctions().await.expect("молоток");

    let me = svc.battle_me(seller).await.expect("книга");
    assert_eq!(me.owned.len(), 1, "вещь вернулась и снова играет");
}

/// Вещи ушедшего из дома уходят с молотка, и вырученное СГОРАЕТ.
///
/// Проверяется не «продавцу не начислено», а то, ради чего это сделано: денег
/// в доме стало меньше ровно на цену лота.
#[sqlx::test]
async fn what_a_person_left_behind_goes_under_the_hammer(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let leaving = seed_user(&pool, "Ушедший").await;
    let buyer = seed_user(&pool, "Покупатель").await;
    a_copy(&svc, &pool, leaving, "boec").await;
    give_dust(&svc, buyer, 500, "buyer").await;

    // Человек ушёл: вещи остались ничьими (`ON DELETE SET NULL`).
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(leaving)
        .execute(&pool)
        .await
        .expect("ушёл");

    assert_eq!(svc.auction_the_estate().await.expect("на молоток"), 1);
    let lots = svc.auctions().await.expect("молоток");
    assert_eq!(lots.len(), 1);
    assert!(lots[0].estate, "лот помечен как вещь ушедшего");
    assert!(lots[0].seller.is_none(), "продавца у него нет");

    let before: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)::bigint FROM battle_wallet_entries WHERE currency = 'dust'",
    )
    .fetch_one(&pool)
    .await
    .expect("в доме");

    svc.place_bid(buyer, lots[0].id, 100).await.expect("ставка");
    sqlx::query("UPDATE auctions SET ends_at = NOW() - interval '1 minute'")
        .execute(&pool)
        .await
        .expect("срок вышел");
    svc.close_due_auctions().await.expect("молоток");

    assert_eq!(
        svc.battle_me(buyer).await.expect("книга").owned.len(),
        1,
        "вещь у купившего"
    );
    let after: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)::bigint FROM battle_wallet_entries WHERE currency = 'dust'",
    )
    .fetch_one(&pool)
    .await
    .expect("в доме");
    assert_eq!(before - after, 100, "вырученное сгорело целиком");
}

/// Ставка в последние минуты отодвигает конец торга.
///
/// Иначе выигрывает не тот, кто дал больше, а тот, у кого быстрее рука.
#[sqlx::test]
async fn a_bid_at_the_last_minute_moves_the_end(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let seller = seed_user(&pool, "Продавец").await;
    let who = seed_user(&pool, "Ставящий").await;
    let copy = a_copy(&svc, &pool, seller, "boec").await;
    give_dust(&svc, who, 500, "who").await;
    let auction = svc
        .start_auction(seller, &lot("copy", copy, 50))
        .await
        .expect("на молоток");

    sqlx::query("UPDATE auctions SET ends_at = NOW() + interval '1 minute' WHERE id = $1")
        .bind(auction)
        .execute(&pool)
        .await
        .expect("минута до конца");
    svc.place_bid(who, auction, 50).await.expect("ставка в конце");

    let left: i64 = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (ends_at - NOW()))::bigint FROM auctions WHERE id = $1",
    )
    .bind(auction)
    .fetch_one(&pool)
    .await
    .expect("сколько осталось");
    assert!(left > 300, "торг отодвинут, а не решён последней секундой");
}
