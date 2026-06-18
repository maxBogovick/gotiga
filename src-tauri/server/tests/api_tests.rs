use gotiga_server::api;
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio::fs;
use std::path::PathBuf;

// Spin up the real router against a test Postgres pool on a random port.
async fn spawn_app(pool: PgPool) -> (String, String, PathBuf) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind random port");
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
    };

    let repo = Repository::new(pool);
    let service = AppService::new(repo, config.clone());
    let router = api::router(service, config.clone());

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (addr, config.admin_api_key, PathBuf::from(upload_dir))
}

#[sqlx::test]
#[ignore = "requires a reachable PostgreSQL test database"]
async fn health_and_public_listing(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let (addr, _api_key, upload_dir) = spawn_app(pool).await;
    let client = reqwest::Client::new();

    // Health check responds OK.
    let resp = client.get(format!("{}/api/v1/health", addr)).send().await.unwrap();
    assert!(resp.status().is_success());

    // Public figurine listing returns a JSON array (empty on a fresh DB).
    let resp = client.get(format!("{}/api/v1/figurines", addr)).send().await.unwrap();
    assert!(resp.status().is_success());
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(list.len(), 0);

    let _ = fs::remove_dir_all(upload_dir).await;
}
