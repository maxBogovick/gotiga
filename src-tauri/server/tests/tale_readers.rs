//! Читатель небылиц после последней строки: «хочу продолжение», голос «о ком
//! записать следующую» и письмо о вышедшей байке.
//!
//! Стенд локальный (`PUBLIC_URL` на 127.0.0.1), поэтому письмо не уходит, а
//! печатается в журнал и отмечается ушедшим — ровно тот путь, которым письма
//! ходят на рабочем сервере, кроме самого SMTP. Проверяется очередь: кому,
//! по какому поводу и сколько раз.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::error::AppError;
use gotiga_server::models::{TalePollVoteRequest, TaleSequelWishRequest};
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-tales-uploads-{}", Uuid::new_v4()),
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
        telegram_chat_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-tales-logs-{}.sqlite", Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".into(),
        auth_pepper: "pepper-for-tests-0123456789".into(),
        auth_pepper_old: None,
    }
}

fn service(pool: &PgPool) -> AppService {
    AppService::new(Repository::new(pool.clone()), config())
}

async fn seed_work(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO figurines (name, short_text, is_visible, status)
         VALUES ($1, 'Воск и глина', TRUE, 'available') RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("завести работу")
}

/// Байка на людях. `minutes_ago` — сколько она уже стоит: выдержка писем
/// десять минут, и свежая байка ещё ждёт.
async fn seed_tale(
    pool: &PgPool,
    title: &str,
    work: Option<Uuid>,
    sequel_of: Option<Uuid>,
    minutes_ago: i32,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO gazette_leaves
             (slug, kind, status, title_en, title_ru, dek_ru, figurine_id, sequel_of, published_at)
         VALUES ($1, 'tale', 'published', $2, $2, 'Эпиграф', $3, $4,
                 NOW() - make_interval(mins => $5))
         RETURNING id",
    )
    .bind(format!("tale-{}", Uuid::new_v4().simple()))
    .bind(title)
    .bind(work)
    .bind(sequel_of)
    .bind(minutes_ago)
    .fetch_one(pool)
    .await
    .expect("завести байку")
}

async fn sign_the_book(pool: &PgPool, email: &str) {
    sqlx::query(
        "INSERT INTO newsletter_subscribers (email, source, lang, unsubscribe_token)
         VALUES ($1, 'home', 'ru', $2)",
    )
    .bind(email)
    .bind(format!("door-{}", Uuid::new_v4().simple()))
    .execute(pool)
    .await
    .expect("вписать в книгу");
}

fn wish(token: &str, want: bool, email: Option<&str>, age: bool) -> TaleSequelWishRequest {
    TaleSequelWishRequest {
        visitor_token: token.into(),
        want,
        email: email.map(str::to_string),
        lang: Some("ru".into()),
        age_confirmed: age,
        telegram: None,
    }
}

fn vote(token: &str, work: Uuid, email: Option<&str>) -> TalePollVoteRequest {
    TalePollVoteRequest {
        visitor_token: token.into(),
        figurine_id: work,
        email: email.map(str::to_string),
        lang: Some("en".into()),
        age_confirmed: email.is_some(),
    }
}

/// Очередь писем по байке: адрес и повод, по алфавиту.
async fn letters(pool: &PgPool, tale: Uuid) -> Vec<(String, String, bool)> {
    sqlx::query_as(
        "SELECT email, reason, sent_at IS NOT NULL FROM tale_letters
          WHERE tale_id = $1 ORDER BY lower(email)",
    )
    .bind(tale)
    .fetch_all(pool)
    .await
    .expect("очередь писем")
}

async fn open_poll_id(svc: &AppService) -> Uuid {
    let poll = svc
        .current_tale_poll(None)
        .await
        .expect("голосование")
        .expect("голосование стоит");
    Uuid::parse_str(&poll.id).expect("id")
}

/// Один голос на читателя, и он меняется; за невыставленную работу голоса
/// нет; открытое голосование одно.
#[sqlx::test]
async fn the_poll_takes_one_voice_per_reader(pool: PgPool) {
    let svc = service(&pool);
    let witch = seed_work(&pool, "Witch").await;
    let wizard = seed_work(&pool, "Wizard").await;
    let granny = seed_work(&pool, "Granny").await;

    assert!(matches!(
        svc.admin_open_tale_poll(vec![witch]).await,
        Err(AppError::BadRequest(_))
    ));
    svc.admin_open_tale_poll(vec![witch, wizard, witch])
        .await
        .expect("открыть: повтор кандидата сворачивается");
    assert!(matches!(
        svc.admin_open_tale_poll(vec![witch, granny]).await,
        Err(AppError::BadRequest(_))
    ));
    let poll = open_poll_id(&svc).await;

    svc.vote_tale_poll(poll, &vote("reader-a", witch, None), None)
        .await
        .expect("голос");
    let changed = svc
        .vote_tale_poll(poll, &vote("reader-a", wizard, None), None)
        .await
        .expect("передумал");
    assert_eq!(changed.my_choice, wizard.to_string());
    assert!(!changed.my_letter);
    assert!(matches!(
        svc.vote_tale_poll(poll, &vote("reader-b", granny, None), None).await,
        Err(AppError::NotFound(_))
    ));

    let seen = svc
        .current_tale_poll(Some("reader-a"))
        .await
        .expect("голосование")
        .expect("стоит");
    assert_eq!(seen.state, "open");
    assert_eq!(seen.candidates.len(), 2);
    assert_eq!(seen.my_choice.as_deref(), Some(wizard.to_string().as_str()));

    let desk = svc.admin_tale_polls().await.expect("стол");
    let counts: Vec<i64> = desk[0].candidates.iter().map(|c| c.votes).collect();
    assert_eq!(counts, vec![0, 1], "голос один и лежит у того, кого выбрали последним");
}

/// Закрытое голосование новых голосов не берёт, но проголосовавший ещё может
/// оставить адрес — и его выбор при этом не меняется.
#[sqlx::test]
async fn a_closed_poll_keeps_its_choices_and_still_takes_an_address(pool: PgPool) {
    let svc = service(&pool);
    let witch = seed_work(&pool, "Witch").await;
    let wizard = seed_work(&pool, "Wizard").await;
    svc.admin_open_tale_poll(vec![witch, wizard]).await.expect("открыть");
    let poll = open_poll_id(&svc).await;
    svc.vote_tale_poll(poll, &vote("reader-a", witch, None), None)
        .await
        .expect("голос");

    svc.admin_close_tale_poll(poll, wizard).await.expect("закрыть");
    assert!(matches!(
        svc.vote_tale_poll(poll, &vote("reader-b", wizard, None), None).await,
        Err(AppError::BadRequest(_))
    ));
    let kept = svc
        .vote_tale_poll(poll, &vote("reader-a", wizard, Some("a@x.io")), None)
        .await
        .expect("адрес к прежнему голосу");
    assert_eq!(kept.my_choice, witch.to_string(), "выбор после закрытия не меняется");
    assert!(kept.my_letter);

    let shown = svc
        .current_tale_poll(Some("reader-a"))
        .await
        .expect("голосование")
        .expect("пока байки нет, полка говорит «пишется»");
    assert_eq!(shown.state, "closed");
    assert_eq!(shown.winner.map(|w| w.figurine_id), Some(wizard.to_string()));
}

/// «Хочу продолжение»: одна просьба на читателя; адрес — только с
/// подтверждённым возрастом; стол видит, сколько ждут.
#[sqlx::test]
async fn a_sequel_wish_is_one_per_reader(pool: PgPool) {
    let svc = service(&pool);
    let tale = seed_tale(&pool, "Лесник", None, None, 60).await;

    let first = svc
        .set_tale_sequel_wish(tale, &wish("reader-a", true, None, false), None)
        .await
        .expect("просьба без адреса");
    assert!(first.wants && !first.letter);
    assert!(matches!(
        svc.set_tale_sequel_wish(tale, &wish("reader-a", true, Some("a@x.io"), false), None)
            .await,
        Err(AppError::BadRequest(_))
    ));
    let with_letter = svc
        .set_tale_sequel_wish(tale, &wish("reader-a", true, Some("a@x.io"), true), None)
        .await
        .expect("просьба с адресом");
    assert!(with_letter.wants && with_letter.letter);
    // Повторное нажатие без адреса адрес не стирает.
    let again = svc
        .set_tale_sequel_wish(tale, &wish("reader-a", true, None, false), None)
        .await
        .expect("ещё раз");
    assert!(again.letter);
    svc.set_tale_sequel_wish(tale, &wish("reader-b", true, None, false), None)
        .await
        .expect("второй читатель");

    let stats = svc.get_tale_stats(tale, Some("reader-a"), false).await.expect("числа");
    assert_eq!(stats.wants_sequel, Some(true));
    // Вернувшийся читатель узнаёт и то, что письмо уже заказано.
    assert_eq!(stats.sequel_letter, Some(true));
    let other = svc.get_tale_stats(tale, Some("reader-b"), false).await.expect("числа");
    assert_eq!((other.wants_sequel, other.sequel_letter), (Some(true), Some(false)));
    let stranger = svc.get_tale_stats(tale, Some("reader-c"), false).await.expect("числа");
    assert_eq!((stranger.wants_sequel, stranger.sequel_letter), (Some(false), Some(false)));
    let desk = svc.admin_tale_stats().await.expect("стол");
    let row = desk.iter().find(|r| r.tale_id == tale.to_string()).expect("строка");
    assert_eq!((row.sequel_wishes, row.sequel_letters), (2, 1));

    let gone = svc
        .set_tale_sequel_wish(tale, &wish("reader-a", false, None, false), None)
        .await
        .expect("передумал");
    assert!(!gone.wants);
}

/// Вышла байка — письма раскладываются один раз: просившим продолжение,
/// голосовавшим за её работу и книге дома; совпавший адрес получает одно
/// письмо, по самому личному поводу. Старые и свежие байки не пишутся.
#[sqlx::test]
async fn a_new_tale_writes_once_to_whoever_asked(pool: PgPool) {
    let svc = service(&pool);
    let witch = seed_work(&pool, "Witch").await;
    let wizard = seed_work(&pool, "Wizard").await;

    // Начало, которое кто-то просил продолжить.
    let beginning = seed_tale(&pool, "Начало", None, None, 600).await;
    // Первый проход отмечает начало разложенным — оно вышло до всех просьб.
    svc.send_tale_letters().await.expect("первый проход");
    sign_the_book(&pool, "book@x.io").await;
    sign_the_book(&pool, "both@x.io").await;
    svc.set_tale_sequel_wish(beginning, &wish("w1", true, Some("wish@x.io"), true), None)
        .await
        .expect("просьба");
    svc.set_tale_sequel_wish(beginning, &wish("w2", true, Some("BOTH@x.io"), true), None)
        .await
        .expect("просьба из книги");

    // Голосование закрыто на Ведьме.
    svc.admin_open_tale_poll(vec![witch, wizard]).await.expect("открыть");
    let poll = open_poll_id(&svc).await;
    svc.vote_tale_poll(poll, &vote("v1", wizard, Some("voter@x.io")), None)
        .await
        .expect("голос");
    svc.admin_close_tale_poll(poll, witch).await.expect("закрыть");

    // Продолжение о Ведьме, вышедшее час назад, и совсем свежая байка.
    let sequel = seed_tale(&pool, "Продолжение", Some(witch), Some(beginning), 60).await;
    let fresh = seed_tale(&pool, "Свежая", None, None, 1).await;

    let sent = svc.send_tale_letters().await.expect("проход");
    assert_eq!(sent, 4);
    assert_eq!(
        letters(&pool, sequel).await,
        vec![
            ("book@x.io".into(), "new".into(), true),
            ("BOTH@x.io".into(), "sequel".into(), true),
            ("voter@x.io".into(), "chosen".into(), true),
            ("wish@x.io".into(), "sequel".into(), true),
        ],
    );
    assert!(letters(&pool, beginning).await.is_empty(), "старое не пишется");
    assert!(letters(&pool, fresh).await.is_empty(), "свежее ждёт выдержку");

    // Исполненное голосование с полки уходит.
    assert!(svc.current_tale_poll(None).await.expect("голосование").is_none());
    let desk = svc.admin_tale_polls().await.expect("стол");
    assert_eq!(desk[0].fulfilled.as_ref().map(|f| f.title_ru.as_str()), Some("Продолжение"));

    // Второй проход ничего не добавляет.
    assert_eq!(svc.send_tale_letters().await.expect("повтор"), 0);
    assert_eq!(letters(&pool, sequel).await.len(), 4);

    // Начало и продолжение видят друг друга.
    let stats = svc.get_tale_stats(beginning, None, false).await.expect("числа");
    assert_eq!(stats.sequel.map(|s| s.title_ru), Some("Продолжение".into()));
    let stats = svc.get_tale_stats(sequel, None, false).await.expect("числа");
    assert_eq!(stats.sequel_of.map(|s| s.title_ru), Some("Начало".into()));
}

/// Порядок, в котором хозяин выложил байку и назвал её продолжением или
/// закрыл голосование, не решает, придёт ли письмо.
#[sqlx::test]
async fn the_order_of_the_keepers_hands_does_not_lose_a_letter(pool: PgPool) {
    let svc = service(&pool);
    let witch = seed_work(&pool, "Witch").await;
    let wizard = seed_work(&pool, "Wizard").await;
    let beginning = seed_tale(&pool, "Начало", None, None, 600).await;
    svc.send_tale_letters().await.expect("первый проход");
    svc.set_tale_sequel_wish(beginning, &wish("w1", true, Some("wish@x.io"), true), None)
        .await
        .expect("просьба");
    svc.admin_open_tale_poll(vec![witch, wizard]).await.expect("открыть");
    let poll = open_poll_id(&svc).await;
    // Голосование открыли два часа назад — до того, как вышла байка.
    sqlx::query("UPDATE tale_polls SET opened_at = NOW() - INTERVAL '2 hours'")
        .execute(&pool)
        .await
        .expect("сдвинуть время");
    svc.vote_tale_poll(poll, &vote("v1", witch, Some("voter@x.io")), None)
        .await
        .expect("голос");

    // Байка о Ведьме вышла и разослана, пока голосование ещё открыто и пока
    // она ещё ничьё не продолжение.
    let tale = seed_tale(&pool, "О Ведьме", Some(witch), None, 60).await;
    svc.send_tale_letters().await.expect("проход");
    assert!(letters(&pool, tale).await.is_empty());

    svc.admin_set_tale_sequel(tale, Some(beginning)).await.expect("назвать продолжением");
    svc.admin_close_tale_poll(poll, witch).await.expect("закрыть");
    let queued: Vec<(String, String)> = letters(&pool, tale)
        .await
        .into_iter()
        .map(|(email, reason, _)| (email, reason))
        .collect();
    assert_eq!(
        queued,
        vec![
            ("voter@x.io".into(), "chosen".into()),
            ("wish@x.io".into(), "sequel".into()),
        ],
    );

    // Байка не продолжает саму себя, и лист вестника ничьё не начало.
    assert!(matches!(
        svc.admin_set_tale_sequel(tale, Some(tale)).await,
        Err(AppError::BadRequest(_))
    ));
}

/// Двери: канал называется ссылкой только по имени; письмо на стенде есть
/// всегда — оно печатается в журнал.
#[sqlx::test]
async fn the_doors_name_the_channel_by_its_name(pool: PgPool) {
    let doors = service(&pool).tale_doors().await;
    assert!(doors.letters);
    assert_eq!(doors.telegram.as_deref(), Some("https://t.me/ritunia_tales"));
}

async fn seed_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (display_name) VALUES ('Читатель') RETURNING id")
        .fetch_one(pool)
        .await
        .expect("завести имя")
}

/// Голос вошедшего с другого устройства в уже закрытое голосование не
/// трогает ничего: прежний голос и его адрес на месте, итог не меняется.
#[sqlx::test]
async fn a_closed_poll_never_loses_a_signed_in_readers_vote(pool: PgPool) {
    let svc = service(&pool);
    let reader = seed_user(&pool).await;
    let witch = seed_work(&pool, "Witch").await;
    let wizard = seed_work(&pool, "Wizard").await;
    svc.admin_open_tale_poll(vec![witch, wizard]).await.expect("открыть");
    let poll = open_poll_id(&svc).await;
    svc.vote_tale_poll(poll, &vote("device-a", witch, Some("a@x.io")), Some(reader))
        .await
        .expect("голос с первого устройства");
    svc.admin_close_tale_poll(poll, witch).await.expect("закрыть");

    // Устаревшая вкладка на втором устройстве.
    assert!(matches!(
        svc.vote_tale_poll(poll, &vote("device-b", wizard, None), Some(reader)).await,
        Err(AppError::BadRequest(_))
    ));
    let kept: Vec<(String, Uuid, Option<String>)> = sqlx::query_as(
        "SELECT visitor_token, figurine_id, email FROM tale_poll_votes WHERE poll_id = $1",
    )
    .bind(poll)
    .fetch_all(&pool)
    .await
    .expect("голоса");
    assert_eq!(kept, vec![("device-a".to_string(), witch, Some("a@x.io".to_string()))]);

    // Пока голосование открыто, голос вошедшего переезжает на новое
    // устройство вместе с адресом — это правило осталось прежним.
    svc.admin_open_tale_poll(vec![witch, wizard]).await.expect("открыть второе");
    let second = open_poll_id(&svc).await;
    svc.vote_tale_poll(second, &vote("device-a", witch, Some("a@x.io")), Some(reader))
        .await
        .expect("голос");
    let moved = svc
        .vote_tale_poll(second, &vote("device-b", wizard, None), Some(reader))
        .await
        .expect("голос со второго устройства");
    assert!(moved.my_letter, "адрес переехал вместе с голосом");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tale_poll_votes WHERE poll_id = $1")
        .bind(second)
        .fetch_one(&pool)
        .await
        .expect("число голосов");
    assert_eq!(count, 1);

    // Стол видит оба голосования, и у каждого свои кандидаты и свой счёт.
    let desk = svc.admin_tale_polls().await.expect("стол");
    assert_eq!(desk.len(), 2);
    for p in &desk {
        assert_eq!(p.candidates.len(), 2);
        assert_eq!(p.candidates.iter().map(|c| c.votes).sum::<i64>(), 1);
    }
}

/// Письмо книги дома, дождавшееся в очереди отписки адресата, не уходит.
/// Просившему продолжение письмо уходит и после отписки — он просил сам, —
/// но без двери из книги, в которой его больше нет.
#[sqlx::test]
async fn an_unsubscribed_address_gets_no_queued_letter(pool: PgPool) {
    let svc = service(&pool);
    let repo = Repository::new(pool.clone());
    let beginning = seed_tale(&pool, "Начало", None, None, 600).await;
    svc.send_tale_letters().await.expect("первый проход");
    sign_the_book(&pool, "gone@x.io").await;
    sign_the_book(&pool, "stays@x.io").await;
    sign_the_book(&pool, "asked@x.io").await;
    svc.set_tale_sequel_wish(beginning, &wish("w1", true, Some("asked@x.io"), true), None)
        .await
        .expect("просьба");

    let tale = seed_tale(&pool, "Продолжение", None, Some(beginning), 60).await;
    assert_eq!(repo.lay_out_tale_letters(tale).await.expect("раскладка"), 3);
    sqlx::query(
        "UPDATE newsletter_subscribers SET unsubscribed_at = NOW()
          WHERE email IN ('gone@x.io', 'asked@x.io')",
    )
    .execute(&pool)
    .await
    .expect("отписаться");

    let claimed = repo.claim_tale_letters(30, 5, 600).await.expect("взять");
    let mut got: Vec<(String, bool)> = claimed
        .iter()
        .map(|l| (l.email.clone(), l.unsubscribe_token.is_some()))
        .collect();
    got.sort();
    assert_eq!(
        got,
        vec![("asked@x.io".to_string(), false), ("stays@x.io".to_string(), true)],
    );
}

/// Взятое на отправку письмо второй раз не выдаётся, пока не истёк срок
/// взятия; отказ почты возвращает его в очередь сразу.
#[sqlx::test]
async fn a_claimed_letter_is_handed_out_once(pool: PgPool) {
    let svc = service(&pool);
    let repo = Repository::new(pool.clone());
    seed_tale(&pool, "Начало", None, None, 600).await;
    svc.send_tale_letters().await.expect("первый проход");
    sign_the_book(&pool, "one@x.io").await;
    let tale = seed_tale(&pool, "Новая", None, None, 60).await;
    repo.lay_out_tale_letters(tale).await.expect("раскладка");

    let first = repo.claim_tale_letters(30, 5, 600).await.expect("первая копия");
    assert_eq!(first.len(), 1);
    let second = repo.claim_tale_letters(30, 5, 600).await.expect("вторая копия");
    assert!(second.is_empty(), "письмо уже взято первой копией");

    repo.tale_letter_failed(tale, "one@x.io", "SMTP timeout")
        .await
        .expect("отказ");
    let retry = repo.claim_tale_letters(30, 5, 600).await.expect("повтор");
    assert_eq!(retry.len(), 1, "после отказа письмо снова в очереди");
}

/// Цепочка продолжений в круг не замыкается — ни прямо, ни через третью.
#[sqlx::test]
async fn a_sequel_chain_never_closes_into_a_circle(pool: PgPool) {
    let svc = service(&pool);
    let a = seed_tale(&pool, "А", None, None, 600).await;
    let b = seed_tale(&pool, "Б", None, None, 600).await;
    let c = seed_tale(&pool, "В", None, None, 600).await;

    svc.admin_set_tale_sequel(b, Some(a)).await.expect("Б продолжает А");
    assert!(matches!(
        svc.admin_set_tale_sequel(a, Some(b)).await,
        Err(AppError::BadRequest(_))
    ));
    svc.admin_set_tale_sequel(c, Some(b)).await.expect("В продолжает Б");
    assert!(matches!(
        svc.admin_set_tale_sequel(a, Some(c)).await,
        Err(AppError::BadRequest(_))
    ));
    // Переназначить начало без круга можно.
    svc.admin_set_tale_sequel(c, Some(a)).await.expect("В продолжает А");
    svc.admin_set_tale_sequel(c, None).await.expect("снять");
}

async fn seed_telegram_user(pool: &PgPool, chat: i64) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (display_name, telegram_id, telegram_username)
         VALUES ('Читатель из Telegram', $1, 'reader') RETURNING id",
    )
    .bind(chat)
    .fetch_one(pool)
    .await
    .expect("завести имя с Telegram")
}

/// Вошедший через Telegram просит сообщить туда — и получает записку, когда
/// продолжение вышло. Гостю и имени без Telegram записка не положена. Стол
/// видит, кто ждёт и дошло ли до него известие.
#[sqlx::test]
async fn a_telegram_reader_is_told_by_a_note(pool: PgPool) {
    let svc = service(&pool);
    let repo = Repository::new(pool.clone());
    let reader = seed_telegram_user(&pool, 777).await;
    let plain = seed_user(&pool).await;
    let reader_user = repo.find_user_by_id(reader).await.expect("имя").expect("есть");
    let plain_user = repo.find_user_by_id(plain).await.expect("имя").expect("есть");

    let beginning = seed_tale(&pool, "Начало", None, None, 600).await;
    svc.send_tale_letters().await.expect("первый проход");

    let mut ask = wish("tg-device", true, None, false);
    ask.telegram = Some(true);
    assert!(matches!(
        svc.set_tale_sequel_wish(beginning, &ask, None).await,
        Err(AppError::BadRequest(_))
    ), "гостю писать в Telegram некуда");
    assert!(matches!(
        svc.set_tale_sequel_wish(beginning, &ask, Some(&plain_user)).await,
        Err(AppError::BadRequest(_))
    ), "у этого имени Telegram не привязан");
    let asked = svc
        .set_tale_sequel_wish(beginning, &ask, Some(&reader_user))
        .await
        .expect("просьба с запиской");
    assert!(asked.wants && asked.telegram && !asked.letter);
    // Повторное нажатие без выбора записку не снимает.
    let again = svc
        .set_tale_sequel_wish(beginning, &wish("tg-device", true, None, false), Some(&reader_user))
        .await
        .expect("ещё раз");
    assert!(again.telegram);
    let stats = svc.get_tale_stats(beginning, Some("tg-device"), false).await.expect("числа");
    assert_eq!(stats.sequel_telegram, Some(true));

    svc.set_tale_sequel_wish(beginning, &wish("guest", true, Some("guest@x.io"), true), None)
        .await
        .expect("просьба с письмом");

    // Стол до продолжения: двое ждут, известий ещё нет.
    let waiting = svc.admin_sequel_wishes(beginning).await.expect("стол");
    assert_eq!(waiting.len(), 2);
    assert!(waiting.iter().all(|w| w.letter == "none" && w.note == "none"));
    let tg = waiting.iter().find(|w| w.by_telegram).expect("читатель из Telegram");
    assert_eq!(tg.name.as_deref(), Some("Читатель из Telegram"));
    assert_eq!(tg.telegram_username.as_deref(), Some("reader"));

    // Продолжение вышло: письмо гостю, записка читателю из Telegram.
    let sequel = seed_tale(&pool, "Продолжение", None, Some(beginning), 60).await;
    svc.send_tale_letters().await.expect("письма");
    assert_eq!(svc.send_tale_notes().await.expect("записки"), 1);
    assert_eq!(svc.send_tale_notes().await.expect("повтор"), 0, "записка уходит один раз");
    let notes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tale_notes WHERE tale_id = $1 AND sent_at IS NOT NULL",
    )
    .bind(sequel)
    .fetch_one(&pool)
    .await
    .expect("записки");
    assert_eq!(notes, 1);

    let waiting = svc.admin_sequel_wishes(beginning).await.expect("стол");
    let tg = waiting.iter().find(|w| w.by_telegram).expect("читатель из Telegram");
    assert_eq!((tg.note.as_str(), tg.letter.as_str()), ("sent", "none"));
    let guest = waiting.iter().find(|w| w.email.is_some()).expect("гость");
    assert_eq!((guest.letter.as_str(), guest.note.as_str()), ("sent", "none"));
}

/// Отвязал Telegram, пока ждал, — записка не крутится неделю, а сразу
/// отмечается недошедшей.
#[sqlx::test]
async fn a_note_to_an_unlinked_name_gives_up_at_once(pool: PgPool) {
    let svc = service(&pool);
    let repo = Repository::new(pool.clone());
    let reader = seed_telegram_user(&pool, 778).await;
    let reader_user = repo.find_user_by_id(reader).await.expect("имя").expect("есть");
    let beginning = seed_tale(&pool, "Начало", None, None, 600).await;
    svc.send_tale_letters().await.expect("первый проход");
    let mut ask = wish("tg", true, None, false);
    ask.telegram = Some(true);
    svc.set_tale_sequel_wish(beginning, &ask, Some(&reader_user))
        .await
        .expect("просьба");
    seed_tale(&pool, "Продолжение", None, Some(beginning), 60).await;
    svc.send_tale_letters().await.expect("раскладка");
    sqlx::query("UPDATE users SET telegram_id = NULL WHERE id = $1")
        .bind(reader)
        .execute(&pool)
        .await
        .expect("отвязать");

    assert_eq!(svc.send_tale_notes().await.expect("проход"), 0);
    let waiting = svc.admin_sequel_wishes(beginning).await.expect("стол");
    assert_eq!(waiting[0].note, "failed");
    assert_eq!(svc.send_tale_notes().await.expect("повтор"), 0);
}

/// Число писем, которое стол показывает до публикации, — то самое, которое
/// заведёт раскладка: адрес, попавший и в книгу, и в просьбу о продолжении,
/// считается один раз. После раскладки стол говорит, что письма уже ушли.
#[sqlx::test]
async fn the_letter_forecast_matches_the_lay_out(pool: PgPool) {
    let svc = service(&pool);
    let repo = Repository::new(pool.clone());
    let beginning = seed_tale(&pool, "Начало", None, None, 600).await;
    svc.send_tale_letters().await.expect("первый проход");
    sign_the_book(&pool, "one@x.io").await;
    sign_the_book(&pool, "asked@x.io").await;
    svc.set_tale_sequel_wish(beginning, &wish("w1", true, Some("Asked@x.io"), true), None)
        .await
        .expect("просьба");
    svc.set_tale_sequel_wish(beginning, &wish("w2", true, Some("only-asked@x.io"), true), None)
        .await
        .expect("вторая просьба");

    let tale = seed_tale(&pool, "Продолжение", None, Some(beginning), 60).await;
    let before = svc.admin_tale_letters_forecast(tale).await.expect("прогноз");
    assert!(!before.laid);
    assert_eq!(before.letters, 3);
    assert_eq!(before.notes, 0);

    assert_eq!(repo.lay_out_tale_letters(tale).await.expect("раскладка"), 3);
    let after = svc.admin_tale_letters_forecast(tale).await.expect("прогноз после");
    assert!(after.laid);
}

/// Адрес байки меняется, пока о ней не ушли письма, и не меняется после:
/// ссылка в разосланном письме не должна вести в «не найдено».
#[sqlx::test]
async fn a_tale_keeps_its_address_once_letters_are_out(pool: PgPool) {
    let svc = service(&pool);
    let repo = Repository::new(pool.clone());
    let tale = seed_tale(&pool, "Байка", None, None, 60).await;
    let rename = |slug: &str| {
        serde_json::from_value::<gotiga_server::models::SaveGazetteLeafRequest>(serde_json::json!({
            "slug": slug, "kind": "tale", "status": "draft",
            "titleEn": "Tale", "titleRu": "Байка", "bodyRu": "Текст",
        }))
        .expect("запрос")
    };

    let moved = svc.admin_update_gazette_leaf(tale, rename("first-name")).await.expect("правка");
    assert_eq!(moved.slug, "first-name");

    repo.lay_out_tale_letters(tale).await.expect("раскладка");
    let kept = svc.admin_update_gazette_leaf(tale, rename("second-name")).await.expect("правка после");
    assert_eq!(kept.slug, "first-name");
}
