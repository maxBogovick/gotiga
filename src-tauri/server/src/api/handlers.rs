use axum::{
    extract::{Path, State, Multipart, Query},
    Json,
    http::StatusCode,
    response::IntoResponse,
    body::Bytes,
};
use crate::services::AppService;
use crate::config::Config;
use crate::models::*;
use crate::error::{Result, AppError};
use uuid::Uuid;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;

fn detect_mime(bytes: &[u8], table: &str) -> &'static str {
    if table.contains("video") {
        return "video/mp4";
    }
    if table.contains("audio") || table == "figurines_audio" {
        return "audio/mpeg";
    }
    match bytes.get(..4) {
        Some([0xFF, 0xD8, 0xFF, _]) => "image/jpeg",
        Some([0x89, 0x50, 0x4E, 0x47]) => "image/png",
        Some([0x52, 0x49, 0x46, 0x46]) => "image/webp",
        _ => "application/octet-stream",
    }
}

fn media_subdir_for_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "jpg" | "jpeg" | "png" | "webp" => Some("images"),
        "mp4" | "webm" | "mov" => Some("videos"),
        "mp3" | "wav" | "ogg" | "m4a" => Some("audio"),
        _ => None,
    }
}

fn public_static_url(path: &str) -> String {
    format!("/static/{}", path)
}

fn clean_static_path(path: &str, public_url: &str) -> String {
    path.strip_prefix(public_url.trim_end_matches('/'))
        .unwrap_or(path)
        .trim_start_matches("/static/")
        .trim_start_matches('/')
        .replace('\\', "/")
}

fn replacement_subdir_for_target(path: &str) -> Option<&'static str> {
    if path.starts_with("images/") {
        Some("images")
    } else if path.starts_with("videos/") {
        Some("videos")
    } else if path.starts_with("audio/") {
        Some("audio")
    } else if path.starts_with("backgrounds/") {
        Some("backgrounds")
    } else {
        None
    }
}

async fn save_image_variants(upload_dir: &str, data: &[u8]) -> Result<serde_json::Value> {
    let id = Uuid::new_v4().to_string();
    let original_relative  = format!("images/original/{}.jpg",  id);
    let preview_relative   = format!("images/preview/{}.jpg",   id);
    let thumb_relative     = format!("images/thumb/{}.jpg",     id);
    let preview_webp       = format!("images/preview/{}.webp",  id);
    let thumb_webp         = format!("images/thumb/{}.webp",    id);

    let image = image::load_from_memory(data)
        .map_err(|e| AppError::BadRequest(format!("Invalid image file: {}", e)))?;

    let original = image.to_rgb8();
    let preview  = image.resize(1800, 1800, FilterType::Lanczos3).to_rgb8();
    let thumb    = image.resize(420, 420, FilterType::Lanczos3).to_rgb8();

    write_jpeg(upload_dir, &original_relative, &original, 95).await?;
    write_jpeg(upload_dir, &preview_relative,  &preview,  86).await?;
    write_jpeg(upload_dir, &thumb_relative,    &thumb,    78).await?;
    write_webp(upload_dir, &preview_webp,      &preview).await?;
    write_webp(upload_dir, &thumb_webp,        &thumb).await?;

    Ok(serde_json::json!({
        "url":                  public_static_url(&preview_relative),
        "relativePath":         preview_relative,
        "webpUrl":              public_static_url(&preview_webp),
        "originalUrl":          public_static_url(&original_relative),
        "originalRelativePath": original_relative,
        "thumbUrl":             public_static_url(&thumb_relative),
        "thumbRelativePath":    thumb_relative,
        "thumbWebpUrl":         public_static_url(&thumb_webp)
    }))
}

async fn write_jpeg(
    upload_dir: &str,
    relative_path: &str,
    image: &image::RgbImage,
    quality: u8,
) -> Result<()> {
    let path = std::path::Path::new(upload_dir).join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(AppError::Io)?;
    }

    let mut bytes = Vec::new();
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut bytes, quality);
        encoder.encode_image(image)
            .map_err(|e| AppError::Internal(format!("Failed to encode image: {}", e)))?;
    }

    let mut file = fs::File::create(&path).await.map_err(AppError::Io)?;
    file.write_all(&bytes).await.map_err(AppError::Io)?;
    Ok(())
}

async fn write_webp(
    upload_dir: &str,
    relative_path: &str,
    image: &image::RgbImage,
) -> Result<()> {
    let path = std::path::Path::new(upload_dir).join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(AppError::Io)?;
    }

    let mut bytes = Vec::new();
    {
        let encoder = WebPEncoder::new_lossless(&mut bytes);
        encoder.encode(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgb8,
        ).map_err(|e| AppError::Internal(format!("Failed to encode WebP: {}", e)))?;
    }

    let mut file = fs::File::create(&path).await.map_err(AppError::Io)?;
    file.write_all(&bytes).await.map_err(AppError::Io)?;
    Ok(())
}

async fn save_regular_media_file(
    upload_dir: &str,
    subdir: &str,
    ext: &str,
    data: &[u8],
) -> Result<String> {
    let media_dir = format!("{}/{}", upload_dir, subdir);
    fs::create_dir_all(&media_dir).await.map_err(AppError::Io)?;

    let file_id = Uuid::new_v4();
    let file_name = format!("{}.{}", file_id, ext);
    let full_path = format!("{}/{}", media_dir, file_name);
    let mut file = fs::File::create(&full_path).await.map_err(AppError::Io)?;
    file.write_all(data).await.map_err(AppError::Io)?;
    Ok(format!("{}/{}", subdir, file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_supported_extensions_to_media_subdirs() {
        assert_eq!(media_subdir_for_ext("jpg"), Some("images"));
        assert_eq!(media_subdir_for_ext("webp"), Some("images"));
        assert_eq!(media_subdir_for_ext("mp4"), Some("videos"));
        assert_eq!(media_subdir_for_ext("mp3"), Some("audio"));
        assert_eq!(media_subdir_for_ext("exe"), None);
    }

    #[test]
    fn builds_public_static_urls() {
        assert_eq!(
            public_static_url("images/preview/abc.jpg"),
            "/static/images/preview/abc.jpg"
        );
    }
}

// === PUBLIC READ-ONLY HANDLERS ===

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok", "version": "1.0.0" })))
}

/*pub async fn get_sync_manifest(
    State(service): State<AppService>
) -> Result<Json<Manifest>> {
    //let manifest = service.generate_manifest().await?;
    Ok(Json(manifest))
}*/

#[derive(serde::Deserialize)]
pub struct ListParams {
    visible: Option<bool>,
}

pub async fn list_figurines(
    State(service): State<AppService>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<FigurineListItemDto>>> {
    let visible_only = params.visible.unwrap_or(true);
    let list = service.list_figurines(visible_only).await?;
    Ok(Json(list))
}

pub async fn get_figurine(
    State(service): State<AppService>,
    Path(id): Path<String>, // Changed to String
) -> Result<Json<FigurineDto>> {
    let dto = service.get_figurine_details(id).await?;
    Ok(Json(dto))
}

// Combined GET dispatcher: /content/texts/:param (author | workshop)
pub async fn get_texts_by_param(
    State(service): State<AppService>,
    Path(param): Path<String>,
) -> Result<axum::response::Response> {
    match param.as_str() {
        "author" => {
            let texts = service.get_author_texts().await?;
            Ok(Json(texts).into_response())
        },
        "workshop" => {
            let items = service.get_workshop_items().await?;
            Ok(Json(items).into_response())
        },
        _ => Err(AppError::NotFound(format!("Unknown text category: {}", param))),
    }
}

pub async fn get_cabinet_zones(
    State(service): State<AppService>,
) -> Result<Json<Vec<CabinetZoneDto>>> {
    let zones = service.get_cabinet_zones().await?;
    Ok(Json(zones))
}

// === ASSET STREAMING ===

pub async fn get_asset(
    State(service): State<AppService>,
    Path((table, id)): Path<(String, String)>, // Changed to String
) -> Result<impl IntoResponse> {
    let data = service.get_asset(&table, id).await?;
    match data {
        Some(bytes) => {
            // Detect MIME from first bytes (magic numbers)
            let mime = detect_mime(&bytes, table.as_str());
            Ok((
                [(axum::http::header::CONTENT_TYPE, mime)],
                Bytes::from(bytes)
            ))
        },
        None => Err(AppError::NotFound("Asset not found".to_string()))
    }
}

// === ADMIN AUTH ===

pub async fn admin_login(
    State(config): State<Config>,
    Json(creds): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>> {
    if creds.login == config.admin_login && creds.password == config.admin_password {
        Ok(Json(serde_json::json!({ "token": config.admin_api_key })))
    } else {
        Err(AppError::Unauthorized)
    }
}

// === ADMIN FIGURINE CRUD ===

pub async fn save_figurine(
    State(service): State<AppService>,
    Json(req): Json<SaveFigurineRequest>,
) -> Result<StatusCode> {
    service.save_figurine(req).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_figurine(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    service.delete_figurine(id).await?;
    Ok(StatusCode::OK)
}

// === ADMIN MEDIA UPLOAD ===

pub async fn upload_file(
    State(_service): State<AppService>,
    State(config): State<Config>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("file").to_string();
            let ext = std::path::Path::new(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
                .to_lowercase();
            let data = field.bytes().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

            let subdir = media_subdir_for_ext(ext.as_str())
                .ok_or_else(|| AppError::BadRequest(format!("Unsupported media extension: {}", ext)))?;
            if subdir == "images" {
                let payload = save_image_variants(&config.upload_dir, &data).await?;
                return Ok(Json(payload));
            }

            let media_dir = format!("{}/{}", config.upload_dir, subdir);
            fs::create_dir_all(&media_dir).await.map_err(AppError::Io)?;

            let file_id = Uuid::new_v4();
            let file_name = format!("{}.{}", file_id, ext);
            let full_path = format!("{}/{}", media_dir, file_name);
            let mut file = fs::File::create(&full_path).await.map_err(AppError::Io)?;
            file.write_all(&data).await.map_err(AppError::Io)?;

            let relative_path = format!("{}/{}", subdir, file_name);
            let url = public_static_url(&relative_path);
            return Ok(Json(serde_json::json!({
                "url": url,
                "relativePath": relative_path
            })));
        }
    }
    Err(AppError::BadRequest("No file field found".to_string()))
}

pub async fn get_media_inventory(
    State(service): State<AppService>,
) -> Result<Json<MediaInventoryDto>> {
    Ok(Json(service.media_inventory().await?))
}

pub async fn get_unused_media_report(
    State(service): State<AppService>,
) -> Result<Json<MediaCleanupReportDto>> {
    Ok(Json(service.unused_media_report().await?))
}

pub async fn cleanup_unused_media(
    State(service): State<AppService>,
) -> Result<Json<serde_json::Value>> {
    let removed = service.cleanup_unused_media().await?;
    Ok(Json(serde_json::json!({ "removed": removed })))
}

pub async fn replace_media_everywhere(
    State(service): State<AppService>,
    State(config): State<Config>,
    mut multipart: Multipart,
) -> Result<Json<MediaReplaceResultDto>> {
    let mut target_path: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_data: Option<Bytes> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name == "targetPath" {
            let value = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            target_path = Some(clean_static_path(&value, &config.public_url));
        } else if name == "file" {
            file_name = Some(field.file_name().unwrap_or("file").to_string());
            file_data = Some(field.bytes().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?);
        }
    }

    let target_path = target_path.ok_or_else(|| AppError::BadRequest("Missing targetPath".to_string()))?;
    let file_name = file_name.ok_or_else(|| AppError::BadRequest("Missing file".to_string()))?;
    let data = file_data.ok_or_else(|| AppError::BadRequest("Missing file data".to_string()))?;
    let target_subdir = replacement_subdir_for_target(&target_path)
        .ok_or_else(|| AppError::BadRequest(format!("Unsupported managed media path: {}", target_path)))?;
    let ext = std::path::Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_lowercase();

    let result = if target_subdir == "images" {
        if media_subdir_for_ext(ext.as_str()) != Some("images") {
            return Err(AppError::BadRequest(format!("Replacement must be an image, got {}", ext)));
        }
        let payload = save_image_variants(&config.upload_dir, &data).await?;
        let new_path = payload.get("relativePath")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Internal("Missing replacement relativePath".to_string()))?;
        let original_path = payload.get("originalRelativePath").and_then(|v| v.as_str());
        let thumb_path = payload.get("thumbRelativePath").and_then(|v| v.as_str());
        service.replace_media_everywhere(&target_path, new_path, original_path, thumb_path).await?
    } else {
        let expected_subdir = media_subdir_for_ext(ext.as_str())
            .ok_or_else(|| AppError::BadRequest(format!("Unsupported replacement extension: {}", ext)))?;
        if target_subdir != "backgrounds" && target_subdir != expected_subdir {
            return Err(AppError::BadRequest(format!("Replacement type does not match target {}", target_path)));
        }
        let new_path = save_regular_media_file(&config.upload_dir, target_subdir, &ext, &data).await?;
        service.replace_media_everywhere(&target_path, &new_path, None, None).await?
    };

    Ok(Json(result))
}

// === ADMIN ZONE CRUD ===

pub async fn save_zone(
    State(service): State<AppService>,
    Json(req): Json<SaveZoneRequest>,
) -> Result<StatusCode> {
    service.save_zone(req).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_zone(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    service.delete_zone(id).await?;
    Ok(StatusCode::OK)
}

// === ADMIN TEXT CRUD ===

pub async fn save_text(
    State(service): State<AppService>,
    Path(param): Path<String>,
    Json(req): Json<SaveTextRequest>,
) -> Result<StatusCode> {
    let cat = match param.as_str() {
        "author" => TextCategory::Author,
        "workshop" => TextCategory::Workshop,
        _ => return Err(AppError::BadRequest(format!("Unknown category: {}", param))),
    };
    service.save_text(cat, req).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_text(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    service.delete_text_item(id).await?;
    Ok(StatusCode::OK)
}

// === ADMIN BACKGROUND ===

pub async fn get_main_background(
    State(service): State<AppService>,
) -> Result<Json<serde_json::Value>> {
    let url = service.get_background().await?;
    Ok(Json(serde_json::json!({ "url": url })))
}

pub async fn upload_main_background(
    State(service): State<AppService>,
    State(config): State<Config>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("bg.jpg").to_string();
            let ext = std::path::Path::new(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("jpg")
                .to_lowercase();
            if media_subdir_for_ext(ext.as_str()) != Some("images") {
                return Err(AppError::BadRequest(format!("Unsupported background extension: {}", ext)));
            }
            let data = field.bytes().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

            let bg_dir = format!("{}/backgrounds", config.upload_dir);
            fs::create_dir_all(&bg_dir).await.map_err(AppError::Io)?;

            let file_name = format!("cabinet-bg.{}", ext);
            let full_path = format!("{}/{}", bg_dir, file_name);
            let mut file = fs::File::create(&full_path).await.map_err(AppError::Io)?;
            file.write_all(&data).await.map_err(AppError::Io)?;

            let relative = format!("/static/backgrounds/{}", file_name);
            service.set_background(relative.clone()).await?;
            let public_url = service.get_background().await?.unwrap_or(relative);
            return Ok(Json(serde_json::json!({ "url": public_url })));
        }
    }
    Err(AppError::BadRequest("No file field found".to_string()))
}

pub async fn get_home_content(
    State(service): State<AppService>,
) -> Result<Json<HomeContent>> {
    let content = service.get_home_content().await?;
    Ok(Json(content))
}

pub async fn save_home_content(
    State(service): State<AppService>,
    Json(content): Json<HomeContent>,
) -> Result<StatusCode> {
    service.save_home_content(content).await?;
    Ok(StatusCode::OK)
}

// === ADMIN / RELEASE MANAGEMENT ===

pub async fn upload_release_db(
    State(service): State<AppService>,
    State(config): State<crate::config::Config>,
    mut multipart: Multipart,
) -> Result<StatusCode> {
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let data = field.bytes().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            
            let release_id = Uuid::new_v4();
            let file_name = format!("release_{}.db", release_id);
            let save_dir = format!("{}/releases", config.upload_dir);
            fs::create_dir_all(&save_dir).await.map_err(AppError::Io)?;
            
            let full_path = format!("{}/{}", save_dir, file_name);
            let mut file = fs::File::create(&full_path).await.map_err(AppError::Io)?;
            file.write_all(&data).await.map_err(AppError::Io)?;
            
            // Register and Hot Swap
            service.register_new_release(&full_path).await?;
            
            return Ok(StatusCode::OK);
        }
    }
    Err(AppError::BadRequest("No file field found".to_string()))
}

pub async fn download_release_db(
    State(service): State<AppService>,
) -> Result<impl IntoResponse> {
    let path = service.get_active_release_path().await?
        .ok_or_else(|| AppError::NotFound("No active release".to_string()))?;

    if !std::path::Path::new(&path).exists() {
        return Err(AppError::NotFound("Release file missing on disk".to_string()));
    }

    let file = fs::read(&path).await.map_err(AppError::Io)?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/x-sqlite3")],
        [(axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"latest.db\"")],
        file
    ))
}

pub async fn list_releases(
    State(service): State<AppService>,
) -> Result<Json<Vec<Release>>> {
    let releases = service.list_releases().await?;
    Ok(Json(releases))
}

pub async fn switch_release(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.switch_to_release(id).await?;
    Ok(StatusCode::OK)
}

// === AUTHOR PROFILE ===

pub async fn get_author_profile(
    State(service): State<AppService>,
) -> Result<Json<crate::models::AuthorProfile>> {
    let profile = service.get_author_profile().await?;
    Ok(Json(profile))
}

pub async fn save_author_profile(
    State(service): State<AppService>,
    Json(profile): Json<crate::models::AuthorProfile>,
) -> Result<StatusCode> {
    service.save_author_profile(profile).await?;
    Ok(StatusCode::OK)
}

// === ORDERS ===

pub async fn create_order(
    State(service): State<AppService>,
    Json(order): Json<crate::models::OrderRequest>,
) -> Result<StatusCode> {
    // Fire-and-forget — never fail the request due to notification errors
    let _ = service.send_order_notification(&order).await;
    Ok(StatusCode::OK)
}
