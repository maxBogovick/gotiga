use tauri::State;
use crate::db::Database;
use crate::db::repository::Repository;
use crate::models::*;

/// Получить список всех фигур (для витрины)
#[tauri::command]
pub async fn get_all_figurines(
    db: State<'_, Database>
) -> Result<Vec<FigurineListItemDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let figurines = repo.get_all_figurines()
        .map_err(|e| format!("Database error: {}", e))?;

    let mut result = Vec::new();

    for fig in figurines {
        // Получить face-изображение
        let images = repo.get_images_for_figurine(&fig.id)
            .map_err(|e| format!("Database error: {}", e))?;

        let face_image = images.iter()
            .find(|img| img.image_type == ImageType::Face)
            .map(|img| {
                if img.file_path.starts_with("http") {
                    img.file_path.clone()
                } else {
                    format!("asset://localhost/{}", img.file_path)
                }
            });

        result.push(FigurineListItemDto {
            id: fig.id,
            name: fig.name,
            status: fig.status.as_str().to_string(),
            face_image_url: face_image,
        });
    }

    Ok(result)
}

/// Получить детальную информацию о фигуре
#[tauri::command]
pub async fn get_figurine(
    id: String,
    db: State<'_, Database>
) -> Result<Option<FigurineDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let figurine = repo.get_figurine_by_id(&id)
        .map_err(|e| format!("Database error: {}", e))?;

    match figurine {
        Some(fig) => {
            let images = repo.get_images_for_figurine(&id)
                .map_err(|e| format!("Database error: {}", e))?;
            
            let steps = repo.get_process_steps_for_figurine(&id)
                .map_err(|e| format!("Database error: {}", e))?;

            // Fetch related items
            let related_raw = repo.get_related_figurines(&id)
                .map_err(|e| format!("Database error: {}", e))?;
            
            let mut related_items = Vec::new();
            for r_fig in related_raw {
                 let r_images = repo.get_images_for_figurine(&r_fig.id)
                    .map_err(|e| format!("Database error: {}", e))?;
                 
                 let r_face_image = r_images.iter()
                    .find(|img| img.image_type == ImageType::Face)
                    .map(|img| {
                        if img.file_path.starts_with("http") {
                            img.file_path.clone()
                        } else {
                            format!("asset://localhost/{}", img.file_path)
                        }
                    });
                
                related_items.push(FigurineListItemDto {
                    id: r_fig.id,
                    name: r_fig.name,
                    status: r_fig.status.as_str().to_string(),
                    face_image_url: r_face_image,
                });
            }

            Ok(Some(FigurineDto::from_figurine(fig, images, steps, related_items)))
        }
        None => Ok(None)
    }
}

/// Получить авторские тексты
#[tauri::command]
pub async fn get_author_texts(
    db: State<'_, Database>
) -> Result<Vec<TextDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let texts = repo.get_texts_by_category("author")
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(texts.into_iter().map(TextDto::from).collect())
}

/// Получить контент мастерской
#[tauri::command]
pub async fn get_workshop_content(
    db: State<'_, Database>
) -> Result<Vec<WorkshopItemDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let texts = repo.get_texts_by_category("workshop")
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(texts.into_iter().map(WorkshopItemDto::from).collect())
}

/// Получить интерактивные зоны кабинета
#[tauri::command]
pub async fn get_cabinet_zones(
    db: State<'_, Database>
) -> Result<Vec<CabinetZoneDto>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = Repository::new(&conn);

    let zones = repo.get_cabinet_zones()
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(zones.into_iter().map(CabinetZoneDto::from).collect())
}
