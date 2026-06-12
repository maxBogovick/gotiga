use axum::{
    extract::{Path, State, Multipart, Query},
    Json,
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    body::Bytes,
};
use crate::services::AppService;
use crate::config::Config;
use crate::models::*;
use crate::error::{Result, AppError};
use uuid::Uuid;

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers.get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}
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
    Path(id): Path<String>,
) -> Result<Json<FigurineDto>> {
    let dto = service.get_figurine_details(id).await?;
    Ok(Json(dto))
}

pub async fn list_in_progress_figurines(
    State(service): State<AppService>,
) -> Result<Json<Vec<FigurineListItemDto>>> {
    let list = service.list_in_progress_figurines().await?;
    Ok(Json(list))
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

// Content is now managed directly in PostgreSQL via the admin UI.
// This endpoint is kept for compatibility but no longer imports SQLite data.
pub async fn upload_release_db(
    State(_service): State<AppService>,
    mut multipart: Multipart,
) -> Result<StatusCode> {
    // Drain the multipart to avoid connection errors
    while let Some(_field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {}
    Ok(StatusCode::OK)
}

// Content is now stored in PostgreSQL — no SQLite file to download.
pub async fn download_release_db(
    State(_service): State<AppService>,
) -> Result<impl IntoResponse> {
    Err::<axum::response::Response, _>(AppError::NotFound(
        "Content is now managed in PostgreSQL; SQLite export is no longer available".to_string()
    ))
}

pub async fn list_releases(
    State(service): State<AppService>,
) -> Result<Json<Vec<Release>>> {
    let releases = service.list_releases().await?;
    Ok(Json(releases))
}

pub async fn switch_release(
    State(_service): State<AppService>,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode> {
    // No-op in server-primary mode — content.db is always the master
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
    service.create_order(&order).await?;
    Ok(StatusCode::CREATED)
}

pub async fn list_orders(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<crate::models::OrdersPage>> {
    let status = params.get("status").map(|s| s.as_str());
    let page = params.get("page").and_then(|p| p.parse::<i64>().ok()).unwrap_or(1).max(1);
    let per_page = params.get("perPage").and_then(|p| p.parse::<i64>().ok()).unwrap_or(20).clamp(1, 100);
    let page_data = service.list_orders(status, page, per_page).await?;
    Ok(Json(page_data))
}

pub async fn update_order_status(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::models::UpdateOrderStatusRequest>,
) -> Result<StatusCode> {
    service.update_order_status(id, &body.status).await?;
    Ok(StatusCode::OK)
}

// === COMMISSIONS ===

fn looks_like_email(s: &str) -> bool {
    let s = s.trim();
    match s.split_once('@') {
        Some((local, domain)) => !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.'),
        None => false,
    }
}

pub async fn create_commission(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(req): Json<crate::models::CommissionRequest>,
) -> Result<Json<crate::models::CommissionCreatedResponse>> {
    // Honeypot: real users never fill `website`. Pretend success, save nothing.
    if req.website.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false) {
        return Ok(Json(crate::models::CommissionCreatedResponse {
            id: Uuid::new_v4().to_string(),
            claim_token: Uuid::new_v4().to_string().replace('-', ""),
        }));
    }

    let ip = extract_ip(&headers);
    service.check_commission_rate_limit(&ip).await?;

    if !looks_like_email(&req.requester_email) {
        return Err(AppError::BadRequest("A valid email is required.".into()));
    }
    if req.description.trim().is_empty() {
        return Err(AppError::BadRequest("Description is required.".into()));
    }
    if req.description.chars().count() > 5000 {
        return Err(AppError::BadRequest("Description is too long.".into()));
    }
    if req.attachment_urls.len() > 5 {
        return Err(AppError::BadRequest("Too many attachments.".into()));
    }

    // If the requester is logged in, attach their account immediately.
    let user = if let Some(token) = bearer_token(&headers) {
        service.get_user_from_session(token).await.ok()
    } else {
        None
    };

    let created = service.create_commission(&req).await?;
    if let Some(u) = user {
        let _ = service.claim_commission(&created.claim_token, u.id).await;
    }
    Ok(Json(created))
}

pub async fn get_commission_by_token(
    State(service): State<AppService>,
    Path(token): Path<String>,
) -> Result<Json<crate::models::CommissionDto>> {
    service.get_commission_by_token(&token).await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Commission not found".to_string()))
}

pub async fn user_claim_commission(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<crate::models::ClaimCommissionRequest>,
) -> Result<Json<crate::models::CommissionDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(service.claim_commission(&body.claim_token, user.id).await?))
}

pub async fn user_list_commissions(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let commissions = service.get_user_commissions(user.id).await?;
    Ok(Json(serde_json::json!({ "commissions": commissions })))
}

pub async fn user_edit_commission(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::models::EditCommissionRequest>,
) -> Result<Json<crate::models::CommissionDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(service.edit_commission(id, Some(user.id), &body).await?))
}

pub async fn user_delete_commission(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    service.delete_commission(id, Some(user.id)).await?;
    Ok(StatusCode::OK)
}

/// Authenticated media upload for ordinary users (commission references, chat
/// attachments). Mirrors the admin /upload but gated on a user session and
/// restricted to images.
pub async fn user_upload_file(
    State(service): State<AppService>,
    State(config): State<Config>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let _user = service.get_user_from_session(token).await?;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        if field.name().unwrap_or("") == "file" {
            let filename = field.file_name().unwrap_or("file").to_string();
            let ext = std::path::Path::new(&filename)
                .extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let data = field.bytes().await
                .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

            if data.len() > 8 * 1024 * 1024 {
                return Err(AppError::BadRequest("Image is too large (max 8MB).".into()));
            }
            if media_subdir_for_ext(&ext) != Some("images") {
                return Err(AppError::BadRequest("Only image uploads are allowed.".into()));
            }
            let payload = save_image_variants(&config.upload_dir, &data).await?;
            return Ok(Json(payload));
        }
    }
    Err(AppError::BadRequest("No file field found".to_string()))
}

pub async fn admin_list_commissions(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<crate::models::CommissionsPage>> {
    let status = params.get("status").map(|s| s.as_str());
    let page = params.get("page").and_then(|p| p.parse::<i64>().ok()).unwrap_or(1).max(1);
    let per_page = params.get("perPage").and_then(|p| p.parse::<i64>().ok()).unwrap_or(20).clamp(1, 100);
    Ok(Json(service.list_commissions(status, page, per_page).await?))
}

pub async fn admin_update_commission(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::models::UpdateCommissionStatusRequest>,
) -> Result<Json<crate::models::CommissionDto>> {
    service.update_commission(id, &body.status, body.admin_notes.as_deref(), body.figurine_id.as_deref()).await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Commission not found".to_string()))
}

pub async fn admin_delete_commission(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.delete_commission(id, None).await?;
    Ok(StatusCode::OK)
}

// === SCHEDULE & BOOKINGS (PUBLIC) ===

pub async fn get_figurine_schedule(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<Json<crate::models::FigurineScheduleDto>> {
    let schedule = service.get_figurine_schedule(id).await?;
    Ok(Json(schedule))
}

pub async fn create_booking(
    State(service): State<AppService>,
    Path(_id): Path<String>,
    Json(req): Json<crate::models::CreateBookingRequest>,
) -> Result<Json<crate::models::BookingCreatedResponse>> {
    let booking = service.create_booking(req).await?;
    Ok(Json(crate::models::BookingCreatedResponse { cancel_token: booking.cancel_token }))
}

pub async fn get_booking_by_token(
    State(service): State<AppService>,
    Path(token): Path<String>,
) -> Result<Json<crate::models::BookingCancelInfo>> {
    service.get_booking_by_token(&token).await?
        .map(Json)
        .ok_or_else(|| crate::error::AppError::NotFound("Booking not found".to_string()))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

/// Public sitemap. Absolute URLs are built from the forwarded Host/proto headers so the
/// same binary works across environments without a hard-coded domain.
pub async fn sitemap_xml(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let host = headers.get("host").and_then(|h| h.to_str().ok()).unwrap_or("localhost");
    let proto = headers.get("x-forwarded-proto").and_then(|h| h.to_str().ok()).unwrap_or("https");
    let base = format!("{proto}://{host}");

    let figurines = service.list_figurines(true).await?;

    let mut urls = String::new();
    for path in ["/", "/figurines", "/author", "/workshop", "/upcoming"] {
        urls.push_str(&format!("  <url><loc>{base}{path}</loc></url>\n"));
    }
    for f in &figurines {
        urls.push_str(&format!(
            "  <url><loc>{base}/figurines/{}</loc></url>\n",
            xml_escape(&f.id)
        ));
    }

    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{urls}</urlset>\n"
    );
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        body,
    ))
}

pub async fn get_bookings_by_tokens(
    State(service): State<AppService>,
    Json(req): Json<crate::models::BookingsByTokensRequest>,
) -> Result<Json<std::collections::HashMap<String, crate::models::BookingCancelInfo>>> {
    // Cap the batch to bound the query and guard against abuse.
    let tokens: Vec<String> = req.tokens.into_iter().take(100).collect();
    if tokens.is_empty() {
        return Ok(Json(std::collections::HashMap::new()));
    }
    Ok(Json(service.get_bookings_by_tokens(&tokens).await?))
}

pub async fn cancel_booking_by_token(
    State(service): State<AppService>,
    Path(token): Path<String>,
) -> Result<StatusCode> {
    service.cancel_booking_by_token(&token).await?;
    Ok(StatusCode::OK)
}

// === SHOWINGS (ADMIN) ===

pub async fn list_showings(
    State(service): State<AppService>,
) -> Result<Json<Vec<crate::models::ShowingDto>>> {
    Ok(Json(service.list_showings().await?))
}

pub async fn save_showing(
    State(service): State<AppService>,
    Json(req): Json<crate::models::SaveShowingRequest>,
) -> Result<Json<crate::models::ShowingDto>> {
    Ok(Json(service.save_showing(req).await?))
}

pub async fn delete_showing(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    service.delete_showing(id).await?;
    Ok(StatusCode::OK)
}

// === BOOKINGS (ADMIN) ===

pub async fn list_bookings(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<crate::models::BookingsPage>> {
    let status = params.get("status").map(|s| s.as_str());
    let figurine_id = params.get("figurineId")
        .and_then(|s| Uuid::parse_str(s).ok());
    let page = params.get("page").and_then(|p| p.parse::<i64>().ok()).unwrap_or(1).max(1);
    let per_page = params.get("perPage").and_then(|p| p.parse::<i64>().ok()).unwrap_or(20).clamp(1, 100);
    Ok(Json(service.list_bookings(status, figurine_id, page, per_page).await?))
}

pub async fn update_booking_status(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::models::UpdateBookingStatusRequest>,
) -> Result<StatusCode> {
    service.update_booking_status(id, body.status, body.admin_notes, body.curator_conditions).await?;
    Ok(StatusCode::OK)
}

// ============================================================
// USER AUTH HANDLERS
// ============================================================

pub async fn user_register(
    State(service): State<AppService>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>> {
    let user = service.register_user(&body).await?;
    Ok(Json(serde_json::json!({ "user": user })))
}

pub async fn user_login_challenge(
    State(service): State<AppService>,
    Json(body): Json<LoginChallengeRequest>,
) -> Result<Json<LoginChallengeResponse>> {
    let response = service.login_challenge(&body.email).await?;
    Ok(Json(response))
}

pub async fn user_login_verify(
    State(service): State<AppService>,
    Json(body): Json<LoginVerifyRequest>,
) -> Result<Json<LoginVerifyResponse>> {
    let response = service.login_verify(&body).await?;
    Ok(Json(response))
}

pub async fn user_logout(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<StatusCode> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    service.logout(token).await?;
    Ok(StatusCode::OK)
}

pub async fn user_me(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<UserDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(UserDto::from(&user)))
}

pub async fn user_link_bookings(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<LinkBookingsRequest>,
) -> Result<Json<serde_json::Value>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let linked = service.link_bookings(user.id, &body.cancel_tokens).await?;
    Ok(Json(serde_json::json!({ "linked": linked })))
}

pub async fn user_profile_bookings(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserBookingDto>>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let bookings = service.get_user_bookings(user.id).await?;
    Ok(Json(bookings))
}

pub async fn user_profile_orders(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserOrderDto>>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let orders = service.get_user_orders(user.id).await?;
    Ok(Json(orders))
}

// ============================================================
// ADMIN USER MANAGEMENT HANDLERS
// ============================================================

pub async fn admin_list_users(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let search = params.get("search").map(|s| s.as_str());
    let page = params.get("page").and_then(|p| p.parse::<i64>().ok()).unwrap_or(1).max(1);
    let per_page = params.get("perPage").and_then(|p| p.parse::<i64>().ok()).unwrap_or(20).clamp(1, 100);
    let (items, total) = service.admin_list_users(search, page, per_page).await?;
    Ok(Json(serde_json::json!({ "items": items, "total": total, "page": page, "perPage": per_page })))
}

pub async fn admin_get_user(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<Json<AdminUserDetail>> {
    let detail = service.admin_get_user_detail(id).await?;
    Ok(Json(detail))
}

pub async fn admin_revoke_user_sessions(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let revoked = service.admin_revoke_user_sessions(id).await?;
    Ok(Json(serde_json::json!({ "revoked": revoked })))
}

pub async fn admin_update_user_notes(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUserAdminNotesRequest>,
) -> Result<StatusCode> {
    service.admin_update_user_notes(id, body.admin_notes.as_deref()).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_set_user_blocked(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<SetUserBlockedRequest>,
) -> Result<StatusCode> {
    service.admin_set_user_blocked(id, body.blocked).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_generate_reset_token(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<Json<ResetTokenResponse>> {
    let resp = service.admin_generate_reset_token(id).await?;
    Ok(Json(resp))
}

pub async fn validate_reset_token(
    State(service): State<AppService>,
    Path(token): Path<String>,
) -> Result<Json<UserDto>> {
    let user = service.validate_reset_token(&token).await?;
    Ok(Json(user))
}

pub async fn apply_password_reset(
    State(service): State<AppService>,
    Json(body): Json<ApplyPasswordResetRequest>,
) -> Result<StatusCode> {
    service.apply_password_reset(&body).await?;
    Ok(StatusCode::OK)
}

// ============================================================
// COMMENTS
// ============================================================

fn extract_ip(headers: &HeaderMap) -> String {
    headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub async fn get_figurine_comments(
    State(service): State<AppService>,
    Path(figurine_id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<CommentDto>>> {
    let newest_first = params.get("sort").map(|v| v == "newest").unwrap_or(false);
    let comments = service.get_figurine_comments(figurine_id, newest_first).await?;
    Ok(Json(comments))
}

pub async fn submit_comment(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(figurine_id): Path<Uuid>,
    Json(body): Json<SubmitCommentRequest>,
) -> Result<StatusCode> {
    let user = if let Some(token) = bearer_token(&headers) {
        service.get_user_from_session(token).await.ok()
    } else {
        None
    };
    let ip = extract_ip(&headers);
    service.submit_comment(figurine_id, user.as_ref(), &body, &ip).await?;
    Ok(StatusCode::CREATED)
}

pub async fn admin_list_comments(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AdminCommentsPage>> {
    let only_pending = params.get("pending").map(|v| v == "true").unwrap_or(false);
    let newest_first = params.get("sort").map(|v| v == "newest").unwrap_or(true);
    let figurine_filter = params.get("figurineId")
        .and_then(|v| Uuid::parse_str(v).ok());
    let page = params.get("page").and_then(|p| p.parse::<i64>().ok()).unwrap_or(1).max(1);
    let per_page = params.get("perPage").and_then(|p| p.parse::<i64>().ok()).unwrap_or(20).clamp(1, 100);
    let page_data = service.admin_list_comments(only_pending, figurine_filter, newest_first, page, per_page).await?;
    Ok(Json(page_data))
}

pub async fn admin_moderate_comment(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<ModerateCommentRequest>,
) -> Result<Json<AdminCommentDto>> {
    let comment = service.admin_moderate_comment(id, body.is_approved, body.admin_reply.as_deref()).await?;
    Ok(Json(comment))
}

pub async fn admin_delete_comment(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.admin_delete_comment(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_get_smtp_settings(
    State(service): State<AppService>,
) -> Result<Json<SmtpSettings>> {
    let settings = service.get_smtp_settings().await?;
    Ok(Json(settings))
}

pub async fn admin_save_smtp_settings(
    State(service): State<AppService>,
    Json(body): Json<SmtpSettings>,
) -> Result<Json<SmtpSettings>> {
    service.save_smtp_settings(body.clone()).await?;
    Ok(Json(body))
}

pub async fn get_contact_settings(
    State(service): State<AppService>,
) -> Result<Json<ContactSettings>> {
    Ok(Json(service.get_contact_settings().await?))
}

pub async fn admin_save_contact_settings(
    State(service): State<AppService>,
    Json(body): Json<ContactSettings>,
) -> Result<Json<ContactSettings>> {
    service.save_contact_settings(body.clone()).await?;
    Ok(Json(body))
}

// === THEME CONFIG ===

pub async fn get_theme_config(
    State(service): State<AppService>,
) -> Result<Json<ThemeConfig>> {
    Ok(Json(service.get_theme_config().await?))
}

pub async fn save_theme_config(
    State(service): State<AppService>,
    Json(body): Json<ThemeConfig>,
) -> Result<Json<ThemeConfig>> {
    service.save_theme_config(body.clone()).await?;
    Ok(Json(body))
}

// === COPY OVERRIDES ===

pub async fn get_copy_overrides(
    State(service): State<AppService>,
) -> Result<Json<CopyOverrides>> {
    Ok(Json(service.get_copy_overrides().await?))
}

pub async fn save_copy_overrides(
    State(service): State<AppService>,
    Json(body): Json<CopyOverrides>,
) -> Result<Json<CopyOverrides>> {
    service.save_copy_overrides(body.clone()).await?;
    Ok(Json(body))
}

pub async fn user_update_profile(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<UserDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let updated = service.update_profile(user.id, &body.display_name).await?;
    Ok(Json(updated))
}

pub async fn user_upload_avatar(
    State(service): State<AppService>,
    State(config): State<Config>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<UserDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        if field.name().unwrap_or("") == "file" {
            let data = field.bytes().await
                .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

            let img = image::load_from_memory(&data)
                .map_err(|e| AppError::BadRequest(format!("Invalid image: {}", e)))?;
            let thumb = img.resize_to_fill(200, 200, FilterType::Lanczos3).to_rgb8();

            let id = Uuid::new_v4();
            let avatar_dir = format!("{}/avatars", config.upload_dir);
            fs::create_dir_all(&avatar_dir).await.map_err(AppError::Io)?;

            let file_path = format!("{}/{}.jpg", avatar_dir, id);
            let mut buf = Vec::new();
            {
                let mut encoder = JpegEncoder::new_with_quality(&mut buf, 88);
                encoder.encode_image(&thumb)
                    .map_err(|e| AppError::Internal(format!("Encode error: {}", e)))?;
            }
            fs::write(&file_path, &buf).await.map_err(AppError::Io)?;

            let url = format!("/static/avatars/{}.jpg", id);
            let updated = service.set_user_avatar(user.id, &url).await?;
            return Ok(Json(updated));
        }
    }
    Err(AppError::BadRequest("No file field found".to_string()))
}

pub async fn user_delete_account(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<StatusCode> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    service.delete_account(user.id).await?;
    Ok(StatusCode::OK)
}

// === MESSAGING THREADS ===

pub async fn user_get_threads(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let threads = service.get_user_threads(user.id).await?;
    let unread = service.count_unread_threads(user.id).await?;
    Ok(Json(serde_json::json!({ "threads": threads, "unread": unread })))
}

pub async fn user_get_thread(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ThreadDetailDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(service.get_thread_detail(id, user.id).await?))
}

pub async fn user_create_thread(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<CreateThreadRequest>,
) -> Result<Json<ThreadDetailDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(service.user_create_thread(user.id, body.subject, body.body, body.category, body.attachment_urls).await?))
}

pub async fn user_reply_to_thread(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<ReplyToThreadRequest>,
) -> Result<Json<ThreadMessageDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(service.user_reply_to_thread(id, user.id, body.body, body.attachment_urls).await?))
}

pub async fn admin_list_threads(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let category = params.get("category").cloned();
    let status = params.get("status").cloned();
    let page = params.get("page").and_then(|p| p.parse::<i64>().ok()).unwrap_or(1).max(1);
    let per_page = params.get("perPage").and_then(|p| p.parse::<i64>().ok()).unwrap_or(25).clamp(1, 100);
    Ok(Json(service.admin_list_threads(category, status, page, per_page).await?))
}

pub async fn admin_get_thread(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<Json<ThreadDetailDto>> {
    Ok(Json(service.admin_get_thread_detail(id).await?))
}

pub async fn admin_create_thread_for_user(
    State(service): State<AppService>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<CreateThreadRequest>,
) -> Result<Json<ThreadDetailDto>> {
    Ok(Json(service.admin_create_thread(user_id, body.subject, body.body, body.category, None, body.attachment_urls).await?))
}

pub async fn admin_reply_to_thread(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReplyToThreadRequest>,
) -> Result<Json<ThreadMessageDto>> {
    Ok(Json(service.admin_reply_to_thread(id, body.body, body.attachment_urls).await?))
}

pub async fn admin_resolve_thread(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.admin_resolve_thread(id).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_reopen_thread(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.admin_reopen_thread(id).await?;
    Ok(StatusCode::OK)
}

// === BOOKING RULES ===

pub async fn get_booking_rules(
    State(service): State<AppService>,
) -> Result<Json<BookingRules>> {
    Ok(Json(service.get_booking_rules().await?))
}

pub async fn save_booking_rules(
    State(service): State<AppService>,
    Json(body): Json<BookingRules>,
) -> Result<StatusCode> {
    service.save_booking_rules(body).await?;
    Ok(StatusCode::OK)
}

// === RESCHEDULE BOOKING ===

pub async fn reschedule_booking_by_token(
    State(service): State<AppService>,
    Path(token): Path<String>,
    Json(body): Json<RescheduleBookingRequest>,
) -> Result<Json<BookingCancelInfo>> {
    Ok(Json(service.reschedule_booking_by_token(&token, body).await?))
}

// === WAITLIST ===

pub async fn join_waitlist(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<CreateWaitlistRequest>,
) -> Result<StatusCode> {
    let user_id = if let Some(token) = bearer_token(&headers) {
        service.get_user_from_session(token).await.ok().map(|u| u.id)
    } else {
        None
    };
    service.join_waitlist(id, body, user_id).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_notify_waitlist(
    State(service): State<AppService>,
    Path(figurine_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(service.admin_notify_waitlist(figurine_id).await?))
}

pub async fn admin_list_waitlist(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<WaitlistEntryDto>>> {
    let figurine_id = params.get("figurineId").cloned();
    Ok(Json(service.list_waitlist_admin(figurine_id).await?))
}

pub async fn admin_remove_from_waitlist(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.remove_waitlist_entry(id).await?;
    Ok(StatusCode::OK)
}
