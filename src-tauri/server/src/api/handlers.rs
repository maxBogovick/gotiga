use axum::{
    extract::{Path, State, Multipart, Query},
    Json,
    http::StatusCode,
};
use crate::services::AppService;
use crate::models::*;
use crate::error::{Result, AppError};
use uuid::Uuid;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use axum::response::IntoResponse;

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok", "version": "1.0.0" })))
}

pub async fn get_sync_manifest(
    State(service): State<AppService>
) -> Result<Json<Manifest>> {
    let manifest = service.generate_manifest().await?;
    Ok(Json(manifest))
}

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
    Path(id): Path<Uuid>,
) -> Result<Json<FigurineDto>> {
    let dto = service.get_figurine_details(id).await?;
    Ok(Json(dto))
}

pub async fn get_author_texts(
    State(service): State<AppService>,
) -> Result<Json<Vec<TextDto>>> {
    let texts = service.get_author_texts().await?;
    Ok(Json(texts))
}

pub async fn get_workshop_items(
    State(service): State<AppService>,
) -> Result<Json<Vec<WorkshopItemDto>>> {
    let items = service.get_workshop_items().await?;
    Ok(Json(items))
}

pub async fn get_cabinet_zones(
    State(service): State<AppService>,
) -> Result<Json<Vec<CabinetZoneDto>>> {
    let zones = service.get_cabinet_zones().await?;
    Ok(Json(zones))
}

// === ADMIN HANDLERS ===

pub async fn upsert_figurine(
    State(service): State<AppService>,
    Json(payload): Json<FigurineDto>,
) -> Result<Json<serde_json::Value>> {
    let id = service.upsert_figurine(payload).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn delete_figurine(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.delete_figurine(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn upload_file(
    State(config): State<crate::config::Config>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    let mut file_url = None;
    let mut relative_path = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "file" {
            let file_name = field.file_name().unwrap_or("unknown").to_string();
            let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
            let data = field.bytes().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

            let extension = std::path::Path::new(&file_name)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("bin");

            let type_dir = if content_type.starts_with("image/") {
                "images"
            } else if content_type.starts_with("video/") {
                "videos"
            } else {
                "misc"
            };

            let uuid = Uuid::new_v4();
            let new_filename = format!("{}.{}", uuid, extension);
            let rel_path = format!("{}/{}", type_dir, new_filename);
            
            let save_dir = format!("{}/{}", config.upload_dir, type_dir);
            fs::create_dir_all(&save_dir).await.map_err(AppError::Io)?;

            let full_path = format!("{}/{}", save_dir, new_filename);
            let mut file = fs::File::create(&full_path).await.map_err(AppError::Io)?;
            file.write_all(&data).await.map_err(AppError::Io)?;

            let public_url = format!("{}/static/{}", config.public_url.trim_end_matches('/'), rel_path);
            
            file_url = Some(public_url);
            relative_path = Some(rel_path);
        }
    }

    if let (Some(url), Some(rel)) = (file_url, relative_path) {
        Ok(Json(serde_json::json!({ "url": url, "relativePath": rel })))
    } else {
        Err(AppError::BadRequest("No file field found".to_string()))
    }
}

pub async fn overwrite_release(
    State(service): State<AppService>,
    Json(payload): Json<ReleasePayload>,
) -> Result<StatusCode> {
    service.process_full_release(payload).await?;
    Ok(StatusCode::OK)
}

pub async fn upload_release_db(
    State(config): State<crate::config::Config>,
    mut multipart: Multipart,
) -> Result<StatusCode> {
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let data = field.bytes().await.map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            
            let save_dir = format!("{}/releases", config.upload_dir);
            fs::create_dir_all(&save_dir).await.map_err(AppError::Io)?;
            
            let full_path = format!("{}/latest.db", save_dir);
            let mut file = fs::File::create(&full_path).await.map_err(AppError::Io)?;
            file.write_all(&data).await.map_err(AppError::Io)?;
            
            return Ok(StatusCode::OK);
        }
    }
    Err(AppError::BadRequest("No file field".to_string()))
}

pub async fn download_release_db(
    State(config): State<crate::config::Config>,
) -> Result<impl IntoResponse> {
    let path = format!("{}/releases/latest.db", config.upload_dir);
    if !std::path::Path::new(&path).exists() {
        return Err(AppError::NotFound("No release database found".to_string()));
    }

    let file = fs::read(&path).await.map_err(AppError::Io)?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/x-sqlite3")],
        [(axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"latest.db\"")],
        file
    ))
}
