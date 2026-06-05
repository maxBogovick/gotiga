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
    pub message: Option<String>,
    pub mode: OrderMode,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
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
    pub purpose: Option<String>,
    pub starts_at: chrono::NaiveDate,
    pub ends_at: chrono::NaiveDate,
    pub status: BookingStatus,
    pub admin_notes: Option<String>,
    pub cancel_token: String,
    pub created_at: DateTime<Utc>,
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
    pub purpose: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub status: BookingStatus,
    pub admin_notes: Option<String>,
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
    pub purpose: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBookingStatusRequest {
    pub status: BookingStatus,
    pub admin_notes: Option<String>,
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
