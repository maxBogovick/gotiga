use serde::{Deserialize, Serialize};

/// Result of a bulk admin operation applied across every figurine/image.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOpResultDto {
    pub affected: usize,
}

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
    #[serde(default)]
    pub name: String,
    pub tagline: Option<String>,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    /// Portrait for the site-header avatar — distinct from `photo_url`.
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

// ============================================================
// ВНУТРЕННИЕ МОДЕЛИ (для работы с БД)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Figurine {
    pub id: String,
    pub name: String,
    /// Transliterated URL slug (unique when set); NULL for rows not yet re-saved.
    pub slug: Option<String>,
    /// True when the slug was hand-typed by an admin (differs from the name-derived
    /// auto slug); false when auto-generated. Mirrors the web server's column.
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
    pub status: FigurineStatus,
    pub sort_order: i32,
    pub updated_at: String,
    pub is_visible: bool,
    pub is_featured: bool,
    /// "The house wakes" — daily showing window in minutes from midnight (0..1439),
    /// guest-local. Both NULL → always open. `until < from` wraps past midnight.
    pub open_from_min: Option<i32>,
    pub open_until_min: Option<i32>,
    /// Optional sealed-door asset URL. NULL → procedural carved door on the client.
    pub sealed_door_image: Option<String>,
    /// Optional "showing room" (FK showing_rooms.id). When set, the room's window
    /// is used instead of the per-figurine open_from/until. NULL → own window.
    pub showing_room_id: Option<String>,
    /// Which detail-page layout to use. NULL → 'specimen' (default).
    pub display_layout: Option<String>,
    /// JSON blob for per-figurine display customisation ({background, blockOrder}).
    pub display_config: Option<String>,
}

/// A named, shared showing window several works can point at (e.g. "Night hall").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowingRoom {
    pub id: String,
    pub name: String,
    pub open_from_min: i32,
    pub open_until_min: i32,
    pub open_days_mask: Option<i32>,
    pub open_month_day: Option<String>,
    pub open_date_from: Option<String>,
    pub open_date_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowingRoomDto {
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

impl From<ShowingRoom> for ShowingRoomDto {
    fn from(r: ShowingRoom) -> Self {
        Self {
            id: r.id,
            name: r.name,
            open_from_min: r.open_from_min,
            open_until_min: r.open_until_min,
            open_days_mask: r.open_days_mask,
            open_month_day: r.open_month_day,
            open_date_from: r.open_date_from,
            open_date_until: r.open_date_until,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FigurineStatus {
    Available,
    Sold,
    Reserved,
    InProgress,
}

impl FigurineStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "sold" => Self::Sold,
            "reserved" => Self::Reserved,
            "in_progress" => Self::InProgress,
            _ => Self::Available,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Sold => "sold",
            Self::Reserved => "reserved",
            Self::InProgress => "in_progress",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub figurine_id: String,
    pub image_type: ImageType,
    pub file_path: String,
    pub original_path: Option<String>,
    pub thumb_path: Option<String>,
    pub depth_path: Option<String>,
    pub parallax_intensity: Option<f32>,
    /// "Keyhole" reveal focus (0..1) + radius (0..1). NULL = centre + default.
    pub focal_x: Option<f32>,
    pub focal_y: Option<f32>,
    pub reveal_radius: Option<f32>,
    /// Per-image darkness override (0..1). NULL → global keyhole darkness.
    pub darkness: Option<f32>,
    pub alt_text: Option<String>,
    pub sort_order: i32,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageType {
    Face,   // крупный план лица
    Detail, // детали
    Full,   // полный вид
}

impl ImageType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "face" => Self::Face,
            "detail" => Self::Detail,
            _ => Self::Full,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Face => "face",
            Self::Detail => "detail",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    pub id: String,
    pub category: TextCategory,
    pub content: String,
    pub caption: Option<String>,
    pub image_path: Option<String>,
    pub sort_order: i32,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextCategory {
    Author,
    Workshop,
}

impl TextCategory {
    pub fn from_str(s: &str) -> Self {
        match s {
            "author" => Self::Author,
            _ => Self::Workshop,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CabinetZone {
    pub id: String,
    pub zone_type: String,
    pub x_percent: f64,
    pub y_percent: f64,
    pub width_percent: f64,
    pub height_percent: f64,
    pub target_route: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStep {
    pub id: String,
    pub figurine_id: String,
    pub step_type: ProcessStepType,
    pub description: String,
    pub image_path: String,
    pub sort_order: i32,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcessStepType {
    Sketch,
    Prototype,
    Modeling,
    Painting,
    Finish,
}

impl ProcessStepType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "prototype" => Self::Prototype,
            "modeling" => Self::Modeling,
            "painting" => Self::Painting,
            "finish" => Self::Finish,
            _ => Self::Sketch,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sketch => "sketch",
            Self::Prototype => "prototype",
            Self::Modeling => "modeling",
            Self::Painting => "painting",
            Self::Finish => "finish",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRelease {
    pub id: String,
    pub version: i32,
    pub created_at: String,
    pub description: Option<String>,
    pub is_active: bool,
}

// ============================================================
// DTO (для передачи на frontend, сериализуемые)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineDto {
    pub id: String,
    pub name: String,
    /// Transliterated URL slug; null for works not yet re-saved.
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
    pub status: String,
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
    pub images: Vec<ImageDto>,
    pub process_steps: Vec<ProcessStepDto>,
    pub related_items: Vec<FigurineListItemDto>,
}

impl FigurineDto {
    pub fn from_figurine(
        figurine: Figurine,
        images: Vec<Image>,
        steps: Vec<ProcessStep>,
        related_items: Vec<FigurineListItemDto>,
        base_path: &str,
    ) -> Self {
        // Helper to prepend protocol if path exists
        let resolve = |p: String| {
            if p.starts_with("http") {
                p
            } else {
                format!("cabinet://localhost/{}", p)
            }
        };

        Self {
            id: figurine.id,
            name: figurine.name,
            slug: figurine.slug,
            short_text: figurine.short_text,
            full_description: figurine.full_description,
            dimensions: figurine.dimensions,
            material: figurine.material,
            technique: figurine.technique,
            year: figurine.year,
            passport_number: figurine.passport_number,
            edition: figurine.edition,
            created_period: figurine.created_period,
            care_instructions: figurine.care_instructions,
            provenance_note: figurine.provenance_note,
            authenticity_note: figurine.authenticity_note,
            included_items: figurine.included_items,
            ambience_path: figurine.ambience_path.map(|p| resolve(p)),
            video_url: figurine.video_url.map(|p| resolve(p)),
            secret_text: figurine.secret_text,
            status: figurine.status.as_str().to_string(),
            sort_order: figurine.sort_order,
            is_visible: figurine.is_visible,
            is_featured: figurine.is_featured,
            open_from_min: figurine.open_from_min,
            open_until_min: figurine.open_until_min,
            sealed_door_image: figurine.sealed_door_image,
            showing_room_id: figurine.showing_room_id,
            display_layout: figurine.display_layout,
            display_config: figurine.display_config,
            images: images
                .into_iter()
                .map(|i| ImageDto::from_image(i, base_path))
                .collect(),
            process_steps: steps
                .into_iter()
                .map(|s| ProcessStepDto::from_step(s, base_path))
                .collect(),
            related_items,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDto {
    pub id: String,
    pub image_type: String,
    pub url: String, // путь для frontend (asset://)
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

impl ImageDto {
    pub fn from_image(image: Image, _base_path: &str) -> Self {
        let resolve = |path: String| format!("cabinet://localhost/{}", path);
        Self {
            id: image.id,
            image_type: image.image_type.as_str().to_string(),
            url: resolve(image.file_path),
            original_url: image.original_path.map(resolve),
            thumb_url: image.thumb_path.map(resolve),
            depth_url: image.depth_path.map(resolve),
            parallax_intensity: image.parallax_intensity,
            focal_x: image.focal_x,
            focal_y: image.focal_y,
            reveal_radius: image.reveal_radius,
            darkness: image.darkness,
            alt_text: image.alt_text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessStepDto {
    pub id: String,
    pub step_type: String,
    pub description: String,
    pub image_url: String,
}

impl ProcessStepDto {
    pub fn from_step(step: ProcessStep, _base_path: &str) -> Self {
        Self {
            id: step.id,
            step_type: step.step_type.as_str().to_string(),
            description: step.description,
            image_url: format!("cabinet://localhost/{}", step.image_path),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineListItemDto {
    pub id: String,
    pub name: String,
    /// Transliterated URL slug; null for works not yet re-saved.
    #[serde(default)]
    pub slug: Option<String>,
    /// True when the slug was hand-typed (differs from the name-derived auto slug).
    #[serde(default)]
    pub slug_manual: bool,
    pub status: String,
    /// Mirrors the web server's list DTO: the home gallery renders this under each
    /// plate. Carried on the list item so the client never has to fetch a full
    /// figurine per pane just to read one caption. The row is already loaded here.
    #[serde(default)]
    pub short_text: Option<String>,
    pub face_image_url: Option<String>,
    /// Second-angle image for the home gallery's hover reveal; null when the
    /// piece has no dedicated "detail" image.
    #[serde(default)]
    pub detail_image_url: Option<String>,
    pub year: Option<i32>,
    pub sort_order: i32,
    pub series: Option<String>,
    pub is_featured: bool,
    /// When the piece was last edited — lets the home "since your visit" ledger
    /// surface updated works, matching the server build's list payload.
    pub updated_at: String,
    /// Face-image "keyhole" reveal focus + radius + darkness, surfaced on the card.
    pub focal_x: Option<f32>,
    pub focal_y: Option<f32>,
    pub reveal_radius: Option<f32>,
    pub darkness: Option<f32>,
    /// Showing window (minutes from midnight); both NULL → always open.
    pub open_from_min: Option<i32>,
    pub open_until_min: Option<i32>,
    /// Optional sealed-door asset; null → procedural carved door.
    pub sealed_door_image: Option<String>,
    /// Showing room this work belongs to (null → uses its own window).
    pub showing_room_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDto {
    pub id: String,
    pub content: String,
}

impl From<Text> for TextDto {
    fn from(text: Text) -> Self {
        Self {
            id: text.id,
            content: text.content,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItemDto {
    pub id: String,
    pub content: String,
    pub caption: Option<String>,
    pub image_url: Option<String>,
}

impl WorkshopItemDto {
    pub fn from_text(text: Text, _base_path: &str) -> Self {
        Self {
            id: text.id,
            content: text.content,
            caption: text.caption,
            image_url: text
                .image_path
                .map(|p| format!("cabinet://localhost/{}", p)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CabinetZoneDto {
    pub id: String,
    pub zone_type: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub target_route: String,
}

impl From<CabinetZone> for CabinetZoneDto {
    fn from(zone: CabinetZone) -> Self {
        Self {
            id: zone.id,
            zone_type: zone.zone_type,
            x: zone.x_percent,
            y: zone.y_percent,
            width: zone.width_percent,
            height: zone.height_percent,
            target_route: zone.target_route,
        }
    }
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
