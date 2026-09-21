//! Путь работы целиком: стол → хозяин → неделя → вердикт → плата.
//!
//! Здесь проверяется то, что бывает ТОЛЬКО НА СТЫКЕ и чего не увидеть ни одной
//! проверкой чистой функции. Все настоящие поломки студии были именно тут:
//! допуск не ставил работу в неделю, побывавшая в сезоне застревала навсегда,
//! вердикт при повторе платил дважды.
//!
//! Правила счёта (Байес, вес голоса, выходы из состояний) проверяются без базы
//! в `studio.rs`. Дублировать их здесь незачем — здесь про переходы.
//!
//! Требует `DATABASE_URL` и права CREATE DATABASE: `#[sqlx::test]` заводит на
//! каждый тест свою мигрированную базу и убирает её за собой.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::models::SaveStudioFrameRequest;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-studio-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-studio-logs-{}.sqlite", Uuid::new_v4()),
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
    .bind(format!("studio-{}@example.test", Uuid::new_v4()))
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("завести человека")
}

/// Ворота настежь: тесты про путь работы, а не про то, кого пускают.
async fn open_the_gate(svc: &AppService) {
    let mut settings = svc.get_studio_settings().await.expect("настройки");
    settings.gate = "all".into();
    svc.save_studio_settings(settings).await.expect("сохранить");
}

/// Работа, доведённая до «отдана хозяину». Тело — голая рама первого чина:
/// про её вид здесь ничего не проверяется.
async fn a_shown_work(svc: &AppService, owner: Uuid, name: &str) -> Uuid {
    let body = serde_json::json!({ "tier": 1 });
    let frame = svc
        .create_studio_frame(
            owner,
            SaveStudioFrameRequest {
                name: name.into(),
                body,
            },
        )
        .await
        .expect("завести раму");
    // Соглашение автора: без него работа на люди не выходит, и это правильно.
    svc.accept_studio_agreement(owner).await.expect("соглашение");
    svc.publish_studio_frame(owner, frame.id)
        .await
        .expect("отдать хозяину");
    frame.id
}

// ── Допуск и неделя ─────────────────────────────────────────────────────────

/// Хозяин допустил — и работа В ТУ ЖЕ МИНУТУ стоит в неделе.
///
/// Пока это были два действия, получалась дурная развилка: хозяин сделал своё,
/// а голосовать не за что, потому что автор ещё не знает, что его допустили.
#[sqlx::test]
async fn admitting_a_work_puts_it_in_the_week(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Костяная").await;

    let before = svc.studio_season_page(None, None).await.expect("сезон");
    assert!(before.entries.is_empty(), "до допуска в неделе пусто");

    svc.admit_studio_frame(frame, None).await.expect("допустить");

    let after = svc.studio_season_page(None, None).await.expect("сезон");
    assert_eq!(after.entries.len(), 1, "после допуска работа в неделе");
    assert_eq!(after.entries[0].name, "Костяная");
}

/// Тупик, который стоил недели.
///
/// Работа, побывавшая в сезоне, не выставлялась больше НИКОГДА: слово `entered`
/// не снимал никто, а выставлять позволено было только другому слову. Здесь это
/// проверяется на живой цепочке, а не на строке из `matches!`.
#[sqlx::test]
async fn a_work_that_ran_in_a_season_runs_again_next_week(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Восковая").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");

    // Неделя кончилась и заявки сняты — как это будет в понедельник.
    sqlx::query("DELETE FROM studio_entries")
        .execute(&pool)
        .await
        .expect("новая неделя");

    svc.enter_studio_season(author, frame)
        .await
        .expect("вторая неделя — работа идёт снова");

    // Третьей попытки нет: `ENTRIES_PER_FRAME` не обещание, а число.
    sqlx::query("DELETE FROM studio_entries")
        .execute(&pool)
        .await
        .expect("третья неделя");
    let refused = svc.enter_studio_season(author, frame).await;
    assert!(refused.is_err(), "третья попытка должна быть отказана");
}

/// Одна заявка от человека в неделю — правило схемы, и служба обязана
/// отвечать словом, а не падать чужой ошибкой базы.
#[sqlx::test]
async fn one_work_from_a_person_in_a_week(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let first = a_shown_work(&svc, author, "Первая").await;
    let second = a_shown_work(&svc, author, "Вторая").await;

    svc.admit_studio_frame(first, None).await.expect("допустить");
    // Вторую тоже допускают: она выходит на люди, но в неделю не встаёт.
    svc.admit_studio_frame(second, None).await.expect("допустить");

    let page = svc.studio_season_page(None, None).await.expect("сезон");
    assert_eq!(page.entries.len(), 1, "в неделе одна работа от человека");

    let refused = svc.enter_studio_season(author, second).await;
    assert!(refused.is_err(), "вторая заявка отказана словом, а не паникой");
}

// ── Вердикт ─────────────────────────────────────────────────────────────────

/// Вердикт, посчитанный дважды, не платит дважды и не меняет мест.
///
/// Задача крутится каждые четверть часа, сервер перезапускают, хозяин может
/// нажать «подвести» руками. Всё это — одна и та же неделя.
#[sqlx::test]
async fn the_verdict_counts_once_however_many_times_it_runs(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Единственная").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");

    let season = svc.studio_season_now().await.expect("сезон");
    let first = svc.judge_studio_season(season.id).await.expect("вердикт");
    let again = svc.judge_studio_season(season.id).await.expect("вердикт");
    assert_eq!(again, 0, "второй проход не подводит ничего");
    let _ = first;

    let paid: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM battle_wallet_entries WHERE user_id = $1 AND reason LIKE 'studio%'",
    )
    .bind(author)
    .fetch_one(&pool)
    .await
    .expect("кошелёк");
    assert_eq!(paid, 1, "за одну неделю платят один раз");
}

/// Порог доверия: работу, которую почти никто не смотрел, к хозяину не несут.
///
/// Иначе тройку займут те, кого не видели: у работы с одной пятёркой средний
/// балл выше, чем у работы с пятьюдесятью по 4.8.
#[sqlx::test]
async fn a_work_nobody_looked_at_does_not_reach_the_keeper(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Незамеченная").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");

    // Одна оценка при домашнем пороге в десять.
    let voter = seed_user(&pool, "Зритель").await;
    let page = svc.studio_season_page(None, None).await.expect("сезон");
    svc.rate_studio_entry(voter, page.entries[0].id, 5)
        .await
        .expect("оценить");

    let season = svc.studio_season_now().await.expect("сезон");
    let chosen = svc.judge_studio_season(season.id).await.expect("вердикт");
    assert_eq!(chosen, 0, "с одной оценкой к хозяину не идут");

    let queue = svc.studio_keeper_queue().await.expect("очередь");
    assert!(queue.is_empty(), "очередь хозяина пуста");
}

/// Автор своей работы не оценивает, и это отказ службы, а не спрятанная кнопка.
#[sqlx::test]
async fn an_author_does_not_mark_their_own_work(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Своя").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");

    let page = svc.studio_season_page(None, None).await.expect("сезон");
    let refused = svc.rate_studio_entry(author, page.entries[0].id, 5).await;
    assert!(refused.is_err(), "свою работу не оценивают");
}

/// Переоценка меняет мнение, а не добавляет голос.
#[sqlx::test]
async fn changing_your_mind_is_not_a_second_vote(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Спорная").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");
    let voter = seed_user(&pool, "Зритель").await;

    let page = svc.studio_season_page(None, None).await.expect("сезон");
    let entry = page.entries[0].id;
    svc.rate_studio_entry(voter, entry, 5).await.expect("оценка");
    svc.rate_studio_entry(voter, entry, 2).await.expect("передумал");

    let votes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM studio_ratings WHERE entry_id = $1")
        .bind(entry)
        .fetch_one(&pool)
        .await
        .expect("оценки");
    assert_eq!(votes, 1, "один человек — один голос");

    let value: i16 = sqlx::query_scalar("SELECT value FROM studio_ratings WHERE entry_id = $1")
        .bind(entry)
        .fetch_one(&pool)
        .await
        .expect("оценка");
    assert_eq!(value, 2, "осталось последнее мнение");
}

// ── Отказ и снятие ──────────────────────────────────────────────────────────

/// Возвращённая работа снова правится: это и есть способ её починить.
///
/// Пока состояния были свалены в одно слово, возврат оставлял работу
/// неправимой, и человеку оставалось завести её заново с нуля.
#[sqlx::test]
async fn a_work_sent_back_can_be_mended(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Возвращённая").await;

    svc.deny_studio_frame(frame, "уголок срисован")
        .await
        .expect("вернуть");

    svc.save_studio_frame(
        author,
        frame,
        SaveStudioFrameRequest {
            name: "Возвращённая, поправленная".into(),
            body: serde_json::json!({ "tier": 2 }),
        },
    )
    .await
    .expect("править возвращённую");

    svc.publish_studio_frame(author, frame)
        .await
        .expect("показать снова");
}

/// Отказ без слова не принимается: отказ, которого нельзя исправить, вернётся
/// той же работой на следующей неделе.
#[sqlx::test]
async fn a_refusal_without_a_word_is_not_a_refusal(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Без слова").await;

    assert!(svc.deny_studio_frame(frame, "   ").await.is_err());
}

// ── Ящик ────────────────────────────────────────────────────────────────────

/// Ящик считается ПО СКЛАДУ, а не хранимым числом: удалённая деталь
/// освобождает место в тот же миг.
#[sqlx::test]
async fn the_box_counts_what_is_actually_in_it(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let owner = seed_user(&pool, "Автор").await;

    let asset = svc
        .add_studio_asset(owner, "уголок", "corner", "/static/studio/x.webp", 10, 10, 4096)
        .await
        .expect("положить деталь");

    let settings = svc.get_studio_settings().await.expect("настройки");
    let full = svc.studio_box(owner, &settings).await.expect("ящик");
    assert_eq!(full.used, 4096);

    svc.remove_studio_asset(owner, asset.id)
        .await
        .expect("убрать деталь");
    let empty = svc.studio_box(owner, &settings).await.expect("ящик");
    assert_eq!(empty.used, 0, "убранное не занимает места");
}

/// Чужая деталь для человека не существует — не «запрещено», а «нет такой».
#[sqlx::test]
async fn another_persons_piece_simply_does_not_exist(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let mine = seed_user(&pool, "Первый").await;
    let theirs = seed_user(&pool, "Второй").await;

    let asset = svc
        .add_studio_asset(mine, "уголок", "corner", "/static/studio/y.webp", 10, 10, 100)
        .await
        .expect("положить");

    assert!(svc.remove_studio_asset(theirs, asset.id).await.is_err());
}

// ── Уборка ──────────────────────────────────────────────────────────────────

/// Уборка выносит брошенное и НЕ ТРОГАЕТ то, что стоит в раме.
///
/// Это главное её свойство и единственное, что стоит проверять: уборщик,
/// который иногда убирает лишнее, хуже, чем уборщик, которого нет.
#[sqlx::test]
async fn the_sweep_takes_the_forgotten_and_leaves_what_is_in_use(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let owner = seed_user(&pool, "Автор").await;

    let used = svc
        .add_studio_asset(owner, "в раме", "corner", "/static/studio/used.webp", 8, 8, 100)
        .await
        .expect("деталь");
    let orphan = svc
        .add_studio_asset(owner, "сирота", "corner", "/static/studio/lost.webp", 8, 8, 100)
        .await
        .expect("деталь");

    // Одна деталь стоит в раме — адресом, как оно и бывает.
    svc.create_studio_frame(
        owner,
        SaveStudioFrameRequest {
            name: "Рама".into(),
            body: serde_json::json!({ "tier": 1, "cornerImage": "/static/studio/used.webp" }),
        },
    )
    .await
    .expect("рама");

    // Обе постарели на два месяца.
    sqlx::query("UPDATE studio_assets SET created_at = NOW() - INTERVAL '60 days'")
        .execute(&pool)
        .await
        .expect("состарить");

    let (_, swept) = svc.sweep_studio_store().await.expect("уборка");
    assert_eq!(swept, 1, "убрана ровно одна деталь");

    let left: Vec<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM studio_assets")
        .fetch_all(&pool)
        .await
        .expect("склад");
    assert_eq!(left, vec![used.id], "осталась та, что стоит в раме");
    assert!(!left.contains(&orphan.id));
}

/// Лист, из которого что-то вырезали, уборке не подлежит: он нужен, чтобы
/// повторить разрез другими настройками, не спрашивая файл заново.
#[sqlx::test]
async fn a_harvested_sheet_is_kept(pool: PgPool) {
    let svc = service(&pool).await;
    let owner = seed_user(&pool, "Автор").await;

    sqlx::query(
        "INSERT INTO studio_sheets (owner_id, name, url, width, height, bytes, harvested_at,
                                    created_at)
         VALUES ($1, 'с урожаем', '/static/studio/a.png', 10, 10, 100, NOW(),
                 NOW() - INTERVAL '90 days'),
                ($1, 'брошенный', '/static/studio/b.png', 10, 10, 100, NULL,
                 NOW() - INTERVAL '90 days')",
    )
    .bind(owner)
    .execute(&pool)
    .await
    .expect("листы");

    let (sheets, _) = svc.sweep_studio_store().await.expect("уборка");
    assert_eq!(sheets, 1, "убран только брошенный");

    let left: String = sqlx::query_scalar("SELECT name FROM studio_sheets")
        .fetch_one(&pool)
        .await
        .expect("лист");
    assert_eq!(left, "с урожаем");
}

// ── Лицензии ────────────────────────────────────────────────────────────────

/// Утверждение печатает тираж АВТОРУ — и печатает его один раз.
///
/// «Утвердить» можно нажать дважды на медленной сети. Тираж — число, а не
/// действие: второе нажатие не должно удвоить его.
#[sqlx::test]
async fn approving_prints_the_run_once(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Утверждаемая").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");

    svc.approve_studio_frame(frame, Some(200))
        .await
        .expect("утвердить");
    // Второе нажатие: рама уже утверждена — отказ, и это правильно.
    let _ = svc.approve_studio_frame(frame, Some(200)).await;

    let made: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM frame_licenses WHERE frame_id = $1")
        .bind(frame)
        .fetch_one(&pool)
        .await
        .expect("лицензии");
    assert_eq!(made, 200, "тираж отпечатан один раз");

    let mine = svc.my_licences(author).await.expect("свои лицензии");
    assert_eq!(mine.len(), 200, "весь тираж у автора");
    assert_eq!(mine[0].author, "Автор");
    assert!(mine.iter().all(|l| !l.locked), "ничего не заперто");

    // Номера идут подряд и не повторяются: «№7 из 200» должно что-то значить.
    let mut serials: Vec<i32> = mine.iter().map(|l| l.serial).collect();
    serials.sort_unstable();
    serials.dedup();
    assert_eq!(serials.len(), 200, "номера не повторяются");
    assert_eq!(*serials.first().unwrap(), 1);
    assert_eq!(*serials.last().unwrap(), 200);
}

/// Утверждение без тиража ничего не печатает: рама взята в игру, но лицензий
/// у неё нет — и это законное состояние, а не половина работы.
#[sqlx::test]
async fn approving_without_a_run_prints_nothing(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Без тиража").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");
    svc.approve_studio_frame(frame, None).await.expect("утвердить");

    assert!(svc.my_licences(author).await.expect("лицензии").is_empty());
}

// ── Лавка ───────────────────────────────────────────────────────────────────

/// Даёт человеку денег на покупку. Прямо в книгу — как их даёт хозяин.
async fn give_dust(pool: &PgPool, who: Uuid, amount: i32) {
    sqlx::query(
        "INSERT INTO battle_wallet_entries (user_id, currency, amount, reason, idem_key)
         VALUES ($1, 'dust', $2, 'test', $3)",
    )
    .bind(who)
    .bind(amount)
    .bind(Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .expect("дать пыли");
}

/// Сделка целиком: деньги ушли, вещь пришла, доля дома СГОРЕЛА.
///
/// Сгоревшая доля — единственный сток валюты. Проверяется не «комиссия
/// посчитана», а то, ради чего она есть: денег в доме стало МЕНЬШЕ.
#[sqlx::test]
async fn a_sale_moves_the_licence_and_burns_the_house_share(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let buyer = seed_user(&pool, "Покупатель").await;
    let frame = a_shown_work(&svc, author, "Продаваемая").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");
    svc.approve_studio_frame(frame, Some(3)).await.expect("утвердить");
    give_dust(&pool, buyer, 1000).await;

    let licence = svc.my_licences(author).await.expect("лицензии")[0].id;
    let listing = svc
        .list_thing(author, "license", licence, 100, "dust")
        .await
        .expect("выставить");

    let in_house_before: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)::bigint FROM battle_wallet_entries WHERE currency = 'dust'",
    )
    .fetch_one(&pool)
    .await
    .expect("в доме");

    svc.buy_listing(buyer, listing).await.expect("купить");

    // Вещь у покупателя, и она отперта.
    let his = svc.my_licences(buyer).await.expect("лицензии");
    assert_eq!(his.len(), 1, "лицензия у покупателя");
    assert!(!his[0].locked, "купленная не заперта");
    assert_eq!(svc.my_licences(author).await.expect("лицензии").len(), 2);

    // Автору пришло за вычетом доли, и доля именно СГОРЕЛА.
    let in_house_after: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)::bigint FROM battle_wallet_entries WHERE currency = 'dust'",
    )
    .fetch_one(&pool)
    .await
    .expect("в доме");
    assert_eq!(
        in_house_before - in_house_after,
        10,
        "десятая часть сотни сгорела, а не осела у кого-то"
    );
}

/// Прилавок называет ЛИЦЕНЗИЮ, а не имя продавца.
///
/// Иначе «моё ли это» приходится решать по имени, а двух одинаковых имён дом
/// не запрещает: тёзка увидел бы чужую запись своей и снял бы её с прилавка.
#[sqlx::test]
async fn the_counter_names_the_licence_not_the_seller(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let one = seed_user(&pool, "Тёзка").await;
    let two = seed_user(&pool, "Тёзка").await;

    for (who, name) in [(one, "Перваяя"), (two, "Втораяя")] {
        let frame = a_shown_work(&svc, who, name).await;
        svc.admit_studio_frame(frame, None).await.expect("допустить");
        svc.approve_studio_frame(frame, Some(3)).await.expect("утвердить");
        let licence = svc.my_licences(who).await.expect("лицензии")[0].id;
        svc.list_thing(who, "license", licence, 100, "dust").await.expect("выставить");
    }

    let counter = svc.market().await.expect("прилавок");
    assert_eq!(counter.len(), 2);
    assert_eq!(counter[0].seller, counter[1].seller, "имена и правда совпали");

    // Каждый узнаёт СВОЮ и только свою.
    let his: Vec<_> = svc.my_licences(one).await.expect("лицензии").iter().map(|l| l.id).collect();
    let mine: Vec<_> = counter.iter().filter(|l| his.contains(&l.licence_id)).collect();
    assert_eq!(mine.len(), 1, "своя одна, а не обе");
    assert_eq!(mine[0].name, "Перваяя");
}

/// Выставленная лицензия ЗАПЕРТА: второй раз её не выставить.
#[sqlx::test]
async fn a_listed_licence_cannot_be_listed_twice(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Одна").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");
    svc.approve_studio_frame(frame, Some(1)).await.expect("утвердить");

    let licence = svc.my_licences(author).await.expect("лицензии")[0].id;
    let listing = svc.list_thing(author, "license", licence, 100, "dust").await.expect("раз");
    assert!(svc.list_thing(author, "license", licence, 200, "dust").await.is_err());

    // Снял — снова можно.
    svc.withdraw_listing(author, listing).await.expect("снять");
    svc.list_thing(author, "license", licence, 200, "dust")
        .await
        .expect("после снятия можно снова");
}

/// Не хватает денег — сделки нет ЦЕЛИКОМ: ни списания, ни смены владельца.
#[sqlx::test]
async fn a_sale_that_cannot_be_paid_leaves_nothing_behind(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let poor = seed_user(&pool, "Без денег").await;
    let frame = a_shown_work(&svc, author, "Дорогая").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");
    svc.approve_studio_frame(frame, Some(1)).await.expect("утвердить");
    let licence = svc.my_licences(author).await.expect("лицензии")[0].id;
    let listing = svc.list_thing(author, "license", licence, 500, "dust").await.expect("выставить");

    assert!(svc.buy_listing(poor, listing).await.is_err());

    assert!(svc.my_licences(poor).await.expect("лицензии").is_empty());
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM battle_wallet_entries")
        .fetch_one(&pool)
        .await
        .expect("книга");
    assert_eq!(rows, 0, "в книге не осталось следа от несостоявшейся сделки");
    let state: String = sqlx::query_scalar("SELECT state FROM market_listings WHERE id = $1")
        .bind(listing)
        .fetch_one(&pool)
        .await
        .expect("объявление");
    assert_eq!(state, "open", "объявление осталось открытым");
}

/// Цена — в коридоре дома. Без потолка первый же автор поставит миллион.
#[sqlx::test]
async fn a_price_outside_the_house_corridor_is_refused(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Ценная").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");
    svc.approve_studio_frame(frame, Some(1)).await.expect("утвердить");
    let licence = svc.my_licences(author).await.expect("лицензии")[0].id;

    assert!(svc.list_thing(author, "license", licence, 1, "dust").await.is_err(), "ниже пола");
    assert!(
        svc.list_thing(author, "license", licence, 999_999, "dust").await.is_err(),
        "выше потолка"
    );
}

/// Свою же лицензию у себя не покупают.
#[sqlx::test]
async fn nobody_buys_from_themselves(pool: PgPool) {
    let svc = service(&pool).await;
    open_the_gate(&svc).await;
    let author = seed_user(&pool, "Автор").await;
    let frame = a_shown_work(&svc, author, "Своя же").await;
    svc.admit_studio_frame(frame, None).await.expect("допустить");
    svc.approve_studio_frame(frame, Some(1)).await.expect("утвердить");
    let licence = svc.my_licences(author).await.expect("лицензии")[0].id;
    let listing = svc.list_thing(author, "license", licence, 100, "dust").await.expect("выставить");
    give_dust(&pool, author, 1000).await;

    assert!(svc.buy_listing(author, listing).await.is_err());
}
