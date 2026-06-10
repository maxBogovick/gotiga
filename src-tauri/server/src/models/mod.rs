use serde::{Deserialize, Serialize};
use sqlx::prelude::Type;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ============================================================
// ENUMS
// ============================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "figurine_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FigurineStatus {
    Available,
    Sold,
    Reserved,
    InProgress,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "order_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    New,
    Seen,
    Replied,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "order_mode", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum OrderMode {
    Request,
    Question,
    Notify,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "image_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ImageType {
    Face,
    Detail,
    Full,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "step_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum StepType {
    Sketch,
    Prototype,
    Modeling,
    Painting,
    Finish,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "zone_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ZoneType {
    Showcase,
    Desk,
    Shelf,
    Note,
    Curator,
    Cabinet,
    Portrait,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "text_category", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TextCategory {
    Author,
    Workshop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "showing_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ShowingType {
    Exhibition,
    Private,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "booking_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BookingStatus {
    Pending,
    Confirmed,
    Rejected,
    Cancelled,
    Completed,
}

// ============================================================
// ENTITIES (DB MAPPING)
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Figurine {
    pub id: Uuid,
    pub name: String,
    pub short_text: Option<String>,
    pub full_description: Option<String>,
    pub dimensions: Option<String>,
    pub material: Option<String>,
    pub technique: Option<String>,
    pub year: Option<i32>,
    pub ambience_path: Option<String>,
    pub video_url: Option<String>,
    pub secret_text: Option<String>,
    pub is_visible: bool,
    pub is_featured: bool,
    pub status: FigurineStatus,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Image {
    pub id: Uuid,
    pub figurine_id: Uuid,
    pub image_type: ImageType,
    pub file_path: String,
    pub original_path: Option<String>,
    pub thumb_path: Option<String>,
    pub alt_text: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ProcessStep {
    pub id: Uuid,
    pub figurine_id: Uuid,
    pub step_type: StepType,
    pub description: Option<String>,
    pub image_path: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Text {
    pub id: Uuid,
    pub category: TextCategory,
    pub content: String,
    pub caption: Option<String>,
    pub image_path: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct CabinetZone {
    pub id: Uuid,
    pub zone_type: ZoneType,
    pub x_percent: f64,
    pub y_percent: f64,
    pub width_percent: f64,
    pub height_percent: f64,
    pub target_route: String,
    pub sort_order: i32,
}

// Postgres — timestamps are real TIMESTAMPTZ, use DateTime<Utc>
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub id: Uuid,
    pub version: i32,
    pub file_path: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
}

// ============================================================
// DTOs (API Contract — camelCase for JS frontend)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineListItemDto {
    pub id: String,
    pub name: String,
    pub status: FigurineStatus,
    pub face_image_url: Option<String>,
    pub year: Option<i32>,
    pub sort_order: i32,
    pub series: Option<String>,
    pub technique: Option<String>,
    pub material: Option<String>,
    pub is_featured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDto {
    pub id: String,
    pub image_type: ImageType,
    pub url: String,
    pub original_url: Option<String>,
    pub thumb_url: Option<String>,
    pub alt_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessStepDto {
    pub id: String,
    pub step_type: StepType,
    pub description: Option<String>,
    pub image_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineDto {
    pub id: String,
    pub name: String,
    pub short_text: Option<String>,
    pub full_description: Option<String>,
    pub dimensions: Option<String>,
    pub material: Option<String>,
    pub technique: Option<String>,
    pub year: Option<i32>,
    pub ambience_path: Option<String>,
    pub video_url: Option<String>,
    pub secret_text: Option<String>,
    pub status: FigurineStatus,
    pub sort_order: i32,
    pub is_visible: bool,
    pub is_featured: bool,

    #[serde(default)]
    pub images: Vec<ImageDto>,
    #[serde(default)]
    pub process_steps: Vec<ProcessStepDto>,
    #[serde(default)]
    pub related_items: Vec<FigurineListItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CabinetZoneDto {
    pub id: String,
    pub zone_type: ZoneType,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub target_route: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDto {
    pub id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItemDto {
    pub id: String,
    pub content: String,
    pub caption: Option<String>,
    pub image_url: Option<String>,
}

// ============================================================
// Admin Request DTOs
// ============================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFigurineRequest {
    pub id: String,
    pub name: String,
    pub short_text: Option<String>,
    pub full_description: Option<String>,
    pub dimensions: Option<String>,
    pub material: Option<String>,
    pub technique: Option<String>,
    pub year: Option<i32>,
    pub ambience_path: Option<String>,
    pub video_url: Option<String>,
    pub secret_text: Option<String>,
    pub status: FigurineStatus,
    pub sort_order: i32,
    pub is_visible: bool,
    pub is_featured: bool,
    #[serde(default)]
    pub images: Vec<SaveImageRequest>,
    #[serde(default)]
    pub process_steps: Vec<SaveStepRequest>,
    #[serde(default)]
    pub related_item_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImageRequest {
    pub id: String,
    pub image_type: ImageType,
    pub url: String,
    pub original_url: Option<String>,
    pub thumb_url: Option<String>,
    pub alt_text: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStepRequest {
    pub id: String,
    pub step_type: StepType,
    pub description: Option<String>,
    pub image_url: String,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTextRequest {
    pub id: String,
    pub content: String,
    pub caption: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveZoneRequest {
    pub id: String,
    pub zone_type: ZoneType,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub target_route: String,
}

// ============================================================
// Misc
// ============================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub version: i64,
    pub generated_at: String,
    pub figurines: Vec<Figurine>,
    pub images: Vec<Image>,
    pub process_steps: Vec<ProcessStep>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePayload {
    pub figurines: Vec<FigurineDto>,
    pub author_texts: Vec<TextDto>,
    pub workshop_items: Vec<WorkshopItemDto>,
    pub zones: Vec<CabinetZoneDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HomeContent {
    pub title: Option<String>,
    pub kicker: Option<String>,
    pub lead: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthorProfile {
    pub name: String,
    pub tagline: Option<String>,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    pub instagram: Option<String>,
    pub telegram: Option<String>,
    pub vk: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderRequest {
    pub figurine_id: String,
    pub figurine_name: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub message: Option<String>,
    #[serde(default = "default_order_mode")]
    pub mode: OrderMode,
}

fn default_order_mode() -> OrderMode { OrderMode::Request }

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub id: Uuid,
    pub figurine_id: String,
    pub figurine_name: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub message: Option<String>,
    pub mode: OrderMode,
    pub status: OrderStatus,
    pub admin_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrderStatusRequest {
    pub status: OrderStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrdersPage {
    pub items: Vec<Order>,
    pub total: i64,
    pub new_count: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaUsageDto {
    pub path: String,
    pub label: String,
    pub entity_type: String,
    pub entity_id: String,
    pub field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFileDto {
    pub path: String,
    pub url: String,
    pub media_type: String,
    pub variant: Option<String>,
    pub size_bytes: u64,
    pub exists: bool,
    pub usages: Vec<MediaUsageDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInventoryDto {
    pub files: Vec<MediaFileDto>,
    pub orphan_count: usize,
    pub used_count: usize,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCleanupReportDto {
    pub files: Vec<MediaFileDto>,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaReplaceResultDto {
    pub old_path: String,
    pub new_path: String,
    pub updated_references: usize,
    pub imported_paths: Vec<String>,
}

// ============================================================
// SHOWINGS & BOOKINGS
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Showing {
    pub id: Uuid,
    pub figurine_id: Uuid,
    pub title: String,
    pub showing_type: ShowingType,
    pub starts_at: chrono::NaiveDate,
    pub ends_at: chrono::NaiveDate,
    pub venue: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Booking {
    pub id: Uuid,
    pub figurine_id: Uuid,
    pub figurine_name: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub purpose: Option<String>,
    pub display_type: Option<String>,
    pub venue: Option<String>,
    pub starts_at: chrono::NaiveDate,
    pub ends_at: chrono::NaiveDate,
    pub status: BookingStatus,
    pub admin_notes: Option<String>,
    pub curator_conditions: Option<String>,
    pub cancel_token: String,
    pub created_at: DateTime<Utc>,
    pub user_id: Option<Uuid>,
}

/// Returned to the user after booking creation — includes the cancel token.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookingCreatedResponse {
    pub cancel_token: String,
}

/// Returned by the public token-lookup endpoint — minimal info, no PII.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookingCancelInfo {
    pub figurine_name: String,
    pub figurine_id: String,
    pub starts_at: String,
    pub ends_at: String,
    pub status: BookingStatus,
    pub admin_notes: Option<String>,
    pub curator_conditions: Option<String>,
}

/// Batch token lookup — clients poll several claim tokens at once.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookingsByTokensRequest {
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowingDto {
    pub id: String,
    pub figurine_id: String,
    pub title: String,
    pub showing_type: ShowingType,
    pub starts_at: String,
    pub ends_at: String,
    pub venue: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookingDto {
    pub id: String,
    pub figurine_id: String,
    pub figurine_name: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub purpose: Option<String>,
    pub display_type: Option<String>,
    pub venue: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub status: BookingStatus,
    pub admin_notes: Option<String>,
    pub curator_conditions: Option<String>,
    pub created_at: String,
}

// Public schedule entry — no requester names for privacy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleEntryDto {
    pub entry_type: String,
    pub title: Option<String>,
    pub showing_type: Option<ShowingType>,
    pub venue: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineScheduleDto {
    pub entries: Vec<ScheduleEntryDto>,
}

// Admin request DTOs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveShowingRequest {
    pub id: Option<String>,
    pub figurine_id: String,
    pub title: String,
    pub showing_type: ShowingType,
    pub starts_at: String,
    pub ends_at: String,
    pub venue: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBookingRequest {
    pub figurine_id: String,
    pub figurine_name: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub purpose: Option<String>,
    pub display_type: Option<String>,
    pub venue: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBookingStatusRequest {
    pub status: BookingStatus,
    pub admin_notes: Option<String>,
    pub curator_conditions: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookingsPage {
    pub items: Vec<BookingDto>,
    pub total: i64,
    pub pending_count: i64,
    pub page: i64,
    pub per_page: i64,
}

// ============================================================
// USER ACCOUNTS & AUTH
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub visual_password_hash: String,
    pub admin_notes: Option<String>,
    pub is_blocked: bool,
    pub password_reset_token: Option<String>,
    pub password_reset_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// One token entry inside the challenge JSONB array
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeToken {
    pub token: String,
    pub category: String,
    pub icon_id: String,
}

// ── Request DTOs ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    /// icon_id per category in fixed order: animals, dishes, seasons, colors
    pub selections: [String; 4],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginChallengeRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginVerifyRequest {
    pub challenge_id: String,
    /// one-time tokens in category order: animals, dishes, seasons, colors
    pub tokens: [String; 4],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkBookingsRequest {
    /// cancel_tokens from localStorage gotiga_claims_*
    pub cancel_tokens: Vec<String>,
}

// ── Response DTOs ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

impl From<&User> for UserDto {
    fn from(u: &User) -> Self {
        UserDto {
            id: u.id.to_string(),
            email: u.email.clone(),
            display_name: u.display_name.clone(),
            avatar_url: u.avatar_url.clone(),
            created_at: u.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    pub display_name: String,
}

/// One icon in a challenge grid step — token replaces real ID
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeIconDto {
    pub token: String,
    pub icon_id: String, // for frontend SVG lookup — safe to send (no ordering info)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeStepDto {
    pub category: String,
    pub icons: Vec<ChallengeIconDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginChallengeResponse {
    pub challenge_id: String,
    pub steps: Vec<ChallengeStepDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginVerifyResponse {
    pub session_token: String,
    pub user: UserDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBookingDto {
    pub id: String,
    pub figurine_id: String,
    pub figurine_name: String,
    pub display_type: Option<String>,
    pub venue: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub status: BookingStatus,
    pub created_at: String,
    pub cancel_token: String,
    pub curator_conditions: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOrderDto {
    pub id: String,
    pub figurine_id: String,
    pub figurine_name: String,
    pub mode: OrderMode,
    pub status: OrderStatus,
    pub created_at: String,
}

// ============================================================
// ADMIN USER MANAGEMENT
// ============================================================

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserListItem {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub admin_notes: Option<String>,
    pub created_at: String,
    pub booking_count: i64,
    pub order_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminSessionDto {
    pub id: String,
    pub created_at: String,
    pub expires_at: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserDetail {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub admin_notes: Option<String>,
    pub created_at: String,
    pub bookings: Vec<UserBookingDto>,
    pub orders: Vec<UserOrderDto>,
    pub sessions: Vec<AdminSessionDto>,
    pub recent_failures: i64,
    pub messages: Vec<MessageThreadDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserAdminNotesRequest {
    pub admin_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserBlockedRequest {
    pub blocked: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetTokenResponse {
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPasswordResetRequest {
    pub token: String,
    /// icon_id per category in fixed order: animals, dishes, seasons, colors
    pub selections: [String; 4],
}

// ============================================================
// FIGURINE COMMENTS
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub figurine_id: Uuid,
    pub user_id: Option<Uuid>,
    pub author_name: String,
    pub author_email: Option<String>,
    pub body: String,
    pub is_approved: bool,
    pub admin_reply: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CommentWithAvatar {
    pub id: Uuid,
    pub figurine_id: Uuid,
    pub user_id: Option<Uuid>,
    pub author_name: String,
    pub author_email: Option<String>,
    pub body: String,
    pub is_approved: bool,
    pub admin_reply: Option<String>,
    pub created_at: DateTime<Utc>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentDto {
    pub id: String,
    pub author_name: String,
    pub author_avatar_url: Option<String>,
    pub body: String,
    pub admin_reply: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCommentDto {
    pub id: String,
    pub figurine_id: String,
    pub figurine_name: String,
    pub author_name: String,
    pub author_email: Option<String>,
    pub body: String,
    pub is_approved: bool,
    pub admin_reply: Option<String>,
    pub created_at: String,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCommentsPage {
    pub items: Vec<AdminCommentDto>,
    pub total: i64,
    pub pending_count: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitCommentRequest {
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub body: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModerateCommentRequest {
    pub is_approved: bool,
    pub admin_reply: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtpSettings {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub from: Option<String>,
}

// ============================================================
// BOOKING RULES
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookingRules {
    /// Minimum booking duration in days (inclusive)
    pub min_days: i64,
    /// Maximum booking duration in days (inclusive)
    pub max_days: i64,
    /// How many days in advance the booking must start (0 = today allowed)
    pub advance_days: i64,
}

impl Default for BookingRules {
    fn default() -> Self {
        Self { min_days: 1, max_days: 30, advance_days: 0 }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescheduleBookingRequest {
    pub starts_at: String,
    pub ends_at: String,
}

// ============================================================
// WAITLIST
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WaitlistEntry {
    pub id: Uuid,
    pub figurine_id: Uuid,
    pub figurine_name: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitlistEntryDto {
    pub id: String,
    pub figurine_id: String,
    pub figurine_name: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
    pub user_id: Option<String>,
}

// ============================================================
// MESSAGING THREADS
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MessageThread {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: String,
    pub reference_id: Option<Uuid>,
    pub subject: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_message_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ThreadMessage {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub from_admin: bool,
    pub body: String,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageThreadDto {
    pub id: String,
    pub category: String,
    pub reference_id: Option<String>,
    pub subject: String,
    pub status: String,
    pub unread: i64,
    pub last_message_at: String,
    pub created_at: String,
    pub preview: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMessageDto {
    pub id: String,
    pub thread_id: String,
    pub from_admin: bool,
    pub body: String,
    pub read_at: Option<String>,
    pub created_at: String,
}

impl From<&ThreadMessage> for ThreadMessageDto {
    fn from(m: &ThreadMessage) -> Self {
        ThreadMessageDto {
            id: m.id.to_string(),
            thread_id: m.thread_id.to_string(),
            from_admin: m.from_admin,
            body: m.body.clone(),
            read_at: m.read_at.map(|t| t.to_rfc3339()),
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadDetailDto {
    pub thread: MessageThreadDto,
    pub messages: Vec<ThreadMessageDto>,
    pub user: Option<ThreadUserDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadUserDto {
    pub id: String,
    pub display_name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateThreadRequest {
    pub subject: String,
    pub body: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyToThreadRequest {
    pub body: String,
}

// ============================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWaitlistRequest {
    pub figurine_name: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub note: Option<String>,
}
