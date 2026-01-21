use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::AppHandle;
use tauri::Manager;
use uuid::Uuid;

#[derive(Clone)]
pub struct FileService {
    base_dir: PathBuf,
}

impl FileService {
    pub fn new(app: &AppHandle) -> Self {
        // Получаем путь к AppData
        let base_dir = app.path().app_data_dir()
            .expect("Critical: Failed to resolve app data directory");
        
        // Гарантируем существование директории
        if !base_dir.exists() {
            if let Err(e) = fs::create_dir_all(&base_dir) {
                eprintln!("Critical: Failed to create app data directory: {}", e);
            }
        }
        
        Self { base_dir }
    }

    /// Получить абсолютный путь к корню данных как строку
    pub fn get_base_path_string(&self) -> String {
        self.base_dir.to_string_lossy().to_string()
    }

    /// Получить полный PathBuf для относительного пути (без проверок безопасности, для внутреннего использования)
    pub fn get_full_path(&self, relative_path: &str) -> PathBuf {
        self.base_dir.join(relative_path)
    }

    /// Безопасно разрешить относительный путь относительно base_dir
    fn resolve_path(&self, relative_path: &str) -> Result<PathBuf, String> {
        let path = Path::new(relative_path);
        
        // Защита от Path Traversal: проверяем компоненты пути
        for component in path.components() {
            match component {
                Component::Normal(_) => {},
                _ => return Err(format!("Invalid path component in '{}': only normal components allowed", relative_path)),
            }
        }

        let dest_path = self.base_dir.join(path);
        
        // Дополнительная проверка: путь должен начинаться с base_dir
        if !dest_path.starts_with(&self.base_dir) {
             return Err("Access denied: Path is outside of app data directory".to_string());
        }

        Ok(dest_path)
    }

    /// Сохранить байты на диск
    pub fn save_file(&self, relative_path: &str, data: &[u8]) -> Result<String, String> {
        let dest_path = self.resolve_path(relative_path)?;

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        fs::write(&dest_path, data).map_err(|e| e.to_string())?;
        
        Ok(relative_path.to_string())
    }

    /// Импортировать внешний файл
    pub fn import_file(&self, source_path: &str, target_subfolder: &str) -> Result<String, String> {
        let src = Path::new(source_path);
        if !src.exists() {
            return Err(format!("Source file not found: {}", source_path));
        }

        let extension = src.extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
            
        // Используем UUID для гарантии уникальности
        let new_filename = if extension.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            format!("{}.{}", Uuid::new_v4(), extension)
        };
        
        // Формируем относительный путь: "videos/uuid.mp4"
        // Используем PathBuf для кросс-платформенного формирования
        let relative_path = Path::new(target_subfolder)
            .join(&new_filename)
            .to_string_lossy()
            .replace('\\', "/"); // Нормализуем слеши для БД

        let dest_path = self.resolve_path(&relative_path)?;

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        fs::copy(src, &dest_path).map_err(|e| e.to_string())?;

        Ok(relative_path)
    }

    /// Проверить существование файла
    pub fn file_exists(&self, relative_path: &str) -> bool {
        if let Ok(path) = self.resolve_path(relative_path) {
            path.exists()
        } else {
            false
        }
    }

        /// Очищает путь от asset://, cabinet:// или абсолютного пути AppData
        pub fn clean_path(&self, full_path: &str) -> String {
            let mut path = full_path.to_string();
    
            // 1. Remove protocols
            if path.starts_with("cabinet://") {
                path = path.replace("cabinet://localhost", "").replace("cabinet://", "");
            } else if path.starts_with("asset://") {
                path = path.replace("asset://localhost", "").replace("asset://", "");
            }
    
            // 2. Remove base dir if present (absolute path case)
            let base_str = self.base_dir.to_string_lossy().to_string();
            // Normalize slashes for comparison just in case
            let path_norm = path.replace('\\', "/");
            let base_norm = base_str.replace('\\', "/");
    
            if path_norm.contains(&base_norm) {
                path = path_norm.replace(&base_norm, "");
            } else if path.contains(&base_str) {
                 path = path.replace(&base_str, "");
            }
    
            // 3. Trim leading slashes
            path.trim_start_matches('/').trim_start_matches('\\').to_string()
        }
    }