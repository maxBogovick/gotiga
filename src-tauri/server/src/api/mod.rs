use axum::{
    routing::{get, post, delete, patch, put},
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
        .route("/figurines/in-progress",        get(handlers::list_in_progress_figurines))
        .route("/figurines/:id",                get(handlers::get_figurine))
        .route("/content/texts/:param",         get(handlers::get_texts_by_param))
        .route("/cabinet/zones",                get(handlers::get_cabinet_zones))
        .route("/assets/:table/:id",            get(handlers::get_asset))
        .route("/main-background",              get(handlers::get_main_background))
        .route("/home-content",                 get(handlers::get_home_content))
        .route("/author/profile",               get(handlers::get_author_profile))
        .route("/orders",                       post(handlers::create_order))
        .route("/figurines/:id/schedule",       get(handlers::get_figurine_schedule))
        .route("/figurines/:id/comments",       get(handlers::get_figurine_comments)
                                                .post(handlers::submit_comment))
        .route("/figurines/:id/book",           post(handlers::create_booking))
        .route("/figurines/:id/waitlist",       post(handlers::join_waitlist))
        .route("/booking-rules",                get(handlers::get_booking_rules))
        .route("/settings/contact",             get(handlers::get_contact_settings))
        .route("/bookings/by-tokens",           post(handlers::get_bookings_by_tokens))
        .route("/bookings/cancel/:token",       get(handlers::get_booking_by_token)
                                                .post(handlers::cancel_booking_by_token))
        .route("/bookings/cancel/:token/reschedule", patch(handlers::reschedule_booking_by_token))
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
        // === SERVER SETTINGS (ADMIN) ===
        .route("/admin/settings/smtp",
            get(handlers::admin_get_smtp_settings)
            .put(handlers::admin_save_smtp_settings)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/settings/contact",
            put(handlers::admin_save_contact_settings)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === THEME CONFIG ===
        .route("/settings/theme",
            get(handlers::get_theme_config))
        .route("/admin/settings/theme",
            put(handlers::save_theme_config)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === COPY OVERRIDES ===
        .route("/settings/copy",
            get(handlers::get_copy_overrides))
        .route("/admin/settings/copy",
            put(handlers::save_copy_overrides)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === COMMENTS (ADMIN) ===
        .route("/admin/comments",
            get(handlers::admin_list_comments)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/comments/:id",
            patch(handlers::admin_moderate_comment)
            .delete(handlers::admin_delete_comment)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === ORDERS (ADMIN) ===
        .route("/admin/orders",
            get(handlers::list_orders)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/orders/:id",
            patch(handlers::update_order_status)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === SHOWINGS (ADMIN) ===
        .route("/admin/showings",
            get(handlers::list_showings)
            .post(handlers::save_showing)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/showings/:id",
            delete(handlers::delete_showing)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === BOOKINGS (ADMIN) ===
        .route("/admin/bookings",
            get(handlers::list_bookings)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/bookings/:id/status",
            put(handlers::update_booking_status)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === BOOKING RULES (ADMIN) ===
        .route("/admin/booking-rules",
            put(handlers::save_booking_rules)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === WAITLIST (ADMIN) ===
        .route("/admin/waitlist",
            get(handlers::admin_list_waitlist)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/waitlist/:id",
            delete(handlers::admin_remove_from_waitlist)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/waitlist/:figurine_id/notify",
            post(handlers::admin_notify_waitlist)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === USER AUTH ===
        .route("/auth/register",          post(handlers::user_register))
        .route("/auth/login/challenge",   post(handlers::user_login_challenge))
        .route("/auth/login/verify",      post(handlers::user_login_verify))
        .route("/auth/logout",            post(handlers::user_logout))
        .route("/auth/me",                get(handlers::user_me))
        .route("/auth/link-bookings",     post(handlers::user_link_bookings))
        .route("/profile/bookings",       get(handlers::user_profile_bookings))
        .route("/profile/orders",         get(handlers::user_profile_orders))
        .route("/profile/me",             patch(handlers::user_update_profile).delete(handlers::user_delete_account))
        .route("/profile/avatar",         post(handlers::user_upload_avatar))
        .route("/profile/threads",            get(handlers::user_get_threads).post(handlers::user_create_thread))
        .route("/profile/threads/:id",        get(handlers::user_get_thread))
        .route("/profile/threads/:id/reply",  post(handlers::user_reply_to_thread))
        // === ADMIN USER MANAGEMENT ===
        .route("/admin/users",
            get(handlers::admin_list_users)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/users/:id",
            get(handlers::admin_get_user)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/users/:id/sessions",
            delete(handlers::admin_revoke_user_sessions)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/users/:id/notes",
            patch(handlers::admin_update_user_notes)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/users/:id/block",
            patch(handlers::admin_set_user_blocked)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/users/:id/reset-token",
            post(handlers::admin_generate_reset_token)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/threads",
            get(handlers::admin_list_threads)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/threads/:id",
            get(handlers::admin_get_thread)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/threads/:id/reply",
            post(handlers::admin_reply_to_thread)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/threads/:id/resolve",
            post(handlers::admin_resolve_thread)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/threads/:id/reopen",
            post(handlers::admin_reopen_thread)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        .route("/admin/users/:id/threads",
            post(handlers::admin_create_thread_for_user)
            .route_layer(middleware::from_fn_with_state(config.clone(), auth_middleware)))
        // === PASSWORD RESET (PUBLIC) ===
        .route("/auth/reset-token/:token",  get(handlers::validate_reset_token))
        .route("/auth/reset-password",      post(handlers::apply_password_reset))
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
        .route("/sitemap.xml", get(handlers::sitemap_xml))
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
