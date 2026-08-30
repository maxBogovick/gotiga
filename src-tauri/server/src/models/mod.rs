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
    /// Editorial grouping (e.g. a named collection) — the archive page's filter axis.
    pub series: Option<String>,
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
    /// Backstage, search-only AI visual caption (see the migration). Folded into
    /// the semantic-search embedding text; never serialised to the public DTO.
    pub visual_caption: Option<String>,
    /// Admin-only Pinterest SEO copy for feed.xml — never serialised to any
    /// public/admin figurine DTO; read only by feed_rss (see the migration).
    pub pinterest_description: Option<String>,
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
    /// JSON blob for the Specimen catalog leaf lists (Features / Perfect for).
    /// NULL → every built-in line is on, no custom lines.
    pub catalog_lists: Option<String>,
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
    /// Free-text size (e.g. "20×15×10 cm"). Already on the TypeScript list type
    /// and the home reel caption; omitting it here meant the client never saw it.
    #[serde(default)]
    pub dimensions: Option<String>,
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

/// One ranked result of a hybrid search ("Хранитель"). The client already
/// holds the archive, so we return only the id + fused rank and let it reorder.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticHit {
    pub id: String,
    /// Reciprocal Rank Fusion score (higher is closer). Not cosine.
    pub score: f32,
}

/// Result of re-indexing figurine embeddings for semantic search.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedIndexSummary {
    pub total: usize,
    /// Newly (re)embedded because their text or the model changed.
    pub indexed: usize,
    /// Unchanged since last index — skipped.
    pub skipped: usize,
    pub failed: usize,
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
    /// Editorial grouping (e.g. a named collection) — the archive page's filter axis.
    #[serde(default)]
    pub series: Option<String>,
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
    /// JSON blob for the Specimen catalog leaf lists (Features / Perfect for).
    /// Null → every built-in line is on, no custom lines.
    #[serde(default)]
    pub catalog_lists: Option<String>,
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
    /// Site-wide page view (home/archive/author/workshop/commission) — allowed
    /// a `None` figurine_id, like `PageEngaged`.
    PageView,
    /// Site-wide engagement (time on page + scroll depth + works_seen), the
    /// non-figurine sibling of `FigurineEngaged`. Also allowed a `None`
    /// figurine_id.
    PageEngaged,
}

impl AnalyticsEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FigurineView => "figurine_view",
            Self::FigurineEngaged => "figurine_engaged",
            Self::FigurineCtaClick => "figurine_cta_click",
            Self::PageView => "page_view",
            Self::PageEngaged => "page_engaged",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsEventRequest {
    pub event_type: AnalyticsEventType,
    /// Required for figurine_view/figurine_engaged/figurine_cta_click; absent
    /// (or blank) for site-wide `page_view` events.
    pub figurine_id: Option<String>,
    pub path: String,
    pub referrer: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub duration_ms: Option<i32>,
    pub scroll_depth: Option<i32>,
    /// Distinct work tiles seen during a home/archive visit (`page_engaged`
    /// only); absent for gridless pages and all other event types.
    pub works_seen: Option<i32>,
    pub cta_type: Option<String>,
    pub page_view_id: Option<String>,
    pub client_ts: Option<DateTime<Utc>>,
    /// Visitor's UI language at the time of the event ('en' | 'ru'), from the
    /// i18n store.
    pub lang: Option<String>,
    /// Which on-site block a figurine-card click came from (e.g.
    /// "home_afisha"), read from a `?src=` link param — separate from
    /// utm_source, which is for external campaigns.
    pub internal_source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnalyticsEventRecord {
    pub occurred_at: DateTime<Utc>,
    pub event_date: NaiveDate,
    pub event_type: &'static str,
    pub figurine_id: Option<Uuid>,
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
    pub works_seen: Option<i32>,
    pub cta_type: Option<String>,
    pub user_id: Option<Uuid>,
    pub lang: Option<String>,
    pub internal_source: Option<String>,
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

/// One (day, country) cell from the permanent `figurine_analytics_geo_daily`
/// rollup — the geography map's "one figurine" mode groups these by country
/// for the choropleth, then filters by country to list the actual dates a
/// visit from there was recorded.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineGeoDailyPoint {
    pub day: NaiveDate,
    pub country_code: String,
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

/// One row of the "starts -> submitted" CTA funnel for a single call-to-action
/// family (request/reserve/booking/waitlist/commission). `starts` comes from the
/// pre-aggregated daily table (client-side CTA clicks); `submitted` is counted
/// straight from the real orders/bookings/waitlist/commissions tables, so the
/// two sides can legitimately disagree (see `starts_are_client_side` on the
/// funnel container) — starts undercount (DNT/bots/direct form links skip the
/// client event) while submitted is authoritative.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CtaFunnelStep {
    pub cta_type: String,
    pub starts: i64,
    pub submitted: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminFigurineAnalyticsListItem {
    pub figurine_id: String,
    pub name: String,
    pub status: FigurineStatus,
    /// Editorial grouping (e.g. a named collection) — the same field the
    /// archive page filters by; surfaced here so a series can be sized up as
    /// a unit, not just piece by piece.
    pub series: Option<String>,
    pub face_url: Option<String>,
    pub signal: AnalyticsSignal,
    /// Week-over-week growth on its own, independent of `signal` — the
    /// signal is a single priority-ordered pick (see `AppService::
    /// analytics_signal`), so a work that's both growing *and*, say,
    /// attention-worthy for having no submissions would only ever show the
    /// higher-priority badge. This field keeps "is it growing" visible
    /// regardless of which signal won.
    pub is_growing: bool,
    pub top_source: Option<String>,
    pub top_country: Option<String>,
    pub top_device: Option<String>,
    pub top_browser: Option<String>,
    /// Every country (ISO 3166-1 alpha-2) with at least one view in range —
    /// not just the top one — from the permanent geo rollup. Drives the
    /// Works table's country filter and the geography map's
    /// country → figurines drilldown, both client-side (no extra request).
    pub countries: Vec<String>,
    pub views: i64,
    pub unique_visitors: i64,
    pub engaged_views: i64,
    pub cta_clicks: i64,
    pub submissions: i64,
    pub conversion_rate: f64,
    /// Daily view counts for the last 14 days ending at the query's `to` date,
    /// zero-filled for missing days — independent of the selected range length
    /// so row sparklines stay a fixed, comparable shape.
    pub sparkline: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminFigurineAnalyticsListPage {
    pub items: Vec<AdminFigurineAnalyticsListItem>,
    pub total: i64,
    pub summary: AnalyticsSummary,
    /// Same-length, immediately-preceding period used for delta comparisons.
    pub previous_summary: AnalyticsSummary,
    pub previous_from: NaiveDate,
    pub previous_to: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminFigurineAnalyticsDetail {
    pub figurine: FigurineListItemDto,
    pub signal: AnalyticsSignal,
    pub summary: AnalyticsSummary,
    pub previous_summary: AnalyticsSummary,
    pub previous_from: NaiveDate,
    pub previous_to: NaiveDate,
    pub daily: Vec<AnalyticsDailyPoint>,
    pub sources: Vec<AnalyticsSourcePoint>,
    pub countries: Vec<AnalyticsBreakdownPoint>,
    pub devices: Vec<AnalyticsBreakdownPoint>,
    pub browsers: Vec<AnalyticsBreakdownPoint>,
    pub referrers: Vec<AnalyticsBreakdownPoint>,
    pub utm_sources: Vec<AnalyticsBreakdownPoint>,
    pub visitor_cohorts: Vec<AnalyticsBreakdownPoint>,
    pub languages: Vec<AnalyticsBreakdownPoint>,
    pub internal_sources: Vec<AnalyticsBreakdownPoint>,
    pub funnel: AnalyticsFunnel,
    pub cta_funnel: Vec<CtaFunnelStep>,
    /// Median milliseconds spent engaged with the card. `None` when there are no
    /// qualifying `figurine_engaged` events in range (not 0 — see medians never
    /// counting NULL/absent samples as zero).
    pub median_duration_ms: Option<f64>,
    /// Median scroll depth (0-100) at engagement. `None` under the same rule.
    pub median_scroll_depth: Option<f64>,
    /// Earliest date for which raw-event-derived fields above (medians,
    /// countries/devices/browsers/referrers/utmSources breakdowns) actually have
    /// data — raw events are pruned after a retention window, so this can be
    /// later than `from` when the selected range reaches further back than that.
    pub raw_data_from: NaiveDate,
}

/// Per-page engagement for the generic (non-figurine) pages, keyed by the same
/// coarse `path_group` as `site_page_views_daily` (home/archive/author/workshop/
/// commission).
///
/// `views`/`unique_visitors` come from the permanent `site_page_views_daily`
/// rollup and cover the full range. Everything derived from raw `page_engaged`
/// events (`engaged_events`, `quick_exit_events`, `reached_works_events` and the
/// medians) only covers the retention window — `raw_data_from` on the response
/// marks how far back that reaches. `median_works_seen`/`reached_works_events`
/// are meaningful only for the grid pages (home/archive); gridless pages never
/// report `works_seen`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SitePageEngagement {
    pub path_group: String,
    pub views: i64,
    pub unique_visitors: i64,
    pub engaged_events: i64,
    /// Engaged visits shorter than `QUICK_EXIT_MS` — the "bounced almost
    /// immediately" count. Divided by `engaged_events` gives the quick-exit rate.
    pub quick_exit_events: i64,
    /// Engaged grid-page visits that saw at least one work tile — divided by
    /// `engaged_events` gives the "reached the collection" rate.
    pub reached_works_events: i64,
    pub median_duration_ms: Option<f64>,
    pub median_scroll_depth: Option<f64>,
    pub median_works_seen: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SitePageEngagementResponse {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// The immediately-preceding, equal-length period — the baseline the panel
    /// draws its deltas against.
    pub previous_from: NaiveDate,
    pub previous_to: NaiveDate,
    /// Earliest day the engagement figures actually reach (retention floor), so
    /// the panel can label the coverage instead of implying the full range has
    /// data.
    pub raw_data_from: NaiveDate,
    pub pages: Vec<SitePageEngagement>,
    /// Same shape as `pages`, for `previous_from..previous_to`.
    pub previous_pages: Vec<SitePageEngagement>,
}

/// One anonymous visitor's activity summary for a single day. "Visitor" is the
/// daily-rotating `visitor_hash` (HMAC of IP-prefix + client hints + date, no
/// raw IP stored) — so this is pseudonymous and cannot be tied to a real person
/// or followed across days, by design. Derived entirely from raw events, so it
/// only covers the retention window.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminVisitorSession {
    pub visitor_hash: String,
    pub day: NaiveDate,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub event_count: i64,
    pub page_views: i64,
    pub figurine_views: i64,
    pub cta_clicks: i64,
    /// Distinct action buttons pressed during the visit (reserve/booking/
    /// waitlist/notify/comment/…) — the visit's "trace". Empty when the visitor
    /// only browsed. This is the intent signal the analytics pipeline can
    /// attribute to an anonymous visit; the actual form submissions live in
    /// their own tables and aren't linked to a `visitor_hash`.
    pub cta_types: Vec<String>,
    /// Most works seen on a single grid page (home/archive) during the visit.
    pub max_works_seen: Option<i32>,
    pub max_scroll_depth: Option<i32>,
    pub country_code: Option<String>,
    pub device_class: Option<String>,
    pub browser_family: Option<String>,
    pub lang: Option<String>,
    /// The visit's entry source (source of the earliest event).
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminVisitorSessionsPage {
    pub sessions: Vec<AdminVisitorSession>,
    pub total: i64,
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// Earliest day sessions actually reach (raw-event retention floor).
    pub raw_data_from: NaiveDate,
}

/// One event on an anonymous visitor's timeline.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminVisitorEvent {
    pub occurred_at: DateTime<Utc>,
    pub event_type: String,
    pub path: String,
    pub figurine_id: Option<Uuid>,
    /// Resolved name for a `figurine_view`/`figurine_engaged` event, so the
    /// timeline reads "opened «Hound»" rather than a bare id.
    pub figurine_name: Option<String>,
    pub duration_ms: Option<i32>,
    pub scroll_depth: Option<i32>,
    pub works_seen: Option<i32>,
    pub cta_type: Option<String>,
    pub source: Option<String>,
    pub internal_source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminVisitorsQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// When true, keep only visits that pressed at least one action button —
    /// the "left a trace" filter for finding hot sessions among idle browsing.
    pub only_actions: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAnalyticsQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub sort: Option<String>,
    pub dir: Option<String>,
}

/// One-off admin operation: re-run figurine_analytics_daily /
/// site_page_views_daily aggregation over a historical range (e.g. after a
/// bugfix to the aggregation query itself — the hot-window job only ever
/// recomputes yesterday+today). Idempotent (delete+reinsert per range).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillAnalyticsRequest {
    /// Defaults to the earliest day already present in figurine_analytics_daily.
    pub from: Option<NaiveDate>,
    /// Defaults to today.
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackfillAnalyticsResponse {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

/// Site-wide traffic overview (all figurines combined) built from the same
/// pre-aggregated daily table — the "pulse of the house" screen (J1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAnalyticsOverview {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub previous_from: NaiveDate,
    pub previous_to: NaiveDate,
    pub summary: AnalyticsSummary,
    pub previous_summary: AnalyticsSummary,
    pub daily: Vec<AnalyticsDailyPoint>,
    pub sources: Vec<AnalyticsSourcePoint>,
    /// Site-wide views by country (every page), for the geography map.
    pub geo: Vec<AnalyticsBreakdownPoint>,
}

/// Site → works → /commission → started form → submitted. Every step but the
/// last is a distinct-visitor count from raw events, so it's bound by
/// analytics::RETENTION_DAYS (see raw_data_from) and undercounts under
/// DNT/bots/direct links; `submitted` is exact (the real `commissions` table).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommissionFunnel {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub raw_data_from: NaiveDate,
    pub visited: i64,
    pub viewed_works: i64,
    pub opened_commission_page: i64,
    pub started_form: i64,
    pub submitted: i64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsAnnotation {
    pub id: Uuid,
    pub day: NaiveDate,
    pub label: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnnotationRequest {
    pub day: NaiveDate,
    pub label: String,
}

/// Daily counts of the site's quieter engagement signals — attention marks,
/// "Book of the House" newsletter signups, comments — none of which are
/// retention-pruned (full history), unlike the raw analytics events.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifeOfHouseDailyPoint {
    pub day: NaiveDate,
    pub marks: i64,
    pub subscribers: i64,
    pub comments: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifeOfHouseTrend {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub daily: Vec<LifeOfHouseDailyPoint>,
    pub marks_total: i64,
    pub subscribers_total: i64,
    pub comments_total: i64,
    pub previous_marks_total: i64,
    pub previous_subscribers_total: i64,
    pub previous_comments_total: i64,
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
    /// Editorial grouping (e.g. a named collection) — the archive page's filter axis.
    #[serde(default)]
    pub series: Option<String>,
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
    /// JSON blob for the Specimen catalog leaf lists (Features / Perfect for).
    /// Null → every built-in line is on, no custom lines.
    #[serde(default)]
    pub catalog_lists: Option<String>,
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
    /// Self-certified "I am 16 or older" checkbox — required on every public
    /// form that collects contact details (see privacy policy "Children" section).
    pub age_confirmed: bool,
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

/// Heart like from a visitor. `liked` is the target state (explicit-set, not a
/// server-side flip) so a doubled request cannot unlike then like again.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikeToggleRequest {
    pub visitor_token: String,
    pub liked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LikeToggleResponse {
    pub liked: bool,
    pub like_count: i64,
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
    pub like_count: i64,
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
    /// Self-certified "I am 16 or older" checkbox — required on every public
    /// form that collects contact details (see privacy policy "Children" section).
    pub age_confirmed: bool,
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
    /// Self-certified "I am 16 or older" checkbox — required on every public
    /// form that collects contact details (see privacy policy "Children" section).
    pub age_confirmed: bool,
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
    /// Self-certified "I am 16 or older" checkbox — required on every public
    /// form that collects contact details (see privacy policy "Children" section).
    pub age_confirmed: bool,
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
// CONTACT MESSAGES — lightweight "write to the author" letters
// ============================================================

/// Anonymous, not tied to a figurine (unlike `Order`) or a logged-in
/// account (unlike `MessageThread`) — a stranger reading the home page can
/// send one without committing to a full commission or creating an account.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ContactMessage {
    pub id: Uuid,
    pub email: String,
    pub message: String,
    pub source: String,
    pub lang: String,
    pub ip: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContactMessageRequest {
    pub email: String,
    pub message: String,
    pub source: Option<String>,
    pub lang: Option<String>,
}

/// Admin view of one letter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactMessageDto {
    pub id: String,
    pub email: String,
    pub message: String,
    pub source: String,
    pub lang: String,
    pub is_read: bool,
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
    /// Self-certified "I am 16 or older" checkbox — required on every public
    /// form that collects contact details (see privacy policy "Children" section).
    pub age_confirmed: bool,
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
    /// Self-certified "I am 16 or older" checkbox — required on every public
    /// form that collects contact details (see privacy policy "Children" section).
    pub age_confirmed: bool,
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

// ============================================================
// CABINET GAZETTE
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GazetteLeaf {
    pub id: Uuid,
    pub slug: String,
    pub kind: String,
    pub status: String,
    pub title_en: String,
    pub title_ru: String,
    pub dek_en: Option<String>,
    pub dek_ru: Option<String>,
    pub body_en: Option<String>,
    pub body_ru: Option<String>,
    pub figurine_id: Option<Uuid>,
    pub href: Option<String>,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub image_urls: Vec<String>,
    pub pinned: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub expected_from: Option<NaiveDate>,
    pub expected_to: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GazetteLeafListed {
    pub id: Uuid,
    pub slug: String,
    pub kind: String,
    pub status: String,
    pub title_en: String,
    pub title_ru: String,
    pub dek_en: Option<String>,
    pub dek_ru: Option<String>,
    pub body_en: Option<String>,
    pub body_ru: Option<String>,
    pub figurine_id: Option<Uuid>,
    pub href: Option<String>,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub image_urls: Vec<String>,
    pub pinned: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub expected_from: Option<NaiveDate>,
    pub expected_to: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub figurine_name: Option<String>,
    pub figurine_slug: Option<String>,
    pub figurine_status: Option<String>,
    pub watch_count: i64,
    pub shelf_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteLeafDto {
    pub id: String,
    pub slug: String,
    pub kind: String,
    pub status: String,
    pub title_en: String,
    pub title_ru: String,
    pub dek_en: Option<String>,
    pub dek_ru: Option<String>,
    pub body_en: Option<String>,
    pub body_ru: Option<String>,
    pub figurine_id: Option<String>,
    pub figurine_name: Option<String>,
    pub figurine_slug: Option<String>,
    pub href: Option<String>,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    pub image_urls: Vec<String>,
    pub pinned: bool,
    pub published_at: Option<String>,
    pub scheduled_at: Option<String>,
    pub expected_from: Option<String>,
    pub expected_to: Option<String>,
    pub figurine_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch_count: Option<i64>,
    /// Place on the shelf of tall tales. Absent for every other kind of leaf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shelf_order: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<GazetteNeighborDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<GazetteNeighborDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteNeighborDto {
    pub slug: String,
    pub title_en: String,
    pub title_ru: String,
}

/// The shelf of tall tales, top to bottom. Position is the index in the list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderTalesRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGazetteLeafRequest {
    pub slug: Option<String>,
    pub kind: String,
    pub status: String,
    pub title_en: String,
    pub title_ru: String,
    pub dek_en: Option<String>,
    pub dek_ru: Option<String>,
    pub body_en: Option<String>,
    pub body_ru: Option<String>,
    pub figurine_id: Option<String>,
    pub href: Option<String>,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub image_url: Option<String>,
    #[serde(default)]
    pub image_urls: Vec<String>,
    #[serde(default)]
    pub pinned: bool,
    pub scheduled_at: Option<String>,
    #[serde(default)]
    pub expected_from: Option<String>,
    #[serde(default)]
    pub expected_to: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteLeavesPage {
    pub items: Vec<GazetteLeafDto>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GazetteWatch {
    pub id: Uuid,
    pub leaf_id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub lang: String,
    pub cancel_token: String,
    pub user_id: Option<Uuid>,
    pub notified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchGazetteLeafRequest {
    pub email: Option<String>,
    pub name: Option<String>,
    pub lang: Option<String>,
    pub age_confirmed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteWatchCreatedResponse {
    pub cancel_token: String,
    pub already_watching: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteWatchInfo {
    pub leaf_slug: String,
    pub title_en: String,
    pub title_ru: String,
    pub notified: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GazetteWatchListed {
    pub id: Uuid,
    pub leaf_id: Uuid,
    pub leaf_slug: String,
    pub title_en: String,
    pub title_ru: String,
    pub cancel_token: String,
    pub notified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteWatchDto {
    pub id: String,
    pub leaf_id: String,
    pub leaf_slug: String,
    pub title_en: String,
    pub title_ru: String,
    pub cancel_token: String,
    pub notified_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteFeed {
    pub id: Uuid,
    pub title: String,
    pub url: String,
    pub enabled: bool,
    pub last_fetched_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub mark_key: String,
    pub mark_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteFeedDto {
    pub id: String,
    pub title: String,
    pub url: String,
    pub enabled: bool,
    pub last_fetched_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub mark_key: String,
    pub mark_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGazetteFeedRequest {
    pub title: String,
    pub url: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub mark_key: Option<String>,
    pub mark_url: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GazetteCutting {
    pub id: Uuid,
    pub feed_id: Uuid,
    pub guid: String,
    pub title: String,
    pub url: String,
    pub summary: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub dismissed: bool,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GazetteCuttingListed {
    pub id: Uuid,
    pub feed_id: Uuid,
    pub guid: String,
    pub title: String,
    pub url: String,
    pub summary: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub dismissed: bool,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub source_name: String,
    pub mark_key: String,
    pub mark_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteCuttingDto {
    pub id: String,
    pub feed_id: String,
    pub title: String,
    pub url: String,
    pub summary: Option<String>,
    pub source_name: String,
    pub published_at: Option<String>,
    pub dismissed: bool,
    pub pinned: bool,
    pub created_at: String,
    pub mark_key: String,
    pub mark_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteHomeDto {
    pub leaves: Vec<GazetteLeafDto>,
    pub cuttings: Vec<GazetteCuttingDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteRoomDto {
    pub year: i32,
    pub years: Vec<i32>,
    pub leaves: Vec<GazetteLeafDto>,
    pub cuttings: Vec<GazetteCuttingDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteCuttingsPage {
    pub items: Vec<GazetteCuttingDto>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GazetteRefreshReport {
    pub feeds: i64,
    pub imported: i64,
    pub errors: Vec<String>,
}

// ============================================================
// СКРОМНЫЕ ЭПИЧЕСКИЕ БИТВЫ — the shelf of cards
// ============================================================

/// A card as the row that was just written. `tier` is the card's rank; the
/// owner's `level` lives on `battle_owned_cards` and never on the card itself.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleCard {
    pub id: Uuid,
    pub slug: String,
    pub figurine_id: Option<Uuid>,
    pub race_id: Option<Uuid>,
    pub status: String,
    pub tier: i16,
    pub type_en: Option<String>,
    pub type_ru: Option<String>,
    pub title_en: String,
    pub title_ru: String,
    pub effect_en: Option<String>,
    pub effect_ru: Option<String>,
    pub lore_en: Option<String>,
    pub lore_ru: Option<String>,
    pub cost: i16,
    pub power: i16,
    pub health: i16,
    pub mana: i16,
    /// JSON array of named properties; see `battles::read_traits`.
    pub traits: Option<String>,
    // ── The body, as the engine reads it ──────────────────────────────────
    /// `unit` | `spell` | `relic`. Not `type_ru`, which is free text for the
    /// header band and which no rule may read.
    pub kind: String,
    /// Flat reduction of bodily damage.
    pub armor: i16,
    /// Flat reduction of charmed damage.
    pub ward: i16,
    /// Which defence answers this card's ordinary blow.
    pub attack_channel: String,
    /// How far the ordinary blow carries, in king's steps.
    pub reach: i16,
    /// How many cells it walks in one move. Zero — it stands where it was put.
    pub step: i16,
    /// Who acts first. Three is the middle.
    pub speed: i16,
    /// How much it mends in one act of mending. Interim: moves into `abilities`
    /// when abilities arrive.
    pub mend: i16,
    /// JSON array of executable abilities, beside the prose in `traits`.
    pub abilities: Option<String>,
    /// What the balance calculator worked out at the last save. A mirror.
    pub budget_points: Option<f64>,
    pub balance_index: Option<f64>,
    /// Editing the numbers is a new version, not a silent change.
    pub rules_version: i32,
    pub price_dust: Option<i32>,
    pub price_feed: Option<i32>,
    /// Price of each rung of the level ladder, in dust: 1→2, 2→3, 3→4, 4→5.
    /// `None` — this card does not rise. Written now, spent in 1c.
    pub level_price_dust: Option<Vec<i32>>,
    pub art_url: Option<String>,
    pub art_focal: Option<String>,
    /// This card's own exception to the tier's shared frame. JSON, see
    /// `battles::FrameOverride`. `None` wears the tier's frame unmodified.
    pub frame_override: Option<String>,
    pub shelf_order: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A card read for a page, carrying what the work it belongs to lends it: its
/// name, its address, and the face photograph the card wears when the keeper
/// has not given it a picture of its own.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleCardListed {
    pub id: Uuid,
    pub slug: String,
    pub figurine_id: Option<Uuid>,
    pub race_id: Option<Uuid>,
    pub status: String,
    pub tier: i16,
    pub type_en: Option<String>,
    pub type_ru: Option<String>,
    pub title_en: String,
    pub title_ru: String,
    pub effect_en: Option<String>,
    pub effect_ru: Option<String>,
    pub lore_en: Option<String>,
    pub lore_ru: Option<String>,
    pub cost: i16,
    pub power: i16,
    pub health: i16,
    pub mana: i16,
    /// JSON array of named properties; see `battles::read_traits`.
    pub traits: Option<String>,
    // ── The body, as the engine reads it ──────────────────────────────────
    /// `unit` | `spell` | `relic`. Not `type_ru`, which is free text for the
    /// header band and which no rule may read.
    pub kind: String,
    /// Flat reduction of bodily damage.
    pub armor: i16,
    /// Flat reduction of charmed damage.
    pub ward: i16,
    /// Which defence answers this card's ordinary blow.
    pub attack_channel: String,
    /// How far the ordinary blow carries, in king's steps.
    pub reach: i16,
    /// How many cells it walks in one move. Zero — it stands where it was put.
    pub step: i16,
    /// Who acts first. Three is the middle.
    pub speed: i16,
    /// How much it mends in one act of mending. Interim: moves into `abilities`
    /// when abilities arrive.
    pub mend: i16,
    /// JSON array of executable abilities, beside the prose in `traits`.
    pub abilities: Option<String>,
    /// What the balance calculator worked out at the last save. A mirror.
    pub budget_points: Option<f64>,
    pub balance_index: Option<f64>,
    /// Editing the numbers is a new version, not a silent change.
    pub rules_version: i32,
    pub price_dust: Option<i32>,
    pub price_feed: Option<i32>,
    /// Price of each rung of the level ladder, in dust: 1→2, 2→3, 3→4, 4→5.
    /// `None` — this card does not rise. Written now, spent in 1c.
    pub level_price_dust: Option<Vec<i32>>,
    pub art_url: Option<String>,
    pub art_focal: Option<String>,
    pub frame_override: Option<String>,
    pub shelf_order: Option<i32>,
    pub lendable: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub figurine_name: Option<String>,
    pub figurine_slug: Option<String>,
    /// Raw stored path of the work's face image; the service turns it into a URL.
    pub figurine_face_path: Option<String>,
    pub figurine_face_id: Option<Uuid>,
    pub race_name_en: Option<String>,
    pub race_name_ru: Option<String>,
    /// The race's shared icon — already a public URL, joined in from the
    /// dictionary so the shelf never has to fetch the race list just to draw it.
    pub race_icon_url: Option<String>,
    /// The race's own dress per level, joined in the same way as the icon —
    /// JSON, see `battles::normalize_level_frames`.
    pub race_level_frames: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleCardDto {
    pub id: String,
    pub slug: String,
    pub status: String,
    /// The card's rank, 1..5. Not to be confused with an owner's level.
    pub tier: i16,
    /// The header band: what this is, and what kind it is.
    pub race_id: Option<String>,
    pub race_name_en: Option<String>,
    pub race_name_ru: Option<String>,
    pub race_icon_url: Option<String>,
    pub race_level_frames: Option<String>,
    pub type_en: Option<String>,
    pub type_ru: Option<String>,
    pub title_en: String,
    pub title_ru: String,
    pub effect_en: Option<String>,
    pub effect_ru: Option<String>,
    pub lore_en: Option<String>,
    pub lore_ru: Option<String>,
    pub cost: i16,
    /// Strength, in the properties band.
    pub power: i16,
    pub health: i16,
    pub mana: i16,
    pub traits: Vec<crate::battles::CardTrait>,
    // ── The body, as the engine reads it ──────────────────────────────────
    pub kind: String,
    pub armor: i16,
    pub ward: i16,
    pub attack_channel: String,
    pub reach: i16,
    pub step: i16,
    pub speed: i16,
    pub mend: i16,
    /// The executable half, beside the prose in `traits`.
    pub abilities: Vec<crate::battles::CardAbility>,
    /// Only the desk needs the verdict; the shelf shows a card, not a ledger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_points: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_index: Option<f64>,
    pub rules_version: i32,
    pub price_dust: Option<i32>,
    pub price_feed: Option<i32>,
    /// Price of each rung of the level ladder, in dust: 1→2, 2→3, 3→4, 4→5.
    /// `None` — this card does not rise. Written now, spent in 1c.
    pub level_price_dust: Option<Vec<i32>>,
    /// What the card actually wears: the keeper's own picture if there is one,
    /// otherwise the work's face. Already a public URL.
    pub art_url: Option<String>,
    /// The override as the keeper typed it, so the desk can tell "no picture of
    /// its own" from "borrowing the work's". Absent on the public shelf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art_url_override: Option<String>,
    pub art_focal: Option<String>,
    /// This card's own exception to the tier's shared frame. JSON, see
    /// `battles::FrameOverride`. `None` wears the tier's frame unmodified.
    pub frame_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shelf_order: Option<i32>,
    /// Готов ли дом одолжить эту карту тому, у кого своего ещё нет.
    /// Отбирается ещё и по чину: одалживается только первый.
    pub lendable: bool,
    pub figurine_id: Option<String>,
    pub figurine_name: Option<String>,
    pub figurine_slug: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBattleCardRequest {
    pub slug: Option<String>,
    pub status: String,
    pub tier: i16,
    pub race_id: Option<String>,
    pub type_en: Option<String>,
    pub type_ru: Option<String>,
    pub title_en: String,
    pub title_ru: String,
    pub effect_en: Option<String>,
    pub effect_ru: Option<String>,
    pub lore_en: Option<String>,
    pub lore_ru: Option<String>,
    #[serde(default)]
    pub cost: i16,
    #[serde(default)]
    pub power: i16,
    #[serde(default)]
    pub health: i16,
    #[serde(default)]
    pub mana: i16,
    #[serde(default)]
    pub traits: Vec<crate::battles::CardTrait>,
    #[serde(default = "crate::battles::default_kind")]
    pub kind: String,
    #[serde(default)]
    pub armor: i16,
    #[serde(default)]
    pub ward: i16,
    #[serde(default = "crate::battles::default_channel")]
    pub attack_channel: String,
    #[serde(default = "crate::battles::default_reach")]
    pub reach: i16,
    #[serde(default = "crate::battles::default_step")]
    pub step: i16,
    #[serde(default = "crate::battles::default_speed")]
    pub speed: i16,
    #[serde(default)]
    pub mend: i16,
    #[serde(default)]
    pub abilities: Vec<crate::battles::CardAbility>,
    pub price_dust: Option<i32>,
    pub price_feed: Option<i32>,
    /// Price of each rung of the level ladder, in dust: 1→2, 2→3, 3→4, 4→5.
    /// `None` — this card does not rise. Written now, spent in 1c.
    pub level_price_dust: Option<Vec<i32>>,
    pub art_url: Option<String>,
    pub art_focal: Option<String>,
    pub frame_override: Option<String>,
    /// Готов ли дом одолжить эту карту. Отсутствует в старом запросе — значит
    /// «нет»: карта не становится заёмной от того, что её сохранили формой,
    /// которая про заём не знает.
    #[serde(default)]
    pub lendable: bool,
    pub figurine_id: Option<String>,
}

/// A card as it will be written, already checked and clamped.
///
/// One struct rather than thirty-odd positional arguments. Not tidiness: two
/// neighbouring `Option<&str>` in a long argument list can be swapped by hand
/// and the compiler will not say a word, and `lore_ru` quietly holding the
/// effect is the kind of bug that is found by a reader, months later.
#[derive(Debug, Clone)]
pub struct BattleCardWrite {
    pub slug: String,
    pub figurine_id: Option<Uuid>,
    pub race_id: Option<Uuid>,
    pub status: String,
    pub tier: i16,
    pub type_en: Option<String>,
    pub type_ru: Option<String>,
    pub title_en: String,
    pub title_ru: String,
    pub effect_en: Option<String>,
    pub effect_ru: Option<String>,
    pub lore_en: Option<String>,
    pub lore_ru: Option<String>,
    pub cost: i16,
    pub power: i16,
    pub health: i16,
    pub mana: i16,
    pub traits: Option<String>,
    pub kind: String,
    pub armor: i16,
    pub ward: i16,
    pub attack_channel: String,
    pub reach: i16,
    pub step: i16,
    pub speed: i16,
    pub mend: i16,
    pub abilities: Option<String>,
    pub budget_points: Option<f64>,
    pub balance_index: Option<f64>,
    pub price_dust: Option<i32>,
    pub price_feed: Option<i32>,
    /// Price of each rung of the level ladder, in dust: 1→2, 2→3, 3→4, 4→5.
    /// `None` — this card does not rise. Written now, spent in 1c.
    pub level_price_dust: Option<Vec<i32>>,
    pub art_url: Option<String>,
    pub art_focal: Option<String>,
    pub frame_override: Option<String>,
    pub lendable: bool,
}

/// What the scales say about a card that has not been saved yet.
///
/// Exists so the keeper sees the verdict while typing without the formula being
/// written a second time in TypeScript. Two implementations of one formula
/// disagree by the second week, and the disagreement is found by a player.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleWeighDto {
    /// What the body costs before a single ability is written on it.
    pub body_points: f64,
    /// One entry per ability, in the order written.
    pub abilities: Vec<AbilityWeightDto>,
    pub total_points: f64,
    /// Points against price. 1.0 is on the curve.
    pub balance_index: f64,
    /// What a card of this rank is allowed to weigh.
    pub tier_budget: f64,
    /// What the price would have to be for this weight to sit on the curve.
    pub suggested_cost: i16,
    /// Годна ли карта к публикации, и что о ней стоит знать. Считается тем же
    /// правилом, которым сохранение откажет, — чтобы подсказка и отказ не
    /// разошлись.
    pub readiness: CardReadinessDto,
}

/// Слова, а не текст: текст живёт в `i18n` на двух языках, а сервер, который
/// его сочиняет, сочиняет его на одном.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardReadinessDto {
    /// Пока непусто — карту нельзя опубликовать.
    pub blocking: Vec<String>,
    /// Так можно, но стоит знать.
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbilityWeightDto {
    /// The ability's own id inside the card, so the desk can put the number
    /// beside the right row.
    pub id: String,
    pub points: f64,
}

// ── Стол ─────────────────────────────────────────────────────────────────────

/// A match on the keeper's bench: an arrangement and everything done to it.
///
/// Nothing is stored. The whole position travels with every request and is
/// folded from the journal each time — which costs a few microseconds and buys
/// three things: no rows to clean up afterwards, no account and no dust
/// involved, and the replay property exercised on literally every click.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchRequest {
    pub setup: ChallengeSetup,
    /// What has already been played. Replayed silently to rebuild the board.
    #[serde(default)]
    pub actions: Vec<battle_core::Action>,
    /// One more move, if the keeper is making one.
    #[serde(default)]
    pub next: Option<battle_core::Action>,
    /// Whether the far side answers by itself after the move.
    #[serde(default)]
    pub auto_keeper: bool,
    /// Какой рукой играет бот на столе: 1 — жадной, 2 — с перебором. Стол
    /// затем и нужен, чтобы этюд проверялся той же рукой, какой его пройдёт
    /// гость; без этого хранитель проверяет этюд на одном боте, а оставляет
    /// на другом.
    #[serde(default = "crate::battles::default_bot_depth")]
    pub bot_depth: i16,
    /// Play the rest out with the bot on both sides and return the ending.
    ///
    /// Not "run it a thousand times": the engine has no chance in it, so a
    /// thousand runs of one arrangement are one run repeated. The answer to
    /// "who wins this?" is a single play.
    #[serde(default)]
    pub play_out: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchDto {
    pub state: battle_core::MatchState,
    pub legal_actions: Vec<battle_core::Action>,
    /// Everything that happened because of this request.
    pub events: Vec<battle_core::Event>,
    /// The journal to send back next time.
    pub actions: Vec<battle_core::Action>,
    pub outcome: Option<String>,
}

/// Одна ступень пересмотра записанной партии.
///
/// Отдельно от `BenchDto` не ради поля-двух: у стола журнал ЖИВОЙ и растёт от
/// запроса к запросу, а здесь он записан и неизменен, и вместо «что послать в
/// следующий раз» нужно «сколько всего ступеней и на какой мы стоим».
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchReplayDto {
    pub state: battle_core::MatchState,
    /// Что произошло на этой ступени — то, что сцена проигрывает.
    pub events: Vec<battle_core::Event>,
    /// Весь журнал: сцена подписывает ступени, не зная правил.
    pub actions: Vec<battle_core::Action>,
    /// На какой ступени стоим и сколько их всего.
    pub upto: usize,
    pub total: usize,
    pub outcome: Option<String>,
    /// Партия перестала переигрываться: правила менялись с тех пор, как её
    /// сыграли, и записанное действие больше не законно. Не ошибка, а факт о
    /// записи — показать доску до этого места честнее, чем отказать целиком.
    pub diverged: bool,
}

/// A keyword in the keeper's dictionary: Шипы, Немота, Покров, Яд.
///
/// A table for the three reasons a race is one — shared by many cards, renamed
/// in one place, read by a rule — and one of its own: `point_value` is the
/// exchange rate from the balance formula, so rebalancing the whole game is an
/// edit in a dictionary rather than a deployment.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleKeyword {
    pub id: Uuid,
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    /// The canonical wording, one per game. A card says "Шипы 3"; what Шипы
    /// means is said here, once.
    pub rules_en: Option<String>,
    pub rules_ru: Option<String>,
    pub icon_url: Option<String>,
    /// Points per unit, from the exchange table. `None` — not priced yet.
    pub point_value: Option<f64>,
    pub sort_order: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleKeywordDto {
    pub id: String,
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub rules_en: Option<String>,
    pub rules_ru: Option<String>,
    pub icon_url: Option<String>,
    pub point_value: Option<f64>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBattleKeywordRequest {
    pub slug: Option<String>,
    pub name_en: String,
    pub name_ru: String,
    pub rules_en: Option<String>,
    pub rules_ru: Option<String>,
    pub icon_url: Option<String>,
    pub point_value: Option<f64>,
}

// ── Испытания и партии ───────────────────────────────────────────────────────

/// One place on a challenge's board: a card, by slug, and where it stands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengePlacement {
    pub card: String,
    pub x: u8,
    pub y: u8,
}

/// A challenge's arrangement, by slug rather than by snapshot.
///
/// A challenge is a template: editing a card must change every challenge that
/// uses it. The snapshot is taken when a match begins and lives in the match.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeSetup {
    #[serde(default)]
    pub player_board: Vec<ChallengePlacement>,
    #[serde(default)]
    pub player_hand: Vec<String>,
    #[serde(default)]
    pub keeper_board: Vec<ChallengePlacement>,
    #[serde(default)]
    pub keeper_hand: Vec<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleChallenge {
    pub id: Uuid,
    pub slug: String,
    pub title_en: String,
    pub title_ru: String,
    pub note_en: Option<String>,
    pub note_ru: Option<String>,
    /// JSON, see `ChallengeSetup`.
    pub setup: String,
    pub bot_depth: i16,
    /// Paid once per challenge, never per victory.
    pub reward_dust: i32,
    pub player_side: String,
    pub status: String,
    pub sort_order: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleChallengeDto {
    pub id: String,
    pub slug: String,
    pub title_en: String,
    pub title_ru: String,
    pub note_en: Option<String>,
    pub note_ru: Option<String>,
    pub setup: ChallengeSetup,
    pub bot_depth: i16,
    pub reward_dust: i32,
    /// `scripted` — обе стороны заданы рукой (этюд, у него есть решение).
    /// `deck` — хранитель ставит своё, гость приводит свой стол (встреча).
    pub player_side: String,
    pub status: String,
    pub sort_order: Option<i32>,
    /// Whether this visitor has already been paid for this challenge. `None`
    /// for a guest, who is not being paid for anything yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub already_paid: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBattleChallengeRequest {
    pub slug: Option<String>,
    pub title_en: String,
    pub title_ru: String,
    pub note_en: Option<String>,
    pub note_ru: Option<String>,
    pub setup: ChallengeSetup,
    #[serde(default = "one")]
    pub bot_depth: i16,
    #[serde(default)]
    pub reward_dust: i32,
    /// Отсутствует в старом запросе — значит `scripted`: испытание не меняет
    /// род от того, что его сохранили формой, которая про род не знает.
    #[serde(default = "scripted")]
    pub player_side: String,
    pub status: String,
}

fn scripted() -> String {
    "scripted".into()
}

/// Испытание, как оно будет записано.
///
/// Струтура, а не одиннадцать позиционных аргументов, и по той же причине, по
/// которой её завела карта: два соседних `&str` в длинном списке меняются
/// местами рукой, и компилятор не скажет ни слова — а испытание, у которого
/// состояние тихо держит род стороны, находит читатель месяцы спустя.
#[derive(Debug, Clone)]
pub struct BattleChallengeWrite<'a> {
    pub slug: &'a str,
    pub title_en: &'a str,
    pub title_ru: &'a str,
    pub note_en: Option<&'a str>,
    pub note_ru: Option<&'a str>,
    pub setup: &'a str,
    pub bot_depth: i16,
    pub reward_dust: i32,
    pub player_side: &'a str,
    pub status: &'a str,
}

fn one() -> i16 {
    1
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleMatch {
    pub id: Uuid,
    pub user_id: Uuid,
    pub challenge_id: Option<Uuid>,
    pub setup: String,
    pub rules_version: i32,
    /// The journal. The truth about a match; everything else is derived.
    pub actions: String,
    pub board_cache: Option<String>,
    pub seq: i32,
    pub outcome: Option<String>,
    pub rounds: Option<i16>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

// ── Сыгранные партии, как их читает хранитель ────────────────────────────────
//
// Единственный источник правды о живой игре. До этого он был закрыт: партии
// писались в базу, а посмотреть их было нечем — баланс правился симуляцией по
// правилам, которых игроки не видели.

/// Одна сыгранная партия строкой.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BattleMatchRow {
    pub id: Uuid,
    pub guest: String,
    pub challenge_id: Option<Uuid>,
    pub title_ru: Option<String>,
    pub title_en: Option<String>,
    pub outcome: Option<String>,
    pub rounds: Option<i16>,
    pub setup: String,
    pub actions: String,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleMatchRowDto {
    pub id: String,
    pub guest: String,
    pub challenge_id: Option<String>,
    pub title_ru: Option<String>,
    pub title_en: Option<String>,
    /// `player` — победил гость, `keeper` — дом, `draw` — ничья, пусто — партия
    /// не доиграна.
    pub outcome: Option<String>,
    pub rounds: Option<i16>,
    /// Длина журнала. Не то же, что круги: за один круг ходов бывает несколько.
    pub moves: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
}

/// Сводка по одному испытанию.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleChallengeTally {
    pub challenge_id: Option<String>,
    pub title_ru: Option<String>,
    pub title_en: Option<String>,
    pub played: i64,
    pub guest_won: i64,
    pub keeper_won: i64,
    pub draws: i64,
    pub unfinished: i64,
}

/// Сводка по одной карте: сколько раз выходила на поле и чем это кончалось.
///
/// Считается по замороженной расстановке партии, где карта названа слугом, —
/// то есть по тому, что действительно стояло на доске, а не по нынешней полке.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleCardTally {
    pub slug: String,
    pub title_ru: Option<String>,
    pub title_en: Option<String>,
    /// В скольких доигранных партиях карта была на поле.
    pub played: i64,
    /// Из них сколько выиграла её сторона.
    pub won: i64,
    pub lost: i64,
    pub draws: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleMatchesDto {
    pub rows: Vec<BattleMatchRowDto>,
    pub by_challenge: Vec<BattleChallengeTally>,
    pub by_card: Vec<BattleCardTally>,
    /// Сколько партий прочитано. Полка не бесконечна, и сводка считается по
    /// прочитанному — хранителю надо знать, по чему именно.
    pub read: i64,
}

/// A match as the scene needs it.
///
/// Carries the engine's own state and its own list of legal actions, unchanged.
/// The client draws what it is given and computes no rule of its own — which is
/// the only way to have a live board and one implementation of the rules.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleMatchDto {
    pub id: String,
    pub challenge_id: Option<String>,
    /// The number the next action must carry.
    pub seq: i32,
    pub state: battle_core::MatchState,
    pub legal_actions: Vec<battle_core::Action>,
    /// What just happened, for the scene to play. Empty when a match is read
    /// rather than acted upon.
    pub events: Vec<battle_core::Event>,
    pub outcome: Option<String>,
    /// Dust credited by this very request. Zero on every later reading — the
    /// reward belongs to the challenge, not to the victory.
    pub reward_dust: i32,
}

/// One move by the player, with the number that makes a repeat harmless.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleActRequest {
    pub seq: i32,
    pub action: battle_core::Action,
}

/// One copy of one card, belonging to one person.
///
/// `level` lives here and never on the card: the card's rank is the keeper's
/// choice and the same for everybody, the level is the state of *your* copy.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleOwnedCard {
    pub id: Uuid,
    pub user_id: Uuid,
    pub card_id: Uuid,
    pub level: i16,
    pub acquired_at: DateTime<Utc>,
    /// NULL while the card still wears the "new" mark on the shelf.
    pub seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleOwnedCardDto {
    pub card_id: String,
    pub level: i16,
    /// Whether the card still wears the mark. Sent instead of the timestamp:
    /// the shelf needs the answer, not the hour.
    pub is_new: bool,
}

/// Everything the shelf needs to know about the person looking at it.
///
/// One request rather than three: the shelf cannot draw a single card until it
/// knows whether that card is theirs, and a page that asks three questions to
/// draw one row is a page that flickers.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleMeDto {
    pub dust: i64,
    pub feed: i64,
    pub owned: Vec<BattleOwnedCardDto>,
    /// Что дали из рук, с записками. Пыль, осевшая с маяков, сюда не попадает:
    /// у неё нет записки и считается она иначе.
    pub gifts: Vec<BattleGiftDto>,
}

/// Строка книги, данная рукой хранителя.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleGrantRow {
    pub currency: String,
    pub amount: i32,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Подарок, как его читает полка.
///
/// Монета, сколько и — главное — за что. Число без записки было бы ровно тем,
/// от чего эта комната отказывается: молча выросшим счётчиком.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleGiftDto {
    pub currency: String,
    pub amount: i32,
    pub note: Option<String>,
    pub at: String,
}

/// Выдать карты гостю напрямую.
///
/// Отдельно от покупки: покупка списывает монеты и проверяет цену, а это
/// подарок — им приводят собрание в нужное состояние, чтобы проверить игру.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GiveBattleCardsRequest {
    pub user_id: Uuid,
    #[serde(default)]
    pub card_ids: Vec<Uuid>,
    /// Все опубликованные карты, которые могут выйти на поле.
    #[serde(default)]
    pub all: bool,
    /// Уровень выданных копий. Перезаписывает имеющийся — намеренно.
    #[serde(default = "one_level")]
    pub level: i16,
}

fn one_level() -> i16 {
    1
}

/// Забрать карты обратно. Пустой список и `all` — забрать все.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeBattleCardsRequest {
    pub user_id: Uuid,
    #[serde(default)]
    pub card_ids: Vec<Uuid>,
    #[serde(default)]
    pub all: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GiveBattleCardsResponse {
    /// Сколько строк собрания затронуто.
    pub touched: u64,
}

/// Из рук хранителя — одному гостю, за настоящее.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantBattleCoinRequest {
    pub user_id: Uuid,
    /// `dust` или `feed`.
    pub currency: String,
    /// Со знаком. Минус — это не штраф, а исправление: ошибка в книге правится
    /// обратной строкой, а не правкой строки, которая была неверна.
    pub amount: i32,
    pub note: Option<String>,
    /// Ключ этого АКТА, а не его содержимого, и его чеканит панель — по одному
    /// на открытую форму. Иначе двойной щелчок раздал бы дважды, а два
    /// состоявшихся показа одному гостю слились бы в один.
    pub idem_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantBattleCoinResponse {
    pub balance: i64,
    /// Легла ли строка сейчас. Ложь — значит этот же ключ уже приходил.
    pub granted_now: bool,
}

// ── Стол гостя ───────────────────────────────────────────────────────────────
//
// Шесть карт: три стоят на клетках своей половины, три в руке. Форма измерена,
// не выбрана — см. миграцию `20260901000000_battle_decks.sql`.

/// Одна карта колоды на клетке. Клетки пишутся клетками, а не порядком в
/// массиве: расстановка — это выбор клеток, и «первая карта = левая дальняя»
/// пришлось бы держать в голове игрока.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckPlacement {
    /// Идентификатор карты: владение записано на `card_id`, а не на слаг.
    pub card: Uuid,
    pub x: u8,
    pub y: u8,
}

/// Стол, как он лежит в базе. Ровно то, что выбрал гость, — без заёма:
/// заём принадлежит дому и досчитывается на каждое чтение, иначе он застыл бы
/// в чужой колоде и пережил бы решение хранителя его отозвать.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckLayout {
    #[serde(default)]
    pub board: Vec<DeckPlacement>,
    #[serde(default)]
    pub hand: Vec<Uuid>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleDeck {
    pub user_id: Uuid,
    /// JSON, см. `DeckLayout`.
    pub layout: String,
    pub updated_at: DateTime<Utc>,
}

/// Одно место на столе — то, что комната рисует.
///
/// Три состояния в двух полях, и ни одно не выводится клиентом:
///   * `card_id` есть, `gone` ложно  — ваша карта;
///   * `card_id` пусто               — место пустое, и дом кладёт `lent_card_id`;
///   * `card_id` есть, `gone` истинно — ваша карта снята с полки, и дом кладёт
///     `lent_card_id` вместо неё. Молча выбросить её значило бы переписать
///     чужую расстановку за спиной.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleDeckSlotDto {
    /// Выбор гостя. `None` — место оставлено пустым.
    pub card_id: Option<String>,
    /// Выбранная карта больше не выходит на поле (снята с полки либо потеряла
    /// здоровье). Показывается зачёркнутой, а не исчезает.
    pub gone: bool,
    /// Чем дом закрывает это место. `None` — закрывать нечего.
    pub lent_card_id: Option<String>,
    /// Только у мест на поле.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u8>,
}

/// Весь стол одним ответом: комната не может нарисовать ни одного места, пока
/// не знает, чьё оно, — та же причина, по которой `battle_me` отвечает разом.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleDeckDto {
    pub board: Vec<BattleDeckSlotDto>,
    pub hand: Vec<BattleDeckSlotDto>,
    /// Раскладывал ли гость стол хоть раз. Первый заход на встречу ведёт на
    /// стол, а не в партию: человек должен увидеть, что он приводит.
    pub laid: bool,
    /// Дому нечего одолжить — хранитель не отметил ни одной карты заёмной.
    /// Комната говорит это вслух, а не рисует пустые места без объяснения.
    pub nothing_to_lend: bool,
}

/// Что гость сохраняет. Заём сюда не приходит: его не выбирают.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBattleDeckRequest {
    #[serde(default)]
    pub board: Vec<DeckPlacement>,
    #[serde(default)]
    pub hand: Vec<Uuid>,
}

/// Taking a card off the shelf.
///
/// The price travels with the request only so the server can refuse a stale
/// one: the card may have been repriced while the page stood open, and taking
/// it at yesterday's price would be the keeper paying for the visitor's tab.
/// The server never trusts this number, it compares it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuyBattleCardRequest {
    pub card_id: Uuid,
    /// `dust` or `feed`.
    pub currency: String,
    pub expected_price: i32,
}

/// What the ceremony is told.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuyBattleCardResponse {
    pub card_id: String,
    pub level: i16,
    /// Balance after, in the currency that was spent — so the shelf does not
    /// have to ask again to redraw the margin.
    pub balance: i64,
    /// False when this card was already theirs and nothing was written. The
    /// ceremony plays once; a repeat is answered, not celebrated.
    pub taken_now: bool,
}

/// Raising your own copy a rung.
///
/// No rung number in the request: the server reads the level it holds and works
/// out which rung that is. A client that could name the rung could name the
/// cheap one.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaiseBattleCardRequest {
    pub card_id: Uuid,
    /// The price the owner saw for the next rung. Compared, never trusted.
    pub expected_price: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RaiseBattleCardResponse {
    pub card_id: String,
    /// The level after. The same number on a repeat: a rung is climbed once.
    pub level: i16,
    pub balance: i64,
    /// False when the rung was already paid for and nothing was written.
    pub raised_now: bool,
}

/// An act of attention worth a grain of dust.
///
/// One kind, one thing, once. The pair becomes the ledger key (`liked:{id}`),
/// and that key is what keeps a re-read tale from paying twice.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleAttentionRequest {
    /// `liked` | `seen` | `read`.
    pub kind: String,
    /// The work or the leaf the attention was paid to.
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleAttentionResponse {
    /// How much settled just now. Zero when this act was already paid for, or
    /// when the keeper pays nothing for this kind.
    pub dust: i32,
    pub balance: i64,
}

/// What the keeper pays for attention the house already counts.
///
/// A setting and not a table: three numbers the keeper tunes, next to the
/// frames. Zero anywhere means that kind pays nothing at all.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleDustRates {
    pub liked: i32,
    pub seen: i32,
    pub read: i32,
}

impl Default for BattleDustRates {
    /// A starting point, not a measurement. Deliberately small beside a
    /// challenge (25): attention is the slow, wide source and a study is the
    /// deliberate one, and the order between them should be felt.
    fn default() -> Self {
        Self { liked: 2, seen: 1, read: 3 }
    }
}

/// The shelf, left to right. Position is the index in the list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderBattleCardsRequest {
    pub ids: Vec<Uuid>,
}

/// A race in the keeper's dictionary. Shared by many cards, so a table rather
/// than a field: renaming one must rename it everywhere at once.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleRace {
    pub id: Uuid,
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub note_en: Option<String>,
    pub note_ru: Option<String>,
    /// Shown in the header band of every card of this race. Already a public
    /// URL — set once here, worn by every card, the same choice already made
    /// for frames.
    pub icon_url: Option<String>,
    /// This race's own dress per level of an owned copy — JSON, see
    /// `battles::normalize_level_frames`. `None` wears the tier's frame at
    /// every level.
    pub level_frames: Option<String>,
    pub sort_order: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A race read for the desk, carrying how many cards stand under it. A separate
/// struct because sqlx maps a row into ONE `FromRow` type — a tuple of a struct
/// and a count is not something it can decode.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BattleRaceListed {
    pub id: Uuid,
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub note_en: Option<String>,
    pub note_ru: Option<String>,
    pub icon_url: Option<String>,
    pub level_frames: Option<String>,
    pub sort_order: Option<i32>,
    pub card_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleRaceDto {
    pub id: String,
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub note_en: Option<String>,
    pub note_ru: Option<String>,
    pub icon_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_frames: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    /// How many cards stand under this race. The keeper sees what a rename or a
    /// removal would touch before doing it.
    pub card_count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBattleRaceRequest {
    pub slug: Option<String>,
    pub name_en: String,
    pub name_ru: String,
    pub note_en: Option<String>,
    pub note_ru: Option<String>,
    pub icon_url: Option<String>,
    pub level_frames: Option<String>,
}
