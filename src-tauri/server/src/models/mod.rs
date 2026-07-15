use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::Type;
use uuid::Uuid;

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
    Reserve,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "reserve_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReserveStatus {
    Requested,
    Reviewing,
    TermsSent,
    Confirmed,
    Declined,
    Expired,
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
// QUERIES
// ============================================================

/// Filter + sort + pagination parameters for the figurine listing endpoint.
#[derive(Debug, Default, Deserialize)]
pub struct FigurineQuery {
    pub status: Option<String>,
    pub search: Option<String>,
    pub sort: Option<String>,
    /// Page number (1-based). None → no pagination, return everything.
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurinesPage {
    pub items: Vec<crate::models::FigurineListItemDto>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

// ============================================================
// ENTITIES (DB MAPPING)
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Figurine {
    pub id: Uuid,
    pub name: String,
    /// Transliterated URL slug (unique when set); NULL for legacy rows not yet
    /// re-saved. The detail route resolves either this or the UUID.
    pub slug: Option<String>,
    /// True when the slug was hand-typed by an admin (differs from the name-derived
    /// auto slug); false when auto-generated. Drives the «Work addresses» badge.
    pub slug_manual: bool,
    pub short_text: Option<String>,
    pub full_description: Option<String>,
    pub dimensions: Option<String>,
    pub material: Option<String>,
    pub technique: Option<String>,
    pub year: Option<i32>,
    pub passport_number: Option<String>,
    pub edition: Option<String>,
    pub created_period: Option<String>,
    pub care_instructions: Option<String>,
    pub provenance_note: Option<String>,
    pub authenticity_note: Option<String>,
    pub included_items: Option<String>,
    pub ambience_path: Option<String>,
    pub video_url: Option<String>,
    pub secret_text: Option<String>,
    pub is_visible: bool,
    pub is_featured: bool,
    /// "The house wakes" — daily showing window in minutes from midnight (0..1439),
    /// guest-local. Both NULL → always open. `until < from` wraps past midnight.
    pub open_from_min: Option<i32>,
    pub open_until_min: Option<i32>,
    /// Optional sealed-door asset URL. NULL → procedural carved door on the client.
    pub sealed_door_image: Option<String>,
    /// Optional "showing room" this work belongs to. When set, the room's window
    /// is used instead of the per-figurine open_from/until. NULL → use own window.
    pub showing_room_id: Option<Uuid>,
    /// Which detail-page layout to use. NULL → 'specimen' (default).
    pub display_layout: Option<String>,
    /// JSON blob for per-figurine display customisation ({background, blockOrder}).
    pub display_config: Option<String>,
    /// "First look" early-release window. While now < this, the work is held back
    /// from the public archive and shown only on the book-holders' shelf. NULL →
    /// no window (ordinary public work).
    pub first_look_until: Option<DateTime<Utc>>,
    pub status: FigurineStatus,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A named, shared showing window several works can point at (e.g. "Night hall").
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ShowingRoom {
    pub id: Uuid,
    pub name: String,
    pub open_from_min: i32,
    pub open_until_min: i32,
    /// Allowed weekdays bitmask (bit0=Mon … bit6=Sun). NULL → every day.
    pub open_days_mask: Option<i32>,
    /// "MM-DD" — opens every year on that date. NULL → unused.
    pub open_month_day: Option<String>,
    /// One-off inclusive date range "YYYY-MM-DD". NULL → unused.
    pub open_date_from: Option<String>,
    pub open_date_until: Option<String>,
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
    pub depth_path: Option<String>,
    pub parallax_intensity: Option<f32>,
    /// "Keyhole" reveal focus (normalised 0..1) and radius (0..1 of the frame).
    /// NULL = unset → renderer falls back to centre + default radius.
    pub focal_x: Option<f32>,
    pub focal_y: Option<f32>,
    pub reveal_radius: Option<f32>,
    /// Per-image darkness override (0..1). NULL → global keyhole darkness.
    pub darkness: Option<f32>,
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

// ============================================================
// DTOs (API Contract — camelCase for JS frontend)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineListItemDto {
    pub id: String,
    pub name: String,
    /// Transliterated URL slug; null for works not yet re-saved. Lets list
    /// links point at `/figurines/{slug}` without a per-item detail fetch.
    #[serde(default)]
    pub slug: Option<String>,
    /// True when the slug was hand-typed (differs from the name-derived auto slug).
    #[serde(default)]
    pub slug_manual: bool,
    pub status: FigurineStatus,
    /// The work's one-line note. Carried on the LIST item on purpose: the home page
    /// renders it under each plate, and without it here the client had to fire one
    /// extra full `GET /figurines/{id}` per visible work just to read this one string
    /// — a dozen round-trips, each pulling the entire record and its image array, to
    /// populate a single caption. It costs nothing to include: `to_list_item` already
    /// receives the whole `Figurine` row, so this is a field that was already fetched
    /// and then thrown away. No new query, no migration.
    #[serde(default)]
    pub short_text: Option<String>,
    /// 420px thumbnail — sized for the archive's dense grid of cards.
    pub face_image_url: Option<String>,
    /// Second-angle image for the home gallery's hover reveal; null when the
    /// piece has no dedicated "detail" image.
    #[serde(default)]
    pub detail_image_url: Option<String>,
    /// The same two images at preview size (1800px). Surfaces where a list item
    /// is rendered LARGE — the home hero and the home reel plates — which would
    /// otherwise upscale the 420px thumbnail two to three times over.
    #[serde(default)]
    pub face_image_large_url: Option<String>,
    #[serde(default)]
    pub detail_image_large_url: Option<String>,
    pub year: Option<i32>,
    pub sort_order: i32,
    pub series: Option<String>,
    pub technique: Option<String>,
    pub material: Option<String>,
    pub is_featured: bool,
    /// When the piece was catalogued — lets the showcase mark recently added works.
    pub created_at: DateTime<Utc>,
    /// When the piece was last edited — lets the home "since your visit" ledger
    /// surface updated works (new photo, edited text, status change), not just
    /// brand-new arrivals.
    pub updated_at: DateTime<Utc>,
    /// Face-image "keyhole" reveal focus + radius + darkness, surfaced on the card.
    pub focal_x: Option<f32>,
    pub focal_y: Option<f32>,
    pub reveal_radius: Option<f32>,
    pub darkness: Option<f32>,
    /// Showing window (minutes from midnight); both NULL → always open. The card
    /// shows a sealed door while the guest's local clock is outside the window.
    pub open_from_min: Option<i32>,
    pub open_until_min: Option<i32>,
    /// Optional sealed-door asset; null → procedural carved door.
    pub sealed_door_image: Option<String>,
    /// Showing room this work belongs to (null → uses its own window).
    pub showing_room_id: Option<String>,
    /// "First look" early-release window — set while the work is a book-holders'
    /// preview; null once public. Lets the shelf note when it opens to all.
    pub first_look_until: Option<DateTime<Utc>>,
    /// "House Favorite" — a rare, loud badge for pieces in the top percentile of
    /// weighted mark score among marked figurines (see Repository::get_favorite_tiers).
    /// Never a number, just a boolean.
    #[serde(default)]
    pub house_favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowingRoomDto {
    pub id: String,
    pub name: String,
    pub open_from_min: i32,
    pub open_until_min: i32,
    pub open_days_mask: Option<i32>,
    pub open_month_day: Option<String>,
    pub open_date_from: Option<String>,
    pub open_date_until: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveShowingRoomRequest {
    pub id: String,
    pub name: String,
    pub open_from_min: i32,
    pub open_until_min: i32,
    #[serde(default)]
    pub open_days_mask: Option<i32>,
    #[serde(default)]
    pub open_month_day: Option<String>,
    #[serde(default)]
    pub open_date_from: Option<String>,
    #[serde(default)]
    pub open_date_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDto {
    pub id: String,
    pub image_type: ImageType,
    pub url: String,
    pub original_url: Option<String>,
    pub thumb_url: Option<String>,
    pub depth_url: Option<String>,
    pub parallax_intensity: Option<f32>,
    pub focal_x: Option<f32>,
    pub focal_y: Option<f32>,
    pub reveal_radius: Option<f32>,
    pub darkness: Option<f32>,
    pub alt_text: Option<String>,
}

/// Result of an on-demand depth-map generation run for one figurine.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthGenSummary {
    pub generated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub results: Vec<DepthGenItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthGenItem {
    pub image_id: String,
    pub status: String, // "done" | "skip" | "fail"
    pub detail: Option<String>,
}

/// Result of a bulk admin operation applied across every figurine/image.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOpSummary {
    pub affected: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkSetParallaxRequest {
    pub intensity: f32,
}

/// Admin request to set/regenerate a single work's URL slug. `slug: None` or a
/// blank string means "regenerate from the work's name".
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSlugRequest {
    pub slug: Option<String>,
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
    /// Transliterated URL slug; null for works not yet re-saved. The client
    /// builds `/figurines/{slug ?? id}` and canonicalises to the slug.
    #[serde(default)]
    pub slug: Option<String>,
    pub short_text: Option<String>,
    pub full_description: Option<String>,
    pub dimensions: Option<String>,
    pub material: Option<String>,
    pub technique: Option<String>,
    pub year: Option<i32>,
    pub passport_number: Option<String>,
    pub edition: Option<String>,
    pub created_period: Option<String>,
    pub care_instructions: Option<String>,
    pub provenance_note: Option<String>,
    pub authenticity_note: Option<String>,
    pub included_items: Option<String>,
    pub ambience_path: Option<String>,
    pub video_url: Option<String>,
    pub secret_text: Option<String>,
    pub status: FigurineStatus,
    pub sort_order: i32,
    pub is_visible: bool,
    pub is_featured: bool,
    /// Showing window (minutes from midnight); both NULL → always open.
    pub open_from_min: Option<i32>,
    pub open_until_min: Option<i32>,
    /// Optional sealed-door asset; null → procedural carved door.
    pub sealed_door_image: Option<String>,
    /// Showing room this work belongs to (null → uses its own window).
    pub showing_room_id: Option<String>,
    /// Which detail-page layout to use. null → 'specimen' (default).
    pub display_layout: Option<String>,
    /// JSON blob for per-figurine display customisation ({background, blockOrder}).
    pub display_config: Option<String>,
    /// "First look" early-release window (null once public).
    pub first_look_until: Option<DateTime<Utc>>,

    #[serde(default)]
    pub images: Vec<ImageDto>,
    #[serde(default)]
    pub process_steps: Vec<ProcessStepDto>,
    #[serde(default)]
    pub related_items: Vec<FigurineListItemDto>,
    /// True once this piece has crossed a private mark-count threshold — the
    /// ONLY public trace of the marks-of-attention system. Deliberately a
    /// boolean, never a number: tells a new visitor "others paused here too"
    /// without exposing a raw count (which would look weak below the
    /// threshold and become a de facto public rating above it).
    #[serde(default)]
    pub noticed_by_others: bool,
    /// "House Favorite" — the loud, rare second tier above `noticed_by_others`.
    /// Same non-numeric principle: a boolean, never a count.
    #[serde(default)]
    pub house_favorite: bool,
}

// ============================================================
// FIGURINE ANALYTICS
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEventType {
    FigurineView,
    FigurineEngaged,
    FigurineCtaClick,
}

impl AnalyticsEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FigurineView => "figurine_view",
            Self::FigurineEngaged => "figurine_engaged",
            Self::FigurineCtaClick => "figurine_cta_click",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsEventRequest {
    pub event_type: AnalyticsEventType,
    pub figurine_id: String,
    pub path: String,
    pub referrer: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub duration_ms: Option<i32>,
    pub scroll_depth: Option<i32>,
    pub cta_type: Option<String>,
    pub page_view_id: Option<String>,
    pub client_ts: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct AnalyticsEventRecord {
    pub occurred_at: DateTime<Utc>,
    pub event_date: NaiveDate,
    pub event_type: &'static str,
    pub figurine_id: Uuid,
    pub visitor_hash: Option<String>,
    pub page_view_id: Option<Uuid>,
    pub path: String,
    pub source: String,
    pub referrer_host: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub device_class: Option<String>,
    pub browser_family: Option<String>,
    pub country_code: Option<String>,
    pub duration_ms: Option<i32>,
    pub scroll_depth: Option<i32>,
    pub cta_type: Option<String>,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsSignal {
    HighConversion,
    AttentionNoSubmissions,
    LowVisibility,
    GrowingInterest,
    LowData,
    Normal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsSummary {
    pub views: i64,
    pub unique_visitors: i64,
    pub engaged_views: i64,
    pub cta_clicks: i64,
    pub submissions: i64,
    pub conversion_rate: f64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsDailyPoint {
    pub day: NaiveDate,
    pub views: i64,
    pub unique_visitors: i64,
    pub engaged_views: i64,
    pub cta_clicks: i64,
    pub submissions: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsSourcePoint {
    pub source: String,
    pub views: i64,
    pub unique_visitors: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsBreakdownPoint {
    pub key: String,
    pub views: i64,
    pub unique_visitors: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsFunnel {
    pub views: i64,
    pub engaged_views: i64,
    pub cta_clicks: i64,
    pub submissions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminFigurineAnalyticsListItem {
    pub figurine_id: String,
    pub name: String,
    pub status: FigurineStatus,
    pub face_url: Option<String>,
    pub signal: AnalyticsSignal,
    pub top_source: Option<String>,
    pub top_country: Option<String>,
    pub top_device: Option<String>,
    pub top_browser: Option<String>,
    pub views: i64,
    pub unique_visitors: i64,
    pub engaged_views: i64,
    pub cta_clicks: i64,
    pub submissions: i64,
    pub conversion_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminFigurineAnalyticsListPage {
    pub items: Vec<AdminFigurineAnalyticsListItem>,
    pub total: i64,
    pub summary: AnalyticsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminFigurineAnalyticsDetail {
    pub figurine: FigurineListItemDto,
    pub signal: AnalyticsSignal,
    pub summary: AnalyticsSummary,
    pub daily: Vec<AnalyticsDailyPoint>,
    pub sources: Vec<AnalyticsSourcePoint>,
    pub countries: Vec<AnalyticsBreakdownPoint>,
    pub devices: Vec<AnalyticsBreakdownPoint>,
    pub browsers: Vec<AnalyticsBreakdownPoint>,
    pub referrers: Vec<AnalyticsBreakdownPoint>,
    pub utm_sources: Vec<AnalyticsBreakdownPoint>,
    pub visitor_cohorts: Vec<AnalyticsBreakdownPoint>,
    pub funnel: AnalyticsFunnel,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAnalyticsQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub sort: Option<String>,
    pub dir: Option<String>,
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
    /// Optional slug override from the admin form. Empty/blank → the service
    /// auto-generates a unique transliterated slug from the name.
    #[serde(default)]
    pub slug: Option<String>,
    pub short_text: Option<String>,
    pub full_description: Option<String>,
    pub dimensions: Option<String>,
    pub material: Option<String>,
    pub technique: Option<String>,
    pub year: Option<i32>,
    pub passport_number: Option<String>,
    pub edition: Option<String>,
    pub created_period: Option<String>,
    pub care_instructions: Option<String>,
    pub provenance_note: Option<String>,
    pub authenticity_note: Option<String>,
    pub included_items: Option<String>,
    pub ambience_path: Option<String>,
    pub video_url: Option<String>,
    pub secret_text: Option<String>,
    pub status: FigurineStatus,
    pub sort_order: i32,
    pub is_visible: bool,
    pub is_featured: bool,
    /// Showing window (minutes from midnight); both NULL → always open.
    #[serde(default)]
    pub open_from_min: Option<i32>,
    #[serde(default)]
    pub open_until_min: Option<i32>,
    /// Optional sealed-door asset; null → procedural carved door.
    #[serde(default)]
    pub sealed_door_image: Option<String>,
    /// Showing room id (string UUID) this work belongs to; null → own window.
    #[serde(default)]
    pub showing_room_id: Option<String>,
    /// Which detail-page layout to use. null → 'specimen' (default).
    #[serde(default)]
    pub display_layout: Option<String>,
    /// JSON blob for per-figurine display customisation ({background, blockOrder}).
    #[serde(default)]
    pub display_config: Option<String>,
    /// "First look" early-release window as an RFC-3339 / ISO-8601 string (or
    /// null to clear). Parsed to a timestamptz on save.
    #[serde(default)]
    pub first_look_until: Option<String>,
    #[serde(default)]
    pub images: Vec<SaveImageRequest>,
    #[serde(default)]
    pub process_steps: Vec<SaveStepRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImageRequest {
    pub id: String,
    pub image_type: ImageType,
    pub url: String,
    pub original_url: Option<String>,
    pub thumb_url: Option<String>,
    pub depth_url: Option<String>,
    pub parallax_intensity: Option<f32>,
    pub focal_x: Option<f32>,
    pub focal_y: Option<f32>,
    pub reveal_radius: Option<f32>,
    pub darkness: Option<f32>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HomeContent {
    pub title: Option<String>,
    pub kicker: Option<String>,
    pub lead: Option<String>,
    pub hero_figurine_id: Option<String>,
    pub hero_caption_title: Option<String>,
    pub hero_caption_meta: Option<String>,
    pub hero_caption_cta: Option<String>,
    pub hero_mode: Option<String>,
    #[serde(default)]
    pub vitrine_figurine_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthorProfile {
    pub name: String,
    pub tagline: Option<String>,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    /// Portrait for the site-header avatar — distinct from `photo_url` (used
    /// by the bio/author page). `#[serde(default)]` so JSON blobs saved
    /// before this field existed still deserialize.
    #[serde(default)]
    pub hero_photo_url: Option<String>,
    pub instagram: Option<String>,
    pub telegram: Option<String>,
    pub vk: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub artstation: Option<String>,
    #[serde(default)]
    pub pinterest: Option<String>,
    #[serde(default)]
    pub youtube: Option<String>,
    /// Header avatar frame styling — all optional, admin-editable.
    #[serde(default)]
    pub avatar_shape: Option<String>,
    #[serde(default)]
    pub avatar_radius: Option<i32>,
    #[serde(default)]
    pub avatar_border_width: Option<f32>,
    #[serde(default)]
    pub avatar_border_color: Option<String>,
    #[serde(default)]
    pub avatar_bg: Option<String>,
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

fn default_order_mode() -> OrderMode {
    OrderMode::Request
}

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
    /// Set only for notify-mode orders — the visitor's receipt/cancel token.
    pub cancel_token: Option<String>,
    /// Set only for reserve-mode orders.
    pub reserve_status: Option<ReserveStatus>,
    pub reserve_expires_at: Option<chrono::NaiveDate>,
    pub admin_terms_note: Option<String>,
    pub invoice_note: Option<String>,
    pub certificate_token: Option<String>,
    pub certificate_number: Option<String>,
    pub certificate_issued_at: Option<DateTime<Utc>>,
    pub certificate_revoked_at: Option<DateTime<Utc>>,
}

/// Returned after submitting an order. Carries a token only for notify-mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderCreatedResponse {
    pub cancel_token: Option<String>,
}

/// Looked up by notify token so a visitor can see / stop their subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyInfo {
    pub figurine_id: String,
    pub figurine_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorCertificateDto {
    pub token: String,
    pub certificate_number: String,
    pub figurine_id: String,
    pub figurine_name: String,
    pub order_id: String,
    pub issued_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicCertificateDto {
    pub token: String,
    pub certificate_number: String,
    pub figurine_id: String,
    pub figurine_name: String,
    pub issued_at: String,
    pub revoked: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrderStatusRequest {
    pub status: OrderStatus,
    pub admin_notes: Option<String>,
    pub reserve_status: Option<ReserveStatus>,
    pub reserve_expires_at: Option<String>,
    pub admin_terms_note: Option<String>,
    pub invoice_note: Option<String>,
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

/// The 3 private "tones" a mark of attention can carry. `desired` is the
/// closest thing to purchase intent ("I'd commission something like this")
/// so it's weighted highest in the admin ranking — see the weighted-score
/// SQL in `Repository::get_favorite_tiers` / `get_admin_mark_stats`.
pub const MARK_TONES: [&str; 3] = ["touched", "mesmerized", "desired"];

/// Set (or clear) the visitor's wax-seal mark on a figurine. `visitor_token` is
/// a client-generated opaque id persisted in localStorage (not a login), used
/// purely for idempotency — it carries no PII and is never linked to an
/// account. `tone: None` clears the mark; `Some(t)` sets/switches it — the
/// client (which already tracks its own local state) decides the target
/// state explicitly rather than the server inferring a toggle, so a
/// double-submit or retry is naturally idempotent instead of flipping twice.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkToggleRequest {
    pub visitor_token: String,
    #[serde(default)]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkToggleResponse {
    pub marked: bool,
    pub tone: Option<String>,
}

/// Admin-only ranking row. Deliberately never exposed on the public site — see
/// migration comment on `figurine_marks` for why counts stay private.
#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminFigurineMarkStat {
    pub figurine_id: Uuid,
    pub figurine_name: String,
    pub status: FigurineStatus,
    pub is_visible: bool,
    pub mark_count: i64,
    pub touched_count: i64,
    pub mesmerized_count: i64,
    pub desired_count: i64,
    pub weighted_score: i64,
    pub last_marked_at: Option<DateTime<Utc>>,
}

/// "Noticed by guests" home-page shelf — a hybrid of admin curation and the
/// private mark ranking (research backs hybrid over pure-algorithmic or
/// pure-editorial for exactly this kind of "surface what resonates" shelf).
/// `pinned_ids` are shown first, in this order; `excluded_ids` are never
/// auto-filled from the ranking (but an admin can still pin an excluded
/// piece explicitly — exclusion only blocks the *automatic* fill).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NoticedByGuestsSettings {
    pub pinned_ids: Vec<Uuid>,
    pub excluded_ids: Vec<Uuid>,
}

/// Batch token lookup — clients poll several claim tokens at once.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookingsByTokensRequest {
    pub tokens: Vec<String>,
}

/// Full replacement of a logged-in user's wishlist (figurine ids).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWishlistRequest {
    pub figurine_ids: Vec<String>,
}

/// Attach a guest request (booking / waitlist / notify / commission) to the
/// logged-in account by its secret code, for visitors who changed device or
/// cleared their browser and lost the localStorage receipt.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkClaimRequest {
    pub token: String,
}

/// Outcome of a link-by-code attempt. `result` is one of:
/// "linked" | "email_mismatch" | "already_linked" | "not_found".
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkClaimResponse {
    pub result: String,
    /// "booking" | "waitlist" | "notify" | "commission" when a row was found.
    pub kind: Option<String>,
    /// Figurine / petition name, for a human-readable confirmation.
    pub name: Option<String>,
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
    /// Personal visual-password alphabet: { category: [icon_id; 8] }.
    /// NULL for legacy accounts registered before per-user pools existed.
    pub visual_pool: Option<serde_json::Value>,
    /// IP / geolocation the account registered from (best-effort, may be NULL).
    pub signup_ip: Option<String>,
    pub signup_country_code: Option<String>,
    pub signup_city: Option<String>,
    /// IP / geolocation the most recent password reset was applied from.
    pub last_reset_ip: Option<String>,
    pub last_reset_country_code: Option<String>,
    pub last_reset_city: Option<String>,
    pub last_reset_at: Option<DateTime<Utc>>,
    /// IP / geolocation the most recent self-service reset link was requested from.
    pub last_reset_request_ip: Option<String>,
    pub last_reset_request_country_code: Option<String>,
    pub last_reset_request_city: Option<String>,
    pub last_reset_request_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Originating request metadata captured for login attempts and sessions.
/// All fields best-effort: IP/UA may be absent behind some proxies, and geo is
/// only populated when an offline GeoIP database is configured.
#[derive(Debug, Clone, Default)]
pub struct ClientContext {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub country_code: Option<String>,
    pub city: Option<String>,
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
    /// icon_id per category in fixed order: animals, dishes, seasons, symbols
    pub selections: [String; 4],
    /// The personal subset shown to this user during registration:
    /// one entry per category (fixed order), each a list of icon_ids.
    /// Persisted so the same grid can be rebuilt at login.
    pub pool: [Vec<String>; 4],
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
    pub admin_notes: Option<String>,
    pub reserve_status: Option<ReserveStatus>,
    pub reserve_expires_at: Option<String>,
    pub admin_terms_note: Option<String>,
    pub invoice_note: Option<String>,
    pub certificate: Option<CollectorCertificateDto>,
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
    pub ip: Option<String>,
    pub country_code: Option<String>,
    pub city: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserDetail {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub admin_notes: Option<String>,
    pub created_at: String,
    pub signup_ip: Option<String>,
    pub signup_country_code: Option<String>,
    pub signup_city: Option<String>,
    pub last_reset_ip: Option<String>,
    pub last_reset_country_code: Option<String>,
    pub last_reset_city: Option<String>,
    pub last_reset_at: Option<String>,
    pub last_reset_request_ip: Option<String>,
    pub last_reset_request_country_code: Option<String>,
    pub last_reset_request_city: Option<String>,
    pub last_reset_request_at: Option<String>,
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
    /// icon_id per category in fixed order: animals, dishes, seasons, symbols
    pub selections: [String; 4],
    /// Fresh personal subset (one list per category) — reset regenerates the
    /// user's pool so the new selections are always replayable at login.
    pub pool: [Vec<String>; 4],
}

/// Self-service "forgot password": email a reset link to the account owner.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordRequest {
    pub email: String,
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

// ============================================================
// VISITOR IMPRESSIONS ("Book of Impressions")
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Impression {
    pub id: Uuid,
    pub message: String,
    pub author_name: Option<String>,
    pub mood: Option<String>,
    pub is_approved: bool,
    pub is_featured: bool,
    pub ip: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Public quote card — only what's safe to show a visitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpressionDto {
    pub id: String,
    pub message: String,
    pub author_name: Option<String>,
    pub mood: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminImpressionDto {
    pub id: String,
    pub message: String,
    pub author_name: Option<String>,
    pub mood: Option<String>,
    pub is_approved: bool,
    pub is_featured: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminImpressionsPage {
    pub items: Vec<AdminImpressionDto>,
    pub total: i64,
    pub pending_count: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitImpressionRequest {
    pub message: String,
    pub author_name: Option<String>,
    pub mood: Option<String>,
    /// Honeypot: real visitors never fill this. Any non-empty value drops the
    /// submission silently (handled in the service, not surfaced as an error).
    #[serde(default)]
    pub hp: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModerateImpressionRequest {
    pub is_approved: bool,
    pub is_featured: bool,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactSettings {
    pub email: Option<String>,
    pub telegram: Option<String>,
    pub phone: Option<String>,
}

/// Customisable Programme / Notice-Board section settings.
/// `max_showings = 0` means "show all". `case_bg = None` falls back to the
/// built-in Deep-Vellum gradient. Text fields fall back to i18n defaults when blank.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgrammeSettings {
    pub max_showings: u32,
    pub case_bg: Option<String>,
    pub curator_note_en: Option<String>,
    pub curator_note_ru: Option<String>,
    pub curator_sign_en: Option<String>,
    pub curator_sign_ru: Option<String>,
    /// Base bronze hex for the case's cast-metal molding (e.g. `#caa45f`).
    /// `None` falls back to the built-in bronze. Frontend derives the light→dark
    /// bevel gradient from this single colour.
    #[serde(default)]
    pub frame_tone: Option<String>,
    /// Molding thickness in px. `None` falls back to the built-in clamp.
    #[serde(default)]
    pub frame_thickness: Option<u32>,
    /// Molding render mode: `"gradient"` (beveled, default), `"flat"` (solid
    /// tone, no bevel) or `"none"` (no molding at all). `None` = gradient.
    #[serde(default)]
    pub frame_mode: Option<String>,
}

impl Default for ProgrammeSettings {
    fn default() -> Self {
        Self {
            max_showings: 0,
            case_bg: None,
            curator_note_en: None,
            curator_note_ru: None,
            curator_sign_en: None,
            curator_sign_ru: None,
            frame_tone: None,
            frame_thickness: None,
            frame_mode: None,
        }
    }
}

/// Customizable "Workshop" feature block on the home page.
/// All text fields are bilingual; `None`/blank values fall back to the i18n
/// defaults on the client. `visible = true` by default so the section shows
/// before any admin configuration exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopFeature {
    pub visible: bool,
    pub photo_back: Option<String>,
    pub photo_front: Option<String>,
    pub eyebrow_en: Option<String>,
    pub eyebrow_ru: Option<String>,
    pub title_en: Option<String>,
    pub title_ru: Option<String>,
    pub text_en: Option<String>,
    pub text_ru: Option<String>,
    pub link1_label_en: Option<String>,
    pub link1_label_ru: Option<String>,
    pub link1_href: Option<String>,
    pub link2_label_en: Option<String>,
    pub link2_label_ru: Option<String>,
    pub link2_href: Option<String>,
}

impl Default for WorkshopFeature {
    fn default() -> Self {
        Self {
            visible: true,
            photo_back: None,
            photo_front: None,
            eyebrow_en: None,
            eyebrow_ru: None,
            title_en: None,
            title_ru: None,
            text_en: None,
            text_ru: None,
            link1_label_en: None,
            link1_label_ru: None,
            link1_href: None,
            link2_label_en: None,
            link2_label_ru: None,
            link2_href: None,
        }
    }
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
        Self {
            min_days: 1,
            max_days: 30,
            advance_days: 0,
        }
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
    pub cancel_token: String,
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
    /// 1-based rank within this figurine's queue, by join time.
    pub position: i64,
}

/// Returned to the visitor right after joining the queue — their receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitlistCreatedResponse {
    pub cancel_token: String,
    pub position: i64,
}

/// Looked up by cancel token so a visitor can see / leave their place in line.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitlistCancelInfo {
    pub figurine_id: String,
    pub figurine_name: String,
    pub position: i64,
    pub created_at: String,
}

// ============================================================
// NEWSLETTER — the house "visitor book"
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Subscriber {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub source: String,
    pub lang: String,
    pub unsubscribe_token: String,
    pub ip: Option<String>,
    pub created_at: DateTime<Utc>,
    pub unsubscribed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionRequest {
    pub email: String,
    pub name: Option<String>,
    pub source: Option<String>,
    pub lang: Option<String>,
}

/// Returned to the visitor right after signing the book — their receipt and
/// the unguessable token that backs the unsubscribe link.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionCreatedResponse {
    pub unsubscribe_token: String,
    /// True when this email was already an active subscriber (idempotent re-sign).
    pub already_subscribed: bool,
}

/// Looked up by unsubscribe token so a visitor can confirm leaving the book.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriberInfo {
    pub email: String,
}

/// Admin view of one active subscriber.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriberDto {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub source: String,
    pub lang: String,
    pub created_at: String,
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
    #[serde(default)]
    pub attachments: Vec<AttachmentDto>,
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
            attachments: Vec::new(),
        }
    }
}

impl ThreadMessageDto {
    pub fn from_with_attachments(m: &ThreadMessage, attachments: Vec<AttachmentDto>) -> Self {
        let mut dto = ThreadMessageDto::from(m);
        dto.attachments = attachments;
        dto
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
    #[serde(default)]
    pub attachment_urls: Vec<AttachmentInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyToThreadRequest {
    pub body: String,
    #[serde(default)]
    pub attachment_urls: Vec<AttachmentInput>,
}

// ============================================================
// COMMISSIONS — petition to the master to create a NEW figurine
// ============================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "commission_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CommissionStatus {
    New,
    Reviewing,
    Accepted,
    InProgress,
    Completed,
    Declined,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Commission {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub claim_token: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub title: String,
    pub description: String,
    pub size_note: Option<String>,
    pub mood: Option<String>,
    pub deadline: Option<chrono::NaiveDate>,
    pub budget_note: Option<String>,
    pub occasion: Option<String>,
    pub source_figurine_id: Option<String>,
    pub similar_keep_note: Option<String>,
    pub similar_change_note: Option<String>,
    pub similar_tags: Vec<String>,
    pub figurine_id: Option<String>,
    pub status: CommissionStatus,
    pub admin_notes: Option<String>,
    pub lang: String,
    pub certificate_token: Option<String>,
    pub certificate_number: Option<String>,
    pub certificate_issued_at: Option<DateTime<Utc>>,
    pub certificate_revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CommissionStatus {
    /// Work has begun — the petition may no longer be deleted or edited.
    pub fn is_started(&self) -> bool {
        matches!(
            self,
            CommissionStatus::Accepted | CommissionStatus::InProgress | CommissionStatus::Completed
        )
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Attachment {
    pub id: Uuid,
    pub url: String,
    pub thumb_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDto {
    pub id: String,
    pub url: String,
    pub thumb_url: Option<String>,
}

impl From<&Attachment> for AttachmentDto {
    fn from(a: &Attachment) -> Self {
        AttachmentDto {
            id: a.id.to_string(),
            url: a.url.clone(),
            thumb_url: a.thumb_url.clone(),
        }
    }
}

/// One uploaded reference, as echoed back by the client after an upload.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInput {
    pub url: String,
    pub thumb_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommissionRequest {
    pub requester_name: Option<String>,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub title: Option<String>,
    pub description: String,
    pub size_note: Option<String>,
    pub mood: Option<String>,
    pub deadline: Option<String>,
    pub budget_note: Option<String>,
    pub occasion: Option<String>,
    pub source_figurine_id: Option<String>,
    pub similar_keep_note: Option<String>,
    pub similar_change_note: Option<String>,
    #[serde(default)]
    pub similar_tags: Vec<String>,
    #[serde(default)]
    pub attachment_urls: Vec<AttachmentInput>,
    /// Honeypot — real users never fill this. Non-empty ⇒ silently dropped.
    #[serde(default)]
    pub website: Option<String>,
    /// UI language at submission time ('ru' | 'en'), for later system messages.
    pub lang: Option<String>,
}

/// Edit of a petition's content (by its owner or the master) — only while work
/// has not yet started.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditCommissionRequest {
    pub title: Option<String>,
    pub description: String,
    pub size_note: Option<String>,
    pub mood: Option<String>,
    pub deadline: Option<String>,
    pub budget_note: Option<String>,
    pub occasion: Option<String>,
    /// Full replacement set of reference images. When omitted, attachments are
    /// left untouched.
    pub attachment_urls: Option<Vec<AttachmentInput>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommissionDto {
    pub id: String,
    pub claim_token: String,
    pub requester_name: String,
    pub requester_email: String,
    pub requester_phone: Option<String>,
    pub title: String,
    pub description: String,
    pub size_note: Option<String>,
    pub mood: Option<String>,
    pub deadline: Option<String>,
    pub budget_note: Option<String>,
    pub occasion: Option<String>,
    pub source_figurine_id: Option<String>,
    pub similar_keep_note: Option<String>,
    pub similar_change_note: Option<String>,
    pub similar_tags: Vec<String>,
    pub figurine_id: Option<String>,
    pub status: CommissionStatus,
    pub admin_notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub attachments: Vec<AttachmentDto>,
    pub thread_id: Option<String>,
    /// Whether work has begun (petition can no longer be deleted or edited).
    pub started: bool,
    /// Certificate of authenticity, present once issued for a completed commission.
    pub certificate: Option<CollectorCertificateDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommissionCreatedResponse {
    pub id: String,
    pub claim_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommissionsPage {
    pub items: Vec<CommissionDto>,
    pub total: i64,
    pub new_count: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCommissionStatusRequest {
    pub status: CommissionStatus,
    pub admin_notes: Option<String>,
    pub figurine_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimCommissionRequest {
    pub claim_token: String,
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

// ============================================================
// THEME CONFIG
// ============================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeConfig {
    /// Flat map: CSS variable suffix → value. E.g. "ink-primary" → "#2C1710"
    #[serde(default)]
    pub colors: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub fonts: ThemeFonts,
    #[serde(default)]
    pub motion: ThemeMotion,
    #[serde(default)]
    pub effects: ThemeEffects,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeEffects {
    /// Global "keyhole" darkness (0..1); None falls back to the renderer default.
    pub keyhole_darkness: Option<f32>,
    /// Seconds of hover before a sealed card self-reveals; None/0 disables it.
    pub keyhole_dwell_reveal: Option<f32>,
    /// Background circle colour behind the raven emblem in the header.
    pub bird_circle_color: Option<String>,
    /// Seconds between walking-bird cameos across the header.
    pub bird_walk_interval: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFonts {
    pub display: Option<String>,
    pub body: Option<String>,
    pub serif: Option<String>,
    pub mono: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeMotion {
    pub duration_fast: Option<String>,
    pub duration_default: Option<String>,
    pub duration_slow: Option<String>,
    pub duration_glacial: Option<String>,
}

// ============================================================
// HOME LAYOUT CONFIG
// ============================================================

/// Per-block style override on the home page (mirrors the frontend's BlockStyle).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeBlockStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// 'sm' | 'base' | 'lg' | 'xl'
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<String>,
    /// Font ID from the frontend's READING_FONTS catalogue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// 'tight' | 'base' | 'roomy' | 'spacious'
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_y: Option<String>,
    /// Letterpress rule above the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divider: Option<bool>,
    /// Device classes the block is hidden on: 'mobile' | 'tablet' | 'desktop'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_on: Option<Vec<String>>,
}

/// Override of one element inside a home block (mirrors HomeElementStyle).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeElementStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Free-range font size in px for text elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_px: Option<f32>,
    /// Free-range width in % of the parent for media elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_pct: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

/// Admin-arranged layout of the home page: block order per zone, visibility,
/// width presets and per-block styles. Absent fields mean "hard-coded default".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeLayoutConfig {
    /// Main flow order (hero, returningBand, gallery, … latelyShelves).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_order: Option<Vec<String>>,
    /// Order inside the returning-visitor band (visitLedger, noticeBoard).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub band_order: Option<Vec<String>>,
    /// Order inside the returning-visitor shelves (firstLook, markedByYou, noticedByGuests).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shelf_order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_blocks: Option<Vec<String>>,
    /// Block ID → 'full' | 'contained' | 'compact'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<std::collections::HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_styles: Option<std::collections::HashMap<String, HomeBlockStyle>>,
    /// Per-element overrides, keyed "blockId.elementId".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elements: Option<std::collections::HashMap<String, HomeElementStyle>>,
    /// Per-block order of orderable elements inside the block's column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_order: Option<std::collections::HashMap<String, Vec<String>>>,
    /// Background of the whole home page (hex); overrides the parchment default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_background: Option<String>,
    /// THE COLLECTION gallery card scroll-reveal treatment (e.g. "rise", "fog").
    /// Absent/unknown → the "rise" default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_effect: Option<String>,
}

/// Named, admin-saved home layout arrangement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeLayoutPreset {
    pub id: String,
    pub name: String,
    /// Opaque JSON blob — the frontend's HomeLayoutConfig object.
    pub config: serde_json::Value,
    pub saved_at: String,
}

// ============================================================
// REEL THEME — appearance of the home reel (room photo + glass panes)
// ============================================================

/// One stop of an admin-built gradient overlay.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradientStop {
    /// Hex colour, e.g. "#140b07".
    pub color: String,
    /// 0..100, position along the gradient.
    pub position: f32,
    /// 0..1.
    pub opacity: f32,
}

/// One pane's look — glass, type, buttons. The hero pane and the work panes
/// carry the same fields and are set independently.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardStyle {
    /// 'solid' | 'gradient' — what the pane is filled with. Absent on themes
    /// saved before the pane could carry a gradient of its own; the frontend
    /// reads that as 'solid', which is exactly how those themes used to render.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_kind: Option<String>,
    /// 'linear' | 'radial' | 'conic'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_angle: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_stops: Option<Vec<GradientStop>>,
    /// The pane's hairline border, its alpha, and what it becomes under the pointer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_opacity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_hover_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_tint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_opacity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_blur: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_saturation: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_radius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_sheen: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_shadow: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_color: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_size: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_size: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_size: Option<f32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub btn_fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub btn_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub btn_radius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub btn_size: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub btn_border: Option<String>,
}

/// Everything the admin can tune about the home reel's look. Every field is
/// optional and the FRONTEND owns the defaults (see `reel-theme.svelte.ts`) —
/// the server is storage, not a second source of truth for the design.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReelTheme {
    /// The opening pane, styled on its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hero: Option<CardStyle>,
    /// Every work pane, and the closing archive pane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work: Option<CardStyle>,
    // ── Backdrop ─────────────────────────────────────────────
    /// 'image' | 'color' | 'gradient' — what sits behind the panes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backdrop_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,
    /// Separate image for narrow screens; a landscape room crops to mush on a phone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image_mobile: Option<String>,
    /// 'cover' | 'contain'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_fit: Option<String>,
    /// CSS object-position, e.g. "center top" or "50% 20%".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_position: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_blur: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_brightness: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_saturation: Option<f32>,
    /// Flat colour used when backdrop_kind is 'color' (and as the letterbox fill).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backdrop_color: Option<String>,
    /// Colour the panes' shadows are cast in. Its own field on purpose: deriving
    /// it from backdrop_color meant that picking a background colour silently
    /// recoloured every shadow on the page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_color: Option<String>,

    // ── Overlay (the dimming veil) ───────────────────────────
    /// 'none' | 'solid' | 'gradient'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_opacity: Option<f32>,
    /// 'linear' | 'radial' | 'conic'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_type: Option<String>,
    /// Degrees, for linear/conic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_angle: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_stops: Option<Vec<GradientStop>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vignette: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grain: Option<f32>,

    // ── Glass panes ──────────────────────────────────────────
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_tint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_opacity: Option<f32>,
    /// backdrop-filter blur, in px. 0 disables the frosting entirely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_blur: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_saturation: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_radius: Option<f32>,
    /// Strength of the lit edge + specular streak, 0..1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_sheen: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass_shadow: Option<f32>,
    /// Drops backdrop-filter everywhere — the escape hatch for weak machines,
    /// where frosting two dozen panes is what makes the reel stutter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance_mode: Option<bool>,

    // ── Type & buttons ───────────────────────────────────────
    /// 'light' | 'dark' — ink on the panes. A light backdrop with light type is
    /// the one setting here that can render the page unreadable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_tone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button_fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button_text: Option<String>,

    // ── Reel density ─────────────────────────────────────────
    /// Vertical gap between panes, in rem.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_gap: Option<f32>,
    /// Max width of a work pane, in rem.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_width: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReelThemePreset {
    pub id: String,
    pub name: String,
    /// Opaque JSON blob — the frontend's ReelTheme object.
    pub config: serde_json::Value,
    pub saved_at: String,
}

// ============================================================
// DISPLAY CONFIG PRESETS
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayConfigPreset {
    pub id: String,
    pub name: String,
    /// Opaque JSON blob — the frontend's DisplayConfig object.
    pub config: serde_json::Value,
    pub saved_at: String,
}

// ============================================================
// COPY OVERRIDES
// ============================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CopyOverrides {
    #[serde(default)]
    pub en: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub ru: std::collections::HashMap<String, String>,
}
