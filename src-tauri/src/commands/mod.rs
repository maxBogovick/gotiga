use crate::db::repository::Repository;
use crate::db::Database;
use crate::models::*;
use crate::services::file_service::FileService;
use crate::services::settings_service::{AppSettings, SettingsService};
use crate::services::sync_service::SyncService;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use tauri::State;

// Helper function to resolve paths
// Возвращаем путь через наш кастомный протокол cabinet://
fn resolve_path(_base_path: &str, relative_path: &str) -> String {
    if relative_path.starts_with("http") {
        relative_path.to_string()
    } else {
        // relative_path - это "images/uuid.jpg"
        format!("cabinet://localhost/{}", relative_path)
    }
}

fn derive_image_variants(file_path: &str) -> (Option<String>, Option<String>) {
    let Some(file_name) = file_path.strip_prefix("images/preview/") else {
        return (None, None);
    };
    (
        Some(format!("images/original/{}", file_name)),
        Some(format!("images/thumb/{}", file_name)),
    )
}

fn is_managed_media_path(path: &str) -> bool {
    path.starts_with("images/") || path.starts_with("videos/") || path.starts_with("audio/")
}

fn media_type_for_path(path: &str) -> String {
    if path.starts_with("images/") {
        "image".to_string()
    } else if path.starts_with("videos/") {
        "video".to_string()
    } else if path.starts_with("audio/") {
        "audio".to_string()
    } else {
        "other".to_string()
    }
}

fn variant_for_path(path: &str) -> Option<String> {
    if path.starts_with("images/original/") {
        Some("original".to_string())
    } else if path.starts_with("images/preview/") {
        Some("preview".to_string())
    } else if path.starts_with("images/thumb/") {
        Some("thumb".to_string())
    } else {
        None
    }
}

fn import_folder_for_existing_path(path: &str) -> Result<&'static str, String> {
    if path.starts_with("images/") {
        Ok("images")
    } else if path.starts_with("videos/") {
        Ok("videos")
    } else if path.starts_with("audio/") {
        Ok("audio")
    } else {
        Err(format!("Unsupported managed media path: {}", path))
    }
}

fn imported_variant_paths(path: &str) -> Vec<String> {
    let mut paths = vec![path.to_string()];
    let (original, thumb) = derive_image_variants(path);
    if let Some(p) = original {
        paths.push(p);
    }
    if let Some(p) = thumb {
        paths.push(p);
    }
    paths
}

fn build_media_inventory(
    repo: &Repository<'_>,
    fs: &FileService,
) -> Result<MediaInventoryDto, String> {
    let base_path = fs.get_base_path_string();
    let usages = repo
        .get_media_usages()
        .map_err(|e| format!("Database error: {}", e))?
        .into_iter()
        .filter_map(|mut usage| {
            let cleaned = fs.clean_path(&usage.path);
            if !is_managed_media_path(&cleaned) {
                return None;
            }
            usage.path = cleaned;
            Some(usage)
        })
        .collect::<Vec<_>>();

    let mut usage_map: HashMap<String, Vec<MediaUsageDto>> = HashMap::new();
    for usage in usages {
        usage_map.entry(usage.path.clone()).or_default().push(usage);
    }

    let mut known_paths: HashSet<String> = usage_map.keys().cloned().collect();
    let files_on_disk = fs.list_managed_media_files()?;
    for (path, _) in &files_on_disk {
        known_paths.insert(path.clone());
    }

    let file_size_map: HashMap<String, u64> = files_on_disk.into_iter().collect();
    let mut files = known_paths
        .into_iter()
        .map(|path| {
            let size = file_size_map
                .get(&path)
                .copied()
                .unwrap_or_else(|| fs.media_file_size(&path).unwrap_or(0));
            let exists = file_size_map.contains_key(&path) || fs.file_exists(&path);
            let usages = usage_map.remove(&path).unwrap_or_default();
            MediaFileDto {
                url: resolve_path(&base_path, &path),
                media_type: media_type_for_path(&path),
                variant: variant_for_path(&path),
                size_bytes: size,
                exists,
                path,
                usages,
            }
        })
        .collect::<Vec<_>>();

    files.sort_by(|a, b| {
        b.usages
            .len()
            .cmp(&a.usages.len())
            .then_with(|| a.path.cmp(&b.path))
    });

    let orphan_count = files.iter().filter(|f| f.usages.is_empty()).count();
    let used_count = files.len().saturating_sub(orphan_count);
    let total_size_bytes = files.iter().map(|f| f.size_bytes).sum();

    Ok(MediaInventoryDto {
        files,
        orphan_count,
        used_count,
        total_size_bytes,
    })
}

/// Получить список всех фигур (для витрины)
#[tauri::command]
pub async fn get_all_figurines(
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<Vec<FigurineListItemDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    let base_path = fs.get_base_path_string();

    let figurines = repo
        .get_all_figurines()
        .map_err(|e| format!("Database error: {}", e))?;

    let mut result = Vec::new();

    for fig in figurines {
        let images = repo
            .get_images_for_figurine(&fig.id)
            .map_err(|e| format!("Database error: {}", e))?;

        let face_image = images
            .iter()
            .find(|img| img.image_type == ImageType::Face)
            .map(|img| {
                resolve_path(
                    &base_path,
                    img.thumb_path.as_deref().unwrap_or(&img.file_path),
                )
            });

        result.push(FigurineListItemDto {
            id: fig.id,
            name: fig.name,
            status: fig.status.as_str().to_string(),
            face_image_url: face_image,
            year: fig.year,
            sort_order: fig.sort_order,
            series: None,
        });
    }

    Ok(result)
}

/// Получить детальную информацию о фигуре
#[tauri::command]
pub async fn get_figurine(
    id: String,
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<Option<FigurineDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    let base_path = fs.get_base_path_string();

    let figurine = repo
        .get_figurine_by_id(&id)
        .map_err(|e| format!("Database error: {}", e))?;

    match figurine {
        Some(fig) => {
            let images = repo
                .get_images_for_figurine(&id)
                .map_err(|e| format!("Database error: {}", e))?;

            let steps = repo
                .get_process_steps_for_figurine(&id)
                .map_err(|e| format!("Database error: {}", e))?;

            let related_raw = repo
                .get_related_figurines(&id)
                .map_err(|e| format!("Database error: {}", e))?;

            let mut related_items = Vec::new();
            for r_fig in related_raw {
                let r_images = repo
                    .get_images_for_figurine(&r_fig.id)
                    .map_err(|e| format!("Database error: {}", e))?;

                let r_face_image = r_images
                    .iter()
                    .find(|img| img.image_type == ImageType::Face)
                    .map(|img| {
                        resolve_path(
                            &base_path,
                            img.thumb_path.as_deref().unwrap_or(&img.file_path),
                        )
                    });

                related_items.push(FigurineListItemDto {
                    id: r_fig.id,
                    name: r_fig.name,
                    status: r_fig.status.as_str().to_string(),
                    face_image_url: r_face_image,
                    year: r_fig.year,
                    sort_order: r_fig.sort_order,
                    series: None,
                });
            }

            Ok(Some(FigurineDto::from_figurine(
                fig,
                images,
                steps,
                related_items,
                &base_path,
            )))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn get_author_texts(db: State<'_, Database>) -> Result<Vec<TextDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let texts = repo
        .get_texts_by_category("author")
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(texts.into_iter().map(TextDto::from).collect())
}

#[tauri::command]
pub async fn get_workshop_content(
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<Vec<WorkshopItemDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    let base_path = fs.get_base_path_string();

    let texts = repo
        .get_texts_by_category("workshop")
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(texts
        .into_iter()
        .map(|t| WorkshopItemDto::from_text(t, &base_path))
        .collect())
}

#[tauri::command]
pub async fn get_cabinet_zones(db: State<'_, Database>) -> Result<Vec<CabinetZoneDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let zones = repo
        .get_cabinet_zones()
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(zones.into_iter().map(CabinetZoneDto::from).collect())
}

// === ADMIN COMMANDS ===

#[tauri::command]
pub async fn get_settings(
    settings_service: State<'_, SettingsService>,
) -> Result<AppSettings, String> {
    Ok(settings_service.get_settings())
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    settings_service: State<'_, SettingsService>,
) -> Result<(), String> {
    settings_service.save_settings(settings)
}

#[tauri::command]
pub async fn import_media(
    file_path: String,
    media_type: String,
    fs: State<'_, FileService>,
) -> Result<String, String> {
    // Импортируем файл и получаем относительный путь
    let relative_path = fs.import_file(&file_path, &media_type)?;

    // Возвращаем полный путь (не asset://), чтобы фронт мог его отобразить
    let base_path = fs.get_base_path_string();
    Ok(resolve_path(&base_path, &relative_path))
}

#[tauri::command]
pub async fn delete_figurine(id: String, db: State<'_, Database>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    repo.delete_figurine(&id)
        .map_err(|e| format!("Database error: {}", e))
}

#[tauri::command]
pub async fn cleanup_unused_media(
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<Vec<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    let referenced = repo
        .get_all_media_paths()
        .map_err(|e| format!("Database error: {}", e))?;
    fs.cleanup_unused_media(&referenced)
}

#[tauri::command]
pub async fn get_media_inventory(
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<MediaInventoryDto, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    build_media_inventory(&repo, &fs)
}

#[tauri::command]
pub async fn get_unused_media_report(
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<MediaCleanupReportDto, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    let inventory = build_media_inventory(&repo, &fs)?;
    let files = inventory
        .files
        .into_iter()
        .filter(|file| file.exists && file.usages.is_empty())
        .collect::<Vec<_>>();
    let total_size_bytes = files.iter().map(|file| file.size_bytes).sum();
    Ok(MediaCleanupReportDto {
        files,
        total_size_bytes,
    })
}

#[tauri::command]
pub async fn cleanup_reported_unused_media(
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<Vec<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    let report = build_media_inventory(&repo, &fs)?;
    let orphan_paths = report
        .files
        .into_iter()
        .filter(|file| file.exists && file.usages.is_empty())
        .map(|file| file.path)
        .collect::<Vec<_>>();
    fs.delete_media_files(&orphan_paths)
}

#[tauri::command]
pub async fn replace_media_everywhere(
    old_path: String,
    replacement_file_path: String,
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<MediaReplaceResultDto, String> {
    let old_path = fs.clean_path(&old_path);
    if !is_managed_media_path(&old_path) {
        return Err(format!(
            "Only managed local media can be replaced: {}",
            old_path
        ));
    }

    let import_folder = import_folder_for_existing_path(&old_path)?;
    let new_path = fs.import_file(&replacement_file_path, import_folder)?;
    let (new_original, new_thumb) = derive_image_variants(&new_path);
    let imported_paths = imported_variant_paths(&new_path);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    let updated_references = repo
        .replace_media_path_everywhere(
            &old_path,
            &new_path,
            new_original.as_deref(),
            new_thumb.as_deref(),
        )
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(MediaReplaceResultDto {
        old_path,
        new_path,
        updated_references,
        imported_paths,
    })
}

#[tauri::command]
pub async fn save_figurine(
    mut figurine: FigurineDto,
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    // Используем fs.clean_path для превращения абсолютных путей обратно в относительные
    let clean_path = |p: Option<String>| -> Option<String> { p.map(|s| fs.clean_path(&s)) };

    // Забираем images и process_steps
    let images_dto = std::mem::take(&mut figurine.images);
    let steps_dto = std::mem::take(&mut figurine.process_steps);
    let figurine_id = figurine.id.clone();
    let now = Utc::now().to_rfc3339();

    let model = Figurine {
        id: figurine.id,
        name: figurine.name,
        short_text: figurine.short_text,
        full_description: figurine.full_description,
        dimensions: figurine.dimensions,
        material: figurine.material,
        technique: figurine.technique,
        year: figurine.year,
        ambience_path: clean_path(figurine.ambience_path),
        video_url: clean_path(figurine.video_url),
        secret_text: figurine.secret_text,
        status: FigurineStatus::from_str(&figurine.status),
        sort_order: figurine.sort_order,
        updated_at: now.clone(),
        is_visible: figurine.is_visible,
    };

    repo.upsert_figurine(&model)
        .map_err(|e| format!("Database error: {}", e))?;

    // Сохраняем картинки
    let images: Vec<Image> = images_dto
        .into_iter()
        .map(|img_dto| {
            let file_path = clean_path(Some(img_dto.url)).unwrap_or_default();
            let original_path = clean_path(img_dto.original_url);
            let thumb_path = clean_path(img_dto.thumb_url);
            let (derived_original, derived_thumb) = derive_image_variants(&file_path);
            Image {
                id: img_dto.id,
                figurine_id: figurine_id.clone(),
                image_type: ImageType::from_str(&img_dto.image_type),
                file_path,
                original_path: original_path.or(derived_original),
                thumb_path: thumb_path.or(derived_thumb),
                alt_text: img_dto.alt_text,
                sort_order: 0,
                updated_at: now.clone(),
            }
        })
        .collect();

    repo.replace_images(&figurine_id, images)
        .map_err(|e| format!("Database error images: {}", e))?;

    // Сохраняем этапы (Grimoire)
    let steps: Vec<ProcessStep> = steps_dto
        .into_iter()
        .map(|step_dto| ProcessStep {
            id: step_dto.id,
            figurine_id: figurine_id.clone(),
            step_type: ProcessStepType::from_str(&step_dto.step_type),
            description: step_dto.description,
            image_path: clean_path(Some(step_dto.image_url)).unwrap_or_default(),
            sort_order: 0,
            updated_at: now.clone(),
        })
        .collect();

    repo.replace_process_steps(&figurine_id, steps)
        .map_err(|e| format!("Database error steps: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn save_cabinet_zone(
    zone: CabinetZoneDto,
    db: State<'_, Database>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let model = CabinetZone {
        id: zone.id,
        zone_type: zone.zone_type,
        x_percent: zone.x,
        y_percent: zone.y,
        width_percent: zone.width,
        height_percent: zone.height,
        target_route: zone.target_route,
    };

    repo.upsert_cabinet_zone(&model).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_cabinet_zone(id: String, db: State<'_, Database>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    repo.delete_cabinet_zone(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_text(
    dto: WorkshopItemDto, // Using this generic DTO which fits both
    category: String,
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let clean_path = |p: Option<String>| -> Option<String> { p.map(|s| fs.clean_path(&s)) };

    let cat = if category == "author" {
        TextCategory::Author
    } else {
        TextCategory::Workshop
    };

    let model = Text {
        id: dto.id,
        category: cat,
        content: dto.content,
        caption: dto.caption,
        image_path: clean_path(dto.image_url),
        sort_order: 0,
        updated_at: Utc::now().to_rfc3339(),
    };

    repo.upsert_text(&model).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_text(id: String, db: State<'_, Database>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    repo.delete_text(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_main_background(
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<Option<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    let base_path = fs.get_base_path_string();

    match repo
        .get_app_resource("main_background")
        .map_err(|e| e.to_string())?
    {
        Some(path) => Ok(Some(resolve_path(&base_path, &path))),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn set_main_background(
    file_path: String,
    db: State<'_, Database>,
    fs: State<'_, FileService>,
) -> Result<String, String> {
    // 1. Импорт файла в локальное хранилище (images)
    let relative_path = fs.import_file(&file_path, "images")?;

    // 2. Чтение байтов для BLOB
    let full_path = fs.get_full_path(&relative_path);
    let data = std::fs::read(&full_path).map_err(|e| e.to_string())?;

    // 3. Сохранение в БД
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);
    repo.upsert_app_resource("main_background", &relative_path, Some(data))
        .map_err(|e| e.to_string())?;

    // 4. Возврат cabinet:// URL
    let base_path = fs.get_base_path_string();
    Ok(resolve_path(&base_path, &relative_path))
}

#[tauri::command]
pub async fn export_release(
    db: State<'_, Database>,
    sync: State<'_, SyncService>,
    settings_service: State<'_, SettingsService>,
) -> Result<String, String> {
    let settings = settings_service.get_settings();
    sync.push_full_release(&db, &settings).await
}

#[tauri::command]
pub async fn pull_updates(
    db: State<'_, Database>,
    sync: State<'_, SyncService>,
    settings_service: State<'_, SettingsService>,
) -> Result<String, String> {
    let settings = settings_service.get_settings();
    sync.pull_updates(&db, &settings).await
}

#[tauri::command]
pub async fn push_figurine(
    figurine: FigurineDto,
    sync: State<'_, SyncService>,
    settings_service: State<'_, SettingsService>,
) -> Result<String, String> {
    let settings = settings_service.get_settings();
    sync.push_figurine(figurine, &settings).await
}

#[tauri::command]
pub async fn get_server_releases(
    sync: State<'_, SyncService>,
    settings_service: State<'_, SettingsService>,
) -> Result<Vec<ServerRelease>, String> {
    let settings = settings_service.get_settings();
    sync.list_server_releases(&settings).await
}

#[tauri::command]
pub async fn activate_server_release(
    id: String,
    sync: State<'_, SyncService>,
    settings_service: State<'_, SettingsService>,
) -> Result<(), String> {
    let settings = settings_service.get_settings();
    sync.activate_server_release(&id, &settings).await
}
