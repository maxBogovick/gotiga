use gotiga_server::{api, config, db, services};
use std::net::SocketAddr;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // 1. Load Config
    let config = config::Config::from_env();

    // 2. Setup Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.rust_log))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting server on {}:{}", config.host, config.port);

    // 3. Connect to DB
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to Postgres");

    // 4. Run Migrations
    sqlx::migrate!("./migrations/")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // 5. Initialize Layers
    let repo = db::Repository::new(pool);
    let service = services::AppService::new(repo, config.clone());
    
    // Initialize content pool (hot load active release)
    if let Err(e) = service.initialize().await {
        tracing::warn!("Failed to load active release: {}. Server will start in empty mode.", e);
    }

    let router = api::router(service, config.clone());

    // 6. Start Server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid address");
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

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
