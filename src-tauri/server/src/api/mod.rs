use axum::{
    routing::{get, post, delete},
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
use axum::extract::DefaultBodyLimit;

mod handlers;

#[derive(Clone)]
pub struct AppState {
    pub service: AppService,
    pub config: Config,
}

impl axum::extract::FromRef<AppState> for AppService {
    fn from_ref(state: &AppState) -> Self { state.service.clone() }
}

impl axum::extract::FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self { state.config.clone() }
}

pub fn router(service: AppService, config: Config) -> Router {
    let state = AppState { service, config: config.clone() };

    // All routes under /api/v1 — no auth on the router level
    let api = Router::new()
        // === PUBLIC READ ===
        .route("/health",                       get(handlers::health_check))
        .route("/sync/db",                      get(handlers::download_release_db))
        .route("/figurines",                    get(handlers::list_figurines))
        .route("/figurines/:id",                get(handlers::get_figurine))
        .route("/content/texts/:param",         get(handlers::get_texts_by_param))
        .route("/cabinet/zones",                get(handlers::get_cabinet_zones))
        .route("/assets/:table/:id",            get(handlers::get_asset))
        .route("/main-background",              get(handlers::get_main_background))
        .route("/home-content",                 get(handlers::get_home_content))
        .route("/author/profile",               get(handlers::get_author_profile))
        .route("/orders",                       post(handlers::create_order))
        // === PUBLIC LOGIN ===
        .route("/admin/login",                  post(handlers::admin_login))
        // === PROTECTED WRITE — use route_layer so auth only runs on matched routes ===
        .route("/figurines",
            post(handlers::save_figurine)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/figurines/:id",
            delete(handlers::delete_figurine)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/upload",
            post(handlers::upload_file)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/media",
            get(handlers::get_media_inventory)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/media/cleanup-report",
            get(handlers::get_unused_media_report)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/media/cleanup",
            post(handlers::cleanup_unused_media)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/media/replace",
            post(handlers::replace_media_everywhere)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/cabinet/zones",
            post(handlers::save_zone)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/cabinet/zones/:id",
            delete(handlers::delete_zone)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/content/texts/:param",
            post(handlers::save_text)
            .delete(handlers::delete_text)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/main-background",
            post(handlers::upload_main_background)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/home-content",
            post(handlers::save_home_content)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/author/profile",
            post(handlers::save_author_profile)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === RELEASES ===
        .route("/admin/releases",
            get(handlers::list_releases)
            .post(handlers::upload_release_db)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/release/db",
            post(handlers::upload_release_db)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/releases/:id/activate",
            post(handlers::switch_release)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // Legacy paths
        .route("/releases",
            get(handlers::list_releases)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/releases/:id/activate",
            post(handlers::switch_release)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 500));

    Router::new()
        .nest("/api/v1", api)
        .nest_service("/static", ServeDir::new(&config.upload_dir))
        .layer(CorsLayer::permissive())
        .with_state(state)
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

    if auth_header != format!("Bearer {}", config.admin_api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
