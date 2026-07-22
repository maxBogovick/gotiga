use gotiga_server::{api, config, db, logs, services};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // 1. Load Config
    let config = config::Config::from_env();
    let log_store = logs::AdminLogStore::open(&config.admin_log_db_path).await?;
    let admin_log_layer = log_store.layer();

    // 2. Setup Logging
    let env_filter = tracing_subscriber::EnvFilter::new(&config.rust_log);
    if matches!(dotenvy::var("LOG_FORMAT").as_deref(), Ok("json")) {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(admin_log_layer)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_current_span(false)
                    .with_span_list(false),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(admin_log_layer)
            .with(tracing_subscriber::fmt::layer().compact())
            .init();
    }

    // NB: never log database_url — it contains DB credentials.
    tracing::info!("Starting server on {}:{}", config.host, config.port);

    // 3. Connect to DB
    // Pool is sized for a public web server: several handlers issue a handful of
    // sequential queries per request (get_figurine_details, sitemap/feed), so 5
    // connections queued behind a 30s default acquire timeout could stall under
    // even light concurrency. Tunable via DB_MAX_CONNECTIONS; timeouts are explicit
    // so a saturated pool fails fast (5s) rather than hanging, and idle/aged
    // connections are recycled.
    let db_max_connections: u32 = dotenvy::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(20);
    let pool = PgPoolOptions::new()
        .max_connections(db_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to Postgres");

    // 4. Run Migrations (runtime path — picks up new .sql files without recompilation)
    sqlx::migrate::Migrator::new(std::path::Path::new("./migrations"))
        .await
        .expect("Failed to load migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // 5. Initialize Layers
    let repo = db::Repository::new(pool);
    let service = services::AppService::new(repo, config.clone());

    // Initialize content pool (hot load active release)
    if let Err(e) = service.initialize().await {
        tracing::warn!(
            "Failed to load active release: {}. Server will start in empty mode.",
            e
        );
    }

    // Background: rebuild image renditions that predate the current pipeline — the 900px
    // `medium` variant (new) and every `.webp` (previously encoded lossless, i.e. 6-7x
    // LARGER than its JPEG). Rebuilt from images/original, idempotent, and detached so it
    // never delays the listener.
    {
        let upload_dir = config.upload_dir.clone();
        tokio::spawn(async move {
            api::backfill_image_variants(upload_dir).await;
        });
    }

    // Background: give an existing main background (uploaded before the resize/
    // WebP pipeline existed) the same treatment a fresh upload gets now. Idempotent
    // and detached — see backfill_background_image's doc comment.
    {
        let upload_dir = config.upload_dir.clone();
        tokio::spawn(async move {
            api::backfill_background_image(upload_dir).await;
        });
    }

    // Background: prune login attempts past the retention window (runs now, then daily).
    {
        let svc = service.clone();
        tokio::spawn(async move {
            const RETENTION_DAYS: i64 = 90;
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
            loop {
                tick.tick().await;
                match svc.prune_login_history(RETENTION_DAYS).await {
                    Ok(n) if n > 0 => {
                        tracing::info!("Pruned {n} login attempts older than {RETENTION_DAYS} days")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("Login-attempt prune failed: {e}"),
                }
            }
        });
    }

    // Background: prune raw analytics events in small chunks so retention never
    // creates a large delete transaction on the small production server.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            const RETENTION_DAYS: i64 = gotiga_server::analytics::RETENTION_DAYS;
            const BATCH_SIZE: i64 = 1000;
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
            loop {
                tick.tick().await;
                loop {
                    match svc
                        .prune_analytics_events_chunked(RETENTION_DAYS, BATCH_SIZE)
                        .await
                    {
                        Ok(0) => break,
                        Ok(n) => {
                            tracing::info!(
                                "Pruned {n} analytics events older than {RETENTION_DAYS} days"
                            );
                            tokio::task::yield_now().await;
                        }
                        Err(e) => {
                            tracing::warn!("Analytics retention prune failed: {e}");
                            break;
                        }
                    }
                }
            }
        });
    }

    // Background: sweep the in-memory rate limiters. The per-request check paths
    // only prune the key they touch, so keys for one-off IPs would otherwise
    // accumulate forever — this drops stale/empty ones on a timer.
    {
        let svc = service.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                tick.tick().await;
                let removed = svc.prune_rate_limiters().await;
                if removed > 0 {
                    tracing::debug!("Rate-limiter GC dropped {removed} stale keys");
                }
            }
        });
    }

    let router = api::router(service.clone(), config.clone(), log_store);

    // 6. Start Server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid address");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    service.shutdown_analytics().await;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Signal received, starting graceful shutdown");
}
