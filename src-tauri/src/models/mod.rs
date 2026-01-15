use serde::{Deserialize, Serialize};

// ============================================================
// ВНУТРЕННИЕ МОДЕЛИ (для работы с БД)
// ============================================================

#[derive(Debug, Clone)]
pub struct Figurine {
    pub id: String,
    pub name: String,
    pub short_text: Option<String>,
    pub year: Option<i32>,
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

// ============================================================
// DTO (для передачи на frontend, сериализуемые)
// ============================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FigurineDto {
    pub id: String,
    pub name: String,
    pub short_text: Option<String>,
    pub year: Option<i32>,
    pub status: String,
    pub images: Vec<ImageDto>,
}

impl FigurineDto {
    pub fn from_figurine(figurine: Figurine, images: Vec<Image>) -> Self {
        Self {
            id: figurine.id,
            name: figurine.name,
            short_text: figurine.short_text,
            year: figurine.year,
            status: figurine.status.as_str().to_string(),
            images: images.into_iter().map(ImageDto::from).collect(),
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
