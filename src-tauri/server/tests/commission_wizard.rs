//! Мастер заявки: способы связи вместо обязательной почты, закрытые списки, срок, фото гостя.

use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::error::AppError;
use gotiga_server::models::CommissionRequest;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use uuid::Uuid;

fn config() -> Config {
    Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port: 0,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-wizard-uploads-{}", Uuid::new_v4()),
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
        admin_log_db_path: format!("/tmp/gotiga-wizard-logs-{}.sqlite", Uuid::new_v4()),
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



fn request(json: serde_json::Value) -> CommissionRequest {
    let mut base = serde_json::json!({ "ageConfirmed": true, "description": "" });
    for (k, v) in json.as_object().unwrap() {
        base[k] = v.clone();
    }
    serde_json::from_value(base).unwrap()
}

fn is_bad_request<T: std::fmt::Debug>(r: Result<T, AppError>) -> bool {
    matches!(r, Err(AppError::BadRequest(_)))
}

#[sqlx::test]
async fn only_a_whatsapp_contact_is_enough(pool: PgPool) {
    let svc = service(&pool);
    let req = request(serde_json::json!({
        "contacts": [{ "channel": "whatsapp", "value": "+7 (900) 123-45-67" }],
        "wish": "new",
        "choices": { "who": ["witch"], "mood": ["sly"] },
        "sizeKey": "m",
        "deadlineKind": "asap",
        "budgetKey": "100_200",
        "budgetCurrency": "EUR"
    }));
    let made = svc.create_commission(&req, false).await.unwrap();
    let dto = svc
        .get_commission_by_token(&made.claim_token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dto.requester_email, None);
    assert_eq!(dto.contacts.len(), 1);
    assert_eq!(dto.contacts[0].value, "+79001234567");
    assert_eq!(dto.wish.as_deref(), Some("new"));
    assert_eq!(dto.choices.who, vec!["witch"]);
    assert_eq!(dto.size_key.as_deref(), Some("m"));
    assert_eq!(dto.budget_currency.as_deref(), Some("EUR"));
    assert_eq!(dto.description, "");
}

#[sqlx::test]
async fn an_email_contact_is_copied_to_the_requester_email(pool: PgPool) {
    let svc = service(&pool);
    let req = request(serde_json::json!({
        "contacts": [{ "channel": "email", "value": "Anna@Mail.com" }]
    }));
    let made = svc.create_commission(&req, false).await.unwrap();
    let dto = svc.get_commission_by_token(&made.claim_token).await.unwrap().unwrap();
    assert_eq!(dto.requester_email.as_deref(), Some("anna@mail.com"));
}

#[sqlx::test]
async fn no_way_to_reach_the_person_is_refused(pool: PgPool) {
    let svc = service(&pool);
    assert!(is_bad_request(svc.create_commission(&request(serde_json::json!({})), false).await));
    assert!(is_bad_request(
        svc.create_commission(&request(serde_json::json!({ "requesterEmail": "  " })), false)
            .await
    ));
}

#[sqlx::test]
async fn a_bad_phone_or_channel_is_refused(pool: PgPool) {
    let svc = service(&pool);
    let bad_phone = request(serde_json::json!({
        "contacts": [{ "channel": "viber", "value": "12345" }]
    }));
    assert!(is_bad_request(svc.create_commission(&bad_phone, false).await));
    let unknown = request(serde_json::json!({
        "contacts": [{ "channel": "icq", "value": "123456789" }]
    }));
    assert!(is_bad_request(svc.create_commission(&unknown, false).await));
}

#[sqlx::test]
async fn unknown_choice_keys_are_dropped(pool: PgPool) {
    let svc = service(&pool);
    let req = request(serde_json::json!({
        "requesterEmail": "a@b.co",
        "choices": { "keep": ["look", "nonsense"], "who": ["dragon"], "extra": ["x"] }
    }));
    let made = svc.create_commission(&req, false).await.unwrap();
    let dto = svc.get_commission_by_token(&made.claim_token).await.unwrap().unwrap();
    assert_eq!(dto.choices.keep, vec!["look"]);
    assert!(dto.choices.who.is_empty());
}

#[sqlx::test]
async fn a_past_deadline_is_refused_and_a_future_one_kept(pool: PgPool) {
    let svc = service(&pool);
    let past = request(serde_json::json!({
        "requesterEmail": "a@b.co", "deadlineKind": "date", "deadline": "2020-01-01"
    }));
    assert!(is_bad_request(svc.create_commission(&past, false).await));
    let future = request(serde_json::json!({
        "requesterEmail": "a@b.co", "deadlineKind": "date", "deadline": "2999-01-01"
    }));
    let made = svc.create_commission(&future, false).await.unwrap();
    let dto = svc.get_commission_by_token(&made.claim_token).await.unwrap().unwrap();
    assert_eq!(dto.deadline.as_deref(), Some("2999-01-01"));
}

#[sqlx::test]
async fn a_guest_photo_is_claimed_once(pool: PgPool) {
    let svc = service(&pool);
    let url = "/static/commission/test-photo.webp";
    svc.record_commission_upload(url).await.unwrap();
    let req = request(serde_json::json!({
        "requesterEmail": "a@b.co",
        "attachmentUrls": [{ "url": url }]
    }));
    let made = svc.create_commission(&req, false).await.unwrap();
    let dto = svc.get_commission_by_token(&made.claim_token).await.unwrap().unwrap();
    assert_eq!(dto.attachments.len(), 1);
    let (claimed,): (bool,) =
        sqlx::query_as("SELECT claimed_at IS NOT NULL FROM commission_uploads WHERE url = $1")
            .bind(url)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(claimed);
    // Второй раз то же фото занять нельзя.
    assert!(is_bad_request(svc.create_commission(&req, false).await));
}

#[sqlx::test]
async fn a_guest_cannot_attach_a_photo_from_elsewhere(pool: PgPool) {
    let svc = service(&pool);
    for url in ["/static/images/preview/x.jpg", "https://example.com/a.jpg", "/static/commission/never-uploaded.webp"] {
        let req = request(serde_json::json!({
            "requesterEmail": "a@b.co",
            "attachmentUrls": [{ "url": url }]
        }));
        assert!(is_bad_request(svc.create_commission(&req, false).await), "{url}");
    }
    // Вошедший по-прежнему прикладывает свои.
    let req = request(serde_json::json!({
        "requesterEmail": "a@b.co",
        "attachmentUrls": [{ "url": "/static/images/preview/x.jpg" }]
    }));
    assert!(svc.create_commission(&req, true).await.is_ok());
}

#[sqlx::test]
async fn a_guest_may_attach_at_most_three_photos(pool: PgPool) {
    let svc = service(&pool);
    let mut list = Vec::new();
    for i in 0..4 {
        let url = format!("/static/commission/p{i}.webp");
        svc.record_commission_upload(&url).await.unwrap();
        list.push(serde_json::json!({ "url": url }));
    }
    let req = request(serde_json::json!({ "requesterEmail": "a@b.co", "attachmentUrls": list }));
    assert!(is_bad_request(svc.create_commission(&req, false).await));
}

#[sqlx::test]
async fn the_sweep_removes_only_old_unclaimed_photos(pool: PgPool) {
    let svc = service(&pool);
    for url in ["/static/commission/old.webp", "/static/commission/fresh.webp", "/static/commission/kept.webp"] {
        svc.record_commission_upload(url).await.unwrap();
    }
    sqlx::query("UPDATE commission_uploads SET created_at = NOW() - INTERVAL '2 days' WHERE url <> '/static/commission/fresh.webp'")
        .execute(&pool).await.unwrap();
    sqlx::query("UPDATE commission_uploads SET claimed_at = NOW() WHERE url = '/static/commission/kept.webp'")
        .execute(&pool).await.unwrap();
    assert_eq!(svc.sweep_commission_uploads().await.unwrap(), 1);
    let (left,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM commission_uploads")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(left, 2);
}
