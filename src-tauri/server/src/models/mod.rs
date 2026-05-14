use serde::{Deserialize, Serialize};
use sqlx::prelude::Type;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ============================================================
// ENUMS
// ============================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "figurine_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum FigurineStatus {
    Available,
    Sold,
    Reserved,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "text_category", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TextCategory {
    Author,
    Workshop,
}

// ============================================================
// ENTITIES (DB MAPPING)
// SQLite stores timestamps as TEXT ("YYYY-MM-DD HH:MM:SS"),
// so we use String instead of DateTime<Utc> for SQLite entities.
// ============================================================

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
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
    pub is_visible: bool,
    pub status: FigurineStatus,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub figurine_id: String,
    pub image_type: ImageType,
    pub file_path: String,
    pub alt_text: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ProcessStep {
    pub id: String,
    pub figurine_id: String,
    pub step_type: StepType,
    pub description: Option<String>,
    pub image_path: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Text {
    pub id: String,
    pub category: TextCategory,
    pub content: String,
    pub caption: Option<String>,
    pub image_path: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct CabinetZone {
    pub id: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDto {
    pub id: String,
    pub image_type: ImageType,
    pub url: String,
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
}
