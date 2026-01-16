use serde::{Deserialize, Serialize};

// ============================================================
// ВНУТРЕННИЕ МОДЕЛИ (для работы с БД)
// ============================================================

#[derive(Debug, Clone)]
pub struct Figurine {
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum FigurineStatus {
    Available,
    Sold,
    Reserved,
}

impl FigurineStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "sold" => Self::Sold,
            "reserved" => Self::Reserved,
            _ => Self::Available,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Sold => "sold",
            Self::Reserved => "reserved",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub id: String,
    pub figurine_id: String,
    pub image_type: ImageType,
    pub file_path: String,
    pub alt_text: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
pub struct ProcessStep {
    pub id: String,
    pub figurine_id: String,
    pub step_type: ProcessStepType,
    pub description: Option<String>,
    pub image_path: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq)]
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
            "sketch" => Self::Sketch,
            "prototype" => Self::Prototype,
            "modeling" => Self::Modeling,
            "painting" => Self::Painting,
            _ => Self::Finish,
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

// ============================================================
// DTO (для передачи на frontend, сериализуемые)
// ============================================================

#[derive(Debug, Clone, Serialize)]
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
    pub status: String,
    pub images: Vec<ImageDto>,
    pub process_steps: Vec<ProcessStepDto>,
    pub related_items: Vec<FigurineListItemDto>,
}

impl FigurineDto {
    pub fn from_figurine(
        figurine: Figurine, 
        images: Vec<Image>, 
        process_steps: Vec<ProcessStep>,
        related_items: Vec<FigurineListItemDto>
    ) -> Self {
        Self {
            id: figurine.id,
            name: figurine.name,
            short_text: figurine.short_text,
            full_description: figurine.full_description,
            dimensions: figurine.dimensions,
            material: figurine.material,
            technique: figurine.technique,
            year: figurine.year,
            ambience_path: figurine.ambience_path,
            video_url: figurine.video_url,
            secret_text: figurine.secret_text,
            status: figurine.status.as_str().to_string(),
            images: images.into_iter().map(ImageDto::from).collect(),
            process_steps: process_steps.into_iter().map(ProcessStepDto::from).collect(),
            related_items,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessStepDto {
    pub id: String,
    pub step_type: String,
    pub description: Option<String>,
    pub image_url: String,
}

impl From<ProcessStep> for ProcessStepDto {
    fn from(step: ProcessStep) -> Self {
        let image_url = if step.image_path.starts_with("http") {
            step.image_path
        } else {
            format!("asset://localhost/{}", step.image_path)
        };
        Self {
            id: step.id,
            step_type: step.step_type.as_str().to_string(),
            description: step.description,
            image_url,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDto {
    pub id: String,
    pub image_type: String,
    pub url: String,  // путь для frontend (asset://)
    pub alt_text: Option<String>,
}

impl From<Image> for ImageDto {
    fn from(image: Image) -> Self {
        let url = if image.file_path.starts_with("http") {
            image.file_path
        } else {
            format!("asset://localhost/{}", image.file_path)
        };
        Self {
            id: image.id,
            image_type: image.image_type.as_str().to_string(),
            url,
            alt_text: image.alt_text,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineListItemDto {
    pub id: String,
    pub name: String,
    pub status: String,
    pub face_image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItemDto {
    pub id: String,
    pub content: String,
    pub caption: Option<String>,
    pub image_url: Option<String>,
}

impl From<Text> for WorkshopItemDto {
    fn from(text: Text) -> Self {
        let image_url = text.image_path.map(|p| {
            if p.starts_with("http") {
                p
            } else {
                format!("asset://localhost/{}", p)
            }
        });
        Self {
            id: text.id,
            content: text.content,
            caption: text.caption,
            image_url,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
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
