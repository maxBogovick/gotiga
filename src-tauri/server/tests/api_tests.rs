use gotiga_server::api;
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::logs::AdminLogStore;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use std::path::PathBuf;
use tokio::fs;
use tokio::net::TcpListener;

// Spin up the real router against a test Postgres pool on a random port.
async fn spawn_app(pool: PgPool) -> (String, String, PathBuf) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let addr = format!("http://127.0.0.1:{}", port);

    let upload_dir = format!("./test_uploads_{}", uuid::Uuid::new_v4());
    fs::create_dir_all(&upload_dir).await.unwrap();

    let config = Config {
        database_url: "postgres://localhost/ignored".to_string(),
        host: "127.0.0.1".to_string(),
        port,
        admin_api_key: "test-admin-api-key-0123456789".to_string(),
        upload_dir: upload_dir.clone(),
        public_url: format!("http://127.0.0.1:{}", port),
        rust_log: "".to_string(),
        admin_login: "admin".to_string(),
        admin_password: "test-password-123".to_string(),
        cors_allowed_origins: vec![format!("http://127.0.0.1:{}", port)],
        telegram_bot_token: None,
        telegram_chat_id: None,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_pass: None,
        smtp_from: None,
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-api-logs-{}.sqlite", uuid::Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".to_string(),
    };

    let repo = Repository::new(pool);
    let service = AppService::new(repo, config.clone());
    let log_store = AdminLogStore::open(&config.admin_log_db_path)
        .await
        .unwrap();
    let router = api::router(service, config.clone(), log_store);

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (addr, config.admin_api_key, PathBuf::from(upload_dir))
}

#[sqlx::test]
#[ignore = "requires a reachable PostgreSQL test database"]
async fn health_and_public_listing(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let (addr, api_key, upload_dir) = spawn_app(pool).await;
    let client = reqwest::Client::new();

    // Health check responds OK.
    let resp = client
        .get(format!("{}/api/v1/health", addr))
        .header("x-request-id", "test-request-id-1")
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok()),
        Some("test-request-id-1")
    );

    // Readiness checks the real database connection.
    let resp = client
        .get(format!("{}/api/v1/ready", addr))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let ready: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(ready["status"], "ready");
    assert_eq!(ready["checks"]["postgres"], "ok");

    // Public figurine listing returns a JSON array (empty on a fresh DB).
    let resp = client
        .get(format!("{}/api/v1/figurines", addr))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(list.len(), 0);

    // Metrics are exposed in Prometheus text format, but only to admin callers.
    let resp = client
        .get(format!("{}/api/v1/metrics", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    let metrics = client
        .get(format!("{}/api/v1/metrics", addr))
        .bearer_auth(&api_key)
        .send()
        .await
        .unwrap();
    assert!(metrics.status().is_success());
    let body = metrics.text().await.unwrap();
    assert!(body.contains("gotiga_http_requests_total"));
    assert!(body.contains("gotiga_http_request_duration_seconds_bucket"));
    assert!(body.contains("gotiga_build_info"));

    let resp = client
        .get(format!("{}/api/v1/admin/logs?limit=10", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    let logs = client
        .get(format!("{}/api/v1/admin/logs?limit=10", addr))
        .bearer_auth(&api_key)
        .send()
        .await
        .unwrap();
    assert!(logs.status().is_success());
    let body: serde_json::Value = logs.json().await.unwrap();
    assert!(body["items"].is_array());
    assert!(body["droppedTotal"].is_number());

    let _ = fs::remove_dir_all(upload_dir).await;
}

#[sqlx::test]
#[ignore = "requires a reachable PostgreSQL test database"]
async fn analytics_accepts_text_plain_and_exposes_admin_page(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let figurine_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO figurines (id, name) VALUES ($1, $2)")
        .bind(figurine_id)
        .bind("Analytics Test Figurine")
        .execute(&pool)
        .await
        .unwrap();

    let (addr, api_key, upload_dir) = spawn_app(pool).await;
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "eventType": "figurine_view",
        "figurineId": figurine_id.to_string(),
        "path": format!("/figurines/{figurine_id}"),
        "pageViewId": uuid::Uuid::new_v4().to_string(),
        "clientTs": chrono::Utc::now().to_rfc3339(),
    });

    let ingest = client
        .post(format!("{}/api/v1/analytics/events", addr))
        .header("content-type", "text/plain;charset=UTF-8")
        .body(payload.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(ingest.status(), reqwest::StatusCode::NO_CONTENT);

    let page = client
        .get(format!("{}/api/v1/admin/analytics/figurines", addr))
        .bearer_auth(&api_key)
        .send()
        .await
        .unwrap();
    assert!(page.status().is_success());
    let body: serde_json::Value = page.json().await.unwrap();
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["figurineId"], figurine_id.to_string());

    let _ = fs::remove_dir_all(upload_dir).await;
}
