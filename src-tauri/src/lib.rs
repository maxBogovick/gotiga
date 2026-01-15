mod db;
mod models;
mod commands;

use db::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Путь к БД в директории данных приложения
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data dir");

            let db_path = app_data_dir.join("cabinet.db");

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
