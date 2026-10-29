//! Журнал фоновых проходов: обёртка пишет итог каждого запуска, не меняет результат прохода
//! и не пускает в таблицу адреса из текста ошибки.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::error::AppError;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-signals-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-signals-logs-{}.sqlite", Uuid::new_v4()),
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


#[sqlx::test]
async fn a_run_is_recorded_and_the_result_passes_through_unchanged(pool: PgPool) {
    let svc = service(&pool);
    let out = svc
        .track_job("demo", 300, async { Ok::<_, AppError>(3_u32) }, |n| {
            Some(format!("did {n}"))
        })
        .await
        .expect("результат проходит как есть");
    assert_eq!(out, 3);

    let jobs = svc.admin_jobs().await.unwrap();
    let j = jobs.iter().find(|j| j.row.job == "demo").expect("строка прохода");
    assert!(j.row.ok && j.state == "ok");
    assert_eq!((j.row.runs, j.row.failures, j.row.in_a_row, j.row.every_secs), (1, 0, 0, 300));
    assert_eq!(j.row.note.as_deref(), Some("did 3"));
    assert!(j.row.did_at.is_some() && j.row.ok_at.is_some());
}

#[sqlx::test]
async fn an_idle_run_keeps_the_last_note_and_a_failure_is_counted(pool: PgPool) {
    let svc = service(&pool);
    svc.track_job("demo", 300, async { Ok::<_, AppError>(2_u32) }, |n| Some(format!("did {n}")))
        .await
        .unwrap();
    // Вхолостую: заметка прежняя, счётчик растёт.
    svc.track_job("demo", 300, async { Ok::<_, AppError>(0_u32) }, |_| None).await.unwrap();
    let j = svc.admin_jobs().await.unwrap().into_iter().find(|j| j.row.job == "demo").unwrap();
    assert_eq!((j.row.runs, j.row.note.as_deref()), (2, Some("did 2")));

    // Ошибка: проход возвращает её же, журнал отмечает сбой и ничего лишнего не хранит.
    let err = svc
        .track_job(
            "demo",
            300,
            async {
                Err::<u32, _>(AppError::Internal(
                    "error sending request for url (https://api.telegram.org/bot1:SECRET/sendMessage)".into(),
                ))
            },
            |_| None,
        )
        .await
        .expect_err("ошибка проходит наружу");
    assert!(err.to_string().contains("error sending request"));
    let j = svc.admin_jobs().await.unwrap().into_iter().find(|j| j.row.job == "demo").unwrap();
    assert!(!j.row.ok);
    assert_eq!(j.state, "failed");
    assert_eq!((j.row.runs, j.row.failures, j.row.in_a_row), (3, 1, 1));
    let stored = j.row.error.clone().unwrap();
    assert!(!stored.contains("SECRET") && !stored.contains("telegram.org"), "{stored}");
    assert_eq!(j.row.note.as_deref(), Some("did 2"), "заметка пережила сбой");

    // Следующий удачный запуск сбрасывает подряд-сбои.
    svc.track_job("demo", 300, async { Ok::<_, AppError>(0_u32) }, |_| None).await.unwrap();
    let j = svc.admin_jobs().await.unwrap().into_iter().find(|j| j.row.job == "demo").unwrap();
    assert!(j.row.ok && j.row.in_a_row == 0 && j.row.failures == 1 && j.row.error.is_none());
}

#[sqlx::test]
async fn a_job_that_stopped_running_shows_as_late(pool: PgPool) {
    let svc = service(&pool);
    svc.track_job("demo", 300, async { Ok::<_, AppError>(()) }, |_| None).await.unwrap();
    sqlx::query("UPDATE job_runs SET started_at = NOW() - INTERVAL '2 hours' WHERE job = 'demo'")
        .execute(&pool)
        .await
        .unwrap();
    let j = svc.admin_jobs().await.unwrap().into_iter().find(|j| j.row.job == "demo").unwrap();
    assert_eq!(j.state, "late");
    assert!(j.row.age_secs >= 7200);
}
