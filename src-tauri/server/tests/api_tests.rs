use gotiga_server::api;
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::services::AppService;
use sqlx::{PgPool, SqlitePool};
use tokio::net::TcpListener;
use tokio::fs;
use std::path::PathBuf;

// Helper to spawn app
async fn spawn_app(pool: PgPool) -> (String, String, PathBuf) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let addr = format!("http://127.0.0.1:{}", port);

    // Create a temporary upload dir for this test
    let upload_dir = format!("./test_uploads_{}", uuid::Uuid::new_v4());
    fs::create_dir_all(format!("{}/releases", upload_dir)).await.unwrap();

    let config = Config {
        database_url: "postgres://...".to_string(), // Ignored by test harness usually
        host: "127.0.0.1".to_string(),
        port,
        admin_api_key: "secret".to_string(),
        upload_dir: upload_dir.clone(),
        public_url: format!("http://127.0.0.1:{}", port),
        rust_log: "".to_string(),
    };

    let repo = Repository::new(pool);
    let service = AppService::new(repo, config.clone());
    let router = api::router(service, config.clone());

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (addr, config.admin_api_key, PathBuf::from(upload_dir))
}

// Helper to create a dummy SQLite DB with content
async fn create_dummy_content_db() -> (PathBuf, Vec<u8>) {
    let file_name = format!("test_content_{}.db", uuid::Uuid::new_v4());
    let path = std::env::temp_dir().join(&file_name);
    
    // Initialize SQLite
    let url = format!("sqlite://{}?mode=rwc", path.to_string_lossy());
    let pool = SqlitePool::connect(&url).await.unwrap();

    // Create Tables (Schema matching schema.sql on client)
    sqlx::query(
        "CREATE TABLE figurines (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, status TEXT, is_visible BOOLEAN, sort_order INTEGER,
            short_text TEXT, full_description TEXT, dimensions TEXT, material TEXT, technique TEXT, year INTEGER,
            ambience_path TEXT, video_url TEXT, secret_text TEXT, updated_at TEXT, created_at TEXT,
            ambience_data BLOB, video_data BLOB
        )"
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "CREATE TABLE images (
            id TEXT PRIMARY KEY, figurine_id TEXT, image_type TEXT, file_path TEXT, alt_text TEXT, sort_order INTEGER, 
            updated_at TEXT, created_at TEXT, data BLOB
        )"
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "CREATE TABLE process_steps (
            id TEXT PRIMARY KEY, figurine_id TEXT, step_type TEXT, description TEXT, image_path TEXT, sort_order INTEGER,
            updated_at TEXT, created_at TEXT, image_data BLOB
        )"
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "CREATE TABLE texts (
            id TEXT PRIMARY KEY, category TEXT, content TEXT, caption TEXT, image_path TEXT, sort_order INTEGER,
            updated_at TEXT, created_at TEXT, image_data BLOB
        )"
    ).execute(&pool).await.unwrap();

    sqlx::query(
        "CREATE TABLE cabinet_zones (
            id TEXT PRIMARY KEY, zone_type TEXT, x_percent REAL, y_percent REAL, width_percent REAL, height_percent REAL, target_route TEXT, sort_order INTEGER
        )"
    ).execute(&pool).await.unwrap();

    // Insert Dummy Data
    let fig_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO figurines (id, name, status, is_visible, sort_order) VALUES (?, 'Test Figurine', 'available', 1, 0)"
    )
    .bind(&fig_id)
    .execute(&pool).await.unwrap();

    let img_id = uuid::Uuid::new_v4().to_string();
    let blob_data = b"fake_image_data".to_vec();
    sqlx::query(
        "INSERT INTO images (id, figurine_id, image_type, file_path, data) VALUES (?, ?, 'face', 'images/face.jpg', ?)"
    )
    .bind(&img_id)
    .bind(&fig_id)
    .bind(&blob_data)
    .execute(&pool).await.unwrap();

    pool.close().await;

    let bytes = fs::read(&path).await.unwrap();
    (path, bytes)
}

#[sqlx::test]
async fn test_full_release_cycle(pool: PgPool) {
    // 1. Setup
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let (addr, api_key, upload_dir) = spawn_app(pool).await;
    let client = reqwest::Client::new();

    // 2. Prepare SQLite File
    let (_tmp_path, db_bytes) = create_dummy_content_db().await;

    // 3. Upload Release DB
    let part = reqwest::multipart::Part::bytes(db_bytes)
        .file_name("release.db");
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = client
        .post(format!("{}/api/v1/release/db", addr))
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .expect("Failed to upload DB");

    if !response.status().is_success() {
        let text = response.text().await.unwrap();
        panic!("Upload failed: {}", text);
    }

    // 4. Verify Content API (List Figurines)
    // The server should have hot-swapped to the new DB.
    let response = client
        .get(format!("{}/api/v1/figurines", addr))
        .send()
        .await
        .expect("Failed to list figurines");

    assert!(response.status().is_success());
    let list: Vec<serde_json::Value> = response.json().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "Test Figurine");

    // 5. Verify Asset Streaming
    // We need the image ID. Let's get details.
    let id = list[0]["id"].as_str().unwrap();
    let response = client
        .get(format!("{}/api/v1/figurines/{}", addr, id))
        .send()
        .await
        .unwrap();
    let details: serde_json::Value = response.json().await.unwrap();
    let img_url = details["images"][0]["url"].as_str().unwrap();
    
    // img_url should be like http://.../assets/images/{uuid}
    assert!(img_url.contains("/assets/images/"));

    // Download Asset
    let response = client.get(img_url).send().await.unwrap();
    assert!(response.status().is_success());
    let bytes = response.bytes().await.unwrap();
    assert_eq!(bytes, b"fake_image_data".as_ref());

    // 6. Verify Sync Download
    let response = client
        .get(format!("{}/api/v1/sync/db", addr))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    // Should be valid sqlite header
    let sync_bytes = response.bytes().await.unwrap();
    assert!(sync_bytes.len() > 100); 

    // Cleanup
    let _ = fs::remove_dir_all(upload_dir).await;
}