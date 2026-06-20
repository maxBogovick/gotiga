use serde::{Deserialize, Serialize};

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
}

// ============================================================
// ВНУТРЕННИЕ МОДЕЛИ (для работы с БД)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Figurine {
    pub id: String,
    pub name: String,
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
    pub status: String,
    pub face_image_url: Option<String>,
    pub year: Option<i32>,
    pub sort_order: i32,
    pub series: Option<String>,
    pub is_featured: bool,
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
