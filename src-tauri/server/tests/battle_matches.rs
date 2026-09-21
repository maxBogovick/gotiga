//! Партия целиком, от испытания до пыли, на свежей базе.
//!
//! Это единственное место, где проверяется вся цепочка разом: карта дома →
//! тело движка → расстановка → ходы игрока и хранителя → журнал → награда.
//! Всё остальное о битвах проверяется в `battle-core` без базы и без сети;
//! здесь — только то, что бывает только на стыке.
//!
//! Требует `DATABASE_URL` и права CREATE DATABASE: `#[sqlx::test]` заводит на
//! каждый тест свою мигрированную базу и убирает её за собой.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::{
    BattleActRequest, ChallengePlacement, ChallengeSetup, DeckPlacement, GiveBattleCardsRequest,
    GrantBattleCoinRequest, RevokeBattleCardsRequest, SaveBattleCardRequest,
    SaveBattleChallengeRequest, SaveBattleDeckRequest,
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
        upload_dir: format!("/tmp/gotiga-battle-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-battle-logs-{}.sqlite", Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".into(),
        auth_pepper: "pepper-for-tests-0123456789".into(),
        auth_pepper_old: None,
    }
}

async fn seed_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (email, display_name, visual_password_hash)
         VALUES ($1, 'Гость', 'x') RETURNING id",
    )
    .bind(format!("battle-{}@example.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("завести гостя")
}

// ── Потолок чина, целиком по цепочке ────────────────────────────────────────
//
// `card_blockers` проверен без базы в `battles.rs`. Здесь — стык: что вес
// действительно доезжает до разбора годности, и что весы показывают хранителю
// ровно то слово, которым сохранение потом откажет. Разойдись эти двое —
// хранитель увидел бы «можно» и получил бы отказ.
//
// Базы не требует: весы — чистая функция над запросом.

#[test]
fn the_scales_refuse_what_outweighs_its_rank() {
    // Тело на 8 очков ровно: 0.5×10 здоровья + 3 силы на дальности 1.
    let mut req = card("гиря", 10, 3, 1);
    req.tier = 1;
    let fit = gotiga_server::services::AppService::weigh_battle_card(&req);
    assert_eq!(fit.total_points, 8.0);
    assert_eq!(fit.tier_budget, 8.0);
    assert!(
        fit.readiness.blocking.is_empty(),
        "ровно в бюджет — на полку можно"
    );

    // Одна лишняя единица силы — и карта первого чина за него вылезла.
    req.power = 4;
    let over = gotiga_server::services::AppService::weigh_battle_card(&req);
    assert_eq!(over.total_points, 9.0);
    assert_eq!(over.readiness.blocking, vec!["overTierBudget".to_string()]);

    // Тот же вес вторым чином проходит: чин и есть разрешение.
    req.tier = 2;
    assert!(
        gotiga_server::services::AppService::weigh_battle_card(&req)
            .readiness
            .blocking
            .is_empty()
    );

    // А черновику можно что угодно — он затем и черновик.
    req.tier = 1;
    req.status = "draft".into();
    assert!(
        gotiga_server::services::AppService::weigh_battle_card(&req)
            .readiness
            .blocking
            .is_empty()
    );
}

#[test]
fn the_scales_no_longer_charge_for_speed() {
    // Скорость бралась по 2 очка за ступень при том, что в движке её нет.
    let mut slow = card("тихий", 10, 3, 1);
    slow.speed = 1;
    let mut quick = card("быстрый", 10, 3, 1);
    quick.speed = 5;
    assert_eq!(
        gotiga_server::services::AppService::weigh_battle_card(&slow).total_points,
        gotiga_server::services::AppService::weigh_battle_card(&quick).total_points,
        "скорость больше ничего не стоит"
    );
}

fn card(slug: &str, health: i16, power: i16, reach: i16) -> SaveBattleCardRequest {
    SaveBattleCardRequest {
        slug: Some(slug.into()),
        status: "published".into(),
        // Движение на поводу: у карты проверок доски его нет — она про числа.
        motion_wear: None,
        tier: 1,
        race_id: None,
        type_en: None,
        type_ru: None,
        title_en: slug.into(),
        title_ru: slug.into(),
        // Опубликованной карте нужен эффект (`card_readiness` → `noEffect`), и
        // без него весь этот файл падал на создании первой же карты.
        effect_en: Some("A test card.".into()),
        effect_ru: Some("Испытательная карта.".into()),
        lore_en: None,
        lore_ru: None,
        cost: 1,
        power,
        health,
        mana: 0,
        traits: Vec::new(),
        kind: "unit".into(),
        armor: 0,
        ward: 0,
        attack_channel: "physical".into(),
        reach,
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
        lendable: false,
        edition_size: None,
        figurine_id: None,
    }
}

fn placement(card: &str, x: u8, y: u8) -> ChallengePlacement {
    ChallengePlacement {
        card: card.into(),
        x,
        y,
    }
}

async fn prepare(pool: &PgPool) -> (AppService, Uuid, Uuid) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(pool).await;

    service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    service
        .admin_create_battle_card(card("voron", 4, 2, 1))
        .await
        .expect("ворон");

    let challenge = service
        .admin_save_battle_challenge(
            None,
            SaveBattleChallengeRequest {
                slug: Some("etude".into()),
                title_en: "A study".into(),
                title_ru: "Этюд".into(),
                note_en: None,
                note_ru: None,
                setup: ChallengeSetup {
                    player_board: vec![placement("boec", 1, 4)],
                    player_hand: vec!["voron".into()],
                    keeper_board: vec![placement("voron", 1, 1)],
                    keeper_hand: vec![],
                    // Правила испытания: `None` — играется домашними. Поле
                    // заведено вместе с этюдными правилами, и тесты про доску
                    // о нём ничего не говорят намеренно.
                    rules: None,
                },
                bot_depth: 1,
                reward_dust: 25,
                reward_finish_dust: 5,
                player_side: "scripted".into(),
                status: "published".into(),
            },
        )
        .await
        .expect("испытание");

    (service, user, Uuid::parse_str(&challenge.id).unwrap())
}

#[sqlx::test]
async fn renaming_a_card_carries_it_into_every_challenge(pool: PgPool) {
    // Испытания ссылаются на карты по слугу — намеренно, испытание это шаблон.
    // До этой правки переименование карты осиротило бы каждое из них молча:
    // карта пропала бы с доски, а узнал бы об этом гость.
    let (service, _user, challenge_id) = prepare(&pool).await;

    let boec = service
        .admin_list_battle_cards()
        .await
        .expect("полка")
        .into_iter()
        .find(|c| c.slug == "boec")
        .expect("боец на полке");

    let mut renamed = card("boec", 6, 3, 1);
    renamed.slug = Some("bogatyr".into());
    let saved = service
        .admin_update_battle_card(Uuid::parse_str(&boec.id).unwrap(), renamed)
        .await
        .expect("переименовать");
    assert_eq!(saved.slug, "bogatyr");

    let after = service
        .list_battle_challenges(None, false)
        .await
        .expect("этюды")
        .into_iter()
        .find(|c| c.id == challenge_id.to_string())
        .expect("этюд на месте");
    assert_eq!(
        after.setup.player_board[0].card, "bogatyr",
        "доска переехала"
    );

    // Соседа по расстановке переименование не тронуло.
    assert_eq!(after.setup.keeper_board[0].card, "voron");
    assert_eq!(after.setup.player_hand, vec!["voron".to_string()]);

    // И этюд по-прежнему играется: связь не порвана, а перенесена.
    assert!(
        service
            .begin_battle_match(_user, challenge_id)
            .await
            .is_ok()
    );
}

#[sqlx::test]
async fn a_rename_never_touches_a_slug_that_merely_contains_it(pool: PgPool) {
    // «voron» лежит внутри «voronok». Текстовая замена переписала бы половину
    // чужого имени — поэтому перенос идёт разбором JSON, а не подстановкой.
    let (service, _user, challenge_id) = prepare(&pool).await;
    service
        .admin_create_battle_card(card("voronok", 5, 2, 1))
        .await
        .expect("воронок");

    let voron = service
        .admin_list_battle_cards()
        .await
        .expect("полка")
        .into_iter()
        .find(|c| c.slug == "voron")
        .expect("ворон");

    let mut renamed = card("voron", 4, 2, 1);
    renamed.slug = Some("gratch".into());
    service
        .admin_update_battle_card(Uuid::parse_str(&voron.id).unwrap(), renamed)
        .await
        .expect("переименовать");

    let after = service
        .list_battle_challenges(None, false)
        .await
        .expect("этюды")
        .into_iter()
        .find(|c| c.id == challenge_id.to_string())
        .expect("этюд");
    assert_eq!(after.setup.keeper_board[0].card, "gratch");
    assert_eq!(after.setup.player_hand, vec!["gratch".to_string()]);
    assert_eq!(
        after.setup.player_board[0].card, "boec",
        "чужое имя не тронуто"
    );
}

#[sqlx::test]
async fn played_matches_are_read_back_with_their_tallies(pool: PgPool) {
    // Единственное окно в живую игру. До него партии писались в базу, а
    // посмотреть их было нечем: баланс правился симуляцией по правилам,
    // которых игроки не видели.
    let (service, user, challenge_id) = prepare(&pool).await;

    let empty = service.admin_read_battle_matches().await.expect("разбор");
    assert!(empty.rows.is_empty(), "ещё никто не играл");

    let mut m = service
        .begin_battle_match(user, challenge_id)
        .await
        .expect("начать");
    let mut turns = 0;
    while m.outcome.is_none() {
        let action = m.legal_actions.first().cloned().expect("законное действие");
        m = service
            .act_in_battle_match(
                user,
                Uuid::parse_str(&m.id).unwrap(),
                BattleActRequest { seq: m.seq, action },
            )
            .await
            .expect("ход");
        turns += 1;
        assert!(turns < 500, "партия не кончилась");
    }
    let outcome = m.outcome.clone().expect("исход");

    let report = service.admin_read_battle_matches().await.expect("разбор");
    assert_eq!(report.read, 1);
    let row = &report.rows[0];
    assert_eq!(row.outcome.as_deref(), Some(outcome.as_str()));
    assert_eq!(row.title_ru.as_deref(), Some("Этюд"));
    assert!(row.moves > 0, "журнал не пуст");
    assert!(row.finished_at.is_some());
    assert!(!row.guest.is_empty(), "видно, кто играл");

    // По испытанию.
    assert_eq!(report.by_challenge.len(), 1);
    let tally = &report.by_challenge[0];
    assert_eq!(tally.played, 1);
    assert_eq!(tally.unfinished, 0);
    assert_eq!(tally.guest_won + tally.keeper_won + tally.draws, 1);

    // По картам. Обе стороны расстановки названы, и у каждой карты сошлось.
    let boec = report
        .by_card
        .iter()
        .find(|c| c.slug == "boec")
        .expect("боец считан");
    let voron = report
        .by_card
        .iter()
        .find(|c| c.slug == "voron")
        .expect("ворон считан");
    assert_eq!(boec.played, 1, "боец стоял только у гостя");
    assert_eq!(boec.won + boec.lost + boec.draws, 1);
    if outcome == "player" {
        assert_eq!(boec.won, 1);
    } else if outcome == "keeper" {
        assert_eq!(boec.lost, 1);
    }

    // Ворон в этом этюде стоит ПО ОБЕ СТОРОНЫ — в руке гостя и на доске
    // хранителя. Считается он дважды, по разу за сторону, и это не сбой счёта:
    // «сколько раз карта выходила на поле» и «за кого» — разные вопросы, и
    // слить их в один значило бы потерять половину ответа.
    assert_eq!(voron.played, 2);
    if outcome == "draw" {
        assert_eq!(voron.draws, 2);
    } else {
        assert_eq!(voron.won, 1, "одна его сторона выиграла");
        assert_eq!(voron.lost, 1, "другая проиграла");
    }
    // Имя карты в сводке — нынешнее, а не слуг.
    assert_eq!(boec.title_ru.as_deref(), Some("boec"));
}

#[sqlx::test]
async fn a_recorded_match_is_reviewed_step_by_step(pool: PgPool) {
    let (service, user, challenge_id) = prepare(&pool).await;

    let mut m = service
        .begin_battle_match(user, challenge_id)
        .await
        .expect("начать");
    let id = Uuid::parse_str(&m.id).unwrap();
    let mut turns = 0;
    while m.outcome.is_none() {
        let action = m.legal_actions.first().cloned().expect("законное действие");
        m = service
            .act_in_battle_match(user, id, BattleActRequest { seq: m.seq, action })
            .await
            .expect("ход");
        turns += 1;
        assert!(turns < 500, "партия не кончилась");
    }
    let outcome = m.outcome.clone().expect("исход");

    // Ступень ноль — расстановка до первого хода.
    let opening = service
        .admin_replay_battle_match(id, 0)
        .await
        .expect("начало");
    assert_eq!(opening.upto, 0);
    assert!(opening.total > 0, "журнал не пуст");
    assert!(
        opening.outcome.is_none(),
        "до первого хода никто не победил"
    );
    assert!(!opening.diverged);
    assert!(opening.events.is_empty(), "ещё ничего не произошло");

    // Середина: доска другая, партия ещё идёт.
    let middle = service
        .admin_replay_battle_match(id, opening.total / 2)
        .await
        .expect("середина");
    assert_eq!(middle.upto, opening.total / 2);
    assert_ne!(middle.state, opening.state, "доска сдвинулась");

    // Конец: тот же исход, что записан. Это и есть проверка, что пересмотр
    // не врёт: журнал переигран теми же правилами, какими был сыгран.
    let end = service
        .admin_replay_battle_match(id, opening.total)
        .await
        .expect("конец");
    assert_eq!(end.upto, end.total);
    assert_eq!(end.outcome.as_deref(), Some(outcome.as_str()));
    assert!(!end.diverged, "запись переигрывается без расхождений");
    assert!(!end.events.is_empty(), "последняя ступень что-то сделала");

    // Ступень за концом журнала обрезается, а не падает.
    let past = service
        .admin_replay_battle_match(id, 10_000)
        .await
        .expect("за концом");
    assert_eq!(past.upto, past.total);
    assert_eq!(past.state, end.state);
}

#[sqlx::test]
async fn a_challenge_is_played_to_its_end_and_pays_once(pool: PgPool) {
    let (service, user, challenge_id) = prepare(&pool).await;

    let mut m = service
        .begin_battle_match(user, challenge_id)
        .await
        .expect("начать");
    assert!(m.outcome.is_none());
    assert!(!m.legal_actions.is_empty(), "гостю есть чем ходить");

    // Играем как игрок: берём первое законное действие, пока партия идёт.
    let mut paid = 0;
    let mut turns = 0;
    while m.outcome.is_none() {
        let action = m.legal_actions.first().cloned().expect("законное действие");
        m = service
            .act_in_battle_match(
                user,
                Uuid::parse_str(&m.id).unwrap(),
                BattleActRequest { seq: m.seq, action },
            )
            .await
            .expect("ход");
        paid += m.reward_dust;
        turns += 1;
        assert!(turns < 500, "партия не кончилась");
    }

    assert!(m.outcome.is_some(), "партия кончилась исходом");
    let balance = service.battle_dust_balance(user).await.unwrap();
    // Две выплаты, два ключа: за доведённое до конца — всегда, за победу —
    // только победителю. Проигравший больше не получает ровно ничего: это был
    // худший из возможных ответов на первую же попытку сыграть.
    //
    // В книге сверх этого лежит ещё пятнадцать: поручение тропы «довести этюд
    // до конца» закрывается этой же партией (`20260907000000_battle_errands`).
    // Оно тоже платит однажды и тоже независимо от исхода.
    const ERRAND_FINISH: i64 = 15;
    if m.outcome.as_deref() == Some("player") {
        assert_eq!(paid, 30, "за конец пять и за победу двадцать пять");
        assert_eq!(balance, 30 + ERRAND_FINISH);
    } else {
        assert_eq!(paid, 5, "за доведённое до конца платят и без победы");
        assert_eq!(balance, 5 + ERRAND_FINISH);
    }

    // И ферма не открылась: тот же этюд, сыгранный заново, не платит ничего.
    let mut m = service
        .restart_battle_match(user, challenge_id)
        .await
        .expect("переиграть");
    let mut turns = 0;
    let mut twice = 0;
    while m.outcome.is_none() {
        let action = m.legal_actions.first().cloned().expect("законное действие");
        m = service
            .act_in_battle_match(
                user,
                Uuid::parse_str(&m.id).unwrap(),
                BattleActRequest { seq: m.seq, action },
            )
            .await
            .expect("ход");
        twice += m.reward_dust;
        turns += 1;
        assert!(turns < 500, "партия не кончилась");
    }
    assert_eq!(twice, 0, "за переигранный этюд не платят второй раз");
    assert_eq!(
        service.battle_dust_balance(user).await.unwrap(),
        balance,
        "книга не выросла"
    );
}

#[sqlx::test]
async fn a_repeated_move_changes_nothing(pool: PgPool) {
    // Двойной щелчок по кнопке — самый обычный способ сходить дважды.
    let (service, user, challenge_id) = prepare(&pool).await;
    let m = service
        .begin_battle_match(user, challenge_id)
        .await
        .unwrap();
    let match_id = Uuid::parse_str(&m.id).unwrap();
    let action = m.legal_actions.first().cloned().unwrap();

    let first = service
        .act_in_battle_match(
            user,
            match_id,
            BattleActRequest {
                seq: m.seq,
                action: action.clone(),
            },
        )
        .await
        .unwrap();
    let again = service
        .act_in_battle_match(user, match_id, BattleActRequest { seq: m.seq, action })
        .await
        .unwrap();

    assert_eq!(again.seq, first.seq, "номер не сдвинулся");
    assert_eq!(again.state, first.state, "доска та же");
    assert!(again.events.is_empty(), "второй раз ничего не произошло");
}

#[sqlx::test]
async fn a_guest_who_comes_back_continues_the_same_match(pool: PgPool) {
    let (service, user, challenge_id) = prepare(&pool).await;
    let first = service
        .begin_battle_match(user, challenge_id)
        .await
        .unwrap();
    let again = service
        .begin_battle_match(user, challenge_id)
        .await
        .unwrap();
    assert_eq!(first.id, again.id, "второй клик не заводит вторую партию");
}

#[sqlx::test]
async fn a_match_is_read_back_from_its_journal(pool: PgPool) {
    // Кэш доски — удобство. Истина — журнал; выбросив кэш, состояние обязано
    // получиться то же самое.
    let (service, user, challenge_id) = prepare(&pool).await;
    let m = service
        .begin_battle_match(user, challenge_id)
        .await
        .unwrap();
    let match_id = Uuid::parse_str(&m.id).unwrap();
    let action = m.legal_actions.first().cloned().unwrap();
    let after = service
        .act_in_battle_match(user, match_id, BattleActRequest { seq: m.seq, action })
        .await
        .unwrap();

    sqlx::query("UPDATE battle_matches SET board_cache = NULL WHERE id = $1")
        .bind(match_id)
        .execute(&pool)
        .await
        .unwrap();

    let replayed = service.read_battle_match(user, match_id).await.unwrap();
    assert_eq!(
        replayed.state, after.state,
        "свёртка журнала даёт ту же доску"
    );
}

#[sqlx::test]
async fn a_challenge_naming_an_unknown_card_is_refused_when_it_is_written(pool: PgPool) {
    // Расстановка, которую нельзя поднять, — это не испытание, а ловушка;
    // отказать надо хранителю за столом, а не гостю при клике.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let refused = service
        .admin_save_battle_challenge(
            None,
            SaveBattleChallengeRequest {
                slug: None,
                title_en: "Broken".into(),
                title_ru: "Сломанный".into(),
                note_en: None,
                note_ru: None,
                setup: ChallengeSetup {
                    player_board: vec![placement("нет-такой", 1, 4)],
                    player_hand: vec![],
                    keeper_board: vec![placement("нет-такой", 1, 1)],
                    keeper_hand: vec![],
                    // Правила испытания: `None` — играется домашними. Поле
                    // заведено вместе с этюдными правилами, и тесты про доску
                    // о нём ничего не говорят намеренно.
                    rules: None,
                },
                bot_depth: 1,
                reward_dust: 0,
                reward_finish_dust: 0,
                player_side: "scripted".into(),
                status: "published".into(),
            },
        )
        .await;
    assert!(refused.is_err());
}

// ── Стол гостя и встреча ─────────────────────────────────────────────────────
//
// Ровно тот стык, ради которого этот файл существует: колода лежит в базе,
// заём приходит из словаря карт, а на поле встаёт снимок. Ни одну из трёх
// частей нельзя проверить по отдельности так, чтобы это что-то значило.

/// Испытание, где сторону гостя приносит его стол.
async fn meeting(service: &AppService, slug: &str) -> Uuid {
    let saved = service
        .admin_save_battle_challenge(
            None,
            SaveBattleChallengeRequest {
                slug: Some(slug.into()),
                title_en: "A meeting".into(),
                title_ru: "Встреча".into(),
                note_en: None,
                note_ru: None,
                // Половины гостя здесь НЕТ и быть не должно: её приносит стол.
                setup: ChallengeSetup {
                    player_board: vec![],
                    player_hand: vec![],
                    keeper_board: vec![placement("voron", 1, 1)],
                    keeper_hand: vec![],
                    // Правила испытания: `None` — играется домашними. Поле
                    // заведено вместе с этюдными правилами, и тесты про доску
                    // о нём ничего не говорят намеренно.
                    rules: None,
                },
                bot_depth: 1,
                reward_dust: 0,
                reward_finish_dust: 0,
                player_side: "deck".into(),
                status: "published".into(),
            },
        )
        .await
        .expect("встреча");
    Uuid::parse_str(&saved.id).unwrap()
}

#[sqlx::test]
async fn a_meeting_needs_no_half_from_the_keeper_for_the_guest(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    service
        .admin_create_battle_card(card("voron", 4, 2, 1))
        .await
        .expect("ворон");
    // Требовать половину гостя от хранителя значило бы требовать чужого.
    meeting(&service, "vstrecha").await;
}

#[sqlx::test]
async fn a_guest_who_owns_nothing_still_takes_the_field(pool: PgPool) {
    // Это и есть весь смысл заёма: замок откладывал бы игру до собранной
    // коллекции, а игра — единственная причина коллекцию собирать.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let mut lent = card("posoh", 5, 2, 1);
    lent.lendable = true;
    service
        .admin_create_battle_card(lent)
        .await
        .expect("заёмная");
    service
        .admin_create_battle_card(card("voron", 4, 2, 1))
        .await
        .expect("ворон");
    let challenge = meeting(&service, "vstrecha").await;

    let table = service.read_battle_deck(user).await.expect("стол");
    assert!(!table.laid, "стол ещё ни разу не раскладывали");
    assert!(!table.nothing_to_lend, "дому есть что одолжить");
    assert!(
        table
            .board
            .iter()
            .all(|s| s.card_id.is_none() && s.lent_card_id.is_some()),
        "все три места на поле закрыты заёмом",
    );

    let mat = service
        .begin_battle_match(user, challenge)
        .await
        .expect("партия");
    assert_eq!(
        mat.state
            .board
            .occupied()
            .filter(|(cell, _)| cell.y >= 3)
            .count(),
        3,
        "на половине гостя стоят три одолженных тела",
    );
}

#[sqlx::test]
async fn the_house_lends_nothing_it_was_not_asked_to(pool: PgPool) {
    // `lendable` заводится с `FALSE`: карта не становится заёмной от того, что
    // она просто есть на полке.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    service
        .admin_create_battle_card(card("voron", 4, 2, 1))
        .await
        .expect("ворон");

    let table = service.read_battle_deck(user).await.expect("стол");
    assert!(
        table.nothing_to_lend,
        "дому нечего одолжить, и он говорит это вслух"
    );
    assert!(table.board.iter().all(|s| s.lent_card_id.is_none()));

    let challenge = meeting(&service, "vstrecha").await;
    let refused = service.begin_battle_match(user, challenge).await;
    assert!(
        matches!(&refused, Err(e) if e.to_string().contains("nothingToBring")),
        "пустая половина — партия, проигранная до первого хода: {refused:?}",
    );
}

#[sqlx::test]
async fn your_own_card_stands_where_you_put_it(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let boec = service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    service
        .admin_create_battle_card(card("voron", 4, 2, 1))
        .await
        .expect("ворон");
    let mine = Uuid::parse_str(&boec.id).unwrap();
    sqlx::query(
        "INSERT INTO card_copies (owner_id, card_id, origin) VALUES ($1, $2, 'gift')",
    )
        .bind(user)
        .bind(mine)
        .execute(&pool)
        .await
        .expect("своя карта");

    let saved = service
        .save_battle_deck(
            user,
            &SaveBattleDeckRequest {
                board: vec![DeckPlacement {
                    card: mine,
                    x: 2,
                    y: 5,
                }],
                hand: vec![],
            },
        )
        .await
        .expect("стол разложен");
    assert!(saved.laid);
    assert_eq!(
        saved
            .board
            .iter()
            .find(|s| s.card_id.is_some())
            .map(|s| (s.x, s.y)),
        Some((Some(2), Some(5))),
        "своя карта осталась на выбранной клетке",
    );

    let challenge = meeting(&service, "vstrecha").await;
    let mat = service
        .begin_battle_match(user, challenge)
        .await
        .expect("партия");
    assert!(
        mat.state
            .board
            .occupied()
            .any(|(cell, _)| cell.x == 2 && cell.y == 5),
        "на поле она встала туда же",
    );
}

#[sqlx::test]
async fn a_card_that_is_not_yours_does_not_lie_on_your_table(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let boec = service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    let theirs = Uuid::parse_str(&boec.id).unwrap();

    let refused = service
        .save_battle_deck(
            user,
            &SaveBattleDeckRequest {
                board: vec![DeckPlacement {
                    card: theirs,
                    x: 1,
                    y: 4,
                }],
                hand: vec![],
            },
        )
        .await;
    assert!(
        matches!(&refused, Err(e) if e.to_string().contains("notYours")),
        "чужая карта отказана словом, а не текстом: {refused:?}",
    );
}

#[sqlx::test]
async fn the_keepers_half_is_refused_at_the_table(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let boec = service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    let mine = Uuid::parse_str(&boec.id).unwrap();
    sqlx::query(
        "INSERT INTO card_copies (owner_id, card_id, origin) VALUES ($1, $2, 'gift')",
    )
        .bind(user)
        .bind(mine)
        .execute(&pool)
        .await
        .expect("своя карта");

    let refused = service
        .save_battle_deck(
            user,
            // Ряд 1 — половина хранителя. Начать партию в его тылу нельзя.
            &SaveBattleDeckRequest {
                board: vec![DeckPlacement {
                    card: mine,
                    x: 1,
                    y: 1,
                }],
                hand: vec![],
            },
        )
        .await;
    assert!(
        matches!(&refused, Err(e) if e.to_string().contains("notYourHalf")),
        "{refused:?}",
    );
}

#[sqlx::test]
async fn a_card_taken_off_the_shelf_keeps_its_place_and_is_stood_in_for(pool: PgPool) {
    // §1.5: молча выбросить её значило бы переписать чужую расстановку за
    // спиной; отказать в партии — наказать гостя за правку хранителя.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let boec = service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    let mut lent = card("posoh", 5, 2, 1);
    lent.lendable = true;
    service
        .admin_create_battle_card(lent)
        .await
        .expect("заёмная");
    let mine = Uuid::parse_str(&boec.id).unwrap();
    sqlx::query(
        "INSERT INTO card_copies (owner_id, card_id, origin) VALUES ($1, $2, 'gift')",
    )
        .bind(user)
        .bind(mine)
        .execute(&pool)
        .await
        .expect("своя карта");
    service
        .save_battle_deck(
            user,
            &SaveBattleDeckRequest {
                board: vec![DeckPlacement {
                    card: mine,
                    x: 1,
                    y: 4,
                }],
                hand: vec![],
            },
        )
        .await
        .expect("стол разложен");

    // Хранитель снимает карту с полки уже после того, как её положили на стол.
    let mut retired = card("boec", 6, 3, 1);
    retired.status = "retired".into();
    service
        .admin_update_battle_card(mine, retired)
        .await
        .expect("снята");

    let table = service.read_battle_deck(user).await.expect("стол");
    let slot = table
        .board
        .iter()
        .find(|s| s.card_id.as_deref() == Some(&mine.to_string()));
    let slot = slot.expect("карта осталась в колоде, а не исчезла");
    assert!(slot.gone, "и помечена снятой");
    assert!(slot.lent_card_id.is_some(), "а место закрывает заём");
}

// ── Из рук ───────────────────────────────────────────────────────────────────
//
// Единственный способ, которым в доме появляется корм. Здесь проверяется не
// арифметика (её проверяет книга), а то, ради чего книга такая: повтор не
// раздаёт дважды, а два разных акта не сливаются в один.

fn grant(user: Uuid, coin: &str, amount: i32, note: &str, key: &str) -> GrantBattleCoinRequest {
    GrantBattleCoinRequest {
        user_id: user,
        currency: coin.into(),
        amount,
        note: Some(note.into()),
        idem_key: key.into(),
    }
}

#[sqlx::test]
async fn feed_is_given_by_hand_and_shows_its_note(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;

    let given = service
        .admin_grant_battle_coin(&grant(user, "feed", 3, "за состоявшийся показ", "a"))
        .await
        .expect("выдача");
    assert!(given.granted_now);
    assert_eq!(given.balance, 3);

    // Число без записки было бы просто выросшим счётчиком — тем самым, от чего
    // эта комната отказывается.
    let me = service.battle_me(user).await.expect("книга");
    assert_eq!(me.feed, 3);
    assert_eq!(me.gifts.len(), 1);
    assert_eq!(me.gifts[0].note.as_deref(), Some("за состоявшийся показ"));
    assert_eq!(me.gifts[0].currency, "feed");
}

#[sqlx::test]
async fn the_same_key_does_not_give_twice(pool: PgPool) {
    // Двойной щелчок по «дать» — классическая двойная выдача, и ключ книги
    // делает второй щелчок пустым действием, а не второй выдачей.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;

    let first = service
        .admin_grant_battle_coin(&grant(user, "feed", 5, "за впечатление", "same"))
        .await
        .expect("первая");
    let second = service
        .admin_grant_battle_coin(&grant(user, "feed", 5, "за впечатление", "same"))
        .await
        .expect("повтор");

    assert!(first.granted_now && !second.granted_now);
    assert_eq!(second.balance, 5, "баланс не удвоился");
}

#[sqlx::test]
async fn two_deliberate_grants_do_not_merge(pool: PgPool) {
    // Два состоявшихся показа одному гостю — это две выдачи, и ключ у них
    // разный, потому что ключ у АКТА, а не у его содержимого.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    service
        .admin_grant_battle_coin(&grant(user, "feed", 5, "показ", "one"))
        .await
        .unwrap();
    let twice = service
        .admin_grant_battle_coin(&grant(user, "feed", 5, "показ ещё раз", "two"))
        .await
        .expect("вторая");
    assert!(twice.granted_now);
    assert_eq!(twice.balance, 10);
}

#[sqlx::test]
async fn a_mistake_is_corrected_by_an_opposite_row(pool: PgPool) {
    // Книга неизменяема: ошибка правится обратной строкой, а не правкой той,
    // что была неверна. Поэтому выдача умеет быть со знаком минус.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    service
        .admin_grant_battle_coin(&grant(user, "feed", 50, "опечатка", "oops"))
        .await
        .unwrap();
    let fixed = service
        .admin_grant_battle_coin(&grant(user, "feed", -45, "поправка к опечатке", "fix"))
        .await
        .expect("поправка");
    assert_eq!(fixed.balance, 5);

    // Поправка — не подарок: на полях у гостя она не показывается.
    let me = service.battle_me(user).await.expect("книга");
    assert_eq!(
        me.gifts.len(),
        1,
        "видна только сама выдача, не её исправление"
    );
}

#[sqlx::test]
async fn a_grant_without_a_note_is_refused(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let mut mute = grant(user, "feed", 1, "", "quiet");
    mute.note = None;
    assert!(service.admin_grant_battle_coin(&mute).await.is_err());

    let blank = grant(user, "feed", 1, "   ", "blank");
    assert!(service.admin_grant_battle_coin(&blank).await.is_err());
}

#[sqlx::test]
async fn a_grant_of_nothing_and_an_unknown_coin_are_refused(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    assert!(
        service
            .admin_grant_battle_coin(&grant(user, "feed", 0, "ничего", "z"))
            .await
            .is_err()
    );
    // Потолок — не про баланс, а про опечатку: лишний ноль в поле хранителя не
    // должен становиться экономикой.
    assert!(
        service
            .admin_grant_battle_coin(&grant(user, "feed", 999_999, "много", "z"))
            .await
            .is_err()
    );
    assert!(
        service
            .admin_grant_battle_coin(&grant(user, "gold", 1, "золото", "z"))
            .await
            .is_err()
    );
}

#[sqlx::test]
async fn a_card_priced_only_in_feed_can_finally_be_taken(pool: PgPool) {
    // То, ради чего это делалось: до сих пор такая карта была недостижима ни
    // для кого — вторая монета была объяснена гостю, но взяться ей было неоткуда.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let mut only_feed = card("voron", 4, 2, 1);
    only_feed.price_dust = None;
    only_feed.price_feed = Some(3);
    let made = service
        .admin_create_battle_card(only_feed)
        .await
        .expect("карта");
    let card_id = Uuid::parse_str(&made.id).unwrap();

    let broke = service
        .buy_battle_card(
            user,
            &gotiga_server::models::BuyBattleCardRequest {
                card_id,
                currency: "feed".into(),
                expected_price: 3,
            },
        )
        .await;
    assert!(broke.is_err(), "без корма карта не берётся");

    service
        .admin_grant_battle_coin(&grant(user, "feed", 3, "за заказ работы", "order-1"))
        .await
        .expect("корм из рук");
    let taken = service
        .buy_battle_card(
            user,
            &gotiga_server::models::BuyBattleCardRequest {
                card_id,
                currency: "feed".into(),
                expected_price: 3,
            },
        )
        .await
        .expect("теперь берётся");
    assert!(taken.taken_now);
    assert_eq!(taken.balance, 0);
}

// ── Прямая выдача карт ───────────────────────────────────────────────────────
//
// Инструмент для проверки игры: без него собрание приводится в нужное
// состояние только через начисление монет, вход под гостем и покупку по одной.

#[sqlx::test]
async fn every_card_can_be_given_at_once(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    service
        .admin_create_battle_card(card("voron", 4, 2, 1))
        .await
        .expect("ворон");

    let given = service
        .admin_give_battle_cards(&GiveBattleCardsRequest {
            user_id: user,
            card_ids: vec![],
            all: true,
            level: 3,
        })
        .await
        .expect("выдача");
    assert_eq!(given.touched, 2);

    let me = service.battle_me(user).await.expect("книга");
    assert_eq!(me.owned.len(), 2);
    assert!(
        me.owned.iter().all(|o| o.level == 3),
        "уровень выдан тот, что просили"
    );
    // Подарок, а не покупка: кошелёк не тронут.
    assert_eq!(me.dust, 0);
    assert_eq!(me.feed, 0);
}

#[sqlx::test]
async fn giving_again_only_moves_the_level(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");

    for level in [1, 5] {
        service
            .admin_give_battle_cards(&GiveBattleCardsRequest {
                user_id: user,
                card_ids: vec![],
                all: true,
                level,
            })
            .await
            .expect("выдача");
    }
    let me = service.battle_me(user).await.expect("книга");
    assert_eq!(me.owned.len(), 1, "карта не задвоилась");
    assert_eq!(me.owned[0].level, 5, "уровень переписан намеренно");
}

#[sqlx::test]
async fn a_card_with_no_health_is_never_given(pool: PgPool) {
    // Выдать тело без здоровья значило бы выдать то, что падает в тот же миг,
    // как встанет, — и оно бы молча заняло место в колоде.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");

    // Опубликованную карту без здоровья служба теперь не даёт завести вовсе.
    // Но заведённые ДО этого правила никуда не делись, и выдача обязана их
    // пропускать — поэтому такая карта кладётся сюда прямо в базу, мимо
    // службы: ровно так она в базе и оказалась бы.
    sqlx::query(
        "INSERT INTO battle_cards (slug, status, tier, title_en, title_ru, cost, power, health, price_dust)
         VALUES ('prizrak', 'published', 1, 'ghost', 'призрак', 1, 3, 0, 10)",
    )
    .execute(&pool)
    .await
    .expect("наследие в базе");

    service
        .admin_give_battle_cards(&GiveBattleCardsRequest {
            user_id: user,
            card_ids: vec![],
            all: true,
            level: 1,
        })
        .await
        .expect("выдача");
    let me = service.battle_me(user).await.expect("книга");
    assert_eq!(
        me.owned.len(),
        1,
        "выдана только та, что может выйти на поле"
    );
}

#[sqlx::test]
async fn cards_can_be_taken_all_the_way_back(pool: PgPool) {
    // Единственный способ проверить пустое собрание и временные карты,
    // которые его закрывают: обратно карты сами не уходят.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    service
        .admin_create_battle_card(card("voron", 4, 2, 1))
        .await
        .expect("ворон");
    service
        .admin_give_battle_cards(&GiveBattleCardsRequest {
            user_id: user,
            card_ids: vec![],
            all: true,
            level: 1,
        })
        .await
        .expect("выдача");

    let taken = service
        .admin_revoke_battle_cards(&RevokeBattleCardsRequest {
            user_id: user,
            card_ids: vec![],
            all: true,
        })
        .await
        .expect("забрали");
    assert_eq!(taken.touched, 2);
    assert!(service.battle_me(user).await.unwrap().owned.is_empty());
}

#[sqlx::test]
async fn revoking_nothing_in_particular_takes_nothing(pool: PgPool) {
    // Пустой список БЕЗ `all` — это не «забрать всё»: иначе пустой запрос,
    // отправленный по ошибке, вычистил бы чужое собрание целиком.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    service
        .admin_give_battle_cards(&GiveBattleCardsRequest {
            user_id: user,
            card_ids: vec![],
            all: true,
            level: 1,
        })
        .await
        .expect("выдача");

    let nothing = service
        .admin_revoke_battle_cards(&RevokeBattleCardsRequest {
            user_id: user,
            card_ids: vec![],
            all: false,
        })
        .await
        .expect("ничего не названо");
    assert_eq!(nothing.touched, 0);
    assert_eq!(service.battle_me(user).await.unwrap().owned.len(), 1);
}

// ── Экземпляры ──────────────────────────────────────────────────────────────
//
// Владение — это не отметка «у него есть эта карта», а строка про ВОТ ЭТУ
// вещь: у неё есть номер, свой уровень и один владелец. Пока владение было
// парой (человек, карта), правило «продал — у тебя больше нет» было
// невыразимо: продавать нечего, когда вещи нет.

#[sqlx::test]
async fn two_people_take_the_same_card_and_get_different_copies(pool: PgPool) {
    let service = AppService::new(Repository::new(pool.clone()), config());
    let first = seed_user(&pool).await;
    let second = seed_user(&pool).await;
    let made = service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    let card_id = Uuid::parse_str(&made.id).unwrap();

    for who in [first, second] {
        service
            .admin_grant_battle_coin(&grant(who, "dust", 10, "на карту", &who.to_string()))
            .await
            .expect("пыль");
        service
            .buy_battle_card(
                who,
                &gotiga_server::models::BuyBattleCardRequest {
                    card_id,
                    currency: "dust".into(),
                    expected_price: 10,
                },
            )
            .await
            .expect("взял");
    }

    let numbers: Vec<Option<i32>> = sqlx::query_scalar(
        "SELECT serial FROM card_copies WHERE card_id = $1 ORDER BY acquired_at",
    )
    .bind(card_id)
    .fetch_all(&pool)
    .await
    .expect("экземпляры");
    assert_eq!(numbers, vec![Some(1), Some(2)], "у каждого свой номер");
}

#[sqlx::test]
async fn an_edition_that_is_out_takes_nothing_from_the_purse(pool: PgPool) {
    // Взять пыль за ненапечатанное — худшее, что умеет сделать лавка. Поэтому
    // номер чеканится ДО списания, и кончившийся тираж откатывает всю сделку.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let lucky = seed_user(&pool).await;
    let late = seed_user(&pool).await;
    let made = service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    let card_id = Uuid::parse_str(&made.id).unwrap();
    sqlx::query("UPDATE battle_cards SET edition_size = 1 WHERE id = $1")
        .bind(card_id)
        .execute(&pool)
        .await
        .expect("тираж в одну штуку");

    let want = gotiga_server::models::BuyBattleCardRequest {
        card_id,
        currency: "dust".into(),
        expected_price: 10,
    };
    for who in [lucky, late] {
        service
            .admin_grant_battle_coin(&grant(who, "dust", 10, "на карту", &who.to_string()))
            .await
            .expect("пыль");
    }
    service
        .buy_battle_card(lucky, &want)
        .await
        .expect("первому досталось");
    assert!(
        service.buy_battle_card(late, &want).await.is_err(),
        "второму тираж не позволил"
    );

    let me = service.battle_me(late).await.expect("книга");
    assert!(me.owned.is_empty(), "карты нет");
    assert_eq!(me.dust, 10, "и пыль на месте");
}

#[sqlx::test]
async fn a_number_taken_back_does_not_come_round_again(pool: PgPool) {
    // Отобранный экземпляр не возвращает номер в тираж: «№1 из 100» у человека
    // на руках и «№1 из 100» у следующего покупателя — это два номера один,
    // и опознать вещь по номеру больше нельзя.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let made = service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    let card_id = Uuid::parse_str(&made.id).unwrap();
    service
        .admin_grant_battle_coin(&grant(user, "dust", 100, "на карты", "much"))
        .await
        .expect("пыль");

    service
        .admin_give_battle_cards(&GiveBattleCardsRequest {
            user_id: user,
            card_ids: vec![card_id],
            all: false,
            level: 1,
        })
        .await
        .expect("подарок");
    service
        .admin_revoke_battle_cards(&RevokeBattleCardsRequest {
            user_id: user,
            card_ids: vec![card_id],
            all: false,
        })
        .await
        .expect("забрали");
    service
        .buy_battle_card(
            user,
            &gotiga_server::models::BuyBattleCardRequest {
                card_id,
                currency: "dust".into(),
                expected_price: 10,
            },
        )
        .await
        .expect("купил");

    let serial: Option<i32> =
        sqlx::query_scalar("SELECT serial FROM card_copies WHERE card_id = $1")
            .bind(card_id)
            .fetch_one(&pool)
            .await
            .expect("экземпляр");
    assert_eq!(serial, Some(2), "первый номер сгорел вместе с экземпляром");
}

#[sqlx::test]
async fn an_edition_cannot_be_set_below_what_is_already_printed(pool: PgPool) {
    // Иначе «из 50», под которым лежит семидесятый экземпляр, — число, которое
    // врёт молча, а на руках у людей уже номера.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let made = service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    let card_id = Uuid::parse_str(&made.id).unwrap();
    service
        .admin_give_battle_cards(&GiveBattleCardsRequest {
            user_id: user,
            card_ids: vec![card_id],
            all: false,
            level: 1,
        })
        .await
        .expect("подарок");

    let mut free = card("boec", 6, 3, 1);
    free.edition_size = Some(0);
    service
        .admin_update_battle_card(card_id, free)
        .await
        .expect("ноль — это «не ограничивать», а не «ни одной»");

    let mut exact = card("boec", 6, 3, 1);
    exact.edition_size = Some(1);
    service
        .admin_update_battle_card(card_id, exact)
        .await
        .expect("ровно отпечатанное назначить можно");

    // Второй подарок печатает второй экземпляр — и тираж в одну штуку
    // становится ложью, которую дом обязан не принять.
    let another = seed_user(&pool).await;
    service
        .admin_give_battle_cards(&GiveBattleCardsRequest {
            user_id: another,
            card_ids: vec![card_id],
            all: false,
            level: 1,
        })
        .await
        .expect("второй подарок");
    let mut lying = card("boec", 6, 3, 1);
    lying.edition_size = Some(1);
    assert!(
        service
            .admin_update_battle_card(card_id, lying)
            .await
            .is_err(),
        "тираж ниже отпечатанного не назначается"
    );
}

#[sqlx::test]
async fn a_deck_of_hand_only_still_plays(pool: PgPool) {
    // Пустое поле при непустой руке — законная колода: `is_spent` считает
    // сторону вышедшей из игры, только если у неё нет ни стоящих тел, ни карты,
    // которую есть на что выложить и куда. Первым ходом карты выставляются.
    //
    // Здесь стояла лишняя проверка на стороне сервера, и она отвергала такую
    // колоду со словами «в колоде нет ни одной карты», когда карт было три.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    let boec = service
        .admin_create_battle_card(card("boec", 6, 3, 1))
        .await
        .expect("боец");
    service
        .admin_create_battle_card(card("voron", 4, 2, 1))
        .await
        .expect("ворон");
    let mine = Uuid::parse_str(&boec.id).unwrap();
    sqlx::query(
        "INSERT INTO card_copies (owner_id, card_id, origin) VALUES ($1, $2, 'gift')",
    )
        .bind(user)
        .bind(mine)
        .execute(&pool)
        .await
        .expect("своя карта");

    service
        .save_battle_deck(
            user,
            &SaveBattleDeckRequest {
                board: vec![],
                hand: vec![mine],
            },
        )
        .await
        .expect("колода только из руки — законна");

    let challenge = meeting(&service, "vstrecha").await;
    let mat = service
        .begin_battle_match(user, challenge)
        .await
        .expect("бой начинается");
    assert!(mat.outcome.is_none(), "бой не проигран до первого хода");
    assert_eq!(
        mat.state
            .board
            .occupied()
            .filter(|(cell, _)| cell.y >= 3)
            .count(),
        0,
        "на поле никого — и это нормально",
    );
    assert_eq!(mat.state.player.hand.len(), 1, "карта в руке");
    // Первым ходом её можно выставить: сервер сам присылает список законных
    // действий, и «выложить» в нём есть.
    assert!(
        mat.legal_actions
            .iter()
            .any(|a| matches!(a, battle_core::Action::Play { .. })),
        "выложить карту с руки — законное действие",
    );
}

#[sqlx::test]
async fn a_deck_with_nothing_at_all_is_still_refused(pool: PgPool) {
    // Отказ остаётся там, где приводить действительно нечего.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let user = seed_user(&pool).await;
    service
        .admin_create_battle_card(card("voron", 4, 2, 1))
        .await
        .expect("ворон");
    let challenge = meeting(&service, "vstrecha").await;

    let refused = service.begin_battle_match(user, challenge).await;
    assert!(
        matches!(&refused, Err(e) if e.to_string().contains("nothingToBring")),
        "{refused:?}",
    );
}

// ── Годность карты ───────────────────────────────────────────────────────────
//
// Хранитель не должен иметь возможности выложить на полку карту, за которую
// гость заплатит настоящей пылью и не сможет ничего с ней сделать.

#[sqlx::test]
async fn a_published_card_without_health_is_refused(pool: PgPool) {
    // `can_take_the_field` требует здоровья: без него карту можно купить, но
    // нельзя ни положить в колоду, ни выставить на поле.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let mut lifeless = card("prizrak", 0, 3, 1);
    lifeless.status = "published".into();
    let refused = service.admin_create_battle_card(lifeless).await;
    assert!(
        matches!(&refused, Err(e) if e.to_string().contains("noHealth")),
        "{refused:?}",
    );
}

#[sqlx::test]
async fn a_draft_without_health_is_allowed(pool: PgPool) {
    // Черновик имеет право быть недоделанным — он затем и черновик.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let mut unfinished = card("nabrosok", 0, 3, 1);
    unfinished.status = "draft".into();
    service
        .admin_create_battle_card(unfinished)
        .await
        .expect("черновик сохраняется");
}

#[sqlx::test]
async fn a_card_costing_more_than_mana_can_reach_is_refused(pool: PgPool) {
    // Мана растёт по единице за ход и упирается в потолок. Карта дороже него
    // не может быть выложена ни на каком ходу.
    let service = AppService::new(Repository::new(pool.clone()), config());
    let mut dear = card("nepodyomnaya", 6, 3, 1);
    dear.status = "published".into();
    dear.cost = gotiga_server::battles::MANA_CEILING + 1;
    let refused = service.admin_create_battle_card(dear).await;
    assert!(
        matches!(&refused, Err(e) if e.to_string().contains("costBeyondMana")),
        "{refused:?}",
    );

    let mut at_the_ceiling = card("na-potolke", 6, 3, 1);
    at_the_ceiling.slug = Some("na-potolke".into());
    at_the_ceiling.status = "published".into();
    at_the_ceiling.cost = gotiga_server::battles::MANA_CEILING;
    service
        .admin_create_battle_card(at_the_ceiling)
        .await
        .expect("ровно потолок — ещё оплатима");
}

#[sqlx::test]
async fn the_scales_say_the_same_thing_the_refusal_will(pool: PgPool) {
    // Предупреждение хранителю и отказ при сохранении — один разбор. Иначе он
    // увидит предупреждение об одном, а отказ получит о другом.
    let mut lifeless = card("prizrak", 0, 3, 1);
    lifeless.status = "published".into();
    let weighed = AppService::weigh_battle_card(&lifeless);
    assert_eq!(weighed.readiness.blocking, vec!["noHealth".to_string()]);

    let service = AppService::new(Repository::new(pool.clone()), config());
    let refused = service.admin_create_battle_card(lifeless).await;
    assert!(matches!(&refused, Err(e) if e.to_string().contains("noHealth")));
}

#[test]
fn notes_warn_without_refusing() {
    // Замечания не мешают сохранить — они мешают ошибиться молча.
    let mut free = card("darom", 6, 3, 1);
    free.status = "published".into();
    free.price_dust = Some(0);
    let weighed = AppService::weigh_battle_card(&free);
    assert!(
        weighed.readiness.blocking.is_empty(),
        "ноль — не препятствие"
    );
    assert!(weighed.readiness.notes.iter().any(|n| n == "freeForACoin"));

    // Ноль и пусто — разные вещи, и разница видна только гостю.
    let mut only_feed = card("tolko-korm", 6, 3, 1);
    only_feed.status = "published".into();
    only_feed.price_dust = None;
    only_feed.price_feed = Some(5);
    assert!(
        AppService::weigh_battle_card(&only_feed)
            .readiness
            .notes
            .iter()
            .any(|n| n == "onlyForFeed"),
    );

    // Одалживается только первый чин — отметка на третьем ничего не даст.
    let mut lent = card("zaemnaya", 6, 3, 1);
    lent.status = "published".into();
    lent.tier = 3;
    lent.lendable = true;
    assert!(
        AppService::weigh_battle_card(&lent)
            .readiness
            .notes
            .iter()
            .any(|n| n == "lendableNotFirstTier"),
    );
}
