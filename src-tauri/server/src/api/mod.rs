use axum::{
    routing::{get, post},
    Router,
    middleware::{self, Next},
    extract::Request,
    http::{StatusCode, HeaderMap},
    response::Response,
};
use crate::services::AppService;
use crate::config::Config;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;

mod handlers;

#[derive(Clone)]
pub struct AppState {
    pub service: AppService,
    pub config: Config,
}

// Implement FromRef to allow sub-extractors to get parts of state
impl axum::extract::FromRef<AppState> for AppService {
    fn from_ref(state: &AppState) -> Self {
        state.service.clone()
    }
}

impl axum::extract::FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

use axum::extract::DefaultBodyLimit;

pub fn router(service: AppService, config: Config) -> Router {
    let state = AppState { service, config: config.clone() };

    // Define Public Routes
    let api_routes = Router::new()
        .route("/health", get(handlers::health_check))
        //.route("/sync/manifest", get(handlers::get_sync_manifest)) // Legacy/Debug
        .route("/sync/db", get(handlers::download_release_db))     // Main Sync
        // Content API (Headless Player)
        .route("/figurines", get(handlers::list_figurines))
        .route("/figurines/:id", get(handlers::get_figurine))
        .route("/content/texts/author", get(handlers::get_author_texts))
        .route("/content/texts/workshop", get(handlers::get_workshop_items))
        .route("/cabinet/zones", get(handlers::get_cabinet_zones))
        // Asset Streaming
        .route("/assets/:table/:id", get(handlers::get_asset));

    // Define Admin Routes
    let admin_routes = Router::new()
        .route("/release/db", post(handlers::upload_release_db))
        .route("/releases", get(handlers::list_releases))
        .route("/releases/:id/activate", post(handlers::switch_release))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 500)); // 500MB limit
        //.layer(middleware::from_fn_with_state(config.clone(), auth_middleware));

    let app = Router::new()
        .nest("/api/v1", api_routes.merge(admin_routes))
        .nest_service("/static", ServeDir::new(&config.upload_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    app
}

async fn auth_middleware(
    axum::extract::State(config): axum::extract::State<Config>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let expected = format!("Bearer {}", config.admin_api_key);

    if auth_header != expected {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}