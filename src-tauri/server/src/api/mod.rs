use crate::config::Config;
use crate::logs::AdminLogStore;
use crate::observability::{ObservabilityState, request_observability_middleware};
use crate::services::AppService;
use axum::extract::DefaultBodyLimit;
use axum::{
    Router,
    extract::Request,
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, patch, post, put},
};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

/// Global request-body cap. Small by default so an unauthenticated request can't
/// pin memory; the large media-upload routes opt into a higher limit explicitly.
const DEFAULT_BODY_LIMIT: usize = 16 * 1024 * 1024; // 16 MB
/// Upper bound for authenticated admin media uploads (videos/audio).
const MEDIA_UPLOAD_LIMIT: usize = 256 * 1024 * 1024; // 256 MB

pub(crate) mod handlers;

/// Re-exported for main.rs (a separate binary crate, so it cannot reach into the
/// crate-private `handlers` module directly).
pub use handlers::backfill_background_image;
pub use handlers::backfill_image_variants;

#[derive(Clone)]
pub struct AppState {
    pub service: AppService,
    pub config: Config,
    pub observability: ObservabilityState,
    pub log_store: AdminLogStore,
}

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

impl axum::extract::FromRef<AppState> for ObservabilityState {
    fn from_ref(state: &AppState) -> Self {
        state.observability.clone()
    }
}

impl axum::extract::FromRef<AppState> for AdminLogStore {
    fn from_ref(state: &AppState) -> Self {
        state.log_store.clone()
    }
}

pub fn router(service: AppService, config: Config, log_store: AdminLogStore) -> Router {
    let observability = service.observability();
    let state = AppState {
        service,
        config: config.clone(),
        observability: observability.clone(),
        log_store,
    };

    // All routes under /api/v1 — no auth on the router level
    let api =
        Router::new()
            // === PUBLIC READ ===
            .route("/health", get(handlers::health_check))
            .route("/ready", get(handlers::readiness_check))
            .route(
                "/metrics",
                get(crate::observability::metrics_handler).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/logs",
                get(crate::logs::admin_list_logs).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/logs/stream",
                get(crate::logs::admin_stream_logs).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route("/figurines", get(handlers::list_figurines))
            .route(
                "/figurines/in-progress",
                get(handlers::list_in_progress_figurines),
            )
            .route(
                "/figurines/first-look",
                get(handlers::list_first_look_figurines),
            )
            .route("/figurines/noticed", get(handlers::list_noticed_by_guests))
            .route("/figurines/:id", get(handlers::get_figurine))
            // Semantic search ("Хранитель") — natural-language ranking of the archive.
            .route("/search", get(handlers::semantic_search))
            .route("/content/texts/:param", get(handlers::get_texts_by_param))
            .route("/showing-rooms", get(handlers::get_showing_rooms))
            .route("/main-background", get(handlers::get_main_background))
            .route("/home-content", get(handlers::get_home_content))
            .route("/author/profile", get(handlers::get_author_profile))
            .route("/orders", post(handlers::create_order))
            .route("/analytics/events", post(handlers::record_analytics_event))
            .route("/commissions", post(handlers::create_commission))
            .route(
                "/commissions/:token",
                get(handlers::get_commission_by_token),
            )
            .route(
                "/figurines/:id/schedule",
                get(handlers::get_figurine_schedule),
            )
            .route(
                "/figurines/:id/comments",
                get(handlers::get_figurine_comments).post(handlers::submit_comment),
            )
            .route("/figurines/:id/book", post(handlers::create_booking))
            .route("/figurines/:id/mark", post(handlers::set_figurine_mark))
            .route("/figurines/:id/like", post(handlers::set_figurine_like))
            .route("/figurines/:id/waitlist", post(handlers::join_waitlist))
            .route("/booking-rules", get(handlers::get_booking_rules))
            .route("/settings/contact", get(handlers::get_contact_settings))
            .route("/settings/programme", get(handlers::get_programme_settings))
            .route(
                "/bookings/by-tokens",
                post(handlers::get_bookings_by_tokens),
            )
            .route(
                "/bookings/cancel/:token",
                get(handlers::get_booking_by_token).post(handlers::cancel_booking_by_token),
            )
            .route(
                "/bookings/cancel/:token/reschedule",
                patch(handlers::reschedule_booking_by_token),
            )
            .route(
                "/waitlist/leave/:token",
                get(handlers::get_waitlist_by_token).post(handlers::leave_waitlist_by_token),
            )
            .route(
                "/orders/notify/:token",
                get(handlers::get_notify_by_token).post(handlers::cancel_notify_by_token),
            )
            // === NEWSLETTER ("visitor book") ===
            .route("/subscribe", post(handlers::subscribe))
            .route(
                "/subscribe/leave/:token",
                get(handlers::get_subscription_by_token).post(handlers::unsubscribe_by_token),
            )
            // === CONTACT MESSAGES ("write to the author") ===
            .route("/contact", post(handlers::submit_contact_message))
            // === VISITOR IMPRESSIONS ("book of impressions") ===
            .route("/impressions", post(handlers::submit_impression))
            .route(
                "/impressions/featured",
                get(handlers::get_featured_impressions),
            )
            // === CABINET GAZETTE ===
            .route("/gazette/home", get(handlers::get_gazette_home))
            .route("/gazette/blotter", get(handlers::get_gazette_room))
            .route(
                "/gazette/for-work/:id",
                get(handlers::list_gazette_for_work),
            )
            .route("/gazette", get(handlers::list_gazette))
            .route(
                "/gazette/:slug/watch",
                post(handlers::watch_gazette_leaf),
            )
            .route(
                "/gazette/watch/:token",
                get(handlers::get_gazette_watch).post(handlers::leave_gazette_watch),
            )
            .route("/gazette/:slug", get(handlers::get_gazette_leaf))
            // === THE SHELF OF TALL TALES ===
            // A tale is a gazette leaf of kind `tale`; only its shelf is its own.
            // The leaf itself is still read through `/gazette/:slug`.
            .route("/tales", get(handlers::list_tales))
            // === СКРОМНЫЕ ЭПИЧЕСКИЕ БИТВЫ ===
            // The shelf and the five frames are the same for every visitor;
            // what a given person owns is not, and lives behind a session.
            .route("/battles/cards", get(handlers::list_battle_cards))
            .route("/battles/frames", get(handlers::get_battle_frames))
            .route("/battles/races", get(handlers::list_battle_races))
            .route("/battles/keywords", get(handlers::list_battle_keywords))
            // Полка испытаний видна и без имени; играть — только с именем.
            .route("/battles/challenges", get(handlers::list_battle_challenges))
            .route(
                "/battles/challenges/:id/begin",
                post(handlers::begin_battle_match),
            )
            .route("/battles/matches/:id", get(handlers::get_battle_match))
            .route("/battles/matches/:id/act", post(handlers::act_in_battle_match))
            // Кошелёк и владение. Всё под сессией: книга лежит на сервере
            // именно потому, что кошелёк в localStorage — бесконечные деньги.
            .route("/battles/me", get(handlers::get_battle_me))
            .route("/battles/buy", post(handlers::buy_battle_card))
            .route("/battles/raise", post(handlers::raise_battle_card_level))
            .route("/battles/cards/:id/seen", post(handlers::mark_battle_card_seen))
            .route("/battles/attention", post(handlers::grant_battle_attention))
            // === PUBLIC LOGIN ===
            .route("/admin/login", post(handlers::admin_login))
            // === PROTECTED WRITE — use route_layer so auth only runs on matched routes ===
            .route(
                "/figurines",
                post(handlers::save_figurine).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/figurines/:id",
                delete(handlers::delete_figurine).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/figurines/:id/generate-depth",
                post(handlers::admin_generate_figurine_depth).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/embeddings/reindex",
                post(handlers::admin_reindex_embeddings).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/figurines/:id/caption",
                get(handlers::admin_get_figurine_caption)
                    .put(handlers::admin_set_figurine_caption)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/figurines/:id/pinterest-description",
                get(handlers::admin_get_figurine_pinterest_description)
                    .put(handlers::admin_set_figurine_pinterest_description)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/upload",
                post(handlers::upload_file)
                    .layer(DefaultBodyLimit::max(MEDIA_UPLOAD_LIMIT))
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/media",
                get(handlers::get_media_inventory).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/media/cleanup-report",
                get(handlers::get_unused_media_report).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/media/cleanup",
                post(handlers::cleanup_unused_media).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/media/replace",
                post(handlers::replace_media_everywhere)
                    .layer(DefaultBodyLimit::max(MEDIA_UPLOAD_LIMIT))
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/showing-rooms",
                post(handlers::save_showing_room).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/showing-rooms/:id",
                delete(handlers::delete_showing_room).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/content/texts/:param",
                post(handlers::save_text)
                    .delete(handlers::delete_text)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/main-background",
                post(handlers::upload_main_background)
                    .layer(DefaultBodyLimit::max(MEDIA_UPLOAD_LIMIT))
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/home-content",
                post(handlers::save_home_content).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/author/profile",
                post(handlers::save_author_profile).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            // === SERVER SETTINGS (ADMIN) ===
            .route(
                "/admin/settings/smtp",
                get(handlers::admin_get_smtp_settings)
                    .put(handlers::admin_save_smtp_settings)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/settings/contact",
                put(handlers::admin_save_contact_settings).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/settings/programme",
                put(handlers::save_programme_settings).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            // === THEME CONFIG ===
            .route("/settings/theme", get(handlers::get_theme_config))
            .route(
                "/admin/settings/theme",
                put(handlers::save_theme_config).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            // === REEL THEME ===
            .route("/settings/reel-theme", get(handlers::get_reel_theme))
            .route(
                "/admin/settings/reel-theme",
                put(handlers::save_reel_theme).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/settings/reel-theme-presets",
                get(handlers::admin_get_reel_theme_presets)
                    .put(handlers::admin_save_reel_theme_presets)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === HOME LAYOUT CONFIG ===
            .route("/settings/home-layout", get(handlers::get_home_layout))
            .route(
                "/admin/settings/home-layout",
                put(handlers::save_home_layout).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/settings/home-layout-presets",
                get(handlers::admin_get_home_layout_presets)
                    .put(handlers::admin_save_home_layout_presets)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === COPY OVERRIDES ===
            .route("/settings/copy", get(handlers::get_copy_overrides))
            .route(
                "/admin/settings/copy",
                put(handlers::save_copy_overrides).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            // === DISPLAY CONFIG PRESETS ===
            .route(
                "/admin/settings/display-presets",
                get(handlers::admin_get_display_presets)
                    .put(handlers::admin_save_display_presets)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === COMMENTS (ADMIN) ===
            .route(
                "/admin/comments",
                get(handlers::admin_list_comments).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/comments/:id",
                patch(handlers::admin_moderate_comment)
                    .delete(handlers::admin_delete_comment)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === IMPRESSIONS (ADMIN) ===
            .route(
                "/admin/impressions",
                get(handlers::admin_list_impressions).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/impressions/:id",
                patch(handlers::admin_moderate_impression)
                    .delete(handlers::admin_delete_impression)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === GAZETTE (ADMIN) ===
            .route(
                "/admin/gazette",
                get(handlers::admin_list_gazette_leaves)
                    .post(handlers::admin_create_gazette_leaf)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/gazette/feeds/refresh",
                post(handlers::admin_refresh_gazette_desk).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/gazette/feeds",
                get(handlers::admin_list_gazette_feeds)
                    .post(handlers::admin_create_gazette_feed)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/gazette/feeds/:id",
                put(handlers::admin_update_gazette_feed)
                    .delete(handlers::admin_delete_gazette_feed)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/gazette/cuttings",
                get(handlers::admin_list_gazette_cuttings).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/gazette/cuttings/:id/dismiss",
                post(handlers::admin_dismiss_gazette_cutting).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/gazette/cuttings/:id/restore",
                post(handlers::admin_restore_gazette_cutting).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/gazette/cuttings/:id/pin",
                post(handlers::admin_pin_gazette_cutting).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/gazette/cuttings/:id/unpin",
                post(handlers::admin_unpin_gazette_cutting).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/gazette/cuttings/:id/promote",
                post(handlers::admin_promote_gazette_cutting).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/gazette/tales/order",
                post(handlers::admin_reorder_tales).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/gazette/:id",
                get(handlers::admin_get_gazette_leaf)
                    .put(handlers::admin_update_gazette_leaf)
                    .delete(handlers::admin_delete_gazette_leaf)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === BATTLES (ADMIN) ===
            .route(
                "/admin/battles/cards",
                get(handlers::admin_list_battle_cards)
                    .post(handlers::admin_create_battle_card)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // Ordered before `/:id` so a shelf rewrite is never read as a card id.
            .route(
                "/admin/battles/cards/order",
                post(handlers::admin_reorder_battle_cards).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/battles/cards/:id",
                get(handlers::admin_get_battle_card)
                    .put(handlers::admin_update_battle_card)
                    .delete(handlers::admin_delete_battle_card)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/battles/frames",
                post(handlers::admin_save_battle_frames).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            // Ставки начисления за внимание. Настройка хранителя, а не схема:
            // числа подбираются на живом доме, а не закладываются в миграцию.
            .route(
                "/admin/battles/dust-rates",
                get(handlers::admin_get_battle_dust_rates)
                    .post(handlers::admin_save_battle_dust_rates)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // Frame pictures keep their transparency, so they cannot go through
            // the ordinary image upload. Its own body limit: a carved frame is
            // a big PNG before it is re-encoded.
            .route(
                "/admin/battles/bench",
                post(handlers::admin_bench_battle).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/battles/challenges",
                get(handlers::admin_list_battle_challenges)
                    .post(handlers::admin_create_battle_challenge)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/battles/challenges/:id",
                put(handlers::admin_update_battle_challenge)
                    .delete(handlers::admin_delete_battle_challenge)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/battles/weigh",
                post(handlers::admin_weigh_battle_card).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/battles/keywords",
                post(handlers::admin_create_battle_keyword).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            // Before `/:id`, so a reorder is never read as a keyword id.
            .route(
                "/admin/battles/keywords/order",
                post(handlers::admin_reorder_battle_keywords).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/battles/keywords/:id",
                put(handlers::admin_update_battle_keyword)
                    .delete(handlers::admin_delete_battle_keyword)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/battles/races",
                post(handlers::admin_create_battle_race).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            // Before `/:id`, so a reorder is never read as a race id.
            .route(
                "/admin/battles/races/order",
                post(handlers::admin_reorder_battle_races).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/battles/races/:id",
                put(handlers::admin_update_battle_race)
                    .delete(handlers::admin_delete_battle_race)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/battles/frames/art",
                post(handlers::admin_upload_battle_frame_art)
                    .route_layer(DefaultBodyLimit::max(MEDIA_UPLOAD_LIMIT))
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === TOOLS (ADMIN) ===
            .route(
                "/admin/tools/convert-image",
                post(handlers::admin_convert_image)
                    .route_layer(DefaultBodyLimit::max(MEDIA_UPLOAD_LIMIT))
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === ORDERS (ADMIN) ===
            .route(
                "/admin/analytics/overview",
                get(handlers::admin_get_analytics_overview).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/commission-funnel",
                get(handlers::admin_get_commission_funnel).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/pages",
                get(handlers::admin_get_site_page_engagement).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/visitors",
                get(handlers::admin_get_visitor_sessions).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/visitors/:hash",
                get(handlers::admin_get_visitor_timeline).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/backfill",
                post(handlers::admin_backfill_analytics).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/life-of-the-house",
                get(handlers::admin_get_life_of_house_trend).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/annotations",
                get(handlers::admin_list_analytics_annotations)
                    .post(handlers::admin_create_analytics_annotation)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/analytics/annotations/:id",
                delete(handlers::admin_delete_analytics_annotation).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/figurines",
                get(handlers::admin_list_figurine_analytics).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/figurines/:id",
                get(handlers::admin_get_figurine_analytics).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/analytics/figurines/:id/geo-daily",
                get(handlers::admin_get_figurine_geo_daily).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/figurine-marks",
                get(handlers::admin_get_mark_stats).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/settings/noticed-by-guests",
                get(handlers::admin_get_noticed_by_guests_settings)
                    .put(handlers::admin_save_noticed_by_guests_settings)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/orders",
                get(handlers::list_orders).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/orders/:id",
                patch(handlers::update_order_status).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/orders/:id/certificate",
                post(handlers::issue_order_certificate)
                    .delete(handlers::revoke_order_certificate)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === COMMISSIONS (ADMIN) ===
            .route(
                "/admin/commissions",
                get(handlers::admin_list_commissions).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/commissions/:id",
                patch(handlers::admin_update_commission)
                    .delete(handlers::admin_delete_commission)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/commissions/:id/certificate",
                post(handlers::issue_commission_certificate)
                    .delete(handlers::revoke_commission_certificate)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            // === SHOWINGS (ADMIN) ===
            .route(
                "/admin/showings",
                get(handlers::list_showings)
                    .post(handlers::save_showing)
                    .route_layer(middleware::from_fn_with_state(
                        config.clone(),
                        auth_middleware,
                    )),
            )
            .route(
                "/admin/showings/:id",
                delete(handlers::delete_showing).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/showings/bulk-clear",
                post(handlers::bulk_clear_showings).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            // === BULK FIGURINE OPS (ADMIN) ===
            .route(
                "/admin/figurines/bulk/clear-darkness",
                post(handlers::bulk_clear_darkness).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/figurines/bulk/reset-parallax",
                post(handlers::bulk_reset_parallax).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/figurines/bulk/set-parallax",
                post(handlers::bulk_set_parallax).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/figurines/bulk/recalculate-parallax",
                post(handlers::bulk_recalculate_parallax).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/figurines/bulk/set-second-angle",
                post(handlers::bulk_set_second_angle).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            // === SLUGS (ADMIN) ===
            .route(
                "/admin/figurines/slugs/backfill",
                post(handlers::backfill_slugs).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/figurines/:id/slug",
                axum::routing::patch(handlers::set_figurine_slug).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            // === BOOKINGS (ADMIN) ===
            .route(
                "/admin/bookings",
                get(handlers::list_bookings).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/bookings/:id/status",
                put(handlers::update_booking_status).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            // === BOOKING RULES (ADMIN) ===
            .route(
                "/admin/booking-rules",
                put(handlers::save_booking_rules).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            // === WAITLIST (ADMIN) ===
            .route(
                "/admin/waitlist",
                get(handlers::admin_list_waitlist).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/waitlist/:id",
                delete(handlers::admin_remove_from_waitlist).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/waitlist/:figurine_id/notify",
                post(handlers::admin_notify_waitlist).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/subscribers",
                get(handlers::admin_list_subscribers).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/subscribers/:id",
                delete(handlers::admin_remove_subscriber).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/contact-messages",
                get(handlers::admin_list_contact_messages).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/contact-messages/:id/read",
                post(handlers::admin_mark_contact_message_read).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/contact-messages/:id",
                delete(handlers::admin_remove_contact_message).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            // === USER AUTH ===
            .route("/auth/register", post(handlers::user_register))
            .route(
                "/auth/login/challenge",
                post(handlers::user_login_challenge),
            )
            .route("/auth/login/verify", post(handlers::user_login_verify))
            .route("/auth/logout", post(handlers::user_logout))
            .route("/auth/me", get(handlers::user_me))
            .route("/auth/link-bookings", post(handlers::user_link_bookings))
            .route(
                "/certificates/:token",
                get(handlers::get_public_certificate),
            )
            .route("/profile/bookings", get(handlers::user_profile_bookings))
            .route("/profile/orders", get(handlers::user_profile_orders))
            .route(
                "/profile/certificates",
                get(handlers::user_profile_certificates),
            )
            .route(
                "/profile/wishlist",
                get(handlers::user_get_wishlist).put(handlers::user_set_wishlist),
            )
            .route("/profile/waitlist", get(handlers::user_profile_waitlist))
            .route(
                "/profile/gazette-watches",
                get(handlers::user_profile_gazette_watches),
            )
            .route("/profile/claims/link", post(handlers::user_link_claim))
            .route(
                "/profile/me",
                patch(handlers::user_update_profile).delete(handlers::user_delete_account),
            )
            .route("/profile/avatar", post(handlers::user_upload_avatar))
            .route("/profile/uploads", post(handlers::user_upload_file))
            .route("/profile/commissions", get(handlers::user_list_commissions))
            .route(
                "/profile/commissions/claim",
                post(handlers::user_claim_commission),
            )
            .route(
                "/profile/commissions/:id",
                patch(handlers::user_edit_commission).delete(handlers::user_delete_commission),
            )
            .route(
                "/profile/threads",
                get(handlers::user_get_threads).post(handlers::user_create_thread),
            )
            .route("/profile/threads/:id", get(handlers::user_get_thread))
            .route(
                "/profile/threads/:id/reply",
                post(handlers::user_reply_to_thread),
            )
            // === ADMIN USER MANAGEMENT ===
            .route(
                "/admin/users",
                get(handlers::admin_list_users).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/users/:id",
                get(handlers::admin_get_user).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/users/:id/sessions",
                delete(handlers::admin_revoke_user_sessions).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/users/:id/notes",
                patch(handlers::admin_update_user_notes).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/users/:id/block",
                patch(handlers::admin_set_user_blocked).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/users/:id/reset-token",
                post(handlers::admin_generate_reset_token).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            .route(
                "/admin/threads",
                get(handlers::admin_list_threads).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/threads/:id",
                get(handlers::admin_get_thread).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/threads/:id/reply",
                post(handlers::admin_reply_to_thread).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/threads/:id/resolve",
                post(handlers::admin_resolve_thread).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/threads/:id/reopen",
                post(handlers::admin_reopen_thread).route_layer(middleware::from_fn_with_state(
                    config.clone(),
                    auth_middleware,
                )),
            )
            .route(
                "/admin/users/:id/threads",
                post(handlers::admin_create_thread_for_user).route_layer(
                    middleware::from_fn_with_state(config.clone(), auth_middleware),
                ),
            )
            // === PASSWORD RESET (PUBLIC) ===
            .route("/auth/forgot-password", post(handlers::forgot_password))
            .route(
                "/auth/reset-token/:token",
                get(handlers::validate_reset_token),
            )
            .route("/auth/reset-password", post(handlers::apply_password_reset))
            .layer(middleware::from_fn(public_cache_middleware))
            .layer(DefaultBodyLimit::max(DEFAULT_BODY_LIMIT));

    // Serve only known media subdirectories — never the whole UPLOAD_DIR (which can
    // contain *.db dumps, temp files, etc.). Each subdir is mounted explicitly.
    let upload_dir = std::path::Path::new(&config.upload_dir);
    let mut app = Router::new()
        .nest("/api/v1", api)
        .route("/sitemap.xml", get(handlers::sitemap_xml))
        // Live RSS feed of new works — connected to Pinterest's "Подключить RSS-канал".
        .route("/feed.xml", get(handlers::feed_rss))
        // Live RSS of gazette leaves — separate from the Pinterest works channel.
        .route("/gazette/feed.xml", get(handlers::gazette_feed_rss));
    for subdir in ["images", "videos", "audio", "backgrounds", "avatars", "frames"] {
        app = app.nest_service(
            &format!("/static/{}", subdir),
            ServeDir::new(upload_dir.join(subdir)),
        );
    }

    // Restrict CORS to configured origins (defaults to PUBLIC_URL) instead of
    // allowing any origin to call a bearer-token API.
    let allowed_origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static(crate::observability::REQUEST_ID_HEADER),
        ])
        .expose_headers([HeaderName::from_static(
            crate::observability::REQUEST_ID_HEADER,
        )]);

    app.layer(cors)
        .layer(middleware::from_fn_with_state(
            observability,
            request_observability_middleware,
        ))
        .with_state(state)
}

/// Public content that is the same for every visitor, and so may be cached for a beat.
///
/// An allowlist rather than "anything that isn't /admin": some unauthenticated GETs return
/// per-visitor material behind an unguessable token (a booking, a reset link), and those
/// must never land in a shared cache. Paths here are the ones nested under /api/v1, i.e.
/// with that prefix already stripped by `nest`.
fn is_public_cacheable(path: &str) -> bool {
    const EXACT: &[&str] = &[
        "/figurines",
        "/figurines/in-progress",
        "/figurines/first-look",
        "/figurines/noticed",
        "/showing-rooms",
        "/main-background",
        "/home-content",
        "/author/profile",
        "/booking-rules",
        "/impressions/featured",
        "/gazette",
        "/gazette/home",
        "/gazette/blotter",
        "/battles/cards",
        "/battles/frames",
        "/battles/races",
        "/battles/keywords",
        "/battles/challenges",
    ];

    if EXACT.contains(&path) {
        return true;
    }
    // Read-only site settings: theme, reel theme, home layout, copy overrides, contacts.
    // (The admin's writable twins live under /admin/settings/… and are not matched here.)
    if path.starts_with("/settings/") || path.starts_with("/content/texts/") {
        return true;
    }
    // A single figurine: /figurines/<id>, but not its sub-resources.
    if let Some(rest) = path.strip_prefix("/figurines/") {
        return !rest.is_empty() && !rest.contains('/');
    }
    if let Some(rest) = path.strip_prefix("/gazette/") {
        if let Some(id) = rest.strip_prefix("for-work/") {
            return !id.is_empty() && !id.contains('/');
        }
        return !rest.is_empty() && !rest.contains('/') && rest != "home" && rest != "blotter";
    }
    false
}

/// Give the public reads a short shared-cache lifetime.
///
/// Nothing on this API sent `Cache-Control` at all, so every visitor — and Cloudflare in
/// front of us — re-fetched the same unchanged catalogue on every view. A minute is short
/// enough that an admin's edit is never stale for more than a beat, and `stale-while-
/// revalidate` means the refresh happens behind the visitor rather than in front of them.
///
/// Requests carrying an `Authorization` header are left alone: the very same path can be an
/// admin read (`/figurines?visible=false` returns hidden work), and that must not be stored
/// in a shared cache. `Vary: Authorization` makes that explicit to any cache in between.
async fn public_cache_middleware(req: Request, next: Next) -> Response {
    let cacheable = req.method() == Method::GET
        && req.headers().get(header::AUTHORIZATION).is_none()
        && is_public_cacheable(req.uri().path());

    let mut res = next.run(req).await;

    if cacheable
        && res.status() == StatusCode::OK
        && !res.headers().contains_key(header::CACHE_CONTROL)
    {
        res.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=60, stale-while-revalidate=600"),
        );
        res.headers_mut().insert(
            header::VARY,
            HeaderValue::from_static("Authorization, Accept-Encoding"),
        );
    }

    res
}

#[cfg(test)]
mod cache_tests {
    use super::is_public_cacheable;

    #[test]
    fn caches_the_public_catalogue_and_settings() {
        for path in [
            "/figurines",
            "/figurines/in-progress",
            "/figurines/first-look",
            "/figurines/noticed",
            "/figurines/2dc46e41-2645-59af-88b5-1fe9c31a463f",
            "/home-content",
            "/author/profile",
            "/showing-rooms",
            "/main-background",
            "/impressions/featured",
            "/gazette",
            "/gazette/home",
            "/gazette/blotter",
            "/gazette/for-work/2dc46e41-2645-59af-88b5-1fe9c31a463f",
            "/gazette/a-leaf",
            "/settings/theme",
            "/settings/home-layout",
            "/content/texts/author",
        ] {
            assert!(is_public_cacheable(path), "{path} should be cacheable");
        }
    }

    #[test]
    fn never_caches_personal_or_admin_reads() {
        // Anything reached with a secret token, anything per-visitor, anything admin: a
        // shared cache holding these is the whole reason this is an allowlist.
        for path in [
            "/admin/settings/theme",
            "/admin/logs",
            "/auth/me",
            "/profile/bookings",
            "/profile/orders",
            "/profile/threads/2dc46e41-2645-59af-88b5-1fe9c31a463f",
            "/auth/reset-token/secret-token",
            "/figurines/2dc46e41-2645-59af-88b5-1fe9c31a463f/comments",
            "/bookings/some-cancel-token",
            "/gazette/watch/secret-token",
            "/profile/gazette-watches",
        ] {
            assert!(!is_public_cacheable(path), "{path} must NOT be cacheable");
        }
    }
}

async fn auth_middleware(
    axum::extract::State(config): axum::extract::State<Config>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!(
                target: "gotiga_server::auth",
                event = "admin_auth",
                outcome = "missing",
                route = %request.uri().path(),
                "admin authorization missing"
            );
            StatusCode::UNAUTHORIZED
        })?;

    if auth_header != format!("Bearer {}", config.admin_api_key) {
        tracing::warn!(
            target: "gotiga_server::auth",
            event = "admin_auth",
            outcome = "rejected",
            route = %request.uri().path(),
            "admin authorization rejected"
        );
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
