use gotiga_server::api;
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use uuid::Uuid;
use serde_json::json;

// Helper to spawn app
async fn spawn_app(pool: PgPool) -> (String, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let addr = format!("http://127.0.0.1:{}", port);

    let config = Config {
        database_url: "postgres://postgres:password@localhost:5432/gotiga_test".to_string(), // Dummy for config
        host: "127.0.0.1".to_string(),
        port,
        admin_api_key: "secret".to_string(),
        upload_dir: "./test_uploads".to_string(),
        public_url: format!("http://127.0.0.1:{}", port),
    };

    let repo = Repository::new(pool);
    let service = AppService::new(repo, config.clone());
    let router = api::router(service, config.clone());

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (addr, config.admin_api_key)
}

#[sqlx::test]
async fn test_health_check(pool: PgPool) {
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (addr, _) = spawn_app(pool).await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/api/v1/health", addr))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["status"], "ok");
}

#[sqlx::test]
async fn test_create_and_get_figurine(pool: PgPool) {
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (addr, api_key) = spawn_app(pool).await;
    let client = reqwest::Client::new();

    // 1. Create
    let new_figurine = json!({
        "name": "Test Figurine",
        "status": "available",
        "isVisible": true,
        "sortOrder": 1,
        "images": [
            {
                "imageType": "face",
                "url": "images/test.jpg"
            }
        ]
    });

    let response = client
        .post(format!("{}/api/v1/figurines", addr))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&new_figurine)
        .send()
        .await
        .expect("Failed to create");

    assert!(response.status().is_success());
    let created_json: serde_json::Value = response.json().await.unwrap();
    let id = created_json["id"].as_str().unwrap();

    // 2. Get Details
    let response = client
        .get(format!("{}/api/v1/figurines/{}", addr, id))
        .send()
        .await
        .expect("Failed to get");
    
    assert!(response.status().is_success());
    let fetched: serde_json::Value = response.json().await.unwrap();
    assert_eq!(fetched["name"], "Test Figurine");
    assert_eq!(fetched["images"][0]["url"], format!("{}/static/images/test.jpg", addr));

    // 3. List
    let response = client
        .get(format!("{}/api/v1/figurines", addr))
        .send()
        .await
        .unwrap();
    
    let list: Vec<serde_json::Value> = response.json().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"], id);
}

#[sqlx::test]
async fn test_manifest_generation(pool: PgPool) {
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (addr, _) = spawn_app(pool).await;
    let client = reqwest::Client::new();

    let response = client.get(format!("{}/api/v1/sync/manifest", addr)).send().await.unwrap();
    assert!(response.status().is_success());
    
    let manifest: serde_json::Value = response.json().await.unwrap();
    assert!(manifest["version"].is_number());
    assert!(manifest["figurines"].is_array());
}
