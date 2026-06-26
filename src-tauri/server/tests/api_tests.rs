use gotiga_server::api;
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::logs::AdminLogStore;
use gotiga_server::services::AppService;
use sqlx::PgPool;
use std::path::PathBuf;
use tokio::fs;
use tokio::net::TcpListener;
use uuid::Uuid;

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

// ── delete_figurine ──────────────────────────────────────────────────────────

/// Deleting a figurine removes its row, all cascade-linked rows, **and** the
/// `figurine_analytics_events` rows that have no FK (manual delete).
#[sqlx::test]
#[ignore = "requires a reachable PostgreSQL test database"]
async fn delete_figurine_cascades_rows_and_analytics_events(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let (addr, api_key, upload_dir) = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    let fig_id = Uuid::new_v4();
    sqlx::query("INSERT INTO figurines (id, name) VALUES ($1, $2)")
        .bind(fig_id)
        .bind("Delete Cascade Test")
        .execute(&pool)
        .await
        .unwrap();

    // Insert a raw analytics event (no FK — the code must delete this manually).
    sqlx::query(
        "INSERT INTO figurine_analytics_events \
         (figurine_id, event_date, event_type, path, source) \
         VALUES ($1, CURRENT_DATE, 'figurine_view', '/figurines/x', 'direct')",
    )
    .bind(fig_id)
    .execute(&pool)
    .await
    .unwrap();

    // Insert a child images row so we can verify cascade-delete too.
    let img_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO images (id, figurine_id, image_type, file_path, sort_order) \
         VALUES ($1, $2, 'face', 'images/dummy.jpg', 0)",
    )
    .bind(img_id)
    .bind(fig_id)
    .execute(&pool)
    .await
    .unwrap();

    // Call DELETE via the admin HTTP endpoint.
    let resp = client
        .delete(format!("{}/api/v1/figurines/{}", addr, fig_id))
        .bearer_auth(&api_key)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "delete returned {}", resp.status());

    // Figurine row must be gone.
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM figurines WHERE id = $1")
            .bind(fig_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 0, "figurine row should be deleted");

    // Cascade: images row must be gone.
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM images WHERE figurine_id = $1")
            .bind(fig_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 0, "images rows should cascade-delete");

    // Manual: analytics events must be gone.
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM figurine_analytics_events WHERE figurine_id = $1",
    )
    .bind(fig_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count.0, 0, "analytics events should be manually deleted");

    let _ = fs::remove_dir_all(upload_dir).await;
}

/// Image files on disk (all variants: main, original, thumb, depth) are
/// removed when the figurine is deleted; http URLs are left untouched.
#[sqlx::test]
#[ignore = "requires a reachable PostgreSQL test database"]
async fn delete_figurine_removes_image_files_from_disk(pool: PgPool) {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let (addr, api_key, upload_dir) = spawn_app(pool.clone()).await;
    let client = reqwest::Client::new();

    let fig_id = Uuid::new_v4();
    sqlx::query("INSERT INTO figurines (id, name) VALUES ($1, $2)")
        .bind(fig_id)
        .bind("File Cleanup Test")
        .execute(&pool)
        .await
        .unwrap();

    // Create real files in the upload dir for each path variant.
    let make_file = |rel: &str| {
        let base = upload_dir.clone();
        let rel = rel.to_string();
        async move {
            let p = base.join(&rel);
            fs::create_dir_all(p.parent().unwrap()).await.unwrap();
            fs::write(&p, b"dummy").await.unwrap();
            rel
        }
    };
    let main_path  = make_file("images/face_main.jpg").await;
    let orig_path  = make_file("images/original/face_orig.jpg").await;
    let thumb_path = make_file("images/thumb/face_thumb.jpg").await;
    let depth_path = make_file("images/depth/face_depth.png").await;

    let img_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO images \
         (id, figurine_id, image_type, file_path, original_path, thumb_path, depth_path, sort_order) \
         VALUES ($1, $2, 'face', $3, $4, $5, $6, 0)",
    )
    .bind(img_id)
    .bind(fig_id)
    .bind(&main_path)
    .bind(&orig_path)
    .bind(&thumb_path)
    .bind(&depth_path)
    .execute(&pool)
    .await
    .unwrap();

    let resp = client
        .delete(format!("{}/api/v1/figurines/{}", addr, fig_id))
        .bearer_auth(&api_key)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    // All four file variants must have been removed.
    for rel in [&main_path, &orig_path, &thumb_path, &depth_path] {
        assert!(
            !upload_dir.join(rel).exists(),
            "file should be deleted: {rel}"
        );
    }

    let _ = fs::remove_dir_all(upload_dir).await;
}
