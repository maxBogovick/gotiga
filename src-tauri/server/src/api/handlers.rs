use axum::{
    extract::{Path, State, Multipart, Query},
    Json,
    http::StatusCode,
    response::IntoResponse,
    body::Bytes,
};
use crate::services::AppService;
use crate::models::*;
use crate::error::{Result, AppError};
use uuid::Uuid;
use tokio::fs;
use tokio::io::AsyncWriteExt;

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
