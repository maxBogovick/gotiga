mod db;
mod models;
mod commands;
mod services;

use db::Database;
use services::file_service::FileService;
use services::sync_service::SyncService;
use services::settings_service::SettingsService;
use tauri::Manager;
use tauri::http::Response;
use std::fs;
use std::ffi::OsStr;
use std::path::Path;

fn build_response(file_path: &Path, content: Vec<u8>) -> Response<Vec<u8>> {
    let extension = file_path.extension()
        .and_then(|e: &OsStr| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream"
    };

    Response::builder()
        .header("Content-Type", mime)
        .header("Access-Control-Allow-Origin", "*")
        .body(content)
        .unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("cabinet", |ctx, request| {
            println!("Cabinet Protocol Request: {}", request.uri());
            let app_handle = ctx.app_handle();
            
            let response_404 = || Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap();

            let response_500 = || Response::builder()
                .status(500)
                .body(Vec::new())
                .unwrap();

            let app_data = match app_handle.path().app_data_dir() {
                Ok(path) => path,
                Err(_) => return response_500(),
            };

            let path = request.uri().path();
            let relative = path.strip_prefix("/").unwrap_or(path);
            let decoded = match urlencoding::decode(relative) {
                Ok(d) => d,
                Err(_) => return response_500(),
            };

            let file_path = app_data.join(decoded.as_ref());
            println!("Cabinet resolving file: {:?}", file_path);

            // 1. Пытаемся прочитать с диска (Dev mode / Local files)
            if file_path.exists() {
                let content = match fs::read(&file_path) {
                    Ok(c) => c,
                    Err(_) => return response_500(),
                };
                return build_response(&file_path, content);
            }

            // 2. Если на диске нет — ищем в BLOB в базе данных
            let db_path = app_data.join("cabinet.db");
            let relative_path = decoded.as_ref().to_string(); // напр. "images/uuid.jpg"

            enum BlobSource {
                Image(String),
                Step(String),
                FigurineVideo(String),
                FigurineAudio(String),
                Text(String),
            }

            let content = match rusqlite::Connection::open(&db_path) {
                Ok(conn) => {
                    let mut found: Option<(Vec<u8>, BlobSource)> = None;

                    // Поиск в images
                    if let Ok((id, data)) = conn.query_row(
                        "SELECT id, data FROM images WHERE file_path = ?1",
                        [&relative_path],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    ) {
                        found = Some((data, BlobSource::Image(id)));
                    } 
                    // Поиск в process_steps
                    else if let Ok((id, data)) = conn.query_row(
                        "SELECT id, image_data FROM process_steps WHERE image_path = ?1",
                        [&relative_path],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    ) {
                        found = Some((data, BlobSource::Step(id)));
                    }
                    // Поиск в figurines (video)
                    else if let Ok((id, data)) = conn.query_row(
                        "SELECT id, video_data FROM figurines WHERE video_url = ?1",
                        [&relative_path],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    ) {
                        found = Some((data, BlobSource::FigurineVideo(id)));
                    }
                    // Поиск в figurines (audio)
                    else if let Ok((id, data)) = conn.query_row(
                        "SELECT id, ambience_data FROM figurines WHERE ambience_path = ?1",
                        [&relative_path],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    ) {
                        found = Some((data, BlobSource::FigurineAudio(id)));
                    }
                    // Поиск в texts (workshop)
                    else if let Ok((id, data)) = conn.query_row(
                        "SELECT id, image_data FROM texts WHERE image_path = ?1",
                        [&relative_path],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    ) {
                        found = Some((data, BlobSource::Text(id)));
                    }

                    found
                },
                Err(_) => None
            };

            if let Some((bytes, source)) = content {
                // ОПТИМИЗАЦИЯ: Ленивое восстановление файла (Self-Repairing) + Обновление пути в БД
                let cache_path = file_path.clone();
                let cache_data = bytes.clone();
                let db_path_clone = db_path.clone();
                let new_db_path_value = relative_path.replace('\\', "/"); // Нормализуем путь для БД
                
                std::thread::spawn(move || {
                    // 1. Restore File
                    if let Some(parent) = cache_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if let Err(e) = fs::write(&cache_path, &cache_data) {
                        eprintln!("Failed to restore file from BLOB: {}", e);
                        return; // Если не записали файл, в базу не пишем (хотя спорно, но безопаснее)
                    } else {
                        println!("Restored missing file from BLOB: {:?}", cache_path);
                    }

                    // 2. Update DB Record (Reliability update)
                    if let Ok(conn) = rusqlite::Connection::open(&db_path_clone) {
                        let res = match source {
                            BlobSource::Image(id) => 
                                conn.execute("UPDATE images SET file_path = ?1 WHERE id = ?2", [&new_db_path_value, &id]),
                            BlobSource::Step(id) => 
                                conn.execute("UPDATE process_steps SET image_path = ?1 WHERE id = ?2", [&new_db_path_value, &id]),
                            BlobSource::FigurineVideo(id) => 
                                conn.execute("UPDATE figurines SET video_url = ?1 WHERE id = ?2", [&new_db_path_value, &id]),
                            BlobSource::FigurineAudio(id) => 
                                conn.execute("UPDATE figurines SET ambience_path = ?1 WHERE id = ?2", [&new_db_path_value, &id]),
                            BlobSource::Text(id) => 
                                conn.execute("UPDATE texts SET image_path = ?1 WHERE id = ?2", [&new_db_path_value, &id]),
                        };
                        
                        if let Err(e) = res {
                            eprintln!("Failed to update DB path after restore: {}", e);
                        }
                    }
                });

                build_response(&file_path, bytes)
            } else {
                println!("File not found on disk or in DB: {}", relative_path);
                response_404()
            }
        })
        .setup(|app| {
            // Путь к БД в директории данных приложения
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data dir");

            let db_path = app_data_dir.join("cabinet.db");

            // Инициализация сервисов
            let file_service = FileService::new(app.handle());
            let sync_service = SyncService::new(app.handle());
            let settings_service = SettingsService::new(app.handle());
            
            app.manage(file_service);
            app.manage(sync_service);
            app.manage(settings_service);

            // Инициализация БД
            let db = Database::new(db_path)
                .expect("Failed to initialize database");
            
            // Заполнение тестовыми данными если БД пуста
            db.seed_if_empty()
                .expect("Failed to seed database");

            // Сохранить в состоянии приложения
            app.manage(db);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_all_figurines,
            commands::get_figurine,
            commands::get_author_texts,
            commands::get_workshop_content,
            commands::get_cabinet_zones,
            commands::import_media,
            commands::save_figurine,
            commands::export_release,
            commands::pull_updates,
            commands::push_figurine,
            commands::get_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
